//! Workspaces, abas e as transições entre eles (porta da lógica de
//! `gsh_viewmodel.dart` sobre `pane_node.dart`), sem I/O e sem UI.
//!
//! [`Workspaces`] é o estado inteiro: lista ordenada de [`Workspace`], o
//! selecionado e o contador de ids. Mutações de um workspace passam por
//! [`Workspaces::edit`], que empresta o workspace junto com o contador (os ids de
//! abas e folhas são únicos entre **todos** os workspaces, como no Flutter).
//!
//! Todo o estado é dado: processos, PTY, viewers e arquivos são dos adaptadores,
//! que referenciam a aba pelo [`TabId`].

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ids::{numeric_suffix, IdGen, PaneId, TabId, WorkspaceId};
use crate::pane::{
    reorder_tabs, Leaf, PaneMove, PaneNode, SplitDir, DEFAULT_FRAC, MAX_FRAC, MIN_FRAC,
};
use crate::tabs::basename;

/// Título do placeholder vazio (`EmptyTab.title` no Flutter). Aparece na saída de
/// `gsh list-tabs`, por isso vive no domínio.
pub const EMPTY_TAB_TITLE: &str = "New tab";

/// Prefixo dos ids de workspace gerados aqui.
const WORKSPACE_PREFIX: &str = "ws";

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum WorkspaceError {
    #[error("workspace not found: {0}")]
    WorkspaceNotFound(WorkspaceId),
    #[error("pane not found: {0}")]
    PaneNotFound(PaneId),
    #[error("tab not found: {0}")]
    TabNotFound(TabId),
    #[error("tab {tab} is not in pane {pane}")]
    TabNotInPane { tab: TabId, pane: PaneId },
    #[error("tab {0} is not an empty placeholder")]
    NotEmptyTab(TabId),
    #[error("workspace name is empty")]
    EmptyName,
    #[error("cannot split pane {0} onto itself: it holds only the moved tab")]
    SplitOntoSelf(PaneId),
    #[error("split fraction must be finite, got {0}")]
    InvalidFraction(f64),
}

type Result<T> = std::result::Result<T, WorkspaceError>;

/// Tipos de aba que existem no Flutter (uma classe de sessão cada). O nome no
/// JSON é o `type` do descritor persistido de lá.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabKind {
    /// Placeholder de pane sem nada aberto (mostra o seletor do "+").
    Empty,
    Terminal,
    /// Neovim embutido (`NeovimSession`, um terminal por baixo).
    #[serde(rename = "file_engine")]
    Neovim,
    /// Viewer/editor de arquivo (`FileViewerSession`).
    #[serde(rename = "viewer")]
    File,
    Diff,
    TaskOutput,
    TelemetryCase,
    Redis,
    Mongo,
    Notebook,
    AgentGraph,
    Farm,
    Harness,
    Chat,
    Browser,
}

impl TabKind {
    /// Prefixo do id da aba (o `_nid('t')`, `_nid('v')`... do Flutter).
    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Empty => "a",
            Self::Terminal | Self::TaskOutput => "t",
            Self::Neovim => "n",
            Self::TelemetryCase => "y",
            Self::Diff => "d",
            _ => "v",
        }
    }

    /// Valor de `kind` na saída de `gsh list-tabs` (`_paneKind` do Flutter).
    pub fn cli_kind(self) -> &'static str {
        match self {
            Self::Terminal | Self::Neovim => "terminal",
            Self::File => "file",
            Self::TaskOutput => "task",
            Self::Redis => "redis",
            Self::Mongo => "mongo",
            Self::Notebook => "notebook",
            Self::AgentGraph => "agents",
            Self::Farm => "farm",
            Self::Chat => "chat",
            _ => "other",
        }
    }
}

/// Uma aba. `title` é o título dinâmico (OSC, nome do arquivo...); `label` é o
/// rótulo manual, que trava o nome exibido e é o que a orquestração endereça.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tab {
    pub id: TabId,
    pub kind: TabKind,
    pub title: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub working_directory: String,
}

impl Tab {
    /// Nome exibido: o rótulo manual vence o título dinâmico.
    pub fn display_title(&self) -> &str {
        self.label.as_deref().unwrap_or(&self.title)
    }

    pub fn title_locked(&self) -> bool {
        self.label.is_some()
    }
}

/// O que se pede para criar uma aba; o id nasce no estado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewTab {
    pub kind: TabKind,
    pub title: String,
    pub working_directory: String,
    pub label: Option<String>,
}

impl NewTab {
    pub fn new(kind: TabKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            working_directory: String::new(),
            label: None,
        }
    }

    pub fn empty() -> Self {
        Self::new(TabKind::Empty, EMPTY_TAB_TITLE)
    }

    pub fn with_working_directory(mut self, dir: impl Into<String>) -> Self {
        self.working_directory = dir.into();
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

/// Rótulo válido: aparado e não vazio.
fn clean_label(label: &str) -> Option<String> {
    let t = label.trim();
    (!t.is_empty()).then(|| t.to_string())
}

fn build_tab(ids: &mut IdGen, spec: NewTab) -> Tab {
    Tab {
        id: TabId::new(ids.next(spec.kind.id_prefix())),
        kind: spec.kind,
        title: spec.title,
        label: spec.label.as_deref().and_then(clean_label),
        working_directory: spec.working_directory,
    }
}

/// Aba ativa depois de `removed` sair de `tabs`: mantém a ativa se não era a
/// removida; senão a anterior (ou a primeira).
fn active_after(tabs: &[TabId], active: &TabId, removed: &TabId, remaining: &[TabId]) -> TabId {
    if active != removed {
        return active.clone();
    }
    let idx = tabs.iter().position(|t| t == removed).unwrap_or(0);
    remaining
        .get(idx.saturating_sub(1))
        .or_else(|| remaining.first())
        .cloned()
        .unwrap_or_else(|| active.clone())
}

fn replace_in_leaf(leaf: &mut Leaf, old: &TabId, new: &TabId) {
    for t in &mut leaf.tabs {
        if t == old {
            *t = new.clone();
        }
    }
}

/// Resultado de [`WorkspaceEditor::close_tab`].
#[derive(Debug, Clone, PartialEq)]
pub struct TabClosed {
    /// A aba removida (o adaptador encerra o processo/sessão dela).
    pub tab: Tab,
    /// A folha ficou sem abas e foi removida (o irmão expandiu).
    pub pane_removed: bool,
    /// A folha era a última do workspace: ganhou este placeholder vazio.
    pub placeholder: Option<TabId>,
}

/// Resultado das operações que criam uma folha nova.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneCreated {
    pub pane: PaneId,
    pub tab: TabId,
}

/// Um workspace: árvore de panes, abas e foco.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    id: WorkspaceId,
    name: String,
    /// Pasta raiz (vazia = sem pasta).
    #[serde(default)]
    path: String,
    tree: PaneNode,
    /// Folha focada; ausente/inválida cai na primeira folha.
    #[serde(default)]
    focused: Option<PaneId>,
    /// Abas em ordem de criação (a árvore guarda só os ids).
    #[serde(default)]
    tabs: Vec<Tab>,
}

impl Workspace {
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn tree(&self) -> &PaneNode {
        &self.tree
    }

    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    pub fn tab(&self, id: &TabId) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == *id)
    }

    /// Folha que contém a aba.
    pub fn leaf_of_tab(&self, id: &TabId) -> Option<&Leaf> {
        self.tree.leaf_of_tab(id)
    }

    /// Folha focada (a primeira se o foco guardado não existe mais).
    pub fn focused_leaf(&self) -> Option<&Leaf> {
        self.focused
            .as_ref()
            .and_then(|id| self.tree.find_leaf(id))
            .or_else(|| self.tree.leaves().into_iter().next())
    }

    pub fn focused_pane_id(&self) -> Option<&PaneId> {
        self.focused_leaf().map(|l| &l.id)
    }

    /// Aba ativa da folha focada (o que `--focused` da CLI mira).
    pub fn active_tab_id(&self) -> Option<&TabId> {
        self.focused_leaf().map(|l| &l.active)
    }

    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_tab_id().and_then(|id| self.tab(id))
    }

    fn leaf(&self, pane: &PaneId) -> Result<&Leaf> {
        self.tree
            .find_leaf(pane)
            .ok_or_else(|| WorkspaceError::PaneNotFound(pane.clone()))
    }

    fn leaf_mut(&mut self, pane: &PaneId) -> Result<&mut Leaf> {
        self.tree
            .find_leaf_mut(pane)
            .ok_or_else(|| WorkspaceError::PaneNotFound(pane.clone()))
    }

    fn is_empty_tab(&self, id: &TabId) -> bool {
        self.tab(id).is_some_and(|t| t.kind == TabKind::Empty)
    }

    fn drop_tab(&mut self, id: &TabId) -> Option<Tab> {
        let idx = self.tabs.iter().position(|t| t.id == *id)?;
        Some(self.tabs.remove(idx))
    }

    fn remove_leaf(&mut self, pane: &PaneId) {
        self.tree = std::mem::take(&mut self.tree).remove_leaf(pane);
    }

    /// Foco inválido (folha removida) volta para a primeira folha.
    fn ensure_focus_valid(&mut self) {
        let valid = self
            .focused
            .as_ref()
            .is_some_and(|id| self.tree.find_leaf(id).is_some());
        if !valid {
            self.focused = self.tree.leaves().first().map(|l| l.id.clone());
        }
    }
}

/// Empréstimo mutável de um workspace com o gerador de ids.
pub struct WorkspaceEditor<'a> {
    ws: &'a mut Workspace,
    ids: &'a mut IdGen,
}

impl WorkspaceEditor<'_> {
    pub fn workspace(&self) -> &Workspace {
        self.ws
    }

    fn require_tab(&self, id: &TabId) -> Result<()> {
        match self.ws.tab(id) {
            Some(_) => Ok(()),
            None => Err(WorkspaceError::TabNotFound(id.clone())),
        }
    }

    fn require_in_pane(&self, pane: &PaneId, tab: &TabId) -> Result<()> {
        if self.ws.leaf(pane)?.tabs.contains(tab) {
            Ok(())
        } else {
            Err(WorkspaceError::TabNotInPane {
                tab: tab.clone(),
                pane: pane.clone(),
            })
        }
    }

    // ---- foco ---------------------------------------------------------------

    /// Foca a folha.
    pub fn focus_pane(&mut self, pane: &PaneId) -> Result<()> {
        self.ws.leaf(pane)?;
        self.ws.focused = Some(pane.clone());
        Ok(())
    }

    /// Ativa a aba na folha e foca a folha (clicar na aba).
    pub fn select_tab(&mut self, pane: &PaneId, tab: &TabId) -> Result<()> {
        self.require_in_pane(pane, tab)?;
        self.ws.leaf_mut(pane)?.active = tab.clone();
        self.ws.focused = Some(pane.clone());
        Ok(())
    }

    /// Ativa a aba onde quer que esteja e foca a folha dela. Devolve a folha.
    pub fn focus_tab(&mut self, tab: &TabId) -> Result<PaneId> {
        let pane = self
            .ws
            .leaf_of_tab(tab)
            .map(|l| l.id.clone())
            .ok_or_else(|| WorkspaceError::TabNotFound(tab.clone()))?;
        self.select_tab(&pane, tab)?;
        Ok(pane)
    }

    /// Move o foco para a folha vizinha. `None` (e nada muda) se há só uma folha
    /// ou não existe vizinha naquela direção; não cicla.
    pub fn focus_toward(&mut self, mv: PaneMove) -> Option<PaneId> {
        let current = self.ws.focused_pane_id()?.clone();
        let target = self.ws.tree.neighbor_leaf(&current, mv)?.clone();
        if target == current {
            return None;
        }
        self.ws.focused = Some(target.clone());
        Some(target)
    }

    // ---- criar abas ---------------------------------------------------------

    /// Anexa um placeholder vazio à folha e o ativa.
    pub fn new_empty_tab(&mut self, pane: &PaneId) -> Result<TabId> {
        self.ws.leaf(pane)?;
        let tab = build_tab(self.ids, NewTab::empty());
        let id = tab.id.clone();
        let leaf = self.ws.leaf_mut(pane)?;
        leaf.tabs.push(id.clone());
        leaf.active = id.clone();
        self.ws.tabs.push(tab);
        self.ws.focused = Some(pane.clone());
        Ok(id)
    }

    /// Abre uma aba na folha e a ativa. Um placeholder vazio é substituído, não
    /// acompanhado: no terminal, quando é a aba **ativa** (mesmo havendo outras);
    /// nos demais tipos, só quando é a **única** aba da folha. O placeholder
    /// substituído sai do workspace.
    pub fn open_tab(&mut self, pane: &PaneId, spec: NewTab) -> Result<TabId> {
        let leaf = self.ws.leaf(pane)?;
        let replaced = if spec.kind == TabKind::Terminal {
            Some(&leaf.active).filter(|a| self.ws.is_empty_tab(a))
        } else if leaf.tabs.len() == 1 {
            leaf.tabs.first().filter(|t| self.ws.is_empty_tab(t))
        } else {
            None
        }
        .cloned();
        let tab = build_tab(self.ids, spec);
        let id = tab.id.clone();
        let leaf = self.ws.leaf_mut(pane)?;
        match &replaced {
            Some(old) => replace_in_leaf(leaf, old, &id),
            None => leaf.tabs.push(id.clone()),
        }
        leaf.active = id.clone();
        self.ws.tabs.push(tab);
        if let Some(old) = replaced {
            self.ws.drop_tab(&old);
        }
        self.ws.focused = Some(pane.clone());
        Ok(id)
    }

    /// Troca o placeholder `empty` da folha por uma aba nova, que vira a ativa.
    pub fn fill_empty(&mut self, pane: &PaneId, empty: &TabId, spec: NewTab) -> Result<TabId> {
        self.require_in_pane(pane, empty)?;
        if !self.ws.is_empty_tab(empty) {
            return Err(WorkspaceError::NotEmptyTab(empty.clone()));
        }
        let tab = build_tab(self.ids, spec);
        let id = tab.id.clone();
        let leaf = self.ws.leaf_mut(pane)?;
        replace_in_leaf(leaf, empty, &id);
        leaf.active = id.clone();
        self.ws.tabs.push(tab);
        self.ws.drop_tab(empty);
        self.ws.focused = Some(pane.clone());
        Ok(id)
    }

    /// Troca a aba por outra, **na mesma posição** (reiniciar um terminal: o id
    /// muda, o lugar não). A nova é ativa só se a antiga era; o foco não muda.
    pub fn replace_tab(&mut self, pane: &PaneId, tab: &TabId, spec: NewTab) -> Result<TabId> {
        self.require_in_pane(pane, tab)?;
        self.require_tab(tab)?;
        let new = build_tab(self.ids, spec);
        let id = new.id.clone();
        let leaf = self.ws.leaf_mut(pane)?;
        if leaf.active == *tab {
            leaf.active = id.clone();
        }
        replace_in_leaf(leaf, tab, &id);
        self.ws.tabs.push(new);
        self.ws.drop_tab(tab);
        Ok(id)
    }

    // ---- fechar -------------------------------------------------------------

    /// Fecha a aba. Última aba de uma folha: a folha some e o irmão expande;
    /// se era a única folha do workspace, fica um placeholder vazio. A ativa
    /// passa para a aba anterior à fechada.
    pub fn close_tab(&mut self, pane: &PaneId, tab: &TabId) -> Result<TabClosed> {
        self.require_in_pane(pane, tab)?;
        self.require_tab(tab)?;
        let leaf = self.ws.leaf(pane)?;
        let remaining: Vec<TabId> = leaf.tabs.iter().filter(|t| *t != tab).cloned().collect();
        let next_active = active_after(&leaf.tabs, &leaf.active, tab, &remaining);
        let only_leaf = self.ws.tree.leaves().len() == 1;

        let mut placeholder = None;
        let mut pane_removed = false;
        if !remaining.is_empty() {
            let leaf = self.ws.leaf_mut(pane)?;
            leaf.tabs = remaining;
            leaf.active = next_active;
        } else if only_leaf {
            let empty = build_tab(self.ids, NewTab::empty());
            let id = empty.id.clone();
            let leaf = self.ws.leaf_mut(pane)?;
            leaf.tabs = vec![id.clone()];
            leaf.active = id.clone();
            self.ws.tabs.push(empty);
            placeholder = Some(id);
        } else {
            self.ws.remove_leaf(pane);
            pane_removed = true;
        }
        let closed = self
            .ws
            .drop_tab(tab)
            .ok_or_else(|| WorkspaceError::TabNotFound(tab.clone()))?;
        self.ws.ensure_focus_valid();
        Ok(TabClosed {
            tab: closed,
            pane_removed,
            placeholder,
        })
    }

    /// Fecha a folha e todas as abas dela (devolvidas para o adaptador encerrar).
    /// Se era a única folha, ela fica com um placeholder vazio.
    pub fn close_pane(&mut self, pane: &PaneId) -> Result<Vec<Tab>> {
        let ids = self.ws.leaf(pane)?.tabs.clone();
        if self.ws.tree.leaves().len() == 1 {
            let empty = build_tab(self.ids, NewTab::empty());
            let id = empty.id.clone();
            let leaf = self.ws.leaf_mut(pane)?;
            leaf.tabs = vec![id.clone()];
            leaf.active = id;
            self.ws.tabs.push(empty);
        } else {
            self.ws.remove_leaf(pane);
        }
        let closed = ids.iter().filter_map(|id| self.ws.drop_tab(id)).collect();
        self.ws.ensure_focus_valid();
        Ok(closed)
    }

    // ---- dividir ------------------------------------------------------------

    /// Divide a folha criando, depois dela (direita/baixo), uma folha com a aba
    /// nova. Para dividir "vazio", passe [`NewTab::empty`]. A folha nova ganha o
    /// foco.
    pub fn split_pane(
        &mut self,
        pane: &PaneId,
        dir: SplitDir,
        spec: NewTab,
    ) -> Result<PaneCreated> {
        self.ws.leaf(pane)?;
        let tab = build_tab(self.ids, spec);
        let created = self.insert_split(pane, dir, false, tab.id.clone());
        self.ws.tabs.push(tab);
        Ok(created)
    }

    fn insert_split(
        &mut self,
        target: &PaneId,
        dir: SplitDir,
        before: bool,
        tab: TabId,
    ) -> PaneCreated {
        let pane = PaneId::new(self.ids.next("pane"));
        let split_id = PaneId::new(self.ids.next("sp"));
        let leaf = Leaf {
            id: pane.clone(),
            tabs: vec![tab.clone()],
            active: tab.clone(),
        };
        self.ws.tree =
            std::mem::take(&mut self.ws.tree).split_leaf(target, dir, leaf, split_id, before);
        self.ws.focused = Some(pane.clone());
        PaneCreated { pane, tab }
    }

    // ---- mover / reordenar --------------------------------------------------

    /// Move a aba de `src` para `dst` (a aba é a mesma; só muda de lugar). `index`
    /// é o slot no destino (0..=len, vale na lista antes da remoção quando é a
    /// mesma folha); `None` anexa no fim. Mesma folha sem `index`: nada muda.
    /// A aba movida fica ativa e `dst` focada; origem vazia some e o irmão expande.
    pub fn move_tab(
        &mut self,
        src: &PaneId,
        tab: &TabId,
        dst: &PaneId,
        index: Option<usize>,
    ) -> Result<()> {
        self.require_in_pane(src, tab)?;
        let src_leaf = self.ws.leaf(src)?.clone();
        let mut dst_tabs = self.ws.leaf(dst)?.tabs.clone();

        if src == dst {
            if let Some(i) = index {
                let leaf = self.ws.leaf_mut(src)?;
                leaf.tabs = reorder_tabs(&src_leaf.tabs, tab, i);
                leaf.active = tab.clone();
                self.ws.focused = Some(src.clone());
            }
            return Ok(());
        }

        let remaining: Vec<TabId> = src_leaf
            .tabs
            .iter()
            .filter(|t| *t != tab)
            .cloned()
            .collect();
        let at = index.map_or(dst_tabs.len(), |i| i.min(dst_tabs.len()));
        dst_tabs.insert(at, tab.clone());
        let dst_leaf = self.ws.leaf_mut(dst)?;
        dst_leaf.tabs = dst_tabs;
        dst_leaf.active = tab.clone();
        self.drain_source(src, &src_leaf, tab, remaining)?;
        self.ws.focused = Some(dst.clone());
        self.ws.ensure_focus_valid();
        Ok(())
    }

    /// [`Self::move_tab`] dentro da mesma folha.
    pub fn reorder_tab(&mut self, pane: &PaneId, tab: &TabId, index: usize) -> Result<()> {
        self.move_tab(pane, tab, pane, Some(index))
    }

    /// Tira a aba da folha de origem: com abas sobrando atualiza a ativa, sem
    /// nenhuma remove a folha.
    fn drain_source(
        &mut self,
        src: &PaneId,
        before: &Leaf,
        tab: &TabId,
        remaining: Vec<TabId>,
    ) -> Result<()> {
        if remaining.is_empty() {
            self.ws.remove_leaf(src);
        } else {
            let leaf = self.ws.leaf_mut(src)?;
            leaf.active = active_after(&before.tabs, &before.active, tab, &remaining);
            leaf.tabs = remaining;
        }
        Ok(())
    }

    /// Move a aba para uma folha nova, criada dividindo `target` em `dir`
    /// (`before` = a folha nova fica à esquerda/acima). A nova folha ganha o foco.
    pub fn move_tab_to_new_split(
        &mut self,
        src: &PaneId,
        tab: &TabId,
        target: &PaneId,
        dir: SplitDir,
        before: bool,
    ) -> Result<PaneId> {
        self.require_in_pane(src, tab)?;
        self.ws.leaf(target)?;
        let src_leaf = self.ws.leaf(src)?.clone();
        let remaining: Vec<TabId> = src_leaf
            .tabs
            .iter()
            .filter(|t| *t != tab)
            .cloned()
            .collect();
        if src == target && remaining.is_empty() {
            return Err(WorkspaceError::SplitOntoSelf(src.clone()));
        }
        let src_removed = remaining.is_empty();
        if !src_removed {
            self.drain_source(src, &src_leaf, tab, remaining)?;
        }
        let created = self.insert_split(target, dir, before, tab.clone());
        if src_removed {
            self.ws.remove_leaf(src);
        }
        self.ws.ensure_focus_valid();
        Ok(created.pane)
    }

    // ---- divisor ------------------------------------------------------------

    /// Define a proporção da divisão, travada em [`MIN_FRAC`]..=[`MAX_FRAC`].
    pub fn resize_split(&mut self, split: &PaneId, frac: f64) -> Result<()> {
        if !frac.is_finite() {
            return Err(WorkspaceError::InvalidFraction(frac));
        }
        let s = self
            .ws
            .tree
            .find_split_mut(split)
            .ok_or_else(|| WorkspaceError::PaneNotFound(split.clone()))?;
        s.frac = frac.clamp(MIN_FRAC, MAX_FRAC);
        Ok(())
    }

    /// Soma `delta` à proporção **atual** (arrastar o divisor), com a mesma trava.
    pub fn resize_split_by(&mut self, split: &PaneId, delta: f64) -> Result<()> {
        if !delta.is_finite() {
            return Err(WorkspaceError::InvalidFraction(delta));
        }
        let s = self
            .ws
            .tree
            .find_split_mut(split)
            .ok_or_else(|| WorkspaceError::PaneNotFound(split.clone()))?;
        s.frac = (s.frac + delta).clamp(MIN_FRAC, MAX_FRAC);
        Ok(())
    }

    // ---- nomes --------------------------------------------------------------

    fn tab_mut(&mut self, id: &TabId) -> Result<&mut Tab> {
        self.ws
            .tabs
            .iter_mut()
            .find(|t| t.id == *id)
            .ok_or_else(|| WorkspaceError::TabNotFound(id.clone()))
    }

    /// Define o rótulo manual (trava o nome). Vazio é ignorado.
    pub fn set_label(&mut self, tab: &TabId, label: &str) -> Result<()> {
        let tab = self.tab_mut(tab)?;
        if let Some(l) = clean_label(label) {
            tab.label = Some(l);
        }
        Ok(())
    }

    /// Descarta o rótulo manual: a aba volta a seguir o título automático.
    pub fn clear_label(&mut self, tab: &TabId) -> Result<()> {
        self.tab_mut(tab)?.label = None;
        Ok(())
    }

    /// Atualiza o título dinâmico (segue vivo mesmo com rótulo manual).
    pub fn set_title(&mut self, tab: &TabId, title: impl Into<String>) -> Result<()> {
        self.tab_mut(tab)?.title = title.into();
        Ok(())
    }
}

/// O estado completo: workspaces na ordem do rail, seleção e contadores de id.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Workspaces {
    #[serde(default)]
    ids: IdGen,
    #[serde(default)]
    next_workspace: u64,
    #[serde(default)]
    workspaces: Vec<Workspace>,
    #[serde(default)]
    selected: Option<WorkspaceId>,
}

impl Workspaces {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn workspaces(&self) -> &[Workspace] {
        &self.workspaces
    }

    pub fn workspace(&self, id: &WorkspaceId) -> Option<&Workspace> {
        self.workspaces.iter().find(|w| w.id == *id)
    }

    pub fn selected_id(&self) -> Option<&WorkspaceId> {
        self.selected.as_ref()
    }

    pub fn selected(&self) -> Option<&Workspace> {
        self.selected.as_ref().and_then(|id| self.workspace(id))
    }

    /// Cria um workspace com uma folha e um placeholder vazio, e o seleciona.
    /// `name` vazio cai no último segmento de `path` (e, sem isso, no próprio
    /// `path`). Com `path` não vazio já existente devolve o workspace que existe
    /// (e o seleciona) em vez de duplicar.
    pub fn create_workspace(&mut self, name: &str, path: &str) -> Result<WorkspaceId> {
        if !path.is_empty() {
            if let Some(existing) = self.workspaces.iter().find(|w| w.path == path) {
                let id = existing.id.clone();
                self.selected = Some(id.clone());
                return Ok(id);
            }
        }
        let trimmed = name.trim();
        let name = if !trimmed.is_empty() {
            trimmed.to_string()
        } else if path.is_empty() {
            return Err(WorkspaceError::EmptyName);
        } else {
            basename(path)
        };

        let empty = build_tab(&mut self.ids, NewTab::empty());
        let pane = PaneId::new(self.ids.next("pane"));
        let id = WorkspaceId::new(format!("{WORKSPACE_PREFIX}{}", self.next_workspace));
        self.next_workspace = self.next_workspace.saturating_add(1);
        self.workspaces.push(Workspace {
            id: id.clone(),
            name,
            path: path.to_string(),
            tree: PaneNode::Leaf(Leaf {
                id: pane.clone(),
                tabs: vec![empty.id.clone()],
                active: empty.id.clone(),
            }),
            focused: Some(pane),
            tabs: vec![empty],
        });
        self.selected = Some(id.clone());
        Ok(id)
    }

    pub fn rename_workspace(&mut self, id: &WorkspaceId, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(WorkspaceError::EmptyName);
        }
        self.workspace_mut(id)?.name = name.to_string();
        Ok(())
    }

    /// Remove o workspace e o devolve (o adaptador encerra as abas dele). Se era o
    /// selecionado, a seleção vai para o primeiro que sobrar (ou nenhum).
    pub fn remove_workspace(&mut self, id: &WorkspaceId) -> Result<Workspace> {
        let idx = self
            .workspaces
            .iter()
            .position(|w| w.id == *id)
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(id.clone()))?;
        let removed = self.workspaces.remove(idx);
        if self.selected.as_ref() == Some(id) || self.selected().is_none() {
            self.selected = self.workspaces.first().map(|w| w.id.clone());
        }
        Ok(removed)
    }

    /// Move `moved` para antes ou depois de `target` na lista.
    pub fn reorder_workspace(
        &mut self,
        moved: &WorkspaceId,
        target: &WorkspaceId,
        before: bool,
    ) -> Result<()> {
        let from = self.index_of(moved)?;
        self.index_of(target)?;
        if moved == target {
            return Ok(());
        }
        let item = self.workspaces.remove(from);
        let at = self
            .workspaces
            .iter()
            .position(|w| w.id == *target)
            .map_or(self.workspaces.len(), |i| if before { i } else { i + 1 });
        self.workspaces.insert(at, item);
        Ok(())
    }

    pub fn select_workspace(&mut self, id: &WorkspaceId) -> Result<()> {
        self.index_of(id)?;
        self.selected = Some(id.clone());
        Ok(())
    }

    /// Empresta o workspace para mutação.
    pub fn edit(&mut self, id: &WorkspaceId) -> Result<WorkspaceEditor<'_>> {
        let ws = self
            .workspaces
            .iter_mut()
            .find(|w| w.id == *id)
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(id.clone()))?;
        Ok(WorkspaceEditor {
            ws,
            ids: &mut self.ids,
        })
    }

    /// Leva o usuário até a aba, de qualquer workspace: seleciona o workspace,
    /// ativa a aba e foca a folha. Devolve onde ela estava.
    pub fn reveal_tab(&mut self, tab: &TabId) -> Result<(WorkspaceId, PaneId)> {
        let ws_id = self
            .workspaces
            .iter()
            .find(|w| w.tab(tab).is_some())
            .map(|w| w.id.clone())
            .ok_or_else(|| WorkspaceError::TabNotFound(tab.clone()))?;
        let pane = self.edit(&ws_id)?.focus_tab(tab)?;
        self.selected = Some(ws_id.clone());
        Ok((ws_id, pane))
    }

    /// Uma entrada por aba, no formato que `tabs::format_tab_list` consome
    /// (`gsh list-tabs`). Só traz o que o domínio sabe: quem tem estado de
    /// execução acrescenta `working`, `harness` e `taskId`.
    pub fn tab_list(&self) -> Vec<Value> {
        let focused = self.selected().and_then(Workspace::active_tab_id);
        self.workspaces
            .iter()
            .flat_map(|w| {
                w.tabs.iter().map(move |t| {
                    json!({
                        "id": t.id,
                        "kind": t.kind.cli_kind(),
                        "title": t.title,
                        "label": t.label,
                        "workspaceId": w.id,
                        "workspacePath": w.path,
                        "focused": Some(&t.id) == focused,
                    })
                })
            })
            .collect()
    }

    /// Normaliza um estado vindo de arquivo (porta do `_sanitizeTree`): descarta
    /// abas que não existem ou que aparecem em duas folhas, põe um placeholder em
    /// folha vazia, conserta a aba ativa e o foco, e avança os contadores além de
    /// qualquer id lido para que os próximos não colidam.
    pub fn repair(&mut self) {
        for w in &mut self.workspaces {
            let present: HashSet<TabId> = w.tabs.iter().map(|t| t.id.clone()).collect();
            let mut used = HashSet::new();
            let mut created = Vec::new();
            sanitize(
                &mut w.tree,
                &present,
                &mut used,
                &mut self.ids,
                &mut created,
            );
            w.tabs.retain(|t| used.contains(&t.id));
            w.tabs.extend(created);
            w.ensure_focus_valid();

            for t in &w.tabs {
                self.ids.bump_past(t.id.as_str());
            }
            bump_tree(&w.tree, &mut self.ids);
            if let Some(n) =
                w.id.as_str()
                    .strip_prefix(WORKSPACE_PREFIX)
                    .and_then(numeric_suffix)
            {
                self.next_workspace = self.next_workspace.max(n.saturating_add(1));
            }
        }
        if self.selected().is_none() {
            self.selected = self.workspaces.first().map(|w| w.id.clone());
        }
    }

    fn index_of(&self, id: &WorkspaceId) -> Result<usize> {
        self.workspaces
            .iter()
            .position(|w| w.id == *id)
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(id.clone()))
    }

    fn workspace_mut(&mut self, id: &WorkspaceId) -> Result<&mut Workspace> {
        self.workspaces
            .iter_mut()
            .find(|w| w.id == *id)
            .ok_or_else(|| WorkspaceError::WorkspaceNotFound(id.clone()))
    }
}

fn bump_tree(node: &PaneNode, ids: &mut IdGen) {
    ids.bump_past(node.id().as_str());
    match node {
        PaneNode::Leaf(l) => l.tabs.iter().for_each(|t| ids.bump_past(t.as_str())),
        PaneNode::Split(s) => {
            bump_tree(&s.a, ids);
            bump_tree(&s.b, ids);
        }
    }
}

fn sanitize(
    node: &mut PaneNode,
    present: &HashSet<TabId>,
    used: &mut HashSet<TabId>,
    ids: &mut IdGen,
    created: &mut Vec<Tab>,
) {
    match node {
        PaneNode::Leaf(l) => {
            l.tabs
                .retain(|t| present.contains(t) && used.insert(t.clone()));
            if l.tabs.is_empty() {
                let empty = build_tab(ids, NewTab::empty());
                used.insert(empty.id.clone());
                l.tabs = vec![empty.id.clone()];
                l.active = empty.id.clone();
                created.push(empty);
            } else if !l.tabs.contains(&l.active) {
                if let Some(first) = l.tabs.first() {
                    l.active = first.clone();
                }
            }
        }
        PaneNode::Split(s) => {
            if !s.frac.is_finite() {
                s.frac = DEFAULT_FRAC;
            }
            sanitize(&mut s.a, present, used, ids, created);
            sanitize(&mut s.b, present, used, ids, created);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PaneId {
        PaneId::new(s)
    }
    fn t(s: &str) -> TabId {
        TabId::new(s)
    }
    fn term(title: &str) -> NewTab {
        NewTab::new(TabKind::Terminal, title).with_working_directory("/w")
    }

    /// Workspace novo: `ws0`, folha `pane1` com o placeholder `a0`.
    fn setup() -> (Workspaces, WorkspaceId) {
        let mut s = Workspaces::new();
        let id = s.create_workspace("proj", "/w").unwrap();
        (s, id)
    }

    fn tabs_of(s: &Workspaces, ws: &WorkspaceId, pane: &str) -> Vec<String> {
        s.workspace(ws)
            .and_then(|w| w.tree().find_leaf(&p(pane)))
            .map(|l| l.tabs.iter().map(|t| t.to_string()).collect())
            .unwrap_or_default()
    }

    fn leaf_ids(s: &Workspaces, ws: &WorkspaceId) -> Vec<String> {
        s.workspace(ws)
            .map(|w| w.tree().leaves().iter().map(|l| l.id.to_string()).collect())
            .unwrap_or_default()
    }

    /// Consistência: toda aba da árvore existe, sem duplicata, e vice-versa.
    fn assert_consistent(s: &Workspaces) {
        for w in s.workspaces() {
            let mut seen = HashSet::new();
            for l in w.tree().leaves() {
                assert!(!l.tabs.is_empty(), "folha vazia {}", l.id);
                assert!(l.tabs.contains(&l.active), "ativa fora da folha {}", l.id);
                for tab in &l.tabs {
                    assert!(w.tab(tab).is_some(), "aba sem registro {tab}");
                    assert!(seen.insert(tab.clone()), "aba duplicada {tab}");
                }
            }
            assert_eq!(seen.len(), w.tabs().len(), "aba órfã em {}", w.id());
            assert!(w.focused_leaf().is_some());
        }
    }

    // ---- workspaces ---------------------------------------------------------

    #[test]
    fn cria_workspace_com_folha_e_placeholder() {
        let (s, id) = setup();
        let w = s.workspace(&id).unwrap();
        assert_eq!(id.as_str(), "ws0");
        assert_eq!(w.name(), "proj");
        assert_eq!(w.path(), "/w");
        assert_eq!(tabs_of(&s, &id, "pane1"), ["a0"]);
        assert_eq!(
            w.tab(&t("a0")).map(|x| (x.kind, x.title.as_str())),
            Some((TabKind::Empty, EMPTY_TAB_TITLE))
        );
        assert_eq!(w.focused_pane_id(), Some(&p("pane1")));
        assert_eq!(w.active_tab_id(), Some(&t("a0")));
        assert_eq!(s.selected_id(), Some(&id));
        assert_consistent(&s);
    }

    #[test]
    fn nome_vazio_cai_no_basename_do_path_e_sem_path_falha() {
        let mut s = Workspaces::new();
        let id = s.create_workspace("  ", "/home/u/remote_pi/").unwrap();
        assert_eq!(s.workspace(&id).unwrap().name(), "remote_pi");
        assert_eq!(s.create_workspace("", ""), Err(WorkspaceError::EmptyName));
        let named = s.create_workspace("  Meu  ", "").unwrap();
        assert_eq!(s.workspace(&named).unwrap().name(), "Meu");
    }

    #[test]
    fn mesmo_path_devolve_o_existente_e_seleciona() {
        let (mut s, first) = setup();
        let other = s.create_workspace("outro", "/o").unwrap();
        assert_eq!(s.selected_id(), Some(&other));
        assert_eq!(s.create_workspace("dup", "/w").unwrap(), first);
        assert_eq!(s.selected_id(), Some(&first));
        assert_eq!(s.workspaces().len(), 2);
    }

    #[test]
    fn ids_de_aba_sao_unicos_entre_workspaces() {
        let (mut s, a) = setup();
        let b = s.create_workspace("b", "/b").unwrap();
        let ta = s
            .edit(&a)
            .unwrap()
            .open_tab(&p("pane1"), term("x"))
            .unwrap();
        let pane_b = s.workspace(&b).unwrap().focused_pane_id().cloned().unwrap();
        let tb = s.edit(&b).unwrap().open_tab(&pane_b, term("y")).unwrap();
        assert_ne!(ta, tb);
        assert_ne!(pane_b, p("pane1"));
    }

    #[test]
    fn renomeia_e_recusa_nome_vazio_ou_id_inexistente() {
        let (mut s, id) = setup();
        s.rename_workspace(&id, "  novo ").unwrap();
        assert_eq!(s.workspace(&id).unwrap().name(), "novo");
        assert_eq!(
            s.rename_workspace(&id, "  "),
            Err(WorkspaceError::EmptyName)
        );
        assert_eq!(s.workspace(&id).unwrap().name(), "novo");
        let ghost = WorkspaceId::new("nope");
        assert_eq!(
            s.rename_workspace(&ghost, "x"),
            Err(WorkspaceError::WorkspaceNotFound(ghost))
        );
    }

    #[test]
    fn remove_workspace_move_selecao_para_o_primeiro_que_sobra() {
        let (mut s, a) = setup();
        let b = s.create_workspace("b", "/b").unwrap();
        let c = s.create_workspace("c", "/c").unwrap();
        assert_eq!(s.selected_id(), Some(&c));
        // Remover um não selecionado mantém a seleção.
        let removed = s.remove_workspace(&a).unwrap();
        assert_eq!(removed.id(), &a);
        assert_eq!(s.selected_id(), Some(&c));
        // Remover o selecionado vai para o primeiro restante.
        s.remove_workspace(&c).unwrap();
        assert_eq!(s.selected_id(), Some(&b));
        // Remover o último deixa sem seleção.
        s.remove_workspace(&b).unwrap();
        assert_eq!(s.selected_id(), None);
        assert!(s.workspaces().is_empty());
        assert_eq!(
            s.remove_workspace(&b),
            Err(WorkspaceError::WorkspaceNotFound(b))
        );
    }

    #[test]
    fn reordena_workspaces() {
        let (mut s, a) = setup();
        let b = s.create_workspace("b", "/b").unwrap();
        let c = s.create_workspace("c", "/c").unwrap();
        let order = |s: &Workspaces| {
            s.workspaces()
                .iter()
                .map(|w| w.id().to_string())
                .collect::<Vec<_>>()
        };
        s.reorder_workspace(&c, &a, true).unwrap();
        assert_eq!(order(&s), ["ws2", "ws0", "ws1"]);
        s.reorder_workspace(&c, &b, false).unwrap();
        assert_eq!(order(&s), ["ws0", "ws1", "ws2"]);
        s.reorder_workspace(&a, &a, true).unwrap();
        assert_eq!(order(&s), ["ws0", "ws1", "ws2"]);
        let ghost = WorkspaceId::new("zz");
        assert_eq!(
            s.reorder_workspace(&a, &ghost, true),
            Err(WorkspaceError::WorkspaceNotFound(ghost.clone()))
        );
        assert_eq!(
            s.reorder_workspace(&ghost, &a, true),
            Err(WorkspaceError::WorkspaceNotFound(ghost))
        );
    }

    #[test]
    fn seleciona_workspace() {
        let (mut s, a) = setup();
        let b = s.create_workspace("b", "/b").unwrap();
        s.select_workspace(&a).unwrap();
        assert_eq!(s.selected().map(Workspace::name), Some("proj"));
        assert!(s.select_workspace(&WorkspaceId::new("zz")).is_err());
        assert_eq!(s.selected_id(), Some(&a));
        let _ = b;
    }

    // ---- abas ---------------------------------------------------------------

    #[test]
    fn terminal_substitui_placeholder_ativo() {
        let (mut s, ws) = setup();
        let id = s
            .edit(&ws)
            .unwrap()
            .open_tab(&p("pane1"), term("zsh"))
            .unwrap();
        assert_eq!(id.as_str(), "t2");
        assert_eq!(tabs_of(&s, &ws, "pane1"), ["t2"]);
        let w = s.workspace(&ws).unwrap();
        assert!(
            w.tab(&t("a0")).is_none(),
            "placeholder substituído sai do workspace"
        );
        assert_eq!(w.active_tab_id(), Some(&id));
        assert_eq!(w.tab(&id).map(|x| x.working_directory.as_str()), Some("/w"));
        assert_consistent(&s);
    }

    #[test]
    fn terminal_substitui_placeholder_ativo_mesmo_com_outras_abas() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let t1 = e.open_tab(&p("pane1"), term("a")).unwrap();
        let empty = e.new_empty_tab(&p("pane1")).unwrap();
        let t3 = e.open_tab(&p("pane1"), term("b")).unwrap();
        assert!(e.workspace().tab(&empty).is_none());
        assert_eq!(tabs_of(&s, &ws, "pane1"), [t1.to_string(), t3.to_string()]);
        assert_consistent(&s);
    }

    #[test]
    fn viewer_so_substitui_placeholder_quando_e_a_unica_aba() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let file = NewTab::new(TabKind::File, "a.md");
        let v = e.open_tab(&p("pane1"), file.clone()).unwrap();
        assert_eq!(v.as_str(), "v2");
        assert_eq!(tabs_of(&s, &ws, "pane1"), ["v2"]);
        // Com o placeholder ativo mas não sozinho, o viewer é anexado.
        let mut e = s.edit(&ws).unwrap();
        let empty = e.new_empty_tab(&p("pane1")).unwrap();
        let v2 = e.open_tab(&p("pane1"), file).unwrap();
        assert_eq!(
            tabs_of(&s, &ws, "pane1"),
            ["v2", empty.as_str(), v2.as_str()]
        );
        assert_consistent(&s);
    }

    #[test]
    fn prefixo_do_id_segue_o_tipo() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let mut id = |k, pane| e.open_tab(&p(pane), NewTab::new(k, "x")).unwrap();
        // Só o primeiro substitui o placeholder; o resto é anexado.
        assert!(id(TabKind::Diff, "pane1").as_str().starts_with('d'));
        assert!(id(TabKind::TaskOutput, "pane1").as_str().starts_with('t'));
        assert!(id(TabKind::TelemetryCase, "pane1")
            .as_str()
            .starts_with('y'));
        assert!(id(TabKind::Neovim, "pane1").as_str().starts_with('n'));
        assert!(id(TabKind::Chat, "pane1").as_str().starts_with('v'));
    }

    #[test]
    fn rotulo_manual_trava_o_nome_e_vazio_e_ignorado() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let id = e
            .open_tab(&p("pane1"), term("zsh").with_label("  Worker "))
            .unwrap();
        let name = |e: &WorkspaceEditor| {
            e.workspace()
                .tab(&id)
                .map(|x| (x.display_title().to_string(), x.title_locked()))
        };
        assert_eq!(name(&e), Some(("Worker".into(), true)));
        e.set_label(&id, "   ").unwrap();
        assert_eq!(name(&e), Some(("Worker".into(), true)));
        e.set_title(&id, "vim").unwrap();
        assert_eq!(name(&e), Some(("Worker".into(), true)));
        e.clear_label(&id).unwrap();
        assert_eq!(name(&e), Some(("vim".into(), false)));
        e.set_label(&id, "  Nome ").unwrap();
        assert_eq!(name(&e), Some(("Nome".into(), true)));
        assert_eq!(
            e.set_label(&t("zz"), "x"),
            Err(WorkspaceError::TabNotFound(t("zz")))
        );
    }

    #[test]
    fn operacao_em_folha_inexistente_nao_cria_aba_orfa() {
        let (mut s, ws) = setup();
        let before = s.clone();
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.open_tab(&p("nope"), term("x")),
            Err(WorkspaceError::PaneNotFound(p("nope")))
        );
        assert_eq!(
            e.new_empty_tab(&p("nope")),
            Err(WorkspaceError::PaneNotFound(p("nope")))
        );
        assert!(e
            .split_pane(&p("nope"), SplitDir::Vertical, term("x"))
            .is_err());
        assert_eq!(s.workspace(&ws), before.workspace(&ws));
    }

    #[test]
    fn fill_empty_troca_o_placeholder_e_recusa_aba_que_nao_e_vazia() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let keep = e.open_tab(&p("pane1"), term("a")).unwrap();
        let empty = e.new_empty_tab(&p("pane1")).unwrap();
        // Ativa outra aba: o placeholder fica em segundo plano.
        e.select_tab(&p("pane1"), &keep).unwrap();
        let filled = e.fill_empty(&p("pane1"), &empty, term("b")).unwrap();
        assert_eq!(
            tabs_of(&s, &ws, "pane1"),
            [keep.to_string(), filled.to_string()]
        );
        assert_eq!(s.workspace(&ws).unwrap().active_tab_id(), Some(&filled));
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.fill_empty(&p("pane1"), &keep, term("c")),
            Err(WorkspaceError::NotEmptyTab(keep.clone()))
        );
        assert_eq!(
            e.fill_empty(&p("pane1"), &t("zz"), term("c")),
            Err(WorkspaceError::TabNotInPane {
                tab: t("zz"),
                pane: p("pane1")
            })
        );
        assert_consistent(&s);
    }

    #[test]
    fn replace_tab_mantem_posicao_e_ativa_so_se_era_ativa() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let a = e.open_tab(&p("pane1"), term("a")).unwrap();
        let b = e.new_empty_tab(&p("pane1")).unwrap();
        let b = e.fill_empty(&p("pane1"), &b, term("b")).unwrap();
        // `b` é a ativa; reiniciar `a` não rouba a ativa.
        let a2 = e.replace_tab(&p("pane1"), &a, term("a2")).unwrap();
        assert_eq!(tabs_of(&s, &ws, "pane1"), [a2.to_string(), b.to_string()]);
        assert_eq!(s.workspace(&ws).unwrap().active_tab_id(), Some(&b));
        let mut e = s.edit(&ws).unwrap();
        let b2 = e.replace_tab(&p("pane1"), &b, term("b2")).unwrap();
        assert_eq!(s.workspace(&ws).unwrap().active_tab_id(), Some(&b2));
        assert!(s.workspace(&ws).unwrap().tab(&a).is_none());
        assert_consistent(&s);
    }

    // ---- fechar -------------------------------------------------------------

    #[test]
    fn fechar_ultima_aba_da_unica_folha_deixa_um_placeholder_novo() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let tid = e.open_tab(&p("pane1"), term("a")).unwrap();
        let closed = e.close_tab(&p("pane1"), &tid).unwrap();
        assert_eq!(closed.tab.id, tid);
        assert!(!closed.pane_removed);
        let ph = closed.placeholder.unwrap();
        assert_ne!(ph, t("a0"), "id novo, nunca reaproveitado");
        assert_eq!(tabs_of(&s, &ws, "pane1"), [ph.to_string()]);
        assert_eq!(
            s.workspace(&ws).unwrap().tab(&ph).map(|x| x.kind),
            Some(TabKind::Empty)
        );
        assert_consistent(&s);
    }

    #[test]
    fn fechar_ativa_passa_para_a_anterior_e_primeira_passa_para_a_proxima() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let a = e.open_tab(&p("pane1"), term("a")).unwrap();
        let b = e.new_empty_tab(&p("pane1")).unwrap();
        let c = e.new_empty_tab(&p("pane1")).unwrap();
        e.close_tab(&p("pane1"), &c).unwrap();
        assert_eq!(s.workspace(&ws).unwrap().active_tab_id(), Some(&b));
        let mut e = s.edit(&ws).unwrap();
        e.select_tab(&p("pane1"), &a).unwrap();
        e.close_tab(&p("pane1"), &a).unwrap();
        assert_eq!(
            s.workspace(&ws).unwrap().active_tab_id(),
            Some(&b),
            "índice 0 cai na primeira que resta"
        );
        assert_consistent(&s);
    }

    #[test]
    fn fechar_aba_inativa_nao_muda_a_ativa() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let a = e.open_tab(&p("pane1"), term("a")).unwrap();
        let b = e.new_empty_tab(&p("pane1")).unwrap();
        e.close_tab(&p("pane1"), &a).unwrap();
        assert_eq!(s.workspace(&ws).unwrap().active_tab_id(), Some(&b));
    }

    #[test]
    fn fechar_aba_que_nao_esta_na_folha_ou_nao_existe_e_erro_sem_efeito() {
        let (mut s, ws) = setup();
        let before = s.clone();
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.close_tab(&p("pane1"), &t("zz")),
            Err(WorkspaceError::TabNotInPane {
                tab: t("zz"),
                pane: p("pane1")
            })
        );
        assert_eq!(
            e.close_tab(&p("nope"), &t("a0")),
            Err(WorkspaceError::PaneNotFound(p("nope")))
        );
        assert_eq!(s, before);
    }

    #[test]
    fn fechar_ultima_aba_de_folha_com_irma_colapsa_a_divisao() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let created = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        assert_eq!(leaf_ids(&s, &ws), ["pane1", created.pane.as_str()]);
        let mut e = s.edit(&ws).unwrap();
        let closed = e.close_tab(&created.pane, &created.tab).unwrap();
        assert!(closed.pane_removed);
        assert_eq!(closed.placeholder, None);
        let w = s.workspace(&ws).unwrap();
        assert!(
            matches!(w.tree(), PaneNode::Leaf(l) if l.id == p("pane1")),
            "irmã ocupa a raiz"
        );
        assert_eq!(
            w.focused_pane_id(),
            Some(&p("pane1")),
            "foco volta para uma folha viva"
        );
        assert_consistent(&s);
    }

    #[test]
    fn fechar_pane_devolve_as_abas_e_o_irmao_expande() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        let extra = e.open_tab(&c.pane, term("c")).unwrap();
        let closed = e.close_pane(&c.pane).unwrap();
        let ids: Vec<_> = closed.iter().map(|x| x.id.clone()).collect();
        assert_eq!(ids, [c.tab.clone(), extra]);
        assert_eq!(leaf_ids(&s, &ws), ["pane1"]);
        assert_consistent(&s);
    }

    #[test]
    fn fechar_pane_unica_reinicia_com_placeholder_na_mesma_folha() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let tid = e.open_tab(&p("pane1"), term("a")).unwrap();
        let closed = e.close_pane(&p("pane1")).unwrap();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].id, tid);
        assert_eq!(leaf_ids(&s, &ws), ["pane1"]);
        assert_eq!(
            s.workspace(&ws).unwrap().active_tab().map(|x| x.kind),
            Some(TabKind::Empty)
        );
        assert_consistent(&s);
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.close_pane(&p("nope")),
            Err(WorkspaceError::PaneNotFound(p("nope")))
        );
    }

    // ---- dividir ------------------------------------------------------------

    #[test]
    fn divide_pane_e_foca_a_nova() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Horizontal, term("b"))
            .unwrap();
        assert_eq!((c.pane.as_str(), c.tab.as_str()), ("pane3", "t2"));
        let w = s.workspace(&ws).unwrap();
        assert_eq!(w.focused_pane_id(), Some(&c.pane));
        match w.tree() {
            PaneNode::Split(sp) => {
                assert_eq!(sp.id, p("sp4"));
                assert_eq!(sp.dir, SplitDir::Horizontal);
                assert_eq!(sp.frac, DEFAULT_FRAC);
                assert_eq!(sp.a.id(), &p("pane1"));
                assert_eq!(sp.b.id(), &c.pane);
            }
            PaneNode::Leaf(_) => panic!("esperava divisão"),
        }
        assert_consistent(&s);
    }

    #[test]
    fn dividir_pane_vazio_cria_outro_placeholder_e_o_original_fica_intacto() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, NewTab::empty())
            .unwrap();
        assert_eq!(tabs_of(&s, &ws, "pane1"), ["a0"]);
        assert_eq!(
            s.workspace(&ws).unwrap().tab(&c.tab).map(|x| x.kind),
            Some(TabKind::Empty)
        );
        assert_consistent(&s);
    }

    #[test]
    fn dividir_a_mesma_folha_varias_vezes_gera_ids_de_divisao_distintos() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        e.split_pane(&p("pane1"), SplitDir::Vertical, term("a"))
            .unwrap();
        e.split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        let mut splits = Vec::new();
        fn walk(n: &PaneNode, out: &mut Vec<String>) {
            if let PaneNode::Split(s) = n {
                out.push(s.id.to_string());
                walk(&s.a, out);
                walk(&s.b, out);
            }
        }
        walk(s.workspace(&ws).unwrap().tree(), &mut splits);
        splits.sort();
        splits.dedup();
        assert_eq!(splits.len(), 2);
        assert_eq!(leaf_ids(&s, &ws).len(), 3);
    }

    // ---- foco ---------------------------------------------------------------

    #[test]
    fn foca_pane_e_recusa_id_inexistente_ou_de_divisao() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        e.focus_pane(&p("pane1")).unwrap();
        assert_eq!(
            s.workspace(&ws).unwrap().focused_pane_id(),
            Some(&p("pane1"))
        );
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.focus_pane(&p("nope")),
            Err(WorkspaceError::PaneNotFound(p("nope")))
        );
        assert_eq!(
            e.focus_pane(&p("sp4")),
            Err(WorkspaceError::PaneNotFound(p("sp4"))),
            "divisão não é focável"
        );
        assert_eq!(
            s.workspace(&ws).unwrap().focused_pane_id(),
            Some(&p("pane1"))
        );
        let _ = c;
    }

    #[test]
    fn seleciona_aba_e_recusa_aba_de_outra_folha_ou_inexistente() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        e.select_tab(&p("pane1"), &t("a0")).unwrap();
        assert_eq!(
            s.workspace(&ws).unwrap().focused_pane_id(),
            Some(&p("pane1"))
        );
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.select_tab(&p("pane1"), &c.tab),
            Err(WorkspaceError::TabNotInPane {
                tab: c.tab.clone(),
                pane: p("pane1")
            })
        );
        assert_eq!(
            e.select_tab(&p("pane1"), &t("zz")),
            Err(WorkspaceError::TabNotInPane {
                tab: t("zz"),
                pane: p("pane1")
            })
        );
    }

    #[test]
    fn focus_tab_acha_a_folha_e_recusa_id_inexistente() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        e.focus_pane(&p("pane1")).unwrap();
        assert_eq!(e.focus_tab(&c.tab), Ok(c.pane.clone()));
        assert_eq!(s.workspace(&ws).unwrap().focused_pane_id(), Some(&c.pane));
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.focus_tab(&t("zz")),
            Err(WorkspaceError::TabNotFound(t("zz")))
        );
        assert_eq!(s.workspace(&ws).unwrap().focused_pane_id(), Some(&c.pane));
    }

    #[test]
    fn reveal_tab_troca_de_workspace() {
        let (mut s, a) = setup();
        let tid = s
            .edit(&a)
            .unwrap()
            .open_tab(&p("pane1"), term("x"))
            .unwrap();
        let b = s.create_workspace("b", "/b").unwrap();
        assert_eq!(s.selected_id(), Some(&b));
        assert_eq!(s.reveal_tab(&tid), Ok((a.clone(), p("pane1"))));
        assert_eq!(s.selected_id(), Some(&a));
        assert_eq!(
            s.reveal_tab(&t("zz")),
            Err(WorkspaceError::TabNotFound(t("zz")))
        );
    }

    #[test]
    fn foco_direcional_nao_cicla_e_sem_vizinho_nao_muda() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(e.focus_toward(PaneMove::Right), None, "uma folha só");
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        assert_eq!(e.focus_toward(PaneMove::Right), None, "já na borda direita");
        assert_eq!(e.focus_toward(PaneMove::Left), Some(p("pane1")));
        assert_eq!(e.focus_toward(PaneMove::Left), None);
        assert_eq!(e.focus_toward(PaneMove::Up), None, "eixo sem divisão");
        assert_eq!(e.focus_toward(PaneMove::Right), Some(c.pane));
    }

    #[test]
    fn foco_ausente_ou_invalido_cai_na_primeira_folha() {
        let (mut s, ws) = setup();
        s.edit(&ws)
            .unwrap()
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        let mut json = serde_json::to_value(&s).unwrap();
        json["workspaces"][0]["focused"] = serde_json::json!("pane-que-sumiu");
        let loaded: Workspaces = serde_json::from_value(json).unwrap();
        assert_eq!(
            loaded.workspace(&ws).unwrap().focused_pane_id(),
            Some(&p("pane1"))
        );
    }

    // ---- reordenar / mover --------------------------------------------------

    #[test]
    fn reordena_aba_na_mesma_folha_e_a_torna_ativa() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let a = e.open_tab(&p("pane1"), term("a")).unwrap();
        let b = e.new_empty_tab(&p("pane1")).unwrap();
        let c = e.new_empty_tab(&p("pane1")).unwrap();
        e.reorder_tab(&p("pane1"), &a, 3).unwrap();
        assert_eq!(
            tabs_of(&s, &ws, "pane1"),
            [b.to_string(), c.to_string(), a.to_string()]
        );
        assert_eq!(s.workspace(&ws).unwrap().active_tab_id(), Some(&a));
        s.edit(&ws)
            .unwrap()
            .reorder_tab(&p("pane1"), &a, 0)
            .unwrap();
        assert_eq!(
            tabs_of(&s, &ws, "pane1"),
            [a.to_string(), b.to_string(), c.to_string()]
        );
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.reorder_tab(&p("pane1"), &t("zz"), 0),
            Err(WorkspaceError::TabNotInPane {
                tab: t("zz"),
                pane: p("pane1")
            })
        );
        assert_consistent(&s);
    }

    #[test]
    fn move_aba_para_outra_folha_no_fim_ou_no_slot() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let a = e.open_tab(&p("pane1"), term("a")).unwrap();
        let x = e.new_empty_tab(&p("pane1")).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        e.move_tab(&p("pane1"), &x, &c.pane, None).unwrap();
        assert_eq!(tabs_of(&s, &ws, "pane1"), [a.to_string()]);
        assert_eq!(
            tabs_of(&s, &ws, c.pane.as_str()),
            [c.tab.to_string(), x.to_string()]
        );
        let w = s.workspace(&ws).unwrap();
        assert_eq!(w.focused_pane_id(), Some(&c.pane));
        assert_eq!(w.active_tab_id(), Some(&x));
        let mut e = s.edit(&ws).unwrap();
        e.move_tab(&c.pane, &x, &p("pane1"), Some(0)).unwrap();
        assert_eq!(tabs_of(&s, &ws, "pane1"), [x.to_string(), a.to_string()]);
        assert_eq!(s.workspace(&ws).unwrap().active_tab_id(), Some(&x));
        assert_consistent(&s);
    }

    #[test]
    fn mover_a_ultima_aba_remove_a_folha_de_origem() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        e.move_tab(&c.pane, &c.tab, &p("pane1"), None).unwrap();
        assert_eq!(leaf_ids(&s, &ws), ["pane1"]);
        assert_eq!(tabs_of(&s, &ws, "pane1"), ["a0", c.tab.as_str()]);
        assert_eq!(
            s.workspace(&ws).unwrap().focused_pane_id(),
            Some(&p("pane1"))
        );
        assert_consistent(&s);
    }

    #[test]
    fn mover_na_mesma_folha_sem_indice_e_noop_e_erros_sao_tipados() {
        let (mut s, ws) = setup();
        let before = s.clone();
        let mut e = s.edit(&ws).unwrap();
        e.move_tab(&p("pane1"), &t("a0"), &p("pane1"), None)
            .unwrap();
        assert_eq!(
            e.move_tab(&p("nope"), &t("a0"), &p("pane1"), None),
            Err(WorkspaceError::PaneNotFound(p("nope")))
        );
        assert_eq!(
            e.move_tab(&p("pane1"), &t("a0"), &p("nope"), None),
            Err(WorkspaceError::PaneNotFound(p("nope")))
        );
        assert!(e
            .move_tab(&p("pane1"), &t("zz"), &p("pane1"), None)
            .is_err());
        assert_eq!(s, before);
    }

    #[test]
    fn move_aba_para_nova_divisao_antes_e_depois() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let a = e.open_tab(&p("pane1"), term("a")).unwrap();
        let x = e.new_empty_tab(&p("pane1")).unwrap();
        let new = e
            .move_tab_to_new_split(&p("pane1"), &x, &p("pane1"), SplitDir::Vertical, true)
            .unwrap();
        assert_eq!(
            leaf_ids(&s, &ws),
            [new.as_str(), "pane1"],
            "before = nova à esquerda"
        );
        assert_eq!(tabs_of(&s, &ws, "pane1"), [a.to_string()]);
        assert_eq!(tabs_of(&s, &ws, new.as_str()), [x.to_string()]);
        assert_eq!(s.workspace(&ws).unwrap().focused_pane_id(), Some(&new));
        assert_consistent(&s);
    }

    #[test]
    fn mover_a_unica_aba_para_nova_divisao_de_outra_folha_remove_a_origem() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        let c = e
            .split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        let new = e
            .move_tab_to_new_split(&c.pane, &c.tab, &p("pane1"), SplitDir::Horizontal, false)
            .unwrap();
        let ids = leaf_ids(&s, &ws);
        assert_eq!(ids, ["pane1", new.as_str()]);
        assert!(!ids.contains(&c.pane.to_string()));
        assert_consistent(&s);
    }

    #[test]
    fn dividir_a_folha_de_uma_aba_contra_si_mesma_e_erro_sem_efeito() {
        let (mut s, ws) = setup();
        let before = s.clone();
        let mut e = s.edit(&ws).unwrap();
        assert_eq!(
            e.move_tab_to_new_split(
                &p("pane1"),
                &t("a0"),
                &p("pane1"),
                SplitDir::Vertical,
                false
            ),
            Err(WorkspaceError::SplitOntoSelf(p("pane1")))
        );
        assert_eq!(s, before);
    }

    // ---- divisor ------------------------------------------------------------

    #[test]
    fn redimensiona_com_trava_e_recusa_id_ruim() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        e.split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        let frac = |s: &Workspaces| match s.workspace(&ws).unwrap().tree() {
            PaneNode::Split(sp) => sp.frac,
            PaneNode::Leaf(_) => f64::NAN,
        };
        let sp = p("sp4");
        let mut e = s.edit(&ws).unwrap();
        e.resize_split(&sp, 0.7).unwrap();
        e.resize_split(&sp, 0.99).unwrap();
        assert_eq!(frac(&s), MAX_FRAC);
        let mut e = s.edit(&ws).unwrap();
        e.resize_split(&sp, 0.0).unwrap();
        assert_eq!(frac(&s), MIN_FRAC);
        let mut e = s.edit(&ws).unwrap();
        e.resize_split(&sp, 0.5).unwrap();
        e.resize_split_by(&sp, 0.1).unwrap();
        e.resize_split_by(&sp, 0.1).unwrap();
        assert!(
            (frac(&s) - 0.7).abs() < 1e-9,
            "deltas acumulam sobre o valor atual"
        );
        let mut e = s.edit(&ws).unwrap();
        e.resize_split_by(&sp, -5.0).unwrap();
        assert_eq!(frac(&s), MIN_FRAC);
        let mut e = s.edit(&ws).unwrap();
        assert!(matches!(
            e.resize_split(&sp, f64::NAN),
            Err(WorkspaceError::InvalidFraction(_))
        ));
        assert!(matches!(
            e.resize_split_by(&sp, f64::INFINITY),
            Err(WorkspaceError::InvalidFraction(_))
        ));
        assert_eq!(
            e.resize_split(&p("pane1"), 0.5),
            Err(WorkspaceError::PaneNotFound(p("pane1"))),
            "folha não é divisão"
        );
        assert_eq!(frac(&s), MIN_FRAC);
    }

    // ---- ids / serde / repair / protocolo -----------------------------------

    #[test]
    fn edit_de_workspace_inexistente_e_erro() {
        let mut s = Workspaces::new();
        let ghost = WorkspaceId::new("ws9");
        assert_eq!(
            s.edit(&ghost).err(),
            Some(WorkspaceError::WorkspaceNotFound(ghost))
        );
    }

    #[test]
    fn tipos_de_aba_usam_os_nomes_do_flutter() {
        let name = |k| serde_json::to_value(k).unwrap();
        assert_eq!(name(TabKind::File), "viewer");
        assert_eq!(name(TabKind::Neovim), "file_engine");
        assert_eq!(name(TabKind::TaskOutput), "task_output");
        assert_eq!(name(TabKind::AgentGraph), "agent_graph");
        assert_eq!(name(TabKind::Terminal), "terminal");
        assert_eq!(TabKind::Neovim.cli_kind(), "terminal");
        assert_eq!(TabKind::AgentGraph.cli_kind(), "agents");
        assert_eq!(TabKind::Browser.cli_kind(), "other");
    }

    #[test]
    fn estado_faz_ida_e_volta_em_json_e_continua_gerando_ids_sem_colidir() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        e.split_pane(&p("pane1"), SplitDir::Vertical, term("b"))
            .unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let mut back: Workspaces = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
        let next = back
            .edit(&ws)
            .unwrap()
            .open_tab(&p("pane1"), term("c"))
            .unwrap();
        assert_eq!(next.as_str(), "t5");
    }

    #[test]
    fn arquivo_antigo_sem_campos_novos_carrega_com_defaults() {
        let old = serde_json::json!({
            "workspaces": [{
                "id": "ws0", "name": "x",
                "tree": {"k": "leaf", "id": "pane1", "tabs": ["a0"], "active": "a0"},
                "tabs": [{"id": "a0", "kind": "empty", "title": "New tab"}]
            }]
        });
        let mut s: Workspaces = serde_json::from_value(old).unwrap();
        let w = s.workspace(&WorkspaceId::new("ws0")).unwrap();
        assert_eq!(w.path(), "");
        assert_eq!(w.tab(&t("a0")).map(|x| x.label.clone()), Some(None));
        assert_eq!(s.selected_id(), None);
        s.repair();
        assert_eq!(s.selected_id(), Some(&WorkspaceId::new("ws0")));
        let id = s.create_workspace("n", "").unwrap();
        assert_eq!(
            id.as_str(),
            "ws1",
            "contador de workspace passou do ws0 lido"
        );
    }

    #[test]
    fn repair_descarta_abas_fantasma_e_duplicadas_e_poe_placeholder_em_folha_vazia() {
        let broken = serde_json::json!({
            "workspaces": [{
                "id": "ws0", "name": "x", "focused": "sumiu",
                "tree": {
                    "k": "split", "id": "sp9", "dir": "vertical", "frac": 0.5,
                    "a": {"k": "leaf", "id": "pane1", "tabs": ["t3", "fantasma", "t4"], "active": "fantasma"},
                    "b": {"k": "leaf", "id": "pane2", "tabs": ["t3", "so-fantasma"], "active": "t3"}
                },
                "tabs": [
                    {"id": "t3", "kind": "terminal", "title": "a"},
                    {"id": "t4", "kind": "terminal", "title": "b"},
                    {"id": "orfa", "kind": "terminal", "title": "c"}
                ]
            }]
        });
        let mut s: Workspaces = serde_json::from_value(broken).unwrap();
        s.repair();
        let ws = WorkspaceId::new("ws0");
        assert_eq!(tabs_of(&s, &ws, "pane1"), ["t3", "t4"]);
        assert_eq!(
            s.workspace(&ws)
                .unwrap()
                .tree()
                .find_leaf(&p("pane1"))
                .map(|l| l.active.to_string())
                .as_deref(),
            Some("t3")
        );
        let pane2 = tabs_of(&s, &ws, "pane2");
        assert_eq!(pane2.len(), 1);
        assert!(
            pane2[0].starts_with('a'),
            "t3 duplicada saiu; placeholder entrou: {pane2:?}"
        );
        assert!(s.workspace(&ws).unwrap().tab(&t("orfa")).is_none());
        assert_eq!(
            s.workspace(&ws).unwrap().focused_pane_id(),
            Some(&p("pane1"))
        );
        assert_consistent(&s);
        // O próximo id passa de tudo que foi lido (sp9).
        let next = s
            .edit(&ws)
            .unwrap()
            .open_tab(&p("pane1"), term("n"))
            .unwrap();
        assert!(numeric_suffix(next.as_str()).unwrap() > 9);
    }

    #[test]
    fn tab_list_alimenta_o_formatador_da_cli() {
        let (mut s, ws) = setup();
        let mut e = s.edit(&ws).unwrap();
        e.open_tab(&p("pane1"), term("zsh").with_label("Worker"))
            .unwrap();
        let c = e
            .split_pane(
                &p("pane1"),
                SplitDir::Vertical,
                NewTab::new(TabKind::File, "a.md"),
            )
            .unwrap();
        let list = s.tab_list();
        assert_eq!(list.len(), 2);
        assert_eq!(list[1]["focused"], true);
        assert_eq!(list[0]["focused"], false);
        assert_eq!(list[1]["id"], c.tab.as_str());
        let lines = crate::tabs::format_tab_list(&list);
        assert_eq!(
            lines,
            [
                "  t2     terminal  w              ⚲ Worker",
                format!(
                    "  {} file      w              a.md",
                    c.tab.as_str().to_string() + "    "
                )
                .as_str(),
            ]
        );
    }
}
