//! `~/.claude/settings.json` do usuário, só leitura (`ClaudeCodeSettingsFile.getValue`).

use std::path::{Path, PathBuf};

use serde_json::Value;

/// `crossSessionInbound`: mensagens de outras sessões (`accept`, `hold`, `refuse`).
pub const CROSS_SESSION_INBOUND: &str = "crossSessionInbound";

/// Environment overrides that take precedence over a profile's own Claude login.
pub const PROFILE_AUTH_ENV: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_CUSTOM_HEADERS",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
    "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_USE_VERTEX",
];

pub(crate) fn apply_profile_env(cmd: &mut std::process::Command, env: &[(String, String)]) {
    if env.iter().any(|(key, _)| key == "CLAUDE_CONFIG_DIR") {
        for key in PROFILE_AUTH_ENV {
            cmd.env_remove(key);
        }
    }
}

fn settings_file() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("CLAUDE_CONFIG_DIR").filter(|dir| !dir.is_empty()) {
        return Some(PathBuf::from(dir).join("settings.json"));
    }
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
    accept_peers_in(None)
}

pub fn accept_peers_in(config_dir: Option<&Path>) -> bool {
    config_dir
        .map(|dir| read_value(&dir.join("settings.json"), CROSS_SESSION_INBOUND))
        .unwrap_or_else(|| claude_setting(CROSS_SESSION_INBOUND))
        .is_none()
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
