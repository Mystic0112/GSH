use serde_json::{json, Value};

use crate::island::{PermissionAsk, PermissionDecision};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudePermissionAsk {
    pub id: String,
    pub tool_name: String,
    pub input_summary: String,
    pub always_rule: Option<String>,
    always_raw: Option<Value>,
}

impl ClaudePermissionAsk {
    pub fn parse(id: impl Into<String>, args: &Value) -> Option<Self> {
        let tool_name = args.get("tool_name").and_then(Value::as_str)?.trim();
        if tool_name.is_empty() {
            return None;
        }
        let input = args.get("tool_input").and_then(Value::as_object);
        let suggestions = args.get("permission_suggestions").and_then(Value::as_array);
        let (always_rule, always_raw) = suggestions
            .and_then(|items| {
                items
                    .iter()
                    .find_map(|item| visible_allow_rule(item, tool_name))
            })
            .map_or((None, None), |(description, rule)| {
                (Some(description), Some(rule))
            });
        let summary = input.map_or_else(String::new, tool_input_summary);
        Some(Self {
            id: id.into(),
            tool_name: tool_name.to_owned(),
            input_summary: summary,
            always_rule,
            always_raw,
        })
    }

    pub fn can_always(&self) -> bool {
        self.always_rule.is_some()
    }

    pub fn notification_body(&self) -> String {
        let detail = crate::labels::format_tool_line(&self.tool_name, &self.input_summary);
        match &self.always_rule {
            Some(rule) => format!("Sempre: {rule}\n{detail}"),
            None => detail,
        }
    }

    pub fn hook_output(&self, decision: PermissionDecision) -> Option<Value> {
        let hook_decision = match decision {
            PermissionDecision::Pass => return None,
            PermissionDecision::Deny => json!({
                "behavior": "deny",
                "message": "Negado pelo usuário no GSH"
            }),
            PermissionDecision::Allow => json!({"behavior": "allow"}),
            PermissionDecision::AllowAlways => json!({
                "behavior": "allow",
                "updatedPermissions": [self.always_raw.as_ref()?]
            }),
        };
        Some(json!({
            "hookSpecificOutput": {
                "hookEventName": "PermissionRequest",
                "decision": hook_decision
            }
        }))
    }
}

impl From<&ClaudePermissionAsk> for PermissionAsk {
    fn from(ask: &ClaudePermissionAsk) -> Self {
        Self {
            id: ask.id.clone(),
            tool_name: ask.tool_name.clone(),
            input_summary: ask.input_summary.clone(),
            always_rule: ask.always_rule.clone(),
        }
    }
}

fn visible_allow_rule(suggestion: &Value, tool_name: &str) -> Option<(String, Value)> {
    if suggestion.get("type")?.as_str()? != "addRules"
        || suggestion.get("behavior")?.as_str()? != "allow"
        || !tool_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return None;
    }
    let [rule] = suggestion.get("rules")?.as_array()?.as_slice() else {
        return None;
    };
    if rule.get("toolName")?.as_str()? != tool_name {
        return None;
    }
    let mut displayed = match rule.get("ruleContent") {
        Some(content) => {
            let content = content.as_str()?;
            if content.is_empty()
                || content.trim() != content
                || !content
                    .bytes()
                    .all(|byte| byte.is_ascii_graphic() || byte == b' ')
            {
                return None;
            }
            format!("{tool_name}({content})")
        }
        None => format!("{tool_name} (qualquer entrada)"),
    };
    let destination = suggestion.get("destination")?.as_str()?;
    let scope = match destination {
        "localSettings" => "projeto, local",
        "projectSettings" => "projeto, compartilhado",
        "userSettings" => "usuário",
        "session" => "sessão",
        _ => return None,
    };
    displayed.push_str(&format!(" · {scope}"));
    let update_rule = match rule.get("ruleContent") {
        Some(content) => json!({"toolName": tool_name, "ruleContent": content}),
        None => json!({"toolName": tool_name}),
    };
    let update = json!({
        "type": "addRules",
        "behavior": "allow",
        "rules": [update_rule],
        "destination": destination
    });
    // shortcut: leave long rules to the terminal until native previews can guarantee full text.
    (displayed.chars().count() <= 70).then_some((displayed, update))
}

pub fn permission_key(id: &str) -> String {
    format!("ask:{id}")
}

pub fn tool_input_summary(input: &serde_json::Map<String, Value>) -> String {
    for key in [
        "command",
        "file_path",
        "notebook_path",
        "url",
        "pattern",
        "query",
        "description",
    ] {
        if let Some(value) = input
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return value.to_owned();
        }
    }
    let raw = Value::Object(input.clone()).to_string();
    let mut chars = raw.chars();
    let head: String = chars.by_ref().take(159).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializa_decisoes_com_a_regra_validada() {
        let ask = ClaudePermissionAsk::parse("p1", &json!({
            "tool_name": "Bash",
            "tool_input": {"command": "cargo test"},
            "permission_suggestions": [{"type": "addRules", "behavior": "allow", "destination": "localSettings", "rules": [{"toolName": "Bash", "ruleContent": "cargo test *"}]}]
        })).unwrap();
        assert_eq!(
            ask.hook_output(PermissionDecision::Allow).unwrap()["hookSpecificOutput"]["decision"],
            json!({"behavior":"allow"})
        );
        assert_eq!(
            ask.hook_output(PermissionDecision::AllowAlways).unwrap()["hookSpecificOutput"]
                ["decision"],
            json!({"behavior":"allow","updatedPermissions":[{"type":"addRules","behavior":"allow","destination":"localSettings","rules":[{"toolName":"Bash","ruleContent":"cargo test *"}]}]})
        );
        assert_eq!(
            ask.hook_output(PermissionDecision::Deny).unwrap()["hookSpecificOutput"]["decision"]
                ["behavior"],
            "deny"
        );
        assert!(ask.hook_output(PermissionDecision::Pass).is_none());
    }

    #[test]
    fn rejeita_sugestao_sem_regra_e_resume_input() {
        let ask = ClaudePermissionAsk::parse(
            "p2",
            &json!({
                "tool_name": "Bash",
                "tool_input": {"command": "cargo check"},
                "permission_suggestions": [{"type": "addRules", "behavior": "allow", "rules": []}]
            }),
        )
        .unwrap();
        assert!(ask.always_rule.is_none());
        assert_eq!(ask.input_summary, "cargo check");
    }

    #[test]
    fn sempre_exibe_o_escopo_e_nao_aceita_regras_ocultas() {
        let rule = json!({"toolName": "Bash", "ruleContent": "cargo test *"});
        for (rules, destination, expected) in [
            (json!([rule]), "localSettings", true),
            (json!([rule, {"toolName": "Read"}]), "localSettings", false),
            (json!([{"toolName": "Read"}]), "localSettings", false),
            (
                json!([{"toolName": "Bash", "ruleContent": "cargo test\nrm *"}]),
                "localSettings",
                false,
            ),
            (json!([rule]), "managedSettings", false),
        ] {
            let request = json!({
                "tool_name": "Bash", "tool_input": {"command": "cargo test"},
                "permission_suggestions": [{"type": "addRules", "behavior": "allow", "rules": rules, "destination": destination}]
            });
            let ask = ClaudePermissionAsk::parse("p3", &request).unwrap();
            assert_eq!(ask.can_always(), expected);
            if expected {
                assert!(ask
                    .notification_body()
                    .starts_with("Sempre: Bash(cargo test *) · projeto, local\n"));
                let output = ask.hook_output(PermissionDecision::AllowAlways).unwrap();
                assert_eq!(
                    output["hookSpecificOutput"]["decision"]["updatedPermissions"][0],
                    request["permission_suggestions"][0]
                );
            } else {
                assert!(ask.hook_output(PermissionDecision::AllowAlways).is_none());
            }
        }
    }

    #[test]
    fn rejeita_pedido_sem_tool_name() {
        assert!(ClaudePermissionAsk::parse("p3", &json!({"tool_input": {}})).is_none());
    }
}
