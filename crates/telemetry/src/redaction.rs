use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map, Value};

pub const REDACTED: &str = "[redacted]";

static SECRET_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(authorization|proxy-authorization|cookie|set-cookie|x-api-key|api[-_]?key|password|passwd|pwd|secret|client[-_]?secret|token|access[-_]?token|refresh[-_]?token|id[-_]?token|private[-_]?key|session[-_]?id)$",
    )
    .expect("valid secret key regex")
});

static JWT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"eyJ[\w-]{8,}\.[\w-]{8,}\.[\w-]{8,}").expect("jwt regex"));
static BEARER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b((?:Bearer|Basic)\s)[A-Za-z0-9+/=._-]{12,}").expect("bearer regex")
});
static PREFIX_KEY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:sk|pk|rk|ghp|gho|xox[abp]|AKIA)[-_][A-Za-z0-9_-]{12,}")
        .expect("prefix key regex")
});
static AKIA_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bAKIA[0-9A-Z]{16}\b").expect("akia regex"));
static QUERY_SECRET_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)((?:password|passwd|token|secret|api[-_]?key)=)[^&\s"]+"#)
        .expect("query secret regex")
});

pub fn is_secret_key(key: &str) -> bool {
    SECRET_KEY_RE.is_match(key.trim())
}

/// Mascara padrões de segredo dentro de texto livre.
pub fn redact_text(text: &str) -> String {
    let out = JWT_RE.replace_all(text, REDACTED);
    let out = BEARER_RE.replace_all(&out, "${1}[redacted]");
    let out = PREFIX_KEY_RE.replace_all(&out, REDACTED);
    let out = AKIA_RE.replace_all(&out, REDACTED);
    QUERY_SECRET_RE
        .replace_all(&out, "${1}[redacted]")
        .into_owned()
}

/// Mascara valores de chaves sensíveis (recursivo) e padrões nos textos.
pub fn redact_attrs(attrs: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::with_capacity(attrs.len());
    for (k, v) in attrs {
        if is_secret_key(k) {
            out.insert(k.clone(), Value::String(REDACTED.to_string()));
        } else {
            out.insert(k.clone(), redact_value(v));
        }
    }
    out
}

fn redact_value(v: &Value) -> Value {
    match v {
        Value::String(s) => Value::String(redact_text(s)),
        Value::Object(m) => Value::Object(redact_attrs(m)),
        Value::Array(arr) => Value::Array(arr.iter().map(redact_value).collect()),
        other => other.clone(),
    }
}
