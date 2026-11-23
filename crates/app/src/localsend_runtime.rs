//! Runtime LocalSend do app: servidor TLS, anúncio/descoberta e ponte de aceite
//! para a thread principal do iced.

use gsh_localsend::{
    DeviceInfo, Discovery, FileMetadata, Identity, Limits, LocalSendServer, Peer, ReceiveDecision,
    Receiver, ServerError, RECEIVE_DECISION_TIMEOUT,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{self, Receiver as EventReceiver, Sender, SyncSender};
use std::sync::Arc;
use std::time::Instant;
use thiserror::Error;

pub struct IncomingRequest {
    pub sender: DeviceInfo,
    pub address: SocketAddr,
    pub files: Vec<FileMetadata>,
    pub reply: SyncSender<ReceiveDecision>,
    pub deadline: std::time::Instant,
}

pub enum ReceiveEvent {
    Request(IncomingRequest),
    Progress {
        address: SocketAddr,
        name: String,
        bytes: u64,
        total: u64,
    },
    Completed {
        address: SocketAddr,
        dir: PathBuf,
    },
}

#[derive(Clone)]
pub struct ReceiveBridge {
    events: Sender<ReceiveEvent>,
    timeout: std::time::Duration,
}

impl ReceiveBridge {
    pub fn channel() -> (Self, EventReceiver<ReceiveEvent>) {
        Self::channel_with_timeout(RECEIVE_DECISION_TIMEOUT)
    }

    fn channel_with_timeout(timeout: std::time::Duration) -> (Self, EventReceiver<ReceiveEvent>) {
        let (events, receiver) = mpsc::channel();
        (Self { events, timeout }, receiver)
    }
}

impl Receiver for ReceiveBridge {
    fn accept(
        &self,
        sender: &DeviceInfo,
        sender_address: SocketAddr,
        files: &[FileMetadata],
    ) -> ReceiveDecision {
        let (reply, answer) = mpsc::sync_channel(1);
        let request = IncomingRequest {
            sender: sender.clone(),
            address: sender_address,
            files: files.to_vec(),
            reply,
            deadline: std::time::Instant::now() + RECEIVE_DECISION_TIMEOUT,
        };
        if self.events.send(ReceiveEvent::Request(request)).is_err() {
            return ReceiveDecision::Reject;
        }
        await_decision(&answer, self.timeout)
    }

    fn progress(&self, sender_address: SocketAddr, name: &str, bytes: u64, total: u64) {
        let _ = self.events.send(ReceiveEvent::Progress {
            address: sender_address,
            name: name.to_owned(),
            bytes,
            total,
        });
    }

    fn completed(&self, sender_address: SocketAddr, dir: &std::path::Path) {
        let _ = self.events.send(ReceiveEvent::Completed {
            address: sender_address,
            dir: dir.to_path_buf(),
        });
    }
}

fn await_decision(
    answer: &EventReceiver<ReceiveDecision>,
    timeout: std::time::Duration,
) -> ReceiveDecision {
    answer
        .recv_timeout(timeout)
        .unwrap_or(ReceiveDecision::Reject)
}

#[derive(Debug, Error)]
pub enum StartError {
    #[error("identity setup: {0}")]
    Identity(#[from] gsh_localsend::IdentityError),
    #[error("server startup: {0}")]
    Server(#[from] ServerError),
    #[error("discovery startup: {0}")]
    Discovery(#[from] gsh_localsend::NetworkError),
}

pub struct LocalSendHost {
    server: LocalSendServer,
    discovery: Option<Discovery>,
    identity: Identity,
}

#[derive(Clone)]
pub struct LocalSendSender {
    identity: Identity,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PeerLookupError {
    #[error("LocalSend peer not found: {0}")]
    NotFound(String),
    #[error("LocalSend peer target is ambiguous: {0}")]
    Ambiguous(String),
}

impl LocalSendHost {
    pub fn start(
        bind: SocketAddr,
        alias: &str,
        limits: Limits,
        receive_bridge: ReceiveBridge,
    ) -> Result<Self, StartError> {
        let identity = Identity::load_or_create(bind.port())?;
        Self::start_with_identity(bind, alias, limits, receive_bridge, identity, true)
    }

    fn start_with_identity(
        bind: SocketAddr,
        alias: &str,
        limits: Limits,
        receive_bridge: ReceiveBridge,
        mut identity: Identity,
        with_discovery: bool,
    ) -> Result<Self, StartError> {
        if !alias.trim().is_empty() {
            identity.info.alias = alias.trim().to_owned();
        }
        let server =
            LocalSendServer::bind(identity.clone(), bind, Arc::new(receive_bridge), limits)?;
        identity.info.port = Some(server.address().port());
        let discovery = with_discovery
            .then(|| Discovery::start(identity.clone()))
            .transpose()?;
        Ok(Self {
            server,
            discovery,
            identity,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.server.address()
    }

    pub fn identity(&self) -> Identity {
        self.identity.clone()
    }

    pub fn info(&self) -> &DeviceInfo {
        &self.identity.info
    }

    pub fn peers(&self) -> Vec<Peer> {
        self.discovery
            .as_ref()
            .map(Discovery::peers)
            .unwrap_or_default()
    }

    pub fn rescan(&self) {
        if let Some(discovery) = &self.discovery {
            discovery.rescan();
        }
    }

    pub fn sender(&self) -> LocalSendSender {
        LocalSendSender {
            identity: self.identity(),
        }
    }

    #[cfg(test)]
    pub(super) fn start_for_test(
        bind: SocketAddr,
        alias: &str,
        limits: Limits,
        receive_bridge: ReceiveBridge,
    ) -> Result<Self, StartError> {
        let identity = Identity::generate(bind.port()).expect("test identity");
        Self::start_with_identity(bind, alias, limits, receive_bridge, identity, false)
    }
}

impl LocalSendSender {
    pub fn send_open_files_until(
        &self,
        peer: &Peer,
        files: Vec<(PathBuf, std::fs::File)>,
        max_file_size: u64,
        deadline: Instant,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(), gsh_localsend::NetworkError> {
        gsh_localsend::send_open_files_until(
            peer,
            &self.identity,
            files,
            max_file_size,
            deadline,
            |_, _, _| {},
            move || cancelled.load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    pub fn send_files(
        &self,
        peer: Peer,
        files: Vec<PathBuf>,
        max_file_size: u64,
        mut progress: impl FnMut(&std::path::Path, u64, u64) + Send + 'static,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(), gsh_localsend::NetworkError> {
        gsh_localsend::send_files(
            &peer,
            &self.identity,
            &files,
            max_file_size,
            |path, sent, total| progress(path, sent, total),
            move || cancelled.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

pub fn find_peer(peers: &[Peer], target: &str) -> Result<Peer, PeerLookupError> {
    let target = target.trim();
    let ip = target.parse::<std::net::Ipv4Addr>().ok();
    let matches = peers
        .iter()
        .filter(|peer| peer.info.alias.eq_ignore_ascii_case(target) || Some(peer.ip) == ip)
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [peer] => Ok(peer.clone()),
        [] => Err(PeerLookupError::NotFound(target.to_owned())),
        _ => Err(PeerLookupError::Ambiguous(target.to_owned())),
    }
}

pub fn limits(max_file_mb: u32, max_files: usize) -> Limits {
    let max_file_size = u64::from(max_file_mb.clamp(1, 2048)) * 1024 * 1024;
    let max_files = max_files.clamp(1, 100);
    Limits {
        max_file_size,
        max_files,
        max_total_size: max_file_size.saturating_mul(max_files as u64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_localsend::{send_files, NetworkError};
    use std::fs;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::time::Duration;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "gsh-app-localsend-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |duration| duration.as_nanos())
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn shell_accept_message_saves_only_under_receive_dir() {
        let root = temp_dir("accept");
        let source = root.join("source.txt");
        let destination = root.join("received");
        fs::create_dir_all(&destination).unwrap();
        fs::write(&source, b"accepted by the app").unwrap();
        let (bridge, events) = ReceiveBridge::channel();
        let host = LocalSendHost::start_for_test(
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0).into(),
            "GSH Test",
            limits(2, 4),
            bridge,
        )
        .unwrap();
        let mut sender_identity = Identity::generate(0).unwrap();
        sender_identity.info.alias = "Sender Test".to_owned();
        let sender = std::thread::spawn({
            let host_info = host.info().clone();
            let source = source.clone();
            move || {
                let peer = Peer {
                    info: host_info,
                    ip: Ipv4Addr::LOCALHOST,
                    seen_at: 0,
                };
                send_files(
                    &peer,
                    &sender_identity,
                    &[source],
                    1024 * 1024,
                    |_, _, _| {},
                    || false,
                )
            }
        });
        let ReceiveEvent::Request(request) = events.recv_timeout(Duration::from_secs(3)).unwrap()
        else {
            panic!("expected receive request event");
        };
        assert_eq!(request.sender.alias, "Sender Test");
        assert_eq!(
            request.address.ip(),
            std::net::IpAddr::V4(Ipv4Addr::LOCALHOST)
        );
        assert_eq!(request.files[0].file_name, "source.txt");
        request
            .reply
            .send(ReceiveDecision::Accept {
                dir: destination.clone(),
            })
            .unwrap();
        sender.join().unwrap().unwrap();
        assert_eq!(
            fs::read(destination.join("source.txt")).unwrap(),
            b"accepted by the app"
        );
        assert_eq!(fs::read_dir(&root).unwrap().count(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shell_reject_message_creates_no_file() {
        let root = temp_dir("reject");
        let source = root.join("source.txt");
        let destination = root.join("received");
        fs::create_dir_all(&destination).unwrap();
        fs::write(&source, b"rejected by the app").unwrap();
        let (bridge, events) = ReceiveBridge::channel();
        let host = LocalSendHost::start_for_test(
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0).into(),
            "GSH Test",
            limits(2, 4),
            bridge,
        )
        .unwrap();
        let sender = std::thread::spawn({
            let host_info = host.info().clone();
            let source = source.clone();
            move || {
                let peer = Peer {
                    info: host_info,
                    ip: Ipv4Addr::LOCALHOST,
                    seen_at: 0,
                };
                send_files(
                    &peer,
                    &Identity::generate(0).unwrap(),
                    &[source],
                    1024 * 1024,
                    |_, _, _| {},
                    || false,
                )
            }
        });
        let ReceiveEvent::Request(request) = events.recv_timeout(Duration::from_secs(3)).unwrap()
        else {
            panic!("expected receive request event");
        };
        request.reply.send(ReceiveDecision::Reject).unwrap();
        assert!(matches!(
            sender.join().unwrap(),
            Err(NetworkError::Http(403))
        ));
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_shell_response_times_out_as_rejection() {
        let root = temp_dir("timeout");
        let source = root.join("source.txt");
        let destination = root.join("received");
        fs::create_dir_all(&destination).unwrap();
        fs::write(&source, b"timed out").unwrap();
        let (bridge, events) = ReceiveBridge::channel_with_timeout(Duration::from_millis(10));
        let host = LocalSendHost::start_for_test(
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0).into(),
            "GSH Test",
            limits(2, 4),
            bridge,
        )
        .unwrap();
        let sender = std::thread::spawn({
            let host_info = host.info().clone();
            let source = source.clone();
            move || {
                let peer = Peer {
                    info: host_info,
                    ip: Ipv4Addr::LOCALHOST,
                    seen_at: 0,
                };
                send_files(
                    &peer,
                    &Identity::generate(0).unwrap(),
                    &[source],
                    1024 * 1024,
                    |_, _, _| {},
                    || false,
                )
            }
        });
        let event = events.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(matches!(event, ReceiveEvent::Request(_)));
        assert!(matches!(
            sender.join().unwrap(),
            Err(NetworkError::Http(403))
        ));
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn peer_lookup_accepts_alias_case_insensitively_or_ip() {
        let peer = Peer {
            info: DeviceInfo {
                alias: "Sala".into(),
                version: "2.0".into(),
                device_model: None,
                device_type: Some("desktop".into()),
                fingerprint: "fingerprint".into(),
                port: Some(53317),
                protocol: Some("https".into()),
                download: false,
            },
            ip: Ipv4Addr::LOCALHOST,
            seen_at: 0,
        };
        assert_eq!(
            find_peer(std::slice::from_ref(&peer), "sAlA").unwrap().ip,
            peer.ip
        );
        assert_eq!(
            find_peer(std::slice::from_ref(&peer), "127.0.0.1")
                .unwrap()
                .info
                .alias,
            "Sala"
        );
        assert!(
            matches!(find_peer(&[], "Sala"), Err(PeerLookupError::NotFound(name)) if name == "Sala")
        );
    }

    #[test]
    fn peer_lookup_rejects_ambiguous_alias() {
        let make_peer = |ip| Peer {
            info: DeviceInfo {
                alias: "Laptop".into(),
                version: "2.0".into(),
                device_model: None,
                device_type: Some("desktop".into()),
                fingerprint: "fingerprint".into(),
                port: Some(53317),
                protocol: Some("https".into()),
                download: false,
            },
            ip,
            seen_at: 0,
        };
        let peers = [
            make_peer(Ipv4Addr::LOCALHOST),
            make_peer(Ipv4Addr::new(10, 0, 0, 2)),
        ];
        assert!(
            matches!(find_peer(&peers, "Laptop"), Err(PeerLookupError::Ambiguous(name)) if name == "Laptop")
        );
    }

    #[test]
    fn app_sender_propagates_cancelled_result() {
        let sender = LocalSendSender {
            identity: Identity::generate(0).unwrap(),
        };
        let cancelled = Arc::new(AtomicBool::new(true));
        let peer = Peer {
            info: sender.identity.info.clone(),
            ip: Ipv4Addr::LOCALHOST,
            seen_at: 0,
        };
        let error = sender
            .send_files(peer, Vec::new(), 1024, |_, _, _| {}, cancelled)
            .unwrap_err();
        assert!(matches!(error, NetworkError::Cancelled));
    }

    #[test]
    fn repeated_host_start_and_drop_releases_listener() {
        let probe = std::net::TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 42), 0))
            .expect("probe listener");
        let address = probe.local_addr().expect("probe addr");
        drop(probe);
        for _ in 0..20 {
            let (bridge, _events) = ReceiveBridge::channel();
            let host =
                LocalSendHost::start_for_test(address, "GSH Test", limits(2, 4), bridge).unwrap();
            assert_eq!(host.address(), address);
            drop(host);
            std::net::TcpListener::bind(address).expect("LocalSend socket released after drop");
        }
    }
}
