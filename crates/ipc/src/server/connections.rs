use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use gsh_core::status::{decode_line, Incoming};
use serde_json::Value;

use super::{Delivery, PendingCommand, Shared};

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

// Suporta rajadas de hooks sem deixar que o cliente crie workers sem limite.
const MAX_CONNECTIONS: usize = 64;
const MAX_LINE_BYTES: u64 = 16 * 1024 * 1024;
const FRAME_TIMEOUT: Duration = Duration::from_secs(10);
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);
const IO_POLL: Duration = Duration::from_millis(25);

const REPLY_INVALID_TOKEN: &[u8] = b"{\"ok\":false,\"error\":\"invalid token\"}\n";
const REPLY_COMMANDS_UNAVAILABLE: &[u8] = b"{\"ok\":false,\"error\":\"commands unavailable\"}\n";

pub(super) enum Listener {
    #[cfg(unix)]
    Unix(UnixListener),
    Tcp(TcpListener),
}

impl Listener {
    pub(super) fn set_nonblocking(&self) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Listener::Unix(l) => l.set_nonblocking(true),
            Listener::Tcp(l) => l.set_nonblocking(true),
        }
    }

    fn accept(&self) -> io::Result<Stream> {
        match self {
            #[cfg(unix)]
            Listener::Unix(l) => l.accept().map(|(s, _)| Stream::Unix(s)),
            Listener::Tcp(l) => l.accept().map(|(s, _)| Stream::Tcp(s)),
        }
    }
}

enum Stream {
    #[cfg(unix)]
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl Stream {
    fn set_nonblocking(&self) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.set_nonblocking(true),
            Stream::Tcp(s) => s.set_nonblocking(true),
        }
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.read(buf),
            Stream::Tcp(s) => s.read(buf),
        }
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.write(buf),
            Stream::Tcp(s) => s.write(buf),
        }
    }
}

/// Servir apenas conexões que têm vaga evita enfileirar trabalho enquanto
/// comandos de permissão podem aguardar a decisão do app por horas.
pub(super) fn accept_loop(listener: Listener, shared: &Arc<Shared>) {
    let mut workers = Vec::<JoinHandle<()>>::with_capacity(MAX_CONNECTIONS);
    while !shared.stopping() {
        for index in (0..workers.len()).rev() {
            if workers.get(index).is_some_and(JoinHandle::is_finished) {
                // A conexão já encerrou. Um panic do worker afeta só esse cliente.
                let _ = workers.swap_remove(index).join();
            }
        }
        match listener.accept() {
            Ok(stream) => {
                if workers.len() == MAX_CONNECTIONS || stream.set_nonblocking().is_err() {
                    continue;
                }
                let deadline = Instant::now() + FRAME_TIMEOUT;
                let shared = Arc::clone(shared);
                if let Ok(worker) = thread::Builder::new()
                    .name("gsh-status-conn".into())
                    .spawn(move || handle(stream, &shared, deadline))
                {
                    workers.push(worker);
                }
            }
            Err(_) => thread::sleep(IO_POLL),
        }
    }
    drop(listener);
    for worker in workers {
        // Aguarda os workers liberarem sockets e canais antes de retornar.
        let _ = worker.join();
    }
}

struct DeadlineIo<'a> {
    stream: &'a mut Stream,
    shared: &'a Shared,
    deadline: Instant,
}

impl DeadlineIo<'_> {
    fn poll<T>(
        &mut self,
        mut operation: impl FnMut(&mut Stream) -> io::Result<T>,
    ) -> io::Result<T> {
        loop {
            if self.shared.stopping() {
                return Err(io::ErrorKind::ConnectionAborted.into());
            }
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(io::ErrorKind::TimedOut.into());
            }
            match operation(self.stream) {
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(remaining.min(IO_POLL));
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                result => return result,
            }
        }
    }
}

impl Read for DeadlineIo<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.poll(|stream| stream.read(buf))
    }
}

impl Write for DeadlineIo<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.poll(|stream| stream.write(buf))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(()) // O socket escreve diretamente, sem buffer em espaço de usuário.
    }
}

fn handle(mut stream: Stream, shared: &Shared, deadline: Instant) {
    let reader = DeadlineIo {
        stream: &mut stream,
        shared,
        deadline,
    };
    let Ok(line) = read_line_capped(reader, MAX_LINE_BYTES) else {
        return;
    };
    if shared.stopping() || Instant::now() >= deadline {
        return;
    }
    let reply: Option<Vec<u8>> = match decode_line(&line, shared.token.as_deref()) {
        Ok(Incoming::Status(update)) => {
            let _ = shared.tx.send(Delivery::Status(update));
            None
        }
        Ok(Incoming::Command) => match serde_json::from_str::<Value>(&line) {
            Ok(request) => {
                let timeout =
                    if request.get("cmd").and_then(Value::as_str) == Some("permission-ask") {
                        Duration::from_secs(86_400)
                    } else {
                        command_timeout(&request)
                    };
                let (reply, response) = mpsc::sync_channel(1);
                if shared
                    .tx
                    .send(Delivery::Command(PendingCommand { request, reply }))
                    .is_ok()
                {
                    let Some(value) = wait_for_response(response, timeout, shared) else {
                        return;
                    };
                    serde_json::to_vec(&value).ok().map(|mut bytes| {
                        bytes.push(b'\n');
                        bytes
                    })
                } else {
                    Some(REPLY_COMMANDS_UNAVAILABLE.to_vec())
                }
            }
            _ => Some(REPLY_COMMANDS_UNAVAILABLE.to_vec()),
        },
        Ok(Incoming::BadToken { command: true }) => Some(REPLY_INVALID_TOKEN.to_vec()),
        Ok(Incoming::BadToken { command: false } | Incoming::Ignored) => None,
        Err(err) => {
            let _ = shared.tx.send(Delivery::Malformed(err));
            None
        }
    };
    if let Some(reply) = reply {
        let mut writer = DeadlineIo {
            stream: &mut stream,
            shared,
            deadline: Instant::now() + REPLY_TIMEOUT,
        };
        let _ = writer.write_all(&reply); // O cliente pode fechar antes de receber.
    }
}

fn wait_for_response(
    response: Receiver<Value>,
    timeout: Duration,
    shared: &Shared,
) -> Option<Value> {
    let deadline = Instant::now() + timeout;
    while !shared.stopping() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        match response.recv_timeout(remaining.min(IO_POLL)) {
            Ok(value) => return Some(value),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    (!shared.stopping()).then(|| serde_json::json!({"ok": false, "error": "command timed out"}))
}

/// LocalSend mantém a requisição aberta durante descoberta/envio; seu prazo é
/// definido pelo CLI e limitado aqui também. Os demais comandos conservam o
/// timeout histórico, inclusive o fluxo de permissões.
fn command_timeout(request: &Value) -> Duration {
    let Some(command) = request.get("cmd").and_then(Value::as_str) else {
        return Duration::from_secs(45);
    };
    let args = request.get("args");
    let seconds = match command {
        "localsend-peers" => args
            .and_then(|value| value.get("waitSeconds"))
            .and_then(Value::as_u64)
            .map(|wait| wait.clamp(1, 30).saturating_add(10)),
        "localsend-send" => args
            .and_then(|value| value.get("timeoutSeconds"))
            .and_then(Value::as_u64)
            .map(|timeout| timeout.clamp(1, 300).saturating_add(15)),
        _ => None,
    };
    Duration::from_secs(seconds.unwrap_or(45))
}

/// Lê até a primeira quebra de linha, ou o fim da conexão, ou `cap` bytes. A
/// linha sem `\n` final conta (o cliente do hook fecha logo após escrever).
fn read_line_capped<R: Read>(reader: R, cap: u64) -> io::Result<String> {
    let mut buf = Vec::new();
    BufReader::new(reader.take(cap)).read_until(b'\n', &mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragmentos_nao_renovam_o_prazo_do_frame() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (socket, _) = listener.accept().unwrap();
        let mut stream = Stream::Tcp(socket);
        stream.set_nonblocking().unwrap();
        let (tx, _) = mpsc::channel();
        let shared = Shared {
            stop: Arc::default(),
            token: None,
            tx,
        };

        thread::scope(|scope| {
            client.write_all(b"a").unwrap();
            scope.spawn(move || {
                for _ in 0..12 {
                    thread::sleep(Duration::from_millis(25));
                    client.write_all(b"a").unwrap();
                }
                client.write_all(b"\n").unwrap();
            });
            let reader = DeadlineIo {
                stream: &mut stream,
                shared: &shared,
                deadline: Instant::now() + Duration::from_millis(200),
            };
            let error = read_line_capped(reader, MAX_LINE_BYTES).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        });
    }

    #[test]
    fn le_so_a_primeira_linha() {
        let got = read_line_capped(&b"um\ndois\n"[..], 100).unwrap();
        assert_eq!(got, "um\n");
    }

    #[test]
    fn linha_sem_quebra_final_vale() {
        assert_eq!(
            read_line_capped(&b"{\"a\":1}"[..], 100).unwrap(),
            "{\"a\":1}"
        );
        assert_eq!(read_line_capped(&b""[..], 100).unwrap(), "");
    }

    #[test]
    fn teto_corta_linha_gigante() {
        let got = read_line_capped(&b"abcdefghij\n"[..], 4).unwrap();
        assert_eq!(got, "abcd");
    }

    #[test]
    fn localsend_command_deadlines_are_bounded_and_other_commands_are_unchanged() {
        assert_eq!(
            command_timeout(
                &serde_json::json!({"cmd":"localsend-peers","args":{"waitSeconds":90}})
            ),
            Duration::from_secs(40)
        );
        assert_eq!(
            command_timeout(
                &serde_json::json!({"cmd":"localsend-send","args":{"timeoutSeconds":-1}})
            ),
            Duration::from_secs(45)
        );
        assert_eq!(
            command_timeout(&serde_json::json!({"cmd":"send"})),
            Duration::from_secs(45)
        );
    }
}
