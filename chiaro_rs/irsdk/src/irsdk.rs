//! In-process access to every variable published through iRacing shared memory.
//!
//! [`Client::read_snapshot`] copies one stable frame. Named access remains
//! available for newly-added or car-specific variables, while [`variables`]
//! provides strict typed keys for stable SDK variables.
//!
//! ```
//! use chiaro_irsdk::{
//!     SessionFlags, SessionState, TelemetryFrame, VariableAccessError, variables,
//! };
//!
//! fn inspect(frame: &TelemetryFrame) -> Result<(), VariableAccessError> {
//!     let speed_metres_per_second = frame.get(variables::player::SPEED)?;
//!     let state = SessionState::from(frame.get(variables::session::SESSION_STATE)?);
//!     let flags = SessionFlags::from(frame.get(variables::session::SESSION_FLAGS)?);
//!     let car_positions = frame.get(variables::cars::CAR_IDX_LAP_DIST_PCT)?;
//!     let _ = (speed_metres_per_second, state, flags, car_positions);
//!     Ok(())
//! }
//! ```

use std::{io, time::Duration};

mod enums;
mod flags;
mod key;
mod sample;
mod session_info;
mod shared_memory;
mod variable;
pub mod variables;

pub use enums::*;
pub use flags::*;
pub use key::{
    ArrayKey, ScalarKey, TelemetryKey, TelemetryPrimitive, VariableAccessError, VariableShape,
};
pub use sample::{OptionalTelemetryValues, TelemetrySample};
pub use session_info::*;
use shared_memory::IracingTelemetrySource;
pub use shared_memory::{IbtFile, IbtFrames, IbtMetadata, IbtReader, IbtSnapshots};
use variable::VariableCatalog;
pub use variable::{
    FrameBuildError, TelemetryFrame, TelemetrySnapshot, TelemetryValue, VariableMetadata,
    VariableType,
};

#[derive(Debug)]
pub struct Client {
    source: IracingTelemetrySource,
    last_sample_packet_id: Option<i32>,
    last_frame_packet_id: Option<i32>,
    last_snapshot_packet_id: Option<i32>,
    last_session_info_update: Option<i32>,
}

impl Client {
    pub fn connect() -> io::Result<Self> {
        Ok(Self {
            source: IracingTelemetrySource::open()?,
            last_sample_packet_id: None,
            last_frame_packet_id: None,
            last_snapshot_packet_id: None,
            last_session_info_update: None,
        })
    }

    pub fn read_latest(&mut self) -> io::Result<Option<TelemetrySample>> {
        let sample = match self.source.read_sample() {
            Ok(sample) => sample,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error),
        };

        if self.last_sample_packet_id == Some(sample.packet_id) {
            return Ok(None);
        }
        if !sample.is_finite() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "iRacing telemetry contains a non-finite value",
            ));
        }

        self.last_sample_packet_id = Some(sample.packet_id);
        Ok(Some(sample))
    }

    /// Returns every SDK variable from the newest stable frame.
    pub fn read_all(&mut self) -> io::Result<Option<TelemetryFrame>> {
        let frame = match self.source.read_frame() {
            Ok(frame) => frame,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error),
        };

        if self.last_frame_packet_id == Some(frame.packet_id()) {
            return Ok(None);
        }

        self.last_frame_packet_id = Some(frame.packet_id());
        Ok(Some(frame))
    }

    /// Copies the newest stable frame and returns one named SDK variable from it.
    pub fn read_value(&mut self, name: &str) -> io::Result<TelemetryValue> {
        self.source.read_value(name)
    }

    /// Returns the desktop sample and every SDK value from the same stable frame.
    pub fn read_snapshot(&mut self) -> io::Result<Option<TelemetrySnapshot>> {
        let snapshot = match self.source.read_snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error),
        };

        if self.last_snapshot_packet_id == Some(snapshot.frame.packet_id()) {
            return Ok(None);
        }
        if !snapshot.sample.is_finite() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "iRacing telemetry contains a non-finite value",
            ));
        }

        self.last_snapshot_packet_id = Some(snapshot.frame.packet_id());
        Ok(Some(snapshot))
    }

    /// Returns all variable definitions published for the current iRacing session.
    pub fn variables(&self) -> &[VariableMetadata] {
        self.source.variables()
    }

    /// Returns the current YAML session information and its exact source bytes.
    pub fn session_info(&self) -> io::Result<SessionInfo> {
        self.source.session_info()
    }

    /// Returns session information only when its SDK update counter changed.
    pub fn read_session_info(&mut self) -> io::Result<Option<SessionInfo>> {
        let update_count = self.source.session_info_update()?;
        if self.last_session_info_update == Some(update_count) {
            return Ok(None);
        }

        let info = match self.source.session_info() {
            Ok(info) => info,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(None),
            Err(error) => return Err(error),
        };

        if self.last_session_info_update == Some(info.update_count) {
            return Ok(None);
        }

        self.last_session_info_update = Some(info.update_count);
        Ok(Some(info))
    }

    /// Waits on IRSDKDataValidEvent instead of polling at a fixed interval.
    ///
    /// iRacing shared memory exposes the latest frame rather than a queued
    /// history, so a stalled consumer can still skip frames.
    pub fn wait_for_update(&mut self, timeout: Duration) -> io::Result<Option<TelemetrySample>> {
        if let Some(sample) = self.read_latest()? {
            return Ok(Some(sample));
        }

        self.source.wait_for_data(timeout)?;
        self.read_latest()
    }

    /// Waits for an SDK update and returns every variable from the newest frame.
    pub fn wait_for_all(&mut self, timeout: Duration) -> io::Result<Option<TelemetryFrame>> {
        if let Some(frame) = self.read_all()? {
            return Ok(Some(frame));
        }

        self.source.wait_for_data(timeout)?;
        self.read_all()
    }

    /// Waits for an SDK update and returns a complete same-frame snapshot.
    pub fn wait_for_snapshot(
        &mut self,
        timeout: Duration,
    ) -> io::Result<Option<TelemetrySnapshot>> {
        if let Some(snapshot) = self.read_snapshot()? {
            return Ok(Some(snapshot));
        }

        self.source.wait_for_data(timeout)?;
        self.read_snapshot()
    }
}
