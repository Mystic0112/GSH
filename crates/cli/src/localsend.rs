//! CLI LocalSend: todo o trabalho de rede acontece no app; esta camada só fala IPC.

use std::fmt::Write as _;
use std::path::Path;
use std::time::Duration;

use gsh_core::wire::cmd_request;
use serde_json::{json, Value};

use crate::env::{resolve_path, self_tab_id};
use crate::error::CliError;
use crate::transport;

const MAX_WAIT_SECONDS: u64 = 30;
const MAX_SEND_TIMEOUT_SECONDS: u64 = 300;

pub fn run(args: &[String]) -> Result<Option<String>, CliError> {
    let Some(action) = args.first().map(String::as_str) else {
        return Err(CliError::usage("gsh localsend: expected peers or send"));
    };
    match action {
        "peers" => peers(args.get(1..).unwrap_or_default()),
        "send" => send(args.get(1..).unwrap_or_default()),
        _ => Err(CliError::usage(format!(
            "gsh localsend: unknown action: {action}"
        ))),
    }
}

fn peers(args: &[String]) -> Result<Option<String>, CliError> {
    let mut json_output = false;
    let mut wait = 3_u64;
    let mut index = 0;
    while let Some(arg) = args.get(index) {
        match arg.as_str() {
            "--json" => json_output = true,
            "--wait" => {
                index += 1;
                wait = parse_seconds(args.get(index), "--wait", MAX_WAIT_SECONDS)?;
            }
            _ => {
                return Err(CliError::usage(format!(
                    "gsh localsend peers: unknown argument {arg}"
                )))
            }
        }
        index += 1;
    }
    let response = request(
        "localsend-peers",
        json!({"waitSeconds": wait, "json": json_output}),
        Duration::from_secs(wait + 10),
    )?;
    let data = response.data();
    if json_output {
        return Ok(Some(format!("{}\n", terminal_json(format!("{data:#}")))));
    }
    let Some(entries) = data.as_array() else {
        return Ok(Some("(none)\n".into()));
    };
    if entries.is_empty() {
        return Ok(Some("(none)\n".into()));
    }
    let lines = entries
        .iter()
        .map(|peer| {
            let alias = peer
                .pointer("/info/alias")
                .and_then(Value::as_str)
                .unwrap_or("?");
            let ip = peer.get("ip").and_then(Value::as_str).unwrap_or("?");
            let port = peer
                .pointer("/info/port")
                .and_then(Value::as_u64)
                .unwrap_or(53317);
            let model = peer
                .pointer("/info/deviceModel")
                .and_then(Value::as_str)
                .unwrap_or("");
            format!(
                "{}\t{}:{port}\t{}",
                escape_terminal_text(alias),
                escape_terminal_text(ip),
                escape_terminal_text(model)
            )
        })
        .collect::<Vec<_>>();
    Ok(Some(format!("{}\n", lines.join("\n"))))
}

fn send(args: &[String]) -> Result<Option<String>, CliError> {
    let Some(target) = args.first() else {
        return Err(CliError::usage("gsh localsend send: missing <alias|ip>"));
    };
    let mut timeout = 120_u64;
    let mut paths = Vec::new();
    let mut index = 1;
    while let Some(arg) = args.get(index) {
        if arg == "--timeout" {
            index += 1;
            timeout = parse_seconds(args.get(index), "--timeout", MAX_SEND_TIMEOUT_SECONDS)?;
        } else if arg.starts_with("--") {
            return Err(CliError::usage(format!(
                "gsh localsend send: unknown option {arg}"
            )));
        } else {
            let path = resolve_path(arg);
            if !Path::new(&path).is_file() {
                return Err(CliError::App(format!(
                    "file not found or not a regular file: {path}"
                )));
            }
            paths.push(path);
        }
        index += 1;
    }
    if paths.is_empty() {
        return Err(CliError::usage("gsh localsend send: missing file"));
    }
    let response = request(
        "localsend-send",
        json!({
            "target": target,
            "files": paths,
            "timeoutSeconds": timeout,
        }),
        Duration::from_secs(timeout + 15),
    )?;
    let data = response.data();
    if data.get("status").and_then(Value::as_str) == Some("sent") {
        let peer = data.get("peer").and_then(Value::as_str).unwrap_or(target);
        let count = data.get("filesSent").and_then(Value::as_u64).unwrap_or(0);
        return Ok(Some(format!(
            "Sent {count} file(s) to {}.\n",
            escape_terminal_text(peer)
        )));
    }
    Ok(Some(format!("{}\n", terminal_json(data.to_string()))))
}

fn escape_terminal_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if character.is_control()
            || matches!(
                character,
                '\u{061c}' | '\u{200e}'..='\u{200f}' | '\u{2028}'..='\u{202e}' | '\u{2066}'..='\u{206f}'
            )
        {
            let _ = write!(escaped, "\\u{:04x}", u32::from(character));
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn terminal_json(serialized: String) -> String {
    // Any literal line breaks here are indentation from the JSON formatter.
    serialized
        .split('\n')
        .map(escape_terminal_text)
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_seconds(value: Option<&String>, option: &str, max: u64) -> Result<u64, CliError> {
    let value = value.ok_or_else(|| CliError::usage(format!("{option} requires seconds")))?;
    let seconds = value
        .parse::<u64>()
        .map_err(|_| CliError::usage(format!("invalid {option} value: {value}")))?;
    if seconds == 0 || seconds > max {
        return Err(CliError::usage(format!(
            "{option} must be between 1 and {max} seconds"
        )));
    }
    Ok(seconds)
}

fn request(
    command: &str,
    args: Value,
    timeout: Duration,
) -> Result<gsh_core::wire::Response, CliError> {
    let tab_id = self_tab_id().ok_or_else(|| {
        CliError::usage("gsh localsend: GSH_TAB_ID is unset — run it inside a GSH terminal")
    })?;
    let req = cmd_request(command, Some(&tab_id), Some(args));
    let response = transport::request(req, timeout)?;
    if let Some(warning) = response.warning() {
        eprintln!("gsh: {}", escape_terminal_text(warning));
    }
    if response.is_ok() {
        Ok(response)
    } else {
        Err(CliError::App(escape_terminal_text(&response.error_text())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_text_escapes_terminal_controls_and_bidi() {
        let alias = "Estação\x1b[2J\t\n\u{009b}\u{202e}model\u{2069}";
        assert_eq!(
            escape_terminal_text(alias),
            r"Estação\u001b[2J\u0009\u000a\u009b\u202emodel\u2069"
        );
        let peer = json!({"info": {"alias": alias}});
        let serialized = terminal_json(format!("{peer:#}"));
        assert!(serialized.contains("\n  "));
        assert!(!serialized.contains(['\u{009b}', '\u{202e}', '\u{2069}']));
        assert_eq!(serde_json::from_str::<Value>(&serialized).unwrap(), peer);
    }
}
