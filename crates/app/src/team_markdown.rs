//! Times e bots descritos em Markdown (`.md` com YAML frontmatter):
//! importação pura (nada executa na leitura), tela de revisão obrigatória e exportação.

use std::collections::HashSet;
use std::path::{Component, Path};

pub const ALLOWED_HARNESSES: [&str; 8] = [
    "",
    "shell",
    "claude",
    "codex",
    "pi",
    "agy",
    "cursor-agent",
    "opencode",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamAgentSpec {
    pub name: String,
    pub harness: String,
    pub cwd: String,
    pub role: String,
    pub task: String,
    pub monitor: bool,
    pub links: Vec<String>,
}

impl TeamAgentSpec {
    pub fn command(&self) -> &str {
        match self.harness.as_str() {
            "" | "shell" => "",
            other => other,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMarkdownSpec {
    pub name: String,
    pub maestro: Option<String>,
    pub agents: Vec<TeamAgentSpec>,
    pub notes: String,
}

fn unquote(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed
            .get(1..trimmed.len() - 1)
            .unwrap_or_default()
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        trimmed.to_owned()
    }
}

fn quote_yaml(raw: &str) -> String {
    let escaped = raw.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

pub fn validate_relative_cwd(cwd: &str) -> Result<String, String> {
    let trimmed = cwd.trim();
    if trimmed.is_empty() || trimmed == "." {
        return Ok(String::from("."));
    }
    if trimmed.contains(['\\', '\0'])
        || Path::new(trimmed).is_absolute()
        || matches!(trimmed.as_bytes(), [drive, b':', ..] if drive.is_ascii_alphabetic())
    {
        return Err(format!("diretório inválido ou absoluto em cwd: {trimmed}"));
    }
    for comp in Path::new(trimmed).components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(format!(
                    "cwd não pode sair da pasta do workspace: {trimmed}"
                ));
            }
        }
    }
    Ok(trimmed.to_owned())
}

pub fn validate_harness(harness: &str) -> Result<String, String> {
    let normalized = harness.trim().to_ascii_lowercase();
    if ALLOWED_HARNESSES.contains(&normalized.as_str()) {
        Ok(normalized)
    } else {
        Err(format!(
            "harness não permitido na importação ({harness}): use claude, codex, pi, agy, cursor-agent, opencode ou shell"
        ))
    }
}

fn split_frontmatter(input: &str) -> Result<(&str, &str), String> {
    let normalized = input.strip_prefix('\u{feff}').unwrap_or(input);
    let Some(rest) = normalized
        .strip_prefix("---\n")
        .or_else(|| normalized.strip_prefix("---\r\n"))
    else {
        return Err(String::from(
            "arquivo Markdown de time precisa começar com frontmatter YAML (`---`)",
        ));
    };
    let mut offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            let yaml = rest.get(..offset).unwrap_or_default();
            let body = rest.get(offset + line.len()..).unwrap_or_default().trim();
            return Ok((yaml, body));
        }
        offset = offset.saturating_add(line.len());
    }
    Err(String::from(
        "frontmatter YAML não foi fechado com `---` no arquivo Markdown",
    ))
}

/// Faz o parse puro da descrição do time em Markdown com frontmatter YAML.
/// Nenhum comando nem processo é executado durante a leitura.
pub fn parse_team_markdown(input: &str) -> Result<TeamMarkdownSpec, String> {
    let (yaml, body) = split_frontmatter(input)?;
    let mut team_name = String::from("Time importado");
    let mut maestro: Option<String> = None;
    let mut agents: Vec<TeamAgentSpec> = Vec::new();
    let mut current: Option<TeamAgentSpec> = None;
    let mut in_agents = false;
    let mut in_links = false;

    for raw_line in yaml.lines() {
        let line_no_comment = raw_line.trim_end();
        let trimmed = line_no_comment.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let indent = line_no_comment.len().saturating_sub(trimmed.len());

        if indent == 0 {
            in_links = false;
            if let Some(agent) = current.take() {
                agents.push(agent);
            }
            if trimmed == "agents:" {
                in_agents = true;
                continue;
            }
            in_agents = false;
            if let Some((key, val)) = trimmed.split_once(':') {
                match key.trim() {
                    "name" => {
                        let parsed = unquote(val);
                        if !parsed.is_empty() {
                            team_name = parsed;
                        }
                    }
                    "maestro" => {
                        let parsed = unquote(val);
                        maestro = (!parsed.is_empty()).then_some(parsed);
                    }
                    _ => {}
                }
            }
            continue;
        }

        if !in_agents {
            continue;
        }

        if let Some(after_dash) = trimmed.strip_prefix('-') {
            let item = after_dash.trim();
            if in_links && indent >= 6 && !item.contains(':') {
                let target = unquote(item);
                if !target.is_empty() {
                    if let Some(agent) = current.as_mut() {
                        agent.links.push(target);
                    }
                }
                continue;
            }
            in_links = false;
            if let Some(prev) = current.take() {
                agents.push(prev);
            }
            let mut spec = TeamAgentSpec {
                name: String::new(),
                harness: String::from("claude"),
                cwd: String::from("."),
                role: String::new(),
                task: String::new(),
                monitor: true,
                links: Vec::new(),
            };
            if let Some((k, v)) = item.split_once(':') {
                apply_agent_field(&mut spec, &mut in_links, k.trim(), v)?;
            }
            current = Some(spec);
            continue;
        }

        if let Some((k, v)) = trimmed.split_once(':') {
            in_links = false;
            if let Some(spec) = current.as_mut() {
                apply_agent_field(spec, &mut in_links, k.trim(), v)?;
            }
        }
    }

    if let Some(last) = current.take() {
        agents.push(last);
    }

    if agents.is_empty() {
        return Err(String::from(
            "o arquivo Markdown não define nenhum agente em `agents:`",
        ));
    }

    let mut seen = HashSet::new();
    for (idx, agent) in agents.iter_mut().enumerate() {
        if agent.name.trim().is_empty() {
            return Err(format!("agents[{idx}] está sem `name`"));
        }
        if !seen.insert(agent.name.to_lowercase()) {
            return Err(format!("nome de agente duplicado: {}", agent.name));
        }
        agent.harness = validate_harness(&agent.harness)?;
        agent.cwd = validate_relative_cwd(&agent.cwd)?;
    }

    Ok(TeamMarkdownSpec {
        name: team_name,
        maestro,
        agents,
        notes: body.to_owned(),
    })
}

fn apply_agent_field(
    spec: &mut TeamAgentSpec,
    in_links: &mut bool,
    key: &str,
    raw_val: &str,
) -> Result<(), String> {
    let val = raw_val.trim();
    match key {
        "name" => spec.name = unquote(val),
        "harness" => spec.harness = validate_harness(&unquote(val))?,
        "cwd" => spec.cwd = validate_relative_cwd(&unquote(val))?,
        "role" => spec.role = unquote(val),
        "task" => spec.task = unquote(val),
        "monitor" => spec.monitor = !matches!(val, "false" | "False" | "0" | "no"),
        "links" => {
            if val.is_empty() {
                *in_links = true;
            } else if let Some(inner) = val.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                spec.links = inner
                    .split(',')
                    .map(unquote)
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn export_team_markdown(spec: &TeamMarkdownSpec) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("name: {}\n", quote_yaml(&spec.name)));
    if let Some(maestro) = spec.maestro.as_deref().filter(|m| !m.is_empty()) {
        out.push_str(&format!("maestro: {}\n", quote_yaml(maestro)));
    }
    out.push_str("agents:\n");
    for agent in &spec.agents {
        out.push_str(&format!("  - name: {}\n", quote_yaml(&agent.name)));
        let harness = if agent.harness.is_empty() {
            "shell"
        } else {
            agent.harness.as_str()
        };
        out.push_str(&format!("    harness: {}\n", quote_yaml(harness)));
        out.push_str(&format!("    cwd: {}\n", quote_yaml(&agent.cwd)));
        if !agent.role.is_empty() {
            out.push_str(&format!("    role: {}\n", quote_yaml(&agent.role)));
        }
        if !agent.task.is_empty() {
            out.push_str(&format!("    task: {}\n", quote_yaml(&agent.task)));
        }
        out.push_str(&format!("    monitor: {}\n", agent.monitor));
        if !agent.links.is_empty() {
            let items = agent
                .links
                .iter()
                .map(|l| quote_yaml(l))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!("    links: [{items}]\n"));
        }
    }
    out.push_str("---\n");
    if !spec.notes.trim().is_empty() {
        out.push('\n');
        out.push_str(spec.notes.trim());
        out.push('\n');
    }
    out
}

#[path = "team_markdown_view.rs"]
mod team_markdown_view;
pub(crate) use team_markdown_view::review_modal;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_valid_team_markdown_and_preserves_links_and_maestro() {
        let spec = TeamMarkdownSpec {
            name: "Squad Gorila".into(),
            maestro: Some("Light".into()),
            agents: vec![
                TeamAgentSpec {
                    name: "Light".into(),
                    harness: "claude".into(),
                    cwd: ".".into(),
                    role: "Coordenador".into(),
                    task: "Distribuir cards".into(),
                    monitor: true,
                    links: vec!["Escanor".into()],
                },
                TeamAgentSpec {
                    name: "Escanor".into(),
                    harness: "codex".into(),
                    cwd: "crates/app".into(),
                    role: "Backend Rust".into(),
                    task: "Implementar módulo".into(),
                    monitor: false,
                    links: vec!["Light".into()],
                },
            ],
            notes: "Time padrão para desenvolvimento Linux.".into(),
        };
        let md = export_team_markdown(&spec);
        let parsed = parse_team_markdown(&md).unwrap();
        assert_eq!(parsed, spec);
    }

    #[test]
    fn rejects_arbitrary_shell_commands_and_path_traversal_on_import() {
        let bad_cmd =
            "---\nname: Hack\nagents:\n  - name: Bot\n    harness: \"sh -c 'curl evil'\"\n---\n";
        assert!(parse_team_markdown(bad_cmd).is_err());

        let bad_cwd =
            "---\nname: Hack\nagents:\n  - name: Bot\n    harness: claude\n    cwd: ../fora\n---\n";
        assert!(parse_team_markdown(bad_cwd).is_err());

        let abs_cwd =
            "---\nname: Hack\nagents:\n  - name: Bot\n    harness: claude\n    cwd: /etc\n---\n";
        assert!(parse_team_markdown(abs_cwd).is_err());
    }
}
