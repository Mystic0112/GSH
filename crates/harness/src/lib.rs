//! O que o Claude Code carrega ao rodar numa pasta (aba Harness), sem UI.
//!
//! Porta de `project_harness.dart` e `claude_harness_source.dart` do Flutter:
//! lê os arquivos direto do disco (sem chamar a CLI) e cria itens novos pelos
//! modelos, sem nunca sobrescrever.
//! - [`read`]: instruções, skills, subagentes, comandos, hooks, permissões e MCP;
//! - [`create`]: arquivo novo de uma seção (e o hook ligado no `settings.json`).

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

mod create;
mod frontmatter;
mod hook;
mod json;
mod paths;
mod read;
mod templates;

use std::path::PathBuf;

pub use create::{create, slug, CreateError};
pub use frontmatter::parse_frontmatter;
pub use hook::hook_script_path;
pub use read::read;

/// Acima disto o Claude Code avisa que o CLAUDE.md pesa na performance.
pub const MAX_INSTRUCTION_CHARS: u64 = 40000;

/// Moldura da lista de um item (skill, subagente, comando) no contexto.
const LISTING_FRAME_CHARS: usize = 12;

/// De onde vem um item: do usuário (`~/.claude`, vale em todo projeto), do
/// projeto (versionado) ou local (só nesta máquina).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    User,
    Project,
    Local,
}

/// Seções da aba, na ordem em que aparecem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Memory,
    Skills,
    Agents,
    Commands,
    Hooks,
    Permissions,
    Mcp,
}

impl Kind {
    pub const ALL: [Self; 7] = [
        Self::Memory,
        Self::Skills,
        Self::Agents,
        Self::Commands,
        Self::Hooks,
        Self::Permissions,
        Self::Mcp,
    ];
}

/// Problema que o linter acha num item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Issue {
    /// Skill/subagente sem `description`: o agente não sabe quando usar.
    NoDescription,
    /// Hook chama um script que não existe.
    MissingScript,
    /// Instrução grande demais: pesa em todo turno.
    TooLarge,
}

/// Um item do harness e o arquivo onde ele mora.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub kind: Kind,
    pub name: String,
    /// Descrição, comando, matcher... (uma linha).
    pub detail: String,
    pub scope: Scope,
    /// Arquivo que define o item (abre no editor). `None` = não editável aqui.
    pub path: Option<PathBuf>,
    pub issues: Vec<Issue>,
    /// Caracteres que o item põe no contexto em **todo** turno (0 = sob demanda).
    pub always_on_chars: usize,
}

/// Tudo que o agente carrega ao rodar no workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectHarness {
    pub entries: Vec<Entry>,
    /// JSON que não abre: o Claude Code ignora o arquivo inteiro sem avisar.
    pub invalid_files: Vec<PathBuf>,
}

impl ProjectHarness {
    /// Tokens fixos por turno (≈ caracteres / 4) da seção `kind`, ou de todas.
    pub fn always_on_tokens(&self, kind: Option<Kind>) -> usize {
        let chars: usize = self
            .entries
            .iter()
            .filter(|entry| kind.is_none_or(|k| entry.kind == k))
            .map(|entry| entry.always_on_chars)
            .sum();
        (chars as f64 / 4.0).round() as usize
    }

    pub fn issue_count(&self) -> usize {
        self.entries
            .iter()
            .map(|entry| entry.issues.len())
            .sum::<usize>()
            + self.invalid_files.len()
    }

    /// Itens da seção: do projeto primeiro, depois os locais e por último os do
    /// usuário. Dentro do escopo, a ordem de leitura.
    pub fn of(&self, kind: Kind) -> Vec<&Entry> {
        [Scope::Project, Scope::Local, Scope::User]
            .into_iter()
            .flat_map(|scope| {
                self.entries
                    .iter()
                    .filter(move |entry| entry.kind == kind && entry.scope == scope)
            })
            .collect()
    }
}

/// O que um item listado põe no contexto de todo turno: nome + descrição mais a
/// moldura da lista. Conta em unidades UTF-16, como o `String.length` do Dart.
pub(crate) fn listing_chars(name: &str, description: &str) -> usize {
    utf16_len(name) + utf16_len(description) + LISTING_FRAME_CHARS
}

pub(crate) fn utf16_len(text: &str) -> usize {
    text.encode_utf16().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: Kind, name: &str, scope: Scope, chars: usize) -> Entry {
        Entry {
            kind,
            name: name.to_owned(),
            detail: String::new(),
            scope,
            path: None,
            issues: Vec::new(),
            always_on_chars: chars,
        }
    }

    #[test]
    fn tokens_are_chars_over_four_rounded() {
        let harness = ProjectHarness {
            entries: vec![
                entry(Kind::Memory, "a", Scope::User, 10),
                entry(Kind::Skills, "b", Scope::User, 3),
            ],
            invalid_files: Vec::new(),
        };
        assert_eq!(harness.always_on_tokens(Some(Kind::Memory)), 3); // 2.5 -> 3
        assert_eq!(harness.always_on_tokens(Some(Kind::Skills)), 1); // 0.75 -> 1
        assert_eq!(harness.always_on_tokens(None), 3); // 3.25 -> 3
        assert_eq!(harness.always_on_tokens(Some(Kind::Mcp)), 0);
    }

    #[test]
    fn of_orders_project_then_local_then_user() {
        let harness = ProjectHarness {
            entries: vec![
                entry(Kind::Hooks, "user", Scope::User, 0),
                entry(Kind::Hooks, "local", Scope::Local, 0),
                entry(Kind::Hooks, "p1", Scope::Project, 0),
                entry(Kind::Mcp, "other", Scope::Project, 0),
                entry(Kind::Hooks, "p2", Scope::Project, 0),
            ],
            invalid_files: Vec::new(),
        };
        let names: Vec<&str> = harness
            .of(Kind::Hooks)
            .into_iter()
            .map(|e| e.name.as_str())
            .collect();
        assert_eq!(names, ["p1", "p2", "local", "user"]);
    }

    #[test]
    fn listing_counts_utf16_units_plus_frame() {
        assert_eq!(listing_chars("ab", "çd"), 2 + 2 + 12);
        assert_eq!(listing_chars("a", "😀"), 1 + 2 + 12);
    }
}
