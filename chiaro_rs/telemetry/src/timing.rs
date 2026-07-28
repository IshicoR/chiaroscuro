use chiaro_irsdk::{SessionInfo, TelemetrySample};

#[derive(Debug, Clone, Copy)]
pub(crate) struct TimingEntry {
    pub elapsed_seconds: f64,
    pub current_lap_ms: i32,
    pub last_lap_ms: i32,
    pub completed_laps: i32,
    pub normalized_car_position: f32,
    pub fuel_litres: f32,
    pub in_pit: bool,
}

impl TimingEntry {
    pub(crate) fn new(elapsed_seconds: f64, sample: TelemetrySample) -> Self {
        Self {
            elapsed_seconds,
            current_lap_ms: sample.current_lap_ms,
            last_lap_ms: sample.last_lap_ms,
            completed_laps: sample.completed_laps,
            normalized_car_position: sample.normalized_car_position,
            fuel_litres: sample.fuel_litres,
            in_pit: sample.in_pit,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LapTiming {
    number: i32,
    duration_ms: i32,
    sectors_ms: Vec<Option<i32>>,
    complete: bool,
    start_fuel_litres: Option<f32>,
    finish_index: usize,
}

impl LapTiming {
    pub const fn number(&self) -> i32 {
        self.number
    }

    pub const fn duration_ms(&self) -> i32 {
        self.duration_ms
    }

    pub fn sectors_ms(&self) -> &[Option<i32>] {
        &self.sectors_ms
    }

    pub const fn is_complete(&self) -> bool {
        self.complete
    }

    pub const fn start_fuel_litres(&self) -> Option<f32> {
        self.start_fuel_litres
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SectorCrossing {
    sector_index: usize,
    lap_number: i32,
    elapsed_seconds: f64,
}

impl SectorCrossing {
    pub const fn sector_index(self) -> usize {
        self.sector_index
    }

    pub const fn lap_number(self) -> i32 {
        self.lap_number
    }

    pub const fn elapsed_seconds(self) -> f64 {
        self.elapsed_seconds
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StintTiming {
    number: usize,
    first_lap: i32,
    last_lap: i32,
    lap_count: usize,
    best_lap_ms: Option<i32>,
    average_lap_ms: Option<i32>,
    start_fuel_litres: f32,
    fuel_used_litres: f32,
    complete: bool,
}

impl StintTiming {
    pub const fn number(self) -> usize {
        self.number
    }

    pub const fn first_lap(self) -> i32 {
        self.first_lap
    }

    pub const fn last_lap(self) -> i32 {
        self.last_lap
    }

    pub const fn lap_count(self) -> usize {
        self.lap_count
    }

    pub const fn best_lap_ms(self) -> Option<i32> {
        self.best_lap_ms
    }

    pub const fn average_lap_ms(self) -> Option<i32> {
        self.average_lap_ms
    }

    pub const fn fuel_used_litres(self) -> f32 {
        self.fuel_used_litres
    }

    pub const fn is_complete(self) -> bool {
        self.complete
    }
}

pub(crate) fn sector_starts(session_info: Option<&SessionInfo>) -> Vec<f64> {
    let Some(mut starts) = session_info
        .and_then(|info| info.parse().ok())
        .and_then(|document| document.split_time_info)
        .map(|split| {
            split
                .sectors
                .into_iter()
                .filter_map(|sector| sector.sector_start_pct)
                .filter(|start| start.is_finite() && (0.0..1.0).contains(start))
                .collect::<Vec<_>>()
        })
    else {
        return Vec::new();
    };
    if starts.is_empty() {
        return starts;
    }
    if !starts.iter().any(|start| *start <= 0.001) {
        starts.push(0.0);
    }
    starts.sort_by(f64::total_cmp);
    starts.dedup_by(|left, right| (*left - *right).abs() < 0.000_001);
    starts
}

pub(crate) fn build(
    entries: &[TimingEntry],
    sector_starts: &[f64],
) -> (Vec<LapTiming>, Vec<StintTiming>) {
    let ranges = lap_ranges(entries);
    let laps = ranges
        .iter()
        .map(|&(start, end, complete)| build_lap(entries, start, end, complete, sector_starts))
        .collect::<Vec<_>>();
    let stints = build_stints(entries, &laps);
    (laps, stints)
}

pub(crate) fn sector_crossings(
    entries: &[TimingEntry],
    sector_starts: &[f64],
) -> Vec<SectorCrossing> {
    if sector_starts.is_empty() {
        return Vec::new();
    }

    let internal_boundaries = sector_starts
        .iter()
        .copied()
        .filter(|start| *start > 0.001)
        .collect::<Vec<_>>();
    let final_sector_index = sector_starts.len().saturating_sub(1);
    let mut crossings = Vec::new();

    for pair in entries.windows(2) {
        let previous = pair[0];
        let current = pair[1];
        if previous.completed_laps == current.completed_laps {
            for (sector_index, boundary) in internal_boundaries.iter().copied().enumerate() {
                if let Some(elapsed_seconds) =
                    interpolate_crossing_seconds(previous, current, boundary)
                {
                    crossings.push(SectorCrossing {
                        sector_index,
                        lap_number: previous.completed_laps.saturating_add(1).max(1),
                        elapsed_seconds,
                    });
                }
            }
        } else if previous.completed_laps.checked_add(1) == Some(current.completed_laps) {
            let elapsed_seconds = (current.elapsed_seconds
                - f64::from(current.current_lap_ms.max(0)) / 1_000.0)
                .clamp(previous.elapsed_seconds, current.elapsed_seconds);
            crossings.push(SectorCrossing {
                sector_index: final_sector_index,
                lap_number: current.completed_laps.max(1),
                elapsed_seconds,
            });
        }
    }

    crossings
}

pub(crate) fn update_active(
    laps: &mut [LapTiming],
    stints: &mut [StintTiming],
    entry: TimingEntry,
) -> bool {
    let mut changed = false;
    if let Some(lap) = laps.last_mut()
        && !lap.complete
        && lap.number == entry.completed_laps.saturating_add(1).max(1)
        && entry.current_lap_ms.saturating_sub(lap.duration_ms) >= 100
    {
        lap.duration_ms = entry.current_lap_ms;
        changed = true;
    }
    if let Some(stint) = stints.last_mut()
        && !stint.complete
    {
        let fuel_used = (stint.start_fuel_litres - entry.fuel_litres).max(0.0);
        if (fuel_used - stint.fuel_used_litres).abs() >= 0.05 {
            stint.fuel_used_litres = fuel_used;
            changed = true;
        }
    }
    changed
}

fn lap_ranges(entries: &[TimingEntry]) -> Vec<(usize, usize, bool)> {
    if entries.is_empty() {
        return Vec::new();
    }
    let mut ranges = Vec::new();
    let mut start = 0;
    for index in 1..entries.len() {
        let previous = entries[index - 1].completed_laps;
        let current = entries[index].completed_laps;
        if previous >= 0 && previous.checked_add(1) == Some(current) {
            ranges.push((start, index, true));
            start = index;
        } else if previous != current {
            start = index;
        }
    }
    ranges.push((start, entries.len(), false));
    ranges
}

fn build_lap(
    entries: &[TimingEntry],
    start: usize,
    end: usize,
    has_boundary: bool,
    sector_starts: &[f64],
) -> LapTiming {
    let first = entries[start];
    let last = entries[end - 1];
    let starts_near_line =
        first.current_lap_ms <= 1_000 || (0.0..=0.05).contains(&first.normalized_car_position);
    let complete = has_boundary && starts_near_line;
    let recorded_end = if complete {
        entries.get(end).copied().unwrap_or(last)
    } else {
        last
    };
    let recorded_ms =
        ((recorded_end.elapsed_seconds - first.elapsed_seconds).max(0.0) * 1_000.0).round() as i32;
    let duration_ms = if complete {
        let stale_last_lap_ms = last.last_lap_ms;
        entries
            .get(end)
            .into_iter()
            .flat_map(|boundary| {
                entries[end..]
                    .iter()
                    .take_while(|entry| entry.completed_laps == boundary.completed_laps)
            })
            .map(|entry| entry.last_lap_ms)
            .find(|duration| *duration > 0 && *duration != stale_last_lap_ms)
            .unwrap_or(recorded_ms)
    } else {
        recorded_ms
    };
    let mut sectors_ms = Vec::new();
    let mut previous_split_ms = 0;
    for boundary in sector_starts.iter().copied().filter(|start| *start > 0.001) {
        let split_ms = entries[start..end]
            .windows(2)
            .find_map(|pair| interpolate_crossing_ms(pair[0], pair[1], boundary));
        sectors_ms.push(split_ms.map(|split| {
            let sector = split.saturating_sub(previous_split_ms);
            previous_split_ms = split;
            sector
        }));
    }
    if !sector_starts.is_empty() {
        sectors_ms.push(complete.then(|| duration_ms.saturating_sub(previous_split_ms)));
    }

    LapTiming {
        number: first.completed_laps.saturating_add(1).max(1),
        duration_ms,
        sectors_ms,
        complete,
        start_fuel_litres: first.fuel_litres.is_finite().then_some(first.fuel_litres),
        finish_index: end.saturating_sub(1),
    }
}

fn interpolate_crossing_ms(
    previous: TimingEntry,
    current: TimingEntry,
    boundary: f64,
) -> Option<i32> {
    if previous.completed_laps != current.completed_laps {
        return None;
    }
    let from = f64::from(previous.normalized_car_position);
    let to = f64::from(current.normalized_car_position);
    if !(from < boundary && boundary <= to) || to <= from {
        return None;
    }
    let ratio = (boundary - from) / (to - from);
    let from_ms = f64::from(previous.current_lap_ms);
    let to_ms = f64::from(current.current_lap_ms);
    Some((from_ms + (to_ms - from_ms) * ratio).round().max(0.0) as i32)
}

fn interpolate_crossing_seconds(
    previous: TimingEntry,
    current: TimingEntry,
    boundary: f64,
) -> Option<f64> {
    if previous.completed_laps != current.completed_laps {
        return None;
    }
    let from = f64::from(previous.normalized_car_position);
    let to = f64::from(current.normalized_car_position);
    if !(from < boundary && boundary <= to) || to <= from {
        return None;
    }
    let ratio = (boundary - from) / (to - from);
    Some(previous.elapsed_seconds + (current.elapsed_seconds - previous.elapsed_seconds) * ratio)
}

fn build_stints(entries: &[TimingEntry], laps: &[LapTiming]) -> Vec<StintTiming> {
    let mut stints = Vec::new();
    let mut index = 0;
    while index < entries.len() {
        while index < entries.len() && entries[index].in_pit {
            index += 1;
        }
        if index == entries.len() {
            break;
        }
        let start = index;
        while index < entries.len() && !entries[index].in_pit {
            index += 1;
        }
        let end = index;
        let completed = laps
            .iter()
            .filter(|lap| lap.complete && (start..end).contains(&lap.finish_index))
            .collect::<Vec<_>>();
        let durations = completed
            .iter()
            .map(|lap| lap.duration_ms)
            .filter(|duration| *duration > 0)
            .collect::<Vec<_>>();
        let first_lap = entries[start].completed_laps.saturating_add(1).max(1);
        let last_lap = completed.last().map_or(first_lap, |lap| lap.number);
        let fuel_used =
            (entries[start].fuel_litres - entries[end.saturating_sub(1)].fuel_litres).max(0.0);
        let average_lap_ms = (!durations.is_empty()).then(|| {
            i32::try_from(
                durations
                    .iter()
                    .map(|duration| i64::from(*duration))
                    .sum::<i64>()
                    / durations.len() as i64,
            )
            .unwrap_or(i32::MAX)
        });
        stints.push(StintTiming {
            number: stints.len() + 1,
            first_lap,
            last_lap,
            lap_count: completed.len(),
            best_lap_ms: durations.iter().copied().min(),
            average_lap_ms,
            start_fuel_litres: entries[start].fuel_litres,
            fuel_used_litres: fuel_used,
            complete: end < entries.len(),
        });
    }
    stints
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        elapsed_seconds: f64,
        completed_laps: i32,
        lap_ms: i32,
        position: f32,
        fuel: f32,
        in_pit: bool,
    ) -> TimingEntry {
        TimingEntry::new(
            elapsed_seconds,
            TelemetrySample {
                completed_laps,
                current_lap_ms: lap_ms,
                normalized_car_position: position,
                fuel_litres: fuel,
                in_pit,
                ..TelemetrySample::default()
            },
        )
    }

    #[test]
    fn computes_sector_times_at_interpolated_boundaries() {
        let entries = [
            entry(0.0, 0, 0, 0.0, 40.0, false),
            entry(20.0, 0, 20_000, 0.2, 39.5, false),
            entry(40.0, 0, 40_000, 0.5, 39.0, false),
            entry(70.0, 0, 70_000, 0.8, 38.5, false),
            entry(90.0, 1, 0, 0.0, 38.0, false),
        ];

        let (laps, _) = build(&entries, &[0.0, 0.3, 0.7]);

        assert_eq!(
            laps[0].sectors_ms(),
            &[Some(26_667), Some(33_333), Some(30_000)]
        );
        assert!(laps[0].is_complete());
    }

    #[test]
    fn records_interpolated_sector_crossings_on_the_chart_timeline() {
        let entries = [
            entry(0.0, 0, 0, 0.0, 40.0, false),
            entry(20.0, 0, 20_000, 0.2, 39.5, false),
            entry(40.0, 0, 40_000, 0.5, 39.0, false),
            entry(70.0, 0, 70_000, 0.8, 38.5, false),
            entry(90.1, 1, 100, 0.001, 38.0, false),
        ];

        let crossings = sector_crossings(&entries, &[0.0, 0.3, 0.7]);

        assert_eq!(crossings.len(), 3);
        assert_eq!(crossings[0].sector_index(), 0);
        assert_eq!(crossings[0].lap_number(), 1);
        assert!((crossings[0].elapsed_seconds() - 26.666_666).abs() < 0.001);
        assert_eq!(crossings[1].sector_index(), 1);
        assert!((crossings[1].elapsed_seconds() - 60.0).abs() < 0.001);
        assert_eq!(crossings[2].sector_index(), 2);
        assert!((crossings[2].elapsed_seconds() - 90.0).abs() < 0.001);
    }

    #[test]
    fn missing_split_metadata_does_not_invent_a_single_sector() {
        assert!(sector_starts(None).is_empty());
    }

    #[test]
    fn pit_transitions_split_stints_and_keep_the_active_stint() {
        let entries = [
            entry(0.0, 0, 0, 0.0, 40.0, true),
            entry(5.0, 0, 5_000, 0.1, 40.0, false),
            entry(60.0, 1, 0, 0.0, 38.0, false),
            entry(120.0, 2, 0, 0.0, 36.0, false),
            entry(150.0, 2, 30_000, 0.5, 35.0, true),
            entry(160.0, 2, 40_000, 0.6, 45.0, false),
            entry(220.0, 3, 0, 0.0, 43.0, false),
        ];

        let (_, stints) = build(&entries, &[0.0, 0.5]);

        assert_eq!(stints.len(), 2);
        assert_eq!(stints[0].lap_count(), 2);
        assert!(stints[0].is_complete());
        assert_eq!(stints[1].lap_count(), 1);
        assert!(!stints[1].is_complete());
    }
}
