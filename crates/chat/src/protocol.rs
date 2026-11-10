//! Protocolo puro `stream-json` do Claude Code (`claude_stream_engine.dart` do Flutter).
//!
//! Funções puras para formatar mensagens de entrada (`stdin`) e converter linhas
//! JSON de saída (`stdout`) em [`ChatEvent`] tipados, sem I/O.

use serde_json::{json, Map, Value};

use crate::error::ChatError;

/// Nome padrão do binário do Claude Code resolvido via `PATH`.
pub const DEFAULT_CLAUDE_BINARY: &str = "claude";

/// Argumentos de linha de comando passados ao `claude` no modo `stream-json`,
/// espelhando `ClaudeStreamEngineFactory.start` em `claude_stream_engine.dart`:
/// `-p --input-format stream-json --output-format stream-json --verbose --include-partial-messages`.
pub const DEFAULT_CLAUDE_ARGS: &[&str] = &[
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--include-partial-messages",
];

/// Chaves prioritárias para extrair o resumo de uma chamada de ferramenta
/// (`toolSummary` em `chat_viewmodel.dart`).
const TOOL_SUMMARY_KEYS: &[&str] = &[
    "command",
    "file_path",
    "notebook_path",
    "url",
    "pattern",
    "query",
    "description",
];

/// Evento tipado emitido pelo motor de chat a partir do `stream-json`.
#[derive(Debug, Clone, PartialEq)]
pub enum ChatEvent {
    /// Sessão inicializada (`system` com `subtype: "init"`).
    Ready {
        session_id: String,
        model: String,
        permission_mode: Option<String>,
    },
    /// Pedaço incremental de texto do assistente (`stream_event` -> `text_delta`).
    TextDelta(String),
    /// Bloco completo de texto do assistente (`assistant` -> `content[type=text]`).
    AssistantText(String),
    /// Pedaço incremental de raciocínio (`stream_event` -> `thinking_delta`).
    ThinkingDelta(String),
    /// Bloco completo de raciocínio (`assistant` -> `content[type=thinking]`).
    Thinking(String),
    /// Chamada de ferramenta iniciada pelo assistente (`assistant` -> `content[type=tool_use]`).
    ToolUse {
        id: String,
        name: String,
        summary: String,
        input: Value,
    },
    /// Resultado de execução de ferramenta (`user` -> `content[type=tool_result]`).
    ToolResult {
        tool_use_id: String,
        is_error: bool,
        content: String,
    },
    /// Resultado final do turno (`result`).
    Result {
        is_error: bool,
        result: String,
        session_id: String,
        cost_usd: Option<f64>,
    },
    /// Erro de protocolo ou de execução do processo filho.
    Error(ChatError),
}

/// Formata uma mensagem do usuário como linha JSON para envio no `stdin` do `claude`
/// (`{"type":"user","message":{"role":"user","content":"..."}}`).
pub fn format_user_message_line(text: &str) -> String {
    json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": text,
        }
    })
    .to_string()
}

/// Gera o resumo curto em uma linha para uma chamada de ferramenta
/// (`toolSummary` em `chat_viewmodel.dart`).
pub fn tool_summary(_name: &str, input: &Value) -> String {
    if let Some(obj) = input.as_object() {
        for key in TOOL_SUMMARY_KEYS {
            if let Some(val) = obj.get(*key).and_then(Value::as_str) {
                let trimmed = val.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }
    let raw = input.to_string();
    if raw.chars().count() <= 160 {
        raw
    } else {
        let prefix: String = raw.chars().take(159).collect();
        format!("{prefix}…")
    }
}

/// Analisa uma linha do `stdout` (`stream-json`) e converte em zero ou mais [`ChatEvent`].
///
/// - Linhas em branco retornam `Ok(vec![])`.
/// - Eventos desconhecidos (ex.: `rate_limit`, `hook_started`, `control_response`) são
///   ignorados sem erro (`Ok(vec![])`).
/// - Linhas que não são um objeto JSON válido retornam `Err(ChatError::InvalidJsonLine)`.
pub fn parse_stream_line(line: &str) -> Result<Vec<ChatEvent>, ChatError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let parsed: Value =
        serde_json::from_str(trimmed).map_err(|err| ChatError::InvalidJsonLine {
            line: trimmed.to_string(),
            cause: err.to_string(),
        })?;

    let Some(obj) = parsed.as_object() else {
        return Err(ChatError::InvalidJsonLine {
            line: trimmed.to_string(),
            cause: "esperado objeto JSON na raiz".to_string(),
        });
    };

    let Some(kind) = obj.get("type").and_then(Value::as_str) else {
        return Ok(Vec::new());
    };

    match kind {
        "system" => Ok(parse_system_event(obj)),
        "assistant" => Ok(parse_assistant_event(obj)),
        "user" => Ok(parse_user_event(obj)),
        "stream_event" => Ok(parse_delta_event(obj)),
        "result" => Ok(parse_result_event(obj)),
        _ => Ok(Vec::new()),
    }
}

fn parse_system_event(obj: &Map<String, Value>) -> Vec<ChatEvent> {
    if obj.get("subtype").and_then(Value::as_str) == Some("init") {
        let session_id = str_field(obj, "session_id");
        let model = str_field(obj, "model");
        let permission_mode = obj
            .get("permissionMode")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        vec![ChatEvent::Ready {
            session_id,
            model,
            permission_mode,
        }]
    } else {
        Vec::new()
    }
}

fn parse_assistant_event(obj: &Map<String, Value>) -> Vec<ChatEvent> {
    let Some(content) = obj
        .get("message")
        .and_then(Value::as_object)
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let is_api_error = obj.get("is_api_error_message").and_then(Value::as_bool) == Some(true)
        || obj.get("isApiErrorMessage").and_then(Value::as_bool) == Some(true);

    if is_api_error {
        let text = extract_result_text(&Value::Array(content.clone()))
            .trim()
            .to_string();
        if text.is_empty() {
            return Vec::new();
        }
        let session_id = obj
            .get("session_id")
            .or_else(|| obj.get("sessionId"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        return vec![ChatEvent::Result {
            is_error: true,
            result: text,
            session_id,
            cost_usd: None,
        }];
    }

    let mut out = Vec::new();
    for item in content {
        let Some(block) = item.as_object() else {
            continue;
        };
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                let text = str_field(block, "text");
                if !text.is_empty() {
                    out.push(ChatEvent::AssistantText(text));
                }
            }
            Some("thinking") => {
                let thinking = str_field(block, "thinking");
                if !thinking.trim().is_empty() {
                    out.push(ChatEvent::Thinking(thinking));
                }
            }
            Some("tool_use") => {
                let id = str_field(block, "id");
                let name = str_field(block, "name");
                let input = block
                    .get("input")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Map::new()));
                let summary = tool_summary(&name, &input);
                out.push(ChatEvent::ToolUse {
                    id,
                    name,
                    summary,
                    input,
                });
            }
            _ => {}
        }
    }
    out
}

fn parse_user_event(obj: &Map<String, Value>) -> Vec<ChatEvent> {
    if obj.get("isReplay").and_then(Value::as_bool) == Some(true) {
        return Vec::new();
    }
    let Some(content) = obj
        .get("message")
        .and_then(Value::as_object)
        .and_then(|m| m.get("content"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for item in content {
        let Some(block) = item.as_object() else {
            continue;
        };
        if block.get("type").and_then(Value::as_str) == Some("tool_result") {
            let tool_use_id = str_field(block, "tool_use_id");
            let is_error = block.get("is_error").and_then(Value::as_bool) == Some(true);
            let content_text = block
                .get("content")
                .map(extract_result_text)
                .unwrap_or_default();
            out.push(ChatEvent::ToolResult {
                tool_use_id,
                is_error,
                content: content_text,
            });
        }
    }
    out
}

fn parse_delta_event(obj: &Map<String, Value>) -> Vec<ChatEvent> {
    let Some(ev) = obj.get("event").and_then(Value::as_object) else {
        return Vec::new();
    };
    if ev.get("type").and_then(Value::as_str) != Some("content_block_delta") {
        return Vec::new();
    }
    let Some(delta) = ev.get("delta").and_then(Value::as_object) else {
        return Vec::new();
    };
    match delta.get("type").and_then(Value::as_str) {
        Some("text_delta") => {
            let text = str_field(delta, "text");
            if text.is_empty() {
                Vec::new()
            } else {
                vec![ChatEvent::TextDelta(text)]
            }
        }
        Some("thinking_delta") => {
            let thinking = str_field(delta, "thinking");
            if thinking.is_empty() {
                Vec::new()
            } else {
                vec![ChatEvent::ThinkingDelta(thinking)]
            }
        }
        _ => Vec::new(),
    }
}

fn parse_result_event(obj: &Map<String, Value>) -> Vec<ChatEvent> {
    let is_error = obj.get("is_error").and_then(Value::as_bool) == Some(true);
    let result = str_field(obj, "result");
    let session_id = str_field(obj, "session_id");
    let cost_usd = obj.get("total_cost_usd").and_then(Value::as_f64);
    vec![ChatEvent::Result {
        is_error,
        result,
        session_id,
        cost_usd,
    }]
}

fn str_field(obj: &Map<String, Value>, key: &str) -> String {
    obj.get(key)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn extract_result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_object)
            .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|b| b.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}
