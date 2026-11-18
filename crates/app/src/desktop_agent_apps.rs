//! Detecção, integração e abertura de apps desktop do grafo.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopApp {
    ClaudeDesktop,
    Cursor,
    CodexDesktop,
}

impl DesktopApp {
    pub const ALL: [Self; 3] = [Self::ClaudeDesktop, Self::Cursor, Self::CodexDesktop];
    pub const fn id(self) -> &'static str {
        match self {
            Self::ClaudeDesktop => "claude-desktop",
            Self::Cursor => "cursor",
            Self::CodexDesktop => "codex-desktop",
        }
    }
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|app| app.id() == id)
    }
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClaudeDesktop => "Claude Desktop",
            Self::Cursor => "Cursor",
            Self::CodexDesktop => "Codex (ChatGPT)",
        }
    }
    pub const fn icon(self) -> &'static [u8] {
        match self {
            Self::ClaudeDesktop => include_bytes!("../assets/harness_icons/claudecode.svg"),
            Self::Cursor => include_bytes!("../assets/harness_icons/cursor.svg"),
            Self::CodexDesktop => include_bytes!("../assets/harness_icons/codex.svg"),
        }
    }
    pub fn config_path(self) -> Option<PathBuf> {
        let home = home_dir()?;
        Some(match self {
            Self::ClaudeDesktop => std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".config"))
                .join("Claude/claude_desktop_config.json"),
            Self::Cursor => home.join(".cursor/mcp.json"),
            Self::CodexDesktop => std::env::var_os("CODEX_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex"))
                .join("config.toml"),
        })
    }
    fn executable(self) -> &'static str {
        match self {
            Self::ClaudeDesktop => "claude-desktop",
            Self::Cursor => "cursor",
            Self::CodexDesktop => "chatgpt",
        }
    }
    fn desktop_files(self) -> &'static [&'static str] {
        match self {
            Self::ClaudeDesktop => &["com.anthropic.Claude.desktop", "claude-desktop.desktop"],
            Self::Cursor => &["cursor.desktop"],
            Self::CodexDesktop => &["chatgpt.desktop"],
        }
    }
    fn opens_folder(self) -> bool {
        matches!(self, Self::Cursor)
    }
    fn toml(self) -> bool {
        matches!(self, Self::CodexDesktop)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Integration {
    pub app: DesktopApp,
    pub config_path: PathBuf,
    pub cli_path: PathBuf,
}

impl Integration {
    pub fn entry(&self) -> String {
        let args = format!("[\"mcp\", \"--agent\", \"{}\"]", self.app.id());
        if self.app.toml() {
            format!("[mcp_servers.gsh]\ncommand = {}\nargs = {args}\nenv_vars = [\"GSH_TAB_ID\", \"GSH_STATUS_SOCK\", \"GSH_STATUS_PORT\", \"GSH_STATUS_TOKEN\"]", serde_json::to_string(&self.cli_path.to_string_lossy()).unwrap_or_default())
        } else {
            format!(
                "\"gsh\": {{\n  \"command\": {},\n  \"args\": {args}\n}}",
                serde_json::to_string(&self.cli_path.to_string_lossy()).unwrap_or_default()
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidConfig(PathBuf),
    CliMissing(PathBuf),
    WriteFailed(String),
}

pub fn detect() -> Vec<Integration> {
    let Some(cli_path) = find_cli() else {
        return Vec::new();
    };
    DesktopApp::ALL
        .into_iter()
        .filter_map(|app| {
            if !installed(app) {
                return None;
            }
            Some(Integration {
                app,
                config_path: app.config_path()?,
                cli_path: cli_path.clone(),
            })
        })
        .collect()
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn find_cli() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("GSH_CLI_PATH")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return Some(path);
    }
    if let Some(home) = home_dir() {
        let local = home.join(".gsh/bin/gsh");
        if local.is_file() {
            return Some(local);
        }
    }
    find_executable("gsh")
}

fn find_executable(name: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")?
        .to_string_lossy()
        .split(':')
        .map(Path::new)
        .map(|dir| dir.join(name))
        .find(|path| path.is_file())
}

fn installed(app: DesktopApp) -> bool {
    let executable = std::env::var_os("PATH")
        .map(|path| {
            path.to_string_lossy()
                .split(':')
                .any(|dir| Path::new(dir).join(app.executable()).is_file())
        })
        .unwrap_or(false);
    let mut dirs = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        PathBuf::from("/var/lib/flatpak/exports/share/applications"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/share"))
                .join("applications"),
        );
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }
    executable
        && dirs.iter().any(|dir| {
            app.desktop_files()
                .iter()
                .any(|file| dir.join(file).is_file())
        })
}

pub fn install(integration: &Integration) -> Result<(), Error> {
    if !integration.cli_path.is_file() {
        return Err(Error::CliMissing(integration.cli_path.clone()));
    }
    let path = &integration.config_path;
    let original = match fs::read_to_string(path) {
        Ok(value) => Some(value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(Error::WriteFailed(format!("{}: {error}", path.display()))),
    };
    let merged = if integration.app.toml() {
        merge_toml(original.as_deref(), integration)
    } else {
        merge_json(original.as_deref(), integration)
    }
    .map_err(|()| Error::InvalidConfig(path.clone()))?;
    if original.as_deref() == Some(merged.as_str()) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| Error::WriteFailed(format!("{}: {e}", parent.display())))?;
    }
    if path.exists() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let backup = path.with_file_name(format!(
            "{}.gsh-bak-{stamp}",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
        fs::copy(path, &backup)
            .map_err(|e| Error::WriteFailed(format!("{}: {e}", backup.display())))?;
        set_private(&backup);
    }
    write_atomic(path, merged.as_bytes())
        .map_err(|e| Error::WriteFailed(format!("{}: {e}", path.display())))?;
    Ok(())
}

fn merge_json(raw: Option<&str>, integration: &Integration) -> Result<String, ()> {
    let mut root: Value = match raw.filter(|text| !text.trim().is_empty()) {
        Some(raw) => serde_json::from_str(raw).map_err(|_| ())?,
        None => json!({}),
    };
    let object = root.as_object_mut().ok_or(())?;
    let servers = object
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(())?;
    let mut entry = Map::new();
    entry.insert(
        "command".into(),
        json!(integration.cli_path.to_string_lossy()),
    );
    entry.insert(
        "args".into(),
        json!(["mcp", "--agent", integration.app.id()]),
    );
    servers.insert("gsh".into(), Value::Object(entry));
    let mut output = serde_json::to_string_pretty(&root).map_err(|_| ())?;
    if raw.is_none_or(|text| text.ends_with('\n')) {
        output.push('\n');
    }
    Ok(output)
}

fn merge_toml(raw: Option<&str>, integration: &Integration) -> Result<String, ()> {
    let mut doc = raw
        .unwrap_or_default()
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| ())?;
    if doc.as_table().contains_key("mcp_servers.gsh") {
        return Err(());
    }
    if !doc.contains_key("mcp_servers") {
        doc.as_table_mut().insert(
            "mcp_servers",
            toml_edit::Item::Table(toml_edit::Table::new()),
        );
    }
    let servers = doc
        .get_mut("mcp_servers")
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or(())?;
    if !servers.contains_key("gsh") {
        servers.insert("gsh", toml_edit::Item::Table(toml_edit::Table::new()));
    }
    let entry = servers
        .get_mut("gsh")
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or(())?;
    entry.insert(
        "command",
        toml_edit::value(integration.cli_path.to_string_lossy().to_string()),
    );
    let mut args = toml_edit::Array::new();
    for value in ["mcp", "--agent", integration.app.id()] {
        args.push(value);
    }
    let mut env_vars = toml_edit::Array::new();
    for value in [
        "GSH_TAB_ID",
        "GSH_STATUS_SOCK",
        "GSH_STATUS_PORT",
        "GSH_STATUS_TOKEN",
    ] {
        env_vars.push(value);
    }
    entry.insert("args", toml_edit::value(args));
    entry.insert("env_vars", toml_edit::value(env_vars));
    Ok(doc.to_string())
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let tmp = path.with_file_name(format!(".{name}.tmp-{}", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }
    set_private(&tmp);
    if let Err(error) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error);
    }
    Ok(())
}

#[cfg(unix)]
fn set_private(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = fs::metadata(path) {
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o600);
        let _ = fs::set_permissions(path, permissions);
    }
}
#[cfg(not(unix))]
fn set_private(_: &Path) {}

pub fn launch(integration: &Integration, folder: &Path) {
    launch_app(integration.app, folder);
}

pub fn launch_app(app: DesktopApp, folder: &Path) {
    let mut command = Command::new(app.executable());
    if app.opens_folder() && folder.is_dir() {
        command.arg(folder);
    }
    command
        .env_remove("GSH_TAB_ID")
        .env_remove("GSH_STATUS_SOCK")
        .env_remove("GSH_STATUS_PORT")
        .env_remove("GSH_STATUS_TOKEN")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let _ = command.spawn();
}

#[cfg(test)]
mod tests {
    use super::*;
    fn integration(app: DesktopApp) -> Integration {
        Integration {
            app,
            config_path: PathBuf::from("config"),
            cli_path: PathBuf::from("/usr/bin/gsh"),
        }
    }
    #[test]
    fn json_merge_preserves_other_servers_and_adds_mcp_entry() {
        let result = merge_json(
            Some("{\"mcpServers\":{\"other\":{\"command\":\"x\"}}}\n"),
            &integration(DesktopApp::Cursor),
        )
        .expect("merge");
        let parsed: Value = serde_json::from_str(&result).expect("json");
        assert_eq!(parsed["mcpServers"]["other"]["command"], "x");
        assert_eq!(
            parsed["mcpServers"]["gsh"]["args"],
            json!(["mcp", "--agent", "cursor"])
        );
    }
    #[test]
    fn rejects_non_object_json_and_keeps_other_toml_mcp_servers() {
        assert!(merge_json(Some("[]"), &integration(DesktopApp::Cursor)).is_err());
        let result = merge_toml(
            Some("[mcp_servers.other]\ncommand='x'\n"),
            &integration(DesktopApp::CodexDesktop),
        )
        .expect("toml");
        let parsed = result
            .parse::<toml_edit::DocumentMut>()
            .expect("valid toml");
        assert_eq!(
            parsed["mcp_servers"]["other"]["command"].as_str(),
            Some("x")
        );
        assert_eq!(
            parsed["mcp_servers"]["gsh"]["args"]
                .as_array()
                .map(|array| array
                    .iter()
                    .filter_map(toml_edit::Value::as_str)
                    .collect::<Vec<_>>()),
            Some(vec!["mcp", "--agent", "codex-desktop"])
        );
    }
}
