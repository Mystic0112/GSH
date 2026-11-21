use crate::filename::{persist_without_overwrite, sanitize_filename};
use crate::http::{
    empty_response, json_response, parse_query, read_body, read_headers, write_response, Response,
};
use crate::identity::Identity;
use crate::protocol::{DeviceInfo, FileMetadata, PrepareUpload, UploadSession, API};
use rustls::ServerConfig;
use sha2::Digest;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const SESSION_TTL: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
pub enum ReceiveDecision {
    Accept { dir: PathBuf },
    Reject,
}

pub trait Receiver: Send + Sync + 'static {
    fn accept(&self, sender: &DeviceInfo, files: &[FileMetadata]) -> ReceiveDecision;
}

#[derive(Debug, Clone)]
pub struct Limits {
    pub max_file_size: u64,
    pub max_files: usize,
    pub max_total_size: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_file_size: 2 * 1024 * 1024 * 1024,
            max_files: 100,
            max_total_size: 4 * 1024 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("cannot bind LocalSend server: {0}")]
    Io(#[from] io::Error),
    #[error("cannot configure HTTPS server: {0}")]
    Tls(String),
    #[error("server thread could not start: {0}")]
    Thread(String),
}

#[derive(Clone)]
struct PendingFile {
    metadata: FileMetadata,
    token: String,
    safe_name: String,
}
struct Session {
    expires: Instant,
    root: PathBuf,
    files: HashMap<String, PendingFile>,
    sender: SocketAddr,
}
struct State {
    identity: Identity,
    receiver: Arc<dyn Receiver>,
    limits: Limits,
    sessions: Mutex<HashMap<String, Session>>,
}

pub struct LocalSendServer {
    address: SocketAddr,
    stop: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl LocalSendServer {
    pub fn bind(
        identity: Identity,
        bind: SocketAddr,
        receiver: Arc<dyn Receiver>,
        limits: Limits,
    ) -> Result<Self, ServerError> {
        let listener = TcpListener::bind(bind)?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let mut identity = identity;
        identity.info.port = Some(address.port());
        let tls = Arc::new(
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
                .map_err(|e| ServerError::Tls(e.to_string()))?
                .with_no_client_auth()
                .with_single_cert(vec![identity.cert.clone()], identity.key.clone_key())
                .map_err(|e| ServerError::Tls(e.to_string()))?,
        );
        let state = Arc::new(State {
            identity,
            receiver,
            limits,
            sessions: Mutex::new(HashMap::new()),
        });
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = thread::Builder::new()
            .name("gsh-localsend-https".into())
            .spawn(move || {
                while !worker_stop.load(std::sync::atomic::Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, peer)) => {
                            let state = Arc::clone(&state);
                            let tls = Arc::clone(&tls);
                            let _ = thread::Builder::new()
                                .name("gsh-localsend-request".into())
                                .spawn(move || serve(stream, peer, tls, state));
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(20))
                        }
                        Err(_) => thread::sleep(Duration::from_millis(100)),
                    }
                }
            })
            .map_err(|e| ServerError::Thread(e.to_string()))?;
        Ok(Self {
            address,
            stop,
            worker: Some(worker),
        })
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}
impl Drop for LocalSendServer {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn serve(stream: TcpStream, peer: SocketAddr, config: Arc<ServerConfig>, state: Arc<State>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(20)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(20)));
    let Ok(connection) = rustls::ServerConnection::new(config) else {
        return;
    };
    let mut tls = rustls::StreamOwned::new(connection, stream);
    let Ok((method, target, headers)) = read_headers(&mut tls) else {
        return;
    };
    let length = headers
        .get("content-length")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let path = target.split('?').next().unwrap_or("");
    let query = target.split_once('?').map(|(_, query)| query).unwrap_or("");
    let response = if (method == "GET" && path == format!("{API}/info"))
        || (method == "POST" && path == format!("{API}/register"))
    {
        json_response(200, &state.identity.info)
    } else if method == "POST" && path == format!("{API}/prepare-upload") {
        let body = read_body(&mut tls, length, 1024 * 1024);
        match body.and_then(|body| serde_json::from_slice::<PrepareUpload>(&body).map_err(|_| 400))
        {
            Ok(request) => prepare(&state, peer, request),
            Err(status) => empty_response(status),
        }
    } else if method == "POST" && path == format!("{API}/upload") {
        let params = parse_query(query);
        match (
            params.get("sessionId"),
            params.get("fileId"),
            params.get("token"),
        ) {
            (Some(session), Some(file), Some(token)) => upload(
                &state,
                UploadRequest {
                    peer,
                    session_id: session,
                    file_id: file,
                    token,
                    length,
                },
                &mut tls,
            ),
            _ => empty_response(400),
        }
    } else if method == "POST" && path == format!("{API}/cancel") {
        let params = parse_query(query);
        if let Some(session) = params.get("sessionId") {
            if let Ok(mut sessions) = state.sessions.lock() {
                sessions.remove(session);
            }
            empty_response(200)
        } else {
            empty_response(400)
        }
    } else {
        empty_response(404)
    };
    let _ = write_response(&mut tls, response);
    tls.conn.send_close_notify();
    let _ = tls.flush();
}

fn prepare(state: &State, peer: SocketAddr, request: PrepareUpload) -> Response {
    let files = request.files.values().cloned().collect::<Vec<_>>();
    let total = files
        .iter()
        .try_fold(0u64, |sum, file| sum.checked_add(file.size));
    if files.is_empty() {
        return empty_response(204);
    }
    if files.len() > state.limits.max_files
        || files.iter().any(|f| f.size > state.limits.max_file_size)
        || total.is_none_or(|size| size > state.limits.max_total_size)
    {
        return empty_response(413);
    }
    if request
        .files
        .iter()
        .any(|(key, file)| key != &file.id || file.id.is_empty())
    {
        return empty_response(400);
    }
    if files
        .iter()
        .any(|file| sanitize_filename(&file.file_name).is_err())
    {
        return empty_response(400);
    }
    let ReceiveDecision::Accept { dir } = state.receiver.accept(&request.info, &files) else {
        return empty_response(403);
    };
    if fs::create_dir_all(&dir).is_err() {
        return empty_response(500);
    }
    let Ok(mut session_id) = random_token() else {
        return empty_response(500);
    };
    if let Ok(mut sessions) = state.sessions.lock() {
        sessions.retain(|_, session| session.expires > Instant::now());
    }
    while state
        .sessions
        .lock()
        .map(|sessions| sessions.contains_key(&session_id))
        .unwrap_or(true)
    {
        let Ok(next) = random_token() else {
            return empty_response(500);
        };
        session_id = next;
    }
    let mut accepted = HashMap::new();
    let mut response_files = std::collections::BTreeMap::new();
    for metadata in files {
        let Some(id) = request
            .files
            .iter()
            .find_map(|(id, item)| (item.id == metadata.id).then(|| id.clone()))
        else {
            continue;
        };
        let safe_name = match sanitize_filename(&metadata.file_name) {
            Ok(name) => name,
            Err(_) => return empty_response(400),
        };
        let Ok(token) = random_token() else {
            return empty_response(500);
        };
        response_files.insert(id.clone(), token.clone());
        accepted.insert(
            id,
            PendingFile {
                metadata,
                token,
                safe_name,
            },
        );
    }
    let session = Session {
        expires: Instant::now() + SESSION_TTL,
        root: dir,
        files: accepted,
        sender: peer,
    };
    if let Ok(mut sessions) = state.sessions.lock() {
        sessions.insert(session_id.clone(), session);
    } else {
        return empty_response(500);
    }
    json_response(
        200,
        &UploadSession {
            session_id,
            files: response_files,
        },
    )
}

struct UploadRequest<'a> {
    peer: SocketAddr,
    session_id: &'a str,
    file_id: &'a str,
    token: &'a str,
    length: u64,
}

fn upload(
    state: &State,
    request: UploadRequest<'_>,
    tls: &mut rustls::StreamOwned<rustls::ServerConnection, TcpStream>,
) -> Response {
    let UploadRequest {
        peer,
        session_id,
        file_id,
        token,
        length,
    } = request;
    let (root, file) = {
        let Ok(mut sessions) = state.sessions.lock() else {
            return empty_response(500);
        };
        sessions.retain(|_, session| session.expires > Instant::now());
        let Some(session) = sessions.get(session_id) else {
            return empty_response(403);
        };
        if session.sender.ip() != peer.ip() {
            return empty_response(403);
        }
        let Some(file) = session.files.get(file_id) else {
            return empty_response(403);
        };
        if !constant_time_eq(file.token.as_bytes(), token.as_bytes()) {
            return empty_response(403);
        }
        if length != file.metadata.size || length > state.limits.max_file_size {
            return empty_response(413);
        }
        (session.root.clone(), file.clone())
    };
    let file_key = sha2::Sha256::digest(file_id.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let part = root.join(format!(".localsend-{session_id}-{file_key}.part"));
    let Ok(mut output) = OpenOptions::new().write(true).create_new(true).open(&part) else {
        return empty_response(500);
    };
    let mut remaining = length;
    let mut checksum = sha2::Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    while remaining > 0 {
        let take = buffer.len().min(remaining as usize);
        match tls.read(&mut buffer[..take]) {
            Ok(0) | Err(_) => {
                let _ = fs::remove_file(&part);
                return empty_response(400);
            }
            Ok(n) => {
                checksum.update(&buffer[..n]);
                if output.write_all(&buffer[..n]).is_err() {
                    let _ = fs::remove_file(&part);
                    return empty_response(500);
                }
                remaining -= n as u64;
            }
        }
    }
    if output.sync_all().is_err() {
        let _ = fs::remove_file(&part);
        return empty_response(500);
    }
    if let Some(expected) = &file.metadata.sha256 {
        let actual = checksum
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = fs::remove_file(&part);
            return empty_response(422);
        }
    }
    drop(output);
    if persist_without_overwrite(&part, &root, &file.safe_name).is_err() {
        let _ = fs::remove_file(&part);
        return empty_response(500);
    }
    if let Ok(mut sessions) = state.sessions.lock() {
        if let Some(session) = sessions.get_mut(session_id) {
            session.files.remove(file_id);
            if session.files.is_empty() {
                sessions.remove(session_id);
            }
        }
    }
    empty_response(200)
}

fn random_token() -> io::Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

include!("receive_tests.rs");
