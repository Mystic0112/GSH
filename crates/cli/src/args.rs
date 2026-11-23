//! Parse de argumentos, manual como na CLI original (não é `clap`): a saída de
//! erro e os códigos de saída têm que ficar idênticos, e quem lê é agente e
//! script. O parse não lê ambiente nem disco; resolver `GSH_TAB_ID`, cwd e
//! caminhos é da execução ([`crate::commands`]).

use gsh_core::wire::FOCUSED;
use serde_json::{Map, Value};

use crate::error::CliError;

pub const HELP: &str = include_str!("../text/help.txt");

pub const MSG_HELP: &str = "gsh msg --to <label|tab-id> [--json] <text...>
  Messages another agent: types `[from: <your tab> · <your harness>] <text>`
  into the target tab and presses Enter. The sender is this tab
  ($GSH_TAB_ID), so it only works inside a GSH terminal.
  --to   stable tab label (exact, case-insensitive) or tab-id; an ambiguous
         label is an error (see `gsh list-tabs`). A teammate's agent on the
         office network is `<Name>/<agent>` (see `gsh peers`): it knocks on
         their door and their agent decides what to do with it.
  Prints the target tab id (--json: {\"from\": …, \"to\": …}).";

pub const DELIVER_HELP: &str = "gsh deliver --to <Name/agent> [--json] <file> [note...]
  Delivers a file (report, .md, .pdf...) to a teammate's agent on the office
  network: it lands in `.gsh/inbox/<your name>/` inside that agent's folder
  and the agent is told it arrived (with the optional note). Needs the team
  farm on with the same team name and password on both sides (see
  `gsh peers` for the addresses). Max 25 MB.";

pub const NEW_TAB_HELP: &str = "gsh new-tab [--cwd <dir>] [--title <name>] [--split h|v]
  --cwd    working directory (default: current directory)
  --title  stable tab label (read-tab/send can target it)
  --split  h|right = side by side, v|down = stacked; omit = new tab
           in the same pane
  Prints the new tab id (e.g. t12).";

pub const CLOSE_TAB_HELP: &str = "gsh close-tab [<label|tab-id>] [--json]
  Closes a tab (the counterpart of `new-tab`). Without a target, closes the
  CURRENT tab — which ends the shell you are typing in.
  A target may be a stable tab label or a tab-id (see `gsh list-tabs`).
  Same behaviour as the tab's \"x\" in the UI: closing the last tab of a split
  removes the split; closing the last tab of a workspace leaves an empty tab.
  Prints the id of the closed tab.";

pub const READ_TAB_HELP: &str =
    "gsh read-tab [<label|tab-id>] [--lines <n>] [--offset <n>] [--from-start] [--json]
  Reads a tab's rendered output (alias: read-pane). Without a target, reads
  the CURRENT tab.
  --lines N     how many lines (default 100, server cap 2000)
  --offset N    skip N lines from the anchor (pagination)
  --from-start  anchor at the start of the buffer (default: tail/end)";

pub const RENAME_TAB_HELP: &str =
    "gsh rename-tab [<label|tab-id>] <title> [--tab-id <id> | --focused] [--json]
  Updates the stable label of a tab. Without a target, renames the CURRENT
  tab. Prints the id of the renamed tab.";

pub const INTERRUPT_HELP: &str =
    "gsh interrupt [<label|tab-id>] [--tab-id <id> | --focused] [--force] [--json]
  Interrupts a tab's running turn or process. Without a target, interrupts
  the CURRENT tab. Prints the id of the interrupted tab.";

pub const WAIT_IDLE_HELP: &str =
    "gsh wait-idle [<label|tab-id>] [--tab-id <id> | --focused] [--json]
  Waits until a tab returns to idle. Without a target, waits on the CURRENT
  tab. Prints the id of the tab.";

/// Qual aba o comando mira. A resolução final precisa do ambiente
/// (`GSH_TAB_ID`), por isso fica pendente até a execução.
#[derive(Debug, Default, PartialEq)]
pub struct Target {
    pub focused: bool,
    pub tab_id: Option<String>,
}

impl Target {
    /// Na ordem: `--focused` (sentinela que o app resolve), `--tab-id`
    /// explícito, e por fim a aba que emitiu o comando (`own`).
    pub fn resolve(&self, own: Option<String>) -> Option<String> {
        if self.focused {
            return Some(FOCUSED.to_string());
        }
        self.tab_id.clone().or(own)
    }
}

/// Flags comuns aos comandos simples: o parser tem que consumir exatamente o
/// que o original consome (uma flag desconhecida vira posicional, ou seja,
/// texto).
#[derive(Debug, Default, PartialEq)]
pub struct Flags {
    pub positionals: Vec<String>,
    pub tab_id: Option<String>,
    pub json: bool,
    pub force: bool,
    pub lines: Option<u64>,
    pub offset: Option<u64>,
    pub from_start: bool,
    pub enter: bool,
    pub focused: bool,
}

fn int_value(flag: &str, raw: &str) -> Result<u64, CliError> {
    raw.parse::<u64>()
        .map_err(|_| CliError::usage(format!("gsh: {flag} requires a non-negative integer")))
}

/// Valor separado de uma flag (`--flag valor`), avançando o índice.
fn next_value<'a>(args: &'a [String], i: &mut usize, flag: &str) -> Result<&'a str, CliError> {
    *i += 1;
    args.get(*i)
        .map(String::as_str)
        .ok_or_else(|| CliError::usage(format!("gsh: {flag} requires a value")))
}

impl Flags {
    pub fn parse(args: &[String]) -> Result<Flags, CliError> {
        let mut f = Flags::default();
        let mut i = 0usize;
        while let Some(a) = args.get(i).map(String::as_str) {
            if a == "--lines" || a == "-n" {
                f.lines = Some(int_value("--lines", next_value(args, &mut i, "--lines")?)?);
            } else if let Some(v) = a.strip_prefix("--lines=") {
                f.lines = Some(int_value("--lines", v)?);
            } else if a == "--offset" {
                f.offset = Some(int_value(
                    "--offset",
                    next_value(args, &mut i, "--offset")?,
                )?);
            } else if let Some(v) = a.strip_prefix("--offset=") {
                f.offset = Some(int_value("--offset", v)?);
            } else if a == "--from-start" {
                f.from_start = true;
            } else if a == "--tab-id" || a == "-t" {
                f.tab_id = Some(next_value(args, &mut i, "--tab-id")?.to_string());
            } else if let Some(v) = a.strip_prefix("--tab-id=") {
                f.tab_id = Some(v.to_string());
            } else if a == "--enter" {
                f.enter = true;
            } else if a == "--focused" {
                f.focused = true;
            } else if a == "--json" {
                f.json = true;
            } else if a == "--force" || a == "-f" {
                f.force = true;
            } else if a == "--" {
                f.positionals
                    .extend(args.get(i + 1..).unwrap_or_default().iter().cloned());
                break;
            } else {
                f.positionals.push(a.to_string());
            }
            i += 1;
        }
        Ok(f)
    }

    pub fn target(&self) -> Target {
        Target {
            focused: self.focused,
            tab_id: self.tab_id.clone(),
        }
    }
}

/// Lê `--flag valor` ou `--flag=valor` na posição atual, avançando o índice ao
/// consumir o valor separado. Devolve `(flag, valor)` quando casa; `--flag` no
/// fim dos argumentos casa com valor `None`.
fn take<'a>(
    args: &[String],
    i: &mut usize,
    flags: &[&'a str],
) -> Option<(&'a str, Option<String>)> {
    let a = args.get(*i)?.as_str();
    for flag in flags {
        if a == *flag {
            *i += 1;
            return Some((flag, args.get(*i).cloned()));
        }
        if let Some(v) = a.strip_prefix(*flag).and_then(|r| r.strip_prefix('=')) {
            return Some((flag, Some(v.to_string())));
        }
    }
    None
}

/// `--to` sai dos argumentos; o resto vai pro [`Flags::parse`] (texto + `--json`).
fn split_to(args: &[String]) -> (Option<String>, Vec<String>) {
    let mut to = None;
    let mut rest = Vec::new();
    let mut i = 0usize;
    while let Some(a) = args.get(i) {
        if a == "--" {
            rest.extend(args.get(i..).unwrap_or_default().iter().cloned());
            break;
        }
        match take(args, &mut i, &["--to"]) {
            Some((_, v)) => to = Some(v.unwrap_or_default()),
            None => rest.push(a.clone()),
        }
        i += 1;
    }
    (to, rest)
}

#[derive(Debug, PartialEq)]
pub enum Split {
    Right,
    Down,
}

impl Split {
    pub fn wire(&self) -> &'static str {
        match self {
            Split::Right => "right",
            Split::Down => "down",
        }
    }
}

/// O que a linha de comando pede. Montado só com os argumentos.
#[derive(Debug, PartialEq)]
pub enum Command {
    /// Texto de ajuda para o stdout (exit 0). Termina em uma quebra de linha só.
    Help(&'static str),
    Version,
    Hook {
        harness: String,
    },
    Mcp {
        agent: String,
    },
    Msg {
        to: String,
        text: String,
        json: bool,
    },
    TeamSend {
        to: String,
        text: String,
        json: bool,
    },
    Send {
        text: String,
        enter: bool,
        target: Target,
    },
    SendKey {
        keys: Vec<String>,
        target: Target,
    },
    TeamPeers {
        json: bool,
    },
    Deliver {
        to: String,
        file: String,
        note: String,
        json: bool,
    },
    Open {
        path: String,
        target: Target,
    },
    ReadTab {
        label: Option<String>,
        lines: Option<u64>,
        offset: Option<u64>,
        from_start: bool,
        target: Target,
        json: bool,
    },
    RenameTab {
        label: Option<String>,
        title: String,
        target: Target,
        json: bool,
    },
    Interrupt {
        label: Option<String>,
        force: bool,
        target: Target,
        json: bool,
    },
    WaitIdle {
        label: Option<String>,
        target: Target,
        json: bool,
    },
    NewTab {
        cwd: Option<String>,
        title: Option<String>,
        split: Option<Split>,
        target: Target,
        json: bool,
    },
    CloseTab {
        label: Option<String>,
        target: Target,
        json: bool,
    },
    ListTabs {
        target: Target,
        json: bool,
    },
    ListWorkspaces {
        json: bool,
    },
    NewWorkspace {
        path: String,
        host: Option<String>,
        name: Option<String>,
        tab_id: Option<String>,
        json: bool,
    },
    CloseWorkspace {
        target: Option<String>,
        tab_id: Option<String>,
        json: bool,
    },
    RenameWorkspace {
        target: Option<String>,
        name: String,
        tab_id: Option<String>,
        json: bool,
    },
    Exec {
        command: String,
        cwd: Option<String>,
        timeout: Option<u64>,
        tab_id: Option<String>,
        json: bool,
    },
    Database {
        wire: &'static str,
        args: Value,
        tab_id: Option<String>,
    },
    Telemetry(Vec<String>),
    LocalSend {
        args: Vec<String>,
    },
}

pub fn parse(argv: &[String]) -> Result<Command, CliError> {
    let Some(first) = argv.first().map(String::as_str) else {
        // Sem argumentos: ajuda no stderr, exit 2 (como o original).
        return Err(CliError::usage(HELP.trim_end_matches('\n')));
    };
    let args = argv.get(1..).unwrap_or_default();
    match first {
        "--help" | "-h" | "help" => Ok(Command::Help(HELP)),
        "--version" | "-v" => Ok(Command::Version),
        "hook" => Ok(Command::Hook {
            harness: harness_from(args),
        }),
        "mcp" => parse_mcp(args),
        "msg" => parse_msg(args),
        "send" => parse_send(args),
        "send-key" => parse_send_key(args),
        "peers" => Ok(Command::TeamPeers {
            json: Flags::parse(args)?.json,
        }),
        "deliver" => parse_deliver(args),
        "open" => parse_open(args),
        "read-tab" | "read-pane" => parse_read_tab(args),
        "rename-tab" => parse_rename_tab(args),
        "interrupt" => parse_interrupt(args),
        "wait-idle" => parse_wait_idle(args),
        "new-tab" => parse_new_tab(args),
        "close-tab" => parse_close_tab(args),
        // `list-panes` é o verbo legado; o comando de wire é o mesmo.
        "list-tabs" | "list-panes" => {
            let f = Flags::parse(args)?;
            Ok(Command::ListTabs {
                target: f.target(),
                json: f.json,
            })
        }
        "list-workspaces" => Ok(Command::ListWorkspaces {
            json: Flags::parse(args)?.json,
        }),
        "new-workspace" | "open-workspace" | "new-remote-workspace" => parse_new_workspace(args),
        "close-workspace" => parse_close_workspace(args),
        "rename-workspace" => parse_rename_workspace(args),
        "exec" => parse_exec(args),
        "db" => parse_database(args).map_err(database_usage_error),
        "redis" => parse_nosql("redis-cmd", args).map_err(database_usage_error),
        "mongo" => parse_nosql("mongo-cmd", args).map_err(database_usage_error),
        "telemetry" => Ok(Command::Telemetry(args.to_vec())),
        "localsend" => Ok(Command::LocalSend {
            args: args.to_vec(),
        }),
        _ => parse_open(argv),
    }
}

fn database_usage_error(error: CliError) -> CliError {
    match error {
        CliError::Usage(message) => CliError::JsonFailure(serde_json::json!({
            "kind": "error",
            "message": message,
        })),
        other => other,
    }
}

const DB_HELP: &str = "gsh db <list|schema|query|execute|run> [options]\n  list [--workspace <id|path>]\n  schema --db <name> [<table>] [--workspace <id|path>]\n  query --db <name> --sql <SQL> [--limit <n>] [--workspace <id|path>]\n  execute --db <name> --sql <SQL> [--workspace <id|path>]\n  run <file.dbq> [--workspace <id|path>]";
const REDIS_HELP: &str = "gsh redis --db <name> <CMD> [args...] [--workspace <id|path>]";
const MONGO_HELP: &str =
    "gsh mongo --db <name> [--database <name>] --command <json> [--workspace <id|path>]";

fn parse_database(args: &[String]) -> Result<Command, CliError> {
    let Some(verb) = args.first().map(String::as_str) else {
        return Err(CliError::usage(DB_HELP));
    };
    if wants_help(args) {
        return Ok(Command::Help(DB_HELP));
    }
    let wire = match verb {
        "list" => "db-list",
        "schema" => "db-schema",
        "query" => "db-query",
        "execute" => "db-execute",
        "run" => "db-run",
        _ => return Err(CliError::usage(DB_HELP)),
    };
    let mut payload = Map::new();
    let mut positionals = Vec::new();
    let mut tab_id = None;
    let mut i = 1;
    while let Some(arg) = args.get(i).map(String::as_str) {
        if arg == "--" {
            positionals.extend(args.get(i + 1..).unwrap_or_default().iter().cloned());
            break;
        }
        let key = match arg {
            "--db" | "--sql" | "--limit" | "--workspace" | "--table" | "--tab-id" => {
                Some(arg.trim_start_matches('-'))
            }
            _ => None,
        };
        let (key, value, consumed) = if let Some(key) = key {
            let value = args
                .get(i + 1)
                .ok_or_else(|| CliError::usage(format!("gsh db {verb}: {arg} requires a value")))?;
            (key, value.clone(), 2)
        } else if let Some((flag, value)) = arg.split_once('=') {
            let key = match flag {
                "--db" | "--sql" | "--limit" | "--workspace" | "--table" | "--tab-id" => {
                    Some(flag.trim_start_matches('-'))
                }
                _ => None,
            };
            if let Some(key) = key {
                (key, value.to_string(), 1)
            } else if arg.starts_with('-') {
                return Err(CliError::usage(format!(
                    "gsh db {verb}: unknown flag \"{arg}\"\n{DB_HELP}"
                )));
            } else {
                positionals.push(arg.to_string());
                i += 1;
                continue;
            }
        } else if arg.starts_with('-') {
            return Err(CliError::usage(format!(
                "gsh db {verb}: unknown flag \"{arg}\"\n{DB_HELP}"
            )));
        } else {
            positionals.push(arg.to_string());
            i += 1;
            continue;
        };
        if key == "tab-id" {
            tab_id = Some(value);
        } else {
            let parsed = if key == "limit" {
                Value::from(value.parse::<u64>().map_err(|_| {
                    CliError::usage("gsh db query: --limit requires a non-negative integer")
                })?)
            } else {
                Value::String(value)
            };
            payload.insert(key.to_string(), parsed);
        }
        i += consumed;
    }
    if matches!(wire, "db-schema" | "db-query" | "db-execute")
        && !payload
            .get("db")
            .is_some_and(|value| value.as_str().is_some_and(|s| !s.is_empty()))
    {
        return Err(CliError::usage(format!(
            "gsh db {verb}: missing --db <name>\n{DB_HELP}"
        )));
    }
    if matches!(wire, "db-query" | "db-execute") {
        let sql = payload
            .get("sql")
            .and_then(Value::as_str)
            .filter(|sql| !sql.trim().is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| positionals.join(" "));
        if sql.trim().is_empty() {
            return Err(CliError::usage(format!(
                "gsh db {verb}: missing --sql \"<statement>\"\n{DB_HELP}"
            )));
        }
        payload.insert("sql".to_string(), Value::String(sql));
    }
    if wire == "db-schema" {
        if !payload.contains_key("table") {
            if let Some(table) = positionals.first() {
                payload.insert("table".to_string(), Value::String(table.clone()));
            }
        }
    } else if wire == "db-run" {
        let path = positionals
            .first()
            .filter(|path| !path.is_empty())
            .ok_or_else(|| CliError::usage(format!("gsh db run: missing <file.dbq>\n{DB_HELP}")))?;
        payload.insert(
            "path".to_string(),
            Value::String(crate::env::resolve_path(path)),
        );
    } else if !positionals.is_empty() && !matches!(wire, "db-query" | "db-execute") {
        return Err(CliError::usage(format!(
            "gsh db {verb}: unexpected argument\n{DB_HELP}"
        )));
    }
    Ok(Command::Database {
        wire,
        args: Value::Object(payload),
        tab_id,
    })
}

fn parse_nosql(wire: &'static str, args: &[String]) -> Result<Command, CliError> {
    let help = if wire == "redis-cmd" {
        REDIS_HELP
    } else {
        MONGO_HELP
    };
    if args.is_empty() {
        return Err(CliError::usage(help));
    }
    if wants_help(args) {
        return Ok(Command::Help(help));
    }
    let mut payload = Map::new();
    let mut positionals = Vec::new();
    let mut tab_id = None;
    let mut i = 0;
    while let Some(arg) = args.get(i).map(String::as_str) {
        let keys: &[&str] = if wire == "redis-cmd" {
            &["--db", "--workspace", "--tab-id"]
        } else {
            &["--db", "--database", "--command", "--workspace", "--tab-id"]
        };
        if let Some((flag, value)) = take(args, &mut i, keys) {
            match flag {
                "--db" => {
                    if let Some(value) = value {
                        payload.insert("db".to_string(), Value::String(value));
                    }
                }
                "--workspace" => {
                    if let Some(value) = value {
                        payload.insert("workspace".to_string(), Value::String(value));
                    }
                }
                "--tab-id" => tab_id = value,
                "--database" => {
                    if let Some(value) = value {
                        payload.insert("database".to_string(), Value::String(value));
                    }
                }
                "--command" => {
                    if let Some(value) = value {
                        payload.insert("command".to_string(), Value::String(value));
                    }
                }
                _ => {}
            }
        } else if wire == "redis-cmd" {
            positionals.push(arg.to_string());
        }
        i += 1;
    }
    if !payload
        .get("db")
        .is_some_and(|value| value.as_str().is_some_and(|s| !s.is_empty()))
    {
        return Err(CliError::usage(format!(
            "gsh {}: missing --db <name>\n{help}",
            if wire == "redis-cmd" {
                "redis"
            } else {
                "mongo"
            }
        )));
    }
    if wire == "redis-cmd" {
        if positionals.is_empty() {
            return Err(CliError::usage(format!(
                "gsh redis: missing command\n{help}"
            )));
        }
        payload.insert("parts".to_string(), serde_json::json!(positionals));
    } else if !payload
        .get("command")
        .is_some_and(|value| value.as_str().is_some_and(|s| !s.is_empty()))
        || !positionals.is_empty()
    {
        return Err(CliError::usage(format!(
            "gsh mongo: missing or invalid --command '<json>'\n{help}"
        )));
    }
    Ok(Command::Database {
        wire,
        args: Value::Object(payload),
        tab_id,
    })
}

fn parse_open(args: &[String]) -> Result<Command, CliError> {
    let flags = Flags::parse(args)?;
    let target = flags.target();
    let path = flags
        .positionals
        .into_iter()
        .next()
        .filter(|path| !path.is_empty())
        .ok_or_else(|| CliError::usage("gsh open: missing file path"))?;
    Ok(Command::Open { path, target })
}

fn parse_mcp(args: &[String]) -> Result<Command, CliError> {
    if wants_help(args) {
        return Ok(Command::Help("gsh mcp --agent <claude-desktop|cursor|codex-desktop>\n  Runs the GSH tools over MCP stdio."));
    }
    let mut agent = None;
    let mut index = 0;
    while let Some(value) = args.get(index) {
        if value == "--agent" {
            index += 1;
            agent = args.get(index).cloned();
        } else if let Some(value) = value.strip_prefix("--agent=") {
            agent = Some(value.to_owned());
        } else {
            return Err(CliError::usage("gsh mcp: expected --agent <id>"));
        }
        index += 1;
    }
    let agent = agent
        .filter(|agent| ["claude-desktop", "cursor", "codex-desktop"].contains(&agent.as_str()))
        .ok_or_else(|| CliError::usage("gsh mcp: missing or invalid --agent"))?;
    Ok(Command::Mcp { agent })
}

fn parse_send(args: &[String]) -> Result<Command, CliError> {
    if wants_help(args) {
        return Ok(Command::Help(concat!(
            "gsh send [--tab-id <id> | --focused] [--enter] <text...>\n",
            "  Types into a terminal tab (default: the current tab).\n",
            "gsh send --to <Name/agent> [--json] <text...>\n",
            "  Sends a message to a teammate's agent on the office network.",
        )));
    }
    let (to, rest) = split_to(args);
    let parsed = Flags::parse(&rest)?;
    if to.as_deref() == Some("") {
        return Err(CliError::usage("gsh send: missing --to"));
    }
    let text = parsed.positionals.join(" ");
    if text.is_empty() {
        return Err(CliError::usage(if to.is_some() {
            "gsh send: missing text"
        } else {
            "gsh send: missing text to send"
        }));
    }
    if let Some(to) = to {
        return Ok(Command::TeamSend {
            to,
            text,
            json: parsed.json,
        });
    }
    Ok(Command::Send {
        text,
        enter: parsed.enter,
        target: parsed.target(),
    })
}

fn parse_send_key(args: &[String]) -> Result<Command, CliError> {
    let parsed = Flags::parse(args)?;
    if wants_help(args) {
        return Ok(Command::Help(SEND_KEY_HELP));
    }
    if parsed.positionals.is_empty() {
        return Err(CliError::usage(
            "gsh send-key: missing key (e.g. Enter, C-c, Escape)",
        ));
    }
    for name in &parsed.positionals {
        if resolve_key(name).is_none() {
            return Err(CliError::usage(format!(
                "gsh send-key: unknown key \"{name}\""
            )));
        }
    }
    let target = parsed.target();
    Ok(Command::SendKey {
        keys: parsed.positionals,
        target,
    })
}

fn parse_new_workspace(args: &[String]) -> Result<Command, CliError> {
    if wants_help(args) {
        return Ok(Command::Help(NEW_WORKSPACE_HELP));
    }
    let (mut path, mut host, mut name, mut tab_id, mut json) = (None, None, None, None, false);
    let mut i = 0;
    while let Some(arg) = args.get(i).map(String::as_str) {
        if arg == "--" {
            path.get_or_insert_with(|| args.get(i + 1..).unwrap_or_default().join(" "));
            break;
        }
        if arg == "--json" {
            json = true;
        } else if arg == "--host" || arg == "--remote" {
            host = Some(
                args.get(i + 1)
                    .ok_or_else(|| CliError::usage("gsh new-workspace: --host requires a value"))?
                    .clone(),
            );
            i += 1;
        } else if let Some(value) = arg
            .strip_prefix("--host=")
            .or_else(|| arg.strip_prefix("--remote="))
        {
            host = Some(value.to_owned());
        } else if arg == "--name" {
            name = Some(
                args.get(i + 1)
                    .ok_or_else(|| CliError::usage("gsh new-workspace: --name requires a value"))?
                    .clone(),
            );
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--name=") {
            name = Some(value.to_owned());
        } else if arg == "--tab-id" || arg == "-t" {
            tab_id = Some(
                args.get(i + 1)
                    .ok_or_else(|| CliError::usage("gsh: --tab-id requires a value"))?
                    .clone(),
            );
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--tab-id=") {
            tab_id = Some(value.to_owned());
        } else if arg.starts_with('-') {
            return Err(CliError::usage(format!("gsh: unknown flag \"{arg}\"")));
        } else if path.is_none() {
            path = Some(arg.to_owned());
        }
        i += 1;
    }
    let path = path
        .filter(|s| !s.is_empty())
        .ok_or_else(|| CliError::usage("gsh new-workspace: missing <path> (or --path <path>)"))?;
    Ok(Command::NewWorkspace {
        path,
        host,
        name,
        tab_id,
        json,
    })
}

fn parse_close_workspace(args: &[String]) -> Result<Command, CliError> {
    let (mut target, mut tab_id, mut json) = (None, None, false);
    let mut i = 0;
    while let Some(arg) = args.get(i).map(String::as_str) {
        if arg == "--help" || arg == "-h" {
            return Ok(Command::Help(CLOSE_WORKSPACE_HELP));
        }
        if arg == "--json" {
            json = true;
        } else if arg == "--tab-id" || arg == "-t" {
            tab_id = Some(
                args.get(i + 1)
                    .ok_or_else(|| CliError::usage("gsh: --tab-id requires a value"))?
                    .clone(),
            );
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--tab-id=") {
            tab_id = Some(value.to_owned());
        } else if arg.starts_with('-') {
            return Err(CliError::usage(format!("gsh: unknown flag \"{arg}\"")));
        } else if target.is_none() {
            target = Some(arg.to_owned());
        }
        i += 1;
    }
    Ok(Command::CloseWorkspace {
        target,
        tab_id,
        json,
    })
}

fn parse_rename_workspace(args: &[String]) -> Result<Command, CliError> {
    let (mut target, mut name, mut tab_id, mut json) = (None, None, None, false);
    let mut i = 0;
    while let Some(arg) = args.get(i).map(String::as_str) {
        if arg == "--help" || arg == "-h" {
            return Ok(Command::Help(RENAME_WORKSPACE_HELP));
        }
        if arg == "--json" {
            json = true;
        } else if arg == "--tab-id" || arg == "-t" {
            tab_id = Some(
                args.get(i + 1)
                    .ok_or_else(|| CliError::usage("gsh: --tab-id requires a value"))?
                    .clone(),
            );
            i += 1;
        } else if let Some(value) = arg.strip_prefix("--tab-id=") {
            tab_id = Some(value.to_owned());
        } else if arg.starts_with('-') {
            return Err(CliError::usage(format!("gsh: unknown flag \"{arg}\"")));
        } else if target.is_none() {
            target = Some(arg.to_owned());
        } else if name.is_none() {
            name = Some(arg.to_owned());
        }
        i += 1;
    }
    let (target, name) = match (target, name) {
        (Some(target), Some(name)) => (Some(target), name),
        (Some(name), None) => (None, name),
        _ => {
            return Err(CliError::usage(
                "gsh rename-workspace: missing workspace name",
            ))
        }
    };
    Ok(Command::RenameWorkspace {
        target,
        name,
        tab_id,
        json,
    })
}

pub const EXEC_HELP: &str = "gsh exec [--cwd <dir>] [--timeout <s>] [--json] [--] <command...>\n  Runs a shell line through the app. Timeout defaults to 60 seconds; exit code\n  is the command's code (124 on timeout). --json prints {ok, code, stdout, stderr, timedOut}.";

fn parse_exec(args: &[String]) -> Result<Command, CliError> {
    let (mut cwd, mut timeout, mut tab_id, mut json) = (None, None, None, false);
    let mut command = Vec::new();
    let mut i = 0;
    while let Some(arg) = args.get(i).map(String::as_str) {
        match arg {
            "--help" | "-h" => return Ok(Command::Help(EXEC_HELP)),
            "--cwd" => {
                cwd = Some(
                    args.get(i + 1)
                        .ok_or_else(|| CliError::usage("gsh exec: --cwd requires a value"))?
                        .clone(),
                );
                i += 1;
            }
            "--timeout" => {
                let raw = args
                    .get(i + 1)
                    .ok_or_else(|| CliError::usage("gsh exec: --timeout requires a value"))?;
                timeout = Some(raw.parse::<u64>().map_err(|_| {
                    CliError::usage("gsh exec: --timeout requires a non-negative integer")
                })?);
                i += 1;
            }
            "--tab-id" | "-t" => {
                tab_id = Some(
                    args.get(i + 1)
                        .ok_or_else(|| CliError::usage("gsh: --tab-id requires a value"))?
                        .clone(),
                );
                i += 1;
            }
            "--json" => json = true,
            "--" => {
                command.extend(args.get(i + 1..).unwrap_or_default().iter().cloned());
                break;
            }
            _ if arg.starts_with("--") => {
                return Err(CliError::usage(format!(
                    "gsh exec: unknown flag {arg}\n{EXEC_HELP}"
                )))
            }
            _ => {
                command.extend(args.get(i..).unwrap_or_default().iter().cloned());
                break;
            }
        }
        i += 1;
    }
    if command.is_empty() {
        return Err(CliError::usage(format!(
            "gsh exec: missing command\n{EXEC_HELP}"
        )));
    }
    Ok(Command::Exec {
        command: command.join(" "),
        cwd,
        timeout,
        tab_id,
        json,
    })
}

/// Byte-for-byte key mapping from the Flutter CLI's `keys.rs`.
pub(crate) fn resolve_key(name: &str) -> Option<String> {
    let lower = name.to_lowercase();
    let sequence = match lower.as_str() {
        "enter" | "return" | "cr" => "\r",
        "tab" => "\t",
        "escape" | "esc" => "\x1b",
        "space" => " ",
        "bspace" | "backspace" => "\x7f",
        "up" => "\x1b[A",
        "down" => "\x1b[B",
        "right" => "\x1b[C",
        "left" => "\x1b[D",
        "home" => "\x1b[H",
        "end" => "\x1b[F",
        "pageup" | "ppage" => "\x1b[5~",
        "pagedown" | "npage" => "\x1b[6~",
        "delete" | "del" => "\x1b[3~",
        _ => {
            if let [b'c', b'-', ch] = lower.as_bytes() {
                if ch.is_ascii_lowercase() {
                    return Some(char::from(*ch - b'`').to_string());
                }
            }
            if name.chars().count() == 1 {
                return Some(name.to_owned());
            }
            return None;
        }
    };
    Some(sequence.to_owned())
}

pub const SEND_KEY_HELP: &str = "gsh send-key [--tab-id <id>] <Key>...\n  Press one or more keys in a terminal tab. Keys: Enter, Tab, Escape, Space,\n  BSpace, arrows, Home, End, PageUp, PageDown, Delete, C-<letter>.";
const NEW_WORKSPACE_HELP: &str = "gsh new-workspace <path> [--host <ssh-target>] [--name <title>] [--json]\n  Adds and selects a workspace, then ensures an initial terminal tab is open.\n  --host <target> remote workspace (not supported by the Rust app yet)\n  --name <title>  custom display title (default: folder name)";
const CLOSE_WORKSPACE_HELP: &str = "gsh close-workspace [<id|path>] [--json]\n  Removes a workspace from GSH's rail; files on disk are kept.\n  Without a target, closes the workspace owning the current tab.";
const RENAME_WORKSPACE_HELP: &str = "gsh rename-workspace [<id|path>] <new-name> [--json]\n  Renames the display title of a workspace in the rail.\n  Without a target, renames the workspace owning the current tab.";

fn parse_read_tab(args: &[String]) -> Result<Command, CliError> {
    let parsed = Flags::parse(args)?;
    if wants_help(args) {
        return Ok(Command::Help(READ_TAB_HELP));
    }
    Ok(Command::ReadTab {
        label: parsed
            .positionals
            .first()
            .cloned()
            .filter(|t| !t.is_empty()),
        lines: parsed.lines,
        offset: parsed.offset,
        from_start: parsed.from_start,
        target: parsed.target(),
        json: parsed.json,
    })
}

fn parse_rename_tab(args: &[String]) -> Result<Command, CliError> {
    let parsed = Flags::parse(args)?;
    if wants_help(args) {
        return Ok(Command::Help(RENAME_TAB_HELP));
    }
    let (label, title) = match parsed.positionals.as_slice() {
        [] => return Err(CliError::usage("gsh rename-tab: missing title")),
        [only] => (None, only.trim().to_string()),
        [target, rest @ ..] => (
            (!target.is_empty()).then(|| target.clone()),
            rest.join(" ").trim().to_string(),
        ),
    };
    if title.is_empty() {
        return Err(CliError::usage("gsh rename-tab: missing title"));
    }
    Ok(Command::RenameTab {
        label,
        title,
        target: parsed.target(),
        json: parsed.json,
    })
}

fn parse_interrupt(args: &[String]) -> Result<Command, CliError> {
    let parsed = Flags::parse(args)?;
    if wants_help(args) {
        return Ok(Command::Help(INTERRUPT_HELP));
    }
    Ok(Command::Interrupt {
        label: parsed
            .positionals
            .first()
            .cloned()
            .filter(|t| !t.is_empty()),
        force: parsed.force,
        target: parsed.target(),
        json: parsed.json,
    })
}

fn parse_wait_idle(args: &[String]) -> Result<Command, CliError> {
    let parsed = Flags::parse(args)?;
    if wants_help(args) {
        return Ok(Command::Help(WAIT_IDLE_HELP));
    }
    Ok(Command::WaitIdle {
        label: parsed
            .positionals
            .first()
            .cloned()
            .filter(|t| !t.is_empty()),
        target: parsed.target(),
        json: parsed.json,
    })
}

fn wants_help(args: &[String]) -> bool {
    args.iter().any(|a| a == "--help" || a == "-h")
}

/// `--harness <nome>` / `--harness=<nome>`. Default `claude`: entries de hook
/// instalados por versões anteriores não passam a flag, e são todos do Claude.
fn harness_from(args: &[String]) -> String {
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let value = if a == "--harness" {
            it.next().map(String::as_str)
        } else {
            a.strip_prefix("--harness=")
        };
        if let Some(v) = value.map(str::trim).filter(|v| !v.is_empty()) {
            return v.to_string();
        }
    }
    "claude".to_string()
}

fn parse_msg(args: &[String]) -> Result<Command, CliError> {
    if wants_help(args) {
        return Ok(Command::Help(MSG_HELP));
    }
    let (to, rest) = split_to(args);
    let parsed = Flags::parse(&rest)?;
    let to = to
        .filter(|t| !t.is_empty())
        .ok_or_else(|| CliError::usage(format!("gsh msg: missing --to\n{MSG_HELP}")))?;
    let text = parsed.positionals.join(" ");
    if text.is_empty() {
        return Err(CliError::usage("gsh msg: missing text"));
    }
    Ok(Command::Msg {
        to,
        text,
        json: parsed.json,
    })
}

fn parse_deliver(args: &[String]) -> Result<Command, CliError> {
    if wants_help(args) {
        return Ok(Command::Help(DELIVER_HELP));
    }
    let (to, rest) = split_to(args);
    let parsed = Flags::parse(&rest)?;
    let to = to
        .filter(|t| !t.is_empty())
        .ok_or_else(|| CliError::usage(format!("gsh deliver: missing --to\n{DELIVER_HELP}")))?;
    let Some((file, note)) = parsed.positionals.split_first() else {
        return Err(CliError::usage("gsh deliver: missing file"));
    };
    Ok(Command::Deliver {
        to,
        file: file.clone(),
        note: note.join(" "),
        json: parsed.json,
    })
}

fn parse_new_tab(args: &[String]) -> Result<Command, CliError> {
    let (mut cwd, mut title, mut split, mut tab_id) = (None, None, None, None);
    let mut json = false;
    let mut i = 0usize;
    while let Some(a) = args.get(i) {
        if a == "--help" || a == "-h" {
            return Ok(Command::Help(NEW_TAB_HELP));
        }
        if a == "--json" {
            json = true;
            i += 1;
            continue;
        }
        if let Some((flag, value)) =
            take(args, &mut i, &["--cwd", "--title", "--split", "--tab-id"])
        {
            match flag {
                "--cwd" => cwd = value,
                "--title" => title = value,
                "--split" => split = value,
                _ => tab_id = value,
            }
        }
        i += 1;
    }

    let split = match split {
        None => None,
        Some(raw) => match raw.to_lowercase().as_str() {
            "h" | "horizontal" | "right" => Some(Split::Right),
            "v" | "vertical" | "down" => Some(Split::Down),
            _ => {
                return Err(CliError::usage(format!(
                    "gsh new-tab: invalid --split \"{raw}\" (use h|right or v|down)"
                )))
            }
        },
    };
    Ok(Command::NewTab {
        cwd,
        title,
        split,
        target: Target {
            focused: false,
            tab_id,
        },
        json,
    })
}

fn parse_close_tab(args: &[String]) -> Result<Command, CliError> {
    // Ordem do original: o parse das flags vem antes do `--help`.
    let parsed = Flags::parse(args)?;
    if wants_help(args) {
        return Ok(Command::Help(CLOSE_TAB_HELP));
    }
    Ok(Command::CloseTab {
        label: parsed
            .positionals
            .first()
            .cloned()
            .filter(|t| !t.is_empty()),
        target: parsed.target(),
        json: parsed.json,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    fn usage_msg(r: Result<Command, CliError>) -> String {
        match r {
            Err(e @ CliError::Usage(_)) => {
                assert_eq!(e.exit_code(), 2);
                e.to_string()
            }
            other => panic!("esperava erro de uso, veio {other:?}"),
        }
    }

    fn db_usage_msg(r: Result<Command, CliError>) -> String {
        match r {
            Err(CliError::JsonFailure(error)) => {
                assert_eq!(CliError::JsonFailure(error.clone()).exit_code(), 1);
                error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap()
                    .to_owned()
            }
            other => panic!("esperava erro JSON de banco, veio {other:?}"),
        }
    }

    #[test]
    fn posicionais_e_tab_id() {
        let f = Flags::parse(&v(&["--tab-id", "t3", "olá", "mundo"])).unwrap();
        assert_eq!(f.tab_id.as_deref(), Some("t3"));
        assert_eq!(f.positionals, vec!["olá", "mundo"]);
    }

    #[test]
    fn forma_com_igual_e_aliases() {
        let f = Flags::parse(&v(&["--tab-id=t9", "--lines=25", "--offset=5"])).unwrap();
        assert_eq!(f.tab_id.as_deref(), Some("t9"));
        assert_eq!(f.lines, Some(25));
        assert_eq!(f.offset, Some(5));
        let f = Flags::parse(&v(&["-t", "t1", "-n", "10", "-f"])).unwrap();
        assert_eq!(f.tab_id.as_deref(), Some("t1"));
        assert_eq!(f.lines, Some(10));
        assert!(f.force);
    }

    #[test]
    fn separador_manda_o_resto_pra_posicionais() {
        let f = Flags::parse(&v(&["--json", "--", "--nao-e-flag", "-x"])).unwrap();
        assert!(f.json);
        assert_eq!(f.positionals, vec!["--nao-e-flag", "-x"]);
    }

    #[test]
    fn flag_sem_valor_ou_com_valor_ruim_e_erro_de_uso() {
        for (args, msg) in [
            (&["--lines"][..], "gsh: --lines requires a value"),
            (&["-n"], "gsh: --lines requires a value"),
            (&["--offset"], "gsh: --offset requires a value"),
            (&["--tab-id"], "gsh: --tab-id requires a value"),
            (&["-t"], "gsh: --tab-id requires a value"),
            (
                &["--lines", "x"],
                "gsh: --lines requires a non-negative integer",
            ),
            (
                &["--lines=-1"],
                "gsh: --lines requires a non-negative integer",
            ),
            (
                &["--offset=a"],
                "gsh: --offset requires a non-negative integer",
            ),
        ] {
            let err = Flags::parse(&v(args)).unwrap_err();
            assert_eq!(err.to_string(), msg, "{args:?}");
            assert_eq!(err.exit_code(), 2);
        }
    }

    #[test]
    fn alvo_focused_vence_tab_id_que_vence_o_ambiente() {
        let f = Flags::parse(&v(&["--focused", "--tab-id", "t1", "oi"])).unwrap();
        assert_eq!(
            f.target().resolve(Some("t7".into())).as_deref(),
            Some(FOCUSED)
        );
        assert_eq!(f.positionals, vec!["oi"], "a flag não vira texto");
        let f = Flags::parse(&v(&["--tab-id", "t1"])).unwrap();
        assert_eq!(f.target().resolve(Some("t7".into())).as_deref(), Some("t1"));
        let f = Flags::parse(&[]).unwrap();
        assert_eq!(f.target().resolve(Some("t7".into())).as_deref(), Some("t7"));
        assert_eq!(f.target().resolve(None), None);
    }

    #[test]
    fn sem_argumentos_e_ajuda_com_exit_2() {
        let err = parse(&[]).unwrap_err();
        assert_eq!(err.exit_code(), 2);
        assert_eq!(format!("{err}\n"), HELP);
    }

    #[test]
    fn ajuda_e_versao() {
        for a in ["--help", "-h", "help"] {
            assert_eq!(parse(&v(&[a])).unwrap(), Command::Help(HELP));
        }
        for a in ["--version", "-v"] {
            assert_eq!(parse(&v(&[a])).unwrap(), Command::Version);
        }
        // O original só conhece `--version`/`-v`; `-V` e `version` não são verbos.
        for a in ["-V", "version"] {
            assert_ne!(parse(&v(&[a])).ok(), Some(Command::Version), "{a}");
        }
    }

    #[test]
    fn mcp_requires_a_supported_desktop_agent() {
        assert_eq!(
            parse(&v(&["mcp", "--agent", "cursor"])).unwrap(),
            Command::Mcp {
                agent: "cursor".into()
            }
        );
        assert_eq!(
            parse(&v(&["mcp", "--agent=codex-desktop"])).unwrap(),
            Command::Mcp {
                agent: "codex-desktop".into()
            }
        );
        assert!(parse(&v(&["mcp", "--agent", "unknown"])).is_err());
    }

    #[test]
    fn open_exige_um_caminho() {
        assert_eq!(
            usage_msg(parse(&v(&["open"]))),
            "gsh open: missing file path"
        );
    }

    #[test]
    fn new_tab_ignora_focused_e_close_tab_valida_flags_antes_da_ajuda() {
        let Command::NewTab { target, .. } = parse(&v(&["new-tab", "--focused"])).unwrap() else {
            panic!("expected new-tab");
        };
        assert!(!target.focused);
        assert_eq!(
            usage_msg(parse(&v(&["close-tab", "--help", "--lines=bad"]))),
            "gsh: --lines requires a non-negative integer"
        );
    }

    #[test]
    fn msg_junta_o_texto_e_aceita_to_nas_duas_formas() {
        let want = Command::Msg {
            to: "Reviewer".into(),
            text: "diff ready".into(),
            json: false,
        };
        assert_eq!(
            parse(&v(&["msg", "--to", "Reviewer", "diff", "ready"])).unwrap(),
            want
        );
        assert_eq!(
            parse(&v(&["msg", "diff", "--to=Reviewer", "ready"])).unwrap(),
            want
        );
        let Command::Msg { json, .. } = parse(&v(&["msg", "--json", "--to", "a", "b"])).unwrap()
        else {
            panic!("esperava Msg");
        };
        assert!(json);
    }

    #[test]
    fn msg_separador_protege_texto_que_parece_flag() {
        let Command::Msg { text, to, .. } =
            parse(&v(&["msg", "--to", "a", "--", "--to", "x"])).unwrap()
        else {
            panic!("esperava Msg");
        };
        assert_eq!(to, "a");
        assert_eq!(text, "--to x");
    }

    #[test]
    fn msg_erros_de_uso() {
        assert_eq!(
            usage_msg(parse(&v(&["msg", "oi"]))),
            format!("gsh msg: missing --to\n{MSG_HELP}")
        );
        assert_eq!(
            usage_msg(parse(&v(&["msg", "--to"]))),
            format!("gsh msg: missing --to\n{MSG_HELP}")
        );
        assert_eq!(
            usage_msg(parse(&v(&["msg", "--to", "a"]))),
            "gsh msg: missing text"
        );
        // Erro de flag vem antes de --to faltando.
        assert_eq!(
            usage_msg(parse(&v(&["msg", "--lines"]))),
            "gsh: --lines requires a value"
        );
    }

    #[test]
    fn team_send_le_colega_agente_e_texto_para_o_app() {
        assert_eq!(
            parse(&v(&["send", "--to", "Bianca/financeiro", "diff", "ready",])).unwrap(),
            Command::TeamSend {
                to: "Bianca/financeiro".into(),
                text: "diff ready".into(),
                json: false,
            }
        );
        assert_eq!(
            parse(&v(&["send", "--to=Bianca/financeiro", "--json", "hello"])).unwrap(),
            Command::TeamSend {
                to: "Bianca/financeiro".into(),
                text: "hello".into(),
                json: true,
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["send", "--to", "Bianca/financeiro"]))),
            "gsh send: missing text"
        );
    }

    #[test]
    fn deliver_separa_arquivo_e_nota() {
        assert_eq!(
            parse(&v(&[
                "deliver", "--to", "B/fin", "r.pdf", "weekly", "report"
            ]))
            .unwrap(),
            Command::Deliver {
                to: "B/fin".into(),
                file: "r.pdf".into(),
                note: "weekly report".into(),
                json: false
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["deliver", "--to", "B/fin"]))),
            "gsh deliver: missing file"
        );
        assert_eq!(
            usage_msg(parse(&v(&["deliver", "r.pdf"]))),
            format!("gsh deliver: missing --to\n{DELIVER_HELP}")
        );
    }

    #[test]
    fn ajuda_de_verbo_vence_em_qualquer_posicao() {
        assert_eq!(
            parse(&v(&["msg", "--to", "a", "-h"])).unwrap(),
            Command::Help(MSG_HELP)
        );
        assert_eq!(
            parse(&v(&["deliver", "--help"])).unwrap(),
            Command::Help(DELIVER_HELP)
        );
        assert_eq!(
            parse(&v(&["new-tab", "--title", "x", "--help"])).unwrap(),
            Command::Help(NEW_TAB_HELP)
        );
        assert_eq!(
            parse(&v(&["close-tab", "-h"])).unwrap(),
            Command::Help(CLOSE_TAB_HELP)
        );
    }

    #[test]
    fn new_tab_le_flags_nas_duas_formas_e_ignora_o_resto() {
        let cmd = parse(&v(&[
            "new-tab", "--cwd=/p", "--title", "Worker", "--split", "H", "--tab-id", "t2", "--json",
            "lixo",
        ]))
        .unwrap();
        assert_eq!(
            cmd,
            Command::NewTab {
                cwd: Some("/p".into()),
                title: Some("Worker".into()),
                split: Some(Split::Right),
                target: Target {
                    focused: false,
                    tab_id: Some("t2".into())
                },
                json: true,
            }
        );
    }

    #[test]
    fn new_tab_split_aceita_sinonimos_e_recusa_o_resto() {
        for (raw, want) in [
            ("h", "right"),
            ("horizontal", "right"),
            ("RIGHT", "right"),
            ("v", "down"),
            ("Vertical", "down"),
            ("down", "down"),
        ] {
            let Command::NewTab { split: Some(s), .. } =
                parse(&v(&["new-tab", "--split", raw])).unwrap()
            else {
                panic!("esperava NewTab com split");
            };
            assert_eq!(s.wire(), want, "{raw}");
        }
        // A mensagem cita o valor como foi digitado.
        assert_eq!(
            usage_msg(parse(&v(&["new-tab", "--split", "Diag"]))),
            "gsh new-tab: invalid --split \"Diag\" (use h|right or v|down)"
        );
        // Sem valor: não há split (comportamento do original).
        let Command::NewTab { split, .. } = parse(&v(&["new-tab", "--split"])).unwrap() else {
            panic!("esperava NewTab");
        };
        assert_eq!(split, None);
    }

    #[test]
    fn close_tab_alvo_posicional_opcional() {
        assert_eq!(
            parse(&v(&["close-tab"])).unwrap(),
            Command::CloseTab {
                label: None,
                target: Target::default(),
                json: false
            }
        );
        let Command::CloseTab { label, json, .. } =
            parse(&v(&["close-tab", "--json", "Extension"])).unwrap()
        else {
            panic!("esperava CloseTab");
        };
        assert_eq!(label.as_deref(), Some("Extension"));
        assert!(json);
    }

    #[test]
    fn list_tabs_e_o_alias_legado() {
        let want = Command::ListTabs {
            target: Target {
                focused: false,
                tab_id: Some("t1".into()),
            },
            json: true,
        };
        assert_eq!(
            parse(&v(&["list-tabs", "--json", "-t", "t1"])).unwrap(),
            want
        );
        assert_eq!(
            parse(&v(&["list-panes", "--json", "-t", "t1"])).unwrap(),
            want
        );
    }

    #[test]
    fn hook_harness_default_e_claude() {
        assert_eq!(harness_from(&[]), "claude");
        assert_eq!(harness_from(&v(&["--harness"])), "claude");
        assert_eq!(harness_from(&v(&["--harness", "  "])), "claude");
        assert_eq!(harness_from(&v(&["--harness", "codex"])), "codex");
        assert_eq!(harness_from(&v(&["--harness=codex"])), "codex");
    }

    #[test]
    fn open_aceita_focused_tab_id_e_atalho_sem_verbo() {
        assert_eq!(
            parse(&v(&["open", "--focused", "README.md"])).unwrap(),
            Command::Open {
                path: "README.md".into(),
                target: Target {
                    focused: true,
                    tab_id: None,
                },
            }
        );
        assert_eq!(
            parse(&v(&["open", "--tab-id=t4", "README.md"])).unwrap(),
            Command::Open {
                path: "README.md".into(),
                target: Target {
                    focused: false,
                    tab_id: Some("t4".into()),
                },
            }
        );
        assert_eq!(
            parse(&v(&["--focused", "README.md"])).unwrap(),
            Command::Open {
                path: "README.md".into(),
                target: Target {
                    focused: true,
                    tab_id: None,
                },
            }
        );
    }

    #[test]
    fn read_tab_e_alias_read_pane_leem_paginacao_e_alvo() {
        let want = Command::ReadTab {
            label: Some("Extension".into()),
            lines: Some(50),
            offset: Some(10),
            from_start: true,
            target: Target {
                focused: false,
                tab_id: Some("t2".into()),
            },
            json: true,
        };
        for verb in ["read-tab", "read-pane"] {
            assert_eq!(
                parse(&v(&[
                    verb,
                    "Extension",
                    "--lines",
                    "50",
                    "--offset=10",
                    "--from-start",
                    "-t",
                    "t2",
                    "--json",
                ]))
                .unwrap(),
                want
            );
        }
        assert_eq!(
            parse(&v(&["read-tab", "--help"])).unwrap(),
            Command::Help(READ_TAB_HELP)
        );
    }

    #[test]
    fn rename_tab_interrupt_e_wait_idle_parseiam_alvo_e_flags() {
        assert_eq!(
            parse(&v(&["rename-tab", "Worker"])).unwrap(),
            Command::RenameTab {
                label: None,
                title: "Worker".into(),
                target: Target::default(),
                json: false,
            }
        );
        assert_eq!(
            parse(&v(&["rename-tab", "t3", "New", "Worker", "--json"])).unwrap(),
            Command::RenameTab {
                label: Some("t3".into()),
                title: "New Worker".into(),
                target: Target::default(),
                json: true,
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["rename-tab"]))),
            "gsh rename-tab: missing title"
        );
        assert_eq!(
            parse(&v(&["rename-tab", "-h"])).unwrap(),
            Command::Help(RENAME_TAB_HELP)
        );

        assert_eq!(
            parse(&v(&["interrupt", "Worker", "--force", "--json"])).unwrap(),
            Command::Interrupt {
                label: Some("Worker".into()),
                force: true,
                target: Target::default(),
                json: true,
            }
        );
        assert_eq!(
            parse(&v(&["interrupt", "--help"])).unwrap(),
            Command::Help(INTERRUPT_HELP)
        );

        assert_eq!(
            parse(&v(&["wait-idle", "Worker", "--tab-id", "t5", "--json"])).unwrap(),
            Command::WaitIdle {
                label: Some("Worker".into()),
                target: Target {
                    focused: false,
                    tab_id: Some("t5".into()),
                },
                json: true,
            }
        );
        assert_eq!(
            parse(&v(&["wait-idle", "-h"])).unwrap(),
            Command::Help(WAIT_IDLE_HELP)
        );
    }

    #[test]
    fn send_de_aba_usa_enter_e_target_quando_sem_to() {
        assert_eq!(
            parse(&v(&["send", "--tab-id", "t4", "--enter", "npm", "test"])).unwrap(),
            Command::Send {
                text: "npm test".into(),
                enter: true,
                target: Target {
                    focused: false,
                    tab_id: Some("t4".into()),
                },
            }
        );
        assert_eq!(
            parse(&v(&["send", "--focused", "oi"])).unwrap(),
            Command::Send {
                text: "oi".into(),
                enter: false,
                target: Target {
                    focused: true,
                    tab_id: None,
                },
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["send", "--enter"]))),
            "gsh send: missing text to send"
        );
        assert_eq!(
            parse(&v(&["send", "oi"])).unwrap(),
            Command::Send {
                text: "oi".into(),
                enter: false,
                target: Target::default()
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["send", "--to=", "oi"]))),
            "gsh send: missing --to"
        );
        assert_eq!(
            usage_msg(parse(&v(&["send", "oi", "--to"]))),
            "gsh send: missing --to"
        );
        assert_eq!(
            usage_msg(parse(&v(&["send"]))),
            "gsh send: missing text to send"
        );
    }

    #[test]
    fn send_key_parseia_target_e_tabela_completa_e_rejeita_nome_invalido() {
        assert_eq!(
            parse(&v(&["send-key", "--tab-id", "t9", "Enter", "C-z"])).unwrap(),
            Command::SendKey {
                keys: vec!["Enter".into(), "C-z".into()],
                target: Target {
                    focused: false,
                    tab_id: Some("t9".into())
                }
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["send-key"]))),
            "gsh send-key: missing key (e.g. Enter, C-c, Escape)"
        );
        assert_eq!(
            usage_msg(parse(&v(&["send-key", "NoSuchKey"]))),
            "gsh send-key: unknown key \"NoSuchKey\""
        );
        assert!(matches!(
            parse(&v(&["send-key", "--help"])).unwrap(),
            Command::Help(_)
        ));
        let named = [
            ("Enter", "\r"),
            ("Tab", "\t"),
            ("Escape", "\x1b"),
            ("Space", " "),
            ("BSpace", "\x7f"),
            ("Up", "\x1b[A"),
            ("Down", "\x1b[B"),
            ("Left", "\x1b[D"),
            ("Right", "\x1b[C"),
            ("Home", "\x1b[H"),
            ("End", "\x1b[F"),
            ("PageUp", "\x1b[5~"),
            ("PageDown", "\x1b[6~"),
            ("Delete", "\x1b[3~"),
        ];
        for (name, expected) in named {
            assert_eq!(resolve_key(name).as_deref(), Some(expected), "{name}");
        }
        for ch in b'a'..=b'z' {
            assert_eq!(
                resolve_key(&format!("C-{}", char::from(ch)))
                    .unwrap()
                    .as_bytes(),
                &[ch - b'`']
            );
        }
    }

    #[test]
    fn workspace_verbs_parseiam_argumentos_e_validam_uso() {
        assert_eq!(
            parse(&v(&[
                "new-workspace",
                "/tmp/project",
                "--host",
                "ssh",
                "--name",
                "Demo",
                "--json"
            ]))
            .unwrap(),
            Command::NewWorkspace {
                path: "/tmp/project".into(),
                host: Some("ssh".into()),
                name: Some("Demo".into()),
                tab_id: None,
                json: true,
            }
        );
        assert_eq!(
            parse(&v(&["close-workspace", "/tmp/project", "--json"])).unwrap(),
            Command::CloseWorkspace {
                target: Some("/tmp/project".into()),
                tab_id: None,
                json: true,
            }
        );
        assert_eq!(
            parse(&v(&["rename-workspace", "ws-1", "Demo"])).unwrap(),
            Command::RenameWorkspace {
                target: Some("ws-1".into()),
                name: "Demo".into(),
                tab_id: None,
                json: false,
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["new-workspace"]))),
            "gsh new-workspace: missing <path> (or --path <path>)"
        );
        assert_eq!(
            usage_msg(parse(&v(&["new-workspace", "/tmp/project", "--host"]))),
            "gsh new-workspace: --host requires a value"
        );
        assert_eq!(
            usage_msg(parse(&v(&["rename-workspace"]))),
            "gsh rename-workspace: missing workspace name"
        );
        assert_eq!(
            usage_msg(parse(&v(&["close-workspace", "--unknown"]))),
            "gsh: unknown flag \"--unknown\""
        );
        for args in [
            v(&["new-workspace", "--help"]),
            v(&["close-workspace", "--help"]),
            v(&["rename-workspace", "--help"]),
            v(&["exec", "--help"]),
        ] {
            assert!(matches!(parse(&args).unwrap(), Command::Help(_)));
        }
    }

    #[test]
    fn exec_preserva_linha_shell_e_valida_flags() {
        assert_eq!(
            parse(&v(&[
                "exec",
                "--cwd",
                "/tmp",
                "--timeout",
                "4",
                "--json",
                "--",
                "echo",
                "oi"
            ]))
            .unwrap(),
            Command::Exec {
                command: "echo oi".into(),
                cwd: Some("/tmp".into()),
                timeout: Some(4),
                tab_id: None,
                json: true,
            }
        );
        assert_eq!(
            usage_msg(parse(&v(&["exec", "--cwd"]))),
            "gsh exec: --cwd requires a value"
        );
        assert_eq!(
            usage_msg(parse(&v(&["exec", "--timeout", "-1", "echo"]))),
            "gsh exec: --timeout requires a non-negative integer"
        );
        assert!(
            usage_msg(parse(&v(&["exec", "--bad"]))).starts_with("gsh exec: unknown flag --bad\n")
        );
        assert!(usage_msg(parse(&v(&["exec"]))).starts_with("gsh exec: missing command\n"));
    }

    #[test]
    fn db_list_monta_wire_e_help_de_uso() {
        assert_eq!(
            parse(&v(&["db", "list", "--workspace", "ws-1", "--tab-id", "t7"])).unwrap(),
            Command::Database {
                wire: "db-list",
                args: serde_json::json!({"workspace":"ws-1"}),
                tab_id: Some("t7".into()),
            }
        );
        assert!(matches!(
            parse(&v(&["db", "--help"])).unwrap(),
            Command::Help(_)
        ));
        assert!(db_usage_msg(parse(&v(&["db", "list", "--bad=x"]))).contains("unknown flag"));
    }

    #[test]
    fn db_schema_preserva_tabela_e_sql_query() {
        assert_eq!(
            parse(&v(&["db", "schema", "--db", "local", "--table", "users"])).unwrap(),
            Command::Database {
                wire: "db-schema",
                args: serde_json::json!({"db":"local","table":"users"}),
                tab_id: None,
            }
        );
        assert_eq!(
            parse(&v(&["db", "query", "--db", "local", "SELECT", "1"])).unwrap(),
            Command::Database {
                wire: "db-query",
                args: serde_json::json!({"db":"local","sql":"SELECT 1"}),
                tab_id: None,
            }
        );
    }

    #[test]
    fn db_run_resolve_caminho_e_workspace() {
        assert_eq!(
            parse(&v(&["db", "run", "/tmp/query.dbq", "--workspace", "ws-1"])).unwrap(),
            Command::Database {
                wire: "db-run",
                args: serde_json::json!({"path":"/tmp/query.dbq","workspace":"ws-1"}),
                tab_id: None,
            }
        );
        assert!(db_usage_msg(parse(&v(&["db", "run"]))).contains("missing <file.dbq>"));
    }

    #[test]
    fn redis_and_mongo_parse_commands_and_workspace_flags() {
        assert_eq!(
            parse(&v(&[
                "redis",
                "--db",
                "cache",
                "GET",
                "user:1",
                "--workspace",
                "ws",
                "--tab-id",
                "t4"
            ]))
            .unwrap(),
            Command::Database {
                wire: "redis-cmd",
                args: serde_json::json!({"db":"cache","parts":["GET","user:1"],"workspace":"ws"}),
                tab_id: Some("t4".into()),
            }
        );
        assert_eq!(
            parse(&v(&[
                "mongo",
                "--db",
                "docs",
                "--database",
                "app",
                "--command",
                "{\"find\":\"users\"}"
            ]))
            .unwrap(),
            Command::Database {
                wire: "mongo-cmd",
                args: serde_json::json!({"db":"docs","database":"app","command":"{\"find\":\"users\"}"}),
                tab_id: None,
            }
        );
    }

    #[test]
    fn database_usage_errors_are_json_exit_one_and_help_is_success() {
        for argv in [
            v(&["db", "list", "--bad=x"]),
            v(&["redis"]),
            v(&["mongo", "--db"]),
        ] {
            let Err(CliError::JsonFailure(error)) = parse(&argv) else {
                panic!("expected JSON database usage error for {argv:?}");
            };
            assert_eq!(CliError::JsonFailure(error.clone()).exit_code(), 1);
            assert_eq!(error.get("kind").and_then(Value::as_str), Some("error"));
            assert!(error.get("message").and_then(Value::as_str).is_some());
        }
        assert!(matches!(
            parse(&v(&["redis", "--help"])),
            Ok(Command::Help(_))
        ));
        assert!(matches!(
            parse(&v(&["mongo", "--help"])),
            Ok(Command::Help(_))
        ));
    }
}
