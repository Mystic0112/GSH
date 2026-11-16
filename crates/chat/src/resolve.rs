//! Onde está o `claude` e com que ambiente ele roda (`resolveExecutable` e
//! `claudeEnvWithDirOnPath` do Flutter): app de janela não herda o PATH do shell.

use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Quanto esperar o `which` do shell do usuário (um `.bashrc` pesado demora).
const WHICH_TIMEOUT: Duration = Duration::from_secs(8);

fn path_sep() -> &'static str {
    if cfg!(windows) {
        ";"
    } else {
        ":"
    }
}

fn is_file(p: &Path) -> bool {
    p.is_file()
}

/// Procura `name` nos diretórios do PATH do próprio processo.
fn search_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    let exts: &[&str] = if cfg!(windows) {
        &["exe", "cmd", "bat"]
    } else {
        &[""]
    };
    env::split_paths(&path).find_map(|dir| {
        exts.iter().find_map(|ext| {
            let file = if ext.is_empty() {
                dir.join(name)
            } else {
                dir.join(format!("{name}.{ext}"))
            };
            is_file(&file).then_some(file)
        })
    })
}

/// `which`/`where` por um comando externo; primeiro caminho absoluto que existe no stdout.
fn run_which(program: &str, args: &[&str]) -> Option<PathBuf> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + WHICH_TIMEOUT;
    // Lê o stdout numa thread: o pipe encheria e travaria o filho se só esperássemos.
    let mut out = child.stdout.take()?;
    let reader = thread::spawn(move || {
        let mut buf = String::new();
        let _ = std::io::Read::read_to_string(&mut out, &mut buf);
        buf
    });
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
        }
    }
    let text = reader.join().ok()?;
    text.lines()
        .map(str::trim)
        .map(PathBuf::from)
        .find(|p| p.is_absolute() && is_file(p))
}

/// Caminho do executável `name`: PATH do processo; senão `which` no shell de login e no
/// interativo (nvm mora no `.bashrc`); senão os lugares onde o instalador do Claude Code
/// deixa o binário; senão o próprio nome (o SO resolve no spawn).
pub fn resolve_executable(name: &str) -> PathBuf {
    if let Some(found) = search_path(name) {
        return found;
    }
    if cfg!(windows) {
        if let Some(found) = run_which("where", &[name]) {
            return found;
        }
    } else {
        let shell = env::var("SHELL")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "/bin/sh".to_string());
        let cmd = format!("which {name}");
        for flag in ["-lc", "-ic"] {
            if let Some(found) = run_which(&shell, &[flag, &cmd]) {
                return found;
            }
        }
        if let Some(home) = env::var_os("HOME").map(PathBuf::from) {
            for rel in [".local/bin", ".claude/local", ".npm-global/bin"] {
                let file = home.join(rel).join(name);
                if is_file(&file) {
                    return file;
                }
            }
        }
        for dir in ["/usr/local/bin", "/opt/homebrew/bin"] {
            let file = Path::new(dir).join(name);
            if is_file(&file) {
                return file;
            }
        }
    }
    PathBuf::from(name)
}

/// O `claude` do npm é um script `#!/usr/bin/env node`. Aberto pelo launcher (sem o PATH do
/// shell, ex.: nvm), o `env` não acha o `node`: põe a pasta do executável (onde o nvm/npm
/// deixa os dois) na frente do PATH. `extra` entra junto; o PATH dele vira a base.
pub fn env_with_dir_on_path(exe: &Path, extra: &[(String, String)]) -> Vec<(String, String)> {
    let dir = exe
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.display().to_string());
    let base = extra
        .iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| v.clone())
        .or_else(|| env::var("PATH").ok())
        .unwrap_or_default();
    let path = match dir {
        Some(d) if base.is_empty() => d,
        Some(d) => format!("{d}{}{base}", path_sep()),
        None => base,
    };
    let mut out: Vec<(String, String)> =
        extra.iter().filter(|(k, _)| k != "PATH").cloned().collect();
    out.push(("PATH".to_string(), path));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasta_do_executavel_vai_na_frente_do_path_do_extra() {
        let extra = vec![
            ("GSH_TAB_ID".to_string(), "c1".to_string()),
            ("PATH".to_string(), "/gsh/bin:/usr/bin".to_string()),
        ];
        let env = env_with_dir_on_path(Path::new("/nvm/bin/claude"), &extra);
        assert!(env.contains(&("GSH_TAB_ID".to_string(), "c1".to_string())));
        let sep = path_sep();
        assert!(env.contains(&(
            "PATH".to_string(),
            format!("/nvm/bin{sep}/gsh/bin:/usr/bin")
        )));
        assert_eq!(env.iter().filter(|(k, _)| k == "PATH").count(), 1);
    }

    #[test]
    fn nome_solto_sem_pasta_nao_mexe_no_path() {
        let extra = vec![("PATH".to_string(), "/a".to_string())];
        let env = env_with_dir_on_path(Path::new("claude"), &extra);
        assert!(env.contains(&("PATH".to_string(), "/a".to_string())));
    }
}
