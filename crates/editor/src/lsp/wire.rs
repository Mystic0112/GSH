use serde_json::{json, Value};
use std::{io, time::Duration};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const MAX_HEADER: usize = 8 * 1024;
const MAX_INCOMING: usize = 8 * 1024 * 1024;
// Full document notifications can expand when JSON-escaped.
const MAX_OUTGOING: usize = 32 * 1024 * 1024;

pub(super) fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(super) async fn read_frame(reader: &mut (impl AsyncBufRead + Unpin)) -> io::Result<Value> {
    let mut remaining = MAX_HEADER;
    let mut length = None;
    loop {
        let mut line = Vec::new();
        let count = reader
            .take(remaining as u64)
            .read_until(b'\n', &mut line)
            .await?;
        if !line.ends_with(b"\r\n") {
            return Err(invalid("incomplete or oversized LSP header"));
        }
        remaining -= count;
        if line == b"\r\n" {
            break;
        }
        let header = std::str::from_utf8(&line).map_err(|_| invalid("invalid LSP header"))?;
        let (name, value) = header
            .split_once(':')
            .ok_or_else(|| invalid("invalid LSP header"))?;
        if name.eq_ignore_ascii_case("Content-Length") {
            let size = value
                .trim()
                .parse::<usize>()
                .ok()
                .filter(|size| (1..=MAX_INCOMING).contains(size))
                .ok_or_else(|| invalid("invalid LSP Content-Length"))?;
            if length.replace(size).is_some() {
                return Err(invalid("duplicate LSP Content-Length"));
            }
        }
    }
    let mut body = vec![0; length.ok_or_else(|| invalid("missing LSP Content-Length"))?];
    reader.read_exact(&mut body).await?;
    let message: Value = serde_json::from_slice(&body)?;
    if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(invalid("invalid JSON-RPC version"));
    }
    let valid_id = |id: &Value| id.is_string() || id.as_i64().is_some() || id.is_null();
    let id = message.get("id");
    let result = message.get("result");
    let error = message.get("error");
    let valid = match message.get("method") {
        Some(method) => {
            method.as_str().is_some_and(|method| !method.is_empty())
                && id.is_none_or(|id| valid_id(id) && !id.is_null())
                && result.is_none()
                && error.is_none()
                && message.get("params").is_none_or(|params| {
                    params.is_null() || params.is_object() || params.is_array()
                })
        }
        None => {
            id.is_some_and(valid_id)
                && (result.is_some() ^ error.is_some())
                && error.is_none_or(|error| {
                    error.get("code").and_then(Value::as_i64).is_some()
                        && error.get("message").and_then(Value::as_str).is_some()
                })
        }
    };
    valid
        .then_some(message)
        .ok_or_else(|| invalid("invalid JSON-RPC message"))
}

pub(super) async fn write_frame(
    writer: &mut (impl AsyncWrite + Unpin),
    value: Value,
) -> io::Result<()> {
    let body = serde_json::to_vec(&value)?;
    if body.len() > MAX_OUTGOING {
        return Err(invalid("LSP message exceeds size limit"));
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        writer
            .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
            .await?;
        writer.write_all(&body).await?;
        writer.flush().await
    })
    .await?
}

pub(super) fn request(id: i64, method: &str, params: Value) -> Value {
    if params.is_null() {
        json!({"jsonrpc": "2.0", "id": id, "method": method})
    } else {
        json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
    }
}

pub(super) fn notification(method: &str, params: Value) -> Value {
    if params.is_null() {
        json!({"jsonrpc": "2.0", "method": method})
    } else {
        json!({"jsonrpc": "2.0", "method": method, "params": params})
    }
}

pub(super) fn response_id(message: &Value) -> Option<i64> {
    if message.get("method").is_none() {
        return message.get("id")?.as_i64();
    }
    None
}

pub(super) fn rpc_result(mut message: Value) -> io::Result<Value> {
    if let Some(result) = message.get_mut("result") {
        Ok(result.take())
    } else {
        let detail = message
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("LSP request failed");
        Err(io::Error::other(
            detail.chars().take(240).collect::<String>(),
        ))
    }
}
