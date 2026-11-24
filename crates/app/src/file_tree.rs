//! Lógica da árvore de arquivos do workspace (`file_system_reader_impl.dart`
//! do app Flutter): filtragem de diretórios de VCS (`.git`, `.hg`, `.svn`),
//! ordenação (pastas primeiro, depois arquivos, ordem alfabética), expansão
//! lazy por diretório e erros de leitura tipados.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::labels;

/// Pastas de controle de versão ocultadas em qualquer nível da árvore
/// (`_hiddenDirs` em `file_system_reader_impl.dart`).
pub const HIDDEN_VCS_DIRS: [&str; 3] = [".git", ".hg", ".svn"];

/// Nó individual de arquivo ou diretório na árvore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
}

/// Erro tipado ao listar um diretório (nunca `panic` e nunca string literal solta).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeReadError {
    NotFound,
    PermissionDenied,
    Io(String),
}

impl TreeReadError {
    pub fn from_io(err: &io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::NotFound,
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ => Self::Io(err.to_string()),
        }
    }

    /// Converte o erro tipado para mensagem de UI usando constantes de [`labels`].
    pub fn ui_message(&self) -> String {
        match self {
            Self::NotFound => labels::FILES_ERR_NOT_FOUND.to_string(),
            Self::PermissionDenied => labels::FILES_ERR_PERMISSION.to_string(),
            Self::Io(detail) => format!("{}{detail}", labels::FILES_ERR_IO_PREFIX),
        }
    }
}

/// Ação disparada quando um arquivo é clicado na árvore (`onTap` em
/// `file_tree_panel.dart` do Flutter): seleciona a linha e abre/ativa a aba
/// `File` no pane central.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewerHook {
    /// Arquivo selecionado na árvore para abrir ou ativar como aba no viewer.
    OpenFile(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileClipboard {
    pub path: PathBuf,
    pub cut: bool,
}

/// Linha visível achatada pronta para renderização no painel de arquivos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeRow {
    Entry {
        entry: FileEntry,
        depth: usize,
        expanded: bool,
        selected: bool,
    },
    Error {
        depth: usize,
        error: TreeReadError,
    },
}

/// Regra pura de filtragem (`file_system_reader_impl.dart`):
/// oculta `.git`, `.hg`, `.svn` quando diretórios e `.DS_Store`, mantendo
/// dotfiles úteis (`.env`, `.claude`, `.gitignore`, etc.).
pub fn should_ignore_entry(name: &str, is_dir: bool) -> bool {
    if name.is_empty() || name == ".DS_Store" {
        return true;
    }
    is_dir && HIDDEN_VCS_DIRS.contains(&name)
}

/// Ordena entradas in-place: diretórios primeiro, depois arquivos, em ordem
/// alfabética case-insensitive (`byName` em `file_system_reader_impl.dart`).
pub fn sort_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// Lê um único nível de `dir_path` (sem recursão), filtrando e ordenando.
pub fn read_dir_sorted(dir_path: &Path) -> Result<Vec<FileEntry>, TreeReadError> {
    let read_dir = fs::read_dir(dir_path).map_err(|e| TreeReadError::from_io(&e))?;
    let mut entries = Vec::new();

    for item in read_dir {
        let Ok(dir_entry) = item else {
            continue;
        };
        let Ok(file_type) = dir_entry.file_type() else {
            continue;
        };
        // Não segue symlinks como diretórios para evitar ciclos (`followLinks: false`).
        let is_dir = file_type.is_dir();
        let name = dir_entry.file_name().to_string_lossy().into_owned();
        if should_ignore_entry(&name, is_dir) {
            continue;
        }
        entries.push(FileEntry {
            name,
            path: dir_entry.path(),
            is_dir,
        });
    }

    sort_entries(&mut entries);
    Ok(entries)
}

/// Estado da árvore de arquivos com carregamento lazy por diretório.
#[derive(Debug, Clone, Default)]
pub struct FileTree {
    root: Option<PathBuf>,
    expanded: BTreeSet<PathBuf>,
    cache: BTreeMap<PathBuf, Result<Vec<FileEntry>, TreeReadError>>,
    selected_file: Option<PathBuf>,
    selected_path: Option<PathBuf>,
    context_target: Option<PathBuf>,
    clipboard: Option<FileClipboard>,
}

impl FileTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Cria a árvore para `root`, lendo apenas o nível raiz (subpastas são lazy).
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        let mut tree = Self::new();
        tree.set_root(Some(root.into()));
        tree
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn selected_file(&self) -> Option<&Path> {
        self.selected_file.as_deref()
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.selected_path.as_deref()
    }

    pub fn clear_selection(&mut self) {
        self.selected_file = None;
        self.selected_path = None;
    }

    pub fn retarget_path(&mut self, from: &Path, to: &Path) {
        fn retarget(path: &Path, from: &Path, to: &Path) -> Option<PathBuf> {
            let suffix = path.strip_prefix(from).ok()?;
            Some(to.join(suffix))
        }
        if let Some(next) = self
            .selected_path
            .as_deref()
            .and_then(|path| retarget(path, from, to))
        {
            self.selected_path = Some(next);
        }
        if let Some(next) = self
            .selected_file
            .as_deref()
            .and_then(|path| retarget(path, from, to))
        {
            self.selected_file = Some(next);
        }
        self.context_target = None;
    }

    /// Selected directory, selected file parent, or workspace root for header creation.
    pub fn create_target_dir(&self) -> Option<&Path> {
        let root = self.root.as_deref()?;
        let Some(selected) = self.selected_path.as_deref() else {
            return Some(root);
        };
        if selected.is_dir() {
            Some(selected)
        } else {
            selected.parent().or(Some(root))
        }
    }

    pub fn context_target(&self) -> Option<&Path> {
        self.context_target.as_deref()
    }

    /// Opens the context menu for a tree item. A target outside the workspace is ignored.
    pub fn show_context_menu(&mut self, target: PathBuf) {
        if self
            .root
            .as_deref()
            .is_some_and(|root| target.starts_with(root))
        {
            self.context_target = Some(target);
        }
    }

    pub fn close_context_menu(&mut self) {
        self.context_target = None;
    }

    pub fn set_clipboard(&mut self, path: PathBuf, cut: bool) {
        if self
            .root
            .as_deref()
            .is_some_and(|root| path.starts_with(root))
        {
            self.clipboard = Some(FileClipboard { path, cut });
        }
        self.close_context_menu();
    }

    pub fn clipboard(&self) -> Option<&FileClipboard> {
        self.clipboard.as_ref()
    }

    pub fn clear_clipboard(&mut self) {
        self.clipboard = None;
    }

    pub fn can_paste(&self, target: &Path) -> bool {
        let Some(root) = self.root.as_deref() else {
            return false;
        };
        let Some(clipboard) = self.clipboard.as_ref() else {
            return false;
        };
        target.starts_with(root)
            && target.is_dir()
            && clipboard.path.starts_with(root)
            && clipboard.path.exists()
    }

    /// Expands every directory from the workspace root through `target_dir`.
    /// This makes an inline create field visible even when its parent was collapsed.
    pub fn reveal_dir(&mut self, target_dir: &Path) -> bool {
        let Some(root) = self.root.clone() else {
            return false;
        };
        if !target_dir.starts_with(&root) || !target_dir.is_dir() {
            return false;
        }
        let Ok(relative) = target_dir.strip_prefix(&root) else {
            return false;
        };
        let mut current = root.clone();
        let components = relative.components();
        for component in components {
            current.push(component);
            self.expanded.insert(current.clone());
            self.cache
                .entry(current.clone())
                .or_insert_with(|| read_dir_sorted(&current));
        }
        true
    }

    pub fn is_expanded(&self, dir: &Path) -> bool {
        self.expanded.contains(dir)
    }

    pub fn is_dir_cached(&self, dir: &Path) -> bool {
        self.cache.contains_key(dir)
    }

    pub fn is_expanded_empty_dir(&self, dir: &Path) -> bool {
        matches!(self.cache.get(dir), Some(Ok(entries)) if entries.is_empty())
    }

    /// Troca a pasta raiz ativa e lê somente o primeiro nível.
    pub fn set_root(&mut self, next_root: Option<PathBuf>) {
        if self.root == next_root {
            return;
        }
        self.root = next_root;
        self.expanded.clear();
        self.cache.clear();
        self.selected_file = None;
        self.selected_path = None;
        self.context_target = None;
        self.clipboard = None;
        if let Some(root) = self.root.clone() {
            let loaded = read_dir_sorted(&root);
            self.cache.insert(root, loaded);
        }
    }

    /// Expande ou recolhe `dir`. Só lê `dir` do disco no momento em que é expandido.
    pub fn toggle_dir(&mut self, dir: &Path) {
        self.selected_path = Some(dir.to_path_buf());
        if self.expanded.contains(dir) {
            self.expanded.remove(dir);
        } else {
            let owned = dir.to_path_buf();
            self.expanded.insert(owned.clone());
            self.cache
                .entry(owned)
                .or_insert_with(|| read_dir_sorted(dir));
        }
    }

    /// Seleciona um arquivo e devolve o [`ViewerHook`] para abrir/ativar a aba de viewer.
    pub fn select_file(&mut self, file_path: PathBuf) -> ViewerHook {
        self.selected_file = Some(file_path.clone());
        self.selected_path = Some(file_path.clone());
        ViewerHook::OpenFile(file_path)
    }

    /// Recolhe todas as subpastas abertas.
    pub fn collapse_all(&mut self) {
        self.expanded.clear();
    }

    /// Recarrega a raiz e apenas os diretórios atualmente expandidos.
    pub fn refresh(&mut self) {
        self.cache.clear();
        if let Some(root) = self.root.clone() {
            let loaded = read_dir_sorted(&root);
            self.cache.insert(root, loaded);
        }
        let expanded_dirs: Vec<PathBuf> = self.expanded.iter().cloned().collect();
        for dir in expanded_dirs {
            let loaded = read_dir_sorted(&dir);
            self.cache.insert(dir, loaded);
        }
    }

    /// Achata a árvore visível (respeitando diretórios expandidos) para a UI.
    pub fn visible_rows(&self) -> Vec<TreeRow> {
        let Some(root) = &self.root else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        self.collect_rows(root, 0, &mut rows);
        rows
    }

    fn collect_rows(&self, dir: &Path, depth: usize, out: &mut Vec<TreeRow>) {
        let Some(state) = self.cache.get(dir) else {
            return;
        };
        match state {
            Ok(entries) => {
                for entry in entries {
                    let expanded = entry.is_dir && self.expanded.contains(&entry.path);
                    let selected = self.selected_path.as_deref() == Some(entry.path.as_path());
                    out.push(TreeRow::Entry {
                        entry: entry.clone(),
                        depth,
                        expanded,
                        selected,
                    });
                    if expanded {
                        self.collect_rows(&entry.path, depth.saturating_add(1), out);
                    }
                }
            }
            Err(err) => {
                out.push(TreeRow::Error {
                    depth,
                    error: err.clone(),
                });
            }
        }
    }
}

#[cfg(test)]
#[path = "file_tree_tests.rs"]
mod tests;
