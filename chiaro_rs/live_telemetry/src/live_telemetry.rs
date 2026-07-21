use std::{
    io, mem,
    time::{Duration, Instant},
};

use chiaro_irsdk::{Client, SessionInfo, TelemetryFrame, TelemetrySample, TelemetrySnapshot};
use chiaro_telemetry::LiveTelemetrySourceInfo;
use iced::{
    Subscription,
    futures::{SinkExt, Stream},
    stream,
};
use smol::{Timer, block_on, unblock};

const EVENT_WAIT_TIMEOUT: Duration = Duration::from_millis(250);
const BATCH_MAX_LATENCY: Duration = Duration::from_millis(16);
const BATCH_SAMPLE_CAPACITY: usize = 2;
const FULL_FRAME_INTERVAL: Duration = Duration::from_secs(1);
const OUTPUT_BUFFER_CAPACITY: usize = 2;
const RETRY_DELAY: Duration = Duration::from_secs(2);
const IRACING_SHARED_MEMORY_ID: &str = "iracing_shared_memory";
const IRACING_SHARED_MEMORY_NAME: &str = "iRacing on this PC";
const IRACING_WINDOWS_ONLY_REASON: &str =
    "Live iRacing telemetry is available only on Windows; IBT recordings remain available.";

/// A selectable live telemetry transport.
///
/// The desktop owns this selection while [`LiveTelemetryMessage`] remains
/// transport-neutral. A cloud transport can therefore be added as another
/// variant without changing the dashboard or telemetry session model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub enum LiveTelemetrySource {
    #[default]
    IracingSharedMemory,
}

impl LiveTelemetrySource {
    pub const fn info(&self) -> LiveTelemetrySourceInfo {
        match self {
            Self::IracingSharedMemory => {
                if cfg!(target_os = "windows") {
                    LiveTelemetrySourceInfo::available(
                        IRACING_SHARED_MEMORY_ID,
                        IRACING_SHARED_MEMORY_NAME,
                    )
                } else {
                    LiveTelemetrySourceInfo::unavailable(
                        IRACING_SHARED_MEMORY_ID,
                        IRACING_SHARED_MEMORY_NAME,
                        IRACING_WINDOWS_ONLY_REASON,
                    )
                }
            },
        }
    }
}

/// One telemetry sample stamped immediately after it was copied from iRacing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapturedTelemetrySample {
    pub captured_at: Instant,
    pub sample: TelemetrySample,
}

/// A small, allocation-free producer batch. Normal 60 Hz samples cross the
/// 16 ms latency boundary individually; only samples in the same short burst
/// are coalesced, up to the fixed capacity.
///
/// The optional full frame is intentionally refreshed at a lower frequency:
/// charts only need [`TelemetrySample`], while the desktop keeps the most
/// recent frame for named SDK values and diagnostics.
#[derive(Debug, Clone)]
pub struct LiveTelemetryBatch {
    pub samples: [Option<CapturedTelemetrySample>; BATCH_SAMPLE_CAPACITY],
    pub latest_frame: Option<TelemetryFrame>,
    pub session_info: Option<SessionInfo>,
}

impl Default for LiveTelemetryBatch {
    fn default() -> Self {
        Self {
            samples: [None; BATCH_SAMPLE_CAPACITY],
            latest_frame: None,
            session_info: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum LiveTelemetryMessage {
    Waiting,
    Connected,
    Batch(LiveTelemetryBatch),
    /// Kept for callers that still submit a same-frame sample/frame pair.
    Snapshot {
        snapshot: TelemetrySnapshot,
        session_info: Option<SessionInfo>,
    },
    Error(String),
}

pub fn subscription(source: &LiveTelemetrySource) -> Subscription<LiveTelemetryMessage> {
    if !source.info().is_available() {
        return Subscription::none();
    }

    match source {
        LiveTelemetrySource::IracingSharedMemory => Subscription::run(client_stream),
    }
}

fn client_stream() -> impl Stream<Item = LiveTelemetryMessage> + 'static {
    stream::channel(OUTPUT_BUFFER_CAPACITY, async move |mut output| {
        loop {
            if output.send(LiveTelemetryMessage::Waiting).await.is_err() {
                return;
            }

            let (next_output, result) = unblock(move || stream_samples(output)).await;
            output = next_output;
            if output.is_closed() {
                return;
            }

            if let Err(error) = result
                && output
                    .send(LiveTelemetryMessage::Error(error.to_string()))
                    .await
                    .is_err()
            {
                return;
            }

            Timer::after(RETRY_DELAY).await;
        }
    })
}

fn stream_samples(
    mut output: iced::futures::channel::mpsc::Sender<LiveTelemetryMessage>,
) -> (
    iced::futures::channel::mpsc::Sender<LiveTelemetryMessage>,
    io::Result<()>,
) {
    let result = stream_samples_inner(&mut output);
    (output, result)
}

fn stream_samples_inner(
    output: &mut iced::futures::channel::mpsc::Sender<LiveTelemetryMessage>,
) -> io::Result<()> {
    let mut client = Client::connect()?;
    let mut connected = false;
    let mut pending = PendingBatch::default();
    let mut last_full_frame_at = None;

    loop {
        if output.is_closed() {
            return Ok(());
        }

        let now = Instant::now();
        if pending.is_due(now) && !send_pending(output, &mut pending) {
            return Ok(());
        }

        let sample = match client.wait_for_update(pending.wait_timeout(Instant::now())) {
            Ok(sample) => sample,
            Err(error) => {
                let _ = send_pending(output, &mut pending);
                return Err(error);
            },
        };
        let Some(sample) = sample else {
            continue;
        };
        let captured_at = Instant::now();

        if !connected {
            connected = true;
            if block_on(output.send(LiveTelemetryMessage::Connected)).is_err() {
                return Ok(());
            }
        }

        // A sample can arrive just after the wait deadline before the loop gets
        // a chance to flush. Close the expired batch before inserting it so a
        // normal ~16.67 ms 60 Hz interval does not get coalesced as a burst.
        if pending.is_due(captured_at) && !send_pending(output, &mut pending) {
            return Ok(());
        }

        let starts_batch = pending.is_empty();
        pending.push(CapturedTelemetrySample {
            captured_at,
            sample,
        });

        if starts_batch {
            match client.read_session_info() {
                Ok(session_info) => pending.batch.session_info = session_info,
                Err(error) => {
                    let _ = send_pending(output, &mut pending);
                    return Err(error);
                },
            }

            if full_frame_is_due(last_full_frame_at, captured_at) {
                match client.read_all() {
                    Ok(Some(frame)) => {
                        pending.batch.latest_frame = Some(frame);
                        last_full_frame_at = Some(captured_at);
                    },
                    Ok(None) => {},
                    Err(error) => {
                        let _ = send_pending(output, &mut pending);
                        return Err(error);
                    },
                }
            }
        }

        if pending.is_due(Instant::now()) && !send_pending(output, &mut pending) {
            return Ok(());
        }
    }
}

#[derive(Debug, Default)]
struct PendingBatch {
    batch: LiveTelemetryBatch,
    started_at: Option<Instant>,
}

impl PendingBatch {
    fn is_empty(&self) -> bool {
        self.batch.samples[0].is_none()
    }

    fn push(&mut self, sample: CapturedTelemetrySample) {
        if self.started_at.is_none() {
            self.started_at = Some(sample.captured_at);
        }
        if let Some(slot) = self.batch.samples.iter_mut().find(|slot| slot.is_none()) {
            *slot = Some(sample);
        }
    }

    fn is_due(&self, now: Instant) -> bool {
        !self.is_empty()
            && (self.batch.samples.iter().all(Option::is_some)
                || self.started_at.is_some_and(|started| {
                    now.saturating_duration_since(started) >= BATCH_MAX_LATENCY
                }))
    }

    fn wait_timeout(&self, now: Instant) -> Duration {
        self.started_at.map_or(EVENT_WAIT_TIMEOUT, |started| {
            BATCH_MAX_LATENCY
                .saturating_sub(now.saturating_duration_since(started))
                .min(EVENT_WAIT_TIMEOUT)
        })
    }

    fn take(&mut self) -> Option<LiveTelemetryBatch> {
        if self.is_empty() {
            return None;
        }

        self.started_at = None;
        Some(mem::take(&mut self.batch))
    }
}

fn send_pending(
    output: &mut iced::futures::channel::mpsc::Sender<LiveTelemetryMessage>,
    pending: &mut PendingBatch,
) -> bool {
    pending
        .take()
        .is_none_or(|batch| block_on(output.send(LiveTelemetryMessage::Batch(batch))).is_ok())
}

fn full_frame_is_due(last_full_frame_at: Option<Instant>, now: Instant) -> bool {
    last_full_frame_at.is_none_or(|last| now.saturating_duration_since(last) >= FULL_FRAME_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::{
        BATCH_MAX_LATENCY, CapturedTelemetrySample, EVENT_WAIT_TIMEOUT, FULL_FRAME_INTERVAL,
        LiveTelemetrySource, PendingBatch, full_frame_is_due,
    };
    use chiaro_irsdk::TelemetrySample;
    use std::time::{Duration, Instant};

    #[test]
    fn local_iracing_source_has_a_stable_transport_identity() {
        let info = LiveTelemetrySource::IracingSharedMemory.info();

        assert_eq!(info.id(), "iracing_shared_memory");
        assert_eq!(info.display_name(), "iRacing on this PC");
        assert_eq!(info.is_available(), cfg!(target_os = "windows"));
        assert_eq!(
            info.unavailable_reason().is_some(),
            !cfg!(target_os = "windows")
        );
    }

    #[test]
    fn coalesces_two_samples_inside_the_latency_window() {
        let started_at = Instant::now();
        let mut pending = PendingBatch::default();

        pending.push(CapturedTelemetrySample {
            captured_at: started_at,
            sample: TelemetrySample {
                packet_id: 10,
                ..TelemetrySample::default()
            },
        });
        assert!(!pending.is_due(started_at + Duration::from_millis(8)));
        assert_eq!(
            pending.wait_timeout(started_at + Duration::from_millis(8)),
            BATCH_MAX_LATENCY - Duration::from_millis(8)
        );

        pending.push(CapturedTelemetrySample {
            captured_at: started_at + Duration::from_millis(8),
            sample: TelemetrySample {
                packet_id: 11,
                ..TelemetrySample::default()
            },
        });
        assert!(pending.is_due(started_at + Duration::from_millis(8)));

        let batch = pending.take().expect("two captured samples");
        let packet_ids = batch
            .samples
            .into_iter()
            .flatten()
            .map(|captured| captured.sample.packet_id)
            .collect::<Vec<_>>();
        assert_eq!(packet_ids, vec![10, 11]);
        assert!(pending.is_empty());
        assert_eq!(pending.wait_timeout(started_at), EVENT_WAIT_TIMEOUT);
    }

    #[test]
    fn flushes_one_sample_before_the_next_60_hz_tick() {
        let started_at = Instant::now();
        let mut pending = PendingBatch::default();
        pending.push(CapturedTelemetrySample {
            captured_at: started_at,
            sample: TelemetrySample::default(),
        });

        assert!(!pending.is_due(started_at + BATCH_MAX_LATENCY - Duration::from_millis(1)));
        assert!(pending.is_due(started_at + BATCH_MAX_LATENCY));
        assert_eq!(
            pending.wait_timeout(started_at + BATCH_MAX_LATENCY),
            Duration::ZERO
        );

        let next_tick = started_at + Duration::from_micros(16_667);
        assert!(pending.is_due(next_tick));
        let first_batch = pending.take().expect("expired single-sample batch");
        assert_eq!(first_batch.samples.into_iter().flatten().count(), 1);

        pending.push(CapturedTelemetrySample {
            captured_at: next_tick,
            sample: TelemetrySample::default(),
        });
        assert!(!pending.is_due(next_tick));
    }

    #[test]
    fn full_frames_are_refreshed_at_the_low_frequency_deadline() {
        let started_at = Instant::now();
        assert!(full_frame_is_due(None, started_at));
        assert!(!full_frame_is_due(
            Some(started_at),
            started_at + FULL_FRAME_INTERVAL - Duration::from_millis(1)
        ));
        assert!(full_frame_is_due(
            Some(started_at),
            started_at + FULL_FRAME_INTERVAL
        ));
    }
}
