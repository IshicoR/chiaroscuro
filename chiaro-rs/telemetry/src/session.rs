use std::{
    collections::VecDeque,
    ops::Range,
    time::{Duration, Instant},
};

use chiaroscuro_irsdk::{SessionInfo, TelemetryFrame, TelemetrySample, TelemetrySnapshot};

use crate::ibt::{IbtInfo, LoadedIbt};

pub const HISTORY_WINDOW: Duration = Duration::from_secs(12);
const LIVE_HISTORY_RETENTION: Duration = Duration::from_secs(5 * 60);
const LIVE_HISTORY_SAMPLE_LIMIT: usize = 5 * 60 * 60;
const LIVE_CHART_SAMPLE_LIMIT: usize = 2_048;
const TRANSIENT_NEUTRAL_MAX_SECONDS: f64 = 0.35;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
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
    history: VecDeque<HistoryEntry>,
    laps: Vec<TelemetryLap>,
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

    pub fn laps(&self) -> &[TelemetryLap] {
        &self.laps
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
        self.session_info = None;
        self.history.clear();
        self.laps.clear();
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
        self.session_info = Some(session_info);
        self.history = samples
            .into_iter()
            .map(|entry| HistoryEntry {
                elapsed_seconds: entry.elapsed_seconds,
                sample: entry.sample,
            })
            .collect();
        self.laps = build_laps(&self.history);
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
        if self.ibt_info().is_some() || !sample.is_finite() {
            return;
        }

        let now = Instant::now();
        let started_at = *self.live_started_at.get_or_insert(now);
        let elapsed_seconds = now.saturating_duration_since(started_at).as_secs_f64();
        self.latest = Some(sample);
        self.packets_received = self.packets_received.saturating_add(1);
        self.history.push_back(HistoryEntry {
            elapsed_seconds,
            sample,
        });

        self.trim_live_history(elapsed_seconds);
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
            self.session_info = Some(session_info);
        }
        self.record_sample(snapshot.sample);
    }

    pub fn points_in(
        &self,
        lap_index: Option<usize>,
        value: impl Fn(&TelemetrySample) -> f32,
    ) -> Vec<[f64; 2]> {
        let Some(range) = self.history_range(lap_index) else {
            return Vec::new();
        };
        let Some(first) = self.history.get(range.start) else {
            return Vec::new();
        };
        let origin = self.elapsed_origin(lap_index, first);
        let stride = self.chart_sample_stride(lap_index, range.len());

        let mut points = self
            .history
            .iter()
            .skip(range.start)
            .take(range.len())
            .step_by(stride)
            .map(|entry| {
                let elapsed = entry.elapsed_seconds - origin;
                [elapsed, f64::from(value(&entry.sample))]
            })
            .collect::<Vec<_>>();
        if (range.len() - 1) % stride != 0
            && let Some(last) = self.history.get(range.end - 1)
        {
            points.push([
                last.elapsed_seconds - origin,
                f64::from(value(&last.sample)),
            ]);
        }
        points
    }

    pub fn gear_points(&self, lap_index: Option<usize>) -> Vec<[f64; 2]> {
        let mut points = self.points_in(lap_index, |sample| sample.gear as f32);
        suppress_transient_neutral_gears(&mut points);
        points
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
        let Some(lap_start) = self.history.get(lap_range.start) else {
            return Vec::new();
        };
        let Some(reference_start) = reference_session.history.get(reference_range.start) else {
            return Vec::new();
        };

        let reference_samples = reference_session
            .history
            .iter()
            .skip(reference_range.start)
            .take(reference_range.len())
            .filter_map(|entry| {
                normalized_track_position(&entry.sample).map(|position| {
                    (
                        position,
                        entry.elapsed_seconds - reference_start.elapsed_seconds,
                        f64::from(entry.sample.gear),
                    )
                })
            })
            .collect::<Vec<_>>();
        let mut timed_gears = reference_samples
            .iter()
            .map(|(_, elapsed, gear)| [*elapsed, *gear])
            .collect::<Vec<_>>();
        suppress_transient_neutral_gears(&mut timed_gears);

        let mut positioned_gears = reference_samples
            .into_iter()
            .zip(timed_gears)
            .map(|((position, _, _), point)| (position, point[1]))
            .collect::<Vec<_>>();
        prepare_positioned_values(&mut positioned_gears);

        self.history
            .iter()
            .skip(lap_range.start)
            .take(lap_range.len())
            .filter_map(|entry| {
                let elapsed = entry.elapsed_seconds - lap_start.elapsed_seconds;
                let position = normalized_track_position(&entry.sample)?;
                nearest_value_at_position(&positioned_gears, position).map(|gear| [elapsed, gear])
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
        let origin = self.elapsed_origin(lap_index, first);
        let start_fuel = first.sample.fuel_litres;
        let stride = self.chart_sample_stride(lap_index, range.len());

        let mut points = self
            .history
            .iter()
            .skip(range.start)
            .take(range.len())
            .step_by(stride)
            .map(|entry| {
                [
                    entry.elapsed_seconds - origin,
                    f64::from((start_fuel - entry.sample.fuel_litres).max(0.0)),
                ]
            })
            .collect::<Vec<_>>();
        if (range.len() - 1) % stride != 0
            && let Some(last) = self.history.get(range.end - 1)
        {
            points.push([
                last.elapsed_seconds - origin,
                f64::from((start_fuel - last.sample.fuel_litres).max(0.0)),
            ]);
        }
        points
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

        let mut reference = reference_session
            .history
            .iter()
            .skip(reference_range.start)
            .take(reference_range.len())
            .filter_map(|entry| {
                normalized_track_position(&entry.sample).map(|position| {
                    (
                        position,
                        entry.elapsed_seconds - reference_start.elapsed_seconds,
                    )
                })
            })
            .collect::<Vec<_>>();
        prepare_positioned_values(&mut reference);

        self.history
            .iter()
            .skip(lap_range.start)
            .take(lap_range.len())
            .filter_map(|entry| {
                let elapsed = entry.elapsed_seconds - lap_start.elapsed_seconds;
                let position = normalized_track_position(&entry.sample)?;
                interpolate_value_at_position(&reference, position)
                    .map(|reference_elapsed| [elapsed, elapsed - reference_elapsed])
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
        let Some(lap_start) = self.history.get(lap_range.start) else {
            return Vec::new();
        };

        let mut reference = reference_session
            .history
            .iter()
            .skip(reference_range.start)
            .take(reference_range.len())
            .filter_map(|entry| {
                normalized_track_position(&entry.sample)
                    .map(|position| (position, value(&entry.sample)))
            })
            .filter(|(_, value)| value.is_finite())
            .collect::<Vec<_>>();
        prepare_positioned_values(&mut reference);

        self.history
            .iter()
            .skip(lap_range.start)
            .take(lap_range.len())
            .filter_map(|entry| {
                let elapsed = entry.elapsed_seconds - lap_start.elapsed_seconds;
                let position = normalized_track_position(&entry.sample)?;
                interpolate_value_at_position(&reference, position).map(|value| [elapsed, value])
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
        let stride = self.chart_sample_stride(lap_index, range.len());
        let offset = nearest - range.start;
        let before = offset / stride * stride;
        let after = (before + stride).min(range.len() - 1);
        let sampled_offset = [before, after].into_iter().min_by(|left, right| {
            (self.history[range.start + *left].elapsed_seconds - target)
                .abs()
                .total_cmp(&(self.history[range.start + *right].elapsed_seconds - target).abs())
        })?;
        let nearest = range.start + sampled_offset;
        let entry = self.history.get(nearest)?;

        Some(FocusedTelemetry {
            point_index: sampled_offset.div_ceil(stride),
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
        let (point_index, entry) = self
            .history
            .iter()
            .skip(range.start)
            .take(range.len())
            .enumerate()
            .filter(|(_, entry)| normalized_track_position(&entry.sample).is_some())
            .min_by(|(_, left), (_, right)| {
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

    fn chart_sample_stride(&self, lap_index: Option<usize>, sample_count: usize) -> usize {
        if lap_index.is_none() && matches!(self.source, SessionSource::Live) {
            sample_count.div_ceil(LIVE_CHART_SAMPLE_LIMIT).max(1)
        } else {
            1
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

fn prepare_positioned_values(values: &mut Vec<(f64, f64)>) {
    values.sort_by(|left, right| left.0.total_cmp(&right.0));
    values.dedup_by(|left, right| (left.0 - right.0).abs() <= f64::EPSILON);
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

#[cfg(test)]
mod tests {
    use std::{collections::VecDeque, path::PathBuf};

    use chiaroscuro_irsdk::{
        SessionInfo, TelemetryFrame, TelemetrySample, TelemetrySnapshot, TelemetryValue,
    };

    use super::{
        ConnectionStatus, HistoryEntry, LIVE_CHART_SAMPLE_LIMIT, LIVE_HISTORY_RETENTION,
        LIVE_HISTORY_SAMPLE_LIMIT, Session, TelemetryLap, build_laps,
        suppress_transient_neutral_gears,
    };
    use crate::ibt::{IbtInfo, LoadedIbt, TimedSample};

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
            vec![[0.0, 100.0], [5.0, 120.0], [10.0, 140.0]]
        );
        assert_eq!(
            current.lap_delta_points_against(0, &reference, 0),
            vec![[0.0, 0.0], [5.0, -1.0], [10.0, -2.0]]
        );

        let focused = reference
            .focused_telemetry_at_position(0, 0.48)
            .expect("reference lap contains the requested position");
        assert_eq!(focused.point_index, 1);
        assert_eq!(focused.elapsed_seconds, 6.0);
        assert_eq!(focused.sample.speed_kmh, 120.0);
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
                [1.0, 3.0],
                [2.0, 3.0],
                [3.0, 4.0],
                [4.0, 4.0],
                [5.0, 4.0],
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
            vec![chiaroscuro_irsdk::VariableMetadata {
                name: "RPM".to_owned(),
                description: "Engine revolutions per minute".to_owned(),
                unit: "revs/min".to_owned(),
                value_type: chiaroscuro_irsdk::VariableType::Float,
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
        assert_eq!(session.packets_received(), 1);
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
    fn live_chart_sampling_keeps_the_latest_point_and_focus_aligned() {
        let sample_count = LIVE_CHART_SAMPLE_LIMIT * 2;
        let history = (0..sample_count)
            .map(|index| HistoryEntry {
                elapsed_seconds: index as f64,
                sample: TelemetrySample {
                    throttle: index as f32,
                    ..TelemetrySample::default()
                },
            })
            .collect();
        let session = Session {
            history,
            ..Session::default()
        };

        let points = session.points_in(None, |sample| sample.throttle);
        let focused = session
            .focused_telemetry(None, (sample_count - 1) as f64)
            .expect("latest sampled point");

        assert_eq!(points.len(), LIVE_CHART_SAMPLE_LIMIT + 1);
        assert_eq!(
            points.last().map(|point| point[0]),
            Some((sample_count - 1) as f64)
        );
        assert_eq!(focused.point_index, points.len() - 1);
        assert_eq!(focused.elapsed_seconds, (sample_count - 1) as f64);
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
            Vec::<chiaroscuro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let mut session = Session::default();

        session.load_ibt(LoadedIbt {
            info: IbtInfo {
                path: PathBuf::from("session.ibt"),
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
            Vec::<chiaroscuro_irsdk::VariableMetadata>::new(),
            Vec::<TelemetryValue>::new(),
        )
        .expect("valid empty frame");
        let mut session = Session::default();

        session.load_ibt(LoadedIbt {
            info: IbtInfo {
                path: PathBuf::from("laps.ibt"),
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
        assert_eq!(points[1][0], 58.0);
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
        assert!((delta[1][0] - 59.0).abs() < 1e-6);
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
