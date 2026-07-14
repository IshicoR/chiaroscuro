mod context;
mod error;
mod setting;
mod shared_memory;
mod telemetry_source;

use async_channel::Sender;
use chiaroscuro_telemetry::encode;
use error::UdpError;
use smol::{Timer, net::UdpSocket};
use std::{
    collections::{HashMap, hash_map::Entry},
    hash::{BuildHasher, Hash, Hasher, RandomState},
    io,
    net::SocketAddr,
    time::{Duration, Instant},
};

use crate::setting::Setting;
use crate::telemetry_source::TelemetrySource;

#[global_allocator]
static ALLOC: rpmalloc::RpMalloc = rpmalloc::RpMalloc;

const CLIENT_QUEUE_CAPACITY: usize = 1024;
const RECEIVE_ERROR_QUEUE_CAPACITY: usize = 1;
const MAX_CLIENTS: usize = 1024;
const MAX_DATAGRAM_SIZE: usize = 65_535;
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);
const CHALLENGE_TOKEN_TTL: Duration = Duration::from_secs(10);
const SEND_INTERVAL: Duration = Duration::from_millis(16);
const SOURCE_RETRY_INTERVAL: Duration = Duration::from_secs(2);
const HELLO_PAYLOAD: &[u8] = b"HELLO";
const CHALLENGE_PREFIX: &str = "CHALLENGE ";
const REGISTER_PREFIX: &str = "REGISTER ";

fn main() -> anyhow::Result<()> {
    set_low_priority()?;
    let setting = Setting::new()?;
    let bind_addr = parse_bind_addr(&setting)?;

    smol::block_on(async {
        let mut server = UdpServer::new(
            bind_addr,
            setting.require_registration,
            setting.mock_telemetry,
        )
        .await?;
        server.serve().await?;

        Ok(())
    })
}

fn parse_bind_addr(setting: &Setting) -> Result<SocketAddr, UdpError> {
    setting
        .bind_addr
        .parse()
        .map_err(|source| UdpError::InvalidBindAddr {
            value: setting.bind_addr.clone(),
            source,
        })
}

#[cfg(target_os = "windows")]
fn set_low_priority() -> Result<(), UdpError> {
    unsafe {
        let result = winapi::um::processthreadsapi::SetPriorityClass(
            winapi::um::processthreadsapi::GetCurrentProcess(),
            winapi::um::winbase::BELOW_NORMAL_PRIORITY_CLASS,
        );

        if result == 0 {
            return Err(UdpError::SetPriorityClass(io::Error::last_os_error()));
        }
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_low_priority() -> Result<(), UdpError> {
    Ok(())
}

#[derive(Debug)]
struct UdpServer {
    socket: UdpSocket,
    clients: HashMap<SocketAddr, ClientState>,
    challenge_tokens: ChallengeTokens,
    require_registration: bool,
    telemetry_source: Option<TelemetrySource>,
    mock_telemetry: bool,
    source_retry_at: Instant,
    last_packet_id: Option<i32>,
}

#[derive(Debug)]
struct ClientState {
    last_seen: Instant,
}

#[derive(Clone, Debug)]
struct ChallengeTokens {
    hash_builder: RandomState,
    epoch: Instant,
}

#[derive(Debug)]
enum ClientEvent {
    Observed(SocketAddr),
    Register { addr: SocketAddr, token: u64 },
}

impl UdpServer {
    async fn new(
        addr: SocketAddr,
        require_registration: bool,
        mock_telemetry: bool,
    ) -> io::Result<Self> {
        let socket = UdpSocket::bind(addr).await?;

        Ok(Self {
            socket,
            clients: HashMap::new(),
            challenge_tokens: ChallengeTokens::new(),
            require_registration,
            telemetry_source: None,
            mock_telemetry,
            source_retry_at: Instant::now(),
            last_packet_id: None,
        })
    }

    async fn serve(&mut self) -> io::Result<()> {
        let (client_event_tx, client_event_rx) = async_channel::bounded(CLIENT_QUEUE_CAPACITY);
        let (receive_error_tx, receive_error_rx) =
            async_channel::bounded(RECEIVE_ERROR_QUEUE_CAPACITY);
        spawn_receiver(
            self.socket.clone(),
            client_event_tx,
            receive_error_tx,
            self.challenge_tokens.clone(),
        );

        loop {
            if let Ok(err) = receive_error_rx.try_recv() {
                return Err(err);
            }

            while let Ok(event) = client_event_rx.try_recv() {
                self.handle_client_event(event);
            }

            self.remove_inactive_clients();
            self.refresh_telemetry_source();
            self.send_telemetry().await;

            Timer::after(SEND_INTERVAL).await;
        }
    }

    fn handle_client_event(&mut self, event: ClientEvent) {
        match event {
            ClientEvent::Observed(addr) if self.require_registration => self.refresh_client(addr),
            ClientEvent::Observed(addr) => self.upsert_client(addr),
            ClientEvent::Register { addr, token } => self.register_client(addr, token),
        }
    }

    fn refresh_client(&mut self, addr: SocketAddr) {
        if let Some(client) = self.clients.get_mut(&addr) {
            client.last_seen = Instant::now();
        }
    }

    fn register_client(&mut self, addr: SocketAddr, token: u64) {
        if !self.challenge_tokens.verify(addr, token, Instant::now()) {
            eprintln!("reject client {addr}: invalid challenge token");
            return;
        }

        self.upsert_client(addr);
    }

    fn upsert_client(&mut self, addr: SocketAddr) {
        let now = Instant::now();
        let has_capacity = self.clients.len() < MAX_CLIENTS;

        match self.clients.entry(addr) {
            Entry::Occupied(mut client) => {
                client.get_mut().last_seen = now;
            },
            Entry::Vacant(client) if has_capacity => {
                client.insert(ClientState { last_seen: now });
                println!("add new client: {addr}");
            },
            Entry::Vacant(_) => {
                eprintln!("reject new client {addr}: client limit reached");
            },
        }
    }

    fn remove_inactive_clients(&mut self) {
        let now = Instant::now();
        self.clients.retain(|addr, client| {
            let active = now.duration_since(client.last_seen) <= CLIENT_TIMEOUT;
            if !active {
                println!("remove inactive client: {addr}");
            }
            active
        });
    }

    fn refresh_telemetry_source(&mut self) {
        if self.telemetry_source.is_some() || Instant::now() < self.source_retry_at {
            return;
        }

        match TelemetrySource::open(self.mock_telemetry) {
            Ok(source) => {
                println!("connected to {}", source.name());
                self.telemetry_source = Some(source);
                self.last_packet_id = None;
            },
            Err(err) => {
                self.source_retry_at = Instant::now() + SOURCE_RETRY_INTERVAL;
                if err.kind() != io::ErrorKind::Unsupported {
                    eprintln!("waiting for Assetto Corsa shared memory: {err}");
                }
            },
        }
    }

    async fn send_telemetry(&mut self) {
        let Some(source) = &mut self.telemetry_source else {
            return;
        };

        let sample = match source.read() {
            Ok(sample) => sample,
            Err(err) => {
                eprintln!("lost Assetto Corsa shared memory: {err}");
                self.telemetry_source = None;
                self.source_retry_at = Instant::now() + SOURCE_RETRY_INTERVAL;
                self.last_packet_id = None;
                return;
            },
        };

        if self.last_packet_id == Some(sample.packet_id) || !sample.is_finite() {
            return;
        }
        self.last_packet_id = Some(sample.packet_id);

        let payload = encode(&sample);
        self.send_to_clients(&payload).await;
    }

    async fn send_to_clients(&mut self, payload: &[u8]) {
        let mut failed_clients = Vec::new();

        for &addr in self.clients.keys() {
            match self.socket.send_to(payload, addr).await {
                Ok(_) => {},
                Err(err) => {
                    eprintln!("failed to send udp packet to {addr}: {err}");
                    failed_clients.push(addr);
                },
            }
        }

        for addr in failed_clients {
            self.clients.remove(&addr);
            println!("remove failed client: {addr}");
        }
    }
}

impl ChallengeTokens {
    fn new() -> Self {
        Self {
            hash_builder: RandomState::new(),
            epoch: Instant::now(),
        }
    }

    fn current_token(&self, addr: SocketAddr, now: Instant) -> u64 {
        self.token_for_slot(addr, self.slot(now))
    }

    fn verify(&self, addr: SocketAddr, token: u64, now: Instant) -> bool {
        let slot = self.slot(now);

        token == self.token_for_slot(addr, slot)
            || slot
                .checked_sub(1)
                .is_some_and(|previous_slot| token == self.token_for_slot(addr, previous_slot))
    }

    fn slot(&self, now: Instant) -> u64 {
        let elapsed = now.duration_since(self.epoch);
        elapsed.as_secs() / CHALLENGE_TOKEN_TTL.as_secs()
    }

    fn token_for_slot(&self, addr: SocketAddr, slot: u64) -> u64 {
        let mut hasher = self.hash_builder.build_hasher();
        addr.hash(&mut hasher);
        slot.hash(&mut hasher);
        hasher.finish()
    }
}

fn spawn_receiver(
    socket_reader: UdpSocket,
    client_event_tx: Sender<ClientEvent>,
    receive_error_tx: Sender<io::Error>,
    challenge_tokens: ChallengeTokens,
) {
    smol::spawn(async move {
        let mut buf = vec![0; MAX_DATAGRAM_SIZE];

        loop {
            match socket_reader.recv_from(&mut buf).await {
                Ok((len, addr)) => {
                    handle_datagram(
                        &socket_reader,
                        &client_event_tx,
                        &challenge_tokens,
                        addr,
                        &buf[..len],
                    )
                    .await;
                },
                Err(err) => {
                    let _ = receive_error_tx.send(err).await;
                    break;
                },
            }
        }
    })
    .detach();
}

async fn handle_datagram(
    socket: &UdpSocket,
    client_event_tx: &Sender<ClientEvent>,
    challenge_tokens: &ChallengeTokens,
    addr: SocketAddr,
    payload: &[u8],
) {
    if payload == HELLO_PAYLOAD {
        send_challenge(socket, challenge_tokens, addr).await;
        return;
    }

    if let Some(token) = parse_register_token(payload) {
        let _ = client_event_tx.try_send(ClientEvent::Register { addr, token });
        return;
    }

    let _ = client_event_tx.try_send(ClientEvent::Observed(addr));
}

async fn send_challenge(socket: &UdpSocket, challenge_tokens: &ChallengeTokens, addr: SocketAddr) {
    let token = challenge_tokens.current_token(addr, Instant::now());
    let challenge = format!("{CHALLENGE_PREFIX}{token:016x}");

    if let Err(err) = socket.send_to(challenge.as_bytes(), addr).await {
        eprintln!("failed to send udp challenge to {addr}: {err}");
    }
}

fn parse_register_token(payload: &[u8]) -> Option<u64> {
    let payload = std::str::from_utf8(payload).ok()?;
    let token = payload.strip_prefix(REGISTER_PREFIX)?;

    u64::from_str_radix(token.trim(), 16).ok()
}

#[cfg(test)]
mod tests {
    use std::{io, net::SocketAddr, time::Duration};

    use chiaroscuro_telemetry::decode;
    use smol::{Timer, future, net::UdpSocket};

    use super::{CHALLENGE_PREFIX, HELLO_PAYLOAD, REGISTER_PREFIX, UdpServer};

    #[test]
    fn mock_stream_uses_registration_and_udp_wire_protocol() {
        smol::block_on(async {
            let bind_addr: SocketAddr = "127.0.0.1:0".parse().expect("valid bind address");
            let mut server = UdpServer::new(bind_addr, true, true)
                .await
                .expect("mock server should bind");
            let server_addr = server
                .socket
                .local_addr()
                .expect("bound server should have an address");
            let server_task = smol::spawn(async move { server.serve().await });

            let client = UdpSocket::bind("127.0.0.1:0")
                .await
                .expect("test client should bind");
            client
                .connect(server_addr)
                .await
                .expect("test client should connect");
            client
                .send(HELLO_PAYLOAD)
                .await
                .expect("hello should be sent");

            let mut buffer = vec![0; 65_535];
            let challenge_len = receive_with_timeout(&client, &mut buffer)
                .await
                .expect("server should return a challenge");
            let challenge =
                std::str::from_utf8(&buffer[..challenge_len]).expect("challenge should be UTF-8");
            let token = challenge
                .strip_prefix(CHALLENGE_PREFIX)
                .expect("challenge prefix should be present");
            client
                .send(format!("{REGISTER_PREFIX}{}", token.trim()).as_bytes())
                .await
                .expect("registration should be sent");

            let sample = loop {
                let len = receive_with_timeout(&client, &mut buffer)
                    .await
                    .expect("registered client should receive telemetry");
                if let Ok(sample) = decode(&buffer[..len]) {
                    break sample;
                }
            };

            assert!(sample.is_finite());
            assert!(sample.speed_kmh > 0.0);
            assert!((0.0..=1.0).contains(&sample.throttle));
            assert!(
                sample
                    .tyre_core_temperature_c
                    .iter()
                    .all(|value| *value > 0.0)
            );

            let _ = server_task.cancel().await;
        });
    }

    async fn receive_with_timeout(socket: &UdpSocket, buffer: &mut [u8]) -> io::Result<usize> {
        future::race(socket.recv(buffer), async {
            Timer::after(Duration::from_secs(2)).await;
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "timed out waiting for UDP packet",
            ))
        })
        .await
    }
}
