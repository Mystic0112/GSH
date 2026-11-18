//! Servidor MCP stdio dos apps desktop, usando o mesmo socket de comandos do `gsh`.

use std::io::{BufRead, Write};
use std::time::Duration;

use gsh_core::wire::cmd_request;
use serde_json::{json, Value};

use crate::{error::CliError, transport};

const TIMEOUT: Duration = Duration::from_secs(60);

pub fn run(agent: &str) -> Result<Option<String>, CliError> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut output = stdout.lock();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| CliError::App(format!("MCP stdin: {error}")))?;
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                let response = json!({"jsonrpc":"2.0", "id":null, "error":{"code":-32700,"message":error.to_string()}});
                writeln!(output, "{response}")
                    .map_err(|error| CliError::App(format!("MCP stdout: {error}")))?;
                output
                    .flush()
                    .map_err(|error| CliError::App(format!("MCP stdout: {error}")))?;
                continue;
            }
        };
        if let Some(response) = handle(&request, agent) {
            writeln!(output, "{response}")
                .map_err(|error| CliError::App(format!("MCP stdout: {error}")))?;
            output
                .flush()
                .map_err(|error| CliError::App(format!("MCP stdout: {error}")))?;
        }
    }
    Ok(None)
}

fn handle(request: &Value, agent: &str) -> Option<Value> {
    let id = request.get("id")?.clone();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let result = match method {
        "initialize" => {
            json!({"protocolVersion":"2024-11-05", "capabilities":{"tools":{}}, "serverInfo":{"name":"gsh","version":gsh_core::VERSION}})
        }
        "ping" => json!({}),
        "tools/list" => json!({"tools": tools()}),
        "tools/call" => {
            let name = request
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let args = request
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match call_tool(agent, name, args) {
                Ok(value) => {
                    json!({"content":[{"type":"text","text":value.to_string()}], "isError":false})
                }
                Err(error) => json!({"content":[{"type":"text","text":error}], "isError":true}),
            }
        }
        "notifications/initialized" | "notifications/cancelled" => return None,
        _ => {
            return Some(
                json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32601,"message":format!("method not found: {method}")}}),
            )
        }
    };
    Some(json!({"jsonrpc":"2.0", "id":id, "result":result}))
}

fn tools() -> Vec<Value> {
    vec![
        json!({"name":"send", "description":"Send a message to an agent in this GSH workspace.", "inputSchema":{"type":"object","properties":{"to":{"type":"string"},"text":{"type":"string"}},"required":["to","text"]}}),
        json!({"name":"peers", "description":"List agents connected to this agent in the graph.", "inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"list_agents", "description":"List agents in this GSH workspace.", "inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"wait_messages", "description":"Get messages waiting for this desktop agent.", "inputSchema":{"type":"object","properties":{"timeout":{"type":"integer","minimum":0,"maximum":60}}}}),
    ]
}

fn call_tool(agent: &str, name: &str, args: Value) -> Result<Value, String> {
    let (command, args) = match name {
        "send" => (
            "msg",
            json!({"to":args.get("to").and_then(Value::as_str).unwrap_or_default(), "text":args.get("text").and_then(Value::as_str).unwrap_or_default()}),
        ),
        "peers" => ("peers", Value::Null),
        "list_agents" => ("agents", Value::Null),
        "wait_messages" => (
            "ext-wait",
            json!({"timeout":args.get("timeout").and_then(Value::as_u64).unwrap_or(45).min(55)}),
        ),
        _ => return Err(format!("unknown tool: {name}")),
    };
    let presence = ipc("ext-ping", agent, Value::Null)?;
    let _ = presence;
    ipc(command, agent, args)
}

fn ipc(command: &str, agent: &str, args: Value) -> Result<Value, String> {
    let response = transport::request(
        cmd_request(command, Some(&format!("ext:{agent}")), Some(args)),
        TIMEOUT,
    )
    .map_err(|error| error.to_string())?;
    if response.is_ok() {
        Ok(response.data())
    } else {
        Err(response.error_text())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_and_tool_catalog_are_valid_mcp_responses() {
        let init = handle(
            &json!({"jsonrpc":"2.0","id":1,"method":"initialize"}),
            "cursor",
        )
        .expect("response");
        assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
        let list = handle(
            &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            "cursor",
        )
        .expect("response");
        assert_eq!(list["result"]["tools"].as_array().map(Vec::len), Some(4));
        assert!(handle(
            &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
            "cursor"
        )
        .is_none());
    }
}
