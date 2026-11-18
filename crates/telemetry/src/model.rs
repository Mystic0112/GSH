use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Id do "workspace" que guarda os runs do próprio app (plano 68).
pub const K_TELEMETRY_APP_WORKSPACE_ID: &str = "__gsh__";

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

pub fn format_iso_utc(ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ms)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(|| "1970-01-01T00:00:00.000Z".to_string())
}

pub fn parse_iso_ms(raw: &str) -> Option<i64> {
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(raw.trim()) {
        return Some(dt.timestamp_millis());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(naive.and_utc().timestamp_millis());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%d %H:%M:%S%.f") {
        return Some(naive.and_utc().timestamp_millis());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%dT%H:%M:%S") {
        return Some(naive.and_utc().timestamp_millis());
    }
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(raw.trim(), "%Y-%m-%d %H:%M:%S") {
        return Some(naive.and_utc().timestamp_millis());
    }
    if let Ok(date) = chrono::NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d") {
        return date
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc().timestamp_millis());
    }
    None
}

/// Severidade normalizada. A ordem importa (comparações `>=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TelemetrySeverity {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

impl TelemetrySeverity {
    pub const fn index(self) -> u8 {
        self as u8
    }

    pub const fn from_index(idx: i64) -> Self {
        match idx {
            0 => Self::Trace,
            1 => Self::Debug,
            2 => Self::Info,
            3 => Self::Warn,
            4 => Self::Error,
            5 => Self::Fatal,
            _ => Self::Info,
        }
    }

    pub const fn is_problem(self) -> bool {
        self.index() >= Self::Warn.index()
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
            Self::Fatal => "fatal",
        }
    }

    pub fn parse_num(n: f64) -> Self {
        if n >= 60.0 {
            Self::Fatal
        } else if n >= 50.0 {
            Self::Error
        } else if n >= 40.0 {
            Self::Warn
        } else if n >= 30.0 {
            Self::Info
        } else if n >= 20.0 {
            Self::Debug
        } else {
            Self::Trace
        }
    }

    pub fn parse_str(raw: &str) -> Self {
        let s = raw.trim().to_ascii_lowercase();
        if let Ok(n) = s.parse::<i64>() {
            return Self::parse_num(n as f64);
        }
        match s.as_str() {
            "trace" | "verbose" | "finest" | "finer" => Self::Trace,
            "debug" | "dbg" | "fine" | "config" => Self::Debug,
            "info" | "information" | "notice" | "log" => Self::Info,
            "warn" | "warning" | "wrn" => Self::Warn,
            "error" | "err" | "severe" | "e" => Self::Error,
            "fatal" | "critical" | "crit" | "panic" | "shout" => Self::Fatal,
            _ => Self::Info,
        }
    }

    pub fn parse_value(raw: Option<&Value>) -> Self {
        let Some(v) = raw else {
            return Self::Info;
        };
        match v {
            Value::Null => Self::Info,
            Value::Number(n) => n.as_f64().map_or(Self::Info, Self::parse_num),
            Value::String(s) => Self::parse_str(s),
            other => Self::parse_str(&other.to_string()),
        }
    }
}

/// De qual torneira o evento veio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryStream {
    Out,
    Err,
    Vm,
    Otlp,
    Proxy,
}

impl TelemetryStream {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Out => "out",
            Self::Err => "err",
            Self::Vm => "vm",
            Self::Otlp => "otlp",
            Self::Proxy => "proxy",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "out" => Some(Self::Out),
            "err" => Some(Self::Err),
            "vm" => Some(Self::Vm),
            "otlp" => Some(Self::Otlp),
            "proxy" => Some(Self::Proxy),
            _ => None,
        }
    }
}

/// Um frame de stack já interpretado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryFrame {
    pub raw: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    #[serde(default, rename = "inProject")]
    pub in_project: bool,
}

impl TelemetryFrame {
    /// `arquivo:linha` curto pra UI e fingerprint (`None` sem arquivo).
    pub fn location(&self) -> Option<String> {
        let file = self.file.as_deref()?;
        Some(match self.line {
            Some(line) => format!("{file}:{line}"),
            None => file.to_string(),
        })
    }
}

/// Erro estruturado anexado ao evento.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryError {
    #[serde(rename = "type", default = "default_error_type")]
    pub err_type: String,
    #[serde(default)]
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stack: Option<String>,
    #[serde(default)]
    pub frames: Vec<TelemetryFrame>,
}

fn default_error_type() -> String {
    "Error".to_string()
}

impl TelemetryError {
    pub fn project_frame(&self) -> Option<&TelemetryFrame> {
        self.frames.iter().find(|f| f.in_project)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TelemetryEvent {
    pub id: Option<String>,
    pub run_id: String,
    pub ts_ms: i64,
    pub stream: TelemetryStream,
    pub severity: TelemetrySeverity,
    pub body: String,
    pub attrs: Map<String, Value>,
    pub error: Option<TelemetryError>,
    pub fingerprint: Option<String>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub request_id: Option<String>,
    pub raw_line_offset: Option<i64>,
    pub repeat: i64,
}

impl TelemetryEvent {
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    pub fn probe(&self) -> Option<String> {
        self.attrs.get("probe").and_then(|v| match v {
            Value::Null => None,
            Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TelemetryRunSource {
    Task,
    Wrapper,
    App,
}

impl TelemetryRunSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Wrapper => "wrapper",
            Self::App => "app",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "task" => Some(Self::Task),
            "wrapper" => Some(Self::Wrapper),
            "app" => Some(Self::App),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryRun {
    pub id: String,
    pub key: String,
    pub project: String,
    pub source: TelemetryRunSource,
    pub cwd: String,
    pub command: String,
    pub name: Option<String>,
    pub task_key: Option<String>,
    pub pane_id: Option<String>,
    pub pid: Option<i64>,
    pub started_at_ms: i64,
    pub ended_at_ms: Option<i64>,
    pub exit_code: Option<i32>,
}

impl TelemetryRun {
    pub fn is_live(&self) -> bool {
        self.ended_at_ms.is_none()
    }

    pub fn key_for(cwd: &str, command: &str, name: Option<&str>) -> String {
        if let Some(n) = name.map(str::trim).filter(|n| !n.is_empty()) {
            return format!("name:{n}");
        }
        let cmd = command.split_whitespace().collect::<Vec<_>>().join(" ");
        format!("cmd:{cwd}::{cmd}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TelemetryTriageStatus {
    Open,
    Resolved,
    Ignored,
}

impl TelemetryTriageStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Resolved => "resolved",
            Self::Ignored => "ignored",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "open" => Some(Self::Open),
            "resolved" => Some(Self::Resolved),
            "ignored" => Some(Self::Ignored),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TelemetryTriageActor {
    Human,
    Agent,
}

impl TelemetryTriageActor {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Human => "human",
            Self::Agent => "agent",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "human" => Some(Self::Human),
            "agent" => Some(Self::Agent),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryTriage {
    pub fingerprint: String,
    pub status: TelemetryTriageStatus,
    pub by: TelemetryTriageActor,
    pub at_ms: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryCaseRun {
    pub run_id: String,
    pub count: i64,
    pub first_at_ms: i64,
    pub last_at_ms: i64,
    pub live: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryCase {
    pub fingerprint: String,
    pub run_key: String,
    pub project: String,
    pub severity: TelemetrySeverity,
    pub err_type: String,
    pub message: String,
    pub location: Option<String>,
    pub count: i64,
    pub first_at_ms: i64,
    pub last_at_ms: i64,
    pub status: TelemetryTriageStatus,
    pub triage: Option<TelemetryTriage>,
    pub is_new: bool,
    pub is_regression: bool,
    pub runs: Vec<TelemetryCaseRun>,
    pub last_event_id: Option<String>,
}

impl TelemetryCase {
    pub fn short_id(&self) -> String {
        let prefix: String = self.fingerprint.chars().take(4).collect();
        format!("e_{prefix}")
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryQuery {
    pub project: Option<String>,
    pub run_id: Option<String>,
    pub run_key: Option<String>,
    pub task_key: Option<String>,
    pub pane_id: Option<String>,
    pub min_severity: Option<TelemetrySeverity>,
    pub since_ms: Option<i64>,
    pub until_ms: Option<i64>,
    pub text: Option<String>,
    pub probe: Option<String>,
    pub only_new: bool,
    pub include_ignored: bool,
    pub include_resolved: bool,
    pub only_errors: bool,
    pub limit: usize,
}

impl Default for TelemetryQuery {
    fn default() -> Self {
        Self {
            project: None,
            run_id: None,
            run_key: None,
            task_key: None,
            pane_id: None,
            min_severity: None,
            since_ms: None,
            until_ms: None,
            text: None,
            probe: None,
            only_new: false,
            include_ignored: false,
            include_resolved: false,
            only_errors: false,
            limit: 100,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TelemetryRetention {
    pub max_bytes: u64,
    pub max_age_ms: i64,
}

impl Default for TelemetryRetention {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024,
            max_age_ms: 7 * 24 * 60 * 60 * 1000,
        }
    }
}

#[derive(Debug, Error)]
pub enum TelemetryStoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("corrupt telemetry data: {0}")]
    Corrupt(String),
    #[error("telemetry store closed")]
    Closed,
}

#[derive(Debug, Clone)]
pub struct TelemetryWorkspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub roots: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TelemetryTaskDef {
    pub id: String,
    pub label: String,
    pub cwd: String,
    pub telemetry_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct TelemetryErrorNotice {
    pub workspace_id: String,
    pub run_id: String,
    pub project: String,
    pub pane_id: Option<String>,
    pub fingerprints: Vec<String>,
}
