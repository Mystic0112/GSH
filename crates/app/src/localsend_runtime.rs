//! Runtime LocalSend do app: servidor TLS, anúncio/descoberta e ponte de aceite
//! para a thread principal do iced.

use gsh_localsend::{
    DeviceInfo, Discovery, FileMetadata, Identity, Limits, LocalSendServer, Peer, ReceiveDecision,
    Receiver, ServerError, RECEIVE_DECISION_TIMEOUT,
};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver as EventReceiver, Sender, SyncSender};
use std::sync::Arc;
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

impl LocalSendHost {
    pub fn start(
        bind: SocketAddr,
        alias: &str,
        limits: Limits,
        receive_bridge: ReceiveBridge,
    ) -> Result<Self, StartError> {
        Self::start_with_discovery(bind, alias, limits, receive_bridge, true)
    }

    fn start_with_discovery(
        bind: SocketAddr,
        alias: &str,
        limits: Limits,
        receive_bridge: ReceiveBridge,
        with_discovery: bool,
    ) -> Result<Self, StartError> {
        let mut identity = Identity::generate(bind.port())?;
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

    #[cfg(test)]
    pub(super) fn start_for_test(
        bind: SocketAddr,
        alias: &str,
        limits: Limits,
        receive_bridge: ReceiveBridge,
    ) -> Result<Self, StartError> {
        Self::start_with_discovery(bind, alias, limits, receive_bridge, false)
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
}
