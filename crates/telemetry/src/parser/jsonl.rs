use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value};

use crate::fingerprint::fingerprint_of;
use crate::model::{
    parse_iso_ms, TelemetryError, TelemetryEvent, TelemetrySeverity, TelemetryStream,
};
use crate::parser::frames::parse_frame;

const LEVEL_KEYS: &[&str] = &["level", "severity", "lvl", "severityText", "loglevel"];
const MSG_KEYS: &[&str] = &["msg", "message", "event", "body", "text"];
const TIME_KEYS: &[&str] = &["time", "ts", "timestamp", "@timestamp", "datetime"];
const ERR_KEYS: &[&str] = &["err", "error", "exception", "exc_info"];
const SERVICE_KEYS: &[&str] = &["service", "name", "logger", "app"];
const TRACE_KEYS: &[&str] = &["trace_id", "traceId", "traceID", "trace"];
const SPAN_KEYS: &[&str] = &["span_id", "spanId", "spanID"];
const REQ_KEYS: &[&str] = &["reqId", "requestId", "request_id", "req_id", "x-request-id"];

static ERR_STRING_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)^([A-Z][\w.]*(?:Error|Exception|Warning)?):\s*(.+)$")
        .expect("err string regex")
});
static STACK_HEAD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Z][\w.]*(?:Error|Exception)):\s*(.+)$").expect("stack head regex")
});

fn value_as_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn take(map: &mut Map<String, Value>, keys: &[&str]) -> Option<Value> {
    for k in keys {
        if let Some(v) = map.remove(*k) {
            return Some(v);
        }
    }
    None
}

fn parse_time(v: Option<Value>) -> Option<i64> {
    let v = v?;
    match v {
        Value::Null => None,
        Value::Number(n) => {
            let num = n.as_i64().or_else(|| n.as_f64().map(|f| f as i64))?;
            Some(if num > 100_000_000_000 {
                num
            } else {
                num * 1000
            })
        }
        Value::String(s) => parse_iso_ms(&s),
        _ => None,
    }
}

/// Tenta interpretar `text` como um objeto JSON de log. `None` se não for.
#[allow(clippy::too_many_arguments)]
pub fn parse_json_line(
    text: &str,
    run_id: &str,
    at_ms: i64,
    stream: TelemetryStream,
    offset: Option<i64>,
    service: Option<&str>,
    project_roots: &[String],
    project_frames: &[String],
) -> Option<TelemetryEvent> {
    let t = text.trim();
    if t.len() < 2 || !t.starts_with('{') || !t.ends_with('}') {
        return None;
    }
    let Value::Object(mut map) = serde_json::from_str::<Value>(t).ok()? else {
        return None;
    };

    let severity = TelemetrySeverity::parse_value(take(&mut map, LEVEL_KEYS).as_ref());
    let msg = take(&mut map, MSG_KEYS)
        .filter(|v| !v.is_null())
        .map(|v| value_as_string(&v))
        .unwrap_or_default();
    let ts_ms = parse_time(take(&mut map, TIME_KEYS)).unwrap_or(at_ms);
    let err_raw = take(&mut map, ERR_KEYS).filter(|v| !v.is_null());
    let stack_top = map
        .remove("stack")
        .filter(|v| !v.is_null())
        .map(|v| value_as_string(&v));
    let svc = take(&mut map, SERVICE_KEYS)
        .filter(|v| !v.is_null())
        .map(|v| value_as_string(&v))
        .or_else(|| service.map(ToOwned::to_owned));
    let trace_id = take(&mut map, TRACE_KEYS)
        .filter(|v| !v.is_null())
        .map(|v| value_as_string(&v));
    let span_id = take(&mut map, SPAN_KEYS)
        .filter(|v| !v.is_null())
        .map(|v| value_as_string(&v));
    let request_id = take(&mut map, REQ_KEYS)
        .filter(|v| !v.is_null())
        .map(|v| value_as_string(&v));

    let error = if err_raw.is_some() || stack_top.is_some() {
        Some(build_error(
            err_raw.as_ref(),
            stack_top,
            &msg,
            project_roots,
            project_frames,
        ))
    } else {
        None
    };

    let mut attrs = Map::new();
    if let Some(s) = svc {
        attrs.insert("service".to_string(), Value::String(s));
    }
    for (k, v) in map {
        attrs.insert(k, v);
    }

    let sev = if error.is_some() && !severity.is_problem() {
        TelemetrySeverity::Error
    } else {
        severity
    };

    let body = if !msg.is_empty() {
        msg
    } else if let Some(ref e) = error {
        e.message.clone()
    } else {
        t.to_string()
    };

    let fingerprint = error.as_ref().map(fingerprint_of);

    Some(TelemetryEvent {
        id: None,
        run_id: run_id.to_string(),
        ts_ms,
        stream,
        severity: sev,
        body,
        attrs,
        error,
        fingerprint,
        trace_id,
        span_id,
        request_id,
        raw_line_offset: offset,
        repeat: 1,
    })
}

fn build_error(
    raw: Option<&Value>,
    stack_top: Option<String>,
    fallback_message: &str,
    project_roots: &[String],
    project_frames: &[String],
) -> TelemetryError {
    let mut err_type = "Error".to_string();
    let mut message = fallback_message.to_string();
    let mut stack = stack_top;

    match raw {
        Some(Value::Object(m)) => {
            if let Some(t) = m
                .get("type")
                .or_else(|| m.get("name"))
                .or_else(|| m.get("kind"))
                .or_else(|| m.get("class"))
                .filter(|v| !v.is_null())
            {
                err_type = value_as_string(t);
            }
            if let Some(msg) = m
                .get("message")
                .or_else(|| m.get("msg"))
                .filter(|v| !v.is_null())
            {
                message = value_as_string(msg);
            }
            if stack.is_none() {
                stack = m
                    .get("stack")
                    .or_else(|| m.get("stacktrace"))
                    .filter(|v| !v.is_null())
                    .map(value_as_string);
            }
        }
        Some(Value::String(s)) if !s.is_empty() => {
            if let Some(c) = ERR_STRING_RE.captures(s) {
                err_type = c.get(1).map_or("Error", |m| m.as_str()).to_string();
                message = c.get(2).map_or("", |m| m.as_str()).to_string();
            } else {
                message = s.clone();
            }
            if s.contains("\n    at ") || s.contains("\n#") {
                stack = Some(s.clone());
            }
        }
        Some(Value::Array(arr)) => {
            message = arr
                .iter()
                .map(value_as_string)
                .collect::<Vec<_>>()
                .join("\n");
        }
        _ => {}
    }

    let mut frames = Vec::new();
    if let Some(ref st) = stack {
        for line in st.lines() {
            if let Some(f) = parse_frame(line, project_roots, project_frames) {
                frames.push(f);
            }
        }
        let first_line = st.lines().next().unwrap_or("");
        if let Some(head) = STACK_HEAD_RE.captures(first_line) {
            if err_type == "Error" {
                if let Some(t) = head.get(1) {
                    err_type = t.as_str().to_string();
                }
                if message.is_empty() || message == fallback_message {
                    if let Some(m) = head.get(2) {
                        message = m.as_str().to_string();
                    }
                }
            }
        }
    }

    TelemetryError {
        err_type,
        message,
        stack,
        frames,
    }
}
