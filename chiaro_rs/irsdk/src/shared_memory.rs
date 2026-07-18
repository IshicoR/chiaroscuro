use std::{io, mem::size_of, ops::Range, sync::Arc};

use crate::{
    SessionInfo, TelemetryFrame, TelemetrySample, TelemetrySnapshot, TelemetryValue,
    VariableCatalog, VariableMetadata, VariableType,
};

#[path = "ibt.rs"]
mod ibt;

pub use ibt::{IbtFile, IbtFrames, IbtMetadata, IbtReader, IbtSnapshots};

#[cfg(target_os = "windows")]
const IRSDK_MEMORY_MAPPING: &str = "Local\\IRSDKMemMapFileName";
#[cfg(target_os = "windows")]
const IRSDK_DATA_VALID_EVENT: &str = "Local\\IRSDKDataValidEvent";
const IRSDK_HEADER_VERSION: i32 = 2;
const IRSDK_CONNECTED: i32 = 1;
const IRSDK_MAX_BUFFERS: usize = 4;
const MAX_VARIABLES: usize = 4_096;
const MAX_BUFFER_LEN: usize = 64 * 1024 * 1024;
const MAX_MAPPING_LEN: usize = 256 * 1024 * 1024;
const MAX_SESSION_INFO_LEN: usize = 16 * 1024 * 1024;
const GRAVITY_METRES_PER_SECOND_SQUARED: f32 = 9.806_65;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct VariableBuffer {
    // Published after the writer finishes the frame.
    tick_count: i32,
    buffer_offset: i32,
    // Newer SDKs publish this before writing; legacy SDKs leave it reserved as zero.
    tick_count_begin: i32,
    pad: [i32; 1],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
struct Header {
    version: i32,
    status: i32,
    tick_rate: i32,
    session_info_update: i32,
    session_info_len: i32,
    session_info_offset: i32,
    num_variables: i32,
    variable_header_offset: i32,
    num_buffers: i32,
    buffer_len: i32,
    // iRacing SDK 1.20 exposes these fields for direct access to the newest buffer.
    current_buffer_tick_count: i32,
    current_buffer: u8,
    pad: [u8; 3],
    variable_buffers: [VariableBuffer; IRSDK_MAX_BUFFERS],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct VariableHeader {
    variable_type: i32,
    offset: i32,
    count: i32,
    count_as_time: u8,
    pad: [u8; 3],
    name: [u8; 32],
    description: [u8; 64],
    unit: [u8; 32],
}

impl Default for VariableHeader {
    fn default() -> Self {
        Self {
            variable_type: 0,
            offset: 0,
            count: 0,
            count_as_time: 0,
            pad: [0; 3],
            name: [0; 32],
            description: [0; 64],
            unit: [0; 32],
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Variable {
    variable_type: VariableType,
    offset: usize,
    count: usize,
}

impl Variable {
    fn element_offset(self, index: usize) -> io::Result<usize> {
        if index >= self.count {
            return Err(invalid_data("iRacing variable array index is out of range"));
        }

        self.offset
            .checked_add(index.saturating_mul(self.variable_type.byte_len()))
            .ok_or_else(|| invalid_data("iRacing variable offset overflowed"))
    }

    fn float(self, data: &[u8]) -> io::Result<f32> {
        if self.variable_type != VariableType::Float || self.count != 1 {
            return Err(invalid_data("telemetry variable is not a scalar float"));
        }
        Ok(f32::from_le_bytes(read_bytes(
            data,
            self.element_offset(0)?,
        )?))
    }

    fn double(self, data: &[u8]) -> io::Result<f64> {
        if self.variable_type != VariableType::Double || self.count != 1 {
            return Err(invalid_data("telemetry variable is not a scalar double"));
        }
        Ok(f64::from_le_bytes(read_bytes(
            data,
            self.element_offset(0)?,
        )?))
    }

    fn int(self, data: &[u8]) -> io::Result<i32> {
        if self.variable_type != VariableType::Int || self.count != 1 {
            return Err(invalid_data("telemetry variable is not a scalar int"));
        }
        Ok(i32::from_le_bytes(read_bytes(
            data,
            self.element_offset(0)?,
        )?))
    }

    fn boolean(self, data: &[u8]) -> io::Result<bool> {
        if self.variable_type != VariableType::Bool || self.count != 1 {
            return Err(invalid_data("telemetry variable is not a scalar bool"));
        }
        Ok(u8::from_le_bytes(read_bytes(data, self.element_offset(0)?)?) != 0)
    }

    fn value(self, data: &[u8]) -> io::Result<TelemetryValue> {
        let scalar = self.count == 1;

        match self.variable_type {
            VariableType::Char if scalar => Ok(TelemetryValue::Char(u8::from_le_bytes(
                read_bytes(data, self.element_offset(0)?)?,
            ))),
            VariableType::Bool if scalar => Ok(TelemetryValue::Bool(
                u8::from_le_bytes(read_bytes(data, self.element_offset(0)?)?) != 0,
            )),
            VariableType::Int if scalar => Ok(TelemetryValue::Int(i32::from_le_bytes(read_bytes(
                data,
                self.element_offset(0)?,
            )?))),
            VariableType::BitField if scalar => Ok(TelemetryValue::BitField(u32::from_le_bytes(
                read_bytes(data, self.element_offset(0)?)?,
            ))),
            VariableType::Float if scalar => Ok(TelemetryValue::Float(f32::from_le_bytes(
                read_bytes(data, self.element_offset(0)?)?,
            ))),
            VariableType::Double if scalar => Ok(TelemetryValue::Double(f64::from_le_bytes(
                read_bytes(data, self.element_offset(0)?)?,
            ))),
            VariableType::Char => Ok(TelemetryValue::Chars(
                self.collect(data, |data, offset| {
                    Ok(u8::from_le_bytes(read_bytes(data, offset)?))
                })?,
            )),
            VariableType::Bool => Ok(TelemetryValue::Bools(
                self.collect(data, |data, offset| {
                    Ok(u8::from_le_bytes(read_bytes(data, offset)?) != 0)
                })?,
            )),
            VariableType::Int => Ok(TelemetryValue::Ints(
                self.collect(data, |data, offset| {
                    Ok(i32::from_le_bytes(read_bytes(data, offset)?))
                })?,
            )),
            VariableType::BitField => Ok(TelemetryValue::BitFields(
                self.collect(data, |data, offset| {
                    Ok(u32::from_le_bytes(read_bytes(data, offset)?))
                })?,
            )),
            VariableType::Float => Ok(TelemetryValue::Floats(
                self.collect(data, |data, offset| {
                    Ok(f32::from_le_bytes(read_bytes(data, offset)?))
                })?,
            )),
            VariableType::Double => Ok(TelemetryValue::Doubles(
                self.collect(data, |data, offset| {
                    Ok(f64::from_le_bytes(read_bytes(data, offset)?))
                })?,
            )),
        }
    }

    fn collect<T>(
        self,
        data: &[u8],
        mut read: impl FnMut(&[u8], usize) -> io::Result<T>,
    ) -> io::Result<Box<[T]>> {
        (0..self.count)
            .map(|index| read(data, self.element_offset(index)?))
            .collect::<io::Result<Vec<_>>>()
            .map(Vec::into_boxed_slice)
    }
}

#[derive(Debug)]
struct VariableTable {
    catalog: Arc<VariableCatalog>,
    variables: Vec<Variable>,
}

impl VariableTable {
    fn parse(headers: &[VariableHeader], buffer_len: usize) -> io::Result<Self> {
        let mut metadata = Vec::with_capacity(headers.len());
        let mut variables = Vec::with_capacity(headers.len());

        for header in headers {
            let variable_type = VariableType::from_raw(header.variable_type).ok_or_else(|| {
                invalid_data(format!(
                    "unknown iRacing variable type {}",
                    header.variable_type
                ))
            })?;
            let offset = usize::try_from(header.offset)
                .map_err(|_| invalid_data("iRacing variable has a negative offset"))?;
            let count = usize::try_from(header.count)
                .map_err(|_| invalid_data("iRacing variable has a negative count"))?;
            if count == 0 {
                return Err(invalid_data("iRacing variable has an empty value array"));
            }

            let byte_len = count
                .checked_mul(variable_type.byte_len())
                .ok_or_else(|| invalid_data("iRacing variable length overflowed"))?;
            let end = offset
                .checked_add(byte_len)
                .ok_or_else(|| invalid_data("iRacing variable range overflowed"))?;
            if end > buffer_len {
                return Err(invalid_data(
                    "iRacing variable lies outside its data buffer",
                ));
            }

            let name = fixed_string(&header.name);
            if name.is_empty() {
                return Err(invalid_data("iRacing variable has no name"));
            }

            let variable = Variable {
                variable_type,
                offset,
                count,
            };
            metadata.push(VariableMetadata {
                name,
                description: fixed_string(&header.description),
                unit: fixed_string(&header.unit),
                value_type: variable_type,
                count,
                count_as_time: header.count_as_time != 0,
            });
            variables.push(variable);
        }

        let metadata = Arc::from(metadata);
        let catalog = VariableCatalog::new(metadata)
            .map(Arc::new)
            .map_err(|error| invalid_data(error.to_string()))?;

        Ok(Self { catalog, variables })
    }

    fn require(&self, name: &str, value_type: VariableType) -> io::Result<Variable> {
        let variable = self
            .catalog
            .index(name)
            .map(|index| self.variables[index])
            .ok_or_else(|| invalid_data(format!("missing iRacing telemetry variable `{name}`")))?;
        self.validate_scalar(name, variable, value_type)?;
        Ok(variable)
    }

    fn optional(&self, name: &str, value_type: VariableType) -> io::Result<Option<Variable>> {
        let Some(index) = self.catalog.index(name) else {
            return Ok(None);
        };
        let variable = self.variables[index];
        self.validate_scalar(name, variable, value_type)?;
        Ok(Some(variable))
    }

    fn validate_scalar(
        &self,
        name: &str,
        variable: Variable,
        value_type: VariableType,
    ) -> io::Result<()> {
        if variable.variable_type != value_type || variable.count != 1 {
            return Err(invalid_data(format!(
                "iRacing telemetry variable `{name}` is {}[{}], expected a scalar {value_type}",
                variable.variable_type, variable.count
            )));
        }
        Ok(())
    }

    fn metadata(&self) -> &[VariableMetadata] {
        self.catalog.metadata()
    }

    fn value(&self, name: &str, data: &[u8]) -> io::Result<TelemetryValue> {
        let index = self.catalog.index(name).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("unknown iRacing telemetry variable `{name}`"),
            )
        })?;
        self.variables[index].value(data)
    }

    fn frame(&self, packet_id: i32, data: &[u8]) -> io::Result<TelemetryFrame> {
        let values = self
            .variables
            .iter()
            .map(|variable| variable.value(data))
            .collect::<io::Result<Vec<_>>>()?
            .into_boxed_slice();

        TelemetryFrame::from_catalog(packet_id, Arc::clone(&self.catalog), values)
            .map_err(|error| invalid_data(error.to_string()))
    }
}

#[derive(Debug)]
struct TelemetryVariables {
    speed: Variable,
    rpm: Variable,
    gear: Variable,
    throttle: Variable,
    brake: Variable,
    clutch: Variable,
    steering_angle: Variable,
    fuel: Variable,
    acceleration: [Variable; 3],
    yaw_rate: Option<Variable>,
    wheel_speed: [Option<Variable>; 4],
    tyre_temperature: [Option<Variable>; 4],
    shock_deflection: [Option<Variable>; 4],
    current_lap_time: Variable,
    last_lap_time: Variable,
    best_lap_time: Variable,
    completed_laps: Variable,
    position: Variable,
    on_pit_road: Variable,
    lap_distance_percentage: Variable,
    session_time_remaining: Variable,
}

impl TelemetryVariables {
    fn resolve(table: &VariableTable) -> io::Result<Self> {
        Ok(Self {
            speed: table.require("Speed", VariableType::Float)?,
            rpm: table.require("RPM", VariableType::Float)?,
            gear: table.require("Gear", VariableType::Int)?,
            throttle: table.require("Throttle", VariableType::Float)?,
            brake: table.require("Brake", VariableType::Float)?,
            clutch: table.require("Clutch", VariableType::Float)?,
            steering_angle: table.require("SteeringWheelAngle", VariableType::Float)?,
            fuel: table.require("FuelLevel", VariableType::Float)?,
            acceleration: [
                table.require("LatAccel", VariableType::Float)?,
                table.require("LongAccel", VariableType::Float)?,
                table.require("VertAccel", VariableType::Float)?,
            ],
            yaw_rate: table.optional("YawRate", VariableType::Float)?,
            wheel_speed: [
                table.optional("LFspeed", VariableType::Float)?,
                table.optional("RFspeed", VariableType::Float)?,
                table.optional("LRspeed", VariableType::Float)?,
                table.optional("RRspeed", VariableType::Float)?,
            ],
            tyre_temperature: [
                table.optional("LFtempCM", VariableType::Float)?,
                table.optional("RFtempCM", VariableType::Float)?,
                table.optional("LRtempCM", VariableType::Float)?,
                table.optional("RRtempCM", VariableType::Float)?,
            ],
            shock_deflection: [
                table.optional("LFshockDefl", VariableType::Float)?,
                table.optional("RFshockDefl", VariableType::Float)?,
                table.optional("LRshockDefl", VariableType::Float)?,
                table.optional("RRshockDefl", VariableType::Float)?,
            ],
            current_lap_time: table.require("LapCurrentLapTime", VariableType::Float)?,
            last_lap_time: table.require("LapLastLapTime", VariableType::Float)?,
            best_lap_time: table.require("LapBestLapTime", VariableType::Float)?,
            completed_laps: table.require("LapCompleted", VariableType::Int)?,
            position: table.require("PlayerCarPosition", VariableType::Int)?,
            on_pit_road: table.require("OnPitRoad", VariableType::Bool)?,
            lap_distance_percentage: table.require("LapDistPct", VariableType::Float)?,
            session_time_remaining: table.require("SessionTimeRemain", VariableType::Double)?,
        })
    }

    fn sample(&self, tick_count: i32, data: &[u8]) -> io::Result<TelemetrySample> {
        let speed_metres_per_second = self.speed.float(data)?;
        let wheel_slip =
            std::array::from_fn(|index| {
                self.wheel_speed[index].map_or(Ok(0.0), |variable| {
                    let wheel_speed = variable.float(data)?;
                    Ok((wheel_speed - speed_metres_per_second)
                        / speed_metres_per_second.abs().max(1.0))
                })
            });
        let wheel_slip = collect_array(wheel_slip)?;
        let tyre_core_temperature_c = collect_array(std::array::from_fn(|index| {
            optional_float(self.tyre_temperature[index], data)
        }))?;
        let suspension_travel_m = collect_array(std::array::from_fn(|index| {
            optional_float(self.shock_deflection[index], data)
        }))?;

        Ok(TelemetrySample {
            packet_id: tick_count,
            speed_kmh: speed_metres_per_second * 3.6,
            rpm: self.rpm.float(data)?.round() as i32,
            gear: self.gear.int(data)?,
            throttle: self.throttle.float(data)?,
            brake: self.brake.float(data)?,
            clutch: self.clutch.float(data)?,
            steering_angle: self.steering_angle.float(data)?,
            fuel_litres: self.fuel.float(data)?,
            acceleration_g: [
                self.acceleration[0].float(data)? / GRAVITY_METRES_PER_SECOND_SQUARED,
                self.acceleration[1].float(data)? / GRAVITY_METRES_PER_SECOND_SQUARED,
                self.acceleration[2].float(data)? / GRAVITY_METRES_PER_SECOND_SQUARED,
            ],
            yaw_rate_rad_s: optional_float(self.yaw_rate, data)?,
            wheel_slip,
            tyre_core_temperature_c,
            suspension_travel_m,
            current_lap_ms: seconds_to_milliseconds(f64::from(self.current_lap_time.float(data)?)),
            last_lap_ms: seconds_to_milliseconds(f64::from(self.last_lap_time.float(data)?)),
            best_lap_ms: seconds_to_milliseconds(f64::from(self.best_lap_time.float(data)?)),
            completed_laps: self.completed_laps.int(data)?,
            position: self.position.int(data)?,
            in_pit: self.on_pit_road.boolean(data)?,
            normalized_car_position: self.lap_distance_percentage.float(data)?,
            session_time_left_s: self.session_time_remaining.double(data)? as f32,
        })
    }
}

fn optional_float(variable: Option<Variable>, data: &[u8]) -> io::Result<f32> {
    variable.map_or(Ok(0.0), |variable| variable.float(data))
}

fn collect_array<const N: usize>(values: [io::Result<f32>; N]) -> io::Result<[f32; N]> {
    let mut output = [0.0; N];
    for (output, value) in output.iter_mut().zip(values) {
        *output = value?;
    }
    Ok(output)
}

fn seconds_to_milliseconds(seconds: f64) -> i32 {
    if !seconds.is_finite() || seconds <= 0.0 {
        return 0;
    }

    (seconds * 1_000.0).round().clamp(0.0, f64::from(i32::MAX)) as i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HeaderLayout {
    num_variables: usize,
    variable_header_offset: usize,
    num_buffers: usize,
    buffer_len: usize,
}

impl HeaderLayout {
    fn parse(header: &Header) -> io::Result<(Self, usize)> {
        if header.version != IRSDK_HEADER_VERSION {
            return Err(invalid_data(format!(
                "unsupported iRacing SDK header version {}",
                header.version
            )));
        }

        let num_variables = positive_len(header.num_variables, "variable count")?;
        if num_variables > MAX_VARIABLES {
            return Err(invalid_data("iRacing variable count is unreasonably large"));
        }
        let num_buffers = positive_len(header.num_buffers, "buffer count")?;
        if num_buffers > IRSDK_MAX_BUFFERS {
            return Err(invalid_data("iRacing buffer count exceeds the SDK limit"));
        }
        let buffer_len = positive_len(header.buffer_len, "buffer length")?;
        if buffer_len > MAX_BUFFER_LEN {
            return Err(invalid_data(
                "iRacing telemetry buffer is unreasonably large",
            ));
        }
        let variable_header_offset = usize::try_from(header.variable_header_offset)
            .map_err(|_| invalid_data("iRacing variable header has a negative offset"))?;

        let variable_headers_len = num_variables
            .checked_mul(size_of::<VariableHeader>())
            .ok_or_else(|| invalid_data("iRacing variable header length overflowed"))?;
        let mut mapping_len = checked_range(
            header.variable_header_offset,
            variable_headers_len,
            "variable headers",
        )?
        .end
        .max(size_of::<Header>());

        mapping_len = mapping_len.max(session_info_range(header)?.end);

        for buffer in header.variable_buffers.iter().take(num_buffers) {
            mapping_len = mapping_len
                .max(checked_range(buffer.buffer_offset, buffer_len, "telemetry buffer")?.end);
        }
        if mapping_len > MAX_MAPPING_LEN {
            return Err(invalid_data("iRacing shared memory is unreasonably large"));
        }

        Ok((
            Self {
                num_variables,
                variable_header_offset,
                num_buffers,
                buffer_len,
            },
            mapping_len,
        ))
    }

    fn matches(self, header: &Header) -> bool {
        usize::try_from(header.num_variables) == Ok(self.num_variables)
            && usize::try_from(header.variable_header_offset) == Ok(self.variable_header_offset)
            && usize::try_from(header.num_buffers) == Ok(self.num_buffers)
            && usize::try_from(header.buffer_len) == Ok(self.buffer_len)
    }
}

fn positive_len(value: i32, label: &str) -> io::Result<usize> {
    let value =
        usize::try_from(value).map_err(|_| invalid_data(format!("iRacing {label} is negative")))?;
    if value == 0 {
        return Err(invalid_data(format!("iRacing {label} is zero")));
    }
    Ok(value)
}

fn checked_range(offset: i32, len: usize, label: &str) -> io::Result<Range<usize>> {
    let start = usize::try_from(offset)
        .map_err(|_| invalid_data(format!("iRacing {label} has a negative offset")))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| invalid_data(format!("iRacing {label} range overflowed")))?;
    Ok(start..end)
}

fn session_info_range(header: &Header) -> io::Result<Range<usize>> {
    let len = usize::try_from(header.session_info_len)
        .map_err(|_| invalid_data("iRacing session info has a negative length"))?;
    if len == 0 {
        return Ok(0..0);
    }
    if len > MAX_SESSION_INFO_LEN {
        return Err(invalid_data("iRacing session info is unreasonably large"));
    }

    checked_range(header.session_info_offset, len, "session info")
}

fn fixed_string(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes[..end]);
    text.into_owned()
}

fn read_bytes<const N: usize>(data: &[u8], offset: usize) -> io::Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| invalid_data("iRacing variable read overflowed"))?;
    data.get(offset..end)
        .ok_or_else(|| invalid_data("iRacing variable read is outside the data buffer"))?
        .try_into()
        .map_err(|_| invalid_data("iRacing variable has an invalid length"))
}

fn latest_buffer_index(header: &Header, num_buffers: usize) -> usize {
    header
        .variable_buffers
        .iter()
        .take(num_buffers)
        .enumerate()
        .max_by_key(|(_, buffer)| buffer.tick_count)
        .map_or(0, |(index, _)| index)
}

fn snapshot_is_stable(before: VariableBuffer, after: VariableBuffer) -> bool {
    before.tick_count == after.tick_count
        && (after.tick_count_begin == 0 || before.tick_count == after.tick_count_begin)
}

fn invalid_data(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

#[cfg(target_os = "windows")]
#[derive(Debug)]
pub(crate) struct IracingTelemetrySource {
    mapping: windows::Mapping,
    layout: HeaderLayout,
    table: VariableTable,
    sample_variables: io::Result<TelemetryVariables>,
    buffer: Vec<u8>,
}

#[cfg(target_os = "windows")]
impl IracingTelemetrySource {
    pub(crate) fn open() -> io::Result<Self> {
        let mapping = windows::Mapping::open(IRSDK_MEMORY_MAPPING, IRSDK_DATA_VALID_EVENT)?;
        let header = mapping.read_header();
        if header.status & IRSDK_CONNECTED == 0 {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "iRacing is not connected to the telemetry session",
            ));
        }

        let (layout, _) = HeaderLayout::parse(&header)?;
        let variable_headers = mapping.read_variable_headers(layout)?;
        let table = VariableTable::parse(&variable_headers, layout.buffer_len)?;
        let sample_variables = TelemetryVariables::resolve(&table);

        Ok(Self {
            mapping,
            layout,
            table,
            sample_variables,
            buffer: vec![0; layout.buffer_len],
        })
    }

    pub(crate) fn read_sample(&mut self) -> io::Result<TelemetrySample> {
        let sample_variables = self
            .sample_variables
            .as_ref()
            .map_err(|error| invalid_data(error.to_string()))?;
        let tick_count = self.mapping.copy_latest(self.layout, &mut self.buffer)?;
        sample_variables.sample(tick_count, &self.buffer)
    }

    pub(crate) fn read_frame(&mut self) -> io::Result<TelemetryFrame> {
        let tick_count = self.mapping.copy_latest(self.layout, &mut self.buffer)?;
        self.table.frame(tick_count, &self.buffer)
    }

    pub(crate) fn read_value(&mut self, name: &str) -> io::Result<TelemetryValue> {
        self.mapping.copy_latest(self.layout, &mut self.buffer)?;
        self.table.value(name, &self.buffer)
    }

    pub(crate) fn read_snapshot(&mut self) -> io::Result<TelemetrySnapshot> {
        let sample_variables = self
            .sample_variables
            .as_ref()
            .map_err(|error| invalid_data(error.to_string()))?;
        let tick_count = self.mapping.copy_latest(self.layout, &mut self.buffer)?;
        let sample = sample_variables.sample(tick_count, &self.buffer)?;
        let frame = self.table.frame(tick_count, &self.buffer)?;
        Ok(TelemetrySnapshot { sample, frame })
    }

    pub(crate) fn variables(&self) -> &[VariableMetadata] {
        self.table.metadata()
    }

    pub(crate) fn session_info(&self) -> io::Result<SessionInfo> {
        let (update_count, raw) = self.mapping.copy_session_info(self.layout)?;
        Ok(SessionInfo::from_raw(update_count, raw))
    }

    pub(crate) fn session_info_update(&self) -> io::Result<i32> {
        self.mapping.session_info_update(self.layout)
    }

    pub(crate) fn wait_for_data(&self, timeout: std::time::Duration) -> io::Result<bool> {
        self.mapping.wait_for_data(timeout)
    }
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub(crate) struct IracingTelemetrySource;

#[cfg(not(target_os = "windows"))]
impl IracingTelemetrySource {
    pub(crate) fn open() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "iRacing shared memory is only available on Windows",
        ))
    }

    pub(crate) fn read_sample(&mut self) -> io::Result<TelemetrySample> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "iRacing shared memory is only available on Windows",
        ))
    }

    pub(crate) fn read_frame(&mut self) -> io::Result<TelemetryFrame> {
        Err(unsupported())
    }

    pub(crate) fn read_value(&mut self, _name: &str) -> io::Result<TelemetryValue> {
        Err(unsupported())
    }

    pub(crate) fn read_snapshot(&mut self) -> io::Result<TelemetrySnapshot> {
        Err(unsupported())
    }

    pub(crate) fn variables(&self) -> &[VariableMetadata] {
        &[]
    }

    pub(crate) fn session_info(&self) -> io::Result<SessionInfo> {
        Err(unsupported())
    }

    pub(crate) fn session_info_update(&self) -> io::Result<i32> {
        Err(unsupported())
    }

    pub(crate) fn wait_for_data(&self, _timeout: std::time::Duration) -> io::Result<bool> {
        Err(unsupported())
    }
}

#[cfg(not(target_os = "windows"))]
fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "iRacing shared memory is only available on Windows",
    )
}

#[cfg(target_os = "windows")]
mod windows {
    use std::{
        fmt, io,
        mem::{MaybeUninit, offset_of, size_of},
        ptr,
        sync::atomic::{Ordering, fence},
        time::Duration,
    };

    use winapi::{
        ctypes::c_void,
        shared::{minwindef::FALSE, winerror::WAIT_TIMEOUT},
        um::{
            handleapi::CloseHandle,
            memoryapi::{FILE_MAP_READ, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile},
            synchapi::{OpenEventW, WaitForSingleObject},
            winbase::{WAIT_FAILED, WAIT_OBJECT_0},
            winnt::{HANDLE, SYNCHRONIZE},
        },
    };

    use super::{
        Header, HeaderLayout, IRSDK_CONNECTED, VariableHeader, invalid_data, latest_buffer_index,
        snapshot_is_stable,
    };

    pub(super) struct Mapping {
        handle: HANDLE,
        data_event: HANDLE,
        view: *const u8,
        len: usize,
    }

    // File-mapping handles and views are process-wide; this owner only performs reads.
    unsafe impl Send for Mapping {}

    impl Mapping {
        pub(super) fn open(name: &str, event_name: &str) -> io::Result<Self> {
            let wide_name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let handle = unsafe { OpenFileMappingW(FILE_MAP_READ, FALSE, wide_name.as_ptr()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }

            let header_view =
                unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, size_of::<Header>()) };
            if header_view.is_null() {
                return Err(close_with_last_error(handle));
            }
            let header = unsafe { ptr::read_volatile(header_view.cast::<Header>()) };
            unsafe {
                UnmapViewOfFile(header_view);
            }

            let (_, mapping_len) = match HeaderLayout::parse(&header) {
                Ok(layout) => layout,
                Err(error) => {
                    unsafe {
                        CloseHandle(handle);
                    }
                    return Err(error);
                },
            };
            let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, mapping_len) };
            if view.is_null() {
                return Err(close_with_last_error(handle));
            }

            let wide_event_name: Vec<u16> = event_name.encode_utf16().chain(Some(0)).collect();
            let data_event = unsafe { OpenEventW(SYNCHRONIZE, FALSE, wide_event_name.as_ptr()) };
            if data_event.is_null() {
                let error = io::Error::last_os_error();
                unsafe {
                    UnmapViewOfFile(view);
                    CloseHandle(handle);
                }
                return Err(error);
            }

            Ok(Self {
                handle,
                data_event,
                view: view.cast::<u8>(),
                len: mapping_len,
            })
        }

        pub(super) fn read_header(&self) -> Header {
            unsafe { ptr::read_volatile(self.view.cast::<Header>()) }
        }

        pub(super) fn read_variable_headers(
            &self,
            layout: HeaderLayout,
        ) -> io::Result<Vec<VariableHeader>> {
            let mut headers = Vec::with_capacity(layout.num_variables);
            for index in 0..layout.num_variables {
                let offset = layout
                    .variable_header_offset
                    .checked_add(index.saturating_mul(size_of::<VariableHeader>()))
                    .ok_or_else(|| invalid_data("iRacing variable header offset overflowed"))?;
                headers.push(self.copy_value(offset)?);
            }
            Ok(headers)
        }

        pub(super) fn copy_latest(
            &self,
            layout: HeaderLayout,
            output: &mut [u8],
        ) -> io::Result<i32> {
            if output.len() != layout.buffer_len {
                return Err(invalid_data(
                    "local iRacing telemetry buffer has the wrong size",
                ));
            }

            for _ in 0..2 {
                let before = self.read_header();
                if before.status & IRSDK_CONNECTED == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::NotConnected,
                        "iRacing telemetry session ended",
                    ));
                }
                if !layout.matches(&before) {
                    return Err(invalid_data("iRacing telemetry layout changed"));
                }

                let latest = latest_buffer_index(&before, layout.num_buffers);
                let variable_buffer = before.variable_buffers[latest];
                if variable_buffer.tick_count < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::WouldBlock,
                        "iRacing has not published a telemetry frame yet",
                    ));
                }
                let offset = usize::try_from(variable_buffer.buffer_offset)
                    .map_err(|_| invalid_data("iRacing telemetry buffer has a negative offset"))?;

                fence(Ordering::SeqCst);
                self.copy_into(offset, output)?;
                fence(Ordering::SeqCst);

                let after = self.read_variable_buffer(latest);
                if snapshot_is_stable(variable_buffer, after) {
                    return Ok(variable_buffer.tick_count);
                }
            }

            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "iRacing updated telemetry while it was being copied",
            ))
        }

        pub(super) fn copy_session_info(&self, layout: HeaderLayout) -> io::Result<(i32, Vec<u8>)> {
            for _ in 0..2 {
                let before = self.read_header();
                if before.status & IRSDK_CONNECTED == 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::NotConnected,
                        "iRacing telemetry session ended",
                    ));
                }
                if !layout.matches(&before) {
                    return Err(invalid_data("iRacing telemetry layout changed"));
                }

                let range = super::session_info_range(&before)?;
                if range.end > self.len {
                    return Err(invalid_data("iRacing session info range is out of bounds"));
                }
                let mut raw = Vec::new();
                raw.try_reserve_exact(range.len()).map_err(|_| {
                    invalid_data("unable to allocate the iRacing session info buffer")
                })?;
                raw.resize(range.len(), 0);

                fence(Ordering::SeqCst);
                self.copy_into(range.start, &mut raw)?;
                fence(Ordering::SeqCst);

                let after = self.read_header();
                if before.session_info_update == after.session_info_update
                    && before.session_info_len == after.session_info_len
                    && before.session_info_offset == after.session_info_offset
                {
                    if let Some(end) = raw.iter().position(|byte| *byte == 0) {
                        raw.truncate(end);
                    }
                    return Ok((before.session_info_update, raw));
                }
            }

            Err(io::Error::new(
                io::ErrorKind::WouldBlock,
                "iRacing updated session info while it was being copied",
            ))
        }

        pub(super) fn session_info_update(&self, layout: HeaderLayout) -> io::Result<i32> {
            let header = self.read_header();
            if header.status & IRSDK_CONNECTED == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "iRacing telemetry session ended",
                ));
            }
            if !layout.matches(&header) {
                return Err(invalid_data("iRacing telemetry layout changed"));
            }
            Ok(header.session_info_update)
        }

        pub(super) fn wait_for_data(&self, timeout: Duration) -> io::Result<bool> {
            let milliseconds = u32::try_from(timeout.as_millis()).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "iRacing event timeout exceeds the Windows limit",
                )
            })?;
            let result = unsafe { WaitForSingleObject(self.data_event, milliseconds) };

            match result {
                WAIT_OBJECT_0 => Ok(true),
                WAIT_TIMEOUT => Ok(false),
                WAIT_FAILED => Err(io::Error::last_os_error()),
                value => Err(io::Error::other(format!(
                    "unexpected iRacing event wait result {value}"
                ))),
            }
        }

        fn read_variable_buffer(&self, index: usize) -> super::VariableBuffer {
            let offset = offset_of!(Header, variable_buffers)
                + index.saturating_mul(size_of::<super::VariableBuffer>());
            unsafe { ptr::read_volatile(self.view.add(offset).cast::<super::VariableBuffer>()) }
        }

        fn copy_value<T: Copy>(&self, offset: usize) -> io::Result<T> {
            let mut output = MaybeUninit::<T>::uninit();
            let bytes = unsafe {
                std::slice::from_raw_parts_mut(output.as_mut_ptr().cast::<u8>(), size_of::<T>())
            };
            self.copy_into(offset, bytes)?;
            Ok(unsafe { output.assume_init() })
        }

        fn copy_into(&self, offset: usize, output: &mut [u8]) -> io::Result<()> {
            let end = offset
                .checked_add(output.len())
                .ok_or_else(|| invalid_data("iRacing shared memory range overflowed"))?;
            if end > self.len {
                return Err(invalid_data("iRacing shared memory range is out of bounds"));
            }

            unsafe {
                ptr::copy_nonoverlapping(self.view.add(offset), output.as_mut_ptr(), output.len());
            }
            Ok(())
        }
    }

    impl fmt::Debug for Mapping {
        fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter
                .debug_struct("Mapping")
                .field("len", &self.len)
                .finish_non_exhaustive()
        }
    }

    impl Drop for Mapping {
        fn drop(&mut self) {
            unsafe {
                CloseHandle(self.data_event);
                UnmapViewOfFile(self.view.cast::<c_void>());
                CloseHandle(self.handle);
            }
        }
    }

    fn close_with_last_error(handle: HANDLE) -> io::Error {
        let error = io::Error::last_os_error();
        unsafe {
            CloseHandle(handle);
        }
        error
    }
}

#[cfg(test)]
mod tests {
    use std::mem::{offset_of, size_of};

    use super::{
        Header, HeaderLayout, MAX_SESSION_INFO_LEN, TelemetryValue, TelemetryVariables, Variable,
        VariableBuffer, VariableHeader, VariableTable, VariableType, latest_buffer_index,
        snapshot_is_stable,
    };

    #[test]
    fn structures_match_the_irsdk_v2_1_20_layout() {
        assert_eq!(size_of::<VariableBuffer>(), 16);
        assert_eq!(offset_of!(VariableBuffer, tick_count), 0);
        assert_eq!(offset_of!(VariableBuffer, buffer_offset), 4);
        assert_eq!(offset_of!(VariableBuffer, tick_count_begin), 8);
        assert_eq!(size_of::<VariableHeader>(), 144);
        assert_eq!(offset_of!(VariableHeader, name), 16);
        assert_eq!(offset_of!(VariableHeader, description), 48);
        assert_eq!(offset_of!(VariableHeader, unit), 112);
        assert_eq!(size_of::<Header>(), 112);
        assert_eq!(offset_of!(Header, current_buffer_tick_count), 40);
        assert_eq!(offset_of!(Header, current_buffer), 44);
        assert_eq!(offset_of!(Header, variable_buffers), 48);
    }

    #[test]
    fn selects_the_newest_published_buffer_for_every_header_variant() {
        let header = Header {
            current_buffer: 2,
            variable_buffers: [
                VariableBuffer {
                    tick_count: 41,
                    ..VariableBuffer::default()
                },
                VariableBuffer {
                    tick_count: 44,
                    ..VariableBuffer::default()
                },
                VariableBuffer {
                    tick_count: 43,
                    ..VariableBuffer::default()
                },
                VariableBuffer::default(),
            ],
            ..Header::default()
        };
        assert_eq!(latest_buffer_index(&header, 3), 1);
    }

    #[test]
    fn detects_a_write_that_overlaps_the_snapshot_copy() {
        let before = VariableBuffer {
            tick_count: 42,
            tick_count_begin: 42,
            ..VariableBuffer::default()
        };
        assert!(snapshot_is_stable(before, before));

        let after_writer_started = VariableBuffer {
            tick_count_begin: 45,
            ..before
        };
        assert!(!snapshot_is_stable(before, after_writer_started));

        let legacy = VariableBuffer {
            tick_count: 42,
            tick_count_begin: 0,
            ..VariableBuffer::default()
        };
        assert!(snapshot_is_stable(legacy, legacy));
        assert!(!snapshot_is_stable(
            legacy,
            VariableBuffer {
                tick_count: 43,
                ..legacy
            }
        ));
    }

    #[test]
    fn converts_every_sdk_scalar_and_array_type() {
        assert_eq!(
            value(VariableType::Char, 1, &[255]),
            TelemetryValue::Char(255)
        );
        assert_eq!(
            value(VariableType::Char, 2, &[1, 2]),
            TelemetryValue::Chars(Box::new([1, 2]))
        );
        assert_eq!(
            value(VariableType::Bool, 1, &[1]),
            TelemetryValue::Bool(true)
        );
        assert_eq!(
            value(VariableType::Bool, 2, &[0, 1]),
            TelemetryValue::Bools(Box::new([false, true]))
        );

        let ints = [1_i32.to_le_bytes(), (-2_i32).to_le_bytes()].concat();
        assert_eq!(
            value(VariableType::Int, 1, &ints[..4]),
            TelemetryValue::Int(1)
        );
        assert_eq!(
            value(VariableType::Int, 2, &ints),
            TelemetryValue::Ints(Box::new([1, -2]))
        );

        let bit_fields = [1_u32.to_le_bytes(), 0x8000_0000_u32.to_le_bytes()].concat();
        assert_eq!(
            value(VariableType::BitField, 1, &bit_fields[..4]),
            TelemetryValue::BitField(1)
        );
        assert_eq!(
            value(VariableType::BitField, 2, &bit_fields),
            TelemetryValue::BitFields(Box::new([1, 0x8000_0000]))
        );

        let floats = [1.25_f32.to_le_bytes(), (-2.5_f32).to_le_bytes()].concat();
        assert_eq!(
            value(VariableType::Float, 1, &floats[..4]),
            TelemetryValue::Float(1.25)
        );
        assert_eq!(
            value(VariableType::Float, 2, &floats),
            TelemetryValue::Floats(Box::new([1.25, -2.5]))
        );

        let doubles = [3.5_f64.to_le_bytes(), (-4.75_f64).to_le_bytes()].concat();
        assert_eq!(
            value(VariableType::Double, 1, &doubles[..8]),
            TelemetryValue::Double(3.5)
        );
        assert_eq!(
            value(VariableType::Double, 2, &doubles),
            TelemetryValue::Doubles(Box::new([3.5, -4.75]))
        );
    }

    #[test]
    fn preserves_variable_metadata_in_full_frames() {
        let mut header = VariableHeader {
            variable_type: VariableType::Float as i32,
            count: 2,
            count_as_time: 1,
            ..VariableHeader::default()
        };
        copy_fixed(&mut header.name, "CarIdxLapDistPct");
        copy_fixed(&mut header.description, "Distance around track");
        copy_fixed(&mut header.unit, "%");
        let data = [0.25_f32.to_le_bytes(), 0.75_f32.to_le_bytes()].concat();

        let table = VariableTable::parse(&[header], data.len()).expect("valid variable table");
        assert_eq!(table.metadata().len(), 1);
        let metadata = &table.metadata()[0];
        assert_eq!(metadata.name, "CarIdxLapDistPct");
        assert_eq!(metadata.description, "Distance around track");
        assert_eq!(metadata.unit, "%");
        assert_eq!(metadata.value_type, VariableType::Float);
        assert_eq!(metadata.count, 2);
        assert!(metadata.count_as_time);

        let frame = table.frame(88, &data).expect("valid full frame");
        assert_eq!(frame.packet_id(), 88);
        assert_eq!(
            frame.value("CarIdxLapDistPct"),
            Some(&TelemetryValue::Floats(Box::new([0.25, 0.75])))
        );
        assert_eq!(
            table
                .value("MissingVariable", &data)
                .expect_err("unknown variable")
                .kind(),
            std::io::ErrorKind::NotFound
        );
    }

    #[test]
    fn validates_header_ranges_and_computes_mapping_length() {
        let header = Header {
            version: 2,
            num_variables: 1,
            variable_header_offset: 112,
            num_buffers: 3,
            buffer_len: 256,
            variable_buffers: [
                VariableBuffer {
                    buffer_offset: 256,
                    ..VariableBuffer::default()
                },
                VariableBuffer {
                    buffer_offset: 512,
                    ..VariableBuffer::default()
                },
                VariableBuffer {
                    buffer_offset: 768,
                    ..VariableBuffer::default()
                },
                VariableBuffer::default(),
            ],
            ..Header::default()
        };

        let (layout, mapping_len) = HeaderLayout::parse(&header).expect("valid layout");
        assert_eq!(layout.buffer_len, 256);
        assert_eq!(mapping_len, 1_024);

        let oversized_session_info = Header {
            session_info_len: i32::try_from(MAX_SESSION_INFO_LEN + 1).expect("test size fits i32"),
            session_info_offset: 1_024,
            ..header
        };
        let error = HeaderLayout::parse(&oversized_session_info)
            .expect_err("oversized session info must be rejected");
        assert!(
            error
                .to_string()
                .contains("session info is unreasonably large")
        );
    }

    #[test]
    fn converts_irsdk_variables_to_desktop_units() {
        let mut headers = Vec::new();
        let mut data = Vec::new();

        add_f32(&mut headers, &mut data, "Speed", 50.0);
        add_f32(&mut headers, &mut data, "RPM", 6_543.6);
        add_i32(&mut headers, &mut data, "Gear", 3);
        add_f32(&mut headers, &mut data, "Throttle", 0.8);
        add_f32(&mut headers, &mut data, "Brake", 0.2);
        add_f32(&mut headers, &mut data, "Clutch", 0.1);
        add_f32(&mut headers, &mut data, "SteeringWheelAngle", -0.25);
        add_f32(&mut headers, &mut data, "FuelLevel", 42.5);
        add_f32(&mut headers, &mut data, "LatAccel", 9.806_65);
        add_f32(&mut headers, &mut data, "LongAccel", -19.613_3);
        add_f32(&mut headers, &mut data, "VertAccel", 4.903_325);
        add_f32(&mut headers, &mut data, "YawRate", 0.5);
        for (name, value) in [
            ("LFspeed", 51.0),
            ("RFspeed", 49.0),
            ("LRspeed", 50.0),
            ("RRspeed", 52.0),
            ("LFtempCM", 80.0),
            ("RFtempCM", 81.0),
            ("LRtempCM", 82.0),
            ("RRtempCM", 83.0),
            ("LFshockDefl", 0.01),
            ("RFshockDefl", 0.02),
            ("LRshockDefl", 0.03),
            ("RRshockDefl", 0.04),
        ] {
            add_f32(&mut headers, &mut data, name, value);
        }
        add_f32(&mut headers, &mut data, "LapCurrentLapTime", 42.125);
        add_f32(&mut headers, &mut data, "LapLastLapTime", 91.25);
        add_f32(&mut headers, &mut data, "LapBestLapTime", 90.875);
        add_i32(&mut headers, &mut data, "LapCompleted", 7);
        add_i32(&mut headers, &mut data, "PlayerCarPosition", 4);
        add_bool(&mut headers, &mut data, "OnPitRoad", true);
        add_f32(&mut headers, &mut data, "LapDistPct", 0.625);
        add_f64(&mut headers, &mut data, "SessionTimeRemain", 512.25);

        let table = VariableTable::parse(&headers, data.len()).expect("valid variable table");
        let variables = TelemetryVariables::resolve(&table).expect("required variables");
        let sample = variables.sample(1234, &data).expect("valid sample");

        assert_eq!(sample.packet_id, 1234);
        assert_eq!(sample.speed_kmh, 180.0);
        assert_eq!(sample.rpm, 6_544);
        assert_eq!(sample.gear, 3);
        assert_eq!(sample.acceleration_g, [1.0, -2.0, 0.5]);
        assert_eq!(sample.yaw_rate_rad_s, 0.5);
        assert_eq!(sample.wheel_slip, [0.02, -0.02, 0.0, 0.04]);
        assert_eq!(sample.tyre_core_temperature_c, [80.0, 81.0, 82.0, 83.0]);
        assert_eq!(sample.suspension_travel_m, [0.01, 0.02, 0.03, 0.04]);
        assert_eq!(sample.current_lap_ms, 42_125);
        assert_eq!(sample.last_lap_ms, 91_250);
        assert_eq!(sample.best_lap_ms, 90_875);
        assert!(sample.in_pit);
        assert_eq!(sample.session_time_left_s, 512.25);
    }

    #[test]
    fn rejects_sample_variables_with_the_wrong_primitive_type() {
        let mut headers = Vec::new();
        let mut data = Vec::new();
        add_i32(&mut headers, &mut data, "Speed", 50);

        let table = VariableTable::parse(&headers, data.len()).expect("valid variable table");
        let error = TelemetryVariables::resolve(&table).expect_err("Speed must be a float");

        assert!(error.to_string().contains("expected a scalar float"));
    }

    fn add_f32(headers: &mut Vec<VariableHeader>, data: &mut Vec<u8>, name: &str, value: f32) {
        add_variable(
            headers,
            data,
            name,
            VariableType::Float,
            &value.to_le_bytes(),
        );
    }

    fn add_f64(headers: &mut Vec<VariableHeader>, data: &mut Vec<u8>, name: &str, value: f64) {
        add_variable(
            headers,
            data,
            name,
            VariableType::Double,
            &value.to_le_bytes(),
        );
    }

    fn add_i32(headers: &mut Vec<VariableHeader>, data: &mut Vec<u8>, name: &str, value: i32) {
        add_variable(headers, data, name, VariableType::Int, &value.to_le_bytes());
    }

    fn add_bool(headers: &mut Vec<VariableHeader>, data: &mut Vec<u8>, name: &str, value: bool) {
        add_variable(headers, data, name, VariableType::Bool, &[u8::from(value)]);
    }

    fn add_variable(
        headers: &mut Vec<VariableHeader>,
        data: &mut Vec<u8>,
        name: &str,
        variable_type: VariableType,
        bytes: &[u8],
    ) {
        let mut header = VariableHeader {
            variable_type: variable_type as i32,
            offset: i32::try_from(data.len()).expect("test buffer offset fits i32"),
            count: 1,
            ..VariableHeader::default()
        };
        header.name[..name.len()].copy_from_slice(name.as_bytes());
        headers.push(header);
        data.extend_from_slice(bytes);
    }

    fn value(variable_type: VariableType, count: usize, data: &[u8]) -> TelemetryValue {
        Variable {
            variable_type,
            offset: 0,
            count,
        }
        .value(data)
        .expect("valid value")
    }

    fn copy_fixed<const N: usize>(target: &mut [u8; N], value: &str) {
        target[..value.len()].copy_from_slice(value.as_bytes());
    }
}
