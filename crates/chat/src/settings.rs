//! `~/.claude/settings.json` do usuário, só leitura (`ClaudeCodeSettingsFile.getValue`).

use std::path::{Path, PathBuf};

use serde_json::Value;

/// `crossSessionInbound`: mensagens de outras sessões (`accept`, `hold`, `refuse`).
pub const CROSS_SESSION_INBOUND: &str = "crossSessionInbound";

fn settings_file() -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".claude").join("settings.json"))
}

/// Valor em `key` do JSON; chave com ponto (`sandbox.enabled`) desce nos objetos aninhados.
pub fn value_at(root: &Value, key: &str) -> Option<Value> {
    let mut node = root;
    for part in key.split('.') {
        node = node.as_object()?.get(part)?;
    }
    (!node.is_null()).then(|| node.clone())
}

fn read_value(file: &Path, key: &str) -> Option<Value> {
    let text = std::fs::read_to_string(file).ok()?;
    let root: Value = serde_json::from_str(&text).ok()?;
    value_at(&root, key)
}

/// Valor cru em `key`; `None` se ausente ou se o arquivo não dá para ler.
pub fn claude_setting(key: &str) -> Option<Value> {
    read_value(&settings_file()?, key)
}

/// Sem escolha explícita em `~/.claude/settings.json`, o Claude Code segura a mensagem de
/// outra sessão quando os modos de permissão diferem, e no modo headless ela expira sem
/// ninguém ver. O chat do GSH entrega por padrão.
pub fn accept_peers() -> bool {
    claude_setting(CROSS_SESSION_INBOUND).is_none()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn chave_com_ponto_desce_nos_objetos() {
        let root = json!({"sandbox": {"enabled": true}, "crossSessionInbound": "hold", "x": null});
        assert_eq!(value_at(&root, "sandbox.enabled"), Some(json!(true)));
        assert_eq!(value_at(&root, CROSS_SESSION_INBOUND), Some(json!("hold")));
        assert_eq!(value_at(&root, "sandbox.nada"), None);
        assert_eq!(value_at(&root, "x"), None);
        assert_eq!(value_at(&root, "crossSessionInbound.a"), None);
    }
}
