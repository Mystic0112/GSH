use super::Failure;
use serde_json::Value;

pub(super) fn parse_subject(response: &str) -> Result<String, Failure> {
    let text = parse_events(response)?;
    subject(&text)
        .map(str::to_owned)
        .ok_or(Failure::InvalidResponse)
}

fn parse_events(response: &str) -> Result<String, Failure> {
    let mut text = None;
    let mut complete = false;
    for line in response.lines().filter(|line| !line.trim().is_empty()) {
        let event: Value = serde_json::from_str(line).map_err(|_| Failure::InvalidResponse)?;
        let kind = event
            .get("type")
            .and_then(Value::as_str)
            .ok_or(Failure::InvalidResponse)?;
        if complete {
            return Err(Failure::InvalidResponse);
        }
        let candidate = match kind {
            "agent_end" => {
                complete = true;
                None
            }
            "error" => return Err(Failure::InvalidResponse),
            "tool_execution_start" | "tool_execution_end" => return Err(Failure::ToolUse),
            "message_end"
                if event
                    .get("message")
                    .and_then(|m| m.get("role"))
                    .and_then(Value::as_str)
                    == Some("assistant") =>
            {
                let message = event.get("message").ok_or(Failure::InvalidResponse)?;
                if message.get("stopReason").and_then(Value::as_str) == Some("toolUse") {
                    return Err(Failure::ToolUse);
                }
                if message.get("stopReason").and_then(Value::as_str) != Some("stop") {
                    return Err(Failure::InvalidResponse);
                }
                let parts = message
                    .get("content")
                    .and_then(Value::as_array)
                    .ok_or(Failure::InvalidResponse)?;
                let mut text = String::new();
                for part in parts {
                    match part.get("type").and_then(Value::as_str) {
                        Some("text") => text.push_str(
                            part.get("text")
                                .and_then(Value::as_str)
                                .ok_or(Failure::InvalidResponse)?,
                        ),
                        Some("thinking") => {}
                        Some("toolCall") => return Err(Failure::ToolUse),
                        _ => return Err(Failure::InvalidResponse),
                    }
                }
                Some(text)
            }
            _ => None,
        };
        if let Some(candidate) = candidate {
            if text.replace(candidate).is_some() {
                return Err(Failure::InvalidResponse);
            }
        }
    }
    text.filter(|_| complete).ok_or(Failure::InvalidResponse)
}

pub(super) fn subject(response: &str) -> Option<&str> {
    let line = response.trim();
    let (header, description) = line.split_once(": ")?;
    if line.len() > 120
        || line.chars().any(char::is_control)
        || description.trim().is_empty()
        || description.starts_with(['`', '"', '\''])
    {
        return None;
    }
    let header = header.strip_suffix('!').unwrap_or(header);
    let commit_type = match header.split_once('(') {
        Some((commit_type, scope)) => {
            let scope = scope.strip_suffix(')')?;
            if scope.trim().is_empty() || scope.contains(['(', ')']) {
                return None;
            }
            commit_type
        }
        None => header,
    };
    (!commit_type.is_empty() && commit_type.bytes().all(|byte| byte.is_ascii_lowercase()))
        .then_some(line)
}
