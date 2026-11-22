//! O transcript que o próprio Claude Code grava e a lista de sessões da máquina
//! (`readClaudeHistory`, `writeClaudeTitle` de `claude_stream_engine.dart` e
//! `claude_session_directory.dart` do Flutter).

#[cfg(not(target_os = "linux"))]
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

use crate::error::ChatError;
use crate::event::ChatEvent;
use crate::json::{obj_of, str_of};
use crate::protocol::parse_stream_object;
use crate::time::parse_rfc3339_ms;

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

#[cfg(target_os = "linux")]
fn open_transcript(file: &Path, append: bool) -> std::io::Result<File> {
    use std::os::fd::FromRawFd;
    use std::os::unix::ffi::OsStrExt;

    let path = std::ffi::CString::new(file.as_os_str().as_bytes())?;
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    let access = if append {
        libc::O_WRONLY | libc::O_APPEND
    } else {
        libc::O_RDONLY
    };
    how.flags = (access | libc::O_CLOEXEC | libc::O_NONBLOCK) as u64;
    how.resolve = libc::RESOLVE_NO_SYMLINKS;
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            libc::AT_FDCWD,
            path.as_ptr(),
            &how,
            std::mem::size_of_val(&how),
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let opened = unsafe { File::from_raw_fd(fd as std::os::fd::RawFd) };
    use std::os::unix::fs::MetadataExt;
    let metadata = opened.metadata()?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(std::io::Error::other("unsafe Claude transcript file"));
    }
    Ok(opened)
}

#[cfg(not(target_os = "linux"))]
fn open_transcript(file: &Path, append: bool) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options
        .write(append)
        .append(append)
        .read(!append)
        .open(file)
}

/// `~/.claude/projects`.
pub fn projects_root() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir).join("projects"));
    }
    home_dir().map(|h| h.join(".claude").join("projects"))
}

/// `[A-Za-z0-9-]+` (o id vira nome de arquivo e argumento de shell).
pub fn is_session_id(id: &str) -> bool {
    !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// A pasta do projeto no transcript: o `cwd` com cada caractere não alfanumérico trocado
/// por `-` (por unidade UTF-16, como o Dart).
pub fn project_slug(cwd: &str) -> String {
    let mut out = String::with_capacity(cwd.len());
    for c in cwd.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            for _ in 0..c.len_utf16() {
                out.push('-');
            }
        }
    }
    out
}

/// `~/.claude/projects/<slug do cwd>/<id>.jsonl`. `None` se o id não tem o formato esperado
/// ou não há HOME.
pub fn transcript_file(cwd: &str, session_id: &str) -> Option<PathBuf> {
    transcript_in(&projects_root()?, cwd, session_id)
}

pub fn transcript_in(root: &Path, cwd: &str, session_id: &str) -> Option<PathBuf> {
    if !is_session_id(session_id) {
        return None;
    }
    Some(
        root.join(project_slug(cwd))
            .join(format!("{session_id}.jsonl")),
    )
}

pub fn profile_transcript_file(root: &Path, cwd: &str, session_id: &str) -> Option<PathBuf> {
    let file = transcript_in(root, cwd, session_id)?;
    let directory = file.parent()?;
    if !fs::symlink_metadata(directory).is_ok_and(|meta| meta.is_dir())
        || !fs::symlink_metadata(&file).is_ok_and(|meta| meta.is_file())
    {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if !fs::symlink_metadata(&file).is_ok_and(|meta| meta.nlink() == 1) {
            return None;
        }
    }
    Some(file)
}

fn at_ms(o: &Map<String, Value>) -> Option<u64> {
    str_of(o, "timestamp").and_then(parse_rfc3339_ms)
}

/// Traduz as linhas do transcript em eventos. Linhas `user`/`assistant` têm o mesmo formato
/// do stream.
pub fn history_from_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> Vec<ChatEvent> {
    let mut out = Vec::new();
    for line in lines {
        let Ok(Value::Object(o)) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if o.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let kind = str_of(&o, "type");
        if kind == Some("custom-title") {
            if let Some(t) = str_of(&o, "customTitle") {
                out.push(ChatEvent::Renamed(t.to_string()));
                continue;
            }
        }
        if kind == Some("user") && o.get("isMeta").and_then(Value::as_bool) != Some(true) {
            let content = obj_of(&o, "message").and_then(|m| m.get("content"));
            if let Some(text) = content.and_then(Value::as_str) {
                if !text.starts_with('<') {
                    out.push(ChatEvent::UserText {
                        text: text.to_string(),
                        at_ms: at_ms(&o),
                    });
                    continue;
                }
            }
        }
        if matches!(kind, Some("user" | "assistant")) {
            let at = at_ms(&o);
            for e in parse_stream_object(&o) {
                out.push(match e {
                    ChatEvent::Text { text, .. } if at.is_some() => {
                        ChatEvent::Text { text, at_ms: at }
                    }
                    other => other,
                });
            }
        }
    }
    out
}

/// A conversa já gravada da sessão. Vazio se o transcript não existe.
pub fn read_history(cwd: &str, session_id: &str) -> Vec<ChatEvent> {
    let Some(file) = transcript_file(cwd, session_id) else {
        return Vec::new();
    };
    read_transcript(&file)
}

pub fn read_transcript(file: &Path) -> Vec<ChatEvent> {
    let Ok(mut source) = open_transcript(file, false) else {
        return Vec::new();
    };
    let mut bytes = Vec::new();
    if source.read_to_end(&mut bytes).is_err() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&bytes);
    history_from_lines(text.lines())
}

/// Grava o nome da sessão como o `/rename` do Claude Code: uma linha `custom-title` no fim do
/// transcript (vale a última). Sem transcript ainda, não faz nada.
pub fn write_title(cwd: &str, session_id: &str, title: &str) -> Result<(), ChatError> {
    match transcript_file(cwd, session_id) {
        Some(file) if file.exists() => append_title(&file, session_id, title),
        _ => Ok(()),
    }
}

pub fn append_title(file: &Path, session_id: &str, title: &str) -> Result<(), ChatError> {
    let line = json!({"type": "custom-title", "customTitle": title, "sessionId": session_id});
    let io = |e: std::io::Error| ChatError::WriteTranscript {
        path: file.display().to_string(),
        cause: e.to_string(),
    };
    let mut f = open_transcript(file, true).map_err(io)?;
    writeln!(f, "{line}").map_err(io)?;
    f.flush().map_err(io)
}

/// Uma sessão do Claude Code desta máquina (seletor "Sessões" e "Retomar conversa").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionEntry {
    pub id: String,
    pub title: String,
    pub project: String,
    /// Última modificação do transcript, em epoch ms.
    pub last_seen_ms: u64,
}

/// Começo (pasta, primeiro pedido) e fim (o título mais novo) do transcript: ler inteiro leva
/// segundos com conversas de dezenas de MB.
const HEAD_BYTES: u64 = 64 * 1024;
const TAIL_BYTES: u64 = 512 * 1024;

fn nonblank(v: Option<&str>) -> Option<String> {
    v.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn short_prompt(s: &str) -> Option<String> {
    let line = s.trim().lines().next().unwrap_or("");
    if s.trim().is_empty() {
        None
    } else if line.chars().count() <= 48 {
        Some(line.to_string())
    } else {
        let head: String = line.chars().take(47).collect();
        Some(format!("{head}…"))
    }
}

#[derive(Default)]
struct Scan {
    custom: Option<String>,
    agent: Option<String>,
    ai: Option<String>,
    first_prompt: Option<String>,
    cwd: Option<String>,
}

impl Scan {
    fn feed(&mut self, bytes: &[u8]) {
        for line in String::from_utf8_lossy(bytes).lines() {
            // Linha cortada na borda do pedaço lido não é JSON: pula.
            let Ok(Value::Object(o)) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if self.cwd.is_none() {
                self.cwd = str_of(&o, "cwd").map(ToOwned::to_owned);
            }
            match str_of(&o, "type") {
                Some("custom-title") => {
                    self.custom = nonblank(str_of(&o, "customTitle")).or(self.custom.take());
                }
                Some("agent-name") => {
                    if self.agent.is_none() {
                        self.agent = nonblank(str_of(&o, "agentName"));
                    }
                }
                Some("ai-title") => {
                    self.ai = nonblank(str_of(&o, "aiTitle")).or(self.ai.take());
                }
                Some("user")
                    if self.first_prompt.is_none()
                        && o.get("isMeta").and_then(Value::as_bool) != Some(true) =>
                {
                    let c = obj_of(&o, "message").and_then(|m| m.get("content"));
                    if let Some(text) = c.and_then(Value::as_str) {
                        if !text.starts_with('<') {
                            self.first_prompt = Some(text.to_string());
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

fn read_chunk(f: &mut File, from: u64, len: u64) -> std::io::Result<Vec<u8>> {
    f.seek(SeekFrom::Start(from))?;
    let mut buf = Vec::new();
    f.take(len).read_to_end(&mut buf)?;
    Ok(buf)
}

fn read_entry(path: &Path, now: SystemTime, within: Duration) -> Option<SessionEntry> {
    let mut f = open_transcript(path, false).ok()?;
    let metadata = f.metadata().ok()?;
    let modified = metadata.modified().ok()?;
    if now.duration_since(modified).is_ok_and(|age| age > within) {
        return None;
    }
    let len = metadata.len();
    let mut scan = Scan::default();
    scan.feed(&read_chunk(&mut f, 0, len.min(HEAD_BYTES)).ok()?);
    if len > HEAD_BYTES {
        let from = if len - TAIL_BYTES.min(len) > HEAD_BYTES {
            len - TAIL_BYTES
        } else {
            HEAD_BYTES
        };
        scan.feed(&read_chunk(&mut f, from, len - from).ok()?);
    }
    let title = scan
        .custom
        .or(scan.agent)
        .or(scan.ai)
        .or_else(|| scan.first_prompt.as_deref().and_then(short_prompt))?;
    // Conversas de teste/rascunho em pasta temporária só poluem a lista.
    if let Some(project) = &scan.cwd {
        if Path::new(project).starts_with(std::env::temp_dir()) {
            return None;
        }
    }
    let parent = path.parent()?.file_name()?.to_string_lossy().into_owned();
    Some(SessionEntry {
        id: path.file_stem()?.to_string_lossy().into_owned(),
        title,
        project: scan.cwd.unwrap_or(parent),
        last_seen_ms: modified
            .duration_since(UNIX_EPOCH)
            .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
            .unwrap_or(0),
    })
}

/// Sessões com transcript mexido nos últimos `within`, a mais recente primeiro. Quem lista é
/// o disco: sessões na nuvem ou em outra máquina ficam de fora.
pub fn recent_sessions(root: &Path, within: Duration) -> Vec<SessionEntry> {
    let Ok(dirs) = fs::read_dir(root) else {
        return Vec::new();
    };
    let now = SystemTime::now();
    let mut out = Vec::new();
    for dir in dirs.filter_map(Result::ok) {
        if !dir.file_type().is_ok_and(|t| t.is_dir()) {
            continue;
        }
        let Ok(files) = fs::read_dir(dir.path()) else {
            continue;
        };
        for f in files.filter_map(Result::ok) {
            let path = f.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl")
                || !f.file_type().is_ok_and(|ty| ty.is_file())
            {
                continue;
            }
            if let Some(entry) = read_entry(&path, now, within) {
                out.push(entry);
            }
        }
    }
    out.sort_by_key(|entry| std::cmp::Reverse(entry.last_seen_ms));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_e_arquivo_do_transcript() {
        assert_eq!(project_slug("/home/h/GSH Desktop"), "-home-h-GSH-Desktop");
        assert!(is_session_id("1b2c-3d"));
        assert!(!is_session_id("../x"));
        assert!(!is_session_id(""));
        assert_eq!(transcript_file("/a", "../x"), None);
    }

    #[test]
    fn historico_traduz_usuario_assistente_e_titulo() {
        let lines = [
            r#"{"type":"custom-title","customTitle":"Refatorar login","sessionId":"s"}"#,
            r#"{"type":"user","message":{"role":"user","content":"antes"},"timestamp":"2026-01-01T03:05:00Z"}"#,
            r#"{"type":"user","isMeta":true,"message":{"role":"user","content":"meta"}}"#,
            r#"{"type":"user","message":{"role":"user","content":"<command-name>x</command-name>"}}"#,
            r#"{"type":"assistant","timestamp":"2026-01-01T03:05:09Z","message":{"content":[{"type":"text","text":"resposta antiga"}]}}"#,
            r#"{"type":"assistant","isSidechain":true,"message":{"content":[{"type":"text","text":"sub"}]}}"#,
            "{linha cortada",
        ];
        let evs = history_from_lines(lines);
        assert_eq!(
            evs,
            vec![
                ChatEvent::Renamed("Refatorar login".into()),
                ChatEvent::UserText {
                    text: "antes".into(),
                    at_ms: Some(1_767_236_700_000)
                },
                ChatEvent::Text {
                    text: "resposta antiga".into(),
                    at_ms: Some(1_767_236_709_000)
                },
            ]
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn transcript_read_rejects_symlink_swap() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!("gsh-chat-symlink-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let target = root.join("target.jsonl");
        let alias = root.join("session.jsonl");
        fs::write(&target, r#"{"type":"user","message":{"content":"secret"}}"#).unwrap();
        symlink(&target, &alias).unwrap();
        assert!(read_transcript(&alias).is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn lista_de_sessoes_usa_titulo_mais_novo_e_ordena() {
        let root = std::env::temp_dir().join(format!("gsh-chat-sess-{}", std::process::id()));
        let proj = root.join("-home-h-proj");
        fs::create_dir_all(&proj).expect("dir");
        // `cwd` fora de /tmp (senão o filtro de rascunho descarta).
        fs::write(
            proj.join("aaaa.jsonl"),
            concat!(
                r#"{"type":"user","cwd":"/home/h/proj","message":{"role":"user","content":"primeiro pedido"}}"#,
                "\n",
                r#"{"type":"ai-title","aiTitle":"Título da IA"}"#,
                "\n",
                r#"{"type":"custom-title","customTitle":"Meu nome"}"#,
                "\n",
            ),
        )
        .expect("a");
        fs::write(
            proj.join("bbbb.jsonl"),
            r#"{"type":"user","cwd":"/home/h/proj","message":{"role":"user","content":"só um pedido"}}"#,
        )
        .expect("b");
        let list = recent_sessions(&root, Duration::from_secs(3600));
        let mut titles: Vec<_> = list.iter().map(|s| s.title.as_str()).collect();
        titles.sort_unstable();
        assert_eq!(titles, ["Meu nome", "só um pedido"]);
        assert!(list.iter().all(|s| s.project == "/home/h/proj"));
        assert!(recent_sessions(&root, Duration::ZERO).len() <= 2);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn titulo_vai_no_fim_do_transcript_e_volta_pelo_historico() {
        let home = std::env::temp_dir().join(format!("gsh-chat-title-{}", std::process::id()));
        let file = home.join("t.jsonl");
        fs::create_dir_all(&home).expect("dir");
        fs::write(&file, "{\"type\":\"user\"}\n").expect("w");
        append_title(&file, "s", "Refatorar login").expect("append");
        let back = fs::read_to_string(&file).expect("read");
        assert_eq!(
            history_from_lines(back.lines()),
            vec![ChatEvent::Renamed("Refatorar login".into())]
        );
        let _ = fs::remove_dir_all(&home);
    }
}
