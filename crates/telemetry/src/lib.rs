pub mod app_bridge;
pub mod cli_handler;
pub mod config;
pub mod db;
pub mod fingerprint;
pub mod ingest;
pub mod model;
pub mod otlp;
pub mod parser;
pub mod proxy;
pub mod redaction;
pub mod registry;
pub mod store;
pub mod vm_service;

pub use app_bridge::AppTelemetryBridge;
pub use cli_handler::{TelemetryCliHandler, K_TELEMETRY_REPLY_CAP};
pub use config::{strip_comments, TelemetryProxyConfig, TelemetryWorkspaceConfig};
pub use db::{b36, event_id, event_rowid, fts_query, TelemetryDb};
pub use fingerprint::{fingerprint_of, normalize_message};
pub use ingest::{TelemetryIngest, TelemetryIngestSession};
pub use model::*;
pub use otlp::{OtlpReceiver, OtlpSink};
pub use parser::{
    is_project_file, looks_like_stack_line, normalize_project_path, parse_frame, parse_json_line,
    scan_alt_screen, strip_ansi, unwrap_prefixes, AltScreenToggle, TelemetryLineParser,
    TelemetryParserConfig,
};
pub use proxy::HttpTelemetryProxy;
pub use redaction::{is_secret_key, redact_attrs, redact_text, REDACTED};
pub use registry::TelemetryStoreRegistry;
pub use store::TelemetryStore;
pub use vm_service::{VmServiceAttachment, VM_SERVICE_URI_PATTERN};
