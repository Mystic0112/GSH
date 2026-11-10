//! Modelo de workspace do domínio (`Project` e `WorkspaceKind` em
//! `project.dart` do app Flutter).
//!
//! Puro, sem I/O e sem UI: deriva nome e inicial a partir do caminho da pasta
//! e mantém a lista em memória com seleção ativa.

use std::path::{Path, PathBuf};

use crate::tabs::basename;

/// Natureza de um workspace no rail (`WorkspaceKind` em `project.dart`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceKind {
    /// Workspace ancorado numa pasta local do sistema de arquivos.
    Project,
    /// Workspace sintético sem pasta local obrigatória (ex.: terminal de sistema).
    SystemTerminal,
}

/// Um workspace exibido no rail lateral (`Project` em `project.dart`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: Option<PathBuf>,
    pub git_branch: Option<String>,
    pub kind: WorkspaceKind,
}

impl Workspace {
    /// Cria um workspace sintético (sem pasta de projeto associada).
    pub fn system(
        id: impl Into<String>,
        name: impl Into<String>,
        git_branch: Option<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            path: None,
            git_branch,
            kind: WorkspaceKind::SystemTerminal,
        }
    }

    /// Cria um workspace de projeto a partir do caminho da pasta (`nome = basename(pasta)`).
    pub fn from_path(id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        let path_buf = path.into();
        let name = folder_display_name(&path_buf);
        Self {
            id: id.into(),
            name,
            path: Some(path_buf),
            git_branch: None,
            kind: WorkspaceKind::Project,
        }
    }

    /// Define ou atualiza a branch git associada ao workspace.
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Define o caminho da pasta associado ao workspace.
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Inicial em maiúsculo para o avatar do rail (`Project.initial` no Flutter).
    pub fn initial(&self) -> String {
        self.name
            .trim()
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string())
    }
}

/// Extrai o nome de exibição da pasta (último segmento não vazio do caminho).
pub fn folder_display_name(path: &Path) -> String {
    if let Some(os_name) = path.file_name() {
        let name = os_name.to_string_lossy();
        if !name.is_empty() {
            return name.into_owned();
        }
    }
    let raw = path.to_string_lossy();
    let base = basename(&raw);
    if base.is_empty() {
        raw.into_owned()
    } else {
        base
    }
}

/// Coleção pura de workspaces com controle do item selecionado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceList {
    items: Vec<Workspace>,
    selected: usize,
    next_id: u64,
}

impl WorkspaceList {
    /// Cria uma lista de workspaces a partir dos itens iniciais, selecionando o primeiro.
    pub fn new(items: Vec<Workspace>) -> Self {
        let next_id = u64::try_from(items.len()).unwrap_or(0).saturating_add(1);
        Self {
            items,
            selected: 0,
            next_id,
        }
    }

    /// Fatia com todos os workspaces registrados.
    pub fn items(&self) -> &[Workspace] {
        &self.items
    }

    /// Índice do workspace atualmente selecionado.
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Referência ao workspace ativo (com fallback seguro para o primeiro item).
    pub fn active(&self) -> Option<&Workspace> {
        self.items.get(self.selected).or_else(|| self.items.first())
    }

    /// Seleciona o workspace pelo índice, caso esteja dentro dos limites.
    pub fn select(&mut self, index: usize) -> Option<&Workspace> {
        if index < self.items.len() {
            self.selected = index;
            self.items.get(index)
        } else {
            None
        }
    }

    /// Abre uma pasta como workspace: se já existir um workspace com o mesmo
    /// caminho, apenas o seleciona; senão cria um novo, adiciona ao final e o
    /// seleciona, retornando o índice selecionado.
    pub fn open_folder(&mut self, path: impl Into<PathBuf>) -> usize {
        let path_buf = path.into();
        if let Some(existing_idx) = self
            .items
            .iter()
            .position(|ws| ws.path.as_ref() == Some(&path_buf))
        {
            self.selected = existing_idx;
            return existing_idx;
        }

        let id = format!("ws-{}", self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        let ws = Workspace::from_path(id, path_buf);
        self.items.push(ws);
        let new_idx = self.items.len().saturating_sub(1);
        self.selected = new_idx;
        new_idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_from_path_extrai_nome_e_inicial() {
        let ws = Workspace::from_path("ws-1", "/home/dev/projetos/meu-app");
        assert_eq!(ws.name, "meu-app");
        assert_eq!(ws.initial(), "M");
        assert_eq!(ws.path, Some(PathBuf::from("/home/dev/projetos/meu-app")));
        assert_eq!(ws.kind, WorkspaceKind::Project);
    }

    #[test]
    fn workspace_list_abre_seleciona_e_evita_duplicatas_de_caminho() {
        let initial = vec![Workspace::system("gsh", "GSH", Some("main".into()))];
        let mut list = WorkspaceList::new(initial);
        assert_eq!(list.selected_index(), 0);
        assert_eq!(list.active().map(|w| w.name.as_str()), Some("GSH"));

        let idx = list.open_folder("/tmp/projeto-alfa");
        assert_eq!(idx, 1);
        assert_eq!(list.selected_index(), 1);
        assert_eq!(list.active().map(|w| w.name.as_str()), Some("projeto-alfa"));

        // Reabrir a mesma pasta apenas seleciona o workspace existente
        let _ = list.select(0);
        assert_eq!(list.selected_index(), 0);
        let reused_idx = list.open_folder("/tmp/projeto-alfa");
        assert_eq!(reused_idx, 1);
        assert_eq!(list.items().len(), 2);

        // Índice fora dos limites é ignorado
        assert!(list.select(99).is_none());
        assert_eq!(list.selected_index(), 1);
    }
}
