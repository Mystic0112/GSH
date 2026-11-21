use super::{NetworkError, Peer};
use crate::identity::Identity;
use crate::protocol::{FileMetadata, PrepareUpload, UploadSession, API, PORT};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub fn send_files(
    peer: &Peer,
    identity: &Identity,
    files: &[std::path::PathBuf],
    max_file_size: u64,
    mut progress: impl FnMut(&Path, u64, u64),
    cancelled: impl Fn() -> bool,
) -> Result<(), NetworkError> {
    let port = peer.info.port.unwrap_or(PORT);
    let mut metadata = std::collections::BTreeMap::new();
    for (index, path) in files.iter().enumerate() {
        let file = File::open(path)?;
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
    let response = https_request(
        SocketAddr::new(peer.ip.into(), port),
        &peer.info.fingerprint,
        "POST",
        &format!("{API}/prepare-upload"),
        &serde_json::to_vec(&request)?,
    )?;
    let session: UploadSession = serde_json::from_slice(&response)?;
    for (index, path) in files.iter().enumerate() {
        if cancelled() {
            let _ = https_request(
                SocketAddr::new(peer.ip.into(), port),
                &peer.info.fingerprint,
                "POST",
                &format!("{API}/cancel?sessionId={}", session.session_id),
                &[],
            );
            return Ok(());
        }
        let token = session
            .files
            .get(&format!("file-{index}"))
            .ok_or(NetworkError::Http(403))?;
        let size = metadata
            .get(&format!("file-{index}"))
            .map(|m| m.size)
            .unwrap_or_default();
        let mut file = File::open(path)?;
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
            |sent| progress(path, sent, size),
        )?;
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

pub(crate) fn https_request(
    address: SocketAddr,
    fingerprint: &str,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<Vec<u8>, NetworkError> {
    let tcp = TcpStream::connect_timeout(&address, Duration::from_secs(10))?;
    tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(30)))?;
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
    let mut tls = rustls::StreamOwned::new(connection, tcp);
    write!(tls, "{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len())?;
    tls.write_all(body)?;
    let mut response = Vec::new();
    tls.read_to_end(&mut response)?;
    parse_http_response(&response)
}

fn https_stream_file(
    address: SocketAddr,
    fingerprint: &str,
    path: &str,
    file: &mut File,
    size: u64,
    mut progress: impl FnMut(u64),
) -> Result<u16, NetworkError> {
    let tcp = TcpStream::connect_timeout(&address, Duration::from_secs(10))?;
    tcp.set_read_timeout(Some(Duration::from_secs(30)))?;
    tcp.set_write_timeout(Some(Duration::from_secs(30)))?;
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
    let mut tls = rustls::StreamOwned::new(connection, tcp);
    write!(tls, "POST {path} HTTP/1.1\r\nHost: {address}\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n")?;
    let mut sent = 0u64;
    let mut buf = [0u8; 64 * 1024];
    while sent < size {
        let max = buf.len().min((size - sent) as usize);
        let n = file.read(&mut buf[..max])?;
        if n == 0 {
            break;
        }
        tls.write_all(&buf[..n])?;
        sent += n as u64;
        progress(sent);
    }
    let mut response = Vec::new();
    tls.read_to_end(&mut response)?;
    let (status, _) = parse_http(&response)?;
    Ok(status)
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
