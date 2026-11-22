use std::path::Path;

use serde_json::{json, Value};
use thiserror::Error;

use super::{Workspace, WorkspaceEditor, WorkspaceState};
use crate::panes::{PaneMove, TabId};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WorkspaceError {
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(String),
    #[error("pane not found: {0}")]
    PaneNotFound(String),
    #[error("split not found: {0}")]
    SplitNotFound(String),
    #[error("tab not found: {0}")]
    TabNotFound(TabId),
    #[error("tab {tab} is not in pane {pane}")]
    TabNotInPane { tab: TabId, pane: String },
    #[error("tab {0} is not an empty placeholder")]
    NotEmptyTab(TabId),
    #[error("workspace name is empty")]
    EmptyName,
    #[error("tab label is empty")]
    EmptyLabel,
    #[error("an empty placeholder must keep its default title and metadata")]
    PlaceholderMetadata,
    #[error("cannot split pane {0} onto itself: it holds only the moved tab")]
    SplitOntoSelf(String),
    #[error("there is no pane {direction:?} of {pane}")]
    NoNeighbor { pane: String, direction: PaneMove },
    #[error("split fraction and delta must be finite")]
    InvalidFraction,
    #[error("domain ID counter exhausted")]
    IdExhausted,
}

pub(super) type Result<T> = std::result::Result<T, WorkspaceError>;

pub(super) fn reserve_ids(next: &mut u64, count: u64) -> Result<u64> {
    let start = *next;
    *next = start
        .checked_add(count)
        .ok_or(WorkspaceError::IdExhausted)?;
    Ok(start)
}

/// Estado dos workspaces e contadores compartilhados de IDs das abas e painéis.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Workspaces {
    items: Vec<WorkspaceState>,
    selected: Option<String>,
    next_workspace: u64,
    next_layout: u64,
}

impl Workspaces {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(&self) -> &[WorkspaceState] {
        &self.items
    }

    pub fn workspace(&self, id: &str) -> Option<&WorkspaceState> {
        self.items
            .iter()
            .find(|workspace| workspace.metadata.id == id)
    }

    pub fn selected_id(&self) -> Option<&str> {
        self.selected.as_deref()
    }

    pub fn selected(&self) -> Option<&WorkspaceState> {
        self.selected_id().and_then(|id| self.workspace(id))
    }

    /// Reabre o workspace da mesma pasta ou cria um workspace com `New tab`.
    pub fn create_workspace(&mut self, name: &str, path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref();
        let name = name.trim();
        let has_path = !path.as_os_str().is_empty();
        if has_path {
            if let Some(existing) = self
                .items
                .iter()
                .find(|ws| ws.metadata.path.as_deref() == Some(path))
            {
                let id = existing.metadata.id.clone();
                self.selected = Some(id.clone());
                return Ok(id);
            }
        } else if name.is_empty() {
            return Err(WorkspaceError::EmptyName);
        }

        let mut next_workspace = self.next_workspace;
        let id = format!("ws{}", reserve_ids(&mut next_workspace, 1)?);
        let mut metadata = if has_path {
            Workspace::from_path(&id, path)
        } else {
            Workspace::system(&id, name, None)
        };
        if !name.is_empty() {
            metadata.name = name.to_owned();
        }
        self.items
            .push(WorkspaceState::new(metadata, &mut self.next_layout)?);
        self.next_workspace = next_workspace;
        self.selected = Some(id.clone());
        Ok(id)
    }

    pub fn rename_workspace(&mut self, id: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(WorkspaceError::EmptyName);
        }
        self.edit(id)?.state.metadata.name = name.to_owned();
        Ok(())
    }

    pub fn select_workspace(&mut self, id: &str) -> Result<()> {
        self.index_of(id)?;
        self.selected = Some(id.to_owned());
        Ok(())
    }

    pub fn remove_workspace(&mut self, id: &str) -> Result<WorkspaceState> {
        let index = self.index_of(id)?;
        let removed = self.items.remove(index);
        if self.selected_id() == Some(id) {
            self.selected = self.items.first().map(|item| item.metadata.id.clone());
        }
        Ok(removed)
    }

    /// Move `id` para antes ou depois do workspace `target`.
    pub fn reorder_workspace(&mut self, id: &str, target: &str, before: bool) -> Result<()> {
        let from = self.index_of(id)?;
        let to = self.index_of(target)?;
        if from != to {
            let target_slot = to + usize::from(!before);
            let item = self.items.remove(from);
            self.items
                .insert(target_slot - usize::from(from < to), item);
        }
        Ok(())
    }

    pub fn edit(&mut self, id: &str) -> Result<WorkspaceEditor<'_>> {
        let state = self
            .items
            .iter_mut()
            .find(|ws| ws.metadata.id == id)
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(id.to_owned()))?;
        Ok(WorkspaceEditor {
            state,
            next: &mut self.next_layout,
        })
    }

    /// Seleciona o workspace, a folha e a aba que corresponde ao ID global.
    pub fn reveal_tab(&mut self, tab: &str) -> Result<(String, String)> {
        let id = self
            .items
            .iter()
            .find(|ws| ws.tab(tab).is_some())
            .map(|ws| ws.metadata.id.clone())
            .ok_or_else(|| WorkspaceError::TabNotFound(tab.to_owned()))?;
        let pane = self.edit(&id)?.focus_tab(tab)?;
        self.selected = Some(id.clone());
        Ok((id, pane))
    }

    /// Entradas no formato de `tabs::format_tab_list` consumido pela CLI.
    pub fn tab_list(&self) -> Vec<Value> {
        let focused = self.selected().and_then(WorkspaceState::active_tab_id);
        self.items
            .iter()
            .flat_map(|ws| {
                ws.tabs().iter().map(move |tab| json!({
                "id": tab.id,
                "kind": tab.kind.cli_kind(),
                "title": tab.title,
                "label": tab.label,
                "workspaceId": ws.metadata.id,
                "workspacePath": ws.metadata.path.as_ref().map(|path| path.to_string_lossy()),
                "focused": Some(tab.id.as_str()) == focused,
            }))
            })
            .collect()
    }

    fn index_of(&self, id: &str) -> Result<usize> {
        self.items
            .iter()
            .position(|ws| ws.metadata.id == id)
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(id.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_exhaustion_does_not_consume_ids_or_change_state() {
        let mut state = Workspaces {
            next_workspace: u64::MAX,
            ..Workspaces::new()
        };
        let before = state.clone();
        assert_eq!(
            state.create_workspace("test", ""),
            Err(WorkspaceError::IdExhausted)
        );
        assert_eq!(state, before);

        let mut state = Workspaces {
            next_layout: u64::MAX - 1,
            ..Workspaces::new()
        };
        let before = state.clone();
        assert_eq!(
            state.create_workspace("test", ""),
            Err(WorkspaceError::IdExhausted)
        );
        assert_eq!(state, before);
    }
}
