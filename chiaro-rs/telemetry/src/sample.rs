/// A simulator-neutral snapshot of the telemetry needed by the desktop.
///
/// Distances use metres, elapsed times use milliseconds, temperatures use
/// Celsius, speed uses kilometres per hour, and pedal inputs are normalized to
/// `0.0..=1.0`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TelemetrySample {
    pub packet_id: i32,
    pub speed_kmh: f32,
    pub rpm: i32,
    pub gear: i32,
    pub throttle: f32,
    pub brake: f32,
    pub clutch: f32,
    pub steering_angle: f32,
    pub fuel_litres: f32,
    pub acceleration_g: [f32; 3],
    pub wheel_slip: [f32; 4],
    pub tyre_core_temperature_c: [f32; 4],
    pub suspension_travel_m: [f32; 4],
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
    }
}
