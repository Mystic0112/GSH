use std::sync::LazyLock;

use regex::Regex;
use sha1::{Digest, Sha1};

use crate::model::TelemetryError;

static UUID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}")
        .expect("valid uuid regex")
});
static ULID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[0-9A-HJKMNP-TV-Z]{26}\b").expect("valid ulid regex"));
static ABS_PATH_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:[A-Za-z]:\\|/)(?:[\w.@-]+[\\/])+[\w.@-]+").expect("valid abs_path regex")
});
static QUOTED_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#""[^"]*"|'[^']*'|`[^`]*`"#).expect("valid quoted regex"));
static NUMERIC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b\w*\d\w*\b").expect("valid numeric regex"));
static WS_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("valid ws regex"));

/// Mensagem com o dado variável mascarado por `*`.
pub fn normalize_message(message: &str) -> String {
    let s = UUID_RE.replace_all(message, "*");
    let s = ULID_RE.replace_all(&s, "*");
    let s = ABS_PATH_RE.replace_all(&s, "*");
    let s = QUOTED_RE.replace_all(&s, "*");
    let s = NUMERIC_RE.replace_all(&s, "*");
    let s = WS_RE.replace_all(&s, " ");
    s.trim().to_lowercase()
}

/// SHA-1 hex do fingerprint (`type\nnormalizedMessage\nprojectLocation`).
pub fn fingerprint_of(error: &TelemetryError) -> String {
    let loc = error
        .project_frame()
        .and_then(|f| f.location())
        .unwrap_or_default();
    let input = format!(
        "{}\n{}\n{}",
        error.err_type,
        normalize_message(&error.message),
        loc
    );
    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    digest.iter().map(|b| format!("{b:02x}")).collect()
}
