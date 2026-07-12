use core::fmt;

use crate::TelemetrySample;

const MAGIC: [u8; 4] = *b"CHIA";
const VERSION: u8 = 1;
const HEADER_LEN: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    Truncated,
    InvalidMagic,
    UnsupportedVersion(u8),
    InvalidLength,
    NonFiniteValue,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Truncated => formatter.write_str("telemetry packet is truncated"),
            Self::InvalidMagic => formatter.write_str("telemetry packet has invalid magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported telemetry packet version {version}")
            },
            Self::InvalidLength => formatter.write_str("telemetry packet has invalid length"),
            Self::NonFiniteValue => {
                formatter.write_str("telemetry packet contains non-finite data")
            },
        }
    }
}

impl std::error::Error for DecodeError {}

pub fn encode(sample: &TelemetrySample) -> Vec<u8> {
    let mut payload = Vec::with_capacity(160);

    push_i32(&mut payload, sample.packet_id);
    push_f32(&mut payload, sample.speed_kmh);
    push_i32(&mut payload, sample.rpm);
    push_i32(&mut payload, sample.gear);
    push_f32(&mut payload, sample.throttle);
    push_f32(&mut payload, sample.brake);
    push_f32(&mut payload, sample.clutch);
    push_f32(&mut payload, sample.steering_angle);
    push_f32(&mut payload, sample.fuel_litres);
    push_f32_array(&mut payload, sample.acceleration_g);
    push_f32_array(&mut payload, sample.wheel_slip);
    push_f32_array(&mut payload, sample.tyre_core_temperature_c);
    push_f32_array(&mut payload, sample.suspension_travel_m);
    push_i32(&mut payload, sample.current_lap_ms);
    push_i32(&mut payload, sample.last_lap_ms);
    push_i32(&mut payload, sample.best_lap_ms);
    push_i32(&mut payload, sample.completed_laps);
    push_i32(&mut payload, sample.position);
    payload.push(u8::from(sample.in_pit));
    push_f32(&mut payload, sample.normalized_car_position);
    push_f32(&mut payload, sample.session_time_left_s);

    let payload_len = u16::try_from(payload.len()).expect("telemetry payload length fits u16");
    let mut packet = Vec::with_capacity(HEADER_LEN + payload.len());
    packet.extend_from_slice(&MAGIC);
    packet.push(VERSION);
    packet.extend_from_slice(&payload_len.to_le_bytes());
    packet.extend_from_slice(&payload);
    packet
}

pub fn decode(packet: &[u8]) -> Result<TelemetrySample, DecodeError> {
    if packet.len() < HEADER_LEN {
        return Err(DecodeError::Truncated);
    }
    if packet[..MAGIC.len()] != MAGIC {
        return Err(DecodeError::InvalidMagic);
    }
    if packet[4] != VERSION {
        return Err(DecodeError::UnsupportedVersion(packet[4]));
    }

    let payload_len = usize::from(u16::from_le_bytes([packet[5], packet[6]]));
    if packet.len() != HEADER_LEN + payload_len {
        return Err(DecodeError::InvalidLength);
    }

    let mut reader = Reader::new(&packet[HEADER_LEN..]);
    let sample = TelemetrySample {
        packet_id: reader.i32()?,
        speed_kmh: reader.f32()?,
        rpm: reader.i32()?,
        gear: reader.i32()?,
        throttle: reader.f32()?,
        brake: reader.f32()?,
        clutch: reader.f32()?,
        steering_angle: reader.f32()?,
        fuel_litres: reader.f32()?,
        acceleration_g: reader.f32_array()?,
        wheel_slip: reader.f32_array()?,
        tyre_core_temperature_c: reader.f32_array()?,
        suspension_travel_m: reader.f32_array()?,
        current_lap_ms: reader.i32()?,
        last_lap_ms: reader.i32()?,
        best_lap_ms: reader.i32()?,
        completed_laps: reader.i32()?,
        position: reader.i32()?,
        in_pit: reader.u8()? != 0,
        normalized_car_position: reader.f32()?,
        session_time_left_s: reader.f32()?,
    };

    if !reader.is_empty() {
        return Err(DecodeError::InvalidLength);
    }
    if !sample.is_finite() {
        return Err(DecodeError::NonFiniteValue);
    }

    Ok(sample)
}

fn push_i32(buffer: &mut Vec<u8>, value: i32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn push_f32(buffer: &mut Vec<u8>, value: f32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn push_f32_array<const N: usize>(buffer: &mut Vec<u8>, values: [f32; N]) {
    for value in values {
        push_f32(buffer, value);
    }
}

struct Reader<'a> {
    remaining: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(remaining: &'a [u8]) -> Self {
        Self { remaining }
    }

    fn u8(&mut self) -> Result<u8, DecodeError> {
        let (&value, remaining) = self.remaining.split_first().ok_or(DecodeError::Truncated)?;
        self.remaining = remaining;
        Ok(value)
    }

    fn i32(&mut self) -> Result<i32, DecodeError> {
        Ok(i32::from_le_bytes(self.bytes()?))
    }

    fn f32(&mut self) -> Result<f32, DecodeError> {
        Ok(f32::from_le_bytes(self.bytes()?))
    }

    fn f32_array<const N: usize>(&mut self) -> Result<[f32; N], DecodeError> {
        let mut values = [0.0; N];
        for value in &mut values {
            *value = self.f32()?;
        }
        Ok(values)
    }

    fn bytes<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let (bytes, remaining) = self
            .remaining
            .split_at_checked(N)
            .ok_or(DecodeError::Truncated)?;
        self.remaining = remaining;
        bytes.try_into().map_err(|_| DecodeError::Truncated)
    }

    fn is_empty(&self) -> bool {
        self.remaining.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{DecodeError, decode, encode};
    use crate::TelemetrySample;

    fn sample() -> TelemetrySample {
        TelemetrySample {
            packet_id: 42,
            speed_kmh: 201.5,
            rpm: 7_850,
            gear: 5,
            throttle: 0.83,
            brake: 0.12,
            clutch: 0.0,
            steering_angle: -4.5,
            fuel_litres: 31.2,
            acceleration_g: [0.4, -1.2, 0.1],
            wheel_slip: [0.1, 0.2, 0.3, 0.4],
            tyre_core_temperature_c: [83.0, 84.0, 79.0, 80.0],
            suspension_travel_m: [0.03, 0.04, 0.05, 0.06],
            current_lap_ms: 42_105,
            last_lap_ms: 91_230,
            best_lap_ms: 90_980,
            completed_laps: 7,
            position: 3,
            in_pit: false,
            normalized_car_position: 0.52,
            session_time_left_s: 503.0,
        }
    }

    #[test]
    fn round_trips_a_sample() {
        let sample = sample();
        assert_eq!(decode(&encode(&sample)), Ok(sample));
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut packet = encode(&sample());
        packet[0] = b'X';
        assert_eq!(decode(&packet), Err(DecodeError::InvalidMagic));
    }

    #[test]
    fn rejects_truncated_payload() {
        let mut packet = encode(&sample());
        packet.pop();
        assert_eq!(decode(&packet), Err(DecodeError::InvalidLength));
    }

    #[test]
    fn rejects_non_finite_values() {
        let mut sample = sample();
        sample.speed_kmh = f32::NAN;
        assert_eq!(decode(&encode(&sample)), Err(DecodeError::NonFiniteValue));
    }
}
