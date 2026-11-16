//! `~/.claude/settings.json` (`ClaudeCodeSettingsFile` e `ClaudeCodeKeys` do Flutter).
//!
//! Mexe só na chave pedida: lê o objeto inteiro, troca a chave e regrava tudo
//! (temporário + rename, para o Claude Code nunca ler meio arquivo). Preserva a ordem
//! das chaves e as que não conhece. Arquivo que não é um objeto JSON não é regravado:
//! melhor recusar do que apagar as configurações de alguém.

use gsh_harness::Json;
use std::path::{Path, PathBuf};

/// Chaves do `settings.json` que o GSH expõe nas próprias configurações.
pub mod keys {
    pub const REMOTE_CONTROL_AT_STARTUP: &str = "remoteControlAtStartup";
    pub const SWITCH_MODELS_ON_FLAG: &str = "switchModelsOnFlag";
    pub const INPUT_NEEDED_NOTIF: &str = "inputNeededNotifEnabled";
    pub const AGENT_PUSH_NOTIF: &str = "agentPushNotifEnabled";
    pub const PROMPT_SUGGESTION: &str = "promptSuggestionEnabled";
    pub const REDUCED_MOTION: &str = "prefersReducedMotion";
    pub const OUTPUT_STYLE: &str = "outputStyle";
    pub const CROSS_SESSION_INBOUND: &str = "crossSessionInbound";
    pub const SANDBOX_ENABLED: &str = "sandbox.enabled";
    pub const SANDBOX_ALLOW_UNSANDBOXED: &str = "sandbox.allowUnsandboxedCommands";
    pub const DISABLE_TELEMETRY: &str = "env.DISABLE_TELEMETRY";
    pub const DISABLE_ERROR_REPORTING: &str = "env.DISABLE_ERROR_REPORTING";
}

/// O `settings.json` lido do disco: o objeto, ou o motivo de não dar para usá-lo.
pub type Snapshot = Result<Json, String>;

pub fn settings_path(home: &Path) -> PathBuf {
    home.join(".claude").join("settings.json")
}

/// Lê o arquivo. Ausente ou vazio vale um objeto vazio.
pub fn read(path: &Path) -> Snapshot {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Json::Obj(Vec::new())),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    if text.trim().is_empty() {
        return Ok(Json::Obj(Vec::new()));
    }
    match Json::parse(&text) {
        Ok(root @ Json::Obj(_)) => Ok(root),
        Ok(_) => Err(format!("{}: não é um objeto JSON", path.display())),
        Err(e) => Err(format!("{}: JSON inválido ({e})", path.display())),
    }
}

/// Valor em `key`; chave com ponto (`sandbox.enabled`) desce nos objetos aninhados.
pub fn value<'a>(snapshot: &'a Snapshot, key: &str) -> Option<&'a Json> {
    let mut node = snapshot.as_ref().ok()?;
    for part in key.split('.') {
        node = node.get(part)?;
    }
    Some(node)
}

/// Grava `value` em `key` criando os objetos do caminho; `None` remove a chave (e o
/// objeto pai que ficar vazio). Regrava o arquivo mesmo sem mudança, como o Flutter.
pub fn set(path: &Path, key: &str, new: Option<Json>) -> Result<(), String> {
    let mut root = read(path)?;
    if let Json::Obj(items) = &mut root {
        let parts: Vec<&str> = key.split('.').collect();
        put(items, &parts, new);
    }
    let mut text = root.to_pretty();
    text.push('\n');
    write_atomic(path, &text).map_err(|e| format!("{}: {e}", path.display()))
}

fn put(map: &mut Vec<(String, Json)>, path: &[&str], value: Option<Json>) {
    let Some((key, rest)) = path.split_first() else {
        return;
    };
    let position = map.iter().position(|(k, _)| k == key);
    if rest.is_empty() {
        match (position, value) {
            (Some(i), None) => {
                map.remove(i);
            }
            (Some(i), Some(v)) => {
                if let Some(slot) = map.get_mut(i) {
                    slot.1 = v;
                }
            }
            (None, Some(v)) => map.push(((*key).to_owned(), v)),
            (None, None) => {}
        }
        return;
    }
    let existing = position.and_then(|i| match map.get(i) {
        Some((_, Json::Obj(items))) => Some(items.clone()),
        _ => None,
    });
    // remover uma folha de algo que não é objeto não muda nada
    if value.is_none() && existing.is_none() {
        return;
    }
    let mut child = existing.unwrap_or_default();
    put(&mut child, rest, value);
    match (position, child.is_empty()) {
        (Some(i), true) => {
            map.remove(i);
        }
        (Some(i), false) => {
            if let Some(slot) = map.get_mut(i) {
                slot.1 = Json::Obj(child);
            }
        }
        (None, false) => map.push(((*key).to_owned(), Json::Obj(child))),
        (None, true) => {}
    }
}

/// Temporário na mesma pasta e rename por cima, criando a pasta se faltar.
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(format!(
        ".gsh-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let temp = path.with_file_name(name);
    let result = std::fs::write(&temp, contents).and_then(|()| std::fs::rename(&temp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("gsh-claude-file-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("settings.json")
    }

    #[test]
    fn set_keeps_order_and_unknown_keys_and_prunes_empty_parents() {
        let path = temp("order");
        std::fs::write(
            &path,
            r#"{"zeta":1,"env":{"KEEP":"x"},"alpha":[1,2],"sandbox":{"enabled":true}}"#,
        )
        .unwrap();
        set(&path, keys::DISABLE_TELEMETRY, Some(Json::Str("1".into()))).unwrap();
        set(&path, keys::SANDBOX_ENABLED, None).unwrap();
        let snap = read(&path);
        assert_eq!(
            value(&snap, "env.DISABLE_TELEMETRY"),
            Some(&Json::Str("1".into()))
        );
        assert_eq!(value(&snap, "env.KEEP"), Some(&Json::Str("x".into())));
        assert!(value(&snap, "sandbox").is_none());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.ends_with("}\n"));
        let zeta = text.find("zeta").unwrap();
        let env = text.find("env").unwrap();
        let alpha = text.find("alpha").unwrap();
        assert!(zeta < env && env < alpha);
        set(&path, keys::DISABLE_TELEMETRY, None).unwrap();
        set(&path, "env.KEEP", None).unwrap();
        assert!(value(&read(&path), "env").is_none());
        let dir = path.parent().unwrap().to_path_buf();
        let leftovers = std::fs::read_dir(&dir).unwrap().count();
        assert_eq!(leftovers, 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_file_is_created_and_non_object_is_refused_untouched() {
        let path = temp("create");
        assert!(read(&path).is_ok());
        set(
            &path,
            keys::OUTPUT_STYLE,
            Some(Json::Str("Learning".into())),
        )
        .unwrap();
        assert_eq!(
            value(&read(&path), "outputStyle"),
            Some(&Json::Str("Learning".into()))
        );
        std::fs::write(&path, "[1,2]").unwrap();
        assert!(set(&path, keys::OUTPUT_STYLE, None).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "[1,2]");
        std::fs::write(&path, "{quebrado").unwrap();
        assert!(read(&path).unwrap_err().contains("JSON inválido"));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{quebrado");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn removing_through_a_non_object_changes_nothing() {
        let path = temp("scalar");
        std::fs::write(&path, r#"{"sandbox":"on"}"#).unwrap();
        set(&path, keys::SANDBOX_ENABLED, None).unwrap();
        assert_eq!(
            value(&read(&path), "sandbox"),
            Some(&Json::Str("on".into()))
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
