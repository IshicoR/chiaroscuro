use std::{io, time::Duration};

use chiaro_irsdk::{Client, SessionInfo, TelemetrySnapshot};
use iced::{
    Subscription,
    futures::{SinkExt, Stream},
    stream,
};
use smol::{Timer, block_on, unblock};

const EVENT_WAIT_TIMEOUT: Duration = Duration::from_millis(250);
const RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub enum LiveTelemetryMessage {
    Waiting,
    Connected,
    Snapshot {
        snapshot: TelemetrySnapshot,
        session_info: Option<SessionInfo>,
    },
    Error(String),
}

pub fn subscription() -> Subscription<LiveTelemetryMessage> {
    Subscription::run(client_stream)
}

fn client_stream() -> impl Stream<Item = LiveTelemetryMessage> + 'static {
    stream::channel(64, async move |mut output| {
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

    loop {
        if output.is_closed() {
            return Ok(());
        }

        if let Some(snapshot) = client.wait_for_snapshot(EVENT_WAIT_TIMEOUT)? {
            let session_info = client.read_session_info()?;
            if !connected {
                connected = true;
                if block_on(output.send(LiveTelemetryMessage::Connected)).is_err() {
                    return Ok(());
                }
            }

            if block_on(output.send(LiveTelemetryMessage::Snapshot {
                snapshot,
                session_info,
            }))
            .is_err()
            {
                return Ok(());
            }
        }
    }
}
