use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use regex::Regex;
use serde_json::{json, Map, Value};
use sha1::{Digest, Sha1};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::task::JoinHandle;

use crate::fingerprint::fingerprint_of;
use crate::model::{
    now_ms, TelemetryError, TelemetryEvent, TelemetryFrame, TelemetrySeverity, TelemetryStream,
};
use crate::parser::frames::parse_frame;
use crate::parser::line_parser::{TelemetryLineParser, TelemetryParserConfig};

pub static VM_SERVICE_URI_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:Dart VM Service|Observatory)[^\n]*?is available at:?\s*(https?://\S+)")
        .expect("vm service uri regex")
});

static ERR_MSG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)^([A-Z][\w.]*(?:Error|Exception))(?:\s*\([^)]*\))?:\s*(.+)$")
        .expect("vm err msg regex")
});

pub type VmSink = Arc<dyn Fn(Vec<TelemetryEvent>) + Send + Sync>;

pub struct VmServiceAttachment {
    closed: Arc<AtomicBool>,
    task: Option<JoinHandle<()>>,
}

struct RpcClient {
    tx: mpsc::Sender<String>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>>,
    seq: AtomicU64,
}

impl RpcClient {
    async fn call(&self, method: &str, params: Value) -> Option<Value> {
        let id = self.seq.fetch_add(1, Ordering::Relaxed).to_string();
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);
        if self.tx.send(req.to_string()).await.is_err() {
            self.pending.lock().await.remove(&id);
            return None;
        }
        match tokio::time::timeout(Duration::from_secs(5), rx).await {
            Ok(Ok(val)) => val.get("result").cloned(),
            _ => {
                self.pending.lock().await.remove(&id);
                None
            }
        }
    }
}

impl VmServiceAttachment {
    pub async fn connect(
        http_uri: &str,
        run_id: String,
        sink: VmSink,
        config: TelemetryParserConfig,
    ) -> Option<Self> {
        let mut parsed = url::Url::parse(http_uri).ok()?;
        let path = parsed.path().to_string();
        let ws_path = if path.ends_with('/') {
            format!("{path}ws")
        } else {
            format!("{path}/ws")
        };
        parsed.set_path(&ws_path);
        let host = parsed.host_str()?.to_string();
        let port = parsed.port_or_known_default()?;

        let stream = tokio::time::timeout(
            Duration::from_secs(5),
            TcpStream::connect((host.as_str(), port)),
        )
        .await
        .ok()?
        .ok()?;

        let (reader, mut writer) = stream.into_split();
        let mut buf_reader = BufReader::new(reader);

        // RFC 6455 HTTP Upgrade handshake
        let mut key_bytes = [0u8; 16];
        let _ = getrandom::fill(&mut key_bytes);
        let ws_key = B64.encode(key_bytes);
        let path_q = match parsed.query() {
            Some(q) => format!("{}?{q}", parsed.path()),
            None => parsed.path().to_string(),
        };
        let req = format!(
            "GET {path_q} HTTP/1.1\r\nHost: {host}:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {ws_key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        );
        writer.write_all(req.as_bytes()).await.ok()?;

        let mut status_line = String::new();
        buf_reader.read_line(&mut status_line).await.ok()?;
        if !status_line.contains("101") {
            return None;
        }
        let mut accept_ok = false;
        let expected_accept = {
            let mut sha = Sha1::new();
            sha.update(format!("{ws_key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes());
            B64.encode(sha.finalize())
        };
        loop {
            let mut line = String::new();
            let n = buf_reader.read_line(&mut line).await.ok()?;
            if n == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                if k.trim().eq_ignore_ascii_case("sec-websocket-accept")
                    && v.trim() == expected_accept
                {
                    accept_ok = true;
                }
            }
        }
        if !accept_ok {
            return None;
        }

        let (out_tx, mut out_rx) = mpsc::channel::<String>(64);
        let pending: Arc<Mutex<HashMap<String, oneshot::Sender<Value>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let rpc = Arc::new(RpcClient {
            tx: out_tx,
            pending: Arc::clone(&pending),
            seq: AtomicU64::new(1),
        });
        let closed = Arc::new(AtomicBool::new(false));
        let parser = Arc::new(Mutex::new(TelemetryLineParser::new(
            run_id.clone(),
            config.clone(),
        )));

        // Writer task
        let writer_closed = Arc::clone(&closed);
        tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if writer_closed.load(Ordering::Relaxed) {
                    break;
                }
                if write_ws_text_frame(&mut writer, msg.as_bytes())
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });

        // Reader + dispatcher task
        let reader_closed = Arc::clone(&closed);
        let rpc_reader = Arc::clone(&rpc);
        let sink_reader = Arc::clone(&sink);
        let run_id_reader = run_id.clone();
        let cfg_reader = config.clone();
        let parser_reader = Arc::clone(&parser);

        let task = tokio::spawn(async move {
            while !reader_closed.load(Ordering::Relaxed) {
                let Some((opcode, payload)) = read_ws_frame(&mut buf_reader).await else {
                    break;
                };
                if opcode == 0x8 {
                    break;
                }
                if opcode == 0x9 || opcode == 0xA {
                    continue;
                }
                if opcode != 0x1 {
                    continue;
                }
                let Ok(val) = serde_json::from_slice::<Value>(&payload) else {
                    continue;
                };
                if let Some(id) = val.get("id").and_then(val_id_str) {
                    if let Some(tx) = rpc_reader.pending.lock().await.remove(&id) {
                        let _ = tx.send(val);
                    }
                    continue;
                }
                if val.get("method").and_then(|v| v.as_str()) == Some("streamNotify") {
                    if let Some(params) = val.get("params") {
                        let stream_id = params
                            .get("streamId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        if let Some(event) = params.get("event") {
                            let rpc_c = Arc::clone(&rpc_reader);
                            let sink_c = Arc::clone(&sink_reader);
                            let run_c = run_id_reader.clone();
                            let cfg_c = cfg_reader.clone();
                            let parser_c = Arc::clone(&parser_reader);
                            let closed_c = Arc::clone(&reader_closed);
                            let ev = event.clone();
                            let sid = stream_id.to_string();
                            tokio::spawn(async move {
                                if closed_c.load(Ordering::Relaxed) {
                                    return;
                                }
                                handle_stream_event(
                                    &sid, &ev, &rpc_c, &run_c, &sink_c, &parser_c, &cfg_c,
                                )
                                .await;
                            });
                        }
                    }
                }
            }
            reader_closed.store(true, Ordering::Relaxed);
        });

        // Subscribe to Logging, Extension, Isolate
        for stream_id in ["Logging", "Extension", "Isolate"] {
            let _ = rpc
                .call("streamListen", json!({"streamId": stream_id}))
                .await;
        }

        Some(Self {
            closed,
            task: Some(task),
        })
    }

    pub fn close(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
        if let Some(h) = self.task.take() {
            h.abort();
        }
    }
}

impl Drop for VmServiceAttachment {
    fn drop(&mut self) {
        self.close();
    }
}

async fn handle_stream_event(
    stream_id: &str,
    event: &Value,
    rpc: &RpcClient,
    run_id: &str,
    sink: &VmSink,
    parser: &Mutex<TelemetryLineParser>,
    config: &TelemetryParserConfig,
) {
    match stream_id {
        "Logging" => {
            let Some(record) = event.get("logRecord") else {
                return;
            };
            let iso = event
                .get("isolate")
                .and_then(|i| i.get("id"))
                .and_then(|v| v.as_str());
            let msg = resolve_instance_str(rpc, iso, record.get("message"))
                .await
                .unwrap_or_default();
            let level = record.get("level").and_then(|v| v.as_i64()).unwrap_or(800);
            let mut sev = severity_of(level);

            let err_ref = record.get("error");
            let error = if err_ref.is_some_and(|e| {
                !e.is_null() && e.get("kind").and_then(|k| k.as_str()) != Some("Null")
            }) {
                let message = resolve_instance_str(rpc, iso, err_ref)
                    .await
                    .unwrap_or_default();
                let stack = resolve_instance_str(rpc, iso, record.get("stackTrace")).await;
                let err_type = err_ref
                    .and_then(|e| e.get("class"))
                    .and_then(|c| c.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Error")
                    .to_string();
                if !sev.is_problem() {
                    sev = TelemetrySeverity::Error;
                }
                Some(build_vm_error(&err_type, &message, stack, config))
            } else {
                None
            };

            let logger = resolve_instance_str(rpc, iso, record.get("loggerName")).await;
            let mut attrs = Map::new();
            if let Some(l) = logger.filter(|s| !s.is_empty()) {
                attrs.insert("logger".to_string(), Value::String(l));
            }
            let ts_ms = record
                .get("time")
                .and_then(|v| v.as_i64())
                .unwrap_or_else(now_ms);
            let fp = error.as_ref().map(fingerprint_of);
            let body = if msg.is_empty() {
                error
                    .as_ref()
                    .map(|e| e.message.clone())
                    .unwrap_or_default()
            } else {
                msg
            };
            sink(vec![TelemetryEvent {
                id: None,
                run_id: run_id.to_string(),
                ts_ms,
                stream: TelemetryStream::Vm,
                severity: sev,
                body,
                attrs,
                error,
                fingerprint: fp,
                trace_id: None,
                span_id: None,
                request_id: None,
                raw_line_offset: None,
                repeat: 1,
            }]);
        }
        "Extension" => {
            let kind = event
                .get("extensionKind")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let data = event
                .get("extensionData")
                .and_then(|d| d.as_object())
                .cloned()
                .unwrap_or_default();
            if kind == "Flutter.Error" {
                let rendered = data
                    .get("renderedErrorText")
                    .or_else(|| data.get("description"))
                    .and_then(|v| v.as_str());
                let Some(text) = rendered.filter(|s| !s.is_empty()) else {
                    return;
                };
                let now = now_ms();
                let mut p = parser.lock().await;
                let mut out = Vec::new();
                for line in text.lines() {
                    out.extend(p.feed(line, TelemetryStream::Vm, now, None));
                }
                out.extend(p.flush());
                if !out.is_empty() {
                    sink(out);
                }
                return;
            }
            if kind.starts_with("Flutter.") || kind.starts_with("ext.") {
                return;
            }
            let sev = TelemetrySeverity::parse_value(data.get("level"));
            let body = data
                .get("msg")
                .or_else(|| data.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or(kind)
                .to_string();
            let mut attrs = Map::new();
            attrs.insert("event".to_string(), Value::String(kind.to_string()));
            for (k, v) in data {
                attrs.insert(k, v);
            }
            sink(vec![TelemetryEvent {
                id: None,
                run_id: run_id.to_string(),
                ts_ms: now_ms(),
                stream: TelemetryStream::Vm,
                severity: sev,
                body,
                attrs,
                error: None,
                fingerprint: None,
                trace_id: None,
                span_id: None,
                request_id: None,
                raw_line_offset: None,
                repeat: 1,
            }]);
        }
        "Isolate" => {
            if event.get("kind").and_then(|v| v.as_str()) != Some("PauseException") {
                return;
            }
            let Some(ex) = event.get("exception") else {
                return;
            };
            let iso = event
                .get("isolate")
                .and_then(|i| i.get("id"))
                .and_then(|v| v.as_str());
            let message = resolve_instance_str(rpc, iso, Some(ex))
                .await
                .unwrap_or_default();
            let err_type = ex
                .get("class")
                .and_then(|c| c.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("Exception");
            let err = build_vm_error(err_type, &message, None, config);
            let fp = fingerprint_of(&err);
            let mut attrs = Map::new();
            attrs.insert("paused".to_string(), Value::Bool(true));
            sink(vec![TelemetryEvent {
                id: None,
                run_id: run_id.to_string(),
                ts_ms: now_ms(),
                stream: TelemetryStream::Vm,
                severity: TelemetrySeverity::Error,
                body: message,
                attrs,
                error: Some(err),
                fingerprint: Some(fp),
                trace_id: None,
                span_id: None,
                request_id: None,
                raw_line_offset: None,
                repeat: 1,
            }]);
        }
        _ => {}
    }
}

fn build_vm_error(
    default_type: &str,
    raw_msg: &str,
    stack: Option<String>,
    config: &TelemetryParserConfig,
) -> TelemetryError {
    let frames: Vec<TelemetryFrame> = stack
        .as_deref()
        .map(|s| {
            s.lines()
                .filter_map(|l| parse_frame(l, &config.project_roots, &config.project_frames))
                .collect()
        })
        .unwrap_or_default();
    if let Some(m) = ERR_MSG_RE.captures(raw_msg) {
        TelemetryError {
            err_type: m.get(1).map_or(default_type, |g| g.as_str()).to_string(),
            message: m.get(2).map_or(raw_msg, |g| g.as_str()).to_string(),
            stack,
            frames,
        }
    } else {
        TelemetryError {
            err_type: default_type.to_string(),
            message: raw_msg.to_string(),
            stack,
            frames,
        }
    }
}

fn severity_of(level: i64) -> TelemetrySeverity {
    if level >= 1200 {
        TelemetrySeverity::Fatal
    } else if level >= 1000 {
        TelemetrySeverity::Error
    } else if level >= 900 {
        TelemetrySeverity::Warn
    } else if level >= 800 {
        TelemetrySeverity::Info
    } else if level >= 500 {
        TelemetrySeverity::Debug
    } else {
        TelemetrySeverity::Trace
    }
}

async fn resolve_instance_str(
    rpc: &RpcClient,
    isolate_id: Option<&str>,
    inst_ref: Option<&Value>,
) -> Option<String> {
    let r = inst_ref?;
    if r.is_null() || r.get("kind").and_then(|v| v.as_str()) == Some("Null") {
        return None;
    }
    let val_str = r.get("valueAsString").and_then(|v| v.as_str());
    let truncated = r
        .get("valueAsStringIsTruncated")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if let Some(s) = val_str {
        if !truncated {
            return Some(s.to_string());
        }
    }
    let Some(iso) = isolate_id else {
        return val_str.map(ToOwned::to_owned);
    };
    let obj_id = r.get("id").and_then(|v| v.as_str())?;
    if r.get("kind").and_then(|v| v.as_str()) == Some("String") {
        if let Some(full) = rpc
            .call(
                "getObject",
                json!({"isolateId": iso, "objectId": obj_id, "count": 16384}),
            )
            .await
        {
            if let Some(s) = full.get("valueAsString").and_then(|v| v.as_str()) {
                return Some(s.to_string());
            }
        }
    }
    if let Some(res) = rpc
        .call(
            "invoke",
            json!({
                "isolateId": iso,
                "targetId": obj_id,
                "selector": "toString",
                "argumentIds": []
            }),
        )
        .await
    {
        if res
            .get("valueAsStringIsTruncated")
            .and_then(|v| v.as_bool())
            == Some(true)
        {
            if let Some(rid) = res.get("id").and_then(|v| v.as_str()) {
                if let Some(full) = rpc
                    .call(
                        "getObject",
                        json!({"isolateId": iso, "objectId": rid, "count": 16384}),
                    )
                    .await
                {
                    if let Some(s) = full.get("valueAsString").and_then(|v| v.as_str()) {
                        return Some(s.to_string());
                    }
                }
            }
        }
        if let Some(s) = res.get("valueAsString").and_then(|v| v.as_str()) {
            return Some(s.to_string());
        }
    }
    val_str.map(ToOwned::to_owned)
}

fn val_id_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

async fn write_ws_text_frame(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    payload: &[u8],
) -> std::io::Result<()> {
    let mut header = Vec::with_capacity(14);
    header.push(0x81); // FIN + text frame
    let len = payload.len();
    if len <= 125 {
        header.push(0x80 | (len as u8));
    } else if len <= 65535 {
        header.push(0x80 | 126);
        header.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        header.push(0x80 | 127);
        header.extend_from_slice(&(len as u64).to_be_bytes());
    }
    let mut mask = [0u8; 4];
    let _ = getrandom::fill(&mut mask);
    header.extend_from_slice(&mask);
    let mut masked = Vec::with_capacity(len);
    for (i, b) in payload.iter().enumerate() {
        masked.push(b ^ mask[i % 4]);
    }
    writer.write_all(&header).await?;
    writer.write_all(&masked).await?;
    writer.flush().await
}

async fn read_ws_frame<R: AsyncReadExt + Unpin>(reader: &mut R) -> Option<(u8, Vec<u8>)> {
    let mut head = [0u8; 2];
    reader.read_exact(&mut head).await.ok()?;
    let opcode = head[0] & 0x0F;
    let masked = (head[1] & 0x80) != 0;
    let mut len = u64::from(head[1] & 0x7F);
    if len == 126 {
        let mut ext = [0u8; 2];
        reader.read_exact(&mut ext).await.ok()?;
        len = u64::from(u16::from_be_bytes(ext));
    } else if len == 127 {
        let mut ext = [0u8; 8];
        reader.read_exact(&mut ext).await.ok()?;
        len = u64::from_be_bytes(ext);
    }
    if len > 16 * 1024 * 1024 {
        return None;
    }
    let mut mask = [0u8; 4];
    if masked {
        reader.read_exact(&mut mask).await.ok()?;
    }
    let mut payload = vec![0u8; usize::try_from(len).ok()?];
    reader.read_exact(&mut payload).await.ok()?;
    if masked {
        for (i, b) in payload.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
    }
    Some((opcode, payload))
}
