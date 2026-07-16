use std::{io, time::Duration};

use chiaroscuro_irsdk::{Client, SessionInfo, TelemetrySnapshot};
use iced::{
    Subscription,
    futures::{SinkExt, Stream},
    stream,
};
use smol::{Timer, block_on, unblock};

const EVENT_WAIT_TIMEOUT: Duration = Duration::from_millis(250);
const RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub enum Event {
    Waiting,
    Connected,
    Snapshot {
        snapshot: TelemetrySnapshot,
        session_info: Option<SessionInfo>,
    },
    Error(String),
}

pub fn subscription() -> Subscription<Event> {
    Subscription::run(client_stream)
}

fn client_stream() -> impl Stream<Item = Event> + 'static {
    stream::channel(64, async move |mut output| {
        loop {
            if output.send(Event::Waiting).await.is_err() {
                return;
            }

            let (next_output, result) = unblock(move || stream_samples(output)).await;
            output = next_output;
            if output.is_closed() {
                return;
            }

            if let Err(error) = result
                && output.send(Event::Error(error.to_string())).await.is_err()
            {
                return;
            }

            Timer::after(RETRY_DELAY).await;
        }
    })
}

fn stream_samples(
    mut output: iced::futures::channel::mpsc::Sender<Event>,
) -> (iced::futures::channel::mpsc::Sender<Event>, io::Result<()>) {
    let result = stream_samples_inner(&mut output);
    (output, result)
}

fn stream_samples_inner(
    output: &mut iced::futures::channel::mpsc::Sender<Event>,
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
                if block_on(output.send(Event::Connected)).is_err() {
                    return Ok(());
                }
            }

            if block_on(output.send(Event::Snapshot {
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
