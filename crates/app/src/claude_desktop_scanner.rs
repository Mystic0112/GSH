//! Varredura segura e somente leitura de extensões e servidores MCP do Claude Desktop.
//!
//! Este módulo inspeciona `claude_desktop_config.json` e o diretório `extensions`
//! (pacotes `.mcpb` e `.dxt`) sem jamais executar comandos, instalar pacotes
//! automaticamente ou expor valores de variáveis de ambiente (`env`).

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Tipo de extensão ou servidor descoberto na instalação do Claude Desktop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveredKind {
    McpStdio,
    McpHttp,
    DesktopBundle,
}

/// Veredito da análise estática de segurança da extensão descoberta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityVerdict {
    SafeToReview,
    Blocked(String),
}

impl SecurityVerdict {
    /// Retorna `true` se o item passou na triagem inicial e pode ser revisado pelo usuário.
    pub fn is_safe_to_review(&self) -> bool {
        matches!(self, Self::SafeToReview)
    }
}

/// Extensão ou servidor MCP descoberto no Claude Desktop, higienizado sem segredos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredExtension {
    pub name: String,
    pub kind: DiscoveredKind,
    pub command_or_url: String,
    pub args: Vec<String>,
    /// Apenas os **nomes** das chaves de `env` — os valores nunca são lidos ou armazenados.
    pub env_keys: Vec<String>,
    pub verdict: SecurityVerdict,
}

/// Caminhos relativos (a partir de `home`) onde o Claude Desktop armazena `claude_desktop_config.json`.
const CONFIG_RELATIVE_PATHS: [&str; 2] = [
    ".config/Claude/claude_desktop_config.json",
    "Library/Application Support/Claude/claude_desktop_config.json",
];

/// Caminhos relativos (a partir de `home`) onde o Claude Desktop armazena pacotes `.mcpb`/`.dxt`.
const EXTENSIONS_RELATIVE_DIRS: [&str; 2] = [
    ".config/Claude/extensions",
    "Library/Application Support/Claude/extensions",
];

/// Analisa o conteúdo JSON de `claude_desktop_config.json` em modo somente leitura,
/// extraindo servidores de `mcpServers` sem expor os valores de `env`.
pub fn parse_claude_desktop_config(raw_json: &str) -> Result<Vec<DiscoveredExtension>, String> {
    let root: Value = serde_json::from_str(raw_json)
        .map_err(|err| format!("JSON inválido em claude_desktop_config.json: {err}"))?;

    let root_obj = root.as_object().ok_or_else(|| {
        "a raiz de claude_desktop_config.json deve ser um objeto JSON".to_string()
    })?;

    let Some(mcp_servers_val) = root_obj.get("mcpServers") else {
        return Ok(Vec::new());
    };

    let mcp_servers = mcp_servers_val
        .as_object()
        .ok_or_else(|| "o campo 'mcpServers' deve ser um objeto JSON".to_string())?;

    let mut discovered = Vec::with_capacity(mcp_servers.len());

    for (name, server_val) in mcp_servers {
        let Some(server_obj) = server_val.as_object() else {
            discovered.push(DiscoveredExtension {
                name: name.clone(),
                kind: DiscoveredKind::McpStdio,
                command_or_url: String::new(),
                args: Vec::new(),
                env_keys: Vec::new(),
                verdict: SecurityVerdict::Blocked(
                    "configuração do servidor MCP não é um objeto JSON".to_string(),
                ),
            });
            continue;
        };

        let url_opt = server_obj.get("url").and_then(Value::as_str);
        let cmd_opt = server_obj.get("command").and_then(Value::as_str);

        let (kind, command_or_url) = match (url_opt, cmd_opt) {
            (Some(url), _) => (DiscoveredKind::McpHttp, url.to_string()),
            (None, Some(cmd)) => (DiscoveredKind::McpStdio, cmd.to_string()),
            (None, None) => (DiscoveredKind::McpStdio, String::new()),
        };

        let mut args = Vec::new();
        let mut invalid_args = false;
        if let Some(args_val) = server_obj.get("args") {
            if let Some(arr) = args_val.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        args.push(s.to_string());
                    } else {
                        invalid_args = true;
                    }
                }
            } else {
                invalid_args = true;
            }
        }

        // Extrai SOMENTE os nomes das chaves de `env`, descartando completamente os valores.
        let mut env_keys = Vec::new();
        let mut invalid_env = false;
        if let Some(env_val) = server_obj.get("env") {
            if let Some(env_obj) = env_val.as_object() {
                env_keys.extend(env_obj.keys().cloned());
            } else {
                invalid_env = true;
            }
        }

        let verdict = evaluate_security_verdict(
            name,
            &kind,
            &command_or_url,
            &args,
            &env_keys,
            invalid_args,
            invalid_env,
        );

        discovered.push(DiscoveredExtension {
            name: name.clone(),
            kind,
            command_or_url,
            args,
            env_keys,
            verdict,
        });
    }

    Ok(discovered)
}

/// Varre os caminhos padrões do Claude Desktop dentro de `home` (Linux e macOS)
/// em modo somente leitura, coletando servidores MCP e pacotes `.mcpb`/`.dxt`.
pub fn scan_home(home: &Path) -> Vec<DiscoveredExtension> {
    let mut results = Vec::new();

    for rel_config in CONFIG_RELATIVE_PATHS {
        let config_path = home.join(rel_config);
        if !config_path.exists() {
            continue;
        }
        if is_symlink(&config_path) {
            results.push(DiscoveredExtension {
                name: rel_config.to_string(),
                kind: DiscoveredKind::McpStdio,
                command_or_url: config_path.to_string_lossy().into_owned(),
                args: Vec::new(),
                env_keys: Vec::new(),
                verdict: SecurityVerdict::Blocked(
                    "arquivo de configuração é um link simbólico".to_string(),
                ),
            });
            continue;
        }
        if let Ok(raw_json) = fs::read_to_string(&config_path) {
            if let Ok(mut parsed) = parse_claude_desktop_config(&raw_json) {
                results.append(&mut parsed);
            }
        }
    }

    for rel_dir in EXTENSIONS_RELATIVE_DIRS {
        let ext_dir = home.join(rel_dir);
        let Ok(read_dir) = fs::read_dir(&ext_dir) else {
            continue;
        };

        let mut bundle_paths: Vec<PathBuf> = read_dir
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| is_desktop_bundle_extension(path))
            .collect();
        bundle_paths.sort();

        for path in bundle_paths {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or_default()
                .to_string();
            let path_str = path.to_string_lossy().into_owned();

            let verdict = if is_symlink(&path) {
                SecurityVerdict::Blocked("pacote de extensão é um link simbólico".to_string())
            } else {
                evaluate_security_verdict(
                    &name,
                    &DiscoveredKind::DesktopBundle,
                    &path_str,
                    &[],
                    &[],
                    false,
                    false,
                )
            };

            results.push(DiscoveredExtension {
                name,
                kind: DiscoveredKind::DesktopBundle,
                command_or_url: path_str,
                args: Vec::new(),
                env_keys: Vec::new(),
                verdict,
            });
        }
    }

    results
}

/// Gera o comando `claude mcp add` correspondente **somente** se a extensão tiver
/// veredito [`SecurityVerdict::SafeToReview`] e for [`DiscoveredKind::McpStdio`] ou
/// [`DiscoveredKind::McpHttp`], escapando cada argumento com aspas simples seguras.
pub fn approved_claude_mcp_add_command(ext: &DiscoveredExtension) -> Result<String, String> {
    if let SecurityVerdict::Blocked(reason) = &ext.verdict {
        return Err(format!(
            "extensão '{}' bloqueada pela revisão de segurança: {reason}",
            ext.name
        ));
    }

    if ext.kind == DiscoveredKind::DesktopBundle {
        return Err(format!(
            "extensão '{}' é um pacote DesktopBundle (.mcpb/.dxt) e não pode ser adicionada via 'claude mcp add'",
            ext.name
        ));
    }

    // Revalidação em profundidade antes de montar a linha de comando.
    if let SecurityVerdict::Blocked(reason) = evaluate_security_verdict(
        &ext.name,
        &ext.kind,
        &ext.command_or_url,
        &ext.args,
        &ext.env_keys,
        false,
        false,
    ) {
        return Err(format!(
            "extensão '{}' reprovada na validação de segurança: {reason}",
            ext.name
        ));
    }

    let quoted_name = shell_single_quote(&ext.name);
    let quoted_target = shell_single_quote(&ext.command_or_url);

    match ext.kind {
        DiscoveredKind::McpHttp => Ok(format!(
            "claude mcp add --transport http {quoted_name} {quoted_target}"
        )),
        DiscoveredKind::McpStdio => {
            let mut cmd = format!("claude mcp add {quoted_name} -- {quoted_target}");
            for arg in &ext.args {
                cmd.push(' ');
                cmd.push_str(&shell_single_quote(arg));
            }
            Ok(cmd)
        }
        DiscoveredKind::DesktopBundle => Err(
            "pacotes DesktopBundle não suportam geração de comando 'claude mcp add'".to_string(),
        ),
    }
}

fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

fn is_desktop_bundle_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mcpb") || ext.eq_ignore_ascii_case("dxt"))
}

fn shell_single_quote(value: &str) -> String {
    let escaped = value.replace('\'', "'\\''");
    format!("'{escaped}'")
}

fn evaluate_security_verdict(
    name: &str,
    kind: &DiscoveredKind,
    command_or_url: &str,
    args: &[String],
    env_keys: &[String],
    invalid_args: bool,
    invalid_env: bool,
) -> SecurityVerdict {
    if name.trim().is_empty() {
        return SecurityVerdict::Blocked("nome da extensão está vazio".to_string());
    }
    if let Some(meta) = find_shell_injection_meta(name) {
        return SecurityVerdict::Blocked(format!(
            "nome da extensão contém metacaractere perigoso ({meta})"
        ));
    }
    if contains_path_traversal(name) {
        return SecurityVerdict::Blocked(
            "nome da extensão contém sequência de path traversal ('..')".to_string(),
        );
    }

    if invalid_args {
        return SecurityVerdict::Blocked(
            "campo 'args' inválido: todos os argumentos devem ser strings".to_string(),
        );
    }

    if invalid_env {
        return SecurityVerdict::Blocked(
            "campo 'env' inválido: deve ser um objeto JSON de variáveis".to_string(),
        );
    }

    for key in env_keys {
        if !is_valid_env_key(key) {
            return SecurityVerdict::Blocked(format!(
                "nome de variável de ambiente inválido: '{key}'"
            ));
        }
    }

    if command_or_url.trim().is_empty() {
        return SecurityVerdict::Blocked("comando ou URL ausente".to_string());
    }

    if let Some(meta) = find_shell_injection_meta(command_or_url) {
        return SecurityVerdict::Blocked(format!(
            "comando ou URL contém metacaractere perigoso de shell injection ({meta})"
        ));
    }

    if contains_path_traversal(command_or_url) {
        return SecurityVerdict::Blocked(
            "comando ou URL contém caminho com path traversal ('..')".to_string(),
        );
    }

    if *kind == DiscoveredKind::McpHttp && !is_allowed_mcp_url(command_or_url) {
        return SecurityVerdict::Blocked(
            "URL não permitida: apenas https://, http://127.0.0.1 ou http://localhost são aceitos"
                .to_string(),
        );
    }

    for arg in args {
        if let Some(meta) = find_shell_injection_meta(arg) {
            return SecurityVerdict::Blocked(format!(
                "argumento contém metacaractere perigoso de shell injection ({meta})"
            ));
        }
        if contains_path_traversal(arg) {
            return SecurityVerdict::Blocked(
                "argumento contém caminho com path traversal ('..')".to_string(),
            );
        }
    }

    SecurityVerdict::SafeToReview
}

fn find_shell_injection_meta(input: &str) -> Option<&'static str> {
    const PATTERNS: [&str; 10] = [";", "|", "&&", "`", "$(", "\n", "\r", "\0", ">", "<"];
    PATTERNS
        .into_iter()
        .find(|&pattern| input.contains(pattern))
}

fn contains_path_traversal(input: &str) -> bool {
    input.contains("..")
}

fn is_valid_env_key(key: &str) -> bool {
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_allowed_mcp_url(url: &str) -> bool {
    if url.chars().any(char::is_whitespace) {
        return false;
    }

    if let Some(rest) = url.strip_prefix("https://") {
        let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
        if authority.is_empty()
            || authority.contains('@')
            || authority.starts_with('.')
            || authority.starts_with(':')
        {
            return false;
        }
        return true;
    }

    for loopback in ["http://127.0.0.1", "http://localhost"] {
        if let Some(rest) = url.strip_prefix(loopback) {
            return rest.is_empty()
                || rest.starts_with(':')
                || rest.starts_with('/')
                || rest.starts_with('?')
                || rest.starts_with('#');
        }
    }

    false
}

#[cfg(test)]
#[path = "claude_desktop_scanner_tests.rs"]
mod tests;
