use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    mem::size_of,
    ops::Range,
    path::Path,
};

use crate::{
    SessionInfo, TelemetryFrame, TelemetrySample, TelemetrySnapshot, TelemetryValue,
    VariableMetadata,
};

use super::{
    Header, HeaderLayout, IRSDK_MAX_BUFFERS, TelemetryVariables, VariableBuffer, VariableHeader,
    VariableTable, checked_range, invalid_data, read_bytes, session_info_range,
};

const DISK_HEADER_LEN: usize = 32;

/// Summary values stored in the disk-only portion of an iRacing telemetry file.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IbtMetadata {
    pub session_start_unix_seconds: i64,
    pub session_start_time_seconds: f64,
    pub session_end_time_seconds: f64,
    pub lap_count: usize,
    pub record_count: usize,
    pub tick_rate: u32,
    pub record_len: usize,
}

impl IbtMetadata {
    pub fn duration_seconds(self) -> f64 {
        self.session_end_time_seconds - self.session_start_time_seconds
    }
}

/// A lazily decoded `.ibt` telemetry source backed by any seekable reader.
#[derive(Debug)]
pub struct IbtReader<R> {
    reader: R,
    metadata: IbtMetadata,
    session_info: SessionInfo,
    table: VariableTable,
    sample_variables: Result<TelemetryVariables, String>,
    record_offset: u64,
    record_buffer: Vec<u8>,
}

/// An `.ibt` reader backed by a buffered file.
pub type IbtFile = IbtReader<BufReader<File>>;

impl IbtReader<BufReader<File>> {
    pub fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        Self::new(BufReader::new(File::open(path)?))
    }
}

impl<R: Read + Seek> IbtReader<R> {
    pub fn new(mut reader: R) -> io::Result<Self> {
        reader.seek(SeekFrom::Start(0))?;
        let header = read_header(&mut reader)?;
        let (layout, _) = HeaderLayout::parse(&header)?;
        if layout.num_buffers != 1 {
            return Err(invalid_data(format!(
                "an IBT file must contain one telemetry buffer, found {}",
                layout.num_buffers
            )));
        }

        let tick_rate = u32::try_from(header.tick_rate)
            .ok()
            .filter(|tick_rate| *tick_rate > 0)
            .ok_or_else(|| invalid_data("IBT tick rate must be positive"))?;
        let disk_header = read_disk_header(&mut reader)?;
        validate_times(disk_header.start_time, disk_header.end_time)?;

        let lap_count = nonnegative_len(disk_header.lap_count, "lap count")?;
        let record_count = nonnegative_len(disk_header.record_count, "record count")?;
        let record_offset = u64::try_from(header.variable_buffers[0].buffer_offset)
            .map_err(|_| invalid_data("IBT record data has a negative offset"))?;

        let variable_headers_range = checked_range(
            header.variable_header_offset,
            layout
                .num_variables
                .checked_mul(size_of::<VariableHeader>())
                .ok_or_else(|| invalid_data("IBT variable header length overflowed"))?,
            "variable headers",
        )?;
        let session_range = session_info_range(&header)?;
        let metadata_end = size_of::<Header>()
            .checked_add(DISK_HEADER_LEN)
            .ok_or_else(|| invalid_data("IBT metadata length overflowed"))?
            .max(variable_headers_range.end)
            .max(session_range.end);
        if record_offset
            < u64::try_from(metadata_end)
                .map_err(|_| invalid_data("IBT metadata offset does not fit in a file"))?
        {
            return Err(invalid_data("IBT record data overlaps file metadata"));
        }

        let file_len = reader.seek(SeekFrom::End(0))?;
        validate_range(variable_headers_range.clone(), file_len, "variable headers")?;
        validate_range(session_range.clone(), file_len, "session info")?;
        let records_len = layout
            .buffer_len
            .checked_mul(record_count)
            .ok_or_else(|| invalid_data("IBT record data length overflowed"))?;
        let record_end = record_offset
            .checked_add(
                u64::try_from(records_len)
                    .map_err(|_| invalid_data("IBT record data length does not fit in a file"))?,
            )
            .ok_or_else(|| invalid_data("IBT record data range overflowed"))?;
        if record_end > file_len {
            return Err(invalid_data(
                "IBT record data extends beyond the end of the file",
            ));
        }

        let variable_headers =
            read_variable_headers(&mut reader, variable_headers_range, layout.num_variables)?;
        let table = VariableTable::parse(&variable_headers, layout.buffer_len)?;
        let sample_variables =
            TelemetryVariables::resolve(&table).map_err(|error| error.to_string());
        let session_info = read_session_info(&mut reader, &header, session_range)?;

        Ok(Self {
            reader,
            metadata: IbtMetadata {
                session_start_unix_seconds: disk_header.start_date,
                session_start_time_seconds: disk_header.start_time,
                session_end_time_seconds: disk_header.end_time,
                lap_count,
                record_count,
                tick_rate,
                record_len: layout.buffer_len,
            },
            session_info,
            table,
            sample_variables,
            record_offset,
            record_buffer: vec![0; layout.buffer_len],
        })
    }

    pub const fn metadata(&self) -> &IbtMetadata {
        &self.metadata
    }

    pub const fn session_info(&self) -> &SessionInfo {
        &self.session_info
    }

    pub fn variables(&self) -> &[VariableMetadata] {
        self.table.metadata()
    }

    pub const fn len(&self) -> usize {
        self.metadata.record_count
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn read_frame(&mut self, index: usize) -> io::Result<TelemetryFrame> {
        let packet_id = self.read_record(index)?;
        self.table.frame(packet_id, &self.record_buffer)
    }

    pub fn read_sample(&mut self, index: usize) -> io::Result<TelemetrySample> {
        let packet_id = self.read_record(index)?;
        let sample_variables = self
            .sample_variables
            .as_ref()
            .map_err(|error| invalid_data(error.clone()))?;
        sample_variables.sample(packet_id, &self.record_buffer)
    }

    pub fn read_snapshot(&mut self, index: usize) -> io::Result<TelemetrySnapshot> {
        let packet_id = self.read_record(index)?;
        let sample_variables = self
            .sample_variables
            .as_ref()
            .map_err(|error| invalid_data(error.clone()))?;
        let sample = sample_variables.sample(packet_id, &self.record_buffer)?;
        let frame = self.table.frame(packet_id, &self.record_buffer)?;
        Ok(TelemetrySnapshot { sample, frame })
    }

    pub fn read_value(&mut self, index: usize, name: &str) -> io::Result<TelemetryValue> {
        self.read_record(index)?;
        self.table.value(name, &self.record_buffer)
    }

    pub fn frames(&mut self) -> IbtFrames<'_, R> {
        IbtFrames {
            source: self,
            next_index: 0,
        }
    }

    pub fn snapshots(&mut self) -> IbtSnapshots<'_, R> {
        IbtSnapshots {
            source: self,
            next_index: 0,
        }
    }

    fn read_record(&mut self, index: usize) -> io::Result<i32> {
        if index >= self.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("IBT record index {index} is out of range"),
            ));
        }
        let relative_offset = self
            .metadata
            .record_len
            .checked_mul(index)
            .ok_or_else(|| invalid_data("IBT record offset overflowed"))?;
        let offset = self
            .record_offset
            .checked_add(
                u64::try_from(relative_offset)
                    .map_err(|_| invalid_data("IBT record offset does not fit in a file"))?,
            )
            .ok_or_else(|| invalid_data("IBT record offset overflowed"))?;
        self.reader.seek(SeekFrom::Start(offset))?;
        self.reader.read_exact(&mut self.record_buffer)?;
        i32::try_from(index).map_err(|_| invalid_data("IBT record index exceeds the SDK limit"))
    }
}

#[derive(Debug)]
pub struct IbtFrames<'a, R> {
    source: &'a mut IbtReader<R>,
    next_index: usize,
}

impl<R: Read + Seek> Iterator for IbtFrames<'_, R> {
    type Item = io::Result<TelemetryFrame>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.source.len() {
            return None;
        }
        let index = self.next_index;
        self.next_index += 1;
        Some(self.source.read_frame(index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.source.len().saturating_sub(self.next_index);
        (remaining, Some(remaining))
    }
}

impl<R: Read + Seek> ExactSizeIterator for IbtFrames<'_, R> {}

#[derive(Debug)]
pub struct IbtSnapshots<'a, R> {
    source: &'a mut IbtReader<R>,
    next_index: usize,
}

impl<R: Read + Seek> Iterator for IbtSnapshots<'_, R> {
    type Item = io::Result<TelemetrySnapshot>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.source.len() {
            return None;
        }
        let index = self.next_index;
        self.next_index += 1;
        Some(self.source.read_snapshot(index))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.source.len().saturating_sub(self.next_index);
        (remaining, Some(remaining))
    }
}

impl<R: Read + Seek> ExactSizeIterator for IbtSnapshots<'_, R> {}

#[derive(Debug, Clone, Copy)]
struct DiskHeader {
    start_date: i64,
    start_time: f64,
    end_time: f64,
    lap_count: i32,
    record_count: i32,
}

fn read_header(reader: &mut impl Read) -> io::Result<Header> {
    let mut bytes = [0; size_of::<Header>()];
    reader.read_exact(&mut bytes)?;

    let mut variable_buffers = [VariableBuffer::default(); IRSDK_MAX_BUFFERS];
    for (index, buffer) in variable_buffers.iter_mut().enumerate() {
        let offset = 48 + index * size_of::<VariableBuffer>();
        *buffer = VariableBuffer {
            tick_count: read_i32(&bytes, offset)?,
            buffer_offset: read_i32(&bytes, offset + 4)?,
            tick_count_begin: read_i32(&bytes, offset + 8)?,
            pad: [read_i32(&bytes, offset + 12)?],
        };
    }

    Ok(Header {
        version: read_i32(&bytes, 0)?,
        status: read_i32(&bytes, 4)?,
        tick_rate: read_i32(&bytes, 8)?,
        session_info_update: read_i32(&bytes, 12)?,
        session_info_len: read_i32(&bytes, 16)?,
        session_info_offset: read_i32(&bytes, 20)?,
        num_variables: read_i32(&bytes, 24)?,
        variable_header_offset: read_i32(&bytes, 28)?,
        num_buffers: read_i32(&bytes, 32)?,
        buffer_len: read_i32(&bytes, 36)?,
        current_buffer_tick_count: read_i32(&bytes, 40)?,
        current_buffer: bytes[44],
        pad: read_bytes(&bytes, 45)?,
        variable_buffers,
    })
}

fn read_disk_header(reader: &mut (impl Read + Seek)) -> io::Result<DiskHeader> {
    reader.seek(SeekFrom::Start(
        u64::try_from(size_of::<Header>())
            .map_err(|_| invalid_data("IBT header size does not fit in a file"))?,
    ))?;
    let mut bytes = [0; DISK_HEADER_LEN];
    reader.read_exact(&mut bytes)?;

    Ok(DiskHeader {
        start_date: read_i64(&bytes, 0)?,
        start_time: read_f64(&bytes, 8)?,
        end_time: read_f64(&bytes, 16)?,
        lap_count: read_i32(&bytes, 24)?,
        record_count: read_i32(&bytes, 28)?,
    })
}

fn read_variable_headers(
    reader: &mut (impl Read + Seek),
    range: Range<usize>,
    count: usize,
) -> io::Result<Vec<VariableHeader>> {
    reader.seek(SeekFrom::Start(u64::try_from(range.start).map_err(
        |_| invalid_data("IBT variable header offset does not fit in a file"),
    )?))?;
    let mut headers = Vec::with_capacity(count);
    for _ in 0..count {
        let mut bytes = [0; size_of::<VariableHeader>()];
        reader.read_exact(&mut bytes)?;
        headers.push(VariableHeader {
            variable_type: read_i32(&bytes, 0)?,
            offset: read_i32(&bytes, 4)?,
            count: read_i32(&bytes, 8)?,
            count_as_time: bytes[12],
            pad: read_bytes(&bytes, 13)?,
            name: read_bytes(&bytes, 16)?,
            description: read_bytes(&bytes, 48)?,
            unit: read_bytes(&bytes, 112)?,
        });
    }
    Ok(headers)
}

fn read_session_info(
    reader: &mut (impl Read + Seek),
    header: &Header,
    range: Range<usize>,
) -> io::Result<SessionInfo> {
    reader.seek(SeekFrom::Start(u64::try_from(range.start).map_err(
        |_| invalid_data("IBT session info offset does not fit in a file"),
    )?))?;
    let mut raw = vec![0; range.len()];
    reader.read_exact(&mut raw)?;
    if let Some(end) = raw.iter().position(|byte| *byte == 0) {
        raw.truncate(end);
    }
    Ok(SessionInfo::from_raw(header.session_info_update, raw))
}

fn validate_range(range: Range<usize>, file_len: u64, label: &str) -> io::Result<()> {
    let end = u64::try_from(range.end)
        .map_err(|_| invalid_data(format!("IBT {label} range does not fit in a file")))?;
    if end > file_len {
        return Err(invalid_data(format!(
            "IBT {label} extends beyond the end of the file"
        )));
    }
    Ok(())
}

fn validate_times(start: f64, end: f64) -> io::Result<()> {
    if !start.is_finite() || !end.is_finite() {
        return Err(invalid_data("IBT session times must be finite"));
    }
    if end < start {
        return Err(invalid_data("IBT session end time precedes its start time"));
    }
    Ok(())
}

fn nonnegative_len(value: i32, label: &str) -> io::Result<usize> {
    usize::try_from(value).map_err(|_| invalid_data(format!("IBT {label} is negative")))
}

fn read_i32(bytes: &[u8], offset: usize) -> io::Result<i32> {
    Ok(i32::from_le_bytes(read_bytes(bytes, offset)?))
}

fn read_i64(bytes: &[u8], offset: usize) -> io::Result<i64> {
    Ok(i64::from_le_bytes(read_bytes(bytes, offset)?))
}

fn read_f64(bytes: &[u8], offset: usize) -> io::Result<f64> {
    Ok(f64::from_le_bytes(read_bytes(bytes, offset)?))
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, mem::size_of};

    use crate::TelemetryValue;

    use super::IbtReader;
    use crate::shared_memory::{Header, VariableHeader};

    const VARIABLE_HEADER_OFFSET: usize = 144;
    const SESSION_INFO: &[u8] = b"WeekendInfo:\n  TrackName: test\n";
    const VARIABLE_COUNT: usize = 3;
    const RECORD_LEN: usize = 20;
    const RECORD_COUNT: usize = 2;

    #[test]
    fn reads_metadata_session_info_and_random_records() {
        let mut source = IbtReader::new(Cursor::new(fixture())).expect("valid IBT fixture");

        assert_eq!(source.metadata().tick_rate, 60);
        assert_eq!(source.metadata().lap_count, 1);
        assert_eq!(source.metadata().record_count, RECORD_COUNT);
        assert!((source.metadata().duration_seconds() - 1.0 / 60.0).abs() < 1e-12);
        assert_eq!(source.variables().len(), VARIABLE_COUNT);
        assert_eq!(
            source
                .session_info()
                .parse()
                .expect("valid session YAML")
                .weekend_info
                .expect("WeekendInfo")
                .track_name
                .as_deref(),
            Some("test")
        );

        let frame = source.read_frame(1).expect("second frame");
        assert_eq!(frame.packet_id(), 1);
        assert_eq!(frame.value("RPM"), Some(&TelemetryValue::Float(6_100.0)));
        assert_eq!(
            frame.value("CarIdxLap"),
            Some(&TelemetryValue::Ints(Box::new([3, 4])))
        );
        assert_eq!(
            source.read_value(0, "SessionTime").expect("session time"),
            TelemetryValue::Double(10.0)
        );
    }

    #[test]
    fn iterates_over_all_frames_without_preloading_them() {
        let mut source = IbtReader::new(Cursor::new(fixture())).expect("valid IBT fixture");
        let rpms = source
            .frames()
            .map(|frame| match frame.expect("frame").value("RPM") {
                Some(TelemetryValue::Float(rpm)) => *rpm,
                value => panic!("unexpected RPM value: {value:?}"),
            })
            .collect::<Vec<_>>();

        assert_eq!(rpms, [5_000.0, 6_100.0]);
    }

    #[test]
    fn rejects_truncated_record_data() {
        let mut bytes = fixture();
        bytes.pop();

        let error = IbtReader::new(Cursor::new(bytes)).expect_err("truncated IBT must fail");
        assert!(error.to_string().contains("record data extends beyond"));
    }

    #[test]
    fn rejects_unknown_variable_types() {
        let mut bytes = fixture();
        write_i32(&mut bytes, VARIABLE_HEADER_OFFSET, 99);

        let error = IbtReader::new(Cursor::new(bytes)).expect_err("unknown type must fail");
        assert!(error.to_string().contains("unknown iRacing variable type"));
    }

    #[test]
    fn rejects_record_indexes_outside_the_file() {
        let mut source = IbtReader::new(Cursor::new(fixture())).expect("valid IBT fixture");

        let error = source
            .read_frame(RECORD_COUNT)
            .expect_err("index must fail");
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    fn fixture() -> Vec<u8> {
        let variable_headers_len = VARIABLE_COUNT * size_of::<VariableHeader>();
        let session_info_offset = VARIABLE_HEADER_OFFSET + variable_headers_len;
        let record_offset = align(session_info_offset + SESSION_INFO.len(), 16);
        let mut bytes = vec![0; record_offset + RECORD_LEN * RECORD_COUNT];

        write_i32(&mut bytes, 0, 2);
        write_i32(&mut bytes, 4, 1);
        write_i32(&mut bytes, 8, 60);
        write_i32(&mut bytes, 12, 7);
        write_i32(&mut bytes, 16, SESSION_INFO.len() as i32);
        write_i32(&mut bytes, 20, session_info_offset as i32);
        write_i32(&mut bytes, 24, VARIABLE_COUNT as i32);
        write_i32(&mut bytes, 28, VARIABLE_HEADER_OFFSET as i32);
        write_i32(&mut bytes, 32, 1);
        write_i32(&mut bytes, 36, RECORD_LEN as i32);
        write_i32(&mut bytes, 52, record_offset as i32);

        let disk = size_of::<Header>();
        write_i64(&mut bytes, disk, 1_750_000_000);
        write_f64(&mut bytes, disk + 8, 10.0);
        write_f64(&mut bytes, disk + 16, 10.0 + 1.0 / 60.0);
        write_i32(&mut bytes, disk + 24, 1);
        write_i32(&mut bytes, disk + 28, RECORD_COUNT as i32);

        write_variable_header(&mut bytes, 0, 5, 0, 1, "SessionTime", "s");
        write_variable_header(&mut bytes, 1, 4, 8, 1, "RPM", "revs/min");
        write_variable_header(&mut bytes, 2, 2, 12, 2, "CarIdxLap", "");
        bytes[session_info_offset..session_info_offset + SESSION_INFO.len()]
            .copy_from_slice(SESSION_INFO);

        write_record(
            &mut bytes[record_offset..record_offset + RECORD_LEN],
            10.0,
            5_000.0,
            [2, 3],
        );
        write_record(
            &mut bytes[record_offset + RECORD_LEN..record_offset + RECORD_LEN * 2],
            10.0 + 1.0 / 60.0,
            6_100.0,
            [3, 4],
        );
        bytes
    }

    fn write_variable_header(
        bytes: &mut [u8],
        index: usize,
        variable_type: i32,
        value_offset: i32,
        count: i32,
        name: &str,
        unit: &str,
    ) {
        let offset = VARIABLE_HEADER_OFFSET + index * size_of::<VariableHeader>();
        write_i32(bytes, offset, variable_type);
        write_i32(bytes, offset + 4, value_offset);
        write_i32(bytes, offset + 8, count);
        copy_string(bytes, offset + 16, 32, name);
        copy_string(bytes, offset + 48, 64, name);
        copy_string(bytes, offset + 112, 32, unit);
    }

    fn write_record(bytes: &mut [u8], session_time: f64, rpm: f32, laps: [i32; 2]) {
        bytes[0..8].copy_from_slice(&session_time.to_le_bytes());
        bytes[8..12].copy_from_slice(&rpm.to_le_bytes());
        bytes[12..16].copy_from_slice(&laps[0].to_le_bytes());
        bytes[16..20].copy_from_slice(&laps[1].to_le_bytes());
    }

    fn write_i32(bytes: &mut [u8], offset: usize, value: i32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_i64(bytes: &mut [u8], offset: usize, value: i64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn write_f64(bytes: &mut [u8], offset: usize, value: f64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn copy_string(bytes: &mut [u8], offset: usize, len: usize, value: &str) {
        let value = value.as_bytes();
        let len = value.len().min(len);
        bytes[offset..offset + len].copy_from_slice(&value[..len]);
    }

    const fn align(value: usize, alignment: usize) -> usize {
        value.div_ceil(alignment) * alignment
    }
}
