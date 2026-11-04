//! Ambiente do processo: variáveis e caminhos.

use std::path::Path;

use gsh_core::wire::{ENV_PANE_ID, ENV_TAB_ID};

/// Variável de ambiente, tratando vazio como ausente.
pub fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|v| !v.is_empty())
}

/// Id da própria tab: `GSH_TAB_ID` com fallback para o legado `GSH_PANE_ID`.
/// O app injeta os dois; o fallback cobre binário novo com app antigo.
pub fn self_tab_id() -> Option<String> {
    env_non_empty(ENV_TAB_ID).or_else(|| env_non_empty(ENV_PANE_ID))
}

/// `HOME` (POSIX) ou `USERPROFILE` (Windows).
pub fn home_dir() -> Option<String> {
    env_non_empty("HOME").or_else(|| env_non_empty("USERPROFILE"))
}

/// Expande `~` e resolve caminho relativo contra o cwd, produzindo um caminho
/// absoluto. **Não normaliza `..`**: quem interpreta é o app.
pub fn resolve_path(path: &str) -> String {
    let expanded = match (home_dir(), path) {
        (Some(home), "~") => home,
        (Some(home), p) => match p.strip_prefix("~/") {
            Some(rest) => format!("{home}/{rest}"),
            None => p.to_string(),
        },
        (None, p) => p.to_string(),
    };
    if Path::new(&expanded).is_absolute() {
        return expanded;
    }
    match std::env::current_dir() {
        Ok(cwd) => cwd.join(&expanded).to_string_lossy().into_owned(),
        Err(_) => expanded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_path_mantem_absoluto_intacto() {
        // No Windows "/tmp/x" não é absoluto (falta o drive).
        let abs = if cfg!(windows) { "C:/tmp/x" } else { "/tmp/x" };
        assert_eq!(resolve_path(abs), abs);
    }

    #[test]
    fn resolve_path_relativo_ancora_no_cwd() {
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(
            resolve_path("a/../b"),
            cwd.join("a/../b").to_string_lossy(),
            "não normaliza .."
        );
    }
}
