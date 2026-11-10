//! Tipos de domínio do `gsh-git` (status, commits, hunks de diff).
//!
//! Ações remotas ou destrutivas (`push`, `pull`, `fetch`, `checkout`,
//! `discard`, `reset`, `stash`, `merge`, `rebase`) ficam fora desta fase:
//! // fase futura: push, pull, fetch, checkout, discard, reset, stash, merge, rebase

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Estado Git de um arquivo ou pasta agregada na árvore.
///
/// A ordem das variantes define a precedência de severidade (do mais fraco ao
/// mais forte), espelhando `GitFileStatus` de `git_file_status.dart`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GitFileStatus {
    /// Ignorado pelo `.gitignore` (`!`).
    Ignored,
    /// Arquivo novo ainda não rastreado (`?`).
    Untracked,
    /// Mudança no index (staged) sem edição pendente no working tree.
    Staged,
    /// Mudança no working tree ainda não comitada.
    Modified,
    /// Removido (no index ou no working tree).
    Deleted,
    /// Conflito de merge (`u` / `UU` / `AA` / `DD`).
    Conflict,
}

impl GitFileStatus {
    /// Retorna o estado mais severo entre dois candidatos.
    pub fn strongest(a: Option<Self>, b: Option<Self>) -> Option<Self> {
        match (a, b) {
            (None, other) => other,
            (other, None) => other,
            (Some(left), Some(right)) => Some(left.max(right)),
        }
    }

    /// Contribuição de um descendente para a cor/status de uma pasta ancestral.
    /// Deleção sobe como [`Self::Modified`] (igual a `strongestForFolder` no Flutter).
    pub fn strongest_for_folder(folder: Option<Self>, child: Self) -> Option<Self> {
        let adjusted = if child == Self::Deleted {
            Self::Modified
        } else {
            child
        };
        Self::strongest(folder, Some(adjusted))
    }

    /// Código curto de badge exibido na árvore e na lista de controle de versão.
    pub fn badge_code(self) -> &'static str {
        match self {
            Self::Ignored => "!",
            Self::Untracked => "U",
            Self::Staged => "A",
            Self::Modified => "M",
            Self::Deleted => "D",
            Self::Conflict => "C",
        }
    }
}

/// Estado completo do repositório retornado por `git status --porcelain=v2 --branch -z`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitRepoStatus {
    /// Nome da branch atual, short OID quando em detached HEAD, ou `"HEAD"`.
    pub branch: String,
    /// Commits à frente do upstream.
    pub ahead: usize,
    /// Commits atrás do upstream.
    pub behind: usize,
    /// Caminhos relativos (com `/`) com mudanças no index (seção Staged).
    pub staged: BTreeMap<String, GitFileStatus>,
    /// Caminhos relativos (com `/`) com mudanças no working tree (seção Changes).
    pub unstaged: BTreeMap<String, GitFileStatus>,
    /// Status combinado por arquivo sujo para colorir a árvore.
    pub files: BTreeMap<String, GitFileStatus>,
    /// Status agregado por diretório ancestral relativo (com `/`).
    pub folder_statuses: BTreeMap<String, GitFileStatus>,
    /// Raízes de pastas untracked colapsadas (sem `/` final).
    pub untracked_dirs: BTreeSet<String>,
    /// Raízes de caminhos ignorados pelo `.gitignore` (sem `/` final).
    pub ignored_roots: BTreeSet<String>,
}

impl GitRepoStatus {
    /// Retorna `true` quando não há arquivos staged nem unstaged.
    pub fn is_clean(&self) -> bool {
        self.files.is_empty() && self.staged.is_empty() && self.unstaged.is_empty()
    }

    /// Total de arquivos com alguma modificação pendente.
    pub fn dirty_count(&self) -> usize {
        self.files.len()
    }

    /// Resolve o status visual de um caminho relativo à raiz do repositório (`/`).
    pub fn status_for_rel_path(&self, rel: &str) -> Option<GitFileStatus> {
        let trimmed = rel.trim_matches('/');
        if trimmed.is_empty() {
            return None;
        }
        if let Some(&file_status) = self.files.get(trimmed) {
            return Some(file_status);
        }
        if let Some(&dir_status) = self.folder_statuses.get(trimmed) {
            return Some(dir_status);
        }
        if is_under_set(&self.untracked_dirs, trimmed) {
            return Some(GitFileStatus::Untracked);
        }
        if is_under_set(&self.ignored_roots, trimmed) {
            return Some(GitFileStatus::Ignored);
        }
        None
    }

    /// Resolve o status visual de um caminho absoluto dentro de `repo_root`.
    pub fn status_for_abs_path(&self, repo_root: &Path, abs_path: &Path) -> Option<GitFileStatus> {
        let rel = abs_path.strip_prefix(repo_root).ok()?;
        let normalized = normalize_rel_path(rel);
        self.status_for_rel_path(&normalized)
    }
}

fn is_under_set(roots: &BTreeSet<String>, rel: &str) -> bool {
    if roots.contains(rel) {
        return true;
    }
    for root in roots {
        if let Some(rest) = rel.strip_prefix(root.as_str()) {
            if rest.starts_with('/') {
                return true;
            }
        }
    }
    false
}

/// Normaliza um caminho relativo para usar sempre `/` como separador.
pub fn normalize_rel_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

/// Metadados resumidos de um commit retornados por `git log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitCommitInfo {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date_iso: String,
    pub date_relative: String,
    pub subject: String,
    pub refs: Vec<String>,
}

/// Tipo de uma linha dentro de um hunk de unified diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

/// Linha individual de um [`DiffHunk`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub text: String,
}

/// Hunk de um unified diff (`@@ -a,b +c,d @@`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// Classificação geral do diff de um arquivo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDiffKind {
    Modified,
    Added,
    Deleted,
    Binary,
    Unchanged,
}

/// Resultado estruturado do diff de um arquivo (staged ou working tree).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub staged: bool,
    pub kind: FileDiffKind,
    pub hunks: Vec<DiffHunk>,
}

impl FileDiff {
    pub fn unchanged(path: impl Into<String>, staged: bool) -> Self {
        Self {
            path: path.into(),
            staged,
            kind: FileDiffKind::Unchanged,
            hunks: Vec::new(),
        }
    }

    pub fn binary(path: impl Into<String>, staged: bool) -> Self {
        Self {
            path: path.into(),
            staged,
            kind: FileDiffKind::Binary,
            hunks: Vec::new(),
        }
    }
}
