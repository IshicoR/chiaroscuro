use std::{collections::VecDeque, time::Instant};

use crate::{appearance::HISTORY_WINDOW, configuration::DEFAULT_SERVER_ADDR};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone, Copy)]
struct HistoryEntry {
    received_at: Instant,
    sample: TelemetrySample,
}

#[derive(Debug, Clone)]
pub struct Session {
    connection: ConnectionStatus,
    server_addr: String,
    packets_received: u64,
    latest: Option<TelemetrySample>,
    history: VecDeque<HistoryEntry>,
    last_error: Option<String>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            connection: ConnectionStatus::Disconnected,
            server_addr: DEFAULT_SERVER_ADDR.to_owned(),
            packets_received: 0,
            latest: None,
            history: VecDeque::new(),
            last_error: None,
        }
    }
}

impl Session {
    pub fn connection(&self) -> ConnectionStatus {
        self.connection
    }

    pub fn wants_connection(&self) -> bool {
        self.connection != ConnectionStatus::Disconnected
    }

    pub fn server_addr(&self) -> &str {
        &self.server_addr
    }

    pub fn set_server_addr(&mut self, server_addr: String) {
        self.server_addr = server_addr;
    }

    pub fn packets_received(&self) -> u64 {
        self.packets_received
    }

    pub fn latest(&self) -> Option<&TelemetrySample> {
        self.latest.as_ref()
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn set_connection_requested(&mut self, connected: bool) {
        self.clear_telemetry();
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
        self.history.clear();
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
        if !sample.is_finite() {
            return;
        }

        let now = Instant::now();
        self.latest = Some(sample);
        self.packets_received = self.packets_received.saturating_add(1);
        self.history.push_back(HistoryEntry {
            received_at: now,
            sample,
        });

        while self
            .history
            .front()
            .is_some_and(|entry| now.duration_since(entry.received_at) > HISTORY_WINDOW)
        {
            self.history.pop_front();
        }
    }

    pub fn points(&self, value: impl Fn(&TelemetrySample) -> f32) -> Vec<[f64; 2]> {
        let Some(first) = self.history.front() else {
            return Vec::new();
        };

        self.history
            .iter()
            .map(|entry| {
                let elapsed = entry
                    .received_at
                    .duration_since(first.received_at)
                    .as_secs_f64();
                [elapsed, f64::from(value(&entry.sample))]
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        time::{Duration, Instant},
    };

    use chiaroscuro_telemetry::TelemetrySample;

    use super::{ConnectionStatus, HistoryEntry, Session};

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
        assert_eq!(session.points(|sample| sample.throttle), vec![[0.0, 0.75]]);
    }

    #[test]
    fn chart_points_advance_from_the_left_edge() {
        let started_at = Instant::now();
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
                    received_at: started_at,
                    sample: first,
                },
                HistoryEntry {
                    received_at: started_at + Duration::from_secs(5),
                    sample: second,
                },
            ]),
            ..Session::default()
        };

        assert_eq!(
            session.points(|sample| sample.throttle),
            vec![[0.0, 0.25], [5.0, 0.75]]
        );
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
