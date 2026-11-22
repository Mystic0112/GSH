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
        Command::Mcp { agent } => crate::mcp::run(&agent),
        Command::Msg { to, text, json } => msg(&to, &text, json).map(Some),
        Command::TeamSend { to, text, json } => team_send(&to, &text, json).map(Some),
        Command::Send {
            text,
            enter,
            target,
        } => send_to_tab(&text, enter, &target),
        Command::TeamPeers { json } => team_peers(json),
        Command::Deliver {
            to,
            file,
            note,
            json,
        } => deliver(&to, &file, &note, json).map(Some),
        Command::Open { path, target } => {
            call(cmd_request(
                "open",
                target.resolve(self_tab_id()).as_deref(),
                Some(json!({"path": resolve_path(&path)})),
            ))?;
            Ok(None)
        }
        Command::ReadTab {
            label,
            lines,
            offset,
            from_start,
            target,
            json,
        } => read_tab(label, lines, offset, from_start, &target, json),
        Command::RenameTab {
            label,
            title,
            target,
            json,
        } => rename_tab(label, &title, &target, json).map(Some),
        Command::Interrupt {
            label,
            force,
            target,
            json,
        } => interrupt_tab(label, force, &target, json).map(Some),
        Command::WaitIdle {
            label,
            target,
            json,
        } => wait_idle(label, &target, json).map(Some),
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
        Command::Telemetry(_) => Ok(None),
        Command::LocalSend { args } => crate::localsend::run(&args),
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
    call_with_timeout(req, DEFAULT_TIMEOUT)
}

fn call_with_timeout(req: Value, timeout: Duration) -> Result<Response, CliError> {
    let resp = transport::request(req, timeout)?;
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

fn format_team_send_success(data: &Value, fallback: &str, json: bool) -> String {
    if json {
        format!("{data}\n")
    } else {
        let destination = data.get("to").and_then(Value::as_str).unwrap_or(fallback);
        format!("Sent to {destination}.\n")
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

fn team_send(to: &str, text: &str, json: bool) -> Result<String, CliError> {
    let from = self_tab_id().ok_or_else(|| {
        CliError::usage("gsh send: GSH_TAB_ID is unset — run it inside a GSH terminal")
    })?;
    let resp = call_with_timeout(
        cmd_request("send", Some(&from), Some(json!({"to": to, "text": text}))),
        Duration::from_secs(45),
    )?;
    Ok(format_team_send_success(&resp.data(), to, json))
}

fn team_peers(json: bool) -> Result<Option<String>, CliError> {
    let from = self_tab_id().ok_or_else(|| {
        CliError::usage("gsh peers: GSH_TAB_ID is unset — run it inside a GSH terminal")
    })?;
    let resp = call(cmd_request("peers", Some(&from), None))?;
    let data = resp.data();
    if json {
        return Ok(Some(format!("{data:#}\n")));
    }
    let Some(entries) = data.as_array() else {
        return Ok(Some("(none)\n".into()));
    };
    if entries.is_empty() {
        return Ok(Some("(none)\n".into()));
    }
    let lines = entries
        .iter()
        .filter_map(|entry| entry.get("label").and_then(Value::as_str))
        .collect::<Vec<_>>();
    Ok((!lines.is_empty()).then(|| format!("{}\n", lines.join("\n"))))
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

fn send_to_tab(text: &str, enter: bool, target: &Target) -> Result<Option<String>, CliError> {
    let tab_id = target.resolve(self_tab_id()).ok_or_else(|| {
        CliError::usage(
            "gsh: no target — pass --tab-id <id> or run inside a GSH \
terminal (GSH_TAB_ID is unset). Use `gsh list-tabs`.",
        )
    })?;
    let response = write_once(&tab_id, text)?;
    if enter {
        let data = response.data();
        let destination = data.get("tabId").and_then(Value::as_str).unwrap_or(&tab_id);
        write_once(destination, "\r")?;
    }
    Ok(None)
}

fn write_once(tab_id: &str, text: &str) -> Result<Response, CliError> {
    call(cmd_request(
        "write",
        Some(tab_id),
        Some(json!({"data": encode_base64(text.as_bytes())})),
    ))
}

fn read_tab(
    label: Option<String>,
    lines: Option<u64>,
    offset: Option<u64>,
    from_start: bool,
    target: &Target,
    json: bool,
) -> Result<Option<String>, CliError> {
    let mut args = Map::new();
    if let Some(t) = label {
        args.insert("target".into(), json!(t));
    }
    if let Some(n) = lines {
        args.insert("lines".into(), json!(n));
    }
    if let Some(n) = offset {
        args.insert("offset".into(), json!(n));
    }
    if from_start {
        args.insert("fromStart".into(), json!(true));
    }
    let tab = target.resolve(self_tab_id());
    let resp = call(cmd_request(
        "read-pane",
        tab.as_deref(),
        Some(Value::Object(args)),
    ))?;
    let data = resp.data();
    if json {
        return Ok(Some(format!("{data}\n")));
    }
    let encoded = data.get("text").and_then(Value::as_str).unwrap_or("");
    let text =
        decode_base64_utf8(encoded).map_err(|()| CliError::App("malformed payload".to_string()))?;
    if data.get("truncated") == Some(&Value::Bool(true)) {
        eprintln!("gsh: output truncated (server cap 2000 lines/read — page with --offset)");
    }
    Ok((!text.is_empty()).then(|| format!("{text}\n")))
}

fn rename_tab(
    label: Option<String>,
    title: &str,
    target: &Target,
    json: bool,
) -> Result<String, CliError> {
    let mut args = Map::new();
    if let Some(t) = label {
        args.insert("target".into(), json!(t));
    }
    args.insert("title".into(), json!(title));
    let tab = target.resolve(self_tab_id());
    let resp = call(cmd_request(
        "rename-tab",
        tab.as_deref(),
        Some(Value::Object(args)),
    ))?;
    Ok(print_data(&resp.data(), "tabId", json))
}

fn interrupt_tab(
    label: Option<String>,
    force: bool,
    target: &Target,
    json: bool,
) -> Result<String, CliError> {
    let mut args = Map::new();
    if let Some(t) = label {
        args.insert("target".into(), json!(t));
    }
    if force {
        args.insert("force".into(), json!(true));
    }
    let tab = target.resolve(self_tab_id());
    let resp = call(cmd_request(
        "interrupt",
        tab.as_deref(),
        Some(Value::Object(args)),
    ))?;
    Ok(print_data(&resp.data(), "tabId", json))
}

fn wait_idle(label: Option<String>, target: &Target, json: bool) -> Result<String, CliError> {
    let mut args = Map::new();
    if let Some(t) = label {
        args.insert("target".into(), json!(t));
    }
    let tab = target.resolve(self_tab_id());
    let resp = call_with_timeout(
        cmd_request("wait-idle", tab.as_deref(), Some(Value::Object(args))),
        Duration::from_secs(45),
    )?;
    Ok(print_data(&resp.data(), "tabId", json))
}

const BASE64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_char(idx: usize) -> char {
    BASE64_ALPHABET
        .get(idx)
        .copied()
        .map(char::from)
        .unwrap_or('A')
}

fn encode_base64(input: &[u8]) -> String {
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk.first().copied().unwrap_or(0) as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(b64_char((n >> 18) & 0x3f));
        out.push(b64_char((n >> 12) & 0x3f));
        if chunk.len() > 1 {
            out.push(b64_char((n >> 6) & 0x3f));
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(b64_char(n & 0x3f));
        } else {
            out.push('=');
        }
    }
    out
}

fn b64_val(byte: u8) -> Option<u32> {
    match byte {
        b'A'..=b'Z' => Some(u32::from(byte - b'A')),
        b'a'..=b'z' => Some(u32::from(byte - b'a' + 26)),
        b'0'..=b'9' => Some(u32::from(byte - b'0' + 52)),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn decode_base64_utf8(encoded: &str) -> Result<String, ()> {
    let bytes = encoded.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err(());
    }
    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    for chunk in bytes.chunks(4) {
        let c0 = chunk.first().copied().ok_or(())?;
        let c1 = chunk.get(1).copied().ok_or(())?;
        let c2 = chunk.get(2).copied().ok_or(())?;
        let c3 = chunk.get(3).copied().ok_or(())?;
        let v0 = b64_val(c0).ok_or(())?;
        let v1 = b64_val(c1).ok_or(())?;
        let v2 = if c2 == b'=' {
            0
        } else {
            b64_val(c2).ok_or(())?
        };
        let v3 = if c3 == b'=' {
            0
        } else {
            b64_val(c3).ok_or(())?
        };
        if c2 == b'=' && c3 != b'=' {
            return Err(());
        }
        let triple = (v0 << 18) | (v1 << 12) | (v2 << 6) | v3;
        out.push(((triple >> 16) & 0xff) as u8);
        if c2 != b'=' {
            out.push(((triple >> 8) & 0xff) as u8);
        }
        if c3 != b'=' {
            out.push((triple & 0xff) as u8);
        }
    }
    String::from_utf8(out).map_err(|_| ())
}

#[cfg(test)]
mod team_send_output_tests {
    use super::{decode_base64_utf8, encode_base64, format_team_send_success};
    use serde_json::json;

    #[test]
    fn successful_send_prints_confirmation_and_json_keeps_flutter_data() {
        let data = json!({"from": "t1", "to": "beta/worker", "team": true});
        assert_eq!(
            format_team_send_success(&data, "ignored", false),
            "Sent to beta/worker.\n"
        );
        assert_eq!(
            format_team_send_success(&data, "ignored", true),
            "{\"from\":\"t1\",\"team\":true,\"to\":\"beta/worker\"}\n"
        );
    }

    #[test]
    fn base64_codec_roundtrips_utf8_and_rejects_malformed() {
        assert_eq!(encode_base64("olá mundo".as_bytes()), "b2zDoSBtdW5kbw==");
        assert_eq!(encode_base64(b"npm test"), "bnBtIHRlc3Q=");
        assert_eq!(encode_base64(b"\r"), "DQ==");
        assert_eq!(
            decode_base64_utf8("bGluaGExCmxpbmhhMg==").unwrap(),
            "linha1\nlinha2"
        );
        assert!(decode_base64_utf8("abc").is_err());
        assert!(decode_base64_utf8("ab=c").is_err());
    }
}
