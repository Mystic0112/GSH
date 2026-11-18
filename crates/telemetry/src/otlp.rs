use std::io::Read;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use bytes::Bytes;
use flate2::read::GzDecoder;
use http::{Method, Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use serde_json::{Map, Value};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::fingerprint::fingerprint_of;
use crate::model::{
    now_ms, TelemetryError, TelemetryEvent, TelemetryFrame, TelemetrySeverity, TelemetryStream,
};
use crate::parser::frames::parse_frame;

pub trait OtlpSink: Send + Sync {
    fn run_for(&self, run_id: Option<&str>, service_name: Option<&str>) -> Option<String>;
    fn add_events(&self, run_id: &str, events: Vec<TelemetryEvent>);
    fn project_roots_of(&self, run_id: &str) -> Vec<String>;
}

pub struct OtlpReceiver {
    sink: Arc<dyn OtlpSink>,
    endpoint: RwLock<String>,
    task: RwLock<Option<JoinHandle<()>>>,
}

impl OtlpReceiver {
    pub fn new(sink: Arc<dyn OtlpSink>) -> Self {
        Self {
            sink,
            endpoint: RwLock::new(String::new()),
            task: RwLock::new(None),
        }
    }

    pub fn endpoint(&self) -> String {
        self.endpoint.read().map(|g| g.clone()).unwrap_or_default()
    }

    pub async fn start(&self) {
        if !self.endpoint().is_empty() {
            return;
        }
        let Ok(listener) = TcpListener::bind("127.0.0.1:0").await else {
            return;
        };
        let Ok(addr) = listener.local_addr() else {
            return;
        };
        if let Ok(mut g) = self.endpoint.write() {
            *g = format!("http://127.0.0.1:{}", addr.port());
        }
        let sink = Arc::clone(&self.sink);
        let handle = tokio::spawn(async move {
            serve_loop(listener, sink).await;
        });
        if let Ok(mut t) = self.task.write() {
            *t = Some(handle);
        }
    }

    pub fn stop(&self) {
        if let Ok(mut t) = self.task.write() {
            if let Some(h) = t.take() {
                h.abort();
            }
        }
        if let Ok(mut g) = self.endpoint.write() {
            g.clear();
        }
    }
}

impl Drop for OtlpReceiver {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn serve_loop(listener: TcpListener, sink: Arc<dyn OtlpSink>) {
    while let Ok((stream, _peer)) = listener.accept().await {
        let sink = Arc::clone(&sink);
        tokio::spawn(async move {
            let io = TokioIo::new(stream);
            let svc = service_fn(move |req| {
                let sink = Arc::clone(&sink);
                async move { Ok::<_, std::convert::Infallible>(handle_request(req, &*sink).await) }
            });
            let _ = http1::Builder::new().serve_connection(io, svc).await;
        });
    }
}

async fn handle_request(req: Request<Incoming>, sink: &dyn OtlpSink) -> Response<Full<Bytes>> {
    if req.method() != Method::POST {
        return Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::new()))
            .unwrap_or_default();
    }
    let path = req.uri().path().to_string();
    let ct = req
        .headers()
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ct.contains("json") {
        let _ = req.into_body().collect().await;
        return Response::builder()
            .status(StatusCode::UNSUPPORTED_MEDIA_TYPE)
            .body(Full::new(Bytes::new()))
            .unwrap_or_default();
    }
    let is_gzip = req
        .headers()
        .get(http::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.eq_ignore_ascii_case("gzip"));

    let Ok(collected) = req.into_body().collect().await else {
        return bad_request();
    };
    let raw_bytes = collected.to_bytes();
    let body_text = if is_gzip {
        let mut decoder = GzDecoder::new(&raw_bytes[..]);
        let mut decoded = String::new();
        if decoder.read_to_string(&mut decoded).is_err() {
            return bad_request();
        }
        decoded
    } else {
        match String::from_utf8(raw_bytes.to_vec()) {
            Ok(s) => s,
            Err(_) => return bad_request(),
        }
    };

    let Ok(json) = serde_json::from_str::<Value>(&body_text) else {
        return bad_request();
    };
    if let Value::Object(map) = json {
        if path == "/v1/logs" {
            process_logs(&map, sink);
        } else if path == "/v1/traces" {
            process_traces(&map, sink);
        }
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from_static(b"{}")))
        .unwrap_or_default()
}

fn bad_request() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Full::new(Bytes::new()))
        .unwrap_or_default()
}

fn process_logs(payload: &Map<String, Value>, sink: &dyn OtlpSink) {
    for rl in obj_list(payload.get("resourceLogs")) {
        let res = parse_attrs(
            rl.get("resource")
                .and_then(|v| v.as_object())
                .and_then(|r| r.get("attributes")),
        );
        let Some(run_id) = run_of(&res, sink) else {
            continue;
        };
        let roots = sink.project_roots_of(&run_id);
        let mut out = Vec::new();
        for sl in obj_list(rl.get("scopeLogs")) {
            for r in obj_list(sl.get("logRecords")) {
                let mut attrs = parse_attrs(r.get("attributes"));
                let sev = parse_otlp_severity(r.get("severityNumber"), r.get("severityText"));
                let body = r
                    .get("body")
                    .and_then(any_value)
                    .map(|v| val_to_str(&v))
                    .unwrap_or_default();
                let ex_type = attrs
                    .remove("exception.type")
                    .filter(|v| !v.is_null())
                    .map(|v| val_to_str(&v));
                let ex_msg = attrs
                    .remove("exception.message")
                    .filter(|v| !v.is_null())
                    .map(|v| val_to_str(&v));
                let ex_stack = attrs
                    .remove("exception.stacktrace")
                    .filter(|v| !v.is_null())
                    .map(|v| val_to_str(&v));

                let err = if ex_type.is_some()
                    || ex_stack.is_some()
                    || (sev.is_problem()
                        && sev.index() >= TelemetrySeverity::Error.index()
                        && ex_msg.is_some())
                {
                    let frames = parse_frames(ex_stack.as_deref(), &roots);
                    Some(TelemetryError {
                        err_type: ex_type.unwrap_or_else(|| "Error".to_string()),
                        message: ex_msg.unwrap_or_else(|| body.clone()),
                        stack: ex_stack,
                        frames,
                    })
                } else {
                    None
                };

                let mut merged_attrs = Map::new();
                if let Some(svc) = res.get("service.name").filter(|v| !v.is_null()) {
                    merged_attrs.insert("service".to_string(), svc.clone());
                }
                for (k, v) in attrs {
                    merged_attrs.insert(k, v);
                }

                let fingerprint = err.as_ref().map(fingerprint_of);
                let final_body = if body.is_empty() {
                    err.as_ref().map(|e| e.message.clone()).unwrap_or_default()
                } else {
                    body
                };

                out.push(TelemetryEvent {
                    id: None,
                    run_id: run_id.clone(),
                    ts_ms: parse_nanos(
                        r.get("timeUnixNano")
                            .or_else(|| r.get("observedTimeUnixNano")),
                    ),
                    stream: TelemetryStream::Otlp,
                    severity: sev,
                    body: final_body,
                    attrs: merged_attrs,
                    error: err,
                    fingerprint,
                    trace_id: r.get("traceId").filter(|v| !v.is_null()).map(val_to_str),
                    span_id: r.get("spanId").filter(|v| !v.is_null()).map(val_to_str),
                    request_id: None,
                    raw_line_offset: None,
                    repeat: 1,
                });
            }
        }
        if !out.is_empty() {
            sink.add_events(&run_id, out);
        }
    }
}

fn process_traces(payload: &Map<String, Value>, sink: &dyn OtlpSink) {
    for rs in obj_list(payload.get("resourceSpans")) {
        let res = parse_attrs(
            rs.get("resource")
                .and_then(|v| v.as_object())
                .and_then(|r| r.get("attributes")),
        );
        let Some(run_id) = run_of(&res, sink) else {
            continue;
        };
        let roots = sink.project_roots_of(&run_id);
        let mut out = Vec::new();
        for ss in obj_list(rs.get("scopeSpans")) {
            for sp in obj_list(ss.get("spans")) {
                let attrs = parse_attrs(sp.get("attributes"));
                let start_ms = parse_nanos(sp.get("startTimeUnixNano"));
                let end_ms = parse_nanos(sp.get("endTimeUnixNano"));
                let duration_ms = end_ms - start_ms;
                let status = sp.get("status").and_then(|v| v.as_object());
                let failed = status.is_some_and(|st| {
                    matches!(st.get("code"), Some(Value::Number(n)) if n.as_i64() == Some(2))
                        || matches!(st.get("code"), Some(Value::String(s)) if s == "STATUS_CODE_ERROR")
                });
                let name = sp
                    .get("name")
                    .filter(|v| !v.is_null())
                    .map(val_to_str)
                    .unwrap_or_else(|| "span".to_string());
                let http_status = attrs
                    .get("http.response.status_code")
                    .or_else(|| attrs.get("http.status_code"))
                    .cloned();
                let http_status_str = http_status
                    .as_ref()
                    .filter(|v| !v.is_null())
                    .map(val_to_str)
                    .unwrap_or_default();

                let mut err: Option<TelemetryError> = None;
                for ev in obj_list(sp.get("events")) {
                    if ev.get("name").and_then(|v| v.as_str()) == Some("exception") {
                        let ea = parse_attrs(ev.get("attributes"));
                        let stack = ea
                            .get("exception.stacktrace")
                            .filter(|v| !v.is_null())
                            .map(val_to_str);
                        let frames = parse_frames(stack.as_deref(), &roots);
                        err = Some(TelemetryError {
                            err_type: ea
                                .get("exception.type")
                                .filter(|v| !v.is_null())
                                .map(val_to_str)
                                .unwrap_or_else(|| "Error".to_string()),
                            message: ea
                                .get("exception.message")
                                .filter(|v| !v.is_null())
                                .map(val_to_str)
                                .unwrap_or_else(|| name.clone()),
                            stack,
                            frames,
                        });
                        break;
                    }
                }
                if err.is_none() && failed {
                    let msg = status
                        .and_then(|st| st.get("message"))
                        .filter(|v| !v.is_null())
                        .map(val_to_str)
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| name.clone());
                    let err_type = match http_status.as_ref() {
                        Some(hs) => format!("HTTP {}", val_to_str(hs)),
                        None => "SpanError".to_string(),
                    };
                    err = Some(TelemetryError {
                        err_type,
                        message: msg,
                        stack: None,
                        frames: Vec::new(),
                    });
                }

                let mut merged_attrs = Map::new();
                if let Some(svc) = res.get("service.name").filter(|v| !v.is_null()) {
                    merged_attrs.insert("service".to_string(), svc.clone());
                }
                merged_attrs.insert("span".to_string(), Value::String(name.clone()));
                merged_attrs.insert("duration_ms".to_string(), Value::from(duration_ms));
                if let Some(parent) = sp
                    .get("parentSpanId")
                    .filter(|v| !v.is_null())
                    .map(val_to_str)
                    .filter(|s| !s.is_empty())
                {
                    merged_attrs.insert("parent_span_id".to_string(), Value::String(parent));
                }
                let req_id = attrs
                    .get("http.request.header.x-request-id")
                    .or_else(|| attrs.get("x-request-id"))
                    .filter(|v| !v.is_null())
                    .map(val_to_str);
                for (k, v) in attrs {
                    merged_attrs.insert(k, v);
                }

                let body = format!("{name} {http_status_str} {duration_ms}ms")
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                let fingerprint = err.as_ref().map(fingerprint_of);
                let sev = if err.is_some() {
                    TelemetrySeverity::Error
                } else {
                    TelemetrySeverity::Debug
                };

                out.push(TelemetryEvent {
                    id: None,
                    run_id: run_id.clone(),
                    ts_ms: start_ms,
                    stream: TelemetryStream::Otlp,
                    severity: sev,
                    body,
                    attrs: merged_attrs,
                    error: err,
                    fingerprint,
                    trace_id: sp.get("traceId").filter(|v| !v.is_null()).map(val_to_str),
                    span_id: sp.get("spanId").filter(|v| !v.is_null()).map(val_to_str),
                    request_id: req_id,
                    raw_line_offset: None,
                    repeat: 1,
                });
            }
        }
        if !out.is_empty() {
            sink.add_events(&run_id, out);
        }
    }
}

fn run_of(res: &Map<String, Value>, sink: &dyn OtlpSink) -> Option<String> {
    let run_id = res.get("gsh.run").filter(|v| !v.is_null()).map(val_to_str);
    let svc = res
        .get("service.name")
        .filter(|v| !v.is_null())
        .map(val_to_str);
    sink.run_for(run_id.as_deref(), svc.as_deref())
}

fn obj_list(v: Option<&Value>) -> Vec<&Map<String, Value>> {
    match v {
        Some(Value::Array(arr)) => arr.iter().filter_map(|e| e.as_object()).collect(),
        _ => Vec::new(),
    }
}

fn parse_attrs(v: Option<&Value>) -> Map<String, Value> {
    let mut out = Map::new();
    for kv in obj_list(v) {
        let Some(k) = kv.get("key").filter(|v| !v.is_null()).map(val_to_str) else {
            continue;
        };
        if let Some(val) = kv.get("value").and_then(any_value) {
            out.insert(k, val);
        }
    }
    out
}

fn any_value(v: &Value) -> Option<Value> {
    let Some(m) = v.as_object() else {
        return Some(v.clone());
    };
    if let Some(s) = m.get("stringValue") {
        return Some(s.clone());
    }
    if let Some(iv) = m.get("intValue") {
        if let Some(s) = iv.as_str() {
            if let Ok(n) = s.parse::<i64>() {
                return Some(Value::from(n));
            }
        }
        return Some(iv.clone());
    }
    if let Some(d) = m.get("doubleValue") {
        return Some(d.clone());
    }
    if let Some(b) = m.get("boolValue") {
        return Some(b.clone());
    }
    if let Some(arr) = m
        .get("arrayValue")
        .and_then(|a| a.as_object())
        .and_then(|a| a.get("values"))
    {
        let items = obj_list(Some(arr))
            .into_iter()
            .filter_map(|item| any_value(&Value::Object(item.clone())))
            .collect();
        return Some(Value::Array(items));
    }
    if let Some(kv) = m
        .get("kvlistValue")
        .and_then(|k| k.as_object())
        .and_then(|k| k.get("values"))
    {
        return Some(Value::Object(parse_attrs(Some(kv))));
    }
    None
}

fn parse_nanos(v: Option<&Value>) -> i64 {
    let Some(v) = v.filter(|v| !v.is_null()) else {
        return now_ms();
    };
    let nanos: Option<i64> = match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    };
    nanos.map_or_else(now_ms, |n| n / 1_000_000)
}

fn parse_otlp_severity(number: Option<&Value>, text: Option<&Value>) -> TelemetrySeverity {
    let n = number.filter(|v| !v.is_null()).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse::<i64>().ok(),
        _ => None,
    });
    if let Some(n) = n {
        if n >= 21 {
            return TelemetrySeverity::Fatal;
        }
        if n >= 17 {
            return TelemetrySeverity::Error;
        }
        if n >= 13 {
            return TelemetrySeverity::Warn;
        }
        if n >= 9 {
            return TelemetrySeverity::Info;
        }
        if n >= 5 {
            return TelemetrySeverity::Debug;
        }
        if n >= 1 {
            return TelemetrySeverity::Trace;
        }
    }
    TelemetrySeverity::parse_value(text)
}

fn parse_frames(stack: Option<&str>, roots: &[String]) -> Vec<TelemetryFrame> {
    let Some(st) = stack else {
        return Vec::new();
    };
    st.lines()
        .filter_map(|l| parse_frame(l, roots, &[]))
        .collect()
}

fn val_to_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

#[allow(dead_code)]
fn _unused(_: SocketAddr) {}
