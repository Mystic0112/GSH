use super::model::{reserve_ids, Result};
use super::{Workspace, WorkspaceError};
use crate::panes::{reorder_tabs, PaneLeaf, PaneMove, PaneNode, SplitDirection, TabId};
use crate::tabs::{NewTab, Tab, TabKind, EMPTY_TAB_TITLE};

const MIN_FRACTION: f64 = 0.16;
const MAX_FRACTION: f64 = 0.84;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneCreated {
    pub pane: String,
    pub tab: TabId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabClosed {
    pub tab: Tab,
    pub pane_removed: bool,
    pub placeholder: Option<TabId>,
}

/// Um workspace com sua árvore de painéis, tabela de abas e foco.
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceState {
    pub(super) metadata: Workspace,
    tree: PaneNode,
    tabs: Vec<Tab>,
    focused: String,
}

impl WorkspaceState {
    pub(super) fn new(metadata: Workspace, next: &mut u64) -> Result<Self> {
        let id = reserve_ids(next, 2)?;
        let tab = NewTab::empty().into_tab(id);
        let focused = format!("pane{}", id + 1);
        Ok(Self {
            metadata,
            tree: PaneNode::leaf(&focused, vec![tab.id.clone()], &tab.id),
            tabs: vec![tab],
            focused,
        })
    }

    pub fn metadata(&self) -> &Workspace {
        &self.metadata
    }

    pub fn tree(&self) -> &PaneNode {
        &self.tree
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn tab(&self, id: &str) -> Option<&Tab> {
        self.tabs.iter().find(|tab| tab.id == id)
    }

    pub fn focused_pane_id(&self) -> &str {
        &self.focused
    }

    pub fn active_tab_id(&self) -> Option<&str> {
        self.tree
            .find_leaf(&self.focused)
            .map(|leaf| leaf.active.as_str())
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_tab_id().and_then(|id| self.tab(id))
    }

    fn leaf(&self, id: &str) -> Result<&PaneLeaf> {
        self.tree
            .find_leaf(id)
            .ok_or_else(|| WorkspaceError::PaneNotFound(id.to_owned()))
    }

    fn tab_index(&self, id: &str) -> Result<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.id == id)
            .ok_or_else(|| WorkspaceError::TabNotFound(id.to_owned()))
    }

    fn tab_leaf(&self, pane: &str, tab: &str) -> Result<PaneLeaf> {
        let leaf = self.leaf(pane)?;
        self.tab_index(tab)?;
        if !leaf.tabs.iter().any(|id| id == tab) {
            return Err(WorkspaceError::TabNotInPane {
                tab: tab.to_owned(),
                pane: pane.to_owned(),
            });
        }
        Ok(leaf.clone())
    }

    fn update_leaf(&mut self, leaf: PaneLeaf) {
        self.tree = self.tree.update_leaf(&leaf.id, |_| leaf.clone());
    }

    fn remove_leaf(&mut self, id: &str) {
        self.tree = self.tree.remove_leaf(id);
        if self.focused == id {
            if let Some(first) = self.tree.leaves().first() {
                self.focused.clone_from(&first.id);
            }
        }
    }
}

/// Operações de um workspace, com alocação de IDs compartilhada pela coleção.
pub struct WorkspaceEditor<'a> {
    pub(super) state: &'a mut WorkspaceState,
    pub(super) next: &'a mut u64,
}

impl WorkspaceEditor<'_> {
    pub fn workspace(&self) -> &WorkspaceState {
        self.state
    }

    pub fn focus_pane(&mut self, pane: &str) -> Result<()> {
        self.state.leaf(pane)?;
        self.state.focused = pane.to_owned();
        Ok(())
    }

    pub fn select_tab(&mut self, pane: &str, tab: &str) -> Result<()> {
        let mut leaf = self.state.tab_leaf(pane, tab)?;
        leaf.active = tab.to_owned();
        self.state.update_leaf(leaf);
        self.state.focused = pane.to_owned();
        Ok(())
    }

    pub fn focus_tab(&mut self, tab: &str) -> Result<String> {
        let pane = self
            .state
            .tree
            .leaves()
            .into_iter()
            .find(|leaf| leaf.tabs.iter().any(|id| id == tab))
            .map(|leaf| leaf.id.clone())
            .ok_or_else(|| WorkspaceError::TabNotFound(tab.to_owned()))?;
        self.select_tab(&pane, tab)?;
        Ok(pane)
    }

    pub fn focus_toward(&mut self, direction: PaneMove) -> Result<String> {
        let pane = self.state.focused_pane_id();
        let neighbor = self
            .state
            .tree
            .neighbor_leaf(pane, direction)
            .ok_or_else(|| WorkspaceError::NoNeighbor {
                pane: pane.to_owned(),
                direction,
            })?;
        self.state.focused.clone_from(&neighbor);
        Ok(neighbor)
    }

    pub fn new_empty_tab(&mut self, pane: &str) -> Result<TabId> {
        self.open_tab(pane, NewTab::empty())
    }

    /// Um terminal substitui o placeholder ativo. Os demais tipos só substituem
    /// um placeholder quando ele é a única aba do painel.
    pub fn open_tab(&mut self, pane: &str, spec: NewTab) -> Result<TabId> {
        let mut leaf = self.state.leaf(pane)?.clone();
        let replaces_empty = match spec.kind {
            TabKind::Empty => false,
            TabKind::Terminal => self.is_empty_tab(&leaf.active),
            _ => leaf.tabs.len() == 1 && self.is_empty_tab(&leaf.active),
        };
        if replaces_empty {
            let empty = leaf.active.clone();
            return self.fill_empty(pane, &empty, spec);
        }
        let id = self.reserve_for_tab(&spec, 1)?;
        let tab = spec.into_tab(id);
        leaf.tabs.push(tab.id.clone());
        leaf.active.clone_from(&tab.id);
        self.state.update_leaf(leaf);
        self.state.focused = pane.to_owned();
        let id = tab.id.clone();
        self.state.tabs.push(tab);
        Ok(id)
    }

    pub fn fill_empty(&mut self, pane: &str, tab: &str, spec: NewTab) -> Result<TabId> {
        let mut leaf = self.state.tab_leaf(pane, tab)?;
        if !self.is_empty_tab(tab) {
            return Err(WorkspaceError::NotEmptyTab(tab.to_owned()));
        }
        leaf.active = tab.to_owned();
        let id = self.replace_in_leaf(leaf, tab, spec)?;
        self.state.focused = pane.to_owned();
        Ok(id)
    }

    /// Troca a aba no mesmo slot. Foco e seleção são preservados.
    pub fn replace_tab(&mut self, pane: &str, tab: &str, spec: NewTab) -> Result<TabId> {
        let leaf = self.state.tab_leaf(pane, tab)?;
        self.replace_in_leaf(leaf, tab, spec)
    }

    fn replace_in_leaf(&mut self, mut leaf: PaneLeaf, old: &str, spec: NewTab) -> Result<TabId> {
        let old_index = self.state.tab_index(old)?;
        let id = self.reserve_for_tab(&spec, 1)?;
        let tab = spec.into_tab(id);
        if let Some(slot) = leaf.tabs.iter_mut().find(|id| *id == old) {
            slot.clone_from(&tab.id);
        }
        if leaf.active == old {
            leaf.active.clone_from(&tab.id);
        }
        self.state.update_leaf(leaf);
        self.state.tabs.remove(old_index);
        let id = tab.id.clone();
        self.state.tabs.push(tab);
        Ok(id)
    }

    pub fn close_tab(&mut self, pane: &str, tab: &str) -> Result<TabClosed> {
        let leaf = self.state.tab_leaf(pane, tab)?;
        let index = self.state.tab_index(tab)?;
        let last_in_pane = leaf.tabs.len() == 1;
        let last_pane = matches!(self.state.tree, PaneNode::Leaf(_));
        let placeholder = if last_in_pane && last_pane {
            Some(self.reset_leaf(pane)?)
        } else {
            self.detach_tab(leaf, tab);
            None
        };
        Ok(TabClosed {
            tab: self.state.tabs.remove(index),
            pane_removed: last_in_pane && !last_pane,
            placeholder,
        })
    }

    pub fn close_pane(&mut self, pane: &str) -> Result<Vec<Tab>> {
        let tabs = self.state.leaf(pane)?.tabs.clone();
        if matches!(self.state.tree, PaneNode::Leaf(_)) {
            self.reset_leaf(pane)?;
        } else {
            self.state.remove_leaf(pane);
        }
        Ok(tabs
            .into_iter()
            .filter_map(|id| {
                self.state
                    .tab_index(&id)
                    .ok()
                    .map(|index| self.state.tabs.remove(index))
            })
            .collect())
    }

    fn reset_leaf(&mut self, pane: &str) -> Result<TabId> {
        let tab = NewTab::empty().into_tab(reserve_ids(self.next, 1)?);
        let id = tab.id.clone();
        self.state.update_leaf(PaneLeaf {
            id: pane.to_owned(),
            tabs: vec![id.clone()],
            active: id.clone(),
        });
        self.state.tabs.push(tab);
        Ok(id)
    }

    pub fn split_pane(
        &mut self,
        pane: &str,
        direction: SplitDirection,
        spec: NewTab,
    ) -> Result<PaneCreated> {
        self.state.leaf(pane)?;
        let id = self.reserve_for_tab(&spec, 3)?;
        let tab = spec.into_tab(id);
        let new_pane = self.insert_split(pane, direction, false, &tab.id, id + 1);
        let created = PaneCreated {
            pane: new_pane,
            tab: tab.id.clone(),
        };
        self.state.tabs.push(tab);
        Ok(created)
    }

    /// `index` é um slot entre abas e é limitado ao tamanho da lista.
    pub fn move_tab(
        &mut self,
        source: &str,
        tab: &str,
        target: &str,
        index: Option<usize>,
    ) -> Result<()> {
        let source_leaf = self.state.tab_leaf(source, tab)?;
        let mut target_leaf = self.state.leaf(target)?.clone();
        if source == target {
            let Some(index) = index else {
                return Ok(());
            };
            let slot = isize::try_from(index).unwrap_or(isize::MAX);
            target_leaf.tabs = reorder_tabs(&target_leaf.tabs, tab, slot);
        } else {
            let len = target_leaf.tabs.len();
            target_leaf
                .tabs
                .insert(index.unwrap_or(len).min(len), tab.to_owned());
            self.detach_tab(source_leaf, tab);
        }
        target_leaf.active = tab.to_owned();
        self.state.update_leaf(target_leaf);
        self.state.focused = target.to_owned();
        Ok(())
    }

    pub fn reorder_tab(&mut self, pane: &str, tab: &str, index: usize) -> Result<()> {
        self.move_tab(pane, tab, pane, Some(index))
    }

    pub fn move_tab_to_new_split(
        &mut self,
        source: &str,
        tab: &str,
        target: &str,
        direction: SplitDirection,
        before: bool,
    ) -> Result<String> {
        let source_leaf = self.state.tab_leaf(source, tab)?;
        self.state.leaf(target)?;
        if source == target && source_leaf.tabs.len() == 1 {
            return Err(WorkspaceError::SplitOntoSelf(source.to_owned()));
        }
        let id = reserve_ids(self.next, 2)?;
        self.detach_tab(source_leaf, tab);
        Ok(self.insert_split(target, direction, before, tab, id))
    }

    fn insert_split(
        &mut self,
        target: &str,
        direction: SplitDirection,
        before: bool,
        tab: &str,
        id: u64,
    ) -> String {
        let pane = format!("pane{id}");
        let split = format!("sp{}", id + 1);
        let leaf = PaneLeaf {
            id: pane.clone(),
            tabs: vec![tab.to_owned()],
            active: tab.to_owned(),
        };
        self.state.tree = self
            .state
            .tree
            .split_leaf(target, direction, leaf, Some(&split), before);
        self.state.focused.clone_from(&pane);
        pane
    }

    fn detach_tab(&mut self, mut leaf: PaneLeaf, tab: &str) {
        if leaf.tabs.len() == 1 {
            self.state.remove_leaf(&leaf.id);
        } else if let Some(index) = leaf.tabs.iter().position(|id| id == tab) {
            leaf.tabs.remove(index);
            if leaf.active == tab {
                if let Some(active) = leaf.tabs.get(index.saturating_sub(1)) {
                    leaf.active.clone_from(active);
                }
            }
            self.state.update_leaf(leaf);
        }
    }

    pub fn resize_split(&mut self, split: &str, fraction: f64) -> Result<()> {
        if !fraction.is_finite() {
            return Err(WorkspaceError::InvalidFraction);
        }
        if self.state.tree.find_split(split).is_none() {
            return Err(WorkspaceError::SplitNotFound(split.to_owned()));
        }
        self.state.tree = self
            .state
            .tree
            .set_fraction(split, fraction.clamp(MIN_FRACTION, MAX_FRACTION));
        Ok(())
    }

    pub fn resize_split_by(&mut self, split: &str, delta: f64) -> Result<()> {
        if !delta.is_finite() {
            return Err(WorkspaceError::InvalidFraction);
        }
        let previous = self
            .state
            .tree
            .find_split(split)
            .ok_or_else(|| WorkspaceError::SplitNotFound(split.to_owned()))?
            .fraction;
        self.resize_split(split, previous + delta)
    }

    pub fn set_label(&mut self, tab: &str, label: &str) -> Result<()> {
        let label = label.trim();
        if label.is_empty() {
            return Err(WorkspaceError::EmptyLabel);
        }
        self.named_tab_mut(tab)?.label = Some(label.to_owned());
        Ok(())
    }

    pub fn clear_label(&mut self, tab: &str) -> Result<()> {
        self.named_tab_mut(tab)?.label = None;
        Ok(())
    }

    pub fn set_title(&mut self, tab: &str, title: impl Into<String>) -> Result<()> {
        self.named_tab_mut(tab)?.title = title.into();
        Ok(())
    }

    fn named_tab_mut(&mut self, id: &str) -> Result<&mut Tab> {
        let tab = self
            .state
            .tabs
            .iter_mut()
            .find(|tab| tab.id == id)
            .ok_or_else(|| WorkspaceError::TabNotFound(id.to_owned()))?;
        if tab.kind == TabKind::Empty {
            return Err(WorkspaceError::PlaceholderMetadata);
        }
        Ok(tab)
    }

    fn is_empty_tab(&self, id: &str) -> bool {
        self.state
            .tab(id)
            .is_some_and(|tab| tab.kind == TabKind::Empty)
    }

    fn reserve_for_tab(&mut self, spec: &NewTab, count: u64) -> Result<u64> {
        if spec.kind == TabKind::Empty {
            if spec.title != EMPTY_TAB_TITLE
                || spec.label.is_some()
                || !spec.working_directory.is_empty()
            {
                return Err(WorkspaceError::PlaceholderMetadata);
            }
        } else if spec
            .label
            .as_deref()
            .is_some_and(|label| label.trim().is_empty())
        {
            return Err(WorkspaceError::EmptyLabel);
        }
        reserve_ids(self.next, count)
    }
}
