use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unwrapped {
    pub text: String,
    pub service: Option<String>,
}

enum BuiltinRule {
    Plain(Regex),
    Fancy(FancyRegex),
}

static BUILTINS: LazyLock<Vec<BuiltinRule>> = LazyLock::new(|| {
    vec![
        BuiltinRule::Plain(Regex::new(r"^flutter:\s?").expect("flutter prefix")),
        BuiltinRule::Plain(
            Regex::new(r"^[IWEDVF]/flutter\s*\(\s*\d+\):\s?").expect("logcat prefix"),
        ),
        BuiltinRule::Plain(Regex::new(r"^\[\d+\]\s").expect("concurrently prefix")),
        BuiltinRule::Plain(
            Regex::new(r"^[a-z][\w:@./-]*:(?:build|dev|start|test)?:\s").expect("turbo prefix"),
        ),
        BuiltinRule::Plain(
            Regex::new(
                r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(?:[.,]\d+)?(?:Z|[+-]\d{2}:?\d{2})?\s+",
            )
            .expect("iso ts prefix"),
        ),
        BuiltinRule::Plain(
            Regex::new(r"^\[\d{2}:\d{2}:\d{2}(?:[.,]\d+)?\]\s?").expect("bracket time prefix"),
        ),
        BuiltinRule::Fancy(
            FancyRegex::new(r"^\d{2}:\d{2}:\d{2}(?:[.,]\d+)?\s+(?=[\[{A-Za-z#])")
                .expect("bare time prefix"),
        ),
    ]
});

static COMPOSE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([a-zA-Z][\w.-]*)\s+\|\s?").expect("compose prefix"));

/// Remove prefixos conhecidos (e os `extra` do `.gsh/telemetry.json`),
/// repetidamente, porque eles se empilham (`api-1 | 2026-... {json}`).
pub fn unwrap_prefixes(line: &str, extra: &[FancyRegex]) -> Unwrapped {
    let mut text = line.to_string();
    let mut service: Option<String> = None;

    for _ in 0..4 {
        let mut changed = false;
        if let Some(c) = COMPOSE_RE.captures(&text) {
            if service.is_none() {
                service = c.get(1).map(|m| m.as_str().to_string());
            }
            if let Some(full) = c.get(0) {
                text = text[full.end()..].to_string();
                changed = true;
            }
        }

        for re in extra {
            if let Ok(Some(m)) = re.find(&text) {
                if m.start() == 0 && m.end() > 0 {
                    text = text[m.end()..].to_string();
                    changed = true;
                }
            }
        }

        for rule in &*BUILTINS {
            let matched_end = match rule {
                BuiltinRule::Plain(re) => re.find(&text).filter(|m| m.end() > 0).map(|m| m.end()),
                BuiltinRule::Fancy(re) => re
                    .find(&text)
                    .ok()
                    .flatten()
                    .filter(|m| m.end() > 0)
                    .map(|m| m.end()),
            };
            if let Some(end) = matched_end {
                text = text[end..].to_string();
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    Unwrapped { text, service }
}
