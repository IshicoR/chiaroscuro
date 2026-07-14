use std::{io, time::Instant};

use chiaroscuro_telemetry::TelemetrySample;

use crate::shared_memory::AcTelemetrySource;

#[derive(Debug)]
pub(crate) enum TelemetrySource {
    AssettoCorsa(AcTelemetrySource),
    Mock(MockTelemetrySource),
}

impl TelemetrySource {
    pub(crate) fn open(mock: bool) -> io::Result<Self> {
        if mock {
            Ok(Self::Mock(MockTelemetrySource::new()))
        } else {
            AcTelemetrySource::open().map(Self::AssettoCorsa)
        }
    }

    pub(crate) fn read(&mut self) -> io::Result<TelemetrySample> {
        match self {
            Self::AssettoCorsa(source) => source.read(),
            Self::Mock(source) => Ok(source.read()),
        }
    }

    pub(crate) fn name(&self) -> &'static str {
        match self {
            Self::AssettoCorsa(_) => "Assetto Corsa shared memory",
            Self::Mock(_) => "mock telemetry",
        }
    }
}

#[derive(Debug)]
pub(crate) struct MockTelemetrySource {
    started_at: Instant,
    packet_id: i32,
    last_elapsed: f32,
    selected_gear: i32,
    shift_started_at: Option<f32>,
    tyre_temperature_c: [f32; 4],
}

impl MockTelemetrySource {
    fn new() -> Self {
        Self {
            started_at: Instant::now(),
            packet_id: 0,
            last_elapsed: 0.0,
            selected_gear: 5,
            shift_started_at: None,
            tyre_temperature_c: [78.0, 78.5, 76.5, 77.0],
        }
    }

    fn read(&mut self) -> TelemetrySample {
        self.packet_id = self.packet_id.wrapping_add(1);
        self.sample_at(self.started_at.elapsed().as_secs_f32())
    }

    fn sample_at(&mut self, elapsed: f32) -> TelemetrySample {
        let lap_elapsed = elapsed % LAP_SECONDS;
        let lap_progress = lap_elapsed / LAP_SECONDS;
        let track = track_state(lap_progress);
        let dt = (elapsed - self.last_elapsed).clamp(0.0, 0.1);
        self.last_elapsed = elapsed;

        let brake = if track.longitudinal_g < -0.06 {
            ((-track.longitudinal_g - 0.04) / 1.55).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let throttle = throttle_for(track, brake);
        let gear = gear_for_speed(track.speed_kmh);
        if gear != self.selected_gear {
            self.selected_gear = gear;
            self.shift_started_at = Some(elapsed);
        }
        let time_since_shift = self.shift_started_at.map(|started| elapsed - started);
        let clutch = time_since_shift
            .filter(|time| *time < SHIFT_DURATION_SECONDS)
            .map_or(0.0, |time| {
                (1.0 - time / SHIFT_DURATION_SECONDS).clamp(0.0, 1.0) * 0.35
            });

        self.update_tyre_temperatures(dt, track, throttle, brake);
        let bump = (elapsed * 17.0).sin() * 0.0007;
        let lateral_load = track.lateral_g * 0.0028;
        let pitch = track.longitudinal_g * 0.0022;
        let completed_laps = 3 + (elapsed / LAP_SECONDS) as i32;

        TelemetrySample {
            packet_id: self.packet_id,
            speed_kmh: track.speed_kmh,
            rpm: engine_rpm(track.speed_kmh, gear, time_since_shift),
            // Assetto Corsa uses 0=reverse, 1=neutral, 2=first gear.
            gear: gear + 1,
            throttle,
            brake,
            clutch,
            steering_angle: steering_angle(track.speed_kmh, track.lateral_g),
            fuel_litres: (48.0 - elapsed * FUEL_LITRES_PER_LAP / LAP_SECONDS).max(0.0),
            acceleration_g: [track.lateral_g, track.longitudinal_g, 0.025 + bump * 8.0],
            wheel_slip: wheel_slip(track.lateral_g, throttle, brake),
            tyre_core_temperature_c: self.tyre_temperature_c,
            suspension_travel_m: [
                0.055 - pitch + lateral_load + bump,
                0.055 - pitch - lateral_load - bump,
                0.061 + pitch + lateral_load - bump,
                0.061 + pitch - lateral_load + bump,
            ],
            current_lap_ms: (lap_elapsed * 1_000.0) as i32,
            last_lap_ms: previous_lap_time(completed_laps),
            best_lap_ms: 91_736,
            completed_laps,
            position: 4,
            in_pit: false,
            normalized_car_position: lap_progress,
            session_time_left_s: (1_800.0 - elapsed).max(0.0),
        }
    }

    fn update_tyre_temperatures(&mut self, dt: f32, track: TrackState, throttle: f32, brake: f32) {
        let right_load = track.lateral_g.max(0.0);
        let left_load = (-track.lateral_g).max(0.0);
        let loads = [left_load, right_load, left_load, right_load];

        for (wheel, temperature) in self.tyre_temperature_c.iter_mut().enumerate() {
            let front = wheel < 2;
            let axle_heat = if front {
                brake * 7.0
            } else {
                throttle * 3.5 + brake * 3.0
            };
            let target = 77.0 + loads[wheel] * 6.0 + track.lateral_g.abs() * 2.5 + axle_heat
                - ((track.speed_kmh - 180.0) / 140.0).max(0.0) * 1.5;
            let response = 1.0 - (-dt / 22.0).exp();
            *temperature += (target - *temperature) * response;
        }
    }
}

const LAP_SECONDS: f32 = 92.4;
const FUEL_LITRES_PER_LAP: f32 = 2.65;
const SHIFT_DURATION_SECONDS: f32 = 0.14;
const GRAVITY_MPS2: f32 = 9.806_65;
const WHEEL_RADIUS_METRES: f32 = 0.33;
const FINAL_DRIVE_RATIO: f32 = 3.73;
const GEAR_RATIOS: [f32; 6] = [3.20, 2.19, 1.65, 1.32, 1.10, 0.93];

#[derive(Clone, Copy, Debug)]
struct TrackNode {
    phase: f32,
    speed_kmh: f32,
    lateral_g: f32,
}

#[derive(Clone, Copy, Debug)]
struct TrackState {
    speed_kmh: f32,
    lateral_g: f32,
    longitudinal_g: f32,
}

// A deterministic GT-style lap: straights, heavy braking zones, a hairpin,
// alternating corners and progressive exits. All other mock channels are
// derived from this profile so that they remain physically related.
const TRACK: [TrackNode; 25] = [
    TrackNode {
        phase: 0.000,
        speed_kmh: 238.0,
        lateral_g: 0.00,
    },
    TrackNode {
        phase: 0.055,
        speed_kmh: 283.0,
        lateral_g: 0.02,
    },
    TrackNode {
        phase: 0.110,
        speed_kmh: 306.0,
        lateral_g: 0.00,
    },
    TrackNode {
        phase: 0.145,
        speed_kmh: 265.0,
        lateral_g: -0.08,
    },
    TrackNode {
        phase: 0.180,
        speed_kmh: 132.0,
        lateral_g: -0.35,
    },
    TrackNode {
        phase: 0.205,
        speed_kmh: 92.0,
        lateral_g: -1.12,
    },
    TrackNode {
        phase: 0.235,
        speed_kmh: 126.0,
        lateral_g: -0.62,
    },
    TrackNode {
        phase: 0.290,
        speed_kmh: 211.0,
        lateral_g: 0.05,
    },
    TrackNode {
        phase: 0.335,
        speed_kmh: 239.0,
        lateral_g: 0.74,
    },
    TrackNode {
        phase: 0.375,
        speed_kmh: 221.0,
        lateral_g: 1.28,
    },
    TrackNode {
        phase: 0.415,
        speed_kmh: 266.0,
        lateral_g: 0.18,
    },
    TrackNode {
        phase: 0.470,
        speed_kmh: 296.0,
        lateral_g: 0.00,
    },
    TrackNode {
        phase: 0.505,
        speed_kmh: 181.0,
        lateral_g: 0.12,
    },
    TrackNode {
        phase: 0.535,
        speed_kmh: 118.0,
        lateral_g: 1.18,
    },
    TrackNode {
        phase: 0.565,
        speed_kmh: 108.0,
        lateral_g: -1.02,
    },
    TrackNode {
        phase: 0.610,
        speed_kmh: 169.0,
        lateral_g: -0.42,
    },
    TrackNode {
        phase: 0.670,
        speed_kmh: 251.0,
        lateral_g: 0.00,
    },
    TrackNode {
        phase: 0.720,
        speed_kmh: 279.0,
        lateral_g: 0.04,
    },
    TrackNode {
        phase: 0.750,
        speed_kmh: 188.0,
        lateral_g: 0.18,
    },
    TrackNode {
        phase: 0.785,
        speed_kmh: 76.0,
        lateral_g: -1.08,
    },
    TrackNode {
        phase: 0.820,
        speed_kmh: 111.0,
        lateral_g: -0.66,
    },
    TrackNode {
        phase: 0.865,
        speed_kmh: 188.0,
        lateral_g: 0.38,
    },
    TrackNode {
        phase: 0.905,
        speed_kmh: 226.0,
        lateral_g: 1.31,
    },
    TrackNode {
        phase: 0.945,
        speed_kmh: 214.0,
        lateral_g: 0.72,
    },
    TrackNode {
        phase: 1.000,
        speed_kmh: 238.0,
        lateral_g: 0.00,
    },
];

fn track_state(phase: f32) -> TrackState {
    let phase = phase.clamp(0.0, 1.0);
    let (start, end) = TRACK
        .windows(2)
        .find_map(|nodes| {
            (phase >= nodes[0].phase && phase <= nodes[1].phase).then_some((nodes[0], nodes[1]))
        })
        .unwrap_or((TRACK[TRACK.len() - 2], TRACK[TRACK.len() - 1]));
    let phase_span = end.phase - start.phase;
    let t = ((phase - start.phase) / phase_span).clamp(0.0, 1.0);
    let blend = t * t * (3.0 - 2.0 * t);
    let blend_derivative = 6.0 * t * (1.0 - t) / (phase_span * LAP_SECONDS);
    let speed_kmh = lerp(start.speed_kmh, end.speed_kmh, blend);
    let speed_derivative_kmh_per_second = (end.speed_kmh - start.speed_kmh) * blend_derivative;

    TrackState {
        speed_kmh,
        lateral_g: lerp(start.lateral_g, end.lateral_g, blend),
        longitudinal_g: speed_derivative_kmh_per_second / 3.6 / GRAVITY_MPS2,
    }
}

fn throttle_for(track: TrackState, brake: f32) -> f32 {
    if brake > 0.0 || track.longitudinal_g < -0.015 {
        return 0.0;
    }

    let drag_and_rolling_resistance = 0.14 + 0.25 * (track.speed_kmh / 310.0).powi(2);
    let acceleration_demand = track.longitudinal_g.max(0.0) / 0.68;
    (drag_and_rolling_resistance + acceleration_demand).clamp(0.0, 1.0)
}

fn gear_for_speed(speed_kmh: f32) -> i32 {
    match speed_kmh {
        speed if speed < 78.0 => 1,
        speed if speed < 122.0 => 2,
        speed if speed < 166.0 => 3,
        speed if speed < 213.0 => 4,
        speed if speed < 258.0 => 5,
        _ => 6,
    }
}

fn engine_rpm(speed_kmh: f32, gear: i32, time_since_shift: Option<f32>) -> i32 {
    let wheel_circumference = 2.0 * std::f32::consts::PI * WHEEL_RADIUS_METRES;
    let wheel_rpm = speed_kmh / 3.6 / wheel_circumference * 60.0;
    let ratio = GEAR_RATIOS[(gear - 1) as usize];
    let shift_cut = time_since_shift
        .filter(|time| *time < SHIFT_DURATION_SECONDS)
        .map_or(0.0, |time| {
            (1.0 - time / SHIFT_DURATION_SECONDS).clamp(0.0, 1.0) * 420.0
        });

    (wheel_rpm * ratio * FINAL_DRIVE_RATIO - shift_cut)
        .clamp(1_100.0, 9_000.0)
        .round() as i32
}

fn steering_angle(speed_kmh: f32, lateral_g: f32) -> f32 {
    let low_speed_lock = 9.0 + (1.0 - speed_kmh / 310.0).clamp(0.0, 1.0) * 18.0;
    lateral_g / 1.35 * low_speed_lock
}

fn wheel_slip(lateral_g: f32, throttle: f32, brake: f32) -> [f32; 4] {
    let right_load = lateral_g.max(0.0) * 0.018;
    let left_load = (-lateral_g).max(0.0) * 0.018;
    [
        0.018 + brake * 0.17 + left_load,
        0.018 + brake * 0.17 + right_load,
        0.022 + brake * 0.10 + throttle * 0.11 + left_load,
        0.022 + brake * 0.10 + throttle * 0.11 + right_load,
    ]
}

fn previous_lap_time(completed_laps: i32) -> i32 {
    const VARIATION_MS: [i32; 4] = [184, -226, 391, 72];
    (LAP_SECONDS * 1_000.0) as i32 + VARIATION_MS[completed_laps.rem_euclid(4) as usize]
}

fn lerp(start: f32, end: f32, amount: f32) -> f32 {
    start + (end - start) * amount
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{LAP_SECONDS, MockTelemetrySource};

    #[test]
    fn mock_lap_is_finite_and_in_realistic_ranges() {
        let mut source = MockTelemetrySource::new();
        let mut gears = BTreeSet::new();
        let mut minimum_speed = f32::MAX;
        let mut maximum_speed = f32::MIN;
        let mut maximum_throttle = 0.0_f32;
        let mut maximum_brake = 0.0_f32;
        let mut minimum_lateral_g = f32::MAX;
        let mut maximum_lateral_g = f32::MIN;

        for step in 0..=(LAP_SECONDS * 4.0) as usize {
            let sample = source.sample_at(step as f32 * 0.25);
            assert!(sample.is_finite());
            assert!((0.0..=1.0).contains(&sample.throttle));
            assert!((0.0..=1.0).contains(&sample.brake));
            assert!(sample.throttle == 0.0 || sample.brake == 0.0);
            assert!((70.0..=310.0).contains(&sample.speed_kmh));
            assert!((2..=7).contains(&sample.gear));
            assert!((1_100..=9_000).contains(&sample.rpm));
            assert!(sample.acceleration_g.iter().all(|value| value.abs() < 3.0));
            assert!((0.0..1.0).contains(&sample.normalized_car_position));
            assert!(
                sample
                    .tyre_core_temperature_c
                    .iter()
                    .all(|value| (65.0..=105.0).contains(value))
            );

            gears.insert(sample.gear);
            minimum_speed = minimum_speed.min(sample.speed_kmh);
            maximum_speed = maximum_speed.max(sample.speed_kmh);
            maximum_throttle = maximum_throttle.max(sample.throttle);
            maximum_brake = maximum_brake.max(sample.brake);
            minimum_lateral_g = minimum_lateral_g.min(sample.acceleration_g[0]);
            maximum_lateral_g = maximum_lateral_g.max(sample.acceleration_g[0]);
        }

        assert!(minimum_speed < 80.0);
        assert!(maximum_speed > 295.0);
        assert!(maximum_throttle > 0.95);
        assert!(maximum_brake > 0.75);
        assert!(minimum_lateral_g < -1.0);
        assert!(maximum_lateral_g > 1.2);
        assert!(gears.len() >= 5);
    }

    #[test]
    fn braking_and_acceleration_channels_are_correlated() {
        let mut source = MockTelemetrySource::new();
        let samples = (0..(LAP_SECONDS * 20.0) as usize)
            .map(|step| source.sample_at(step as f32 * 0.05))
            .collect::<Vec<_>>();
        let hard_braking = samples
            .iter()
            .max_by(|left, right| left.brake.total_cmp(&right.brake))
            .expect("mock lap should contain samples");
        let hard_acceleration = samples
            .iter()
            .max_by(|left, right| left.throttle.total_cmp(&right.throttle))
            .expect("mock lap should contain samples");

        assert!(hard_braking.brake > 0.75);
        assert_eq!(hard_braking.throttle, 0.0);
        assert!(hard_braking.acceleration_g[1] < -1.0);
        assert!(hard_acceleration.throttle > 0.95);
        assert_eq!(hard_acceleration.brake, 0.0);
        assert!(hard_acceleration.acceleration_g[1] > 0.3);
    }
}
