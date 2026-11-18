use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryProxyConfig {
    pub listen: u16,
    pub upstream: String,
}

#[derive(Debug, Clone, Default)]
pub struct TelemetryWorkspaceConfig {
    pub unwrap: Vec<FancyRegex>,
    pub ignore: Vec<FancyRegex>,
    pub project_frames: Vec<String>,
    pub proxy: Option<TelemetryProxyConfig>,
}

static BLOCK_COMMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/\*[\s\S]*?\*/").expect("valid block comment regex"));
static LINE_COMMENT_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| FancyRegex::new(r#"(?<!["\w:])//[^\n]*"#).expect("valid line comment regex"));

impl TelemetryWorkspaceConfig {
    pub fn path_for(workspace_path: &str) -> PathBuf {
        Path::new(workspace_path)
            .join(".gsh")
            .join("telemetry.json")
    }

    pub fn load(workspace_path: &str) -> Self {
        if workspace_path.is_empty() {
            return Self::default();
        }
        let path = Self::path_for(workspace_path);
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(_) => Self::default(),
        }
    }

    pub fn parse(text: &str) -> Self {
        let stripped = strip_comments(text);
        let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&stripped) else {
            return Self::default();
        };

        let unwrap = compile_regexes(map.get("unwrap"));
        let ignore = compile_regexes(map.get("ignore"));
        let project_frames = match map.get("projectFrames") {
            Some(Value::Array(arr)) => arr
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .collect(),
            _ => Vec::new(),
        };

        let proxy = parse_proxy(map.get("proxy"));

        Self {
            unwrap,
            ignore,
            project_frames,
            proxy,
        }
    }
}

fn compile_regexes(val: Option<&Value>) -> Vec<FancyRegex> {
    let Some(Value::Array(arr)) = val else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .filter_map(|s| FancyRegex::new(s).ok())
        .collect()
}

fn parse_proxy(val: Option<&Value>) -> Option<TelemetryProxyConfig> {
    let Value::Object(pm) = val? else {
        return None;
    };
    let listen_raw = pm.get("listen")?;
    let listen: u16 = match listen_raw {
        Value::Number(n) => u16::try_from(n.as_u64()?).ok()?,
        Value::String(s) => s.trim().parse::<u16>().ok()?,
        _ => return None,
    };
    if listen == 0 {
        return None;
    }
    let up_str = match pm.get("upstream")? {
        Value::String(s) => s.trim().to_string(),
        other => other.to_string(),
    };
    let parsed = url::Url::parse(&up_str).ok()?;
    if parsed.scheme().is_empty() {
        return None;
    }
    Some(TelemetryProxyConfig {
        listen,
        upstream: up_str,
    })
}

pub fn strip_comments(s: &str) -> String {
    let no_block = BLOCK_COMMENT_RE.replace_all(s, "");
    LINE_COMMENT_RE.replace_all(&no_block, "").into_owned()
}
