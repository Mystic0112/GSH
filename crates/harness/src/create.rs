//! Criação de itens pelos modelos (`ClaudeHarnessSource.create` do Flutter).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::json::Json;
use crate::templates::{agent_md, skill_md, CLAUDE_MD, COMMAND_MD, HOOK_COMMAND_PREFIX, HOOK_SH};
use crate::Kind;

/// Falha ao criar o arquivo (o Flutter só registra no log e não abre nada).
#[derive(Debug, Error)]
pub enum CreateError {
    #[error("falha ao gravar {path}")]
    Write { path: PathBuf, source: io::Error },
    #[error("falha ao ler {path}")]
    Read { path: PathBuf, source: io::Error },
}

/// Nome digitado -> nome de arquivo (`Revisar PR` -> `revisar-pr`); vazio vira `novo`.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        let allowed = c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-');
        let c = if allowed { c } else { '-' };
        // `[^a-z0-9._-]+` -> `-` e depois `-+` -> `-`: nunca dois `-` seguidos
        if c != '-' || !out.ends_with('-') {
            out.push(c);
        }
    }
    let trimmed = out.trim_matches(|c| c == '-' || c == '.');
    if trimmed.is_empty() {
        String::from("novo")
    } else {
        trimmed.to_owned()
    }
}

/// Cria o arquivo da seção pelo modelo (sem sobrescrever o que já existe) e devolve o
/// caminho para abrir no editor. `name` vale para skills, subagentes, comandos e hooks;
/// hooks sem nome e Permissões só acham o `.claude/settings.json`, o MCP o `.mcp.json`.
pub fn create(workspace: &Path, kind: Kind, name: Option<&str>) -> Result<PathBuf, CreateError> {
    let slug = slug(name.unwrap_or(""));
    let project = workspace.join(".claude");
    let (path, body) = match kind {
        Kind::Memory => (workspace.join("CLAUDE.md"), CLAUDE_MD.to_owned()),
        Kind::Skills => (
            project.join("skills").join(&slug).join("SKILL.md"),
            skill_md(&slug),
        ),
        Kind::Agents => (
            project.join("agents").join(format!("{slug}.md")),
            agent_md(&slug),
        ),
        Kind::Commands => (
            project.join("commands").join(format!("{slug}.md")),
            COMMAND_MD.to_owned(),
        ),
        Kind::Hooks if name.is_some() => (
            project.join("hooks").join(format!("{slug}.sh")),
            HOOK_SH.to_owned(),
        ),
        Kind::Hooks | Kind::Permissions => (project.join("settings.json"), String::from("{\n}\n")),
        Kind::Mcp => (
            workspace.join(".mcp.json"),
            String::from("{\n  \"mcpServers\": {}\n}\n"),
        ),
    };
    let write_error = |source| CreateError::Write {
        path: path.clone(),
        source,
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(write_error)?;
    }
    // create_new: nunca sobrescreve, nem se o arquivo surgir entre a checagem e a gravação.
    let created = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .and_then(|mut file| file.write_all(body.as_bytes()));
    match created {
        Ok(()) => {
            if kind == Kind::Hooks && name.is_some() {
                wire_hook(&project.join("settings.json"), &slug)?;
            }
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists && path.is_file() => {}
        Err(e) => return Err(write_error(e)),
    }
    Ok(path)
}

/// Liga o script novo como `PreToolUse` de `Bash` no `settings.json` do projeto. JSON
/// quebrado não é tocado (o lint já avisa dele).
fn wire_hook(settings: &Path, slug: &str) -> Result<(), CreateError> {
    let mut root = if settings.is_file() {
        let bytes = fs::read(settings).map_err(|source| CreateError::Read {
            path: settings.to_path_buf(),
            source,
        })?;
        match String::from_utf8(bytes)
            .ok()
            .and_then(|t| Json::parse(&t).ok())
        {
            Some(json) => json,
            None => return Ok(()),
        }
    } else {
        Json::Obj(Vec::new())
    };
    let Json::Obj(top) = &mut root else {
        return Ok(());
    };
    let Some(Json::Obj(hooks)) = slot(top, "hooks", || Json::Obj(Vec::new())) else {
        return Ok(());
    };
    let Some(Json::Arr(pre)) = slot(hooks, "PreToolUse", || Json::Arr(Vec::new())) else {
        return Ok(());
    };
    let command = format!("{HOOK_COMMAND_PREFIX}{slug}.sh\"");
    pre.push(Json::Obj(vec![
        (String::from("matcher"), Json::Str(String::from("Bash"))),
        (
            String::from("hooks"),
            Json::Arr(vec![Json::Obj(vec![
                (String::from("type"), Json::Str(String::from("command"))),
                (String::from("command"), Json::Str(command)),
            ])]),
        ),
    ]));
    let text = format!("{}\n", root.to_pretty());
    fs::write(settings, text).map_err(|source| CreateError::Write {
        path: settings.to_path_buf(),
        source,
    })
}

/// `map[key] ??= make()` do Dart: cria quando falta ou é `null`.
fn slot<'a>(
    items: &'a mut Vec<(String, Json)>,
    key: &str,
    make: impl FnOnce() -> Json,
) -> Option<&'a mut Json> {
    match items.iter().position(|(k, _)| k == key) {
        Some(index) => {
            let (_, value) = items.get_mut(index)?;
            if value.is_null() {
                *value = make();
            }
            Some(value)
        }
        None => {
            items.push((key.to_owned(), make()));
            items.last_mut().map(|(_, value)| value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::read;

    #[test]
    fn slug_vectors() {
        assert_eq!(slug("Revisar PR!"), "revisar-pr");
        assert_eq!(slug("../../etc"), "etc");
        assert_eq!(slug("  "), "novo");
        assert_eq!(slug("a  b--c"), "a-b-c");
        assert_eq!(slug("Ação"), "a-o");
        assert_eq!(slug("v1.2_x"), "v1.2_x");
        assert_eq!(slug("..."), "novo");
    }

    #[test]
    fn creates_by_template_without_overwriting() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("repo");
        fs::create_dir_all(&ws).unwrap();
        fs::write(ws.join("CLAUDE.md"), "# repo").unwrap();

        let skill = create(&ws, Kind::Skills, Some("Revisar PR!")).unwrap();
        assert_eq!(skill, ws.join(".claude/skills/revisar-pr/SKILL.md"));
        let text = fs::read_to_string(&skill).unwrap();
        assert!(text.contains("name: revisar-pr"));
        assert!(!text.contains("$name"));
        let cmd = create(&ws, Kind::Commands, Some("pr")).unwrap();
        assert!(cmd.ends_with("commands/pr.md"));
        assert!(fs::read_to_string(&cmd).unwrap().contains("$ARGUMENTS"));
        let agent = create(&ws, Kind::Agents, None).unwrap();
        assert!(agent.ends_with("agents/novo.md"));
        let md = create(&ws, Kind::Memory, None).unwrap();
        assert_eq!(fs::read_to_string(md).unwrap(), "# repo"); // já existia
        let mcp = create(&ws, Kind::Mcp, None).unwrap();
        assert_eq!(
            fs::read_to_string(mcp).unwrap(),
            "{\n  \"mcpServers\": {}\n}\n"
        );
        let settings = create(&ws, Kind::Permissions, None).unwrap();
        assert_eq!(fs::read_to_string(&settings).unwrap(), "{\n}\n");
        assert_eq!(create(&ws, Kind::Hooks, None).unwrap(), settings);
    }

    #[test]
    fn new_hook_is_wired_once_and_keeps_the_rest_of_settings() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let ws = dir.path().join("repo");
        fs::create_dir_all(ws.join(".claude")).unwrap();
        let settings = ws.join(".claude/settings.json");
        fs::write(
            &settings,
            "{\"zeta\": 1, \"hooks\": {\"Stop\": []}, \"alpha\": {\"k\": [1, 2]}}",
        )
        .unwrap();

        let sh = create(&ws, Kind::Hooks, Some("Sem force")).unwrap();
        assert_eq!(sh, ws.join(".claude/hooks/sem-force.sh"));
        assert!(fs::read_to_string(&sh).unwrap().contains("exit 2"));

        let harness = read(&ws, &home);
        let hooks = harness.of(Kind::Hooks);
        let hook = hooks
            .iter()
            .find(|e| e.detail.contains("sem-force.sh"))
            .unwrap();
        assert_eq!(hook.name, "PreToolUse · Bash");
        assert!(hook.issues.is_empty()); // o script existe

        // as chaves que já havia continuam, na ordem do arquivo
        let text = fs::read_to_string(&settings).unwrap();
        let (zeta, hooks_at, alpha) = (
            text.find("\"zeta\"").unwrap(),
            text.find("\"hooks\"").unwrap(),
            text.find("\"alpha\"").unwrap(),
        );
        assert!(zeta < hooks_at && hooks_at < alpha);
        assert!(text.ends_with("}\n"));

        // de novo com o mesmo nome: não duplica o hook
        create(&ws, Kind::Hooks, Some("Sem force")).unwrap();
        let again = read(&ws, &home);
        let count = again
            .of(Kind::Hooks)
            .iter()
            .filter(|e| e.detail.contains("sem-force"))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn broken_settings_json_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("repo");
        fs::create_dir_all(ws.join(".claude")).unwrap();
        let settings = ws.join(".claude/settings.json");
        fs::write(&settings, "{quebrado").unwrap();
        create(&ws, Kind::Hooks, Some("x")).unwrap();
        assert_eq!(fs::read_to_string(&settings).unwrap(), "{quebrado");
    }

    #[test]
    fn missing_settings_is_created_with_the_hook() {
        let dir = tempfile::tempdir().unwrap();
        let ws = dir.path().join("repo");
        create(&ws, Kind::Hooks, Some("x")).unwrap();
        let text = fs::read_to_string(ws.join(".claude/settings.json")).unwrap();
        assert!(text.contains("\"PreToolUse\""));
        assert!(text.contains("\"matcher\": \"Bash\""));
        assert!(text.contains("hooks/x.sh\\\"\""), "{text}");
    }
}
