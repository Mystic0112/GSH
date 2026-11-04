//! Execução dos verbos: monta a requisição com `gsh-core`, fala com o app pelo
//! [`transport`] e formata a saída. Cada verbo devolve o texto do stdout (já
//! com a quebra de linha final); erro e código de saída vêm de [`CliError`].

use std::time::Duration;

use gsh_core::tabs::format_tab_list;
use gsh_core::wire::{cmd_request, Response};
use serde_json::{json, Map, Value};

use crate::args::{Command, Target};
use crate::env::{resolve_path, self_tab_id};
use crate::error::CliError;
use crate::{hook, transport};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Executa o comando. `Ok(None)` = nada no stdout.
pub fn run(cmd: Command) -> Result<Option<String>, CliError> {
    match cmd {
        Command::Help(text) => Ok(Some(with_newline(text))),
        Command::Version => Ok(Some(format!("gsh {}\n", gsh_core::VERSION))),
        Command::Hook { harness } => hook::run(&harness),
        Command::Msg { to, text, json } => msg(&to, &text, json).map(Some),
        Command::Deliver {
            to,
            file,
            note,
            json,
        } => deliver(&to, &file, &note, json).map(Some),
        Command::NewTab {
            cwd,
            title,
            split,
            target,
            json,
        } => new_tab(cwd, title, split.map(|s| s.wire()), &target, json).map(Some),
        Command::CloseTab {
            label,
            target,
            json,
        } => close_tab(label, &target, json).map(Some),
        Command::ListTabs { target, json } => list_tabs(&target, json),
    }
}

fn with_newline(text: &str) -> String {
    if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
}

/// Envia e exige `ok: true`. O aviso do app vai pro stderr (nunca pro stdout,
/// que pode ser uma linha JSON que agentes parseiam).
fn call(req: Value) -> Result<Response, CliError> {
    let resp = transport::request(req, DEFAULT_TIMEOUT)?;
    if let Some(w) = resp.warning() {
        eprintln!("gsh: {w}");
    }
    if resp.is_ok() {
        Ok(resp)
    } else {
        Err(CliError::App(resp.error_text()))
    }
}

/// `--json` ecoa o `data` inteiro; sem ele, só o campo `key` (string).
fn print_data(data: &Value, key: &str, json: bool) -> String {
    if json {
        format!("{data}\n")
    } else {
        format!("{}\n", data.get(key).and_then(Value::as_str).unwrap_or(""))
    }
}

fn msg(to: &str, text: &str, json: bool) -> Result<String, CliError> {
    let from = self_tab_id().ok_or_else(|| {
        CliError::usage(
            "gsh msg: GSH_TAB_ID is unset — run it inside a GSH terminal \
(the sender is the current tab)",
        )
    })?;
    let resp = call(cmd_request(
        "msg",
        Some(&from),
        Some(json!({"to": to, "text": text})),
    ))?;
    Ok(print_data(&resp.data(), "to", json))
}

fn deliver(to: &str, file: &str, note: &str, json: bool) -> Result<String, CliError> {
    let from = self_tab_id().ok_or_else(|| {
        CliError::usage("gsh deliver: GSH_TAB_ID is unset — run it inside a GSH terminal")
    })?;
    // O app tem cwd próprio: manda o caminho absoluto.
    let resp = call(cmd_request(
        "deliver",
        Some(&from),
        Some(json!({"to": to, "path": resolve_path(file), "text": note})),
    ))?;
    Ok(print_data(&resp.data(), "to", json))
}

fn new_tab(
    cwd: Option<String>,
    title: Option<String>,
    split: Option<&str>,
    target: &Target,
    json: bool,
) -> Result<String, CliError> {
    let cwd = resolve_path(&cwd.unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| ".".to_string())
    }));
    let mut args = Map::new();
    args.insert("cwd".into(), json!(cwd));
    if let Some(t) = title.filter(|t| !t.is_empty()) {
        args.insert("title".into(), json!(t));
    }
    if let Some(s) = split {
        args.insert("split".into(), json!(s));
    }
    let tab = target.resolve(self_tab_id());
    let resp = call(cmd_request(
        "new-tab",
        tab.as_deref(),
        Some(Value::Object(args)),
    ))?;
    Ok(print_data(&resp.data(), "tabId", json))
}

fn close_tab(label: Option<String>, target: &Target, json: bool) -> Result<String, CliError> {
    let mut args = Map::new();
    if let Some(t) = label {
        args.insert("target".into(), json!(t));
    }
    // Sem alvo posicional, o app cai na própria tab ($GSH_TAB_ID).
    let tab = target.resolve(self_tab_id());
    let resp = call(cmd_request(
        "close-tab",
        tab.as_deref(),
        Some(Value::Object(args)),
    ))?;
    Ok(print_data(&resp.data(), "tabId", json))
}

fn list_tabs(target: &Target, json: bool) -> Result<Option<String>, CliError> {
    // O verbo é `list-tabs`, o comando de wire continua `list-panes`.
    let tab = target.resolve(self_tab_id());
    let resp = call(cmd_request("list-panes", tab.as_deref(), None))?;
    let data = resp.data_list();
    if json {
        return Ok(Some(format!("{:#}\n", Value::Array(data))));
    }
    if data.is_empty() {
        return Ok(Some("(none)\n".to_string()));
    }
    // Lista não vazia só de entradas que não são objeto: o original não imprime nada.
    let lines = format_tab_list(&data);
    Ok((!lines.is_empty()).then(|| format!("{}\n", lines.join("\n"))))
}
