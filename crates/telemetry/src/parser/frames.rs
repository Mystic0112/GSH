use std::sync::LazyLock;

use regex::Regex;

use crate::model::TelemetryFrame;

static DART_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^#\d+\s+(.+?)\s+\((.+?)(?::(\d+))?(?::(\d+))?\)\s*$").expect("dart frame")
});
static DART_TEST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*((?:package:|dart:|file:|/|[A-Za-z]:\\)?\S+\.dart)\s+(\d+)(?::(\d+))?\s+(.+)$")
        .expect("dart test frame")
});
static NODE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s+at\s+(?:(.+?)\s+\()?((?:file://|node:|[A-Za-z]:\\|/|\.{0,2}/)?[^():]+?):(\d+)(?::(\d+))?\)?\s*\{?\s*$",
    )
    .expect("node frame")
});
static PYTHON_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s+File "(.+?)", line (\d+)(?:, in (.+))?$"#).expect("python frame")
});
static RUST_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+at\s+(\S+\.rs):(\d+)(?::(\d+))?$").expect("rust frame"));
static GO_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+(\S+\.go):(\d+)(?:\s+\+0x[0-9a-f]+)?$").expect("go frame"));

static STACK_MORE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+\.\.\.\s*\d*\s*(more|frames|elided)").expect("stack more"));
static STACK_ANON_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+at\s+<anonymous>").expect("stack anon"));
static WIN_DRIVE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z]:\\").expect("win drive"));

const FRAMEWORK_PKGS: &[&str] = &[
    "flutter",
    "flutter_test",
    "flutter_web_plugins",
    "test",
    "test_api",
    "test_core",
    "matcher",
    "stack_trace",
    "async",
    "stream_channel",
    "collection",
    "meta",
    "vector_math",
    "characters",
    "sky_engine",
];

fn parse_u32(s: Option<regex::Match<'_>>) -> Option<u32> {
    s.and_then(|m| m.as_str().parse::<u32>().ok())
}

/// Interpreta uma linha como frame; `None` se não for.
pub fn parse_frame(
    line: &str,
    project_roots: &[String],
    project_frames: &[String],
) -> Option<TelemetryFrame> {
    let (file, func, ln, col) = if let Some(m) = DART_RE.captures(line) {
        (
            m.get(2).map(|x| x.as_str()),
            m.get(1).map(|x| x.as_str()),
            parse_u32(m.get(3)),
            parse_u32(m.get(4)),
        )
    } else if let Some(m) = NODE_RE.captures(line) {
        (
            m.get(2).map(|x| x.as_str()),
            m.get(1).map(|x| x.as_str()),
            parse_u32(m.get(3)),
            parse_u32(m.get(4)),
        )
    } else if let Some(m) = PYTHON_RE.captures(line) {
        (
            m.get(1).map(|x| x.as_str()),
            m.get(3).map(|x| x.as_str()),
            parse_u32(m.get(2)),
            None,
        )
    } else if let Some(m) = DART_TEST_RE.captures(line) {
        (
            m.get(1).map(|x| x.as_str()),
            m.get(4).map(|x| x.as_str()),
            parse_u32(m.get(2)),
            parse_u32(m.get(3)),
        )
    } else if let Some(m) = RUST_RE.captures(line) {
        (
            m.get(1).map(|x| x.as_str()),
            None,
            parse_u32(m.get(2)),
            parse_u32(m.get(3)),
        )
    } else {
        let m = GO_RE.captures(line)?;
        (
            m.get(1).map(|x| x.as_str()),
            None,
            parse_u32(m.get(2)),
            None,
        )
    };

    let file = file.filter(|f| !f.is_empty())?;
    let in_project = is_project_file(file, project_roots, project_frames);
    let normalized_file = if in_project {
        normalize_project_path(file, project_roots)
    } else {
        file.to_string()
    };

    Some(TelemetryFrame {
        raw: line.trim_end().to_string(),
        file: Some(normalized_file),
        line: ln,
        column: col,
        function: func.map(|s| s.trim().to_string()),
        in_project,
    })
}

/// `true` se a linha parece continuação de stack (frame ou marcador).
pub fn looks_like_stack_line(line: &str) -> bool {
    parse_frame(line, &[], &[]).is_some()
        || line.trim() == "<asynchronous suspension>"
        || STACK_MORE_RE.is_match(line)
        || STACK_ANON_RE.is_match(line)
}

fn strip_file_scheme(file: &str) -> String {
    if let Some(rest) = file.strip_prefix("file://") {
        if let Ok(u) = url::Url::parse(file) {
            if let Ok(p) = u.to_file_path() {
                return p.to_string_lossy().into_owned();
            }
        }
        return rest.to_string();
    }
    file.to_string()
}

pub fn normalize_path_str(raw: &str) -> String {
    let win = WIN_DRIVE_RE.is_match(raw);
    let sep = if win { '\\' } else { '/' };
    let parts_raw: Vec<&str> = raw.split(['/', '\\']).collect();
    let mut stack: Vec<&str> = Vec::new();
    let is_abs = raw.starts_with('/') || win;
    let drive_prefix = if win {
        parts_raw.first().copied()
    } else {
        None
    };
    let start_idx = usize::from(win);

    for seg in parts_raw.iter().skip(start_idx) {
        match *seg {
            "" | "." => {}
            ".." => {
                if let Some(last) = stack.last() {
                    if *last != ".." {
                        stack.pop();
                    } else if !is_abs {
                        stack.push("..");
                    }
                } else if !is_abs {
                    stack.push("..");
                }
            }
            other => stack.push(other),
        }
    }

    if let Some(drive) = drive_prefix {
        if stack.is_empty() {
            format!("{drive}\\")
        } else {
            format!("{drive}\\{}", stack.join("\\"))
        }
    } else if is_abs {
        if stack.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", stack.join(&sep.to_string()))
        }
    } else if stack.is_empty() {
        ".".to_string()
    } else {
        stack.join("/")
    }
}

pub fn is_within_path(parent: &str, child: &str) -> bool {
    let np = normalize_path_str(parent);
    let nc = normalize_path_str(child);
    if np == nc {
        return false;
    }
    if np == "/" {
        return nc.starts_with('/');
    }
    let sep = if WIN_DRIVE_RE.is_match(&np) {
        '\\'
    } else {
        '/'
    };
    let prefix = if np.ends_with(sep) {
        np
    } else {
        format!("{np}{sep}")
    };
    nc.starts_with(&prefix)
}

pub fn relative_path(target: &str, from: &str) -> String {
    let nt = normalize_path_str(target);
    let nf = normalize_path_str(from);
    if nt == nf {
        return ".".to_string();
    }
    let sep = if WIN_DRIVE_RE.is_match(&nf) {
        '\\'
    } else {
        '/'
    };
    let prefix = if nf == "/" {
        "/".to_string()
    } else if nf.ends_with(sep) {
        nf
    } else {
        format!("{nf}{sep}")
    };
    nt.strip_prefix(&prefix).unwrap_or(&nt).to_string()
}

/// Decide se um caminho de frame pertence ao código do workspace.
pub fn is_project_file(file: &str, project_roots: &[String], project_frames: &[String]) -> bool {
    let f = strip_file_scheme(file);
    if f.starts_with("dart:") || f.starts_with("node:") {
        return false;
    }
    if f.contains("node_modules")
        || f.contains(".pub-cache")
        || f.contains("/flutter/packages/")
        || f.contains("site-packages")
        || f.contains("/.cargo/registry/")
        || f.contains("/rustlib/")
        || f.starts_with("/usr/")
        || f.starts_with('<')
    {
        return false;
    }
    if let Some(rest) = f.strip_prefix("package:") {
        let name = rest.split('/').next().unwrap_or("");
        return !FRAMEWORK_PKGS.contains(&name);
    }
    if f.starts_with('/') || WIN_DRIVE_RE.is_match(&f) {
        if project_roots.is_empty() {
            return false;
        }
        return project_roots.iter().any(|r| is_within_path(r, &f));
    }
    let rel = f.strip_prefix("./").unwrap_or(&f);
    if project_frames.iter().any(|prefix| rel.starts_with(prefix)) {
        return true;
    }
    !rel.starts_with("..")
}

/// Caminho curto pra UI/fingerprint: `package:app/x.dart` → `lib/x.dart`,
/// absoluto sob uma root → relativo à root, `./src/x` → `src/x`.
pub fn normalize_project_path(file: &str, project_roots: &[String]) -> String {
    let f = strip_file_scheme(file);
    if let Some(rest) = f.strip_prefix("package:") {
        return match rest.find('/') {
            None => rest.to_string(),
            Some(slash) => format!("lib/{}", &rest[slash + 1..]),
        };
    }
    if f.starts_with('/') || WIN_DRIVE_RE.is_match(&f) {
        let nf = normalize_path_str(&f);
        for r in project_roots {
            if is_within_path(r, &nf) {
                return relative_path(&nf, r);
            }
        }
        return f;
    }
    f.strip_prefix("./").unwrap_or(&f).to_string()
}
