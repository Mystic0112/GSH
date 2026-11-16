//! A CLI `claude` instalada (`ClaudeCliInfoSource` e `claude_info_parsers.dart` do Flutter):
//! conta, logout, limites de uso (`get_usage`), servidores MCP e plugins/marketplaces.

use super::files;
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
    if let Some(list) = body.get("limits").and_then(Json::as_arr) {
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
    } else if let Some(limits) = body.get("rate_limits").filter(|n| n.as_obj().is_some()) {
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
                if item.as_obj().is_some() {
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
    Usage {
        available,
        subscription_type: body
            .get("subscription_type")
            .and_then(Json::as_str)
            .map(str::to_owned),
        windows,
    }
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

pub fn account(home: &Path) -> Result<Account, InfoError> {
    match run(home, &["auth", "status", "--json"], TIMEOUT) {
        Ok(stdout) => parse_auth_status(&stdout),
        // `claude auth status --json` sai com código 1 quando `loggedIn: false`, mas
        // imprime o JSON válido em stdout.
        Err(InfoError::Failed(Some(detail))) => {
            parse_auth_status(&detail).map_err(|_| InfoError::Failed(Some(detail)))
        }
        Err(err) => Err(err),
    }
}

pub fn logout(home: &Path) -> Result<(), InfoError> {
    run(home, &["auth", "logout"], TIMEOUT).map(|_| ())
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
pub fn usage(home: &Path) -> Result<Usage, InfoError> {
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
