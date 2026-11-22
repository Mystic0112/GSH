use super::{NetworkError, Peer};
use crate::identity::Identity;
use crate::protocol::{FileMetadata, PrepareUpload, UploadSession, API, PORT};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const IDLE_TIMEOUT: Duration = Duration::from_secs(30);
// The receiver can wait 30 seconds for the user's decision.
#[cfg(test)]
const REQUEST_TIMEOUT: Duration = Duration::from_secs(45);
const UPLOAD_TIMEOUT: Duration = Duration::from_secs(300);
const MAX_REQUEST_BODY: usize = 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;

struct DeadlineSocket {
    tcp: TcpStream,
    deadline: Instant,
}

impl DeadlineSocket {
    fn remaining(&self) -> io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::TimedOut,
                    "LocalSend request deadline elapsed",
                )
            })
    }
}

impl Read for DeadlineSocket {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.tcp
            .set_read_timeout(Some(self.remaining()?.min(IDLE_TIMEOUT)))?;
        self.tcp.read(buf).map_err(socket_timeout)
    }
}

impl Write for DeadlineSocket {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tcp
            .set_write_timeout(Some(self.remaining()?.min(IDLE_TIMEOUT)))?;
        self.tcp.write(buf).map_err(socket_timeout)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.remaining()?;
        self.tcp.flush()
    }
}

fn socket_timeout(error: io::Error) -> io::Error {
    if error.kind() == io::ErrorKind::WouldBlock {
        io::Error::new(io::ErrorKind::TimedOut, error)
    } else {
        error
    }
}

pub fn send_files(
    peer: &Peer,
    identity: &Identity,
    files: &[PathBuf],
    max_file_size: u64,
    progress: impl FnMut(&Path, u64, u64),
    cancelled: impl Fn() -> bool,
) -> Result<(), NetworkError> {
    if cancelled() {
        return Err(NetworkError::Cancelled);
    }
    let files = files
        .iter()
        .map(|path| Ok((path.clone(), File::open(path)?)))
        .collect::<std::io::Result<Vec<_>>>()?;
    send_open_files(peer, identity, files, max_file_size, progress, cancelled)
}

/// Sends already opened files, using each path only as its display name.
pub fn send_open_files(
    peer: &Peer,
    identity: &Identity,
    files: Vec<(PathBuf, File)>,
    max_file_size: u64,
    progress: impl FnMut(&Path, u64, u64),
    cancelled: impl Fn() -> bool,
) -> Result<(), NetworkError> {
    send_open_files_until(
        peer,
        identity,
        files,
        max_file_size,
        Instant::now() + Duration::from_secs(24 * 60 * 60),
        progress,
        cancelled,
    )
}

pub fn send_open_files_until(
    peer: &Peer,
    identity: &Identity,
    files: Vec<(PathBuf, File)>,
    max_file_size: u64,
    deadline: Instant,
    mut progress: impl FnMut(&Path, u64, u64),
    cancelled: impl Fn() -> bool,
) -> Result<(), NetworkError> {
    if cancelled() {
        return Err(NetworkError::Cancelled);
    }
    let port = peer.info.port.unwrap_or(PORT);
    let mut metadata = std::collections::BTreeMap::new();
    for (index, (path, file)) in files.iter().enumerate() {
        let size = file.metadata()?.len();
        if size > max_file_size {
            return Err(NetworkError::TooLarge);
        }
        let id = format!("file-{index}");
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_owned();
        metadata.insert(
            id.clone(),
            FileMetadata {
                id,
                file_name: name,
                size,
                file_type: "application/octet-stream".into(),
                sha256: None,
                preview: None,
                metadata: None,
            },
        );
    }
    let request = PrepareUpload {
        info: identity.info.clone(),
        files: metadata.clone(),
    };
    let response = https_request_until(
        SocketAddr::new(peer.ip.into(), port),
        &peer.info.fingerprint,
        "POST",
        &format!("{API}/prepare-upload"),
        &serde_json::to_vec(&request)?,
        deadline,
    )?;
    let session: UploadSession = serde_json::from_slice(&response)?;
    for (index, (path, mut file)) in files.into_iter().enumerate() {
        if cancelled() {
            let _ = https_request_until(
                SocketAddr::new(peer.ip.into(), port),
                &peer.info.fingerprint,
                "POST",
                &format!("{API}/cancel?sessionId={}", session.session_id),
                &[],
                deadline,
            );
            return Err(NetworkError::Cancelled);
        }
        let token = session
            .files
            .get(&format!("file-{index}"))
            .ok_or(NetworkError::Http(403))?;
        let size = metadata
            .get(&format!("file-{index}"))
            .map(|m| m.size)
            .unwrap_or_default();
        let file_id = format!("file-{index}");
        let response = https_stream_file(
            SocketAddr::new(peer.ip.into(), port),
            &peer.info.fingerprint,
            &format!(
                "{API}/upload?sessionId={}&fileId={}&token={}",
                session.session_id, file_id, token
            ),
            &mut file,
            size,
            |sent| progress(&path, sent, size),
            &UploadControl {
                cancelled: &cancelled,
                deadline,
            },
        );
        let response = match response {
            Ok(status) => status,
            Err(NetworkError::Cancelled) => {
                let _ = https_request_until(
                    SocketAddr::new(peer.ip.into(), port),
                    &peer.info.fingerprint,
                    "POST",
                    &format!("{API}/cancel?sessionId={}", session.session_id),
                    &[],
                    deadline,
                );
                return Err(NetworkError::Cancelled);
            }
            Err(error) => return Err(error),
        };
        if response != 200 {
            return Err(NetworkError::Http(response));
        }
    }
    Ok(())
}

struct PinVerifier {
    fingerprint: String,
}
impl std::fmt::Debug for PinVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinVerifier").finish()
    }
}
impl ServerCertVerifier for PinVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let hash = Sha256::digest(end_entity.as_ref());
        let actual = hash
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":");
        if actual.eq_ignore_ascii_case(&self.fingerprint) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::UnknownIssuer,
            ))
        }
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

#[cfg(test)]
pub(crate) fn https_request(
    address: SocketAddr,
    fingerprint: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<Vec<u8>, NetworkError> {
    https_request_with_timeout(address, fingerprint, method, path, body, REQUEST_TIMEOUT)
}

pub(super) fn https_request_with_timeout(
    address: SocketAddr,
    fingerprint: &str,
    method: &str,
    path: &str,
    body: &[u8],
    timeout: Duration,
) -> Result<Vec<u8>, NetworkError> {
    https_request_until(
        address,
        fingerprint,
        method,
        path,
        body,
        Instant::now() + timeout,
    )
}

fn https_request_until(
    address: SocketAddr,
    fingerprint: &str,
    method: &str,
    path: &str,
    body: &[u8],
    deadline: Instant,
) -> Result<Vec<u8>, NetworkError> {
    if body.len() > MAX_REQUEST_BODY {
        return Err(NetworkError::RequestTooLarge);
    }
    let header = request_header(address, method, path, body.len() as u64)?;
    let mut tls = connect_tls_until(address, fingerprint, deadline)?;
    tls.write_all(header.as_bytes())?;
    tls.write_all(body)?;
    let response = read_response(&mut tls)?;
    parse_http_response(&response)
}

fn connect_tls_until(
    address: SocketAddr,
    fingerprint: &str,
    deadline: Instant,
) -> Result<rustls::StreamOwned<rustls::ClientConnection, DeadlineSocket>, NetworkError> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "LocalSend deadline elapsed"))?;
    let tcp = TcpStream::connect_timeout(&address, remaining.min(CONNECT_TIMEOUT))?;
    let config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13, &rustls::version::TLS12])
            .map_err(|e| NetworkError::Tls(e.to_string()))?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(PinVerifier {
                fingerprint: fingerprint.to_owned(),
            }))
            .with_no_client_auth();
    let name =
        ServerName::try_from("localsend.local").map_err(|e| NetworkError::Tls(e.to_string()))?;
    let connection = rustls::ClientConnection::new(Arc::new(config), name)
        .map_err(|e| NetworkError::Tls(e.to_string()))?;
    Ok(rustls::StreamOwned::new(
        connection,
        DeadlineSocket { tcp, deadline },
    ))
}

fn request_header(
    address: SocketAddr,
    method: &str,
    path: &str,
    size: u64,
) -> Result<String, NetworkError> {
    const MAX_HEADER: usize = 16 * 1024;
    if method.len().saturating_add(path.len()) >= MAX_HEADER {
        return Err(NetworkError::RequestTooLarge);
    }
    let header = format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n");
    if header.len() > MAX_HEADER {
        return Err(NetworkError::RequestTooLarge);
    }
    Ok(header)
}

struct UploadControl<'a> {
    cancelled: &'a dyn Fn() -> bool,
    deadline: Instant,
}

fn https_stream_file(
    address: SocketAddr,
    fingerprint: &str,
    path: &str,
    file: &mut File,
    size: u64,
    mut progress: impl FnMut(u64),
    control: &UploadControl<'_>,
) -> Result<u16, NetworkError> {
    let header = request_header(address, "POST", path, size)?;
    let upload_deadline = control.deadline.min(Instant::now() + UPLOAD_TIMEOUT);
    let mut tls = connect_tls_until(address, fingerprint, upload_deadline)?;
    tls.write_all(header.as_bytes())?;
    let mut sent = 0u64;
    let mut buf = [0u8; 64 * 1024];
    while sent < size {
        if (control.cancelled)() {
            return Err(NetworkError::Cancelled);
        }
        tls.sock.remaining()?;
        let max = (buf.len() as u64).min(size - sent) as usize;
        let n = file.read(&mut buf[..max])?;
        if n == 0 {
            break;
        }
        tls.write_all(&buf[..n])?;
        sent += n as u64;
        progress(sent);
    }
    let response = read_response(&mut tls)?;
    let (status, _) = parse_http(&response)?;
    Ok(status)
}

fn read_response(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, DeadlineSocket>,
) -> Result<Vec<u8>, NetworkError> {
    tls.flush()?;
    let response = read_bounded_response(tls)?;
    tls.sock.remaining()?;
    Ok(response)
}

fn read_bounded_response(response: &mut impl Read) -> Result<Vec<u8>, NetworkError> {
    let mut bytes = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RESPONSE_BYTES {
        return Err(NetworkError::ResponseTooLarge);
    }
    Ok(bytes)
}

fn parse_http_response(bytes: &[u8]) -> Result<Vec<u8>, NetworkError> {
    let (status, body) = parse_http(bytes)?;
    if status != 200 {
        return Err(NetworkError::Http(status));
    }
    Ok(body.to_vec())
}
fn parse_http(bytes: &[u8]) -> Result<(u16, &[u8]), NetworkError> {
    let split = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| NetworkError::Tls("malformed HTTP response".into()))?;
    let header =
        std::str::from_utf8(&bytes[..split]).map_err(|e| NetworkError::Tls(e.to_string()))?;
    let status = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| NetworkError::Tls("missing HTTP status".into()))?;
    Ok((status, bytes.get(split + 4..).unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn response_limit_includes_headers_and_body() {
        let bytes = vec![0; MAX_RESPONSE_BYTES + 1];
        assert_eq!(
            read_bounded_response(&mut &bytes[..MAX_RESPONSE_BYTES])
                .unwrap()
                .len(),
            MAX_RESPONSE_BYTES
        );
        let mut reader = bytes.as_slice().chain(io::repeat(0));
        assert!(matches!(
            read_bounded_response(&mut reader),
            Err(NetworkError::ResponseTooLarge)
        ));
        assert_eq!(reader.get_ref().0.len(), 0);
    }

    #[test]
    fn oversized_request_is_rejected_before_connecting() {
        let address = "127.0.0.1:0".parse().unwrap();
        let body = vec![0; MAX_REQUEST_BODY + 1];
        assert!(matches!(
            https_request(address, "", "POST", "/", &body),
            Err(NetworkError::RequestTooLarge)
        ));
        let path = "/".repeat(16 * 1024);
        assert!(matches!(
            https_request(address, "", "POST", &path, &[]),
            Err(NetworkError::RequestTooLarge)
        ));
    }

    #[test]
    fn expired_agent_deadline_stops_before_network_io() {
        let identity = Identity::generate(0).unwrap();
        let peer = Peer {
            info: identity.info.clone(),
            ip: std::net::Ipv4Addr::LOCALHOST,
            seen_at: 0,
        };
        let error = send_open_files_until(
            &peer,
            &identity,
            Vec::new(),
            1024,
            Instant::now() - Duration::from_secs(1),
            |_, _, _| {},
            || false,
        )
        .unwrap_err();
        assert!(
            matches!(error, NetworkError::Io(error) if error.kind() == io::ErrorKind::TimedOut)
        );
    }

    #[test]
    fn deadline_expires_during_a_trickling_tls_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let identity = Identity::generate(address.port()).unwrap();
        let fingerprint = identity.info.fingerprint;
        let config = rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![identity.cert], identity.key)
        .unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let connection = rustls::ServerConnection::new(Arc::new(config)).unwrap();
            let mut tls = rustls::StreamOwned::new(connection, stream);
            crate::http::read_headers(&mut tls).unwrap();
            tls.write_all(b"HTTP/1.1 200 OK\r\n\r\n").unwrap();
            tls.flush().unwrap();
            let mut bytes = 0;
            for _ in 0..40 {
                if tls.write_all(b"x").and_then(|()| tls.flush()).is_err() {
                    break;
                }
                bytes += 1;
                thread::sleep(Duration::from_millis(25));
            }
            tls.conn.send_close_notify();
            let _ = tls.flush();
            bytes
        });
        let error = https_request_with_timeout(
            address,
            &fingerprint,
            "GET",
            "/",
            &[],
            Duration::from_millis(500),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            NetworkError::Io(error) if error.kind() == io::ErrorKind::TimedOut
        ));
        assert!(server.join().unwrap() > 1);
    }
}
