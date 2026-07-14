use std::{
    io,
    net::SocketAddr,
    time::{Duration, Instant},
};

use iced::{
    Subscription,
    futures::{SinkExt, Stream},
    stream,
};
use smol::{Timer, future, net::UdpSocket};

const HELLO_PAYLOAD: &[u8] = b"HELLO";
const CHALLENGE_PREFIX: &str = "CHALLENGE ";
const REGISTER_PREFIX: &str = "REGISTER ";
const KEEPALIVE_PAYLOAD: &[u8] = b"KEEPALIVE";
const CLIENT_TICK: Duration = Duration::from_secs(1);
const RETRY_DELAY: Duration = Duration::from_secs(2);
const SERVER_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub enum Event {
    Waiting,
    Connected,
    Sample(TelemetrySample),
    Error(String),
}

pub fn subscription(server_addr: String) -> Subscription<Event> {
    Subscription::run_with(ServerAddress(server_addr), client_stream)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ServerAddress(String);

fn client_stream(server_addr: &ServerAddress) -> impl Stream<Item = Event> + 'static + use<> {
    let server_addr = server_addr.0.clone();

    stream::channel(64, async move |mut output| {
        loop {
            if output.send(Event::Waiting).await.is_err() {
                return;
            }

            let result = receive_from_server(&server_addr, &mut output).await;
            if let Err(error) = result
                && output.send(Event::Error(error.to_string())).await.is_err()
            {
                return;
            }

            Timer::after(RETRY_DELAY).await;
        }
    })
}

async fn receive_from_server(
    server_addr: &str,
    output: &mut iced::futures::channel::mpsc::Sender<Event>,
) -> io::Result<()> {
    let server_addr: SocketAddr = server_addr.parse().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid telemetry server address `{server_addr}`: {error}"),
        )
    })?;
    let bind_addr = if server_addr.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = UdpSocket::bind(bind_addr).await?;
    socket.connect(server_addr).await?;
    socket.send(HELLO_PAYLOAD).await?;

    let mut buffer = vec![0; 65_535];
    let mut registered = false;
    let mut connected = false;
    let mut last_response = Instant::now();

    loop {
        let event = future::race(
            async { ReceiveEvent::Datagram(socket.recv(&mut buffer).await) },
            async {
                Timer::after(CLIENT_TICK).await;
                ReceiveEvent::Tick
            },
        )
        .await;

        match event {
            ReceiveEvent::Datagram(Ok(len)) => {
                last_response = Instant::now();
                let payload = &buffer[..len];
                if let Some(token) = challenge_token(payload) {
                    let registration = format!("{REGISTER_PREFIX}{token}");
                    socket.send(registration.as_bytes()).await?;
                    registered = true;
                    continue;
                }

                let Ok(sample) = decode(payload) else {
                    continue;
                };
                if !connected {
                    connected = true;
                    if output.send(Event::Connected).await.is_err() {
                        return Ok(());
                    }
                }
                if output.send(Event::Sample(sample)).await.is_err() {
                    return Ok(());
                }
            },
            ReceiveEvent::Datagram(Err(error)) => return Err(error),
            ReceiveEvent::Tick => {
                if last_response.elapsed() >= SERVER_TIMEOUT {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        format!("telemetry server `{server_addr}` stopped responding"),
                    ));
                }

                let payload = if registered {
                    KEEPALIVE_PAYLOAD
                } else {
                    HELLO_PAYLOAD
                };
                socket.send(payload).await?;
            },
        }
    }
}

fn challenge_token(payload: &[u8]) -> Option<&str> {
    std::str::from_utf8(payload)
        .ok()?
        .strip_prefix(CHALLENGE_PREFIX)
        .map(str::trim)
}

enum ReceiveEvent {
    Datagram(io::Result<usize>),
    Tick,
}

#[cfg(test)]
mod tests {
    use super::challenge_token;

    #[test]
    fn parses_a_challenge_token() {
        assert_eq!(challenge_token(b"CHALLENGE 0011aabb"), Some("0011aabb"));
        assert_eq!(challenge_token(b"telemetry"), None);
    }
}
