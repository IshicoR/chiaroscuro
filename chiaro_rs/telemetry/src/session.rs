use std::{
    collections::VecDeque,
    ops::Range,
    time::{Duration, Instant},
};

use chiaro_irsdk::{SessionInfo, TelemetryFrame, TelemetrySample, TelemetrySnapshot};

use crate::ibt::{IbtInfo, LoadedIbt};
use crate::timing::{self, LapTiming, SectorCrossing, StintTiming, TimingEntry};

pub const HISTORY_WINDOW: Duration = Duration::from_secs(12);
/// Internal X-axis units for one lap. Basis-point scaling keeps long, flat line
/// segments representable in the chart renderer's cumulative `f32` geometry.
pub const LAP_DISTANCE_AXIS_MAX: f64 = 10_000.0;
// The live chart renders the latest 12 seconds. Keep one extra second so the
// line still crosses the left viewport edge smoothly, but discard data that is
// no longer useful instead of repeatedly tessellating a much larger window.
const LIVE_HISTORY_RETENTION: Duration = Duration::from_secs(13);
const LIVE_HISTORY_SAMPLE_LIMIT: usize = 13 * 60;
const TRANSIENT_NEUTRAL_MAX_SECONDS: f64 = 0.35;
const LAP_DISTANCE_BUCKET_COUNT: usize = 10_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

/// Stable presentation and capability metadata for a live telemetry source.
///
/// Transport crates provide this descriptor while the session and UI remain
/// independent from the concrete transport (shared memory, cloud, and so on).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiveTelemetrySourceInfo {
    id: &'static str,
    display_name: &'static str,
    unavailable_reason: Option<&'static str>,
}

impl LiveTelemetrySourceInfo {
    pub const fn available(id: &'static str, display_name: &'static str) -> Self {
        Self {
            id,
            display_name,
            unavailable_reason: None,
        }
    }

    pub const fn unavailable(
        id: &'static str,
        display_name: &'static str,
        reason: &'static str,
    ) -> Self {
        Self {
            id,
            display_name,
            unavailable_reason: Some(reason),
        }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn display_name(self) -> &'static str {
        self.display_name
    }

    pub const fn is_available(self) -> bool {
        self.unavailable_reason.is_none()
    }

    pub const fn unavailable_reason(self) -> Option<&'static str> {
        self.unavailable_reason
    }
}

#[derive(Debug, Clone, Copy)]
struct HistoryEntry {
    elapsed_seconds: f64,
    sample: TelemetrySample,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelemetryLap {
    number: i32,
    start_index: usize,
    end_index: usize,
    duration_ms: i32,
    complete: bool,
}

impl TelemetryLap {
    pub const fn number(self) -> i32 {
        self.number
    }

    pub const fn duration_ms(self) -> i32 {
        self.duration_ms
    }

    pub const fn is_complete(self) -> bool {
        self.complete
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusedTelemetry {
    pub point_index: usize,
    pub elapsed_seconds: f64,
    pub sample: TelemetrySample,
}

#[derive(Debug, Clone, Default)]
enum SessionSource {
    #[default]
    Live,
    Ibt(IbtInfo),
}

#[derive(Debug, Clone, Default)]
pub struct Session {
    connection: ConnectionStatus,
    source: SessionSource,
    packets_received: u64,
    latest: Option<TelemetrySample>,
    latest_frame: Option<TelemetryFrame>,
    session_info: Option<SessionInfo>,
    session_info_revision: u64,
    history: VecDeque<HistoryEntry>,
    laps: Vec<TelemetryLap>,
    timing_history: Vec<TimingEntry>,
    lap_timings: Vec<LapTiming>,
    stints: Vec<StintTiming>,
    sector_starts: Vec<f64>,
    sector_crossings: Vec<SectorCrossing>,
    timing_revision: u64,
    live_started_at: Option<Instant>,
    last_error: Option<String>,
}

impl Session {
    pub fn connection(&self) -> ConnectionStatus {
        self.connection
    }

    pub fn wants_connection(&self) -> bool {
        self.connection != ConnectionStatus::Disconnected
    }

    pub fn ibt_info(&self) -> Option<&IbtInfo> {
        match &self.source {
            SessionSource::Live => None,
            SessionSource::Ibt(info) => Some(info),
        }
    }

    pub fn chart_duration_seconds(&self) -> f64 {
        self.ibt_info().map_or_else(
            || HISTORY_WINDOW.as_secs_f64(),
            |info| info.duration_seconds.max(1.0),
        )
    }

    pub fn live_chart_time_bounds(&self) -> Option<(f64, f64)> {
        if !matches!(self.source, SessionSource::Live) {
            return None;
        }

        self.history
            .front()
            .zip(self.history.back())
            .map(|(first, last)| (first.elapsed_seconds, last.elapsed_seconds))
    }

    /// Returns the beginning of the retained live chart data.
    ///
    /// The visible window can be narrower than this retained range. Keeping
    /// this boundary aligned with the history front preserves point indices
    /// when old live samples are trimmed. Recorded sessions return `None`.
    pub fn live_chart_minimum_x(&self) -> Option<f64> {
        if !matches!(self.source, SessionSource::Live) {
            return None;
        }

        self.history.front().map(|first| first.elapsed_seconds)
    }

    /// Finds the finite value range across the retained live history without
    /// allocating an intermediate point collection.
    pub fn live_value_range(&self, value: impl Fn(&TelemetrySample) -> f32) -> Option<(f64, f64)> {
        if !matches!(self.source, SessionSource::Live) {
            return None;
        }

        self.history
            .iter()
            .map(|entry| f64::from(value(&entry.sample)))
            .filter(|value| value.is_finite())
            .fold(None, |range, value| {
                Some(range.map_or((value, value), |(min, max): (f64, f64)| {
                    (min.min(value), max.max(value))
                }))
            })
    }

    /// Finds the finite range of an optional value across retained live history.
    pub fn live_value_range_optional(
        &self,
        value: impl Fn(&TelemetrySample) -> Option<f32>,
    ) -> Option<(f64, f64)> {
        if !matches!(self.source, SessionSource::Live) {
            return None;
        }

        self.history
            .iter()
            .filter_map(|entry| value(&entry.sample))
            .map(f64::from)
            .filter(|value| value.is_finite())
            .fold(None, |range, value| {
                Some(range.map_or((value, value), |(min, max): (f64, f64)| {
                    (min.min(value), max.max(value))
                }))
            })
    }

    pub fn laps(&self) -> &[TelemetryLap] {
        &self.laps
    }

    pub fn lap_timings(&self) -> &[LapTiming] {
        &self.lap_timings
    }

    pub fn stints(&self) -> &[StintTiming] {
        &self.stints
    }

    pub fn sector_starts(&self) -> &[f64] {
        &self.sector_starts
    }

    pub fn sector_crossings(&self) -> &[SectorCrossing] {
        &self.sector_crossings
    }

    pub const fn timing_revision(&self) -> u64 {
        self.timing_revision
    }

    pub fn fastest_complete_lap_index(&self) -> Option<usize> {
        self.laps
            .iter()
            .enumerate()
            .filter(|(_, lap)| lap.complete && lap.duration_ms > 0)
            .min_by_key(|(_, lap)| lap.duration_ms)
            .map(|(index, _)| index)
    }

    pub fn preferred_lap_index(&self) -> Option<usize> {
        self.laps
            .iter()
            .rposition(|lap| lap.complete)
            .or_else(|| self.laps.len().checked_sub(1))
    }

    pub fn chart_duration_seconds_for(&self, lap_index: Option<usize>) -> f64 {
        let Some(index) = lap_index else {
            return self.chart_duration_seconds();
        };
        let Some(lap) = self.laps.get(index) else {
            return 1.0;
        };
        let recorded_duration = self
            .history
            .get(lap.start_index)
            .zip(self.history.get(lap.end_index.saturating_sub(1)))
            .map_or(0.0, |(first, last)| {
                (last.elapsed_seconds - first.elapsed_seconds).max(0.0)
            });

        recorded_duration
            .max(f64::from(lap.duration_ms) / 1_000.0)
            .max(1.0)
    }

    pub fn packets_received(&self) -> u64 {
        self.packets_received
    }

    pub fn latest(&self) -> Option<&TelemetrySample> {
        self.latest.as_ref()
    }

    pub fn latest_frame(&self) -> Option<&TelemetryFrame> {
        self.latest_frame.as_ref()
    }

    pub fn session_info(&self) -> Option<&SessionInfo> {
        self.session_info.as_ref()
    }

    /// Monotonically identifies the SessionInfo value held by this session.
    ///
    /// The SDK update counter can restart when iRacing reconnects, so UI
    /// caches must use this revision instead of comparing the SDK counter.
    pub fn session_info_revision(&self) -> u64 {
        self.session_info_revision
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn set_connection_requested(&mut self, connected: bool) {
        self.clear_telemetry();
        self.source = SessionSource::Live;
        self.connection = if connected {
            ConnectionStatus::Connecting
        } else {
            ConnectionStatus::Disconnected
        };
        self.last_error = None;
    }

    fn clear_telemetry(&mut self) {
        self.packets_received = 0;
        self.latest = None;
        self.latest_frame = None;
        self.replace_session_info(None);
        self.history.clear();
        self.laps.clear();
        self.timing_history.clear();
        self.lap_timings.clear();
        self.stints.clear();
        self.sector_starts.clear();
        self.sector_crossings.clear();
        self.timing_revision = self.timing_revision.wrapping_add(1);
        self.live_started_at = None;
    }

    pub fn begin_ibt_load(&mut self) {
        self.connection = ConnectionStatus::Disconnected;
        self.last_error = None;
    }

    pub fn load_ibt(&mut self, loaded: LoadedIbt) {
        let LoadedIbt {
            info,
            samples,
            latest_frame,
            session_info,
        } = loaded;
        let latest = samples.last().map(|entry| entry.sample);

        self.connection = ConnectionStatus::Disconnected;
        self.packets_received = u64::try_from(info.record_count).unwrap_or(u64::MAX);
        self.latest = latest;
        self.latest_frame = Some(latest_frame);
        self.replace_session_info(Some(session_info));
        self.history = samples
            .into_iter()
            .map(|entry| HistoryEntry {
                elapsed_seconds: entry.elapsed_seconds,
                sample: entry.sample,
            })
            .collect();
        self.laps = build_laps(&self.history);
        self.timing_history = self
            .history
            .iter()
            .map(|entry| TimingEntry::new(entry.elapsed_seconds, entry.sample))
            .collect();
        self.rebuild_timing();
        self.live_started_at = None;
        self.last_error = None;
        self.source = SessionSource::Ibt(info);
    }

    pub fn mark_ibt_error(&mut self, error: String) {
        self.connection = ConnectionStatus::Disconnected;
        self.last_error = Some(error);
    }

    pub fn mark_waiting(&mut self) {
        if self.wants_connection() {
            self.connection = ConnectionStatus::Connecting;
        }
    }

    pub fn mark_connected(&mut self) {
        if self.wants_connection() {
            self.connection = ConnectionStatus::Connected;
            self.last_error = None;
        }
    }

    pub fn mark_error(&mut self, error: String) {
        if self.wants_connection() {
            self.connection = ConnectionStatus::Connecting;
            self.last_error = Some(error);
        }
    }

    pub fn record_sample(&mut self, sample: TelemetrySample) {
        self.record_sample_at(Instant::now(), sample);
    }

    /// Records one live sample using the time it was captured by the telemetry
    /// producer instead of the time the UI happened to process it.
    pub fn record_sample_at(&mut self, captured_at: Instant, sample: TelemetrySample) {
        if self.ibt_info().is_some() || !sample.is_finite() {
            return;
        }

        let started_at = *self.live_started_at.get_or_insert(captured_at);
        let elapsed_seconds = captured_at
            .saturating_duration_since(started_at)
            .as_secs_f64();
        self.latest = Some(sample);
        self.packets_received = self.packets_received.saturating_add(1);
        self.history.push_back(HistoryEntry {
            elapsed_seconds,
            sample,
        });
        let should_rebuild_timing = self.timing_history.last().is_none_or(|previous| {
            previous.completed_laps != sample.completed_laps
                || previous.in_pit != sample.in_pit
                || crossed_sector(
                    previous.normalized_car_position,
                    sample.normalized_car_position,
                    &self.sector_starts,
                )
        });
        self.timing_history
            .push(TimingEntry::new(elapsed_seconds, sample));
        if should_rebuild_timing {
            self.rebuild_timing();
        } else if timing::update_active(
            &mut self.lap_timings,
            &mut self.stints,
            TimingEntry::new(elapsed_seconds, sample),
        ) {
            self.timing_revision = self.timing_revision.wrapping_add(1);
        }

        self.trim_live_history(elapsed_seconds);
    }

    /// Applies a producer-side batch while preserving the capture time of every
    /// sample. The full SDK frame and session information are optional because
    /// they are refreshed less frequently than the chart samples.
    pub fn record_live_batch(
        &mut self,
        samples: impl IntoIterator<Item = (Instant, TelemetrySample)>,
        latest_frame: Option<TelemetryFrame>,
        session_info: Option<SessionInfo>,
    ) {
        if self.ibt_info().is_some() {
            return;
        }

        if let Some(frame) = latest_frame {
            self.latest_frame = Some(frame);
        }
        if let Some(session_info) = session_info {
            self.replace_session_info(Some(session_info));
        }
        for (captured_at, sample) in samples {
            self.record_sample_at(captured_at, sample);
        }
    }

    fn trim_live_history(&mut self, elapsed_seconds: f64) {
        while self.history.len() > LIVE_HISTORY_SAMPLE_LIMIT
            || self.history.front().is_some_and(|entry| {
                elapsed_seconds - entry.elapsed_seconds > LIVE_HISTORY_RETENTION.as_secs_f64()
            })
        {
            self.history.pop_front();
        }
    }

    pub fn record_snapshot(
        &mut self,
        snapshot: TelemetrySnapshot,
        session_info: Option<SessionInfo>,
    ) {
        if self.ibt_info().is_some() {
            return;
        }

        self.latest_frame = Some(snapshot.frame);
        if let Some(session_info) = session_info {
            self.replace_session_info(Some(session_info));
        }
        self.record_sample(snapshot.sample);
    }

    fn replace_session_info(&mut self, session_info: Option<SessionInfo>) {
        self.session_info = session_info;
        let sector_starts = timing::sector_starts(self.session_info.as_ref());
        if self.sector_starts != sector_starts {
            self.sector_starts = sector_starts;
            self.rebuild_timing();
        }
        self.session_info_revision = self.session_info_revision.wrapping_add(1);
    }

    fn rebuild_timing(&mut self) {
        let (laps, stints) = timing::build(&self.timing_history, &self.sector_starts);
        let sector_crossings = timing::sector_crossings(&self.timing_history, &self.sector_starts);
        if self.lap_timings != laps
            || self.stints != stints
            || self.sector_crossings != sector_crossings
        {
            self.lap_timings = laps;
            self.stints = stints;
            self.sector_crossings = sector_crossings;
            self.timing_revision = self.timing_revision.wrapping_add(1);
        }
    }

    pub fn points_in(
        &self,
        lap_index: Option<usize>,
        value: impl Fn(&TelemetrySample) -> f32,
    ) -> Vec<[f64; 2]> {
        let Some(range) = self.history_range(lap_index) else {
            return Vec::new();
        };
        if lap_index.is_some() {
            return positioned_entries(&self.history, range)
                .map(|(position, entry)| [position, f64::from(value(&entry.sample))])
                .collect();
        }
        let Some(first) = self.history.get(range.start) else {
            return Vec::new();
        };
        let origin = self.elapsed_origin(lap_index, first);

        self.history
            .iter()
            .skip(range.start)
            .take(range.len())
            .map(|entry| {
                [
                    entry.elapsed_seconds - origin,
                    f64::from(value(&entry.sample)),
                ]
            })
            .collect()
    }

    /// Returns points for a channel that may not be published by the current car.
    pub fn points_in_optional(
        &self,
        lap_index: Option<usize>,
        value: impl Fn(&TelemetrySample) -> Option<f32>,
    ) -> Vec<[f64; 2]> {
        let Some(range) = self.history_range(lap_index) else {
            return Vec::new();
        };
        let Some(first) = self.history.get(range.start) else {
            return Vec::new();
        };
        let origin = self.elapsed_origin(lap_index, first);

        if lap_index.is_some() {
            return positioned_entries(&self.history, range)
                .filter_map(|(position, entry)| {
                    value(&entry.sample)
                        .filter(|value| value.is_finite())
                        .map(|value| [position, f64::from(value)])
                })
                .collect();
        }

        self.history
            .iter()
            .skip(range.start)
            .take(range.len())
            .filter_map(|entry| {
                value(&entry.sample)
                    .filter(|value| value.is_finite())
                    .map(|value| [entry.elapsed_seconds - origin, f64::from(value)])
            })
            .collect()
    }

    /// Returns live points received after `packets_received`, treating it as a
    /// count of samples already consumed by the caller.
    ///
    /// A cursor older than the retained history is clamped to its first sample;
    /// a cursor at or beyond the current count produces no points. Recorded
    /// sessions return an empty collection because their X axis is lap based.
    pub fn live_points_since(
        &self,
        packets_received: u64,
        value: impl Fn(&TelemetrySample) -> f32,
    ) -> Vec<[f64; 2]> {
        if !matches!(self.source, SessionSource::Live) || self.history.is_empty() {
            return Vec::new();
        }

        let retained_count = u64::try_from(self.history.len()).unwrap_or(u64::MAX);
        let first_retained_packet = self.packets_received.saturating_sub(retained_count);
        let start_packet = packets_received.clamp(first_retained_packet, self.packets_received);
        let skip = usize::try_from(start_packet - first_retained_packet)
            .unwrap_or(self.history.len())
            .min(self.history.len());

        self.history
            .iter()
            .skip(skip)
            .map(|entry| [entry.elapsed_seconds, f64::from(value(&entry.sample))])
            .collect()
    }

    /// Returns newly received live points while omitting unavailable values.
    pub fn live_points_since_optional(
        &self,
        packets_received: u64,
        value: impl Fn(&TelemetrySample) -> Option<f32>,
    ) -> Vec<[f64; 2]> {
        if !matches!(self.source, SessionSource::Live) || self.history.is_empty() {
            return Vec::new();
        }

        let retained_count = u64::try_from(self.history.len()).unwrap_or(u64::MAX);
        let first_retained_packet = self.packets_received.saturating_sub(retained_count);
        let start_packet = packets_received.clamp(first_retained_packet, self.packets_received);
        let skip = usize::try_from(start_packet - first_retained_packet)
            .unwrap_or(self.history.len())
            .min(self.history.len());

        self.history
            .iter()
            .skip(skip)
            .filter_map(|entry| {
                value(&entry.sample)
                    .filter(|value| value.is_finite())
                    .map(|value| [entry.elapsed_seconds, f64::from(value)])
            })
            .collect()
    }

    pub fn gear_points(&self, lap_index: Option<usize>) -> Vec<[f64; 2]> {
        let Some(lap_index) = lap_index else {
            let mut points = self.points_in(None, |sample| sample.gear as f32);
            suppress_transient_neutral_gears(&mut points);
            return points;
        };
        let Some(range) = self.history_range(Some(lap_index)) else {
            return Vec::new();
        };
        let Some(first) = self.history.get(range.start) else {
            return Vec::new();
        };
        let entries = self
            .history
            .iter()
            .skip(range.start)
            .take(range.len())
            .collect::<Vec<_>>();
        let mut timed_gears = entries
            .iter()
            .map(|entry| {
                [
                    entry.elapsed_seconds - first.elapsed_seconds,
                    f64::from(entry.sample.gear),
                ]
            })
            .collect::<Vec<_>>();
        suppress_transient_neutral_gears(&mut timed_gears);

        positioned_entry_indices(&self.history, range.clone())
            .into_iter()
            .map(|(position, history_index)| {
                [position, timed_gears[history_index - range.start][1]]
            })
            .collect()
    }

    pub fn comparison_gear_points(
        &self,
        lap_index: usize,
        reference_session: &Self,
        reference_lap_index: usize,
    ) -> Vec<[f64; 2]> {
        let Some(lap_range) = self.history_range(Some(lap_index)) else {
            return Vec::new();
        };
        let Some(reference_range) = reference_session.history_range(Some(reference_lap_index))
        else {
            return Vec::new();
        };
        let Some(reference_start) = reference_session.history.get(reference_range.start) else {
            return Vec::new();
        };

        let reference_entries = reference_session
            .history
            .iter()
            .skip(reference_range.start)
            .take(reference_range.len())
            .collect::<Vec<_>>();
        let mut timed_gears = reference_entries
            .iter()
            .map(|entry| {
                [
                    entry.elapsed_seconds - reference_start.elapsed_seconds,
                    f64::from(entry.sample.gear),
                ]
            })
            .collect::<Vec<_>>();
        suppress_transient_neutral_gears(&mut timed_gears);

        let positioned_gears =
            positioned_entry_indices(&reference_session.history, reference_range.clone())
                .into_iter()
                .map(|(position, history_index)| {
                    (
                        position,
                        timed_gears[history_index - reference_range.start][1],
                    )
                })
                .collect::<Vec<_>>();

        positioned_entries(&self.history, lap_range)
            .filter_map(|(position, _)| {
                nearest_value_at_position(&positioned_gears, position).map(|gear| [position, gear])
            })
            .collect()
    }

    pub fn fuel_used_points(&self, lap_index: Option<usize>) -> Vec<[f64; 2]> {
        let Some(range) = self.history_range(lap_index) else {
            return Vec::new();
        };
        let Some(first) = self.history.get(range.start) else {
            return Vec::new();
        };
        let start_fuel = first.sample.fuel_litres;
        if lap_index.is_some() {
            return positioned_entries(&self.history, range)
                .map(|(position, entry)| {
                    [
                        position,
                        f64::from((start_fuel - entry.sample.fuel_litres).max(0.0)),
                    ]
                })
                .collect();
        }
        let origin = self.elapsed_origin(lap_index, first);

        self.history
            .iter()
            .skip(range.start)
            .take(range.len())
            .map(|entry| {
                [
                    entry.elapsed_seconds - origin,
                    f64::from((start_fuel - entry.sample.fuel_litres).max(0.0)),
                ]
            })
            .collect()
    }

    pub fn lap_start_fuel_litres(&self, lap_index: usize) -> Option<f32> {
        let range = self.history_range(Some(lap_index))?;
        self.history
            .get(range.start)
            .map(|entry| entry.sample.fuel_litres)
            .filter(|fuel_litres| fuel_litres.is_finite())
    }

    pub fn lap_delta_points(&self, lap_index: usize, reference_lap_index: usize) -> Vec<[f64; 2]> {
        self.lap_delta_points_against(lap_index, self, reference_lap_index)
    }

    pub fn lap_delta_points_against(
        &self,
        lap_index: usize,
        reference_session: &Self,
        reference_lap_index: usize,
    ) -> Vec<[f64; 2]> {
        let Some(lap_range) = self.history_range(Some(lap_index)) else {
            return Vec::new();
        };
        let Some(reference_range) = reference_session.history_range(Some(reference_lap_index))
        else {
            return Vec::new();
        };
        let Some(lap_start) = self.history.get(lap_range.start) else {
            return Vec::new();
        };
        let Some(reference_start) = reference_session.history.get(reference_range.start) else {
            return Vec::new();
        };

        let reference = positioned_entries(&reference_session.history, reference_range)
            .map(|(position, entry)| {
                (
                    position,
                    entry.elapsed_seconds - reference_start.elapsed_seconds,
                )
            })
            .collect::<Vec<_>>();

        positioned_entries(&self.history, lap_range)
            .filter_map(|(position, entry)| {
                let elapsed = entry.elapsed_seconds - lap_start.elapsed_seconds;
                interpolate_value_at_position(&reference, position)
                    .map(|reference_elapsed| [position, elapsed - reference_elapsed])
            })
            .collect()
    }

    pub fn comparison_points(
        &self,
        lap_index: usize,
        reference_session: &Self,
        reference_lap_index: usize,
        value: impl Fn(&TelemetrySample) -> f32,
    ) -> Vec<[f64; 2]> {
        self.comparison_points_by(
            lap_index,
            reference_session,
            reference_lap_index,
            |sample| f64::from(value(sample)),
        )
    }

    /// Samples an optional reference channel at the current lap positions.
    pub fn comparison_points_optional(
        &self,
        lap_index: usize,
        reference_session: &Self,
        reference_lap_index: usize,
        value: impl Fn(&TelemetrySample) -> Option<f32>,
    ) -> Vec<[f64; 2]> {
        let Some(lap_range) = self.history_range(Some(lap_index)) else {
            return Vec::new();
        };
        let Some(reference_range) = reference_session.history_range(Some(reference_lap_index))
        else {
            return Vec::new();
        };
        let reference = positioned_entries(&reference_session.history, reference_range)
            .filter_map(|(position, entry)| {
                value(&entry.sample)
                    .filter(|value| value.is_finite())
                    .map(|value| (position, f64::from(value)))
            })
            .collect::<Vec<_>>();

        positioned_entries(&self.history, lap_range)
            .filter_map(|(position, _)| {
                interpolate_value_at_position(&reference, position).map(|value| [position, value])
            })
            .collect()
    }

    pub fn comparison_fuel_used_points(
        &self,
        lap_index: usize,
        reference_session: &Self,
        reference_lap_index: usize,
    ) -> Vec<[f64; 2]> {
        let Some(reference_range) = reference_session.history_range(Some(reference_lap_index))
        else {
            return Vec::new();
        };
        let Some(reference_start) = reference_session.history.get(reference_range.start) else {
            return Vec::new();
        };
        let start_fuel = reference_start.sample.fuel_litres;

        self.comparison_points_by(
            lap_index,
            reference_session,
            reference_lap_index,
            move |sample| f64::from((start_fuel - sample.fuel_litres).max(0.0)),
        )
    }

    fn comparison_points_by(
        &self,
        lap_index: usize,
        reference_session: &Self,
        reference_lap_index: usize,
        value: impl Fn(&TelemetrySample) -> f64,
    ) -> Vec<[f64; 2]> {
        let Some(lap_range) = self.history_range(Some(lap_index)) else {
            return Vec::new();
        };
        let Some(reference_range) = reference_session.history_range(Some(reference_lap_index))
        else {
            return Vec::new();
        };
        let reference = positioned_entries(&reference_session.history, reference_range)
            .map(|(position, entry)| (position, value(&entry.sample)))
            .filter(|(_, value)| value.is_finite())
            .collect::<Vec<_>>();

        positioned_entries(&self.history, lap_range)
            .filter_map(|(position, _)| {
                interpolate_value_at_position(&reference, position).map(|value| [position, value])
            })
            .collect()
    }

    pub fn focused_telemetry(
        &self,
        lap_index: Option<usize>,
        elapsed_seconds: f64,
    ) -> Option<FocusedTelemetry> {
        if !elapsed_seconds.is_finite() {
            return None;
        }

        let range = self.history_range(lap_index)?;
        let first = self.history.get(range.start)?;
        let origin = self.elapsed_origin(lap_index, first);
        let target = origin + elapsed_seconds;
        let mut low = range.start;
        let mut high = range.end;

        while low < high {
            let middle = low + (high - low) / 2;
            if self.history.get(middle)?.elapsed_seconds < target {
                low = middle + 1;
            } else {
                high = middle;
            }
        }

        let right = low.min(range.end - 1);
        let left = right.saturating_sub(1).max(range.start);
        let nearest = [left, right].into_iter().min_by(|left, right| {
            (self.history[*left].elapsed_seconds - target)
                .abs()
                .total_cmp(&(self.history[*right].elapsed_seconds - target).abs())
        })?;
        let entry = self.history.get(nearest)?;

        Some(FocusedTelemetry {
            point_index: nearest - range.start,
            elapsed_seconds: entry.elapsed_seconds - origin,
            sample: entry.sample,
        })
    }

    pub fn focused_telemetry_at_position(
        &self,
        lap_index: usize,
        normalized_position: f32,
    ) -> Option<FocusedTelemetry> {
        if !normalized_position.is_finite() {
            return None;
        }

        let range = self.history_range(Some(lap_index))?;
        let first = self.history.get(range.start)?;
        let target = normalized_position.clamp(0.0, 1.0);
        let (point_index, (_, entry)) = positioned_entries(&self.history, range)
            .enumerate()
            .min_by(|(_, (_, left)), (_, (_, right))| {
                (left.sample.normalized_car_position - target)
                    .abs()
                    .total_cmp(&(right.sample.normalized_car_position - target).abs())
            })?;

        Some(FocusedTelemetry {
            point_index,
            elapsed_seconds: entry.elapsed_seconds - first.elapsed_seconds,
            sample: entry.sample,
        })
    }

    fn history_range(&self, lap_index: Option<usize>) -> Option<Range<usize>> {
        match lap_index {
            Some(index) => self
                .laps
                .get(index)
                .map(|lap| lap.start_index..lap.end_index),
            None => Some(0..self.history.len()),
        }
    }

    fn elapsed_origin(&self, lap_index: Option<usize>, first: &HistoryEntry) -> f64 {
        if lap_index.is_none() && matches!(self.source, SessionSource::Live) {
            0.0
        } else {
            first.elapsed_seconds
        }
    }
}

fn interpolate_value_at_position(reference: &[(f64, f64)], position: f64) -> Option<f64> {
    let right = reference.partition_point(|(reference_position, _)| *reference_position < position);
    match (right.checked_sub(1), reference.get(right)) {
        (None, Some((_, elapsed))) => Some(*elapsed),
        (Some(left), None) => reference.get(left).map(|(_, elapsed)| *elapsed),
        (Some(left), Some((right_position, right_elapsed))) => {
            let (left_position, left_elapsed) = reference[left];
            let span = right_position - left_position;
            if span <= f64::EPSILON {
                Some(*right_elapsed)
            } else {
                let fraction = (position - left_position) / span;
                Some(left_elapsed + (right_elapsed - left_elapsed) * fraction)
            }
        },
        (None, None) => None,
    }
}

fn nearest_value_at_position(reference: &[(f64, f64)], position: f64) -> Option<f64> {
    let right = reference.partition_point(|(reference_position, _)| *reference_position < position);
    match (right.checked_sub(1), reference.get(right)) {
        (None, Some((_, value))) => Some(*value),
        (Some(left), None) => reference.get(left).map(|(_, value)| *value),
        (Some(left), Some((right_position, right_value))) => {
            let (left_position, left_value) = reference[left];
            if position - left_position <= *right_position - position {
                Some(left_value)
            } else {
                Some(*right_value)
            }
        },
        (None, None) => None,
    }
}

fn suppress_transient_neutral_gears(points: &mut [[f64; 2]]) {
    let mut index = 0;
    while index < points.len() {
        if points[index][1].round() as i32 != 0 {
            index += 1;
            continue;
        }

        let neutral_start = index;
        while index < points.len() && points[index][1].round() as i32 == 0 {
            index += 1;
        }
        let neutral_end = index;

        let Some(previous_index) = neutral_start.checked_sub(1) else {
            continue;
        };
        let Some(next) = points.get(neutral_end) else {
            continue;
        };
        let previous_gear = points[previous_index][1].round() as i32;
        let next_gear = next[1].round() as i32;
        let shift_duration = next[0] - points[previous_index][0];

        if previous_gear > 0
            && next_gear > 0
            && shift_duration.is_finite()
            && shift_duration <= TRANSIENT_NEUTRAL_MAX_SECONDS
        {
            for point in &mut points[neutral_start..neutral_end] {
                point[1] = f64::from(previous_gear);
            }
        }
    }
}

fn normalized_track_position(sample: &TelemetrySample) -> Option<f64> {
    (0.0..=1.0)
        .contains(&sample.normalized_car_position)
        .then(|| f64::from(sample.normalized_car_position))
}

fn positioned_entries(
    history: &VecDeque<HistoryEntry>,
    range: Range<usize>,
) -> impl Iterator<Item = (f64, &HistoryEntry)> {
    positioned_entry_indices(history, range)
        .into_iter()
        .map(|(position, index)| (position, &history[index]))
}

fn positioned_entry_indices(
    history: &VecDeque<HistoryEntry>,
    range: Range<usize>,
) -> Vec<(f64, usize)> {
    let mut buckets = vec![None; LAP_DISTANCE_BUCKET_COUNT + 1];
    for (index, entry) in history
        .iter()
        .enumerate()
        .skip(range.start)
        .take(range.len())
    {
        let Some(position) = normalized_track_position(&entry.sample) else {
            continue;
        };
        let bucket = (position * LAP_DISTANCE_BUCKET_COUNT as f64)
            .floor()
            .min(LAP_DISTANCE_BUCKET_COUNT as f64) as usize;
        let chart_position = position * LAP_DISTANCE_AXIS_MAX;
        let candidate = (chart_position, index, entry.sample.speed_kmh);
        if buckets[bucket].is_none_or(|(_, _, speed_kmh)| candidate.2 >= speed_kmh) {
            buckets[bucket] = Some(candidate);
        }
    }
    buckets
        .into_iter()
        .flatten()
        .map(|(position, index, _)| (position, index))
        .collect()
}

fn build_laps(history: &VecDeque<HistoryEntry>) -> Vec<TelemetryLap> {
    if history.is_empty() {
        return Vec::new();
    }

    let mut laps = Vec::new();
    let mut start_index = 0;
    for index in 1..history.len() {
        let previous_completed = history[index - 1].sample.completed_laps;
        let completed = history[index].sample.completed_laps;
        if previous_completed >= 0 && previous_completed.checked_add(1) == Some(completed) {
            laps.push(build_lap(history, start_index, index));
            start_index = index;
        } else if completed != previous_completed {
            // Session resets and the SDK's -1 initialization state are not lap finishes.
            start_index = index;
        }
    }
    laps.push(build_lap(history, start_index, history.len()));
    laps
}

fn build_lap(
    history: &VecDeque<HistoryEntry>,
    start_index: usize,
    end_index: usize,
) -> TelemetryLap {
    let first = history
        .get(start_index)
        .expect("lap range must start inside telemetry history");
    let last = history
        .get(end_index - 1)
        .expect("lap range must end inside telemetry history");
    let has_finish_boundary = end_index < history.len();
    let starts_near_line = first.sample.current_lap_ms <= 1_000
        || (0.0..=0.05).contains(&first.sample.normalized_car_position);
    let complete = has_finish_boundary && starts_near_line;
    let recorded_duration_ms =
        seconds_to_milliseconds((last.elapsed_seconds - first.elapsed_seconds).max(0.0));
    let duration_ms = if complete {
        completed_lap_duration_ms(history, end_index).unwrap_or(recorded_duration_ms)
    } else {
        recorded_duration_ms
    };

    TelemetryLap {
        number: first.sample.completed_laps.saturating_add(1).max(1),
        start_index,
        end_index,
        duration_ms,
        complete,
    }
}

fn completed_lap_duration_ms(
    history: &VecDeque<HistoryEntry>,
    boundary_index: usize,
) -> Option<i32> {
    let last_before_boundary = history.get(boundary_index.checked_sub(1)?)?;
    let boundary = history.get(boundary_index)?;
    let stale_last_lap_ms = last_before_boundary.sample.last_lap_ms;
    let next_lap = boundary.sample.completed_laps;

    history
        .iter()
        .skip(boundary_index)
        .take_while(|entry| entry.sample.completed_laps == next_lap)
        .map(|entry| entry.sample.last_lap_ms)
        .find(|duration_ms| *duration_ms > 0 && *duration_ms != stale_last_lap_ms)
        .or_else(|| {
            (last_before_boundary.sample.current_lap_ms > 0)
                .then_some(last_before_boundary.sample.current_lap_ms)
        })
}

fn seconds_to_milliseconds(seconds: f64) -> i32 {
    (seconds * 1_000.0).round().clamp(0.0, f64::from(i32::MAX)) as i32
}

fn crossed_sector(previous: f32, current: f32, sector_starts: &[f64]) -> bool {
    let previous = f64::from(previous);
    let current = f64::from(current);
    current > previous
        && sector_starts
            .iter()
            .any(|boundary| previous < *boundary && *boundary <= current)
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        time::{Duration, Instant},
    };

    use chiaro_irsdk::{
        OptionalTelemetryValues, SessionInfo, TelemetryFrame, TelemetrySample, TelemetrySnapshot,
        TelemetryValue,
    };

    use super::{
        ConnectionStatus, HistoryEntry, LIVE_HISTORY_RETENTION, LIVE_HISTORY_SAMPLE_LIMIT, Session,
        TelemetryLap, build_laps, suppress_transient_neutral_gears,
    };
    use crate::ibt::{IbtInfo, LoadedIbt, RecordingSource, TimedSample};

    #[test]
    fn connection_request_transitions_through_connecting() {
        let mut session = Session::default();
        session.set_connection_requested(true);
        assert_eq!(session.connection(), ConnectionStatus::Connecting);
        session.mark_connected();
        assert_eq!(session.connection(), ConnectionStatus::Connected);
        session.set_connection_requested(false);
        assert_eq!(session.connection(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn empty_session_has_no_preferred_lap() {
        assert_eq!(Session::default().preferred_lap_index(), None);
    }

    #[test]
    fn connection_changes_clear_previous_telemetry() {
        let mut session = Session::default();
        session.record_sample(TelemetrySample::default());

        session.set_connection_requested(true);

        assert_eq!(session.packets_received(), 0);
        assert!(session.latest().is_none());
        assert!(session.history.is_empty());
    }

    #[test]
    fn records_latest_sample_and_chart_points() {
        let mut session = Session::default();
        let sample = TelemetrySample {
            throttle: 0.75,
            ..TelemetrySample::default()
        };
        session.record_sample(sample);

        assert_eq!(session.latest(), Some(&sample));
        assert_eq!(session.packets_received(), 1);
        assert_eq!(
            session.points_in(None, |sample| sample.throttle),
            vec![[0.0, 0.75]]
        );
    }

    #[test]
    fn calculates_fuel_used_from_the_start_of_the_selected_range() {
        let session = Session {
            history: VecDeque::from([
                HistoryEntry {
                    elapsed_seconds: 10.0,
                    sample: TelemetrySample {
                        fuel_litres: 40.0,
                        ..TelemetrySample::default()
                    },
                },
                HistoryEntry {
                    elapsed_seconds: 12.0,
                    sample: TelemetrySample {
                        fuel_litres: 39.25,
                        ..TelemetrySample::default()
                    },
                },
            ]),
            ..Session::default()
        };

        assert_eq!(
            session.fuel_used_points(None),
            vec![[10.0, 0.0], [12.0, 0.75]]
        );
    }

    #[test]
    fn reports_fuel_remaining_at_the_start_of_each_lap() {
        let session = Session {
            history: VecDeque::from([
                HistoryEntry {
                    elapsed_seconds: 0.0,
                    sample: TelemetrySample {
                        fuel_litres: 40.0,
                        ..TelemetrySample::default()
                    },
                },
                HistoryEntry {
                    elapsed_seconds: 1.0,
                    sample: TelemetrySample {
                        fuel_litres: 39.5,
                        ..TelemetrySample::default()
                    },
                },
                HistoryEntry {
                    elapsed_seconds: 2.0,
                    sample: TelemetrySample {
                        fuel_litres: 38.0,
                        ..TelemetrySample::default()
                    },
                },
            ]),
            laps: vec![
                TelemetryLap {
                    number: 1,
                    start_index: 0,
                    end_index: 2,
                    duration_ms: 1_000,
                    complete: true,
                },
                TelemetryLap {
                    number: 2,
                    start_index: 2,
                    end_index: 3,
                    duration_ms: 0,
                    complete: false,
                },
            ],
            ..Session::default()
        };

        assert_eq!(session.lap_start_fuel_litres(0), Some(40.0));
        assert_eq!(session.lap_start_fuel_litres(1), Some(38.0));
        assert_eq!(session.lap_start_fuel_litres(2), None);
    }

    #[test]
    fn compares_laps_from_different_sessions_by_track_position() {
        let session_with_lap = |start: f64, duration: f64, speeds: [f32; 3]| Session {
            history: [0.0_f32, 0.5, 1.0]
                .into_iter()
                .zip(speeds)
                .enumerate()
                .map(|(index, (position, speed_kmh))| HistoryEntry {
                    elapsed_seconds: start + duration * index as f64 / 2.0,
                    sample: TelemetrySample {
                        normalized_car_position: position,
                        speed_kmh,
                        ..TelemetrySample::default()
                    },
                })
                .collect(),
            laps: vec![TelemetryLap {
                number: 1,
                start_index: 0,
                end_index: 3,
                duration_ms: super::seconds_to_milliseconds(duration),
                complete: true,
            }],
            ..Session::default()
        };
        let current = session_with_lap(10.0, 10.0, [90.0, 100.0, 110.0]);
        let reference = session_with_lap(30.0, 12.0, [100.0, 120.0, 140.0]);

        assert_eq!(
            current.comparison_points(0, &reference, 0, |sample| sample.speed_kmh),
            vec![[0.0, 100.0], [5_000.0, 120.0], [10_000.0, 140.0]]
        );
        assert_eq!(
            current.lap_delta_points_against(0, &reference, 0),
            vec![[0.0, 0.0], [5_000.0, -1.0], [10_000.0, -2.0]]
        );

        let focused = reference
            .focused_telemetry_at_position(0, 0.48)
            .expect("reference lap contains the requested position");
        assert_eq!(focused.point_index, 1);
        assert_eq!(focused.elapsed_seconds, 6.0);
        assert_eq!(focused.sample.speed_kmh, 120.0);
    }

    #[test]
    fn optional_comparison_points_interpolate_only_available_reference_values() {
        let current = Session {
            history: [0.0_f32, 0.25, 0.5, 1.0]
                .into_iter()
                .enumerate()
                .map(|(index, normalized_car_position)| HistoryEntry {
                    elapsed_seconds: index as f64,
                    sample: TelemetrySample {
                        normalized_car_position,
                        ..TelemetrySample::default()
                    },
                })
                .collect(),
            laps: vec![TelemetryLap {
                number: 1,
                start_index: 0,
                end_index: 4,
                duration_ms: 3_000,
                complete: true,
            }],
            ..Session::default()
        };
        let reference = Session {
            history: [(0.0_f32, Some(10.0)), (0.5, None), (1.0, Some(30.0))]
                .into_iter()
                .enumerate()
                .map(|(index, (normalized_car_position, torque))| HistoryEntry {
                    elapsed_seconds: index as f64,
                    sample: TelemetrySample {
                        normalized_car_position,
                        steering_wheel_torque_nm: OptionalTelemetryValues::from_options([torque]),
                        ..TelemetrySample::default()
                    },
                })
                .collect(),
            laps: vec![TelemetryLap {
                number: 1,
                start_index: 0,
                end_index: 3,
                duration_ms: 2_000,
                complete: true,
            }],
            ..Session::default()
        };

        assert_eq!(
            current.comparison_points_optional(0, &reference, 0, |sample| {
                sample.steering_wheel_torque_nm.get(0)
            }),
            vec![
                [0.0, 10.0],
                [2_500.0, 15.0],
                [5_000.0, 20.0],
                [10_000.0, 30.0],
            ]
        );
        assert!(
            current
                .comparison_points_optional(0, &reference, 0, |sample| {
                    sample.brake_line_pressure_bar.get(0)
                })
                .is_empty()
        );
    }

    #[test]
    fn comparison_gears_filter_neutral_before_discrete_position_alignment() {
        let session_with_gears = |positions: &[f32], times: &[f64], gears: &[i32]| Session {
            history: positions
                .iter()
                .zip(times)
                .zip(gears)
                .map(
                    |((&normalized_car_position, &elapsed_seconds), &gear)| HistoryEntry {
                        elapsed_seconds,
                        sample: TelemetrySample {
                            normalized_car_position,
                            gear,
                            ..TelemetrySample::default()
                        },
                    },
                )
                .collect(),
            laps: vec![TelemetryLap {
                number: 1,
                start_index: 0,
                end_index: positions.len(),
                duration_ms: super::seconds_to_milliseconds(
                    times.last().copied().unwrap_or_default(),
                ),
                complete: true,
            }],
            ..Session::default()
        };
        let current = session_with_gears(
            &[0.0, 0.125, 0.375, 0.625, 0.875, 1.0],
            &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
            &[1, 1, 1, 1, 1, 1],
        );
        let reference = session_with_gears(
            &[0.0, 0.25, 0.5, 0.75, 1.0],
            &[0.0, 0.1, 0.2, 0.3, 0.4],
            &[3, 0, 4, 4, 4],
        );

        assert_eq!(
            current.comparison_gear_points(0, &reference, 0),
            vec![
                [0.0, 3.0],
                [1_250.0, 3.0],
                [3_750.0, 3.0],
                [6_250.0, 4.0],
                [8_750.0, 4.0],
                [10_000.0, 4.0],
            ]
        );

        assert_eq!(
            reference.gear_points(Some(0)),
            vec![
                [0.0, 3.0],
                [2_500.0, 3.0],
                [5_000.0, 4.0],
                [7_500.0, 4.0],
                [10_000.0, 4.0],
            ]
        );
    }

    #[test]
    fn gear_filter_preserves_sustained_neutral_and_reverse_transitions() {
        let mut sustained = vec![[0.0, 3.0], [0.1, 0.0], [0.5, 0.0], [0.6, 4.0]];
        suppress_transient_neutral_gears(&mut sustained);
        assert_eq!(
            sustained,
            vec![[0.0, 3.0], [0.1, 0.0], [0.5, 0.0], [0.6, 4.0]]
        );

        let mut reverse = vec![[0.0, -1.0], [0.1, 0.0], [0.2, 1.0]];
        suppress_transient_neutral_gears(&mut reverse);
        assert_eq!(reverse, vec![[0.0, -1.0], [0.1, 0.0], [0.2, 1.0]]);
    }

    #[test]
    fn records_full_frames_and_session_info() {
        let sample = TelemetrySample {
            packet_id: 42,
            rpm: 6_000,
            ..TelemetrySample::default()
        };
        let frame = TelemetryFrame::try_new(
            42,
            vec![chiaro_irsdk::VariableMetadata {
                name: "RPM".to_owned(),
                description: "Engine revolutions per minute".to_owned(),
                unit: "revs/min".to_owned(),
                value_type: chiaro_irsdk::VariableType::Float,
                count: 1,
                count_as_time: false,
            }],
            vec![TelemetryValue::Float(6_000.0)],
        )
        .expect("valid telemetry frame");
        let info = SessionInfo {
            update_count: 3,
            yaml: "WeekendInfo:".to_owned(),
            raw: b"WeekendInfo:".to_vec(),
        };
        let mut session = Session::default();

        session.record_snapshot(
            TelemetrySnapshot {
                sample,
                frame: frame.clone(),
            },
            Some(info.clone()),
        );

        assert_eq!(session.latest(), Some(&sample));
        assert_eq!(session.latest_frame(), Some(&frame));
        assert_eq!(session.session_info(), Some(&info));
        assert_eq!(session.session_info_revision(), 1);
        assert_eq!(session.packets_received(), 1);
    }

    #[test]
    fn session_info_revision_changes_when_a_reconnect_reuses_the_sdk_counter() {
        let first = SessionInfo {
            update_count: 3,
            yaml: "CarSetup:\n  UpdateCount: 1".to_owned(),
            raw: Vec::new(),
        };
        let second = SessionInfo {
            update_count: 3,
            yaml: "CarSetup:\n  UpdateCount: 2".to_owned(),
            raw: Vec::new(),
        };
        let mut session = Session::default();

        session.record_live_batch(
            std::iter::empty::<(Instant, TelemetrySample)>(),
            None,
            Some(first),
        );
        let first_revision = session.session_info_revision();
        session.set_connection_requested(true);
        let cleared_revision = session.session_info_revision();
        session.record_live_batch(
            std::iter::empty::<(Instant, TelemetrySample)>(),
            None,
            Some(second.clone()),
        );

        assert!(cleared_revision > first_revision);
        assert!(session.session_info_revision() > cleared_revision);
        assert_eq!(session.session_info(), Some(&second));
    }

    #[test]
    fn live_batch_uses_producer_capture_times_and_keeps_optional_metadata() {
        let frame = TelemetryFrame::try_new(
            8,
            Vec::<chiaro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let info = SessionInfo {
            update_count: 4,
            yaml: "WeekendInfo:".to_owned(),
            raw: b"WeekendInfo:".to_vec(),
        };
        let captured_at = Instant::now();
        let first = TelemetrySample {
            packet_id: 7,
            throttle: 0.25,
            ..TelemetrySample::default()
        };
        let second = TelemetrySample {
            packet_id: 8,
            throttle: 0.75,
            ..TelemetrySample::default()
        };
        let mut session = Session::default();

        session.record_live_batch(
            [
                (captured_at, first),
                (captured_at + Duration::from_millis(17), second),
            ],
            Some(frame.clone()),
            Some(info.clone()),
        );

        let points = session.points_in(None, |sample| sample.throttle);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0], [0.0, 0.25]);
        assert!((points[1][0] - 0.017).abs() < 1e-9);
        assert_eq!(points[1][1], 0.75);
        assert_eq!(session.latest(), Some(&second));
        assert_eq!(session.latest_frame(), Some(&frame));
        assert_eq!(session.session_info(), Some(&info));
        assert_eq!(session.packets_received(), 2);
    }

    #[test]
    fn live_chart_points_use_session_elapsed_time() {
        let first = TelemetrySample {
            throttle: 0.25,
            ..TelemetrySample::default()
        };
        let second = TelemetrySample {
            throttle: 0.75,
            ..TelemetrySample::default()
        };
        let session = Session {
            history: VecDeque::from([
                HistoryEntry {
                    elapsed_seconds: 10.0,
                    sample: first,
                },
                HistoryEntry {
                    elapsed_seconds: 15.0,
                    sample: second,
                },
            ]),
            ..Session::default()
        };

        assert_eq!(
            session.points_in(None, |sample| sample.throttle),
            vec![[10.0, 0.25], [15.0, 0.75]]
        );
        assert_eq!(session.live_chart_time_bounds(), Some((10.0, 15.0)));
        let focused = session
            .focused_telemetry(None, 15.0)
            .expect("latest live sample");
        assert_eq!(focused.point_index, 1);
        assert_eq!(focused.elapsed_seconds, 15.0);
    }

    #[test]
    fn live_points_since_returns_only_unseen_retained_points() {
        let session = Session {
            packets_received: 5,
            history: VecDeque::from([
                HistoryEntry {
                    elapsed_seconds: 3.0,
                    sample: TelemetrySample {
                        throttle: 0.25,
                        ..TelemetrySample::default()
                    },
                },
                HistoryEntry {
                    elapsed_seconds: 4.0,
                    sample: TelemetrySample {
                        throttle: 0.5,
                        ..TelemetrySample::default()
                    },
                },
                HistoryEntry {
                    elapsed_seconds: 5.0,
                    sample: TelemetrySample {
                        throttle: 0.75,
                        ..TelemetrySample::default()
                    },
                },
            ]),
            ..Session::default()
        };

        assert_eq!(
            session.live_points_since(3, |sample| sample.throttle),
            vec![[4.0, 0.5], [5.0, 0.75]]
        );
        assert_eq!(
            session.live_points_since(0, |sample| sample.throttle),
            vec![[3.0, 0.25], [4.0, 0.5], [5.0, 0.75]]
        );
        assert!(
            session
                .live_points_since(5, |sample| sample.throttle)
                .is_empty()
        );
        assert!(
            session
                .live_points_since(10, |sample| sample.throttle)
                .is_empty()
        );
    }

    #[test]
    fn live_chart_minimum_x_tracks_the_retained_history_front() {
        let mut session = Session {
            history: VecDeque::from([
                HistoryEntry {
                    elapsed_seconds: 5.0,
                    sample: TelemetrySample::default(),
                },
                HistoryEntry {
                    elapsed_seconds: 20.0,
                    sample: TelemetrySample::default(),
                },
            ]),
            ..Session::default()
        };

        assert_eq!(session.live_chart_minimum_x(), Some(5.0));

        session.history.pop_front();
        assert_eq!(session.live_chart_minimum_x(), Some(20.0));
    }

    #[test]
    fn live_value_range_ignores_non_finite_values() {
        let session = Session {
            history: [-0.25, f32::NAN, f32::INFINITY, 0.75]
                .into_iter()
                .enumerate()
                .map(|(index, throttle)| HistoryEntry {
                    elapsed_seconds: index as f64,
                    sample: TelemetrySample {
                        throttle,
                        ..TelemetrySample::default()
                    },
                })
                .collect(),
            ..Session::default()
        };

        assert_eq!(
            session.live_value_range(|sample| sample.throttle),
            Some((-0.25, 0.75))
        );
        assert_eq!(session.live_value_range(|_| f32::NAN), None);
    }

    #[test]
    fn optional_points_and_ranges_skip_unavailable_and_non_finite_values() {
        let session = Session {
            packets_received: 4,
            history: [Some(3.0), None, Some(f32::NAN), Some(-1.0)]
                .into_iter()
                .enumerate()
                .map(|(index, pressure)| HistoryEntry {
                    elapsed_seconds: 10.0 + index as f64,
                    sample: TelemetrySample {
                        brake_line_pressure_bar: OptionalTelemetryValues::from_options([
                            pressure, None, None, None,
                        ]),
                        ..TelemetrySample::default()
                    },
                })
                .collect(),
            ..Session::default()
        };

        assert_eq!(
            session.points_in_optional(None, |sample| sample.brake_line_pressure_bar.get(0)),
            vec![[10.0, 3.0], [13.0, -1.0]]
        );
        assert_eq!(
            session
                .live_points_since_optional(1, |sample| { sample.brake_line_pressure_bar.get(0) }),
            vec![[13.0, -1.0]]
        );
        assert_eq!(
            session.live_value_range_optional(|sample| sample.brake_line_pressure_bar.get(0)),
            Some((-1.0, 3.0))
        );
        assert!(
            session
                .points_in_optional(None, |sample| sample.brake_line_pressure_bar.get(1))
                .is_empty()
        );
        assert_eq!(
            session.live_value_range_optional(|sample| sample.brake_line_pressure_bar.get(1)),
            None
        );
    }

    #[test]
    fn selected_lap_chart_points_are_ordered_by_position_and_keep_focus_aligned() {
        let history = [-1.0_f32, 0.25, 0.25, 0.2, 0.75]
            .into_iter()
            .enumerate()
            .map(|(index, normalized_car_position)| HistoryEntry {
                elapsed_seconds: index as f64 * 5.0,
                sample: TelemetrySample {
                    normalized_car_position,
                    throttle: index as f32 / 4.0,
                    ..TelemetrySample::default()
                },
            })
            .collect();
        let session = Session {
            history,
            laps: vec![TelemetryLap {
                number: 1,
                start_index: 0,
                end_index: 5,
                duration_ms: 20_000,
                complete: true,
            }],
            ..Session::default()
        };

        let points = session.points_in(Some(0), |sample| sample.throttle);
        assert_eq!(points.len(), 3);
        assert!((points[0][0] - 2_000.0).abs() < 1e-3);
        assert_eq!(points[0][1], 0.75);
        assert_eq!(points[1], [2_500.0, 0.5]);
        assert_eq!(points[2], [7_500.0, 1.0]);
        let focused = session
            .focused_telemetry_at_position(0, 0.3)
            .expect("lap contains a nearby valid track position");
        assert_eq!(focused.point_index, 1);
        assert_eq!(focused.sample.normalized_car_position, 0.25);
    }

    #[test]
    fn live_chart_keeps_all_retained_points_and_focus_aligned() {
        let history = (0..LIVE_HISTORY_SAMPLE_LIMIT)
            .map(|index| HistoryEntry {
                elapsed_seconds: index as f64 / 60.0,
                sample: TelemetrySample {
                    throttle: (index % 2) as f32,
                    ..TelemetrySample::default()
                },
            })
            .collect();
        let session = Session {
            history,
            ..Session::default()
        };

        let points = session.points_in(None, |sample| sample.throttle);
        let latest_time = (LIVE_HISTORY_SAMPLE_LIMIT - 1) as f64 / 60.0;
        let focused = session
            .focused_telemetry(None, latest_time)
            .expect("latest retained point");

        assert_eq!(points.len(), LIVE_HISTORY_SAMPLE_LIMIT);
        assert_eq!(points[0][1], 0.0);
        assert_eq!(points[1][1], 1.0);
        assert_eq!(points[2][1], 0.0);
        assert_eq!(points.last().map(|point| point[0]), Some(latest_time));
        assert_eq!(focused.point_index, points.len() - 1);
        assert_eq!(focused.elapsed_seconds, latest_time);
    }

    #[test]
    fn live_history_rolls_over_by_sample_count_and_elapsed_time() {
        let mut session = Session {
            history: (0..=LIVE_HISTORY_SAMPLE_LIMIT)
                .map(|index| HistoryEntry {
                    elapsed_seconds: index as f64 / 100.0,
                    sample: TelemetrySample::default(),
                })
                .collect(),
            ..Session::default()
        };

        let latest = session
            .history
            .back()
            .expect("latest sample")
            .elapsed_seconds;
        session.trim_live_history(latest);

        assert_eq!(session.history.len(), LIVE_HISTORY_SAMPLE_LIMIT);
        assert_eq!(
            session.history.front().map(|entry| entry.elapsed_seconds),
            Some(0.01)
        );

        session.history.push_back(HistoryEntry {
            elapsed_seconds: LIVE_HISTORY_RETENTION.as_secs_f64() + 1.0,
            sample: TelemetrySample::default(),
        });
        session.trim_live_history(LIVE_HISTORY_RETENTION.as_secs_f64() + 1.0);

        assert!(
            session
                .history
                .front()
                .is_some_and(|entry| entry.elapsed_seconds >= 1.0)
        );
    }

    #[test]
    fn live_timing_history_survives_chart_history_trimming() {
        let started_at = Instant::now();
        let mut session = Session::default();
        for (seconds, completed_laps, last_lap_ms) in [(0, 0, 0), (60, 1, 60_000), (120, 2, 60_000)]
        {
            session.record_sample_at(
                started_at + Duration::from_secs(seconds),
                TelemetrySample {
                    completed_laps,
                    current_lap_ms: 0,
                    last_lap_ms,
                    normalized_car_position: 0.0,
                    fuel_litres: 40.0 - seconds as f32 / 30.0,
                    ..TelemetrySample::default()
                },
            );
        }

        assert_eq!(session.history.len(), 1);
        assert_eq!(
            session
                .lap_timings()
                .iter()
                .filter(|lap| lap.is_complete())
                .count(),
            2
        );
        assert_eq!(session.stints().len(), 1);
    }

    #[test]
    fn live_history_rollover_preserves_the_remaining_chart_points() {
        let mut session = Session {
            history: (0..LIVE_HISTORY_SAMPLE_LIMIT)
                .map(|index| HistoryEntry {
                    elapsed_seconds: index as f64 / 60.0,
                    sample: TelemetrySample {
                        throttle: index as f32,
                        ..TelemetrySample::default()
                    },
                })
                .collect(),
            ..Session::default()
        };
        let before = session.points_in(None, |sample| sample.throttle);

        session.history.push_back(HistoryEntry {
            elapsed_seconds: LIVE_HISTORY_RETENTION.as_secs_f64(),
            sample: TelemetrySample {
                throttle: LIVE_HISTORY_SAMPLE_LIMIT as f32,
                ..TelemetrySample::default()
            },
        });
        session.trim_live_history(LIVE_HISTORY_RETENTION.as_secs_f64());
        let after = session.points_in(None, |sample| sample.throttle);

        assert_eq!(after.len(), LIVE_HISTORY_SAMPLE_LIMIT);
        assert_eq!(&before[1..], &after[..after.len() - 1]);
        assert_eq!(
            after.last(),
            Some(&[
                LIVE_HISTORY_RETENTION.as_secs_f64(),
                LIVE_HISTORY_SAMPLE_LIMIT as f64,
            ])
        );
    }

    #[test]
    fn live_chart_time_remains_monotonic_across_a_lap_boundary() {
        let session = Session {
            history: VecDeque::from([
                HistoryEntry {
                    elapsed_seconds: 19.0,
                    sample: TelemetrySample {
                        completed_laps: 0,
                        normalized_car_position: 0.99,
                        throttle: 0.25,
                        ..TelemetrySample::default()
                    },
                },
                HistoryEntry {
                    elapsed_seconds: 20.0,
                    sample: TelemetrySample {
                        completed_laps: 1,
                        normalized_car_position: 0.0,
                        throttle: 0.75,
                        ..TelemetrySample::default()
                    },
                },
            ]),
            ..Session::default()
        };

        assert_eq!(
            session.points_in(None, |sample| sample.throttle),
            vec![[19.0, 0.25], [20.0, 0.75]]
        );
    }

    #[test]
    fn loads_an_ibt_recording_on_its_recorded_timeline() {
        let first = TelemetrySample {
            throttle: 0.25,
            ..TelemetrySample::default()
        };
        let last = TelemetrySample {
            throttle: 0.75,
            ..TelemetrySample::default()
        };
        let frame = TelemetryFrame::try_new(
            1,
            Vec::<chiaro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let mut session = Session::default();

        session.load_ibt(LoadedIbt {
            info: IbtInfo {
                source: RecordingSource::local_file("session.ibt"),
                file_name: "session.ibt".to_owned(),
                track_name: "Test Circuit".to_owned(),
                track_id: None,
                track_config_name: None,
                car_id: None,
                car_name: None,
                duration_seconds: 20.0,
                lap_count: 2,
                record_count: 1_200,
                tick_rate: 60,
            },
            samples: vec![
                TimedSample {
                    elapsed_seconds: 0.0,
                    sample: first,
                },
                TimedSample {
                    elapsed_seconds: 20.0,
                    sample: last,
                },
            ],
            latest_frame: frame,
            session_info: SessionInfo {
                update_count: 1,
                yaml: String::new(),
                raw: Vec::new(),
            },
        });

        assert!(!session.wants_connection());
        assert_eq!(session.packets_received(), 1_200);
        assert_eq!(session.latest(), Some(&last));
        assert_eq!(session.chart_duration_seconds(), 20.0);
        assert_eq!(session.laps().len(), 1);
        assert_eq!(session.laps()[0].number(), 1);
        assert!(!session.laps()[0].is_complete());
        assert_eq!(
            session.points_in(None, |sample| sample.throttle),
            vec![[0.0, 0.25], [20.0, 0.75]]
        );
        assert_eq!(
            session.ibt_info().map(|info| info.track_name.as_str()),
            Some("Test Circuit")
        );

        session.set_connection_requested(true);
        assert!(session.ibt_info().is_none());
    }

    #[test]
    fn splits_ibt_history_into_complete_and_partial_laps() {
        let timed_sample = |elapsed_seconds,
                            completed_laps,
                            current_lap_ms,
                            last_lap_ms,
                            normalized_car_position,
                            throttle| TimedSample {
            elapsed_seconds,
            sample: TelemetrySample {
                completed_laps,
                current_lap_ms,
                last_lap_ms,
                normalized_car_position,
                throttle,
                ..TelemetrySample::default()
            },
        };
        let frame = TelemetryFrame::try_new(
            5,
            Vec::<chiaro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let mut session = Session::default();

        session.load_ibt(LoadedIbt {
            info: IbtInfo {
                source: RecordingSource::local_file("laps.ibt"),
                file_name: "laps.ibt".to_owned(),
                track_name: "Test Circuit".to_owned(),
                track_id: None,
                track_config_name: None,
                car_id: None,
                car_name: None,
                duration_seconds: 130.0,
                lap_count: 2,
                record_count: 6,
                tick_rate: 60,
            },
            samples: vec![
                timed_sample(0.0, 0, 0, 0, 0.0, 0.1),
                timed_sample(59.0, 0, 59_000, 0, 0.99, 0.2),
                timed_sample(60.0, 1, 0, 60_000, 0.0, 0.3),
                timed_sample(118.0, 1, 58_000, 60_000, 0.98, 0.4),
                timed_sample(119.0, 2, 0, 59_000, 0.0, 0.5),
                timed_sample(130.0, 2, 11_000, 59_000, 0.2, 0.6),
            ],
            latest_frame: frame,
            session_info: SessionInfo {
                update_count: 1,
                yaml: String::new(),
                raw: Vec::new(),
            },
        });

        assert_eq!(session.laps().len(), 3);
        assert_eq!(session.laps()[0].duration_ms(), 60_000);
        assert!(session.laps()[0].is_complete());
        assert_eq!(session.laps()[1].number(), 2);
        assert_eq!(session.laps()[1].duration_ms(), 59_000);
        assert!(session.laps()[1].is_complete());
        assert!(!session.laps()[2].is_complete());
        assert_eq!(session.preferred_lap_index(), Some(1));
        assert_eq!(session.fastest_complete_lap_index(), Some(1));
        let points = session.points_in(Some(1), |sample| sample.throttle);
        assert_eq!(points.len(), 2);
        assert_eq!(points[0][0], 0.0);
        assert!((points[1][0] - 9_800.0).abs() < 1e-3);
        assert!((points[0][1] - 0.3).abs() < 1e-6);
        assert!((points[1][1] - 0.4).abs() < 1e-6);

        let focused = session
            .focused_telemetry(Some(1), 40.0)
            .expect("focused point");
        assert_eq!(focused.point_index, 1);
        assert_eq!(focused.elapsed_seconds, 58.0);
        assert_eq!(focused.sample.throttle, 0.4);

        let delta = session.lap_delta_points(0, 1);
        assert_eq!(delta.len(), 2);
        assert_eq!(delta[0], [0.0, 0.0]);
        assert!((delta[1][0] - 9_900.0).abs() < 1e-3);
        assert!((delta[1][1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn waits_for_the_last_lap_time_to_update_after_the_finish_boundary() {
        let entry = |elapsed_seconds,
                     completed_laps,
                     current_lap_ms,
                     last_lap_ms,
                     normalized_car_position| HistoryEntry {
            elapsed_seconds,
            sample: TelemetrySample {
                completed_laps,
                current_lap_ms,
                last_lap_ms,
                normalized_car_position,
                ..TelemetrySample::default()
            },
        };
        let history = VecDeque::from([
            entry(0.0, 0, 0, 0, 0.0),
            entry(113.25, 0, 113_283, 0, 0.99),
            entry(113.30, 1, 113_316, 0, 0.0),
            entry(113.40, 1, 100, 113_298, 0.001),
            entry(223.00, 1, 109_752, 113_298, 0.99),
            entry(223.05, 2, 109_769, 113_298, 0.0),
            entry(223.15, 2, 100, 109_749, 0.001),
        ]);

        let laps = build_laps(&history);

        assert_eq!(laps.len(), 3);
        assert_eq!(laps[0].duration_ms(), 113_298);
        assert_eq!(laps[1].number(), 2);
        assert_eq!(laps[1].duration_ms(), 109_749);
        assert!(laps[0].is_complete());
        assert!(laps[1].is_complete());
        assert!(!laps[2].is_complete());
    }

    #[test]
    fn discards_lap_counter_initialization_and_reset_segments() {
        let entry = |elapsed_seconds,
                     completed_laps,
                     current_lap_ms,
                     last_lap_ms,
                     normalized_car_position| HistoryEntry {
            elapsed_seconds,
            sample: TelemetrySample {
                completed_laps,
                current_lap_ms,
                last_lap_ms,
                normalized_car_position,
                ..TelemetrySample::default()
            },
        };
        let history = VecDeque::from([
            entry(0.0, 0, 0, 0, 0.0),
            entry(5.0, 0, 5_000, 0, 0.1),
            entry(6.0, -1, 0, 0, 0.0),
            entry(7.0, 0, 0, 0, 0.0),
            entry(17.0, 0, 10_000, 0, 0.99),
            entry(17.1, 1, 10_020, 0, 0.0),
            entry(17.2, 1, 100, 10_000, 0.001),
        ]);

        let laps = build_laps(&history);

        assert_eq!(laps.len(), 2);
        assert_eq!(laps[0].number(), 1);
        assert_eq!(laps[0].duration_ms(), 10_000);
        assert!(laps[0].is_complete());
        assert_eq!(laps[1].number(), 2);
        assert!(!laps[1].is_complete());
    }

    #[test]
    fn ignores_non_finite_samples() {
        let mut session = Session::default();
        session.record_sample(TelemetrySample {
            speed_kmh: f32::NAN,
            ..TelemetrySample::default()
        });
        assert_eq!(session.packets_received(), 0);
        assert!(session.latest().is_none());
    }
}
