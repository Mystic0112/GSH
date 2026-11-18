use std::sync::LazyLock;

use regex::Regex;

static CSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]").expect("csi regex"));
static OSC_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)").expect("osc regex"));
static OTHER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b[@-Z\\-_]").expect("other esc regex"));
static CONTROLS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]").expect("controls regex"));

static ALT_ENTER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[\?(?:1049|1047|47)h").expect("alt enter regex"));
static ALT_EXIT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[\?(?:1049|1047|47)l").expect("alt exit regex"));

/// Resultado de [`scan_alt_screen`]: se a linha liga/desliga o alt-screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AltScreenToggle {
    None,
    Enter,
    Exit,
}

/// Remove CSI/OSC/ESC simples e controles (mantém `\t`).
pub fn strip_ansi(s: &str) -> String {
    if !s.contains('\x1b') && !CONTROLS_RE.is_match(s) {
        return s.to_string();
    }
    let s = OSC_RE.replace_all(s, "");
    let s = CSI_RE.replace_all(&s, "");
    let s = OTHER_RE.replace_all(&s, "");
    CONTROLS_RE.replace_all(&s, "").into_owned()
}

/// Detecta a última transição de alt-screen na linha (a última vence quando
/// a mesma linha entra e sai).
pub fn scan_alt_screen(raw: &str) -> AltScreenToggle {
    if !raw.contains("\x1b[?") {
        return AltScreenToggle::None;
    }
    let enter = ALT_ENTER_RE.find_iter(raw).last().map(|m| m.start());
    let exit = ALT_EXIT_RE.find_iter(raw).last().map(|m| m.start());
    match (enter, exit) {
        (None, None) => AltScreenToggle::None,
        (Some(_), None) => AltScreenToggle::Enter,
        (None, Some(_)) => AltScreenToggle::Exit,
        (Some(en), Some(ex)) => {
            if en > ex {
                AltScreenToggle::Enter
            } else {
                AltScreenToggle::Exit
            }
        }
    }
}
