use std::io;

use chiaroscuro_telemetry::TelemetrySample;

#[cfg(target_os = "windows")]
const PHYSICS_MAPPING: &str = "Local\\acpmf_physics";
#[cfg(target_os = "windows")]
const GRAPHICS_MAPPING: &str = "Local\\acpmf_graphics";

#[cfg(any(target_os = "windows", test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct PhysicsPage {
    packet_id: i32,
    gas: f32,
    brake: f32,
    fuel: f32,
    gear: i32,
    rpms: i32,
    steer_angle: f32,
    speed_kmh: f32,
    velocity: [f32; 3],
    acceleration_g: [f32; 3],
    wheel_slip: [f32; 4],
    wheel_load: [f32; 4],
    wheel_pressure: [f32; 4],
    wheel_angular_speed: [f32; 4],
    tyre_wear: [f32; 4],
    tyre_dirty_level: [f32; 4],
    tyre_core_temperature: [f32; 4],
    camber_rad: [f32; 4],
    suspension_travel: [f32; 4],
    drs: f32,
    traction_control: f32,
    heading: f32,
    pitch: f32,
    roll: f32,
    centre_of_gravity_height: f32,
    car_damage: [f32; 5],
    tyres_out: i32,
    pit_limiter_on: i32,
    abs: f32,
    kers_charge: f32,
    kers_input: f32,
    auto_shifter_on: i32,
    ride_height: [f32; 2],
    turbo_boost: f32,
    ballast: f32,
    air_density: f32,
    air_temperature: f32,
    road_temperature: f32,
    local_angular_velocity: [f32; 3],
    final_force_feedback: f32,
    performance_meter: f32,
    engine_brake: i32,
    ers_recovery_level: i32,
    ers_power_level: i32,
    ers_heat_charging: i32,
    ers_is_charging: i32,
    kers_current_kj: f32,
    drs_available: i32,
    drs_enabled: i32,
    brake_temperature: [f32; 4],
    clutch: f32,
}

#[cfg(any(target_os = "windows", test))]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct GraphicsPage {
    packet_id: i32,
    status: i32,
    session: i32,
    current_time: [u16; 15],
    last_time: [u16; 15],
    best_time: [u16; 15],
    split: [u16; 15],
    completed_laps: i32,
    position: i32,
    current_lap_ms: i32,
    last_lap_ms: i32,
    best_lap_ms: i32,
    session_time_left: f32,
    distance_travelled: f32,
    is_in_pit: i32,
    current_sector_index: i32,
    last_sector_time: i32,
    number_of_laps: i32,
    tyre_compound: [u16; 33],
    replay_time_multiplier: f32,
    normalized_car_position: f32,
}

#[cfg(target_os = "windows")]
pub(crate) struct AcTelemetrySource {
    physics: windows::Mapping<PhysicsPage>,
    graphics: windows::Mapping<GraphicsPage>,
}

#[cfg(target_os = "windows")]
impl AcTelemetrySource {
    pub(crate) fn open() -> io::Result<Self> {
        Ok(Self {
            physics: windows::Mapping::open(PHYSICS_MAPPING)?,
            graphics: windows::Mapping::open(GRAPHICS_MAPPING)?,
        })
    }

    pub(crate) fn read(&self) -> io::Result<TelemetrySample> {
        let physics = self.physics.read();
        let graphics = self.graphics.read();
        Ok(to_sample(physics, graphics))
    }
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug)]
pub(crate) struct AcTelemetrySource;

#[cfg(not(target_os = "windows"))]
impl AcTelemetrySource {
    pub(crate) fn open() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Assetto Corsa shared memory is only available on Windows",
        ))
    }

    pub(crate) fn read(&self) -> io::Result<TelemetrySample> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Assetto Corsa shared memory is only available on Windows",
        ))
    }
}

#[cfg(target_os = "windows")]
fn to_sample(physics: PhysicsPage, graphics: GraphicsPage) -> TelemetrySample {
    TelemetrySample {
        packet_id: physics.packet_id,
        speed_kmh: physics.speed_kmh,
        rpm: physics.rpms,
        gear: physics.gear,
        throttle: physics.gas,
        brake: physics.brake,
        clutch: physics.clutch,
        steering_angle: physics.steer_angle,
        fuel_litres: physics.fuel,
        acceleration_g: physics.acceleration_g,
        wheel_slip: physics.wheel_slip,
        tyre_core_temperature_c: physics.tyre_core_temperature,
        suspension_travel_m: physics.suspension_travel,
        current_lap_ms: graphics.current_lap_ms,
        last_lap_ms: graphics.last_lap_ms,
        best_lap_ms: graphics.best_lap_ms,
        completed_laps: graphics.completed_laps,
        position: graphics.position,
        in_pit: graphics.is_in_pit != 0,
        normalized_car_position: graphics.normalized_car_position,
        session_time_left_s: graphics.session_time_left,
    }
}

#[cfg(target_os = "windows")]
mod windows {
    use std::{io, marker::PhantomData, mem::size_of, ptr};

    use winapi::{
        ctypes::c_void,
        shared::minwindef::FALSE,
        um::{
            handleapi::CloseHandle,
            memoryapi::{FILE_MAP_READ, MapViewOfFile, OpenFileMappingW, UnmapViewOfFile},
            winnt::HANDLE,
        },
    };

    pub(super) struct Mapping<T> {
        handle: HANDLE,
        view: *mut c_void,
        marker: PhantomData<T>,
    }

    impl<T: Copy> Mapping<T> {
        pub(super) fn open(name: &str) -> io::Result<Self> {
            let wide_name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
            let handle = unsafe { OpenFileMappingW(FILE_MAP_READ, FALSE, wide_name.as_ptr()) };
            if handle.is_null() {
                return Err(io::Error::last_os_error());
            }

            let view = unsafe { MapViewOfFile(handle, FILE_MAP_READ, 0, 0, size_of::<T>()) };
            if view.is_null() {
                let error = io::Error::last_os_error();
                unsafe {
                    CloseHandle(handle);
                }
                return Err(error);
            }

            Ok(Self {
                handle,
                view,
                marker: PhantomData,
            })
        }

        pub(super) fn read(&self) -> T {
            unsafe { ptr::read_volatile(self.view.cast::<T>()) }
        }
    }

    impl<T> Drop for Mapping<T> {
        fn drop(&mut self) {
            unsafe {
                UnmapViewOfFile(self.view);
                CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{offset_of, size_of};

    use super::{GraphicsPage, PhysicsPage};

    #[test]
    fn physics_prefix_matches_assetto_corsa_pack_4_layout() {
        assert_eq!(offset_of!(PhysicsPage, speed_kmh), 28);
        assert_eq!(offset_of!(PhysicsPage, acceleration_g), 44);
        assert_eq!(offset_of!(PhysicsPage, tyre_core_temperature), 152);
        assert_eq!(offset_of!(PhysicsPage, suspension_travel), 184);
        assert_eq!(offset_of!(PhysicsPage, clutch), 364);
        assert_eq!(size_of::<PhysicsPage>(), 368);
    }

    #[test]
    fn graphics_prefix_matches_assetto_corsa_pack_4_layout() {
        assert_eq!(offset_of!(GraphicsPage, completed_laps), 132);
        assert_eq!(offset_of!(GraphicsPage, current_lap_ms), 140);
        assert_eq!(offset_of!(GraphicsPage, session_time_left), 152);
        assert_eq!(offset_of!(GraphicsPage, normalized_car_position), 248);
        assert_eq!(size_of::<GraphicsPage>(), 252);
    }
}
