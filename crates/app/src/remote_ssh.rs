//! Linux terminal transport over an OpenSSH streamlocal forward.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

mod tunnel;
use tunnel::{Endpoint, Tunnel};

const IO_POLL: Duration = Duration::from_millis(150);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(12);
const FRAME_LIMIT: usize = 6 * 1024 * 1024; // Base64 of the Flutter server's 4 MiB replay.
const INPUT_CHUNK: usize = 16 * 1024;

pub fn open(
    target: String,
    socket: String,
    cwd: Option<String>,
    cols: u16,
    rows: u16,
    scrollback_lines: usize,
) -> io::Result<gsh_term::TermSession> {
    let endpoint = Endpoint::new(target, socket)?;
    check_size(cols, rows)?;
    let shared = Arc::new(Shared {
        closed: AtomicBool::new(false),
        retry_wake: Condvar::new(),
        link: Mutex::new(Link {
            writer: None,
            cols,
            rows,
        }),
    });
    let mut connection = Connection::open(&endpoint, &shared.closed)?;
    connection.send(&json!({
        "t": "pty.open", "rid": 1, "cmd": "", "cwd": cwd, "cols": cols, "rows": rows,
    }))?;
    let reply = connection.receive(Some(Instant::now() + CONNECT_TIMEOUT), &shared.closed)?;
    expect(&reply, "pty.opened", "rid", 1)?;
    let id = reply
        .get("id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 128)
        .ok_or_else(|| invalid("Invalid remote terminal ID"))?;
    let id: Arc<str> = id.into();
    let mut reader = PtyReader {
        endpoint,
        connection: Some(connection),
        shared: Arc::clone(&shared),
        id: Arc::clone(&id),
        offset: 0,
        pending: io::Cursor::new(Vec::new()),
        finished: false,
    };
    if reader.attach().is_err() {
        reader.disconnect();
    }
    let writer = PtyWriter {
        shared: Arc::clone(&shared),
        id: Arc::clone(&id),
    };
    let resize = Arc::clone(&shared);
    gsh_term::TermSession::from_stream(
        reader,
        writer,
        move |cols, rows| resize.resize(&id, cols, rows),
        move || shared.close(),
        cols,
        rows,
        scrollback_lines,
    )
    .map_err(io::Error::other)
}

struct Connection {
    reader: BufReader<UnixStream>,
    _tunnel: Tunnel,
}

impl Connection {
    fn open(endpoint: &Endpoint, closed: &AtomicBool) -> io::Result<Self> {
        let deadline = Instant::now() + CONNECT_TIMEOUT;
        let (stream, tunnel) = endpoint.connect(closed, deadline)?;
        stream.set_read_timeout(Some(IO_POLL))?;
        stream.set_write_timeout(Some(IO_POLL))?;
        let mut connection = Self {
            reader: BufReader::new(stream),
            _tunnel: tunnel,
        };
        connection.send(&json!({"t": "hello", "v": 2, "client": "gsh-app-rust"}))?;
        let hello = connection.receive(Some(deadline), closed)?;
        expect(&hello, "hello.ack", "v", 2)?;
        Ok(connection)
    }

    fn send(&self, value: &Value) -> io::Result<()> {
        send(self.reader.get_ref(), value)
    }

    fn receive(&mut self, deadline: Option<Instant>, closed: &AtomicBool) -> io::Result<Value> {
        receive(&mut self.reader, deadline, closed)
    }
}

struct Shared {
    link: Mutex<Link>,
    closed: AtomicBool,
    retry_wake: Condvar,
}

struct Link {
    writer: Option<UnixStream>,
    cols: u16,
    rows: u16,
}

impl Shared {
    fn close(&self) {
        let _link = lock(&self.link);
        self.closed.store(true, Ordering::Release);
        self.retry_wake.notify_all();
    }

    fn wait_before_retry(&self, delay: Duration) {
        let _ = self
            .retry_wake
            .wait_timeout_while(lock(&self.link), delay, |_| {
                !self.closed.load(Ordering::Acquire)
            });
    }

    fn resize(&self, id: &str, cols: u16, rows: u16) -> io::Result<()> {
        check_size(cols, rows)?;
        let mut link = lock(&self.link);
        if self.closed.load(Ordering::Acquire) {
            return Err(closed_error());
        }
        link.cols = cols;
        link.rows = rows;
        if link.writer.is_some() {
            let _ = link.send(&resize_frame(id, cols, rows));
        }
        Ok(())
    }
}

fn check_size(cols: u16, rows: u16) -> io::Result<()> {
    if !(2..=500).contains(&cols) || !(1..=500).contains(&rows) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Dimensões do terminal remoto fora do limite",
        ));
    }
    Ok(())
}

impl Link {
    fn send(&mut self, value: &Value) -> io::Result<()> {
        let writer = self.writer.as_ref().ok_or_else(closed_error)?;
        if let Err(error) = send(writer, value) {
            let _ = writer.shutdown(Shutdown::Both);
            self.writer = None;
            return Err(error);
        }
        Ok(())
    }
}

struct PtyWriter {
    shared: Arc<Shared>,
    id: Arc<str>,
}

impl Write for PtyWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut link = lock(&self.shared.link);
        if self.shared.closed.load(Ordering::Acquire) {
            return Err(closed_error());
        }
        let chunk = bytes.get(..bytes.len().min(INPUT_CHUNK)).unwrap_or(bytes);
        link.send(&json!({
            "t": "pty.input", "id": &*self.id, "d": STANDARD.encode(chunk),
        }))?;
        Ok(chunk.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct PtyReader {
    endpoint: Endpoint,
    connection: Option<Connection>,
    shared: Arc<Shared>,
    id: Arc<str>,
    offset: u64,
    pending: io::Cursor<Vec<u8>>,
    finished: bool,
}

impl PtyReader {
    fn attach(&mut self) -> io::Result<()> {
        let connection = self.connection.as_ref().ok_or_else(closed_error)?;
        connection.send(&json!({"t": "pty.attach", "id": &*self.id, "from": self.offset}))?;
        let mut link = lock(&self.shared.link);
        if self.shared.closed.load(Ordering::Acquire) {
            return Err(closed_error());
        }
        connection.send(&resize_frame(&self.id, link.cols, link.rows))?;
        link.writer = Some(connection.reader.get_ref().try_clone()?);
        Ok(())
    }

    fn disconnect(&mut self) {
        lock(&self.shared.link).writer = None;
        self.connection = None;
    }

    fn reconnect(&mut self) -> io::Result<()> {
        let mut delay = Duration::from_millis(250);
        while !self.shared.closed.load(Ordering::Acquire) {
            match Connection::open(&self.endpoint, &self.shared.closed) {
                Ok(connection) => {
                    self.connection = Some(connection);
                    if self.attach().is_ok() {
                        return Ok(());
                    }
                    self.disconnect();
                }
                Err(error) if error.kind() == io::ErrorKind::InvalidData => return Err(error),
                Err(_) => {}
            }
            self.shared.wait_before_retry(delay);
            delay = (delay * 2).min(Duration::from_secs(5));
        }
        Err(closed_error())
    }

    fn close_remote(&mut self) {
        lock(&self.shared.link).writer = None;
        if let Some(connection) = self.connection.as_mut() {
            let _ = connection.send(&json!({"t": "pty.kill", "id": &*self.id}));
            if connection.send(&json!({"t": "pty.list", "rid": 2})).is_ok() {
                // Wait briefly for the list response so the tunnel can deliver kill.
                let deadline = Instant::now() + Duration::from_millis(500);
                let keep_reading = AtomicBool::new(false);
                while let Ok(reply) = connection.receive(Some(deadline), &keep_reading) {
                    if reply.get("t").and_then(Value::as_str) == Some("pty.sessions")
                        && reply.get("rid").and_then(Value::as_u64) == Some(2)
                    {
                        break;
                    }
                }
            }
        }
        self.disconnect();
    }
}

impl Drop for PtyReader {
    fn drop(&mut self) {
        self.close_remote();
    }
}

impl Read for PtyReader {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        while !destination.is_empty() && !self.shared.closed.load(Ordering::Acquire) {
            let count = self.pending.read(destination)?;
            if count != 0 || self.finished {
                return Ok(count);
            }
            if self.connection.is_none() && self.reconnect().is_err() {
                self.finished = true;
                self.pending = notice("O gsh-server não pode retomar esta sessão.");
                continue;
            }
            let connection = self.connection.as_mut().ok_or_else(closed_error)?;
            let frame = match connection.receive(None, &self.shared.closed) {
                Ok(frame) => frame,
                Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                    self.disconnect();
                    self.finished = true;
                    self.pending = notice("Resposta inválida do gsh-server.");
                    continue;
                }
                Err(_) if self.shared.closed.load(Ordering::Acquire) => break,
                Err(_) => {
                    self.disconnect();
                    continue;
                }
            };
            match frame.get("t").and_then(Value::as_str) {
                Some("pty.output") if frame.get("id").and_then(Value::as_str) == Some(&self.id) => {
                    let displayed = output(&frame, &mut self.offset)?;
                    self.pending = io::Cursor::new(displayed);
                }
                Some("pty.exited") if frame.get("id").and_then(Value::as_str) == Some(&self.id) => {
                    let code = frame
                        .get("code")
                        .and_then(Value::as_i64)
                        .ok_or_else(|| invalid("Invalid exit code"))?;
                    self.close_remote();
                    self.finished = true;
                    self.pending = notice(&format!("Terminal remoto encerrado (código {code})."));
                }
                Some("err") => {
                    self.pending = notice("O gsh-server recusou a operação no terminal.");
                    if matches!(
                        frame.get("code").and_then(Value::as_str),
                        Some("session_not_found" | "sessionNotFound")
                    ) {
                        self.disconnect();
                        self.finished = true;
                    }
                }
                _ => {}
            }
        }
        Ok(0)
    }
}

fn output(frame: &Value, offset: &mut u64) -> io::Result<Vec<u8>> {
    let start = frame
        .get("off")
        .and_then(Value::as_u64)
        .ok_or_else(|| invalid("Invalid PTY output offset"))?;
    let data = STANDARD
        .decode(
            frame
                .get("d")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("Invalid PTY output"))?,
        )
        .map_err(|_| invalid("Invalid PTY output encoding"))?;
    let end = start
        .checked_add(data.len() as u64)
        .ok_or_else(|| invalid("Invalid PTY output size"))?;
    if end <= *offset {
        return Ok(Vec::new());
    }
    let displayed = if start > *offset {
        let mut gap = notice("Parte do histórico remoto saiu do buffer.").into_inner();
        gap.extend_from_slice(&data);
        gap
    } else if start < *offset {
        data.get((*offset - start) as usize..)
            .unwrap_or_default()
            .to_vec()
    } else {
        data
    };
    *offset = end;
    Ok(displayed)
}

fn resize_frame(id: &str, cols: u16, rows: u16) -> Value {
    json!({"t": "pty.resize", "id": id, "cols": cols, "rows": rows})
}

fn notice(text: &str) -> io::Cursor<Vec<u8>> {
    io::Cursor::new(format!("\x18\x1b[0m\r\n[GSH] {text}\r\n").into_bytes())
}

fn send(mut stream: &UnixStream, value: &Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    bytes.push(b'\n');
    stream.write_all(&bytes)
}

fn receive(
    reader: &mut impl BufRead,
    deadline: Option<Instant>,
    closed: &AtomicBool,
) -> io::Result<Value> {
    let mut frame = Vec::new();
    loop {
        if closed.load(Ordering::Acquire) {
            return Err(closed_error());
        }
        if deadline.is_some_and(|at| Instant::now() >= at) {
            return Err(timeout("Tempo esgotado na resposta do gsh-server"));
        }
        let bytes = match reader.fill_buf() {
            Ok([]) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof)),
            Ok(bytes) => bytes,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock
                        | io::ErrorKind::TimedOut
                        | io::ErrorKind::Interrupted
                ) =>
            {
                continue
            }
            Err(error) => return Err(error),
        };
        let newline = bytes.iter().position(|&b| b == b'\n');
        let len = newline.map_or(bytes.len(), |i| i + 1);
        if frame.len() + len > FRAME_LIMIT {
            return Err(invalid("GSH server frame too large"));
        }
        frame.extend_from_slice(bytes.get(..len).unwrap_or(bytes));
        reader.consume(len);
        if newline.is_some() {
            return serde_json::from_slice(&frame).map_err(|_| invalid("Invalid GSH server frame"));
        }
    }
}

fn expect(frame: &Value, kind: &str, field: &str, value: u64) -> io::Result<()> {
    if frame.get("t").and_then(Value::as_str) == Some(kind)
        && frame.get(field).and_then(Value::as_u64) == Some(value)
    {
        return Ok(());
    }
    let detail = frame
        .get("detail")
        .and_then(Value::as_str)
        .or_else(|| frame.get("code").and_then(Value::as_str));
    Err(invalid(
        detail.unwrap_or("Resposta inesperada do gsh-server"),
    ))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}

fn timeout(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, message)
}

fn closed_error() -> io::Error {
    io::Error::new(io::ErrorKind::NotConnected, "Terminal remoto desconectado")
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_ssh_option_and_forward_syntax_in_arguments() {
        for target in ["-Fconfig", "user@-host", "user@host;id", "user@one@two"] {
            assert!(Endpoint::new(target.into(), "/tmp/gsh.sock".into()).is_err());
        }
        for socket in ["relative", "/tmp/sock:22", "/tmp/%h.sock", "/tmp/\nsock"] {
            assert!(Endpoint::new("me@host".into(), socket.into()).is_err());
        }
        assert!(Endpoint::new("host-alias".into(), "/tmp/gsh.sock".into()).is_ok());
    }

    #[test]
    fn replay_preserves_binary_output_and_trims_prior_bytes() {
        let frame = |offset, bytes: &[u8]| json!({"off": offset, "d": STANDARD.encode(bytes)});
        let mut offset = 3;
        assert_eq!(
            output(&frame(1, b"abc\xff"), &mut offset).unwrap(),
            b"c\xff"
        );
        assert_eq!(offset, 5);
        assert!(output(&frame(1, b"abc\xff"), &mut offset)
            .unwrap()
            .is_empty());
        assert!(output(&frame(9, b"de"), &mut offset)
            .unwrap()
            .ends_with(b"de"));
        assert_eq!(offset, 11);
    }

    #[test]
    fn reads_coalesced_frames_without_losing_the_second_line() {
        let mut wire = BufReader::with_capacity(
            3,
            &b"{\"t\":\"hello.ack\",\"v\":2}\n{\"t\":\"pty.exited\",\"code\":0}\n"[..],
        );
        let closed = AtomicBool::new(false);
        let hello = receive(&mut wire, None, &closed).unwrap();
        assert!(expect(&hello, "hello.ack", "v", 2).is_ok());
        let exit = receive(&mut wire, None, &closed).unwrap();
        assert_eq!(exit.get("t").and_then(Value::as_str), Some("pty.exited"));
    }
}
