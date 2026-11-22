use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Write},
    os::unix::net::UnixStream,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    linux::Sessions,
    pty::{self, Event, Session},
};

const VERSION: u32 = 2;
const MAX_FRAME: usize = 64 * 1024;
const OUTPUT_CHUNK: usize = 16 * 1024;
const OUTPUT_WINDOW: usize = 256 * 1024;

#[derive(Deserialize)]
#[serde(tag = "t")]
enum Request {
    #[serde(rename = "hello")]
    Hello { v: u32, client: String },
    #[serde(rename = "pty.open")]
    Open {
        #[serde(flatten)]
        spec: pty::Open,
    },
    #[serde(rename = "pty.list")]
    List,
    #[serde(rename = "pty.attach")]
    Attach {
        id: String,
        #[serde(default)]
        from: u64,
    },
    #[serde(rename = "pty.detach")]
    Detach { id: String },
    #[serde(rename = "pty.input")]
    Input { id: String, d: String },
    #[serde(rename = "pty.ack")]
    Ack { id: String, n: usize },
    #[serde(rename = "pty.resize")]
    Resize { id: String, rows: u16, cols: u16 },
    #[serde(rename = "pty.kill")]
    Kill { id: String },
    #[serde(rename = "rpc")]
    Rpc { rid: i64 },
    #[serde(other)]
    Unknown,
}

struct Attachment {
    session: Weak<Session>,
    next: u64,
    credit: usize,
}

pub(crate) fn connect(
    mut stream: UnixStream,
    sessions: &Sessions,
    stop: &AtomicBool,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_millis(12)))?;
    stream.set_write_timeout(Some(Duration::from_secs(1)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut attachments = HashMap::new();
    let mut frame = Vec::new();
    let handshake_by = Instant::now() + Duration::from_secs(5);
    let mut handshaken = false;
    while !stop.load(Ordering::Acquire) {
        if !handshaken && Instant::now() >= handshake_by {
            return Ok(());
        }
        pump(&mut stream, &mut attachments)?;
        match read_frame(&mut reader, &mut frame) {
            Ok(Frame::Closed) => return Ok(()),
            Ok(Frame::Waiting) => continue,
            Ok(Frame::Ready) => {}
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                return send(&mut stream, json!({"t":"err","code":"frame_too_large"}));
            }
            Err(error) => return Err(error),
        }
        let parsed: Result<Value, _> = serde_json::from_slice(&frame);
        frame.clear();
        let parsed = parsed.unwrap_or(Value::Null);
        let rid = parsed.get("rid").and_then(Value::as_i64);
        let id = parsed
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| id.len() <= 36)
            .map(str::to_owned);
        let valid_rid = parsed.get("rid").is_none_or(|r| r.is_null() || r.is_i64());
        let request = serde_json::from_value::<Request>(parsed);
        if !handshaken {
            match request {
                Ok(Request::Hello { v: VERSION, client })
                    if !client.is_empty() && client.len() <= 256 =>
                {
                    let exe = std::env::current_exe()
                        .ok()
                        .map(|path| path.to_string_lossy().into_owned());
                    send(
                        &mut stream,
                        json!({
                            "t":"hello.ack", "v":VERSION, "server":env!("CARGO_PKG_VERSION"),
                            "pid":std::process::id(), "exe":exe
                        }),
                    )?;
                    handshaken = true;
                }
                Ok(Request::Hello { v, .. }) if v != VERSION => {
                    return send(
                        &mut stream,
                        json!({"t":"err","code":"version_mismatch",
                        "detail":format!("server={VERSION} client={v}")}),
                    );
                }
                _ => return send(&mut stream, json!({"t":"err","code":"handshake_required"})),
            }
            continue;
        }
        let response = match request {
            Ok(request) if valid_rid => dispatch(request, rid, sessions, &mut attachments),
            _ => Err(Failure::code("bad_message")),
        };
        match response {
            Ok(Some(response)) => send(&mut stream, response)?,
            Ok(None) => {}
            Err(failure) => {
                let mut response = json!({"t":"err", "code":failure.code});
                if let Some(detail) = failure.detail {
                    response["detail"] = json!(detail);
                }
                if let Some(id) = id {
                    response["id"] = json!(id);
                }
                send(&mut stream, correlated(response, rid))?;
            }
        }
    }
    Ok(())
}

fn dispatch(
    request: Request,
    rid: Option<i64>,
    sessions: &Sessions,
    attachments: &mut HashMap<String, Attachment>,
) -> Result<Option<Value>, Failure> {
    let find = |id: &str| {
        sessions
            .with_sessions(|sessions| sessions.iter().find(|session| session.id == id).cloned())
            .ok_or_else(|| Failure::code("session_not_found"))
    };
    match request {
        Request::Open { spec } => {
            let session = sessions
                .open(spec)
                .map_err(|error| Failure::io("spawn_failed", error))?;
            Ok(Some(correlated(
                json!({"t":"pty.opened","id":session.id,"pid":session.pid}),
                rid,
            )))
        }
        Request::List => {
            let list = sessions.with_sessions(|sessions| {
                sessions
                    .iter()
                    .map(|session| session.info())
                    .collect::<Vec<_>>()
            });
            Ok(Some(correlated(
                json!({"t":"pty.sessions","sessions":list}),
                rid,
            )))
        }
        Request::Attach { id, from } => {
            let session = find(&id)?;
            if !session.has_offset(from) {
                return Err(Failure::code("invalid_offset"));
            }
            attachments.insert(
                id,
                Attachment {
                    session: Arc::downgrade(&session),
                    next: from,
                    credit: OUTPUT_WINDOW,
                },
            );
            Ok(None)
        }
        Request::Detach { id } => {
            attachments.remove(&id);
            Ok(None)
        }
        Request::Input { id, d } => {
            if d.len() > MAX_INPUT_ENCODED {
                return Err(Failure::code("input_too_large"));
            }
            let bytes = STANDARD
                .decode(d)
                .map_err(|_| Failure::code("bad_message"))?;
            if bytes.len() > pty::MAX_INPUT_BYTES {
                return Err(Failure::code("input_too_large"));
            }
            if !bytes.is_empty() {
                find(&id)?
                    .input(bytes)
                    .map_err(|error| Failure::io("input_failed", error))?;
            }
            Ok(None)
        }
        Request::Ack { id, n } => {
            if let Some(attachment) = attachments.get_mut(&id) {
                attachment.credit = attachment.credit.saturating_add(n).min(OUTPUT_WINDOW);
            }
            Ok(None)
        }
        Request::Resize { id, rows, cols } => {
            let size =
                pty::dimensions(rows, cols).map_err(|error| Failure::io("invalid_size", error))?;
            find(&id)?
                .resize(size)
                .map_err(|error| Failure::io("resize_failed", error))?;
            Ok(None)
        }
        Request::Kill { id } => {
            attachments.remove(&id);
            if !sessions.kill(&id) {
                return Err(Failure::code("session_not_found"));
            }
            Ok(None)
        }
        Request::Rpc { rid } => Ok(Some(json!({"t":"rpc.res","rid":rid,"ok":false,
            "code":"unsupported_method"}))),
        Request::Unknown => Ok(None),
        Request::Hello { .. } => Err(Failure::code("bad_message")),
    }
}

const MAX_INPUT_ENCODED: usize = pty::MAX_INPUT_BYTES.div_ceil(3) * 4;

fn pump(stream: &mut UnixStream, attachments: &mut HashMap<String, Attachment>) -> io::Result<()> {
    let mut result = Ok(());
    attachments.retain(|id, attachment| {
        if result.is_err() {
            return false;
        }
        // Bounded work per terminal leaves space for keyboard, resize and acknowledgments.
        let Some(session) = attachment.session.upgrade() else {
            result = send(
                stream,
                json!({"t":"err","code":"session_not_found","id":id}),
            );
            return false;
        };
        let limit = if session.flow {
            OUTPUT_CHUNK.min(attachment.credit)
        } else {
            OUTPUT_CHUNK
        };
        match session.event(attachment.next, limit) {
            Some(Event::Output(offset, bytes)) => {
                attachment.next = offset + bytes.len() as u64;
                attachment.credit = attachment.credit.saturating_sub(bytes.len());
                result = send(
                    stream,
                    json!({"t":"pty.output","id":id,"off":offset,
                    "d":STANDARD.encode(bytes)}),
                );
                true
            }
            Some(Event::Exit(code)) => {
                result = send(stream, json!({"t":"pty.exited","id":id,"code":code}));
                false
            }
            None => true,
        }
    });
    result
}

struct Failure {
    code: &'static str,
    detail: Option<String>,
}

impl Failure {
    fn code(code: &'static str) -> Self {
        Self { code, detail: None }
    }

    fn io(code: &'static str, error: io::Error) -> Self {
        let mut detail = error.to_string();
        if let Some((boundary, _)) = detail.char_indices().find(|(index, _)| *index >= 1024) {
            detail.truncate(boundary);
        }
        Self {
            code,
            detail: Some(detail),
        }
    }
}

fn correlated(mut response: Value, rid: Option<i64>) -> Value {
    if let Some(rid) = rid {
        response["rid"] = json!(rid);
    }
    response
}

enum Frame {
    Closed,
    Ready,
    Waiting,
}

fn read_frame(reader: &mut impl BufRead, partial: &mut Vec<u8>) -> io::Result<Frame> {
    loop {
        let buffer = match reader.fill_buf() {
            Ok([]) => return Ok(Frame::Closed),
            Ok(buffer) => buffer,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Ok(Frame::Waiting)
            }
            Err(error) => return Err(error),
        };
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let end = newline.unwrap_or(buffer.len());
        if partial.len() + end > MAX_FRAME {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "frame too large",
            ));
        }
        partial.extend_from_slice(&buffer[..end]);
        reader.consume(end + usize::from(newline.is_some()));
        if newline.is_some() {
            return Ok(Frame::Ready);
        }
    }
}

fn send(stream: &mut UnixStream, message: Value) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(&message)?;
    bytes.push(b'\n');
    stream.write_all(&bytes)
}
