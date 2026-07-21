/// A fixed set of optional floating-point telemetry values.
///
/// iRacing only publishes some channels for specific cars or telemetry
/// sources. Keeping availability separate from the stored values avoids
/// treating a missing channel as a real zero without using non-finite sentinel
/// values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OptionalTelemetryValues<const N: usize> {
    values: [f32; N],
    available: [bool; N],
}

impl<const N: usize> OptionalTelemetryValues<N> {
    pub fn from_options(values: [Option<f32>; N]) -> Self {
        let mut output = Self::default();
        for (index, value) in values.into_iter().enumerate() {
            if let Some(value) = value {
                output.values[index] = value;
                output.available[index] = true;
            }
        }
        output
    }

    pub fn get(&self, index: usize) -> Option<f32> {
        self.available
            .get(index)
            .copied()
            .filter(|available| *available)
            .and_then(|_| self.values.get(index).copied())
    }

    pub fn iter(&self) -> impl ExactSizeIterator<Item = Option<f32>> + '_ {
        self.values
            .iter()
            .copied()
            .zip(self.available.iter().copied())
            .map(|(value, available)| available.then_some(value))
    }

    pub fn is_finite(&self) -> bool {
        self.iter().flatten().all(f32::is_finite)
    }
}

impl<const N: usize> Default for OptionalTelemetryValues<N> {
    fn default() -> Self {
        Self {
            values: [0.0; N],
            available: [false; N],
        }
    }
}

/// A snapshot of the iRacing telemetry consumed by the desktop.
///
/// Distances use metres, elapsed times use milliseconds, temperatures use
/// Celsius, speed uses kilometres per hour, and pedal inputs are normalized to
/// `0.0..=1.0`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TelemetrySample {
    pub packet_id: i32,
    pub speed_kmh: f32,
    pub rpm: i32,
    /// iRacing encoding: -1=reverse, 0=neutral, 1..=forward gears.
    pub gear: i32,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub steering_angle: f32,
    pub fuel_litres: f32,
    pub acceleration_g: [f32; 3],
    pub yaw_rate_rad_s: f32,
    pub wheel_slip: [f32; 4],
    pub tyre_core_temperature_c: [f32; 4],
    pub suspension_travel_m: [f32; 4],
    /// Brake-line pressure for LF, RF, LR, and RR. Only recorded telemetry
    /// publishes these channels.
    pub brake_line_pressure_bar: OptionalTelemetryValues<4>,
    pub abs_active: Option<bool>,
    pub steering_wheel_torque_nm: OptionalTelemetryValues<1>,
    /// Tyre carcass temperatures ordered LF, RF, LR, RR, with I/M/O values for
    /// each tyre.
    pub tyre_carcass_temperature_imo_c: OptionalTelemetryValues<12>,
    /// Hot tyre pressure for LF, RF, LR, and RR. Only recorded telemetry
    /// publishes these channels.
    pub tyre_pressure_kpa: OptionalTelemetryValues<4>,
    pub current_lap_ms: i32,
    pub last_lap_ms: i32,
    pub best_lap_ms: i32,
    pub completed_laps: i32,
    pub position: i32,
    pub in_pit: bool,
    pub normalized_car_position: f32,
    pub session_time_left_s: f32,
}

impl TelemetrySample {
    pub fn is_finite(self) -> bool {
        let scalars = [
            self.speed_kmh,
            self.throttle,
            self.brake,
            self.clutch,
            self.steering_angle,
            self.fuel_litres,
            self.yaw_rate_rad_s,
            self.normalized_car_position,
            self.session_time_left_s,
        ];

        scalars
            .into_iter()
            .chain(self.acceleration_g)
            .chain(self.wheel_slip)
            .chain(self.tyre_core_temperature_c)
            .chain(self.suspension_travel_m)
            .all(f32::is_finite)
            && self.brake_line_pressure_bar.is_finite()
            && self.steering_wheel_torque_nm.is_finite()
            && self.tyre_carcass_temperature_imo_c.is_finite()
            && self.tyre_pressure_kpa.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::{OptionalTelemetryValues, TelemetrySample};

    #[test]
    fn optional_values_preserve_partial_availability() {
        let values = OptionalTelemetryValues::from_options([Some(1.5), None, Some(-2.0)]);

        assert_eq!(values.get(0), Some(1.5));
        assert_eq!(values.get(1), None);
        assert_eq!(values.get(2), Some(-2.0));
        assert_eq!(values.get(3), None);
        assert_eq!(
            values.iter().collect::<Vec<_>>(),
            [Some(1.5), None, Some(-2.0)]
        );
        assert!(values.is_finite());
    }

    #[test]
    fn only_available_values_participate_in_finite_validation() {
        assert!(OptionalTelemetryValues::<4>::default().is_finite());
        assert!(!OptionalTelemetryValues::from_options([Some(f32::NAN), None]).is_finite());
        assert!(!OptionalTelemetryValues::from_options([None, Some(f32::INFINITY)]).is_finite());

        let sample = TelemetrySample {
            steering_wheel_torque_nm: OptionalTelemetryValues::from_options([Some(f32::NAN)]),
            ..TelemetrySample::default()
        };
        assert!(!sample.is_finite());
    }
}
