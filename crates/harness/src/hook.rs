//! Arquivo de script que um hook chama.

use std::path::{Path, PathBuf};

use crate::paths::normalize;

/// Interpretadores cujo primeiro argumento é o script.
const INTERPRETERS: [&str; 8] = [
    "bash", "sh", "zsh", "node", "python", "python3", "bun", "deno",
];

/// Arquivo de script que o hook chama, para conferir se existe. `None` = é um
/// programa do PATH (`gsh`, `npx`...) ou não dá para dizer.
pub fn hook_script_path(command: &str, workspace: &Path, home: &Path) -> Option<PathBuf> {
    let workspace_text = workspace.to_string_lossy();
    let replaced = command
        .replace("${CLAUDE_PROJECT_DIR}", &workspace_text)
        .replace("$CLAUDE_PROJECT_DIR", &workspace_text);
    let mut rest = replaced.trim().to_owned();
    let mut token = take(&mut rest);
    if INTERPRETERS.contains(&token.as_str()) {
        token = take(&mut rest);
    }
    if token.is_empty() || token.starts_with('-') {
        return None;
    }
    if let Some(tail) = token.strip_prefix("~/") {
        return Some(home.join(tail));
    }
    if Path::new(&token).is_absolute() {
        return Some(PathBuf::from(token));
    }
    // Sem barra é comando do PATH; com barra, relativo à pasta do projeto.
    if !token.contains('/') {
        return None;
    }
    Some(normalize(&workspace.join(token)))
}

/// Primeiro argumento de `rest` (entre aspas ou até o espaço); o resto fica em `rest`.
fn take(rest: &mut String) -> String {
    let Some(first) = rest.chars().next() else {
        return String::new();
    };
    if first == '"' || first == '\'' {
        let body = rest.get(1..).unwrap_or_default();
        return match body.find(first) {
            Some(end) => {
                let token = body.get(..end).unwrap_or_default().to_owned();
                *rest = body.get(end + 1..).unwrap_or_default().trim().to_owned();
                token
            }
            None => {
                let token = body.to_owned();
                rest.clear();
                token
            }
        };
    }
    match rest.find(' ') {
        Some(space) => {
            let token = rest.get(..space).unwrap_or_default().to_owned();
            *rest = rest.get(space + 1..).unwrap_or_default().trim().to_owned();
            token
        }
        None => std::mem::take(rest),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(command: &str) -> Option<PathBuf> {
        hook_script_path(command, Path::new("/w"), Path::new("/h"))
    }

    #[test]
    fn same_vectors_as_the_dart_test() {
        assert_eq!(
            path("\"/opt/x y/gsh\" hook"),
            Some(PathBuf::from("/opt/x y/gsh"))
        );
        assert_eq!(path("gsh hook"), None);
        assert_eq!(
            path("bash ./scripts/n.sh"),
            Some(PathBuf::from("/w/scripts/n.sh"))
        );
        assert_eq!(
            path("$CLAUDE_PROJECT_DIR/.claude/hooks/a.sh"),
            Some(PathBuf::from("/w/.claude/hooks/a.sh"))
        );
        assert_eq!(path("~/bin/x"), Some(PathBuf::from("/h/bin/x")));
    }

    #[test]
    fn wired_hook_command_resolves_inside_the_project() {
        assert_eq!(
            path("bash \"${CLAUDE_PROJECT_DIR}/.claude/hooks/a.sh\""),
            Some(PathBuf::from("/w/.claude/hooks/a.sh"))
        );
    }

    #[test]
    fn flags_empty_and_unclosed_quotes() {
        assert_eq!(path(""), None);
        assert_eq!(path("bash"), None);
        assert_eq!(path("--version"), None);
        assert_eq!(path("\"/opt/x"), Some(PathBuf::from("/opt/x")));
    }
}
