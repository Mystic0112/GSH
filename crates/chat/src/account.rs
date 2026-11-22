//! Modelos, comandos, uso do plano e contexto (`claude_account.dart` do Flutter).
//!
//! Parsers puros sobre o corpo das respostas `control_response` e o probe da conta: um
//! `claude -p` de vida curta que responde `initialize` + `get_usage` sem abrir conversa.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};

use crate::event::{
    AccountProbe, ChatEvent, CommandOption, Limit, LimitKind, ModelOption, PlanUsage,
};
use crate::json::{arr_of, int_of, obj_of, str_of, string_of};
use crate::resolve::env_with_dir_on_path;
use crate::time::parse_rfc3339_ms;

/// Modelos da resposta do `initialize` (`models: [{value, displayName, description,
/// resolvedModel}]`).
pub fn parse_models(init: &Value) -> Vec<ModelOption> {
    let Some(list) = init.get("models").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(Value::as_object)
        .filter_map(|m| {
            let value = str_of(m, "value").filter(|v| !v.is_empty())?;
            let efforts = if m.get("supportsEffort").and_then(Value::as_bool) == Some(true) {
                arr_of(m, "supportedEffortLevels")
                    .map(|l| {
                        l.iter()
                            .filter_map(Value::as_str)
                            .map(ToOwned::to_owned)
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
            Some(ModelOption {
                value: value.to_string(),
                display_name: str_of(m, "displayName").unwrap_or(value).to_string(),
                description: string_of(m, "description"),
                resolved: str_of(m, "resolvedModel").map(ToOwned::to_owned),
                efforts,
                // `initialize` e `get_usage` atuais não expõem esse requisito.
                requires_credits: false,
            })
        })
        .collect()
}

/// Tira o escopo do fim da descrição: `... (user)`, `(project)`, `(local)`, `(plugin ...)`.
fn strip_scope(description: &str) -> String {
    let d = description.trim_end();
    if let Some(body) = d.strip_suffix(')') {
        if let Some(open) = body.rfind('(') {
            let inner = body.get(open + 1..).unwrap_or("");
            let scoped = matches!(inner, "user" | "project" | "local")
                || (inner.starts_with("plugin") && !inner.contains(')'));
            if scoped {
                return body.get(..open).unwrap_or("").trim().to_string();
            }
        }
    }
    description.trim().to_string()
}

/// Comandos da resposta do `initialize` (`commands: [{name, description, argumentHint,
/// aliases}]`). A descrição traz o escopo no fim ("(user)", "(project)"), que fica de fora.
pub fn parse_commands(init: &Value) -> Vec<CommandOption> {
    let Some(list) = init.get("commands").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(Value::as_object)
        .filter_map(|c| {
            let name = str_of(c, "name").filter(|n| !n.is_empty())?;
            Some(CommandOption {
                name: name.to_string(),
                description: strip_scope(str_of(c, "description").unwrap_or("")),
                argument_hint: string_of(c, "argumentHint"),
                aliases: arr_of(c, "aliases")
                    .map(|l| {
                        l.iter()
                            .filter_map(Value::as_str)
                            .map(ToOwned::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
            })
        })
        .collect()
}

fn clamp_percent(n: f64) -> u8 {
    let r = n.round();
    if r <= 0.0 {
        0
    } else if r >= 100.0 {
        100
    } else {
        r as u8
    }
}

/// Limites da resposta do `get_usage` (`limits: [{kind, percent, severity, resets_at,
/// scope}]`). Sem `limits`, cai nos `rate_limits` antigos.
pub fn parse_plan_usage(usage: &Value) -> Option<PlanUsage> {
    let u = usage.as_object()?;
    let plan = str_of(u, "subscription_type").map(ToOwned::to_owned);
    let mut limits = Vec::new();
    if let Some(list) = u.get("limits").and_then(Value::as_array) {
        for l in list.iter().filter_map(Value::as_object) {
            let Some(pct) = l.get("percent").and_then(Value::as_f64) else {
                continue;
            };
            let model = obj_of(l, "scope")
                .and_then(|s| obj_of(s, "model"))
                .and_then(|m| str_of(m, "display_name"))
                .map(ToOwned::to_owned);
            let kind = match str_of(l, "kind") {
                Some("session") => LimitKind::Session,
                Some("weekly_all") => LimitKind::Weekly,
                Some("weekly_scoped") if model.is_some() => LimitKind::WeeklyModel,
                _ => LimitKind::Other,
            };
            limits.push(Limit {
                kind,
                percent: clamp_percent(pct),
                model,
                resets_at_ms: str_of(l, "resets_at").and_then(parse_rfc3339_ms),
                warning: l
                    .get("severity")
                    .is_some_and(|s| !s.is_null() && s != "normal"),
            });
        }
    } else if let Some(rl) = obj_of(u, "rate_limits") {
        for (key, kind) in [
            ("five_hour", LimitKind::Session),
            ("seven_day", LimitKind::Weekly),
        ] {
            let Some(w) = obj_of(rl, key) else { continue };
            let Some(util) = w.get("utilization").and_then(Value::as_f64) else {
                continue;
            };
            limits.push(Limit {
                kind,
                percent: clamp_percent(util),
                model: None,
                resets_at_ms: str_of(w, "resets_at").and_then(parse_rfc3339_ms),
                warning: false,
            });
        }
    }
    if limits.is_empty() {
        return None;
    }
    Some(PlanUsage {
        limits: limits
            .into_iter()
            .filter(|l| l.kind != LimitKind::Other)
            .collect(),
        plan,
    })
}

/// Tamanho do contexto numa mensagem `assistant`: o `usage` dela é de UMA chamada à API
/// (input + cache = tudo que o modelo leu). O `usage` do `result` não serve: soma as
/// chamadas do turno inteiro.
pub fn parse_message_context(assistant: &Map<String, Value>) -> Option<ChatEvent> {
    // Subagente tem o próprio contexto.
    if assistant
        .get("parent_tool_use_id")
        .is_some_and(|v| !v.is_null())
    {
        return None;
    }
    let u = obj_of(obj_of(assistant, "message")?, "usage")?;
    let tokens = int_of(u, "input_tokens")
        + int_of(u, "cache_creation_input_tokens")
        + int_of(u, "cache_read_input_tokens");
    (tokens > 0).then_some(ChatEvent::Context { tokens, window: 0 })
}

/// Janela do modelo ao fim do turno (`result.modelUsage[*].contextWindow`). Os tokens vêm das
/// mensagens ([`parse_message_context`]).
pub fn parse_result_context(result: &Map<String, Value>) -> Option<ChatEvent> {
    let window = obj_of(result, "modelUsage")
        .map(|mu| {
            mu.values()
                .filter_map(Value::as_object)
                .map(|m| int_of(m, "contextWindow"))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    (window > 0).then_some(ChatEvent::Context { tokens: 0, window })
}

const PROBE_WAIT: Duration = Duration::from_secs(15);

/// Pergunta à conta, num `claude -p` de vida curta, os modelos e o uso (`initialize` +
/// `get_usage`). Não abre conversa nem gasta tokens. Bloqueia até 30 s: chame de uma thread.
pub fn probe_account(exe: &Path, extra_env: &[(String, String)]) -> Option<AccountProbe> {
    let mut cmd = Command::new(exe);
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
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::null());
    for (k, v) in env_with_dir_on_path(exe, extra_env) {
        cmd.env(k, v);
    }
    crate::settings::apply_profile_env(&mut cmd, extra_env);
    let mut child = cmd.spawn().ok()?;
    let result = run_probe(&mut child);
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn run_probe(child: &mut std::process::Child) -> Option<AccountProbe> {
    let mut stdin = child.stdin.take()?;
    let stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel::<(String, Value)>();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let Ok(Value::Object(o)) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if str_of(&o, "type") != Some("control_response") {
                continue;
            }
            let Some(r) = obj_of(&o, "response") else {
                continue;
            };
            let id = string_of(r, "request_id");
            let body = r.get("response").cloned().unwrap_or(Value::Null);
            if tx.send((id, body)).is_err() {
                return;
            }
        }
    });
    for (id, subtype) in [("probe-init", "initialize"), ("probe-usage", "get_usage")] {
        let line = json!({
            "type": "control_request",
            "request_id": id,
            "request": {"subtype": subtype},
        });
        writeln!(stdin, "{line}").ok()?;
    }
    stdin.flush().ok()?;
    let mut got: HashMap<String, Value> = HashMap::new();
    let init = wait_for(&rx, &mut got, "probe-init", Instant::now() + PROBE_WAIT)?;
    let usage = wait_for(&rx, &mut got, "probe-usage", Instant::now() + PROBE_WAIT)
        .and_then(|u| parse_plan_usage(&u));
    Some(AccountProbe {
        models: parse_models(&init),
        usage,
        commands: parse_commands(&init),
    })
}

fn wait_for(
    rx: &mpsc::Receiver<(String, Value)>,
    got: &mut HashMap<String, Value>,
    id: &str,
    deadline: Instant,
) -> Option<Value> {
    loop {
        if let Some(v) = got.remove(id) {
            return Some(v);
        }
        let left = deadline.checked_duration_since(Instant::now())?;
        let (got_id, body) = rx.recv_timeout(left).ok()?;
        got.insert(got_id, body);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Vetores de `test/data/claude_account_test.dart`.
    #[test]
    fn modelos_do_initialize() {
        let m = parse_models(&json!({
            "models": [
                {
                    "value": "default",
                    "resolvedModel": "claude-opus-5-5",
                    "displayName": "Default (recommended)",
                    "description": "Opus 5.5 · Best for everyday, complex tasks",
                    "supportsEffort": true,
                    "supportedEffortLevels": ["low", "high", "max"],
                },
                {"value": "haiku", "displayName": "Haiku 5.5"},
                {"displayName": "sem value"},
            ]
        }));
        let values: Vec<_> = m.iter().map(|e| e.value.as_str()).collect();
        assert_eq!(values, ["default", "haiku"]);
        assert_eq!(m[0].resolved.as_deref(), Some("claude-opus-5-5"));
        assert_eq!(m[0].efforts, ["low", "high", "max"]);
        assert!(m[1].efforts.is_empty());
        assert!(!m[0].requires_credits);
        assert!(parse_models(&Value::Null).is_empty());
    }

    #[test]
    fn limites_do_get_usage_formato_novo_e_antigo() {
        let u = parse_plan_usage(&json!({
            "subscription_type": "team",
            "limits": [
                {"kind": "session", "percent": 3, "severity": "normal",
                 "resets_at": "2026-10-25T07:30:00.337736+00:00"},
                {"kind": "weekly_all", "percent": 86, "severity": "warning",
                 "resets_at": "2026-10-25T20:00:00+00:00"},
                {"kind": "weekly_scoped", "percent": 59, "severity": "normal",
                 "scope": {"model": {"display_name": "Fable"}}},
                {"kind": "mystery", "percent": 10},
            ]
        }))
        .expect("usage");
        assert_eq!(u.plan.as_deref(), Some("team"));
        let rows: Vec<_> = u
            .limits
            .iter()
            .map(|l| (l.kind, l.percent, l.warning, l.model.clone()))
            .collect();
        assert_eq!(
            rows,
            [
                (LimitKind::Session, 3, false, None),
                (LimitKind::Weekly, 86, true, None),
                (LimitKind::WeeklyModel, 59, false, Some("Fable".to_string())),
            ]
        );
        assert_eq!(
            u.limits[0].resets_at_ms.map(|ms| ms / 1000 % 86_400 / 3600),
            Some(7)
        );

        let old = parse_plan_usage(&json!({
            "rate_limits": {
                "five_hour": {"utilization": 12, "resets_at": "2026-10-25T07:30:00+00:00"},
                "seven_day": {"utilization": 40},
            }
        }))
        .expect("old");
        let pcts: Vec<_> = old.limits.iter().map(|l| l.percent).collect();
        assert_eq!(pcts, [12, 40]);
        assert_eq!(parse_plan_usage(&json!({"limits": []})), None);
    }

    #[test]
    fn contexto_tokens_da_mensagem_janela_do_result() {
        let assistant = json!({
            "type": "assistant",
            "parent_tool_use_id": null,
            "message": {"usage": {
                "input_tokens": 2,
                "cache_creation_input_tokens": 300,
                "cache_read_input_tokens": 36700,
                "output_tokens": 50,
            }},
        });
        let o = assistant.as_object().expect("obj");
        assert_eq!(
            parse_message_context(o),
            Some(ChatEvent::Context {
                tokens: 37_002,
                window: 0
            })
        );
        let sub =
            json!({"parent_tool_use_id": "toolu_1", "message": {"usage": {"input_tokens": 9}}});
        assert_eq!(parse_message_context(sub.as_object().expect("obj")), None);
        let result = json!({
            "usage": {"input_tokens": 2, "cache_creation_input_tokens": 36932},
            "modelUsage": {"claude-haiku-5-5": {"contextWindow": 1_000_000}},
        });
        assert_eq!(
            parse_result_context(result.as_object().expect("obj")),
            Some(ChatEvent::Context {
                tokens: 0,
                window: 1_000_000
            })
        );
    }

    #[test]
    fn comandos_do_initialize() {
        let c = parse_commands(&json!({
            "commands": [
                {"name": "escanor",
                 "description": "Invoca o Escanor, backend PHP/Laravel (user)",
                 "argumentHint": ""},
                {"name": "review", "description": "Revisa", "aliases": ["rev"]},
                {"description": "sem nome"},
            ]
        }));
        let names: Vec<_> = c.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["escanor", "review"]);
        assert_eq!(c[0].description, "Invoca o Escanor, backend PHP/Laravel");
        assert_eq!(c[1].aliases, ["rev"]);
    }
}
