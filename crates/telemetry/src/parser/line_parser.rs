use std::sync::LazyLock;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use serde_json::{Map, Value};

use crate::fingerprint::{fingerprint_of, normalize_message};
use crate::model::{
    TelemetryError, TelemetryEvent, TelemetryFrame, TelemetrySeverity, TelemetryStream,
};
use crate::parser::ansi::{scan_alt_screen, strip_ansi, AltScreenToggle};
use crate::parser::frames::{
    is_project_file, looks_like_stack_line, normalize_project_path, parse_frame,
};
use crate::parser::jsonl::parse_json_line;
use crate::parser::prefix_unwrap::unwrap_prefixes;

#[derive(Debug, Clone, Default)]
pub struct TelemetryParserConfig {
    pub unwrap: Vec<FancyRegex>,
    pub project_roots: Vec<String>,
    pub project_frames: Vec<String>,
}

static FLUTTER_START_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[═=]+╡\s*EXCEPTION CAUGHT BY (.+?)\s*╞[═=]*").expect("flutter start")
});
static FLUTTER_END_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[═=]{8,}\s*$").expect("flutter end"));
static FLUTTER_THROWN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^The following (\S+) was thrown(.*?):?\s*$").expect("flutter thrown")
});
static FLUTTER_STACK_HEAD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^When the exception was thrown, this was the stack").expect("flutter stack head")
});
static ANOTHER_EXCEPTION_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Another exception was thrown:\s*(.+)$").expect("another exception")
});
static ERROR_HEAD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?:Unhandled (?:exception|Exception):\s*)?([A-Z][\w.]*(?:Error|Exception|Warning|Failure|Fault))(?:\s*\(([^)]*)\))?:\s*(.+)$",
    )
    .expect("error head")
});
static UNHANDLED_ONLY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Unhandled (?:exception|Exception):\s*$").expect("unhandled only")
});
static PLAIN_ERROR_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(Error|Exception|error|Fatal error):\s*(.+)$").expect("plain error")
});
static PYTHON_START_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^Traceback \(most recent call last\):\s*$").expect("python start")
});
static PYTHON_TAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z_][\w.]*)(?::\s*(.*))?$").expect("python tail"));
static RUST_PANIC_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^thread '(.+?)' panicked at (\S+?):(\d+):(\d+):?\s*(.*)$").expect("rust panic")
});
static GO_PANIC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^panic:\s*(.+)$").expect("go panic"));
static DART_TEST_FAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d{2}:\d{2} \+\d+(?: ~\d+)?(?: -\d+)?: (.+?) \[E\]\s*$").expect("dart test fail")
});
static JEST_FAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*● (.+)$").expect("jest fail"));
static PYTEST_FAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^FAILED (\S+)(?: - (.+))?$").expect("pytest fail"));
static TYPE_FROM_MSG_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^([A-Z][\w.]*(?:Error|Exception|Warning))(?:\s*\([^)]*\))?:\s*(.*)$")
        .expect("type from msg")
});

static RUST_FRAME_NUM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s+\d+:\s").expect("rust frame num"));

static SEV_FATAL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(fatal|panic(?:ked)?)\b").expect("sev fatal"));
static SEV_ERROR_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    FancyRegex::new(
        r"(?i)(?<![\w-])(error|err|exception|failed|failure|unhandled|traceback|crash(?:ed)?|econnrefused|enoent)(?![\w-])",
    )
    .expect("sev error")
});
static SEV_WARN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(warn|warning|deprecat\w*)\b").expect("sev warn"));
static SEV_DEBUG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^\s*(?:\[?debug\]?|dbg)\b").expect("sev debug"));
static ZERO_COUNT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(0|no|zero)\s+(errors?|warnings?|failures?)\b").expect("zero count")
});

#[derive(Debug, Clone)]
struct Ctx {
    stream: TelemetryStream,
    at_ms: i64,
    offset: Option<i64>,
    service: Option<String>,
}

const MAX_BLOCK_LINES: usize = 400;

struct BlockState {
    ctx: Ctx,
    done: bool,
    lines: usize,
    kind: BlockKind,
}

enum BlockKind {
    Error {
        err_type: Option<String>,
        message: String,
        frame_lines: Vec<String>,
        detail: Vec<String>,
        extra_msg: usize,
    },
    Flutter {
        library: String,
        err_type: Option<String>,
        msg: Vec<String>,
        frame_lines: Vec<String>,
        in_stack: bool,
        in_msg: bool,
    },
    Python {
        frame_lines: Vec<String>,
        err_type: Option<String>,
        message: Option<String>,
    },
    Rust {
        file: String,
        line: u32,
        column: u32,
        inline_message: String,
        msg: Vec<String>,
        frame_lines: Vec<String>,
    },
    Go {
        message: String,
        frame_lines: Vec<String>,
        started: bool,
    },
    Test {
        name: String,
        detail: Option<String>,
        frame_lines: Vec<String>,
    },
}

impl BlockState {
    fn consume(&mut self, text: &str) -> bool {
        let cur = self.lines;
        self.lines += 1;
        if cur > MAX_BLOCK_LINES {
            return false;
        }
        match &mut self.kind {
            BlockKind::Error {
                err_type,
                message,
                frame_lines,
                detail,
                extra_msg,
            } => {
                if err_type.is_none() {
                    if let Some(c) = ERROR_HEAD_RE.captures(text) {
                        *err_type = c.get(1).map(|m| m.as_str().to_string());
                        *message = c
                            .get(3)
                            .or_else(|| c.get(2))
                            .map_or("", |m| m.as_str())
                            .to_string();
                        return true;
                    }
                    if let Some(c) = TYPE_FROM_MSG_RE.captures(text) {
                        *err_type = c.get(1).map(|m| m.as_str().to_string());
                        *message = c.get(2).map_or("", |m| m.as_str()).to_string();
                        return true;
                    }
                    if text.trim().is_empty() {
                        return true;
                    }
                    *err_type = Some("Error".to_string());
                    *message = text.trim().to_string();
                    return true;
                }
                if looks_like_stack_line(text) {
                    frame_lines.push(text.to_string());
                    return true;
                }
                if text.trim().is_empty() {
                    if frame_lines.is_empty() && *extra_msg < 3 {
                        *extra_msg += 1;
                        return *extra_msg > 0;
                    }
                    return false;
                }
                if frame_lines.is_empty()
                    && *extra_msg < 6
                    && (text.starts_with(' ')
                        || text.starts_with('\t')
                        || text.starts_with('{')
                        || text.starts_with('['))
                {
                    *message = format!("{message}\n{}", text.trim_end());
                    *extra_msg += 1;
                    return true;
                }
                if !frame_lines.is_empty()
                    && (text.starts_with(' ') || text.starts_with('\t') || text == "}")
                {
                    detail.push(text.trim().to_string());
                    return true;
                }
                false
            }
            BlockKind::Flutter {
                err_type,
                msg,
                frame_lines,
                in_stack,
                in_msg,
                ..
            } => {
                if FLUTTER_END_RE.is_match(text) {
                    self.done = true;
                    return true;
                }
                if let Some(t) = FLUTTER_THROWN_RE.captures(text) {
                    *err_type = t.get(1).map(|m| m.as_str().to_string());
                    *in_msg = true;
                    return true;
                }
                if FLUTTER_STACK_HEAD_RE.is_match(text) {
                    *in_stack = true;
                    *in_msg = false;
                    return true;
                }
                if *in_stack {
                    if looks_like_stack_line(text) {
                        frame_lines.push(text.to_string());
                    }
                    if text.trim().is_empty() && !frame_lines.is_empty() {
                        *in_stack = false;
                    }
                    return true;
                }
                if *in_msg {
                    if text.trim().is_empty() {
                        *in_msg = msg.is_empty();
                    } else {
                        msg.push(text.trim().to_string());
                    }
                }
                true
            }
            BlockKind::Python {
                frame_lines,
                err_type,
                message,
            } => {
                if text.starts_with(' ') || text.starts_with('\t') {
                    if parse_frame(text, &[], &[]).is_some() {
                        frame_lines.push(text.to_string());
                    }
                    return true;
                }
                if text.trim().is_empty() {
                    return true;
                }
                if let Some(m) = PYTHON_TAIL_RE.captures(text) {
                    *err_type = m
                        .get(1)
                        .and_then(|g| g.as_str().split('.').next_back())
                        .map(ToOwned::to_owned);
                    *message = Some(m.get(2).map_or("", |g| g.as_str()).to_string());
                    self.done = true;
                    return true;
                }
                false
            }
            BlockKind::Rust {
                inline_message,
                msg,
                frame_lines,
                ..
            } => {
                if text.starts_with("note:") || text.starts_with("stack backtrace:") {
                    return true;
                }
                if RUST_FRAME_NUM_RE.is_match(text) {
                    return true;
                }
                if parse_frame(text, &[], &[]).is_some() {
                    frame_lines.push(text.to_string());
                    return true;
                }
                if text.trim().is_empty() {
                    return msg.is_empty() && inline_message.is_empty();
                }
                if msg.is_empty() && inline_message.is_empty() && !text.starts_with(' ') {
                    msg.push(text.trim().to_string());
                    return true;
                }
                false
            }
            BlockKind::Go {
                frame_lines,
                started,
                ..
            } => {
                if text.trim().is_empty() {
                    return !*started;
                }
                if text.starts_with("goroutine ") {
                    *started = true;
                    return true;
                }
                if parse_frame(text, &[], &[]).is_some() {
                    frame_lines.push(text.to_string());
                    return true;
                }
                *started && !text.starts_with(' ') && text.contains('(')
            }
            BlockKind::Test {
                detail,
                frame_lines,
                ..
            } => {
                if text.trim().is_empty() {
                    return true;
                }
                if !(text.starts_with(' ') || text.starts_with('\t')) {
                    return false;
                }
                if parse_frame(text, &[], &[]).is_some() {
                    frame_lines.push(text.to_string());
                } else if detail.is_none() {
                    *detail = Some(text.trim().to_string());
                }
                true
            }
        }
    }

    fn build(self, config: &TelemetryParserConfig) -> TelemetryError {
        let parse_frames = |lines: &[String]| -> Vec<TelemetryFrame> {
            lines
                .iter()
                .filter_map(|l| parse_frame(l, &config.project_roots, &config.project_frames))
                .collect()
        };
        match self.kind {
            BlockKind::Error {
                err_type,
                message,
                frame_lines,
                ..
            } => TelemetryError {
                err_type: err_type.unwrap_or_else(|| "Error".to_string()),
                message: message.trim().to_string(),
                stack: (!frame_lines.is_empty()).then(|| frame_lines.join("\n")),
                frames: parse_frames(&frame_lines),
            },
            BlockKind::Flutter {
                library,
                err_type,
                msg,
                frame_lines,
                ..
            } => {
                let mut t = err_type.unwrap_or_else(|| "FlutterError".to_string());
                let mut m = msg.join(" ");
                if let Some(inner) = TYPE_FROM_MSG_RE.captures(&m) {
                    if let (Some(g1), Some(g2)) = (inner.get(1), inner.get(2)) {
                        t = g1.as_str().to_string();
                        m = g2.as_str().to_string();
                    }
                }
                if m.is_empty() {
                    m = format!("Exception caught by {library}");
                }
                TelemetryError {
                    err_type: t,
                    message: m,
                    stack: (!frame_lines.is_empty()).then(|| frame_lines.join("\n")),
                    frames: parse_frames(&frame_lines),
                }
            }
            BlockKind::Python {
                frame_lines,
                err_type,
                message,
            } => {
                let reversed: Vec<String> = frame_lines.iter().rev().cloned().collect();
                TelemetryError {
                    err_type: err_type.unwrap_or_else(|| "Exception".to_string()),
                    message: message.unwrap_or_default(),
                    stack: Some(frame_lines.join("\n")),
                    frames: parse_frames(&reversed),
                }
            }
            BlockKind::Rust {
                file,
                line,
                column,
                inline_message,
                msg,
                frame_lines,
            } => {
                let head = TelemetryFrame {
                    raw: format!("{file}:{line}:{column}"),
                    file: Some(normalize_project_path(&file, &config.project_roots)),
                    line: Some(line),
                    column: Some(column),
                    function: None,
                    in_project: is_project_file(
                        &file,
                        &config.project_roots,
                        &config.project_frames,
                    ),
                };
                let mut frames = vec![head];
                frames.extend(parse_frames(&frame_lines));
                TelemetryError {
                    err_type: "Panic".to_string(),
                    message: if !inline_message.is_empty() {
                        inline_message
                    } else {
                        msg.join(" ")
                    },
                    stack: (!frame_lines.is_empty()).then(|| frame_lines.join("\n")),
                    frames,
                }
            }
            BlockKind::Go {
                message,
                frame_lines,
                ..
            } => TelemetryError {
                err_type: "Panic".to_string(),
                message,
                stack: (!frame_lines.is_empty()).then(|| frame_lines.join("\n")),
                frames: parse_frames(&frame_lines),
            },
            BlockKind::Test {
                name,
                detail,
                frame_lines,
            } => TelemetryError {
                err_type: "Test failed".to_string(),
                message: match detail {
                    None => name,
                    Some(d) => format!("{name}: {d}"),
                },
                stack: (!frame_lines.is_empty()).then(|| frame_lines.join("\n")),
                frames: parse_frames(&frame_lines),
            },
        }
    }
}

pub struct TelemetryLineParser {
    run_id: String,
    config: TelemetryParserConfig,
    in_alt: bool,
    block: Option<BlockState>,
    last_error: Option<TelemetryError>,
}

impl TelemetryLineParser {
    pub fn new(run_id: impl Into<String>, config: TelemetryParserConfig) -> Self {
        Self {
            run_id: run_id.into(),
            config,
            in_alt: false,
            block: None,
            last_error: None,
        }
    }

    pub fn feed(
        &mut self,
        line: &str,
        stream: TelemetryStream,
        at_ms: i64,
        offset: Option<i64>,
    ) -> Vec<TelemetryEvent> {
        let toggle = scan_alt_screen(line);
        if toggle == AltScreenToggle::Enter {
            self.in_alt = true;
        }
        if toggle == AltScreenToggle::Exit {
            self.in_alt = false;
            return Vec::new();
        }
        if self.in_alt {
            return Vec::new();
        }
        let clean = strip_ansi(line);
        let clean = clean.trim_end();

        let u = unwrap_prefixes(clean, &self.config.unwrap);
        let text = u.text;
        let ctx = Ctx {
            stream,
            at_ms,
            offset,
            service: u.service.clone(),
        };

        let mut out = Vec::new();
        if let Some(mut blk) = self.block.take() {
            if blk.consume(&text) {
                if blk.done {
                    let b_ctx = blk.ctx.clone();
                    let err = blk.build(&self.config);
                    out.push(self.error_event(err, &b_ctx));
                } else {
                    self.block = Some(blk);
                }
                return out;
            }
            let b_ctx = blk.ctx.clone();
            let err = blk.build(&self.config);
            out.push(self.error_event(err, &b_ctx));
        }

        if text.trim().is_empty() {
            return out;
        }

        if let Some(b) = Self::start_block(&text, &ctx) {
            self.block = Some(b);
            return out;
        }

        if let Some(another) = ANOTHER_EXCEPTION_RE.captures(&text) {
            if let Some(msg) = another.get(1) {
                let err = self.error_from_message(msg.as_str(), true);
                out.push(self.error_event(err, &ctx));
                return out;
            }
        }

        if let Some(json_ev) = parse_json_line(
            &text,
            &self.run_id,
            at_ms,
            stream,
            offset,
            u.service.as_deref(),
            &self.config.project_roots,
            &self.config.project_frames,
        ) {
            if let Some(ref e) = json_ev.error {
                self.last_error = Some(e.clone());
            }
            out.push(json_ev);
            return out;
        }

        out.push(self.raw_event(&text, &ctx));
        out
    }

    pub fn flush(&mut self) -> Vec<TelemetryEvent> {
        let Some(blk) = self.block.take() else {
            return Vec::new();
        };
        let b_ctx = blk.ctx.clone();
        let err = blk.build(&self.config);
        vec![self.error_event(err, &b_ctx)]
    }

    fn start_block(text: &str, ctx: &Ctx) -> Option<BlockState> {
        if let Some(m) = FLUTTER_START_RE.captures(text) {
            return Some(BlockState {
                ctx: ctx.clone(),
                done: false,
                lines: 0,
                kind: BlockKind::Flutter {
                    library: m.get(1).map_or("", |g| g.as_str()).to_string(),
                    err_type: None,
                    msg: Vec::new(),
                    frame_lines: Vec::new(),
                    in_stack: false,
                    in_msg: false,
                },
            });
        }
        if PYTHON_START_RE.is_match(text) {
            return Some(BlockState {
                ctx: ctx.clone(),
                done: false,
                lines: 0,
                kind: BlockKind::Python {
                    frame_lines: Vec::new(),
                    err_type: None,
                    message: None,
                },
            });
        }
        if let Some(m) = RUST_PANIC_RE.captures(text) {
            let file = m.get(2).map_or("", |g| g.as_str()).to_string();
            let line = m.get(3).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
            let column = m.get(4).and_then(|g| g.as_str().parse().ok()).unwrap_or(0);
            let inline_message = m.get(5).map_or("", |g| g.as_str()).trim().to_string();
            return Some(BlockState {
                ctx: ctx.clone(),
                done: false,
                lines: 0,
                kind: BlockKind::Rust {
                    file,
                    line,
                    column,
                    inline_message,
                    msg: Vec::new(),
                    frame_lines: Vec::new(),
                },
            });
        }
        if let Some(m) = GO_PANIC_RE.captures(text) {
            return Some(BlockState {
                ctx: ctx.clone(),
                done: false,
                lines: 0,
                kind: BlockKind::Go {
                    message: m.get(1).map_or("", |g| g.as_str()).to_string(),
                    frame_lines: Vec::new(),
                    started: false,
                },
            });
        }
        if let Some(m) = DART_TEST_FAIL_RE.captures(text) {
            return Some(BlockState {
                ctx: ctx.clone(),
                done: false,
                lines: 0,
                kind: BlockKind::Test {
                    name: m.get(1).map_or("", |g| g.as_str()).to_string(),
                    detail: None,
                    frame_lines: Vec::new(),
                },
            });
        }
        if text.starts_with("  ") {
            if let Some(m) = JEST_FAIL_RE.captures(text) {
                return Some(BlockState {
                    ctx: ctx.clone(),
                    done: false,
                    lines: 0,
                    kind: BlockKind::Test {
                        name: m.get(1).map_or("", |g| g.as_str()).to_string(),
                        detail: None,
                        frame_lines: Vec::new(),
                    },
                });
            }
        }
        if let Some(m) = PYTEST_FAIL_RE.captures(text) {
            return Some(BlockState {
                ctx: ctx.clone(),
                done: true,
                lines: 0,
                kind: BlockKind::Test {
                    name: m.get(1).map_or("", |g| g.as_str()).to_string(),
                    detail: m.get(2).map(|g| g.as_str().to_string()),
                    frame_lines: Vec::new(),
                },
            });
        }
        if UNHANDLED_ONLY_RE.is_match(text) {
            return Some(BlockState {
                ctx: ctx.clone(),
                done: false,
                lines: 0,
                kind: BlockKind::Error {
                    err_type: None,
                    message: String::new(),
                    frame_lines: Vec::new(),
                    detail: Vec::new(),
                    extra_msg: 0,
                },
            });
        }
        if let Some(m) = ERROR_HEAD_RE.captures(text) {
            return Some(BlockState {
                ctx: ctx.clone(),
                done: false,
                lines: 0,
                kind: BlockKind::Error {
                    err_type: m.get(1).map(|g| g.as_str().to_string()),
                    message: m.get(3).map_or("", |g| g.as_str()).to_string(),
                    frame_lines: Vec::new(),
                    detail: Vec::new(),
                    extra_msg: 0,
                },
            });
        }
        if let Some(m) = PLAIN_ERROR_RE.captures(text) {
            let t = m.get(1).map_or("Error", |g| g.as_str());
            let err_type = if t == "Fatal error" {
                "FatalError".to_string()
            } else {
                let mut chars = t.chars();
                match chars.next() {
                    Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                    None => "Error".to_string(),
                }
            };
            return Some(BlockState {
                ctx: ctx.clone(),
                done: false,
                lines: 0,
                kind: BlockKind::Error {
                    err_type: Some(err_type),
                    message: m.get(2).map_or("", |g| g.as_str()).to_string(),
                    frame_lines: Vec::new(),
                    detail: Vec::new(),
                    extra_msg: 0,
                },
            });
        }
        None
    }

    fn error_from_message(&self, raw: &str, reuse_frames: bool) -> TelemetryError {
        let m = TYPE_FROM_MSG_RE.captures(raw);
        let err_type = m
            .as_ref()
            .and_then(|c| c.get(1))
            .map_or("Error", |g| g.as_str())
            .to_string();
        let message = m
            .as_ref()
            .and_then(|c| c.get(2))
            .map_or(raw, |g| g.as_str())
            .to_string();
        let mut frames = Vec::new();
        if reuse_frames {
            if let Some(ref last) = self.last_error {
                if last.err_type == err_type
                    && normalize_message(&last.message) == normalize_message(&message)
                {
                    frames = last.frames.clone();
                }
            }
        }
        TelemetryError {
            err_type,
            message,
            stack: None,
            frames,
        }
    }

    fn error_event(&mut self, err: TelemetryError, ctx: &Ctx) -> TelemetryEvent {
        self.last_error = Some(err.clone());
        let sev = if err.err_type.ends_with("Warning") {
            TelemetrySeverity::Warn
        } else if err.err_type == "Panic" || err.err_type == "FatalError" {
            TelemetrySeverity::Fatal
        } else {
            TelemetrySeverity::Error
        };
        let mut attrs = Map::new();
        if let Some(ref svc) = ctx.service {
            attrs.insert("service".to_string(), Value::String(svc.clone()));
        }
        let fp = fingerprint_of(&err);
        TelemetryEvent {
            id: None,
            run_id: self.run_id.clone(),
            ts_ms: ctx.at_ms,
            stream: ctx.stream,
            severity: sev,
            body: err.message.clone(),
            attrs,
            error: Some(err),
            fingerprint: Some(fp),
            trace_id: None,
            span_id: None,
            request_id: None,
            raw_line_offset: ctx.offset,
            repeat: 1,
        }
    }

    fn raw_event(&self, text: &str, ctx: &Ctx) -> TelemetryEvent {
        let sev = if SEV_FATAL_RE.is_match(text) {
            TelemetrySeverity::Fatal
        } else if SEV_ERROR_RE.is_match(text).unwrap_or(false) && !ZERO_COUNT_RE.is_match(text) {
            TelemetrySeverity::Error
        } else if SEV_WARN_RE.is_match(text) && !ZERO_COUNT_RE.is_match(text) {
            TelemetrySeverity::Warn
        } else if SEV_DEBUG_RE.is_match(text) {
            TelemetrySeverity::Debug
        } else {
            TelemetrySeverity::Info
        };
        let mut attrs = Map::new();
        if let Some(ref svc) = ctx.service {
            attrs.insert("service".to_string(), Value::String(svc.clone()));
        }
        TelemetryEvent {
            id: None,
            run_id: self.run_id.clone(),
            ts_ms: ctx.at_ms,
            stream: ctx.stream,
            severity: sev,
            body: text.trim().to_string(),
            attrs,
            error: None,
            fingerprint: None,
            trace_id: None,
            span_id: None,
            request_id: None,
            raw_line_offset: ctx.offset,
            repeat: 1,
        }
    }
}
