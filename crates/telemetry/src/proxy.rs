use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use serde_json::{Map, Value};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::config::TelemetryProxyConfig;
use crate::fingerprint::fingerprint_of;
use crate::model::{now_ms, TelemetryError, TelemetryEvent, TelemetrySeverity, TelemetryStream};
use crate::redaction::{redact_text, REDACTED};

const BODY_CAP: usize = 64 * 1024;
const HOP_HEADERS: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "host",
    "content-length",
];

pub type ProxySink = Arc<dyn Fn(Vec<TelemetryEvent>) + Send + Sync>;

pub struct HttpTelemetryProxy {
    config: TelemetryProxyConfig,
    run_id: String,
    sink: ProxySink,
    client: reqwest::Client,
    task: RwLock<Option<JoinHandle<()>>>,
}

impl HttpTelemetryProxy {
    pub fn new(config: TelemetryProxyConfig, run_id: String, sink: ProxySink) -> Self {
        let client = reqwest::Client::builder()
            .no_gzip()
            .no_brotli()
            .no_deflate()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            run_id,
            sink,
            client,
            task: RwLock::new(None),
        }
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn is_running(&self) -> bool {
        self.task.read().is_ok_and(|g| g.is_some())
    }

    pub async fn start(&self) -> bool {
        if self.is_running() {
            return true;
        }
        let addr = format!("127.0.0.1:{}", self.config.listen);
        let Ok(listener) = TcpListener::bind(&addr).await else {
            return false;
        };
        let cfg = self.config.clone();
        let run_id = self.run_id.clone();
        let sink = Arc::clone(&self.sink);
        let client = self.client.clone();
        let handle = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let cfg = cfg.clone();
                let run_id = run_id.clone();
                let sink = Arc::clone(&sink);
                let client = client.clone();
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let svc = service_fn(move |req| {
                        let cfg = cfg.clone();
                        let run_id = run_id.clone();
                        let sink = Arc::clone(&sink);
                        let client = client.clone();
                        async move {
                            Ok::<_, std::convert::Infallible>(
                                handle_proxy_req(req, &cfg, &run_id, &sink, &client).await,
                            )
                        }
                    });
                    let _ = http1::Builder::new().serve_connection(io, svc).await;
                });
            }
        });
        if let Ok(mut g) = self.task.write() {
            *g = Some(handle);
        }
        true
    }

    pub fn stop(&self) {
        if let Ok(mut g) = self.task.write() {
            if let Some(h) = g.take() {
                h.abort();
            }
        }
    }

    pub async fn replay(&self, events: &[TelemetryEvent]) -> usize {
        let mut count = 0usize;
        for ev in events {
            if ev.stream != TelemetryStream::Proxy {
                continue;
            }
            let Some(method_str) = ev.attrs.get("method").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(path) = ev
                .attrs
                .get("path")
                .or_else(|| ev.attrs.get("route"))
                .and_then(|v| v.as_str())
            else {
                continue;
            };
            let Ok(method) = reqwest::Method::from_bytes(method_str.as_bytes()) else {
                continue;
            };
            let Ok(target) = build_target(&self.config.upstream, path) else {
                continue;
            };
            let mut req = self.client.request(method, target);
            let req_id = ev.request_id.clone().unwrap_or_else(|| random_hex(8));
            req = req
                .header("x-request-id", format!("{req_id}-replay"))
                .timeout(Duration::from_secs(60));
            if let Some(body) = ev.attrs.get("req_body").and_then(|v| v.as_str()) {
                if !body.is_empty() && !body.contains(REDACTED) {
                    if let Some(ct) = ev.attrs.get("req_type").and_then(|v| v.as_str()) {
                        req = req.header(reqwest::header::CONTENT_TYPE, ct);
                    }
                    req = req.body(body.to_string());
                }
            }
            if req.send().await.is_ok() {
                count += 1;
            }
        }
        count
    }
}

impl Drop for HttpTelemetryProxy {
    fn drop(&mut self) {
        self.stop();
    }
}

async fn handle_proxy_req(
    req: Request<Incoming>,
    config: &TelemetryProxyConfig,
    run_id: &str,
    sink: &ProxySink,
    client: &reqwest::Client,
) -> Response<Full<Bytes>> {
    let started = now_ms();
    let sw = Instant::now();
    let method = req.method().clone();
    let uri_str = req.uri().to_string();
    let headers = req.headers().clone();

    let req_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| random_hex(8));

    let traceparent_hdr = headers
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .map(ToOwned::to_owned);
    let trace_id = traceparent_hdr
        .as_deref()
        .and_then(|tp| tp.split('-').nth(1))
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| random_hex(16));
    let traceparent =
        traceparent_hdr.unwrap_or_else(|| format!("00-{trace_id}-{}-01", random_hex(8)));

    let req_type = headers
        .get(http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(mime_of);

    let req_bytes = req
        .into_body()
        .collect()
        .await
        .map(|c| c.to_bytes())
        .unwrap_or_default();

    let status_code: u16;
    let mut res_bytes = Bytes::new();
    let mut res_type: Option<String> = None;
    let mut failure: Option<String> = None;
    let mut out_builder = Response::builder();

    match build_target(&config.upstream, &uri_str)
        .ok()
        .and_then(|url| {
            let rm = reqwest::Method::from_bytes(method.as_str().as_bytes()).ok()?;
            Some((url, rm))
        }) {
        Some((target, rm)) => {
            let mut up = client.request(rm, target).timeout(Duration::from_secs(120));
            for (name, val) in &headers {
                let lower = name.as_str().to_ascii_lowercase();
                if HOP_HEADERS.contains(&lower.as_str()) {
                    continue;
                }
                if let Ok(v) = val.to_str() {
                    up = up.header(name.as_str(), v);
                }
            }
            up = up
                .header("x-request-id", &req_id)
                .header("traceparent", &traceparent)
                .body(req_bytes.clone());

            match up.send().await {
                Ok(res) => {
                    status_code = res.status().as_u16();
                    res_type = res
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .map(mime_of);
                    out_builder = out_builder.status(status_code);
                    for (name, val) in res.headers() {
                        let lower = name.as_str().to_ascii_lowercase();
                        if HOP_HEADERS.contains(&lower.as_str()) {
                            continue;
                        }
                        if let Ok(hv) = http::HeaderValue::from_bytes(val.as_bytes()) {
                            out_builder = out_builder.header(name.as_str(), hv);
                        }
                    }
                    out_builder = out_builder.header("x-request-id", &req_id);
                    match res.bytes().await {
                        Ok(b) => res_bytes = b,
                        Err(e) => {
                            failure = Some(e.to_string());
                        }
                    }
                }
                Err(e) => {
                    status_code = StatusCode::BAD_GATEWAY.as_u16();
                    failure = Some(e.to_string());
                    out_builder = out_builder.status(StatusCode::BAD_GATEWAY);
                    res_bytes = Bytes::from_static(b"gsh telemetry proxy: upstream unreachable");
                }
            }
        }
        None => {
            status_code = StatusCode::BAD_GATEWAY.as_u16();
            failure = Some("invalid upstream url".to_string());
            out_builder = out_builder.status(StatusCode::BAD_GATEWAY);
            res_bytes = Bytes::from_static(b"gsh telemetry proxy: upstream unreachable");
        }
    }

    let elapsed_ms = i64::try_from(sw.elapsed().as_millis()).unwrap_or(0);
    let route = uri_str.split('?').next().unwrap_or(&uri_str).to_string();
    let err = if failure.is_some() {
        Some(TelemetryError {
            err_type: "UpstreamUnreachable".to_string(),
            message: format!("{method} {route}"),
            stack: None,
            frames: Vec::new(),
        })
    } else if status_code >= 500 {
        Some(TelemetryError {
            err_type: format!("HTTP {status_code}"),
            message: format!("{method} {route}"),
            stack: None,
            frames: Vec::new(),
        })
    } else {
        None
    };

    let sev = if err.is_some() {
        TelemetrySeverity::Error
    } else if status_code >= 400 {
        TelemetrySeverity::Warn
    } else {
        TelemetrySeverity::Debug
    };

    let mut attrs = Map::new();
    attrs.insert("method".into(), Value::String(method.to_string()));
    attrs.insert("route".into(), Value::String(route.clone()));
    attrs.insert("path".into(), Value::String(uri_str.clone()));
    attrs.insert("status".into(), Value::from(status_code));
    attrs.insert("duration_ms".into(), Value::from(elapsed_ms));
    attrs.insert("request_id".into(), Value::String(req_id.clone()));
    if let Some(ref rt) = req_type {
        attrs.insert("req_type".into(), Value::String(rt.clone()));
    }
    if !req_bytes.is_empty() && is_text_mime(req_type.as_deref()) {
        attrs.insert(
            "req_body".into(),
            Value::String(redact_text(&capped_utf8(&req_bytes))),
        );
    }
    if let Some(ref rt) = res_type {
        attrs.insert("res_type".into(), Value::String(rt.clone()));
    }
    if !res_bytes.is_empty() && is_text_mime(res_type.as_deref()) {
        attrs.insert(
            "res_body".into(),
            Value::String(redact_text(&capped_utf8(&res_bytes))),
        );
    }
    if let Some(ref fail) = failure {
        attrs.insert("failure".into(), Value::String(fail.clone()));
    }

    let fp = err.as_ref().map(fingerprint_of);
    sink(vec![TelemetryEvent {
        id: None,
        run_id: run_id.to_string(),
        ts_ms: started,
        stream: TelemetryStream::Proxy,
        severity: sev,
        body: format!("{method} {route} {status_code} {elapsed_ms}ms"),
        attrs,
        error: err,
        fingerprint: fp,
        trace_id: Some(trace_id),
        span_id: None,
        request_id: Some(req_id),
        raw_line_offset: None,
        repeat: 1,
    }]);

    out_builder
        .body(Full::new(res_bytes))
        .unwrap_or_else(|_| Response::new(Full::new(Bytes::new())))
}

fn build_target(upstream: &str, path_and_query: &str) -> Result<url::Url, url::ParseError> {
    let base = url::Url::parse(upstream)?;
    let (path, query) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (path_and_query, None),
    };
    let mut target = base.clone();
    target.set_path(path);
    target.set_query(query);
    Ok(target)
}

fn mime_of(raw: &str) -> String {
    raw.split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

fn is_text_mime(mime: Option<&str>) -> bool {
    let Some(m) = mime else {
        return false;
    };
    m.starts_with("text/")
        || m.contains("json")
        || m.contains("xml")
        || m.contains("x-www-form-urlencoded")
}

fn capped_utf8(bytes: &[u8]) -> String {
    let slice = if bytes.len() > BODY_CAP {
        &bytes[..BODY_CAP]
    } else {
        bytes
    };
    String::from_utf8_lossy(slice).into_owned()
}

fn random_hex(num_bytes: usize) -> String {
    let mut buf = vec![0u8; num_bytes];
    let _ = getrandom::fill(&mut buf);
    buf.iter().map(|b| format!("{b:02x}")).collect()
}
