use std::path::{Path, PathBuf};

use chiaro_irsdk::{IbtFile, SessionInfo, TelemetryFrame, TelemetrySample};

const BASE_CHART_SAMPLES: usize = 10_000;
const CHART_SAMPLES_PER_LAP: usize = 3_600;
const MAX_CHART_SAMPLES: usize = 150_000;

#[derive(Debug, Clone)]
pub struct TimedSample {
    pub elapsed_seconds: f64,
    pub sample: TelemetrySample,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordingOrigin {
    LocalFile,
    CachedRemote { provider: String, object_id: String },
}

/// A recording resolved to a local readable file.
///
/// Cloud integrations can download an object into a cache and retain its
/// provider identity here. The IBT parser therefore stays independent from
/// authentication, storage APIs, and download transports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordingSource {
    local_path: PathBuf,
    display_name: String,
    origin: RecordingOrigin,
}

impl RecordingSource {
    pub fn local_file(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let display_name = file_name(&path);
        Self {
            local_path: path,
            display_name,
            origin: RecordingOrigin::LocalFile,
        }
    }

    pub fn cached_remote(
        local_path: impl Into<PathBuf>,
        display_name: impl Into<String>,
        provider: impl Into<String>,
        object_id: impl Into<String>,
    ) -> Self {
        Self {
            local_path: local_path.into(),
            display_name: display_name.into(),
            origin: RecordingOrigin::CachedRemote {
                provider: provider.into(),
                object_id: object_id.into(),
            },
        }
    }

    pub fn local_path(&self) -> &Path {
        &self.local_path
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn origin(&self) -> &RecordingOrigin {
        &self.origin
    }

    pub fn description(&self) -> String {
        match &self.origin {
            RecordingOrigin::LocalFile => self.local_path.display().to_string(),
            RecordingOrigin::CachedRemote { provider, .. } => {
                format!("{} · {provider}", self.display_name)
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct IbtInfo {
    pub source: RecordingSource,
    pub file_name: String,
    pub track_name: String,
    pub track_id: Option<i32>,
    pub track_config_name: Option<String>,
    pub car_id: Option<i32>,
    pub car_name: Option<String>,
    pub duration_seconds: f64,
    pub lap_count: usize,
    pub record_count: usize,
    pub tick_rate: u32,
}

#[derive(Debug, Clone)]
pub struct LoadedIbt {
    pub info: IbtInfo,
    pub samples: Vec<TimedSample>,
    pub latest_frame: TelemetryFrame,
    pub session_info: SessionInfo,
}

pub fn load_ibt(path: &Path) -> Result<LoadedIbt, String> {
    load_ibt_source(&RecordingSource::local_file(path))
}

pub fn load_ibt_source(source: &RecordingSource) -> Result<LoadedIbt, String> {
    let path = source.local_path();
    let file_name = source.display_name().to_owned();
    let file = IbtFile::open(path)
        .map_err(|error| format!("Failed to open {}: {error}", source.description()))?;
    let metadata = *file.metadata();

    if file.is_empty() {
        return Err(format!("{file_name} contains no telemetry records"));
    }
    let session_info = file.session_info().clone();
    load_open_ibt(file, metadata, session_info, source.clone(), file_name)
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

fn load_open_ibt(
    mut file: IbtFile,
    metadata: chiaro_irsdk::IbtMetadata,
    session_info: SessionInfo,
    source: RecordingSource,
    file_name: String,
) -> Result<LoadedIbt, String> {
    let document = session_info.parse().ok();
    let weekend = document
        .as_ref()
        .and_then(|document| document.weekend_info.as_ref());
    let track_name = weekend
        .and_then(|weekend| {
            weekend
                .track_display_name
                .clone()
                .or_else(|| weekend.track_name.clone())
        })
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "Unknown track".to_owned());
    let track_id = weekend.and_then(|weekend| weekend.track_id);
    let track_config_name = weekend
        .and_then(|weekend| weekend.track_config_name.clone())
        .filter(|name| !name.trim().is_empty());
    let driver_info = document
        .as_ref()
        .and_then(|document| document.driver_info.as_ref());
    let driver = driver_info.and_then(|driver_info| {
        let player_index = driver_info.driver_car_idx?;
        driver_info
            .drivers
            .iter()
            .find(|driver| driver.car_idx == Some(player_index))
    });
    let car_id = driver.and_then(|driver| driver.car_id);
    let car_name = driver
        .and_then(|driver| {
            driver
                .car_screen_name
                .clone()
                .or_else(|| driver.car_screen_name_short.clone())
                .or_else(|| driver.car_path.clone())
        })
        .filter(|name| !name.trim().is_empty());
    let metadata_duration = metadata.duration_seconds();
    let duration_seconds = if metadata_duration > 0.0 || metadata.record_count <= 1 {
        metadata_duration
    } else {
        (metadata.record_count - 1) as f64 / f64::from(metadata.tick_rate)
    };
    let sample_limit = chart_sample_limit(metadata.record_count, metadata.lap_count);
    let indices = sampled_indices(metadata.record_count, sample_limit);
    let mut samples = Vec::with_capacity(indices.len());

    for index in indices {
        let sample = file.read_sample(index).map_err(|error| {
            format!(
                "Failed to read record {} from {file_name}: {error}",
                index + 1
            )
        })?;
        if !sample.is_finite() {
            return Err(format!(
                "Record {} in {file_name} contains a non-finite value",
                index + 1
            ));
        }

        samples.push(TimedSample {
            elapsed_seconds: elapsed_at(index, metadata.record_count, duration_seconds),
            sample,
        });
    }

    let latest_frame = file
        .read_frame(metadata.record_count - 1)
        .map_err(|error| format!("Failed to read the last frame from {file_name}: {error}"))?;

    Ok(LoadedIbt {
        info: IbtInfo {
            source,
            file_name,
            track_name,
            track_id,
            track_config_name,
            car_id,
            car_name,
            duration_seconds,
            lap_count: metadata.lap_count,
            record_count: metadata.record_count,
            tick_rate: metadata.tick_rate,
        },
        samples,
        latest_frame,
        session_info,
    })
}

fn sampled_indices(record_count: usize, limit: usize) -> Vec<usize> {
    if record_count <= limit {
        return (0..record_count).collect();
    }

    let last = (record_count - 1) as u128;
    let denominator = (limit - 1) as u128;
    (0..limit)
        .map(|position| ((position as u128 * last) / denominator) as usize)
        .collect()
}

fn chart_sample_limit(record_count: usize, lap_count: usize) -> usize {
    let lap_target = lap_count
        .saturating_add(1)
        .saturating_mul(CHART_SAMPLES_PER_LAP);
    BASE_CHART_SAMPLES
        .max(lap_target)
        .min(MAX_CHART_SAMPLES)
        .min(record_count)
}

fn elapsed_at(index: usize, record_count: usize, duration_seconds: f64) -> f64 {
    if record_count <= 1 {
        0.0
    } else {
        duration_seconds * index as f64 / (record_count - 1) as f64
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        RecordingOrigin, RecordingSource, chart_sample_limit, elapsed_at, sampled_indices,
    };

    #[test]
    fn local_recording_source_keeps_the_resolved_file_identity() {
        let source = RecordingSource::local_file("recordings/session.ibt");

        assert_eq!(source.local_path(), Path::new("recordings/session.ibt"));
        assert_eq!(source.display_name(), "session.ibt");
        assert_eq!(source.origin(), &RecordingOrigin::LocalFile);
        assert_eq!(source.description(), "recordings/session.ibt");
    }

    #[test]
    fn cached_remote_source_keeps_cloud_identity_separate_from_its_cache_path() {
        let source = RecordingSource::cached_remote(
            "cache/recording.ibt",
            "Nordschleife.ibt",
            "Chiaroscuro Cloud",
            "recording-42",
        );

        assert_eq!(source.local_path(), Path::new("cache/recording.ibt"));
        assert_eq!(source.display_name(), "Nordschleife.ibt");
        assert_eq!(source.description(), "Nordschleife.ibt · Chiaroscuro Cloud");
        assert_eq!(
            source.origin(),
            &RecordingOrigin::CachedRemote {
                provider: "Chiaroscuro Cloud".to_owned(),
                object_id: "recording-42".to_owned(),
            }
        );
    }

    #[test]
    fn sampling_keeps_both_ends_of_a_recording() {
        assert_eq!(sampled_indices(5, 3), vec![0, 2, 4]);
        assert_eq!(sampled_indices(3, 3), vec![0, 1, 2]);
    }

    #[test]
    fn sampled_time_spans_the_recording_duration() {
        assert_eq!(elapsed_at(0, 5, 20.0), 0.0);
        assert_eq!(elapsed_at(4, 5, 20.0), 20.0);
    }

    #[test]
    fn sampling_budget_scales_with_the_number_of_laps() {
        assert_eq!(chart_sample_limit(500_000, 1), 10_000);
        assert_eq!(chart_sample_limit(500_000, 20), 75_600);
        assert_eq!(chart_sample_limit(500_000, 100), 150_000);
        assert_eq!(chart_sample_limit(4_000, 20), 4_000);
    }
}
