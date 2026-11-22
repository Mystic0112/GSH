use crate::identity::Identity;
use crate::protocol::{Announcement, DeviceInfo, API, MULTICAST, PORT, VERSION};
use get_if_addrs::{get_if_addrs, IfAddr};
use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use thiserror::Error;

const PEER_TTL: Duration = Duration::from_secs(30);
const DISCOVERY_TICK: Duration = Duration::from_secs(5);
const REGISTER_TIMEOUT: Duration = Duration::from_secs(2);
const REGISTER_QUEUE_SIZE: usize = 16;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub info: DeviceInfo,
    pub ip: Ipv4Addr,
    pub seen_at: u64,
}

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("network I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("TLS setup failed: {0}")]
    Tls(String),
    #[error("LocalSend peer certificate fingerprint did not match discovery")]
    Fingerprint,
    #[error("LocalSend peer returned HTTP {0}")]
    Http(u16),
    #[error("invalid LocalSend response: {0}")]
    Json(#[from] serde_json::Error),
    #[error("no interface could join the LocalSend multicast group")]
    NoInterfaces,
    #[error("file exceeds configured LocalSend size limit")]
    TooLarge,
    #[error("LocalSend request exceeds the HTTP size limit")]
    RequestTooLarge,
    #[error("LocalSend response exceeds the HTTP size limit")]
    ResponseTooLarge,
    #[error("secure random source failed: {0}")]
    Random(String),
    #[error("LocalSend transfer cancelled")]
    Cancelled,
}

pub struct Discovery {
    peers: Arc<RwLock<HashMap<String, Peer>>>,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Discovery {
    pub fn start(identity: Identity) -> Result<Self, NetworkError> {
        Self::start_with_socket(identity, open_multicast()?)
    }

    fn start_with_socket(identity: Identity, socket: UdpSocket) -> Result<Self, NetworkError> {
        let peers = Arc::new(RwLock::new(HashMap::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let registers = start_register_worker(identity.info.clone(), Arc::clone(&stop))?;
        let peer_map = Arc::clone(&peers);
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("gsh-localsend-discovery".into())
            .spawn(move || {
                let mut last_announce = Instant::now() - DISCOVERY_TICK;
                let mut buffer = [0u8; 65536];
                while !worker_stop.load(Ordering::Relaxed) {
                    if last_announce.elapsed() >= DISCOVERY_TICK {
                        let announce = Announcement {
                            info: identity.info.clone(),
                            announce: true,
                        };
                        send_multicast(&socket, &announce);
                        last_announce = Instant::now();
                    }
                    match socket.recv_from(&mut buffer) {
                        Ok((len, addr)) => {
                            if let Ok(message) =
                                serde_json::from_slice::<Announcement>(&buffer[..len])
                            {
                                if message.info.fingerprint == identity.info.fingerprint {
                                    continue;
                                }
                                let std::net::IpAddr::V4(peer_ip) = addr.ip() else {
                                    continue;
                                };
                                update_peer(&peer_map, message.info.clone(), peer_ip);
                                if message.announce {
                                    if let Some(port) = message.info.port {
                                        let address = SocketAddr::new(peer_ip.into(), port);
                                        // Keep multicast responsive even when the register queue is full.
                                        let _ =
                                            registers.try_send((address, message.info.fingerprint));
                                    }
                                    let response = Announcement {
                                        info: identity.info.clone(),
                                        announce: false,
                                    };
                                    send_multicast(&socket, &response);
                                }
                            }
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                            ) => {}
                        Err(_) => thread::sleep(Duration::from_millis(100)),
                    }
                    if let Ok(mut peers) = peer_map.write() {
                        peers.retain(|_, peer| elapsed_since(peer.seen_at) < PEER_TTL.as_secs());
                    }
                }
            })
            .map_err(NetworkError::Io)?;
        Ok(Self {
            peers,
            stop,
            worker: Some(worker),
        })
    }
    pub fn peers(&self) -> Vec<Peer> {
        self.peers
            .read()
            .map(|peers| peers.values().cloned().collect())
            .unwrap_or_default()
    }
}

fn start_register_worker(
    local: DeviceInfo,
    stop: Arc<AtomicBool>,
) -> Result<mpsc::SyncSender<(SocketAddr, String)>, NetworkError> {
    let body = serde_json::to_vec(&local)?;
    let path = format!("{API}/register");
    let (sender, receiver) = mpsc::sync_channel::<(SocketAddr, String)>(REGISTER_QUEUE_SIZE);
    // Drop joins multicast only. An active register can outlive it by REGISTER_TIMEOUT.
    drop(
        thread::Builder::new()
            .name("gsh-localsend-register".into())
            .spawn(move || {
                while let Ok((address, fingerprint)) = receiver.recv() {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    let _ = client::https_request_with_timeout(
                        address,
                        &fingerprint,
                        "POST",
                        &path,
                        &body,
                        REGISTER_TIMEOUT,
                    );
                }
            })?,
    );
    Ok(sender)
}

fn send_multicast(socket: &UdpSocket, announcement: &Announcement) {
    let Ok(bytes) = serde_json::to_vec(announcement) else {
        return;
    };
    if let Ok(interfaces) = get_if_addrs() {
        for interface in interfaces {
            if let IfAddr::V4(address) = interface.addr {
                if !address.ip.is_loopback() && !address.ip.is_unspecified() {
                    send_on_interface(socket, &bytes, address.ip);
                }
            }
        }
    }
}

fn send_on_interface(socket: &UdpSocket, bytes: &[u8], interface: Ipv4Addr) {
    let Ok(cloned) = socket.try_clone() else {
        return;
    };
    let selected = Socket::from(cloned);
    if selected.set_multicast_if_v4(&interface).is_ok() {
        let target = SocketAddrV4::new(MULTICAST, PORT);
        let _ = selected.send_to(bytes, &target.into());
    }
}
impl Drop for Discovery {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn open_multicast() -> Result<UdpSocket, NetworkError> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(all(
        unix,
        not(any(target_os = "solaris", target_os = "illumos", target_os = "cygwin"))
    ))]
    socket.set_reuse_port(true)?;
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, PORT).into())?;
    socket.set_multicast_loop_v4(true)?;
    let interfaces = get_if_addrs().map_err(NetworkError::Io)?;
    let mut joined = Vec::new();
    for interface in interfaces {
        if let IfAddr::V4(addr) = interface.addr {
            if addr.ip.is_loopback() || addr.ip.is_unspecified() {
                continue;
            }
            if socket.join_multicast_v4(&MULTICAST, &addr.ip).is_ok() {
                joined.push(addr.ip);
            }
        }
    }
    if joined.is_empty() {
        return Err(NetworkError::NoInterfaces);
    }
    let socket: UdpSocket = socket.into();
    socket.set_read_timeout(Some(Duration::from_millis(200)))?;
    Ok(socket)
}

fn update_peer(peers: &Arc<RwLock<HashMap<String, Peer>>>, info: DeviceInfo, ip: Ipv4Addr) {
    if let Ok(mut peers) = peers.write() {
        peers.insert(
            info.fingerprint.clone(),
            Peer {
                info,
                ip,
                seen_at: now_seconds(),
            },
        );
    }
}
fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}
fn elapsed_since(timestamp: u64) -> u64 {
    now_seconds().saturating_sub(timestamp)
}

#[path = "client.rs"]
pub(crate) mod client;
pub use client::{send_files, send_open_files, send_open_files_until};

pub fn local_info(identity: &Identity) -> DeviceInfo {
    let mut info = identity.info.clone();
    info.version = VERSION.into();
    info
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::net::{SocketAddrV4, TcpListener};

    fn test_identity() -> Identity {
        let generated = rcgen::generate_simple_self_signed(vec!["mcast-test".into()]).unwrap();
        let cert = generated.cert.der().clone();
        let fingerprint = Sha256::digest(cert.as_ref())
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":");
        Identity {
            info: DeviceInfo {
                alias: "mcast-test".into(),
                version: VERSION.into(),
                device_model: None,
                device_type: Some("desktop".into()),
                fingerprint,
                port: Some(PORT),
                protocol: Some("https".into()),
                download: false,
            },
            cert,
            key: rustls::pki_types::PrivateKeyDer::Pkcs8(
                rustls::pki_types::PrivatePkcs8KeyDer::from(generated.key_pair.serialize_der()),
            ),
            directory: std::env::temp_dir(),
        }
    }

    #[test]
    fn multicast_discovery_finds_an_announcement() {
        let mut identity = test_identity();
        identity.info.port = Some(53317);
        let discovery = Discovery::start(identity).unwrap();
        let sender = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap();
        sender.set_multicast_loop_v4(true).unwrap();
        let fake = DeviceInfo {
            alias: "multicast-test-peer".into(),
            version: VERSION.into(),
            device_model: None,
            device_type: Some("desktop".into()),
            fingerprint: "not-self".into(),
            port: Some(53317),
            protocol: Some("https".into()),
            download: false,
        };
        let packet = serde_json::to_vec(&Announcement {
            info: fake,
            announce: false,
        })
        .unwrap();
        sender
            .send_to(&packet, SocketAddrV4::new(MULTICAST, PORT))
            .unwrap();
        for _ in 0..30 {
            if discovery
                .peers()
                .iter()
                .any(|peer| peer.info.alias == "multicast-test-peer")
            {
                return;
            }
            thread::sleep(Duration::from_millis(100));
        }
        panic!("multicast announcement was not discovered");
    }

    #[test]
    fn discovery_drop_does_not_wait_for_stalled_registration() {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let address = socket.local_addr().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let discovery = Discovery::start_with_socket(test_identity(), socket).unwrap();

        let mut peer = test_identity().info;
        peer.port = Some(listener.local_addr().unwrap().port());
        let announcement = serde_json::to_vec(&Announcement {
            info: peer,
            announce: true,
        })
        .unwrap();
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        sender.send_to(&announcement, address).unwrap();

        let deadline = Instant::now() + REGISTER_TIMEOUT;
        let (_stalled_connection, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < deadline, "register did not connect");
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => panic!("register accept failed: {error}"),
            }
        };
        let start = Instant::now();
        drop(discovery);
        assert!(start.elapsed() < REGISTER_TIMEOUT / 2);
    }
}
