//! Transporte: uma requisição por conexão, uma linha JSON em cada direção.
//!
//! POSIX usa socket Unix (`GSH_STATUS_SOCK`); Windows usa TCP no loopback
//! (`GSH_STATUS_PORT` + `GSH_STATUS_TOKEN`), porque lá não há UDS. O token só
//! importa no TCP: o loopback é acessível por qualquer processo local, enquanto
//! no UDS a permissão do arquivo já protege.

use std::io::{self, Read, Write};
use std::time::Duration;

use gsh_core::wire::{self, Response, ENV_STATUS_PORT, ENV_STATUS_SOCK, ENV_STATUS_TOKEN};
use serde_json::Value;
use thiserror::Error;

use crate::env::{env_non_empty, home_dir};

/// Flavor assado no binário (`GSH_FLAVOR=debug` na hora de compilar). O app
/// namespaceia o socket por flavor, e a CLI procura primeiro o do seu.
const FLAVOR: Option<&str> = option_env!("GSH_FLAVOR");

#[derive(Debug, Error)]
pub enum TransportError {
    #[error(
        "no GSH app to talk to (GSH_STATUS_SOCK is unset and no socket found in ~/.gsh). \
Is the app running?"
    )]
    NotConfigured,
    #[error("not inside a GSH terminal (GSH_STATUS_SOCK is unset)")]
    NotInTerminal,
    #[error("could not connect to app: {0}")]
    Connect(#[source] io::Error),
}

/// Conexão aberta com o app, abstraindo UDS x TCP.
enum Conn {
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
    Tcp(std::net::TcpStream),
}

impl Conn {
    fn set_read_timeout(&self, dur: Duration) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.set_read_timeout(Some(dur)),
            Conn::Tcp(s) => s.set_read_timeout(Some(dur)),
        }
    }
}

impl Read for Conn {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.read(buf),
            Conn::Tcp(s) => s.read(buf),
        }
    }
}

impl Write for Conn {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.write(buf),
            Conn::Tcp(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.flush(),
            Conn::Tcp(s) => s.flush(),
        }
    }
}

#[cfg(unix)]
fn candidate_sockets() -> Vec<String> {
    wire::candidate_sockets(
        env_non_empty(ENV_STATUS_SOCK),
        home_dir(),
        FLAVOR == Some("debug"),
    )
}

/// Windows: `{port, tok}` que o app grava em `~/.gsh/status[-debug].json` ao
/// subir, o equivalente do caminho bem conhecido do socket Unix para a CLI
/// rodada de fora de uma aba. Mesma ordem de flavor do POSIX.
#[cfg(windows)]
fn endpoint_from_file() -> Option<(u16, Option<String>)> {
    let home = home_dir()?;
    let mut names = ["status.json", "status-debug.json"];
    if FLAVOR == Some("debug") {
        names.reverse();
    }
    names.iter().find_map(|name| {
        let raw = std::fs::read_to_string(format!("{home}\\.gsh\\{name}")).ok()?;
        wire::parse_endpoint_file(&raw)
    })
}

#[cfg(not(windows))]
fn endpoint_from_file() -> Option<(u16, Option<String>)> {
    None
}

fn env_port() -> Option<u16> {
    env_non_empty(ENV_STATUS_PORT).and_then(|p| p.parse::<u16>().ok())
}

fn connect() -> Result<Conn, TransportError> {
    let port = env_port().or_else(|| endpoint_from_file().map(|(p, _)| p));

    #[cfg(unix)]
    {
        let mut last_err: Option<io::Error> = None;
        for path in candidate_sockets() {
            // Socket órfão (app fechado sem limpar) existe no disco mas recusa
            // conexão: quem decide é o connect, não o exists.
            match std::os::unix::net::UnixStream::connect(&path) {
                Ok(s) => return Ok(Conn::Unix(s)),
                Err(e) => last_err = Some(e),
            }
        }
        if port.is_none() {
            return Err(last_err.map_or(TransportError::NotInTerminal, TransportError::Connect));
        }
    }

    match port {
        Some(port) => std::net::TcpStream::connect(("127.0.0.1", port))
            .map(Conn::Tcp)
            .map_err(TransportError::Connect),
        None => Err(TransportError::NotInTerminal),
    }
}

/// `true` quando há algum caminho possível até o app (env de socket, porta, ou
/// um socket bem conhecido no disco).
fn transport_configured() -> bool {
    if env_non_empty(ENV_STATUS_SOCK).is_some() || env_port().is_some() {
        return true;
    }
    #[cfg(unix)]
    {
        candidate_sockets()
            .iter()
            .any(|p| std::path::Path::new(p).exists())
    }
    #[cfg(not(unix))]
    endpoint_from_file().is_some()
}

/// Token do TCP: a env vence; sem ela, o arquivo de endpoint (Windows).
fn token() -> Option<String> {
    std::env::var(ENV_STATUS_TOKEN)
        .ok()
        .or_else(|| endpoint_from_file().and_then(|(_, tok)| tok))
}

/// Envia uma requisição de comando e devolve a resposta decodificada. Resposta
/// ausente ou malformada vira um `Response` com `ok: false` (erro de aplicação);
/// só a falta de app ou de conexão é `Err`.
pub fn request(req: Value, timeout: Duration) -> Result<Response, TransportError> {
    if !transport_configured() {
        return Err(TransportError::NotConfigured);
    }
    let req = match token() {
        Some(tok) => wire::with_token(req, &tok),
        None => req,
    };

    let mut conn = connect()?;
    let _ = conn.set_read_timeout(timeout);

    let mut payload = req.to_string();
    payload.push('\n');
    conn.write_all(payload.as_bytes())
        .and_then(|_| conn.flush())
        .map_err(TransportError::Connect)?;

    // O servidor responde UMA linha e fecha. Lemos até a primeira quebra (ou o
    // EOF), o que também protege se ele mantiver a conexão aberta.
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut chunk = [0u8; 4096];
    loop {
        match conn.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(chunk.get(..n).unwrap_or_default());
                if buf.contains(&b'\n') {
                    break;
                }
            }
            Err(_) => break, // timeout ou erro: trata como resposta ausente
        }
    }
    Ok(Response::parse(&String::from_utf8_lossy(&buf)))
}

/// Entrega uma linha de status (sem resposta) pelo socket do hook: o Unix se
/// `GSH_STATUS_SOCK` existir, senão o TCP do loopback. `None` = não entregou.
pub fn send_status(line: &str, sock: Option<&str>, port: Option<u16>) -> Option<()> {
    #[cfg(unix)]
    if let Some(path) = sock {
        let mut s = std::os::unix::net::UnixStream::connect(path).ok()?;
        s.write_all(line.as_bytes()).ok()?;
        let _ = s.flush();
        return Some(());
    }
    #[cfg(not(unix))]
    let _ = sock;

    let mut s = std::net::TcpStream::connect(("127.0.0.1", port?)).ok()?;
    s.write_all(line.as_bytes()).ok()?;
    let _ = s.flush();
    Some(())
}
