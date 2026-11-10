//! Instalador de hooks do Claude Code (`claude_hook_installer_impl.dart` no Flutter).
//!
//! Fase 4: implementado como função pura sobre o conteúdo de `settings.json`
//! (recebe o caminho do `settings.json` e o comando do hook, e devolve o novo
//! JSON formatado sem gravar em `~/.claude/settings.json`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Map, Number, Value};
use thiserror::Error;

/// Eventos do Claude Code nos quais o GSH registra o hook de status.
pub const CLAUDE_HOOK_EVENTS: [&str; 8] = [
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "Notification",
    "Stop",
    "SessionStart",
    "SessionEnd",
    "PermissionRequest",
];

/// Chave marcadora gravada em cada entrada de hook do GSH.
pub const HOOK_MARKER_KEY: &str = "_gsh";

/// Versão atual do marcador de hook do GSH.
pub const HOOK_MARKER_VALUE: &str = "v1";

/// Evento que mantém o processo de hook bloqueado aguardando decisão na ilha.
pub const PERMISSION_REQUEST_EVENT: &str = "PermissionRequest";

/// Timeout (24h em segundos) exigido pelo Claude Code no `PermissionRequest`
/// para que o usuário possa aprovar ou negar na ilha sem expirar em 60s.
pub const PERMISSION_REQUEST_TIMEOUT_SECS: u64 = 86_400;

/// Erros possíveis ao preparar o `settings.json` do Claude Code.
#[derive(Debug, Error)]
pub enum ClaudeHookError {
    #[error("não foi possível ler {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("JSON inválido em {path}: {source}")]
    InvalidJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("o conteúdo de {path} não é um objeto JSON")]
    RootNotObject { path: PathBuf },
    #[error("falha ao serializar settings.json: {source}")]
    Serialize {
        #[source]
        source: serde_json::Error,
    },
}

/// Formata o comando do hook para o `settings.json` a partir do caminho do binário `gsh`.
///
/// Caminhos com espaços recebem aspas duplas ao redor do executável
/// (`"/caminho com espaco/gsh" hook`), espelhando `_resolveHookCommand` do Flutter.
pub fn format_claude_hook_command(cli_path: &Path) -> String {
    let display = cli_path.display().to_string();
    if display.contains(' ') {
        format!("\"{display}\" hook")
    } else {
        format!("{display} hook")
    }
}

/// Lê `settings_path` (se existir), injeta/atualiza os hooks do GSH para todos os
/// [`CLAUDE_HOOK_EVENTS`] de forma idempotente e devolve o novo conteúdo JSON.
///
/// **Nota:** Esta função **não** escreve em disco — devolve a `String` pronta
/// para que a camada superior decida quando persistir.
pub fn prepare_claude_settings(
    settings_path: &Path,
    hook_command: &str,
) -> Result<String, ClaudeHookError> {
    let existing = read_optional_settings(settings_path)?;
    build_claude_settings_json(existing.as_deref(), settings_path, hook_command)
}

/// Remove todos os hooks do GSH do `settings_path` (se existir) e devolve o
/// novo conteúdo JSON preservando hooks de outros plugins/usuário.
pub fn remove_claude_hooks(settings_path: &Path) -> Result<String, ClaudeHookError> {
    let existing = read_optional_settings(settings_path)?;
    let mut root = parse_root_object(existing.as_deref(), settings_path)?;

    if let Some(Value::Object(hooks_map)) = root.get_mut("hooks") {
        for event in CLAUDE_HOOK_EVENTS {
            let should_remove_key = match hooks_map.get_mut(event) {
                Some(Value::Array(list)) => {
                    list.retain(|item| !is_our_hook_group(item));
                    list.is_empty()
                }
                _ => false,
            };
            if should_remove_key {
                hooks_map.remove(event);
            }
        }
    }

    serialize_root(root)
}

/// Aplica a transformação pura do `settings.json` em memória.
pub fn build_claude_settings_json(
    existing_json: Option<&str>,
    source_path: &Path,
    hook_command: &str,
) -> Result<String, ClaudeHookError> {
    let mut root = parse_root_object(existing_json, source_path)?;

    let mut hooks_map = match root.remove("hooks") {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };

    for event in CLAUDE_HOOK_EVENTS {
        let mut event_list = match hooks_map.remove(event) {
            Some(Value::Array(list)) => list
                .into_iter()
                .filter(|item| !is_our_hook_group(item))
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };

        event_list.push(build_hook_group(event, hook_command));
        hooks_map.insert(event.to_string(), Value::Array(event_list));
    }

    root.insert("hooks".to_string(), Value::Object(hooks_map));
    serialize_root(root)
}

fn read_optional_settings(settings_path: &Path) -> Result<Option<String>, ClaudeHookError> {
    match fs::read_to_string(settings_path) {
        Ok(content) => Ok(Some(content)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(ClaudeHookError::Read {
            path: settings_path.to_path_buf(),
            source,
        }),
    }
}

fn parse_root_object(
    existing_json: Option<&str>,
    source_path: &Path,
) -> Result<Map<String, Value>, ClaudeHookError> {
    let Some(raw) = existing_json.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Map::new());
    };

    let value: Value =
        serde_json::from_str(raw).map_err(|source| ClaudeHookError::InvalidJson {
            path: source_path.to_path_buf(),
            source,
        })?;

    match value {
        Value::Object(map) => Ok(map),
        _ => Err(ClaudeHookError::RootNotObject {
            path: source_path.to_path_buf(),
        }),
    }
}

fn serialize_root(root: Map<String, Value>) -> Result<String, ClaudeHookError> {
    let mut out = serde_json::to_string_pretty(&Value::Object(root))
        .map_err(|source| ClaudeHookError::Serialize { source })?;
    out.push('\n');
    Ok(out)
}

fn build_hook_group(event: &str, hook_command: &str) -> Value {
    let mut hook_entry = Map::new();
    hook_entry.insert("type".to_string(), Value::String("command".to_string()));
    hook_entry.insert(
        "command".to_string(),
        Value::String(hook_command.to_string()),
    );
    hook_entry.insert(
        HOOK_MARKER_KEY.to_string(),
        Value::String(HOOK_MARKER_VALUE.to_string()),
    );
    if event == PERMISSION_REQUEST_EVENT {
        hook_entry.insert(
            "timeout".to_string(),
            Value::Number(Number::from(PERMISSION_REQUEST_TIMEOUT_SECS)),
        );
    }

    let mut group = Map::new();
    group.insert(
        "hooks".to_string(),
        Value::Array(vec![Value::Object(hook_entry)]),
    );
    Value::Object(group)
}

/// Verifica se um matcher group pertence ao GSH (`_isOurHook` no Flutter).
///
/// Reconhece:
/// 1. Entradas marcadas com `"_gsh": "v1"`.
/// 2. Entradas legadas cujo `command` contém `"gsh-hook"`.
/// 3. Entradas cujo `command` invoca `gsh hook` ou `gsh.exe hook` (com ou sem aspas
///    e barras POSIX/Windows).
pub fn is_our_hook_group(item: &Value) -> bool {
    let Some(hooks) = item
        .as_object()
        .and_then(|o| o.get("hooks"))
        .and_then(Value::as_array)
    else {
        return false;
    };

    hooks.iter().any(|hook| {
        let Some(obj) = hook.as_object() else {
            return false;
        };
        if obj.get(HOOK_MARKER_KEY).and_then(Value::as_str) == Some(HOOK_MARKER_VALUE) {
            return true;
        }
        if let Some(cmd) = obj.get("command").and_then(Value::as_str) {
            if cmd.contains("gsh-hook") || matches_legacy_gsh_hook_cmd(cmd) {
                return true;
            }
        }
        false
    })
}

/// Equivalente exato a `RegExp(r'(?:^|/)gsh(?:\.exe)?"?\s+hook(?:\s|$)')`
/// sobre `cmd.trim().replace('\\', "/").to_ascii_lowercase()`.
fn matches_legacy_gsh_hook_cmd(cmd: &str) -> bool {
    let normalized = cmd.trim().replace('\\', "/").to_ascii_lowercase();
    for (idx, _) in normalized.match_indices("gsh") {
        let valid_boundary =
            idx == 0 || normalized.as_bytes().get(idx.wrapping_sub(1)) == Some(&b'/');
        if !valid_boundary {
            continue;
        }
        let Some(after_gsh) = normalized.get(idx..).and_then(|s| s.strip_prefix("gsh")) else {
            continue;
        };
        let after_exe = after_gsh.strip_prefix(".exe").unwrap_or(after_gsh);
        let after_quote = after_exe.strip_prefix('"').unwrap_or(after_exe);
        let after_ws = after_quote.trim_start_matches(char::is_whitespace);
        if after_ws.len() == after_quote.len() {
            continue;
        }
        let Some(after_hook) = after_ws.strip_prefix("hook") else {
            continue;
        };
        if after_hook.is_empty() || after_hook.starts_with(char::is_whitespace) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;

    static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

    struct TempSettingsDir {
        path: PathBuf,
    }

    impl TempSettingsDir {
        fn new() -> Self {
            let id = TEMP_SEQ.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "gsh-claude-hook-test-{}-{}",
                std::process::id(),
                id
            ));
            fs::create_dir_all(&path).expect("create temp test dir");
            Self { path }
        }

        fn settings_path(&self) -> PathBuf {
            self.path.join("settings.json")
        }
    }

    impl Drop for TempSettingsDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn cria_settings_do_zero_quando_arquivo_nao_existe_sem_gravar_em_disco() {
        let tmp = TempSettingsDir::new();
        let settings_path = tmp.settings_path();

        let output = prepare_claude_settings(&settings_path, "/usr/local/bin/gsh hook").unwrap();

        // Garante que a função pura NÃO escreveu o arquivo no disco.
        assert!(!settings_path.exists());

        let parsed: Value = serde_json::from_str(&output).unwrap();
        let hooks = parsed["hooks"].as_object().unwrap();

        for event in CLAUDE_HOOK_EVENTS {
            let arr = hooks[event].as_array().unwrap();
            assert_eq!(arr.len(), 1, "evento {event} deve ter exatamente 1 grupo");
            let entry = &arr[0]["hooks"][0];
            assert_eq!(entry["type"], "command");
            assert_eq!(entry["command"], "/usr/local/bin/gsh hook");
            assert_eq!(entry["_gsh"], "v1");
            if event == PERMISSION_REQUEST_EVENT {
                assert_eq!(entry["timeout"], 86_400);
            } else {
                assert!(entry.get("timeout").is_none());
            }
        }
    }

    #[test]
    fn preserva_chaves_existentes_e_hooks_de_outros_plugins_e_eh_idempotente() {
        let tmp = TempSettingsDir::new();
        let settings_path = tmp.settings_path();

        let initial = serde_json::json!({
            "theme": "dark",
            "permissions": {
                "allow": ["Bash(cargo test:*)"]
            },
            "hooks": {
                "PreToolUse": [
                    {
                        "matcher": "Write",
                        "hooks": [
                            { "type": "command", "command": "/usr/local/bin/other-tool check" }
                        ]
                    }
                ]
            }
        });
        fs::write(
            &settings_path,
            serde_json::to_string_pretty(&initial).unwrap(),
        )
        .unwrap();

        let first = prepare_claude_settings(&settings_path, "gsh hook").unwrap();
        fs::write(&settings_path, &first).unwrap();

        // Segunda execução com comando atualizado não duplica a entrada do GSH.
        let second = prepare_claude_settings(&settings_path, "/opt/gsh/bin/gsh hook").unwrap();
        let parsed: Value = serde_json::from_str(&second).unwrap();

        assert_eq!(parsed["theme"], "dark");
        assert_eq!(parsed["permissions"]["allow"][0], "Bash(cargo test:*)");

        let pre_tool = parsed["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre_tool.len(), 2);
        assert_eq!(
            pre_tool[0]["hooks"][0]["command"],
            "/usr/local/bin/other-tool check"
        );
        assert_eq!(pre_tool[1]["hooks"][0]["command"], "/opt/gsh/bin/gsh hook");
        assert_eq!(pre_tool[1]["hooks"][0]["_gsh"], "v1");
    }

    #[test]
    fn substitui_entradas_legadas_windows_e_posix_sem_marcador_gsh() {
        let tmp = TempSettingsDir::new();
        let settings_path = tmp.settings_path();

        let legacy = serde_json::json!({
            "hooks": {
                "UserPromptSubmit": [
                    {
                        "hooks": [
                            { "type": "command", "command": "\"C:\\Program Files\\GSH\\gsh.exe\" hook" }
                        ]
                    },
                    {
                        "hooks": [
                            { "type": "command", "command": "other-tool hook" }
                        ]
                    }
                ],
                "Stop": [
                    {
                        "hooks": [
                            { "type": "command", "command": "C:\\Users\\me\\AppData\\Local\\GSH\\gsh.exe hook" }
                        ]
                    },
                    {
                        "hooks": [
                            { "type": "command", "command": "/home/me/.gsh/bin/gsh-hook" }
                        ]
                    }
                ]
            }
        });
        fs::write(&settings_path, legacy.to_string()).unwrap();

        let updated = prepare_claude_settings(&settings_path, "gsh hook").unwrap();
        let parsed: Value = serde_json::from_str(&updated).unwrap();

        let prompt_hooks = parsed["hooks"]["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(prompt_hooks.len(), 2);
        assert_eq!(prompt_hooks[0]["hooks"][0]["command"], "other-tool hook");
        assert_eq!(prompt_hooks[1]["hooks"][0]["command"], "gsh hook");
        assert_eq!(prompt_hooks[1]["hooks"][0]["_gsh"], "v1");

        let stop_hooks = parsed["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_hooks.len(), 1);
        assert_eq!(stop_hooks[0]["hooks"][0]["command"], "gsh hook");
    }

    #[test]
    fn remove_claude_hooks_limpa_apenas_grupos_do_gsh() {
        let tmp = TempSettingsDir::new();
        let settings_path = tmp.settings_path();

        let installed = build_claude_settings_json(
            Some(
                &serde_json::json!({
                    "hooks": {
                        "PreToolUse": [
                            {
                                "hooks": [
                                    { "type": "command", "command": "custom-linter --strict" }
                                ]
                            }
                        ]
                    }
                })
                .to_string(),
            ),
            &settings_path,
            "gsh hook",
        )
        .unwrap();
        fs::write(&settings_path, installed).unwrap();

        let cleaned = remove_claude_hooks(&settings_path).unwrap();
        let parsed: Value = serde_json::from_str(&cleaned).unwrap();
        let hooks = parsed["hooks"].as_object().unwrap();

        assert_eq!(hooks.len(), 1);
        let pre_tool = hooks["PreToolUse"].as_array().unwrap();
        assert_eq!(pre_tool.len(), 1);
        assert_eq!(pre_tool[0]["hooks"][0]["command"], "custom-linter --strict");
    }

    #[test]
    fn formata_comando_com_aspas_quando_caminho_tem_espacos() {
        assert_eq!(
            format_claude_hook_command(Path::new("/usr/local/bin/gsh")),
            "/usr/local/bin/gsh hook"
        );
        assert_eq!(
            format_claude_hook_command(Path::new("/opt/GSH Desktop/bin/gsh")),
            "\"/opt/GSH Desktop/bin/gsh\" hook"
        );
    }
}
