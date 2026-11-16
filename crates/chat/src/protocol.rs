//! Protocolo puro `stream-json` do Claude Code (`claude_stream_engine.dart` do Flutter).
//!
//! Funções puras: argumentos do processo, linhas de `stdin` (mensagem do usuário,
//! `control_request`, resposta de permissão) e a tradução das linhas de `stdout` em
//! [`ChatEvent`]. Sem I/O.

use serde_json::{json, Map, Value};

use crate::account::{parse_message_context, parse_result_context};
use crate::error::ChatError;
use crate::event::{ChatEvent, SessionState};
use crate::json::{arr_of, obj_of, str_of, string_of};

/// Nome padrão do binário do Claude Code resolvido via `PATH`.
pub const DEFAULT_CLAUDE_BINARY: &str = "claude";

/// Mensagem que acompanha um `deny` do usuário (`answerPermission` do Flutter).
pub const DEFAULT_DENY_MESSAGE: &str = "Negado pelo usuário no GSH";

/// `request_id` do `initialize` que o motor manda ao subir.
pub const INIT_REQUEST_ID: &str = "gsh-init";
/// Prefixo dos `request_id` de `set_model`.
pub const MODEL_REQUEST_PREFIX: &str = "gsh-model-";
/// Prefixo dos `request_id` de `get_usage`.
pub const USAGE_REQUEST_PREFIX: &str = "gsh-usage-";

/// Chaves prioritárias para extrair o resumo de uma chamada de ferramenta (`toolSummary`).
const TOOL_SUMMARY_KEYS: &[&str] = &[
    "command",
    "file_path",
    "notebook_path",
    "url",
    "pattern",
    "query",
    "description",
];

/// O que muda entre uma conversa e outra na linha de comando do `claude`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaunchOptions {
    pub resume_session_id: Option<String>,
    pub show_thinking: bool,
    pub model: Option<String>,
    pub effort: Option<String>,
    /// `crossSessionInbound` ausente no `~/.claude/settings.json`: o chat entrega por padrão.
    pub accept_peers: bool,
}

/// Argumentos do `claude` (`ClaudeStreamEngineFactory.start`): `stream-json` nos dois
/// sentidos e os pedidos de permissão respondidos por nós (`--permission-prompts host`).
pub fn launch_args(opts: &LaunchOptions) -> Vec<String> {
    let mut a: Vec<String> = [
        "-p",
        "--input-format",
        "stream-json",
        "--output-format",
        "stream-json",
        "--verbose",
        "--permission-prompts",
        "host",
        "--permission-prompt-tool",
        "stdio",
        "--include-partial-messages",
        // Ecoa os turnos de usuário: é o único jeito de ver as mensagens que outras sessões
        // injetam (SendMessage entre sessões).
        "--replay-user-messages",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    if opts.accept_peers {
        a.push("--settings".to_string());
        a.push("{\"crossSessionInbound\":\"accept\"}".to_string());
    }
    // Só deixa "Ignorar permissões" disponível no seletor de modo; a sessão continua
    // começando em `default` (pedindo permissão).
    a.push("--allow-dangerously-skip-permissions".to_string());
    // Resumos de raciocínio (`thinking` com texto). Sem a flag o CLI manda o bloco vazio.
    if opts.show_thinking {
        a.push("--thinking-display".to_string());
        a.push("summarized".to_string());
    }
    for (flag, value) in [
        ("--resume", &opts.resume_session_id),
        ("--model", &opts.model),
        ("--effort", &opts.effort),
    ] {
        if let Some(v) = value {
            a.push(flag.to_string());
            a.push(v.clone());
        }
    }
    a
}

/// Linha de `stdin` com a mensagem do usuário; `content` é string ou lista de blocos.
pub fn user_message_line(content: &Value) -> String {
    json!({
        "type": "user",
        "message": {"role": "user", "content": content},
    })
    .to_string()
}

/// Linha de `stdin` com um texto simples (`content` string).
pub fn format_user_message_line(text: &str) -> String {
    user_message_line(&Value::String(text.to_string()))
}

/// Linha `control_request`.
pub fn control_request_line(request_id: &str, request: Value) -> String {
    json!({
        "type": "control_request",
        "request_id": request_id,
        "request": request,
    })
    .to_string()
}

/// `initialize`: lista de modelos e comandos da conta.
pub fn initialize_line() -> String {
    control_request_line(INIT_REQUEST_ID, json!({"subtype": "initialize"}))
}

pub fn set_model_line(seq: u64, model: &str) -> String {
    control_request_line(
        &format!("{MODEL_REQUEST_PREFIX}{seq}"),
        json!({"subtype": "set_model", "model": model}),
    )
}

pub fn set_effort_line(seq: u64, level: &str) -> String {
    control_request_line(
        &format!("gsh-effort-{seq}"),
        json!({"subtype": "apply_flag_settings", "settings": {"effortLevel": level}}),
    )
}

pub fn get_usage_line(seq: u64) -> String {
    control_request_line(
        &format!("{USAGE_REQUEST_PREFIX}{seq}"),
        json!({"subtype": "get_usage"}),
    )
}

pub fn set_permission_mode_line(seq: u64, mode: &str) -> String {
    control_request_line(
        &format!("gsh-{seq}"),
        json!({"subtype": "set_permission_mode", "mode": mode}),
    )
}

pub fn interrupt_line(seq: u64) -> String {
    control_request_line(&format!("gsh-{seq}"), json!({"subtype": "interrupt"}))
}

/// Resposta a um `can_use_tool`. `allow` devolve o input (o da pergunta, ou o trocado pelo
/// chamador) e, com `always`, a regra sugerida em `updatedPermissions`; `deny` leva a mensagem.
pub fn permission_response_line(
    request_id: &str,
    allow: bool,
    always_rule: Option<&Value>,
    input: Value,
    message: Option<&str>,
) -> String {
    let body = if allow {
        let mut b = json!({"behavior": "allow", "updatedInput": input});
        if let (Some(rule), Some(map)) = (always_rule, b.as_object_mut()) {
            map.insert("updatedPermissions".to_string(), json!([rule]));
        }
        b
    } else {
        json!({
            "behavior": "deny",
            "message": message.unwrap_or(DEFAULT_DENY_MESSAGE),
        })
    };
    json!({
        "type": "control_response",
        "response": {
            "subtype": "success",
            "request_id": request_id,
            "response": body,
        },
    })
    .to_string()
}

/// Uma linha que diz o que a ferramenta vai fazer: o comando do Bash, o arquivo do
/// Edit/Read/Write, a URL do WebFetch; senão o input compacto (`toolSummary`).
pub fn tool_summary(input: &Map<String, Value>) -> String {
    for key in TOOL_SUMMARY_KEYS {
        if let Some(v) = str_of(input, key) {
            if !v.trim().is_empty() {
                return v.trim().to_string();
            }
        }
    }
    let raw = Value::Object(input.clone()).to_string();
    if raw.chars().count() <= 160 {
        raw
    } else {
        let head: String = raw.chars().take(159).collect();
        format!("{head}…")
    }
}

/// Decodifica uma linha do `stdout`: `Ok(None)` para linha em branco ou que não é objeto
/// (o Dart ignora), `Err` para JSON quebrado.
pub fn parse_json_line(line: &str) -> Result<Option<Map<String, Value>>, ChatError> {
    let t = line.trim();
    if t.is_empty() {
        return Ok(None);
    }
    match serde_json::from_str::<Value>(t) {
        Ok(Value::Object(o)) => Ok(Some(o)),
        Ok(_) => Ok(None),
        Err(e) => Err(ChatError::InvalidJsonLine {
            line: t.to_string(),
            cause: e.to_string(),
        }),
    }
}

/// A primeira sugestão `addRules` de um `can_use_tool` (o que o Claude Code gravaria com
/// "sempre permitir").
pub fn add_rules(req: &Value) -> Option<Value> {
    req.get("permission_suggestions")?
        .as_array()?
        .iter()
        .find(|s| {
            s.get("type").and_then(Value::as_str) == Some("addRules")
                && s.get("behavior").and_then(Value::as_str) == Some("allow")
        })
        .cloned()
}

/// `Bash(npm test *)`: a regra como o settings.json escreve.
pub fn rule_label(sugg: &Value) -> Option<String> {
    let rules = sugg.get("rules")?.as_array()?;
    let r = rules.first()?.as_object()?;
    let tool = string_of(r, "toolName");
    match str_of(r, "ruleContent") {
        Some(c) if !c.is_empty() => Some(format!("{tool}({c})")),
        _ => Some(tool),
    }
}

/// O texto que a outra sessão escreveu, sem o envelope `<cross-session-message ...>...
/// </cross-session-message>` nem o aviso que o Claude Code acrescenta para o modelo.
pub fn peer_message_text(content: &str) -> String {
    const OPEN: &str = "<cross-session-message";
    const CLOSE: &str = "</cross-session-message>";
    let inner = content.find(OPEN).and_then(|at| {
        let after = content.get(at + OPEN.len()..)?;
        let body = after.get(after.find('>')? + 1..)?;
        body.get(..body.find(CLOSE)?)
    });
    inner.unwrap_or(content).trim().to_string()
}

fn result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_object)
            .filter(|b| str_of(b, "type") == Some("text"))
            .filter_map(|b| str_of(b, "text"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

/// Traduz uma linha do `stream-json` do Claude Code em eventos. Tipos que a conversa não
/// mostra (hooks, rate limit, resumos) viram lista vazia. `control_response` não passa por
/// aqui: é resposta a um pedido nosso e quem a trata é o motor.
pub fn parse_stream_object(o: &Map<String, Value>) -> Vec<ChatEvent> {
    match str_of(o, "type") {
        Some("system") => parse_system(o),
        Some("assistant") => parse_assistant(o),
        Some("user") => parse_user(o),
        Some("stream_event") => parse_delta(o),
        Some("control_request") => parse_ask(o),
        Some("result") => {
            let mut out: Vec<ChatEvent> = parse_result_context(o).into_iter().collect();
            out.push(ChatEvent::TurnEnd {
                is_error: o.get("is_error").and_then(Value::as_bool) == Some(true),
                text: string_of(o, "result"),
                session_id: string_of(o, "session_id"),
                cost_usd: o.get("total_cost_usd").and_then(Value::as_f64),
            });
            out
        }
        _ => Vec::new(),
    }
}

fn parse_system(o: &Map<String, Value>) -> Vec<ChatEvent> {
    match str_of(o, "subtype") {
        Some("init") => vec![ChatEvent::Ready {
            session_id: string_of(o, "session_id"),
            model: string_of(o, "model"),
            permission_mode: str_of(o, "permissionMode").map(ToOwned::to_owned),
        }],
        Some("post_turn_summary") => {
            let state = match str_of(o, "status_category") {
                Some("completed") => SessionState::Completed,
                Some("review_ready") => SessionState::ReviewReady,
                // `need_input` é como o remoto renomeia o `blocked`.
                Some("blocked" | "need_input") => SessionState::Blocked,
                Some("failed") => SessionState::Failed,
                _ => return Vec::new(),
            };
            vec![ChatEvent::SessionClassified {
                state,
                detail: string_of(o, "status_detail").trim().to_string(),
            }]
        }
        Some("informational") => {
            let text = string_of(o, "content");
            if text.is_empty() {
                Vec::new()
            } else {
                vec![ChatEvent::Info {
                    text,
                    warning: str_of(o, "level") == Some("warning"),
                }]
            }
        }
        Some("status") => match str_of(o, "permissionMode") {
            Some(mode) => vec![ChatEvent::ModeChanged(mode.to_string())],
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn parse_assistant(o: &Map<String, Value>) -> Vec<ChatEvent> {
    let Some(content) = obj_of(o, "message").and_then(|m| arr_of(m, "content")) else {
        return Vec::new();
    };
    let flagged = |k: &str| o.get(k).and_then(Value::as_bool) == Some(true);
    if flagged("is_api_error_message") || flagged("isApiErrorMessage") {
        let text = result_text(&Value::Array(content.clone()))
            .trim()
            .to_string();
        if text.is_empty() {
            return Vec::new();
        }
        let session_id = str_of(o, "session_id")
            .or_else(|| str_of(o, "sessionId"))
            .unwrap_or("")
            .to_string();
        return vec![ChatEvent::TurnEnd {
            is_error: true,
            text,
            session_id,
            cost_usd: None,
        }];
    }
    let mut out: Vec<ChatEvent> = parse_message_context(o).into_iter().collect();
    for c in content.iter().filter_map(Value::as_object) {
        match str_of(c, "type") {
            Some("text") => {
                let text = string_of(c, "text");
                if !text.is_empty() {
                    out.push(ChatEvent::Text { text, at_ms: None });
                }
            }
            Some("thinking") => {
                let t = string_of(c, "thinking");
                if !t.trim().is_empty() {
                    out.push(ChatEvent::Thinking(t));
                }
            }
            Some("tool_use") => out.push(ChatEvent::ToolCall {
                id: string_of(c, "id"),
                name: string_of(c, "name"),
                input: obj_of(c, "input").cloned().unwrap_or_default(),
            }),
            _ => {}
        }
    }
    out
}

fn parse_user(o: &Map<String, Value>) -> Vec<ChatEvent> {
    let content = obj_of(o, "message").and_then(|m| m.get("content"));
    if o.get("isReplay").and_then(Value::as_bool) == Some(true) {
        // Eco do que nós mesmos mandamos: ignora. Só interessa o turno que outra sessão
        // injetou (origin.kind == peer).
        let origin = obj_of(o, "origin").filter(|g| str_of(g, "kind") == Some("peer"));
        return match (origin, content.and_then(Value::as_str)) {
            (Some(origin), Some(text)) => vec![ChatEvent::PeerMessage {
                from_name: string_of(origin, "name"),
                from: string_of(origin, "from"),
                text: peer_message_text(text),
            }],
            _ => Vec::new(),
        };
    }
    let Some(list) = content.and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(Value::as_object)
        .filter(|c| str_of(c, "type") == Some("tool_result"))
        .map(|c| ChatEvent::ToolResult {
            tool_use_id: string_of(c, "tool_use_id"),
            is_error: c.get("is_error").and_then(Value::as_bool) == Some(true),
            text: c.get("content").map(result_text).unwrap_or_default(),
        })
        .collect()
}

fn parse_delta(o: &Map<String, Value>) -> Vec<ChatEvent> {
    let Some(ev) = obj_of(o, "event").filter(|e| str_of(e, "type") == Some("content_block_delta"))
    else {
        return Vec::new();
    };
    let Some(delta) = obj_of(ev, "delta") else {
        return Vec::new();
    };
    match str_of(delta, "type") {
        Some("thinking_delta") => {
            let t = string_of(delta, "thinking");
            if t.is_empty() {
                Vec::new()
            } else {
                vec![ChatEvent::ThinkingDelta(t)]
            }
        }
        Some("text_delta") => {
            let t = string_of(delta, "text");
            if t.is_empty() {
                Vec::new()
            } else {
                vec![ChatEvent::TextDelta(t)]
            }
        }
        _ => Vec::new(),
    }
}

fn parse_ask(o: &Map<String, Value>) -> Vec<ChatEvent> {
    let Some(req) = obj_of(o, "request").filter(|r| str_of(r, "subtype") == Some("can_use_tool"))
    else {
        return Vec::new();
    };
    let sugg = o.get("request").and_then(add_rules);
    vec![ChatEvent::PermissionAsk {
        request_id: string_of(o, "request_id"),
        tool_name: string_of(req, "tool_name"),
        input: obj_of(req, "input").cloned().unwrap_or_default(),
        description: str_of(req, "description").map(ToOwned::to_owned),
        always_rule: sugg.as_ref().and_then(rule_label),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn events(v: Value) -> Vec<ChatEvent> {
        parse_stream_object(v.as_object().expect("objeto"))
    }

    // Formatos reais do `claude -p --output-format stream-json` (2.1.309), enxugados — os
    // mesmos de `test/data/claude_stream_engine_test.dart`.
    fn init() -> Value {
        json!({"type": "system", "subtype": "init", "session_id": "s-1", "model": "claude-opus-5-5"})
    }
    fn tool_use() -> Value {
        json!({"type": "assistant", "message": {"content": [
            {"type": "thinking", "thinking": ""},
            {"type": "tool_use", "id": "toolu_1", "name": "Bash",
             "input": {"command": "rm teste.txt", "description": "Delete"}},
        ]}})
    }
    fn ask() -> Value {
        json!({"type": "control_request", "request_id": "req-1", "request": {
            "subtype": "can_use_tool", "tool_name": "Bash",
            "input": {"command": "rm teste.txt"}, "description": "Delete teste.txt file"}})
    }
    fn denied() -> Value {
        json!({"type": "user", "message": {"role": "user", "content": [
            {"type": "tool_result", "content": "Negado pelo usuário no GSH",
             "is_error": true, "tool_use_id": "toolu_1"}]}})
    }
    fn result() -> Value {
        json!({"type": "result", "subtype": "success", "is_error": false,
               "result": "negado", "session_id": "s-1", "total_cost_usd": 0.26})
    }

    #[test]
    fn parser_traduz_as_linhas_que_a_conversa_usa_e_ignora_o_resto() {
        assert_eq!(
            events(init()),
            vec![ChatEvent::Ready {
                session_id: "s-1".into(),
                model: "claude-opus-5-5".into(),
                permission_mode: None
            }]
        );
        let calls = events(tool_use());
        assert_eq!(calls.len(), 1, "o thinking vazio não vira evento");
        match &calls[0] {
            ChatEvent::ToolCall { id, name, input } => {
                assert_eq!((id.as_str(), name.as_str()), ("toolu_1", "Bash"));
                assert_eq!(str_of(input, "command"), Some("rm teste.txt"));
            }
            other => panic!("esperava ToolCall, veio {other:?}"),
        }
        match events(ask()).as_slice() {
            [ChatEvent::PermissionAsk {
                request_id,
                tool_name,
                always_rule: None,
                ..
            }] => assert_eq!((request_id.as_str(), tool_name.as_str()), ("req-1", "Bash")),
            other => panic!("pedido inesperado: {other:?}"),
        }
        assert_eq!(
            events(denied()),
            vec![ChatEvent::ToolResult {
                tool_use_id: "toolu_1".into(),
                is_error: true,
                text: "Negado pelo usuário no GSH".into()
            }]
        );
        assert_eq!(
            events(result()),
            vec![ChatEvent::TurnEnd {
                is_error: false,
                text: "negado".into(),
                session_id: "s-1".into(),
                cost_usd: Some(0.26)
            }]
        );
        assert!(events(json!({"type": "rate_limit_event"})).is_empty());
        assert!(events(json!({"type": "system", "subtype": "hook_started"})).is_empty());
    }

    #[test]
    fn thinking_thinking_delta_e_post_turn_summary() {
        let delta = |t: &str| {
            json!({"type": "stream_event", "event": {"type": "content_block_delta",
                   "delta": {"type": "thinking_delta", "thinking": t}}})
        };
        assert_eq!(
            events(delta("Che")),
            vec![ChatEvent::ThinkingDelta("Che".into())]
        );
        let block = json!({"type": "assistant", "message": {"content": [
            {"type": "thinking", "thinking": "Checando 1009.", "signature": "x"}]}});
        assert_eq!(
            events(block),
            vec![ChatEvent::Thinking("Checando 1009.".into())]
        );
        for (raw, state) in [
            ("completed", SessionState::Completed),
            ("review_ready", SessionState::ReviewReady),
            ("blocked", SessionState::Blocked),
            ("need_input", SessionState::Blocked),
            ("failed", SessionState::Failed),
        ] {
            let line = json!({"type": "system", "subtype": "post_turn_summary",
                "summarizes_uuid": "u-1", "status_category": raw,
                "status_detail": "1009 é primo ", "needs_action": "", "session_id": "s-1"});
            assert_eq!(
                events(line),
                vec![ChatEvent::SessionClassified {
                    state,
                    detail: "1009 é primo".into()
                }]
            );
        }
        let other = json!({"type": "system", "subtype": "post_turn_summary",
                           "status_category": "outra_coisa"});
        assert!(events(other).is_empty());
    }

    #[test]
    fn texto_ao_vivo_e_bloco_completo() {
        let d = json!({"type": "stream_event", "event": {"type": "content_block_delta",
                       "delta": {"type": "text_delta", "text": "Ol"}}});
        assert_eq!(events(d), vec![ChatEvent::TextDelta("Ol".into())]);
        let t = json!({"type": "assistant", "message": {"content": [{"type": "text", "text": "Olá!"}]}});
        assert_eq!(
            events(t),
            vec![ChatEvent::Text {
                text: "Olá!".into(),
                at_ms: None
            }]
        );
    }

    #[test]
    fn sempre_permitir_regra_sugerida_chega_no_pedido_e_na_resposta() {
        let mut line = ask();
        line["request"]["permission_suggestions"] = json!([{
            "type": "addRules", "behavior": "allow", "destination": "localSettings",
            "rules": [{"toolName": "Bash", "ruleContent": "rm *"}]}]);
        match events(line.clone()).as_slice() {
            [ChatEvent::PermissionAsk { always_rule, .. }] => {
                assert_eq!(always_rule.as_deref(), Some("Bash(rm *)"));
            }
            other => panic!("{other:?}"),
        }
        let rule = add_rules(&line["request"]).expect("regra");
        let out: Value = serde_json::from_str(&permission_response_line(
            "req-1",
            true,
            Some(&rule),
            json!({"command": "rm teste.txt"}),
            None,
        ))
        .expect("json");
        assert_eq!(out["response"]["response"]["behavior"], "allow");
        assert_eq!(out["response"]["response"]["updatedPermissions"][0], rule);
        assert_eq!(out["response"]["request_id"], "req-1");
    }

    #[test]
    fn negar_leva_a_mensagem() {
        let out: Value = serde_json::from_str(&permission_response_line(
            "p1",
            false,
            None,
            json!({}),
            Some("use final.txt"),
        ))
        .expect("json");
        assert_eq!(
            out["response"]["response"],
            json!({"behavior": "deny", "message": "use final.txt"})
        );
        let dflt: Value = serde_json::from_str(&permission_response_line(
            "p2",
            false,
            None,
            json!({}),
            None,
        ))
        .expect("json");
        assert_eq!(
            dflt["response"]["response"]["message"],
            DEFAULT_DENY_MESSAGE
        );
    }

    #[test]
    fn mensagens_entre_sessoes() {
        let peer = json!({"type": "user", "isReplay": true, "isSynthetic": true,
            "origin": {"kind": "peer", "from": "uds:/run/user/1000/cc-socks/1.sock", "name": "Fazenda"},
            "message": {"role": "user", "content":
                "Another Claude session sent a message:\n<cross-session-message from=\"uds:/x\" from-name=\"Fazenda\">\nTerminei o build.\n</cross-session-message>\n\nThis came from another Claude session…"}});
        assert_eq!(
            events(peer),
            vec![ChatEvent::PeerMessage {
                from_name: "Fazenda".into(),
                from: "uds:/run/user/1000/cc-socks/1.sock".into(),
                text: "Terminei o build.".into()
            }]
        );
        // Eco do nosso próprio envio é ignorado.
        let echo = json!({"type": "user", "isReplay": true,
                          "message": {"role": "user", "content": "oi"}});
        assert!(events(echo).is_empty());
        let info = json!({"type": "system", "subtype": "informational", "level": "warning",
            "content": "Cross-session message held by the receiving session."});
        assert_eq!(
            events(info),
            vec![ChatEvent::Info {
                text: "Cross-session message held by the receiving session.".into(),
                warning: true
            }]
        );
    }

    #[test]
    fn rate_limit_vira_um_fim_de_turno_com_erro() {
        let line = json!({"type": "assistant", "is_api_error_message": true,
            "error": "rate_limit", "session_id": "s-1",
            "message": {"model": "<synthetic>", "stop_reason": "stop_sequence", "content": [
                {"type": "text", "text": "You've hit your limit · resets 5pm (America/Sao_Paulo)"}]}});
        assert_eq!(
            events(line),
            vec![ChatEvent::TurnEnd {
                is_error: true,
                text: "You've hit your limit · resets 5pm (America/Sao_Paulo)".into(),
                session_id: "s-1".into(),
                cost_usd: None
            }]
        );
    }

    #[test]
    fn status_troca_o_modo_e_pergunta_do_agente_chega_com_opcoes() {
        let s = json!({"type": "system", "subtype": "status", "permissionMode": "default"});
        assert_eq!(events(s), vec![ChatEvent::ModeChanged("default".into())]);
        let q = json!({"type": "control_request", "request_id": "q1", "request": {
            "subtype": "can_use_tool", "tool_name": "AskUserQuestion",
            "input": {"questions": [{"question": "Qual sua cor favorita?", "header": "Cor",
                "multiSelect": false, "options": [{"label": "Azul", "description": "Cor azul"}]}]}}});
        match events(q).as_slice() {
            [ChatEvent::PermissionAsk {
                tool_name, input, ..
            }] => {
                assert_eq!(tool_name, "AskUserQuestion");
                assert!(input.contains_key("questions"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn resultado_traz_a_janela_do_modelo_antes_do_fim_do_turno() {
        let mut r = result();
        r["modelUsage"] = json!({"claude-haiku-5-5": {"contextWindow": 1_000_000}});
        let evs = events(r);
        assert_eq!(evs.len(), 2);
        assert_eq!(
            evs[0],
            ChatEvent::Context {
                tokens: 0,
                window: 1_000_000
            }
        );
    }

    #[test]
    fn argumentos_de_lancamento_seguem_a_fabrica_do_flutter() {
        let a = launch_args(&LaunchOptions {
            resume_session_id: Some("s-1".into()),
            show_thinking: true,
            model: Some("opus".into()),
            effort: Some("high".into()),
            accept_peers: true,
        });
        let joined = a.join(" ");
        assert_eq!(
            joined,
            "-p --input-format stream-json --output-format stream-json --verbose \
             --permission-prompts host --permission-prompt-tool stdio \
             --include-partial-messages --replay-user-messages \
             --settings {\"crossSessionInbound\":\"accept\"} \
             --allow-dangerously-skip-permissions --thinking-display summarized \
             --resume s-1 --model opus --effort high"
        );
        let plain = launch_args(&LaunchOptions::default()).join(" ");
        assert!(!plain.contains("--settings"));
        assert!(plain.ends_with("--allow-dangerously-skip-permissions"));
    }

    #[test]
    fn linhas_de_controle() {
        let m: Value = serde_json::from_str(&set_model_line(3, "opus")).expect("json");
        assert_eq!(m["request_id"], "gsh-model-3");
        assert_eq!(
            m["request"],
            json!({"subtype": "set_model", "model": "opus"})
        );
        let e: Value = serde_json::from_str(&set_effort_line(4, "max")).expect("json");
        assert_eq!(
            e["request"],
            json!({"subtype": "apply_flag_settings", "settings": {"effortLevel": "max"}})
        );
        let i: Value = serde_json::from_str(&initialize_line()).expect("json");
        assert_eq!(i["request_id"], INIT_REQUEST_ID);
        let u: Value = serde_json::from_str(&user_message_line(&json!("oi"))).expect("json");
        assert_eq!(
            u,
            json!({"type": "user", "message": {"role": "user", "content": "oi"}})
        );
    }

    #[test]
    fn resumo_da_ferramenta() {
        let o = |v: Value| v.as_object().cloned().expect("objeto");
        assert_eq!(tool_summary(&o(json!({"command": " ls "}))), "ls");
        assert_eq!(tool_summary(&o(json!({"x": 1}))), "{\"x\":1}");
        let long = "a".repeat(200);
        let s = tool_summary(&o(json!({"x": long})));
        assert_eq!(s.chars().count(), 160);
        assert!(s.ends_with('…'));
    }

    #[test]
    fn json_quebrado_e_erro_tipado_e_linha_vazia_nao() {
        assert_eq!(parse_json_line("  "), Ok(None));
        assert_eq!(parse_json_line("[1]"), Ok(None));
        assert!(matches!(
            parse_json_line("{nao"),
            Err(ChatError::InvalidJsonLine { .. })
        ));
    }
}
