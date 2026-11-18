use std::sync::Arc;

use serde_json::{Map, Value};
use tokio::sync::Mutex;

use crate::ingest::{TelemetryIngest, TelemetryIngestSession};
use crate::model::{now_ms, TelemetryError, TelemetryEvent, TelemetrySeverity, TelemetryStream};

/// Plano 68: o próprio GSH como fonte da Telemetria. Um run por boot
/// (`source: app`) no store `__gsh__`, consultado com `gsh telemetry --app`.
pub struct AppTelemetryBridge {
    session: Mutex<Option<Arc<TelemetryIngestSession>>>,
}

impl AppTelemetryBridge {
    pub fn new() -> Self {
        Self {
            session: Mutex::new(None),
        }
    }

    pub async fn start(&self, ingest: &Arc<TelemetryIngest>, version: &str) {
        let mut guard = self.session.lock().await;
        if guard.is_some() {
            return;
        }
        if let Ok(s) = ingest.open_app_run(version).await {
            s.add_events(vec![Self::build_event(
                s.run_id(),
                TelemetrySeverity::Info,
                "boot",
                &format!("GSH {version} started"),
                now_ms(),
                None,
            )]);
            *guard = Some(s);
        }
    }

    pub async fn close(&self) {
        let session = {
            let mut guard = self.session.lock().await;
            guard.take()
        };
        if let Some(s) = session {
            let _ = tokio::time::timeout(std::time::Duration::from_secs(2), s.close(Some(0))).await;
        }
    }

    pub async fn log(&self, severity: TelemetrySeverity, tag: &str, message: &str) {
        let guard = self.session.lock().await;
        let Some(ref s) = *guard else {
            return;
        };
        s.add_events(vec![Self::build_event(
            s.run_id(),
            severity,
            tag,
            message,
            now_ms(),
            None,
        )]);
    }

    pub async fn metric(&self, metric: &str, fields: Map<String, Value>) {
        let guard = self.session.lock().await;
        let Some(ref s) = *guard else {
            return;
        };
        let mut attrs = Map::new();
        attrs.insert("metric".to_string(), Value::String(metric.to_string()));
        for (k, v) in fields {
            attrs.insert(k, v);
        }
        s.add_events(vec![TelemetryEvent {
            id: None,
            run_id: s.run_id().to_string(),
            ts_ms: now_ms(),
            stream: TelemetryStream::Otlp,
            severity: TelemetrySeverity::Info,
            body: format!("perf {metric}"),
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

    fn build_event(
        run_id: &str,
        severity: TelemetrySeverity,
        tag: &str,
        message: &str,
        at_ms: i64,
        err_type: Option<&str>,
    ) -> TelemetryEvent {
        let err = if severity.is_problem() {
            Some(TelemetryError {
                err_type: err_type
                    .unwrap_or(if severity == TelemetrySeverity::Warn {
                        "Warning"
                    } else {
                        "Error"
                    })
                    .to_string(),
                message: message.to_string(),
                stack: None,
                frames: Vec::new(),
            })
        } else {
            None
        };
        let mut attrs = Map::new();
        attrs.insert("tag".to_string(), Value::String(tag.to_string()));
        TelemetryEvent {
            id: None,
            run_id: run_id.to_string(),
            ts_ms: at_ms,
            stream: TelemetryStream::Err,
            severity,
            body: format!("[{tag}] {message}"),
            attrs,
            error: err,
            fingerprint: None,
            trace_id: None,
            span_id: None,
            request_id: None,
            raw_line_offset: None,
            repeat: 1,
        }
    }
}

impl Default for AppTelemetryBridge {
    fn default() -> Self {
        Self::new()
    }
}
