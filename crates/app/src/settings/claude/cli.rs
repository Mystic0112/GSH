//! A CLI `claude` instalada (`ClaudeCliInfoSource` e `claude_info_parsers.dart` do Flutter):
//! conta, logout, limites de uso (`get_usage`), servidores MCP e plugins/marketplaces.

use super::{files, profiles};
use crate::labels;
use gsh_harness::Json;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// O que deu errado ao consultar a CLI. Sem frase: a tela traduz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InfoError {
    CliMissing,
    Timeout,
    Failed(Option<String>),
}

/// `claude auth status --json`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Account {
    pub logged_in: bool,
    pub auth_method: Option<String>,
    pub api_provider: Option<String>,
    pub email: Option<String>,
    pub org_id: Option<String>,
    pub org_name: Option<String>,
    pub subscription_type: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageWindowKind {
    Session,
    Weekly,
    WeeklyModel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageWindow {
    pub kind: UsageWindowKind,
    pub label: Option<String>,
    pub percent: Option<f64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Usage {
    pub available: bool,
    pub subscription_type: Option<String>,
    pub windows: Vec<UsageWindow>,
    pub extra: Option<ExtraUsage>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExtraUsage {
    pub enabled: bool,
    pub percent: Option<f64>,
    pub spend_limit_reached: bool,
    pub resets_at: Option<i64>,
    pub amounts: Option<ExtraUsageAmounts>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtraUsageAmounts {
    pub currency: String,
    pub used_minor: Option<u64>,
    pub limit_minor: Option<u64>,
    decimal_places: u8,
}

impl ExtraUsageAmounts {
    pub fn format_minor(&self, minor: u64) -> String {
        let digits = usize::from(self.decimal_places);
        if digits == 0 {
            return format!("{minor} {}", self.currency);
        }
        let scale = 10_u64.pow(u32::from(self.decimal_places));
        format!(
            "{},{:0digits$} {}",
            minor / scale,
            minor % scale,
            self.currency
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpStatus {
    Connected,
    NeedsAuth,
    Failed,
    Pending,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpOrigin {
    Local,
    ClaudeAi,
    Plugin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerInfo {
    pub name: String,
    pub target: String,
    pub status: McpStatus,
    pub status_text: String,
}

impl McpServerInfo {
    pub fn origin(&self) -> McpOrigin {
        if self.name.starts_with("claude.ai ") {
            McpOrigin::ClaudeAi
        } else if self.name.starts_with("plugin:") {
            McpOrigin::Plugin
        } else {
            McpOrigin::Local
        }
    }

    pub fn display_name(&self) -> &str {
        match self.origin() {
            McpOrigin::ClaudeAi => self.name.strip_prefix("claude.ai ").unwrap_or(&self.name),
            McpOrigin::Plugin => self.name.rsplit(':').next().unwrap_or(&self.name),
            McpOrigin::Local => &self.name,
        }
    }

    pub fn plugin_name(&self) -> Option<&str> {
        if self.origin() == McpOrigin::Plugin {
            self.name.split(':').nth(1)
        } else {
            None
        }
    }

    pub fn is_remote(&self) -> bool {
        self.target.starts_with("http://") || self.target.starts_with("https://")
    }

    pub fn command_line(&self) -> (&str, &str) {
        match self.target.find(' ') {
            Some(i) => (
                self.target.get(..i).unwrap_or(&self.target),
                self.target.get(i + 1..).unwrap_or(""),
            ),
            None => (&self.target, ""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInfo {
    pub id: String,
    pub enabled: bool,
    pub version: Option<String>,
    pub scope: Option<String>,
    pub install_path: Option<String>,
    pub updated_at: Option<i64>,
    pub display_name: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
}

impl PluginInfo {
    pub fn name(&self) -> &str {
        self.display_name
            .as_deref()
            .unwrap_or_else(|| self.id.split('@').next().unwrap_or(&self.id))
    }

    pub fn marketplace(&self) -> &str {
        if self.id.contains('@') {
            self.id.rsplit('@').next().unwrap_or("")
        } else {
            ""
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarketplacePlugin {
    pub name: String,
    pub marketplace: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub category: Option<String>,
    pub installed: bool,
}

impl MarketplacePlugin {
    pub fn id(&self) -> String {
        format!("{}@{}", self.name, self.marketplace)
    }
}

/// `{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","email":"…",
/// "orgId":"…","orgName":"…","subscriptionType":"team",…}`.
pub fn parse_auth_status(stdout: &str) -> Result<Account, InfoError> {
    let root = Json::parse(stdout.trim())
        .map_err(|e| InfoError::Failed(Some(format!("auth status: {e}"))))?;
    if root.as_obj().is_none() {
        return Err(InfoError::Failed(Some(String::from(
            "auth status: not an object",
        ))));
    }
    let text = |key: &str| root.get(key).and_then(Json::as_str).map(str::to_owned);
    Ok(Account {
        logged_in: root.get("loggedIn") == Some(&Json::Bool(true)),
        auth_method: text("authMethod"),
        api_provider: text("apiProvider"),
        email: text("email"),
        org_id: text("orgId"),
        org_name: text("orgName"),
        subscription_type: text("subscriptionType"),
    })
}

/// Corpo (`response.response`) da resposta ao `control_request` `get_usage`.
/// Aceita tanto `rate_limits: {five_hour, seven_day, model_scoped}` (`claude_info_parsers.dart`)
/// quanto `limits: [{kind, percent, resets_at, scope}]` (`claude_account.dart`).
pub fn parse_usage_reply(body: &Json) -> Usage {
    let mut windows = Vec::new();
    let add_window = |raw: Option<&Json>,
                      kind: UsageWindowKind,
                      label: Option<String>,
                      out: &mut Vec<UsageWindow>| {
        let Some(node) = raw.filter(|n| n.as_obj().is_some()) else {
            return;
        };
        let percent = match node.get("utilization").or_else(|| node.get("percent")) {
            Some(Json::Num(n)) => n.as_f64(),
            _ => None,
        };
        let resets_at = node
            .get("resets_at")
            .and_then(Json::as_str)
            .and_then(parse_iso8601);
        out.push(UsageWindow {
            kind,
            label,
            percent,
            resets_at,
        });
    };
    let rate_limits = body.get("rate_limits");
    let limits = body
        .get("limits")
        .and_then(Json::as_arr)
        .filter(|list| !list.is_empty())
        .or_else(|| {
            rate_limits
                .and_then(|rate| rate.get("limits"))
                .and_then(Json::as_arr)
        });
    if let Some(list) = limits {
        for item in list {
            if item.as_obj().is_none() {
                continue;
            }
            let model = item
                .get("scope")
                .and_then(|s| s.get("model"))
                .and_then(|m| m.get("display_name"))
                .and_then(Json::as_str)
                .map(str::to_owned);
            let kind = match item.get("kind").and_then(Json::as_str) {
                Some("session") => Some((UsageWindowKind::Session, None)),
                Some("weekly_all") => Some((UsageWindowKind::Weekly, None)),
                Some("weekly_scoped") if model.is_some() => {
                    Some((UsageWindowKind::WeeklyModel, model))
                }
                _ => None,
            };
            if let Some((kind, label)) = kind {
                add_window(Some(item), kind, label, &mut windows);
            }
        }
    }
    if windows.is_empty() {
        if let Some(limits) = rate_limits {
            add_window(
                limits.get("five_hour"),
                UsageWindowKind::Session,
                None,
                &mut windows,
            );
            add_window(
                limits.get("seven_day"),
                UsageWindowKind::Weekly,
                None,
                &mut windows,
            );
            if let Some(scoped) = limits.get("model_scoped").and_then(Json::as_arr) {
                for item in scoped {
                    let name = item
                        .get("display_name")
                        .and_then(Json::as_str)
                        .map(str::to_owned);
                    add_window(Some(item), UsageWindowKind::WeeklyModel, name, &mut windows);
                }
            }
        }
    }
    let available =
        body.get("rate_limits_available") == Some(&Json::Bool(true)) || !windows.is_empty();
    let extra = rate_limits
        .and_then(|rate| rate.get("extra_usage"))
        .and_then(|extra| parse_extra_usage(extra, limits.unwrap_or_default()));
    Usage {
        available,
        subscription_type: body
            .get("subscription_type")
            .and_then(Json::as_str)
            .map(str::to_owned),
        windows,
        extra,
    }
}

fn parse_extra_usage(extra: &Json, limits: &[Json]) -> Option<ExtraUsage> {
    let Json::Bool(enabled) = extra.get("is_enabled")? else {
        return None;
    };
    let percent = match extra.get("utilization") {
        Some(Json::Num(n)) => n.as_f64().filter(|percent| *percent >= 0.0),
        _ => None,
    };
    let resets_at = limits
        .iter()
        .find(|row| {
            row.get("kind").and_then(Json::as_str) == Some("spend")
                && row.get("is_active") == Some(&Json::Bool(true))
        })
        .and_then(|row| row.get("resets_at"))
        .and_then(Json::as_str)
        .and_then(parse_iso8601);
    Some(ExtraUsage {
        enabled: *enabled,
        percent,
        spend_limit_reached: extra.get("spend_limit_reached") == Some(&Json::Bool(true)),
        resets_at,
        amounts: parse_extra_amounts(extra),
    })
}

fn parse_extra_amounts(extra: &Json) -> Option<ExtraUsageAmounts> {
    let currency = extra.get("currency")?.as_str()?;
    if currency.len() != 3 || !currency.bytes().all(|ch| ch.is_ascii_alphabetic()) {
        return None;
    }
    let amount_at = |key| match extra.get(key) {
        Some(Json::Num(number)) => number.as_u64(),
        _ => None,
    };
    // The Claude CLI's documented payload expresses both amounts in cents.
    let decimal_places = match extra.non_null("decimal_places") {
        None => 2,
        Some(Json::Num(n)) => n.as_u64().filter(|digits| *digits <= 6)? as u8,
        _ => return None,
    };
    let used_minor = amount_at("used_credits");
    let limit_minor = amount_at("monthly_limit");
    (used_minor.is_some() || limit_minor.is_some()).then(|| ExtraUsageAmounts {
        currency: currency.to_ascii_uppercase(),
        used_minor,
        limit_minor,
        decimal_places,
    })
}

/// `claude mcp list` (uma linha por servidor: `<nome>: <url ou comando> - <glifo> <status>`).
pub fn parse_mcp_list(stdout: &str) -> Vec<McpServerInfo> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let Some(dash) = line.rfind(" - ") else {
            continue;
        };
        let Some(colon) = line.find(": ") else {
            continue;
        };
        if colon > dash {
            continue;
        }
        let raw = line.get(dash + 3..).unwrap_or("").trim();
        let status_text = match raw.find(' ') {
            Some(space)
                if space > 0 && raw.get(..space).is_some_and(|g| g.chars().count() <= 2) =>
            {
                raw.get(space + 1..).unwrap_or(raw)
            }
            _ => raw,
        };
        let name = line.get(..colon).unwrap_or("").trim().to_owned();
        let target = line.get(colon + 2..dash).unwrap_or("").trim().to_owned();
        out.push(McpServerInfo {
            name,
            target,
            status: mcp_status(status_text),
            status_text: status_text.to_owned(),
        });
    }
    out
}

fn mcp_status(text: &str) -> McpStatus {
    let lower = text.to_lowercase();
    if lower.starts_with("connected") {
        McpStatus::Connected
    } else if lower.contains("needs authentication") {
        McpStatus::NeedsAuth
    } else if lower.contains("failed") {
        McpStatus::Failed
    } else if lower.contains("pending") {
        McpStatus::Pending
    } else {
        McpStatus::Unknown
    }
}

/// `claude plugin list --json`.
pub fn parse_plugin_list(stdout: &str) -> Result<Vec<PluginInfo>, InfoError> {
    let root = Json::parse(stdout.trim())
        .map_err(|e| InfoError::Failed(Some(format!("plugin list: {e}"))))?;
    let Some(items) = root.as_arr() else {
        return Err(InfoError::Failed(Some(String::from(
            "plugin list: not an array",
        ))));
    };
    let mut out = Vec::new();
    for item in items {
        let Some(id) = item.get("id").and_then(Json::as_str) else {
            continue;
        };
        let str_at = |k: &str| item.get(k).and_then(Json::as_str).map(str::to_owned);
        out.push(PluginInfo {
            id: id.to_owned(),
            enabled: item.get("enabled") == Some(&Json::Bool(true)),
            version: str_at("version"),
            scope: str_at("scope"),
            install_path: str_at("installPath"),
            updated_at: item
                .get("lastUpdated")
                .and_then(Json::as_str)
                .and_then(parse_iso8601),
            display_name: None,
            author: None,
            description: None,
        });
    }
    Ok(out)
}

/// `claude plugin marketplace list --json` -> nome -> pasta local.
pub fn parse_marketplace_list(stdout: &str) -> Result<BTreeMap<String, String>, InfoError> {
    let root = Json::parse(stdout.trim())
        .map_err(|e| InfoError::Failed(Some(format!("marketplace list: {e}"))))?;
    let Some(items) = root.as_arr() else {
        return Err(InfoError::Failed(Some(String::from(
            "marketplace list: not an array",
        ))));
    };
    let mut out = BTreeMap::new();
    for item in items {
        if let (Some(name), Some(loc)) = (
            item.get("name").and_then(Json::as_str),
            item.get("installLocation").and_then(Json::as_str),
        ) {
            out.insert(name.to_owned(), loc.to_owned());
        }
    }
    Ok(out)
}

pub fn account(home: &Path, profile: Option<&Path>) -> Result<Account, InfoError> {
    match run_with_profile(home, profile, &["auth", "status", "--json"], TIMEOUT) {
        Ok(stdout) => parse_auth_status(&stdout),
        // `claude auth status --json` sai com código 1 quando `loggedIn: false`, mas
        // imprime o JSON válido em stdout.
        Err(InfoError::Failed(Some(detail))) => {
            parse_auth_status(&detail).map_err(|_| InfoError::Failed(Some(detail)))
        }
        Err(err) => Err(err),
    }
}

pub fn logout(home: &Path, profile: Option<&Path>) -> Result<(), InfoError> {
    run_with_profile(home, profile, &["auth", "logout"], TIMEOUT).map(|_| ())
}

pub(crate) fn auth_terminal(
    home: &Path,
    profile: Option<&Path>,
) -> Result<gsh_term::ShellCommand, InfoError> {
    auth_terminal_with_exe(&resolve_claude(), home, profile)
}

fn auth_terminal_with_exe(
    exe: &Path,
    home: &Path,
    profile: Option<&Path>,
) -> Result<gsh_term::ShellCommand, InfoError> {
    let program = exe.to_str().ok_or(InfoError::CliMissing)?;
    let mut command = gsh_term::ShellCommand::new(program)
        .args(["auth", "login"])
        .env("PATH", path_with(exe));
    if !home.as_os_str().is_empty() {
        command = command.cwd(home);
    }
    if let Some(profile) = verified_account_profile(profile)? {
        for key in gsh_chat::settings::PROFILE_AUTH_ENV {
            command = command.env_remove(key);
        }
        command = command.env(profiles::CONFIG_DIR, profile);
    }
    Ok(command)
}

pub fn mcp_servers(home: &Path) -> Result<Vec<McpServerInfo>, InfoError> {
    Ok(parse_mcp_list(&run(
        home,
        &["mcp", "list"],
        Duration::from_secs(90),
    )?))
}

pub fn plugins(home: &Path) -> Result<Vec<PluginInfo>, InfoError> {
    let list = parse_plugin_list(&run(home, &["plugin", "list", "--json"], TIMEOUT)?)?;
    Ok(files::with_manifests(list))
}

pub fn directory(home: &Path) -> Result<Vec<MarketplacePlugin>, InfoError> {
    let locations = parse_marketplace_list(&run(
        home,
        &["plugin", "marketplace", "list", "--json"],
        TIMEOUT,
    )?)?;
    let installed: BTreeSet<String> = plugins(home)
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.id)
        .collect();
    Ok(files::directory(&locations, &installed))
}

/// Sobe `claude -p` em `stream-json`, envia `get_usage` e encerra assim que a resposta chega.
pub fn usage(home: &Path, profile: Option<&Path>) -> Result<Usage, InfoError> {
    let exe = resolve_claude();
    let mut cmd = Command::new(&exe);
    cmd.args([
        "-p",
        "--no-session-persistence",
        "--settings",
        "{\"disableAllHooks\":true}",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
    ])
    .env("PATH", path_with(&exe))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    if !home.as_os_str().is_empty() {
        cmd.current_dir(home);
    }
    apply_account_profile(&mut cmd, profile)?;
    let mut child = cmd.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => InfoError::CliMissing,
        _ => InfoError::Failed(Some(e.to_string())),
    })?;
    let mut stdin = child.stdin.take();
    if let Some(pipe) = &mut stdin {
        let req = "{\"type\":\"control_request\",\"request_id\":\"gsh-usage\",\"request\":{\"subtype\":\"get_usage\",\"skip_behaviors\":true}}\n";
        let _ = pipe.write_all(req.as_bytes());
        let _ = pipe.flush();
    }
    let stdout = child.stdout.take();
    let reader_handle = std::thread::spawn(move || -> Option<Result<Usage, InfoError>> {
        let pipe = stdout?;
        for line in BufReader::new(pipe).lines() {
            let Ok(line) = line else {
                break;
            };
            let Ok(obj) = Json::parse(&line) else {
                continue;
            };
            if obj.get("type").and_then(Json::as_str) != Some("control_response") {
                continue;
            }
            let Some(resp) = obj.get("response").filter(|n| n.as_obj().is_some()) else {
                continue;
            };
            if resp.get("request_id").and_then(Json::as_str) != Some("gsh-usage") {
                continue;
            }
            if resp.get("subtype").and_then(Json::as_str) != Some("success") {
                let err = resp.get("error").map(Json::dart_string);
                return Some(Err(InfoError::Failed(err)));
            }
            if let Some(inner) = resp.get("response").filter(|n| n.as_obj().is_some()) {
                return Some(Ok(parse_usage_reply(inner)));
            }
            return Some(Err(InfoError::Failed(None)));
        }
        None
    });
    let started = Instant::now();
    loop {
        if reader_handle.is_finished() {
            break;
        }
        if started.elapsed() >= TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return Err(InfoError::Timeout);
        }
        if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) && reader_handle.is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let _ = child.kill();
    let _ = child.wait();
    match reader_handle.join().ok().flatten() {
        Some(result) => result,
        None => Err(InfoError::Failed(None)),
    }
}

const TIMEOUT: Duration = Duration::from_secs(30);

fn run(home: &Path, args: &[&str], timeout: Duration) -> Result<String, InfoError> {
    run_with_profile(home, None, args, timeout)
}

fn run_with_profile(
    home: &Path,
    profile: Option<&Path>,
    args: &[&str],
    timeout: Duration,
) -> Result<String, InfoError> {
    let exe = resolve_claude();
    let mut command = Command::new(&exe);
    command
        .args(args)
        .env("PATH", path_with(&exe))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !home.as_os_str().is_empty() {
        command.current_dir(home);
    }
    apply_account_profile(&mut command, profile)?;
    let mut child = command.spawn().map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => InfoError::CliMissing,
        _ => InfoError::Failed(Some(e.to_string())),
    })?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= timeout => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(InfoError::Timeout);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(e) => return Err(InfoError::Failed(Some(e.to_string()))),
        }
    };
    let out = stdout.join().unwrap_or_default();
    let err = stderr.join().unwrap_or_default();
    if status.success() {
        return Ok(out);
    }
    let err = err.trim();
    Err(InfoError::Failed(Some(
        if err.is_empty() { out.trim() } else { err }.to_owned(),
    )))
}

fn apply_account_profile(cmd: &mut Command, profile: Option<&Path>) -> Result<(), InfoError> {
    if let Some(profile) = verified_account_profile(profile)? {
        for key in gsh_chat::settings::PROFILE_AUTH_ENV {
            cmd.env_remove(key);
        }
        cmd.env(profiles::CONFIG_DIR, profile);
    }
    Ok(())
}

fn verified_account_profile(profile: Option<&Path>) -> Result<Option<PathBuf>, InfoError> {
    profile
        .map(|path| profiles::verify(path, false).map_err(|err| InfoError::Failed(Some(err))))
        .transpose()
}

fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            text = String::from_utf8_lossy(&bytes).into_owned();
        }
        text
    })
}

fn path_with(exe: &Path) -> std::ffi::OsString {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(dir) = exe.parent().filter(|d| !d.as_os_str().is_empty()) {
        dirs.push(dir.to_path_buf());
    }
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    std::env::join_paths(dirs).unwrap_or_default()
}

fn resolve_claude() -> PathBuf {
    use std::sync::OnceLock;
    static EXE: OnceLock<PathBuf> = OnceLock::new();
    EXE.get_or_init(|| which("claude").unwrap_or_else(|| PathBuf::from("claude")))
        .clone()
}

fn which(name: &str) -> Option<PathBuf> {
    if cfg!(windows) {
        return first_existing(&run_capture("where", &[name])?);
    }
    let shell = std::env::var("SHELL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            String::from(if cfg!(target_os = "macos") {
                "/bin/zsh"
            } else {
                "/bin/sh"
            })
        });
    let line = format!("which {name}");
    first_existing(&run_capture(&shell, &["-lc", &line])?)
        .or_else(|| first_existing(&run_capture(&shell, &["-ic", &line])?))
}

fn run_capture(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = drain(child.stdout.take());
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() >= Duration::from_secs(4) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => return None,
        }
    }
    stdout.join().ok()
}

fn first_existing(stdout: &str) -> Option<PathBuf> {
    stdout
        .lines()
        .map(str::trim)
        .map(PathBuf::from)
        .find(|path| path.is_absolute() && path.exists())
}

pub fn now_epoch_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Parseia ISO-8601 (`YYYY-MM-DDTHH:MM:SS[.fff][Z|+HH:MM|-HH:MM]`) em segundos Unix.
pub fn parse_iso8601(raw: &str) -> Option<i64> {
    let s = raw.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i64 = s.get(0..4)?.parse().ok()?;
    let month: i64 = s.get(5..7)?.parse().ok()?;
    let day: i64 = s.get(8..10)?.parse().ok()?;
    let hour: i64 = s.get(11..13)?.parse().ok()?;
    let min: i64 = s.get(14..16)?.parse().ok()?;
    let sec: i64 = s.get(17..19)?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let tail = s.get(19..)?;
    let tz_offset_secs = if let Some(pos) = tail.rfind(['+', '-']) {
        let sign = if tail.get(pos..pos + 1) == Some("-") {
            -1
        } else {
            1
        };
        let tz = tail.get(pos + 1..)?;
        let tz_h: i64 = tz.get(0..2)?.parse().ok()?;
        let tz_m: i64 = tz
            .get(3..5)
            .or_else(|| tz.get(2..4))
            .unwrap_or("0")
            .parse()
            .unwrap_or(0);
        sign * (tz_h * 3600 + tz_m * 60)
    } else {
        0
    };
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3600 + min * 60 + sec - tz_offset_secs)
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let m_adj = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * m_adj + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, usize, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, usize::try_from(m).unwrap_or(1), d)
}

#[cfg(unix)]
#[allow(clippy::useless_conversion)]
fn local_offset_secs(epoch_secs: i64) -> i64 {
    let t = libc::time_t::try_from(epoch_secs).unwrap_or_default();
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let filled = unsafe { libc::localtime_r(&t, &mut tm) };
    if filled.is_null() {
        0
    } else {
        i64::from(tm.tm_gmtoff)
    }
}

#[cfg(not(unix))]
fn local_offset_secs(_epoch_secs: i64) -> i64 {
    0
}

const MONTHS: [&str; 12] = [
    "jan.", "fev.", "mar.", "abr.", "mai.", "jun.", "jul.", "ago.", "set.", "out.", "nov.", "dez.",
];

const WEEKDAYS: [&str; 7] = ["seg.", "ter.", "qua.", "qui.", "sex.", "sáb.", "dom."];

/// `"18 de ago."` na hora local.
pub fn short_date(epoch_secs: i64) -> String {
    let local = epoch_secs.saturating_add(local_offset_secs(epoch_secs));
    let days = local.div_euclid(86_400);
    let (_, month, day) = civil_from_days(days);
    let m_name = MONTHS
        .get(month.saturating_sub(1))
        .copied()
        .unwrap_or("jan.");
    format!("{day} de {m_name}")
}

/// `"dom., 17:00"` na hora local.
pub fn weekday_time(epoch_secs: i64) -> String {
    let local = epoch_secs.saturating_add(local_offset_secs(epoch_secs));
    let days = local.div_euclid(86_400);
    let day_secs = local.rem_euclid(86_400);
    // 1970-01-01 foi quinta-feira (índice 3 em seg=0..dom=6)
    let wd_idx = usize::try_from((days + 3).rem_euclid(7)).unwrap_or(0);
    let wd = WEEKDAYS.get(wd_idx).copied().unwrap_or("seg.");
    format!("{wd}, {:02}:{:02}", day_secs / 3600, (day_secs % 3600) / 60)
}

/// `"4 h 48 min"` até `target_secs`.
pub fn duration_until(target_secs: i64, now_secs: i64) -> String {
    let diff = target_secs - now_secs;
    if diff <= 0 {
        return labels::usage_minutes(0);
    }
    let total_mins = diff / 60;
    let h = total_mins / 60;
    let m = total_mins % 60;
    if h == 0 {
        labels::usage_minutes(m)
    } else {
        labels::usage_hours_minutes(h, m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_and_ignores_non_strings() {
        let a = parse_auth_status(
            r#"{"loggedIn":true,"authMethod":"claude.ai","email":"a@b.c","orgId":7,"subscriptionType":"team"}"#,
        )
        .unwrap();
        assert!(a.logged_in);
        assert_eq!(a.email.as_deref(), Some("a@b.c"));
        assert_eq!(a.org_id, None);
        assert_eq!(a.subscription_type.as_deref(), Some("team"));
        assert!(
            !parse_auth_status(r#"{"loggedIn":"true"}"#)
                .unwrap()
                .logged_in
        );
        assert!(parse_auth_status("[]").is_err());
        assert!(parse_auth_status("lixo").is_err());
    }

    #[test]
    fn reads_extra_credits_from_the_cli_account_usage_payload() {
        let body = Json::parse(
            r#"{"rate_limits":{"limits":[{"kind":"session","percent":3},{"kind":"spend","is_active":true,"resets_at":"2026-10-01T00:00:00.000Z"}],"extra_usage":{"is_enabled":true,"utilization":54,"spend_limit_reached":false,"used_credits":27140,"monthly_limit":50000,"currency":"usd"}}}"#,
        )
        .unwrap();
        let usage = parse_usage_reply(&body);
        assert_eq!(usage.windows.len(), 1);
        let extra = usage.extra.unwrap();
        assert!(extra.enabled);
        assert_eq!(extra.percent, Some(54.0));
        assert!(!extra.spend_limit_reached);
        assert_eq!(extra.resets_at, parse_iso8601("2026-10-01T00:00:00.000Z"));
        let amounts = extra.amounts.unwrap();
        assert_eq!(
            amounts.format_minor(amounts.used_minor.unwrap()),
            "271,40 USD"
        );
        assert_eq!(
            amounts.format_minor(amounts.limit_minor.unwrap()),
            "500,00 USD"
        );
        let body = Json::parse(
            r#"{"rate_limits":{"extra_usage":{"is_enabled":false,"utilization":null,"spend_limit_reached":true}}}"#,
        )
        .unwrap();
        assert_eq!(
            parse_usage_reply(&body).extra,
            Some(ExtraUsage {
                enabled: false,
                percent: None,
                spend_limit_reached: true,
                resets_at: None,
                amounts: None,
            })
        );
        let missing = Json::parse(r#"{"rate_limits":{"extra_usage":{}}}"#).unwrap();
        assert_eq!(parse_usage_reply(&missing).extra, None);

        let jpy = Json::parse(
            r#"{"currency":"JPY","decimal_places":0,"used_credits":190,"monthly_limit":null}"#,
        )
        .unwrap();
        let amounts = parse_extra_amounts(&jpy).unwrap();
        assert_eq!(amounts.format_minor(amounts.used_minor.unwrap()), "190 JPY");
        assert_eq!(amounts.limit_minor, None);
        let invalid =
            Json::parse(r#"{"currency":"USD","decimal_places":100,"used_credits":10}"#).unwrap();
        assert_eq!(parse_extra_amounts(&invalid), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn profile_scopes_cli_command_and_removes_inherited_auth_overrides() {
        use std::os::unix::fs::PermissionsExt;

        let scratch = tempfile::tempdir().unwrap();
        std::fs::set_permissions(scratch.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let profile = scratch.path().join("Conta oficial");
        profiles::verify(&profile, true).unwrap();
        let mut command = Command::new("claude");
        for key in gsh_chat::settings::PROFILE_AUTH_ENV {
            command.env(key, "test-override");
        }
        command.env("UNRELATED_TEST_SETTING", "keep");
        apply_account_profile(&mut command, Some(&profile)).unwrap();
        let env: BTreeMap<_, _> = command.get_envs().collect();
        for key in gsh_chat::settings::PROFILE_AUTH_ENV {
            assert_eq!(env.get(std::ffi::OsStr::new(key)), Some(&None));
        }
        assert_eq!(
            env.get(std::ffi::OsStr::new(profiles::CONFIG_DIR)),
            Some(&Some(profile.as_os_str()))
        );
        assert!(env.contains_key(std::ffi::OsStr::new("UNRELATED_TEST_SETTING")));

        let mut default_account = Command::new("claude");
        apply_account_profile(&mut default_account, None).unwrap();
        assert_eq!(default_account.get_envs().count(), 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn interactive_auth_uses_the_same_cli_scope_without_workspace_startup() {
        use std::os::unix::fs::PermissionsExt;

        let scratch = tempfile::tempdir().unwrap();
        std::fs::set_permissions(scratch.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let profile = profiles::verify(&scratch.path().join("Conta oficial"), true).unwrap();
        let exe = scratch.path().join("claude ' fixture");
        std::fs::write(
            &exe,
            "#!/bin/sh\nprintf '%s|%s|%s|%s\n' \"$*\" \"$PWD\" \"$CLAUDE_CONFIG_DIR\" \"${ANTHROPIC_API_KEY+x}\" >> auth.log\n",
        ).unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o700)).unwrap();
        let command = auth_terminal_with_exe(&exe, scratch.path(), Some(&profile)).unwrap();
        let _session = gsh_term::TermSession::spawn(command, 80, 24).unwrap();
        let expected = format!(
            "auth login|{home}|{profile}|\n",
            home = scratch.path().display(),
            profile = profile.display(),
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        let log = scratch.path().join("auth.log");
        while std::fs::read_to_string(&log).as_deref().ok() != Some(expected.as_str()) {
            assert!(
                Instant::now() < deadline,
                "{:?}",
                std::fs::read_to_string(&log)
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn parses_usage_mcp_plugins_and_dates() {
        let body = Json::parse(
            r#"{"rate_limits_available":true,"subscription_type":"team","rate_limits":{"five_hour":{"utilization":42.5,"resets_at":"2026-11-16T19:00:00Z"},"seven_day":{"utilization":10},"model_scoped":[{"display_name":"Opus","utilization":80}]}}"#,
        )
        .unwrap();
        let u = parse_usage_reply(&body);
        assert!(u.available);
        assert_eq!(u.subscription_type.as_deref(), Some("team"));
        assert_eq!(u.windows.len(), 3);
        assert_eq!(u.windows[0].percent, Some(42.5));
        assert_eq!(u.windows[2].label.as_deref(), Some("Opus"));

        let new_body = Json::parse(
            r#"{"subscription_type":"team","limits":[{"kind":"session","percent":3,"resets_at":"2026-10-25T07:30:00Z"},{"kind":"weekly_all","percent":86,"resets_at":"2026-10-25T20:00:00Z"},{"kind":"weekly_scoped","percent":59,"scope":{"model":{"display_name":"Fable"}}}]}"#,
        )
        .unwrap();
        let u_new = parse_usage_reply(&new_body);
        assert!(u_new.available);
        assert_eq!(u_new.subscription_type.as_deref(), Some("team"));
        assert_eq!(u_new.windows.len(), 3);
        assert_eq!(u_new.windows[0].percent, Some(3.0));
        assert_eq!(u_new.windows[1].percent, Some(86.0));
        assert_eq!(u_new.windows[2].label.as_deref(), Some("Fable"));
        assert_eq!(u_new.windows[2].percent, Some(59.0));

        let unavail = Json::parse(
            r#"{"subscription_type":null,"rate_limits_available":false,"rate_limits":null}"#,
        )
        .unwrap();
        assert!(!parse_usage_reply(&unavail).available);

        let mcp = parse_mcp_list(
            "claude.ai Notion: https://mcp.notion.com - ! Needs authentication\nplugin:foo:bar: npx bar - ✓ Connected\n",
        );
        assert_eq!(mcp.len(), 2);
        assert_eq!(mcp[0].display_name(), "Notion");
        assert!(mcp[0].is_remote());
        assert_eq!(mcp[0].status, McpStatus::NeedsAuth);
        assert_eq!(mcp[1].display_name(), "bar");
        assert_eq!(mcp[1].plugin_name(), Some("foo"));
        assert_eq!(mcp[1].command_line(), ("npx", "bar"));

        let pls =
            parse_plugin_list(r#"[{"id":"a@mkt","enabled":true,"version":"1.0.0"}]"#).unwrap();
        assert_eq!(pls[0].name(), "a");
        assert_eq!(pls[0].marketplace(), "mkt");

        let mkts =
            parse_marketplace_list(r#"[{"name":"m1","installLocation":"/tmp/m1"}]"#).unwrap();
        assert_eq!(mkts.get("m1").map(String::as_str), Some("/tmp/m1"));

        assert_eq!(duration_until(1000 + 3660, 1000), "1 h 1 min");
        assert_eq!(duration_until(900, 1000), "0 min");
    }

    #[test]
    fn first_existing_takes_absolute_existing_lines_only() {
        let here = std::env::current_exe().unwrap();
        let out = format!("claude not found\nrelative/claude\n{}\n", here.display());
        assert_eq!(first_existing(&out), Some(here));
        assert_eq!(first_existing("nada\n"), None);
    }
}
