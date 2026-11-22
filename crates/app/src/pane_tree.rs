//! Árvore de panes do centro (`pane_node.dart` e `closePane`/`splitPane` de
//! `gsh_viewmodel.dart` no Flutter).
//!
//! Árvore binária de divisões aninháveis, cada folha com um [`PaneModel`] (abas).
//! A estrutura (eixo, proporção, "o irmão expande ao fechar") é a do
//! `pane_grid::State` do iced, que é dado puro: nada aqui precisa de janela nem
//! de renderer. `Axis::Vertical` = lado a lado (`SplitDir.vertical` do Flutter),
//! `Axis::Horizontal` = empilhado (`SplitDir.horizontal`).

use iced::widget::pane_grid::{self, Axis, Node, Pane, Split};

use crate::pane_model::{BrowserId, ChatId, PaneModel, PaneTab, QueryId, TermId};

/// Limites da proporção de uma divisão (`resizeSplitBy` do Flutter: `clamp(0.16, 0.84)`).
pub const MIN_RATIO: f32 = 0.16;
pub const MAX_RATIO: f32 = 0.84;

/// Resultado de fechar um pane: as abas que ele tinha (o chamador encerra as
/// sessões) e, se era o único pane, o terminal novo que ocupa o lugar.
#[derive(Debug, PartialEq, Eq)]
pub struct ClosedPane {
    pub tabs: Vec<PaneTab>,
    pub fresh_terminal: Option<TermId>,
}

pub struct PaneTree {
    grid: pane_grid::State<PaneModel>,
    focused: Pane,
    /// Só devolvido por `focused_model*` se o foco apontasse para um pane que não
    /// existe, o que a árvore nunca permite; existe para essas funções serem totais.
    fallback: PaneModel,
    next_term_id: TermId,
    next_chat_id: ChatId,
    next_query_id: QueryId,
    next_browser_id: BrowserId,
}

impl Default for PaneTree {
    /// Árvore de um pane só, com o terminal `1` aberto.
    fn default() -> Self {
        let mut first = PaneModel::empty();
        first.open_terminal(1);
        let (grid, focused) = pane_grid::State::new(first);
        Self {
            grid,
            focused,
            fallback: PaneModel::empty(),
            next_term_id: 2,
            next_chat_id: 1,
            next_query_id: 1,
            next_browser_id: 1,
        }
    }
}

impl PaneTree {
    pub(crate) fn from_restored_parts(
        grid: pane_grid::State<PaneModel>,
        focused: Pane,
        next_term_id: TermId,
    ) -> Self {
        let next_chat_id = grid
            .iter()
            .flat_map(|(_, m)| m.tabs())
            .filter_map(|t| {
                if let PaneTab::Chat(id) = t {
                    Some(*id)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let next_query_id = grid
            .iter()
            .flat_map(|(_, m)| m.tabs())
            .filter_map(|t| {
                if let PaneTab::Query { id, .. } = t {
                    Some(*id)
                } else {
                    None
                }
            })
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let next_browser_id = grid
            .iter()
            .flat_map(|(_, m)| m.tabs())
            .filter_map(|t| match t {
                PaneTab::Redis { id, .. } | PaneTab::Mongo { id, .. } => Some(*id),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        Self {
            grid,
            focused,
            fallback: PaneModel::empty(),
            next_term_id,
            next_chat_id,
            next_query_id,
            next_browser_id,
        }
    }

    pub fn grid(&self) -> &pane_grid::State<PaneModel> {
        &self.grid
    }

    pub fn focused(&self) -> Pane {
        self.focused
    }

    pub fn focused_model(&self) -> &PaneModel {
        self.grid.get(self.focused).unwrap_or(&self.fallback)
    }

    pub fn focused_model_mut(&mut self) -> &mut PaneModel {
        self.grid
            .get_mut(self.focused)
            .unwrap_or(&mut self.fallback)
    }

    pub fn model(&self, pane: Pane) -> Option<&PaneModel> {
        self.grid.get(pane)
    }

    pub fn model_mut(&mut self, pane: Pane) -> Option<&mut PaneModel> {
        self.grid.get_mut(pane)
    }

    /// Panes na ordem do layout (esquerda para direita, cima para baixo), como `leaves()`.
    pub fn leaves(&self) -> Vec<Pane> {
        fn walk(node: &Node, out: &mut Vec<Pane>) {
            match node {
                Node::Pane(pane) => out.push(*pane),
                Node::Split { a, b, .. } => {
                    walk(a, out);
                    walk(b, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(self.grid.layout(), &mut out);
        out
    }

    /// Modelos de todos os panes, na ordem do layout.
    pub fn models(&self) -> Vec<&PaneModel> {
        self.leaves()
            .into_iter()
            .filter_map(|pane| self.grid.get(pane))
            .collect()
    }

    /// Pane que contém `tab`.
    pub fn pane_of(&self, tab: &PaneTab) -> Option<Pane> {
        self.grid
            .iter()
            .find(|(_, model)| model.tabs().contains(tab))
            .map(|(pane, _)| *pane)
    }

    /// Move an existing tab to another position or pane. `target` is the
    /// insertion slot before removal, matching the Flutter pane reorder flow.
    pub fn move_tab(&mut self, from: Pane, index: usize, to: Pane, target: usize) -> bool {
        if from == to {
            return self
                .grid
                .get_mut(from)
                .is_some_and(|model| model.reorder_tab(index, target));
        }
        if self.grid.get(from).is_none() || self.grid.get(to).is_none() {
            return false;
        }
        let Some(source) = self.grid.get(from) else {
            return false;
        };
        let label = source.tabs().get(index).and_then(|tab| match tab {
            PaneTab::Terminal(id) => source.terminal_label(*id).map(str::to_owned),
            _ => None,
        });
        let Some(tab) = self.grid.get_mut(from).and_then(|model| model.close(index)) else {
            return false;
        };
        let Some(target_model) = self.grid.get_mut(to) else {
            return false;
        };
        target_model.insert_transferred(target, tab, label);
        self.close_if_empty(from);
        true
    }

    /// Move a tab into a newly split pane beside `target`, keeping the tab's
    /// session and OSC title alive. Splitting a pane against itself when it has
    /// only that tab is the same no-op as Flutter's `moveTabToNewSplit`.
    pub fn move_tab_to_split(
        &mut self,
        from: Pane,
        index: usize,
        target: Pane,
        axis: Axis,
        before: bool,
    ) -> Option<Pane> {
        let source = self.grid.get(from)?;
        if from == target && source.tabs().len() == 1 {
            return None;
        }
        let tab = source.tabs().get(index)?.clone();
        let label = match &tab {
            PaneTab::Terminal(id) => source.terminal_label(*id).map(str::to_owned),
            _ => None,
        };
        let mut new_model = PaneModel::empty();
        new_model.insert_transferred(0, tab, label);
        let (new_pane, _) = self.grid.split(axis, target, new_model)?;
        if before {
            self.grid.swap(new_pane, target);
        }
        if self
            .grid
            .get_mut(from)
            .and_then(|model| model.close(index))
            .is_none()
        {
            let _ = self.grid.close(new_pane);
            return None;
        }
        if from != target {
            self.close_if_empty(from);
        }
        self.focused = new_pane;
        Some(new_pane)
    }

    /// Pane que contém a aba da query `id` e o índice dela.
    pub fn query_location(&self, id: QueryId) -> Option<(Pane, usize)> {
        self.grid
            .iter()
            .find_map(|(pane, model)| model.query_index(id).map(|index| (*pane, index)))
    }

    /// Atualiza o título da aba da query `id` em todos os panes.
    pub fn retitle_query(&mut self, id: QueryId, title: &str) {
        for (_, model) in self.grid.iter_mut() {
            model.retitle_query(id, title);
        }
    }

    /// Pane e índice da aba de caso de telemetria com `fingerprint`.
    pub fn find_telemetry_case(&self, fingerprint: &str) -> Option<(Pane, usize)> {
        self.grid.iter().find_map(|(&pane, model)| {
            model.tabs().iter().position(|tab| {
                matches!(tab, PaneTab::TelemetryCase { fingerprint: fp, .. } if fp == fingerprint)
            }).map(|index| (pane, index))
        })
    }

    /// Indica se algum pane tem uma aba de caso de telemetria aberta.
    pub fn has_telemetry_case(&self) -> bool {
        self.grid.iter().any(|(_, model)| {
            model
                .tabs()
                .iter()
                .any(|tab| matches!(tab, PaneTab::TelemetryCase { .. }))
        })
    }

    /// Atualiza o título da aba de caso de telemetria em todos os panes.
    pub fn retitle_telemetry_case(&mut self, fingerprint: &str, title: &str) {
        for (_, model) in self.grid.iter_mut() {
            model.retitle_telemetry_case(fingerprint, title);
        }
    }

    /// Foca `pane` (`focus()` do Flutter); ignora ids que não existem mais.
    pub fn focus(&mut self, pane: Pane) {
        if self.grid.get(pane).is_some() {
            self.focused = pane;
        }
    }

    pub fn alloc_term_id(&mut self) -> TermId {
        let id = self.next_term_id;
        self.next_term_id = self.next_term_id.saturating_add(1);
        id
    }

    pub fn alloc_chat_id(&mut self) -> ChatId {
        let id = self.next_chat_id;
        self.next_chat_id = self.next_chat_id.saturating_add(1);
        id
    }

    pub fn alloc_query_id(&mut self) -> QueryId {
        let id = self.next_query_id;
        self.next_query_id = self.next_query_id.saturating_add(1);
        id
    }

    pub fn alloc_browser_id(&mut self) -> BrowserId {
        let id = self.next_browser_id;
        self.next_browser_id = self.next_browser_id.saturating_add(1);
        id
    }

    pub fn find_redis(&self, conn: &str) -> Option<(Pane, usize, BrowserId)> {
        self.grid.iter().find_map(|(&pane, model)| {
            model
                .tabs()
                .iter()
                .enumerate()
                .find_map(|(ix, tab)| match tab {
                    PaneTab::Redis { id, conn: c } if c == conn => Some((pane, ix, *id)),
                    _ => None,
                })
        })
    }

    pub fn find_mongo(&self, conn: &str, collection: &str) -> Option<(Pane, usize, BrowserId)> {
        self.grid.iter().find_map(|(&pane, model)| {
            model
                .tabs()
                .iter()
                .enumerate()
                .find_map(|(ix, tab)| match tab {
                    PaneTab::Mongo {
                        id,
                        conn: c,
                        collection: col,
                    } if c == conn && col == collection => Some((pane, ix, *id)),
                    _ => None,
                })
        })
    }

    /// Divide `pane` em `axis` com um pane novo depois dele (direita/abaixo), já
    /// com um terminal novo, e foca o novo (`splitPane` do Flutter). Devolve o
    /// id do terminal para o chamador subir a sessão.
    pub fn split_with_terminal(&mut self, pane: Pane, axis: Axis) -> Option<TermId> {
        self.grid.get(pane)?;
        let id = self.alloc_term_id();
        let mut model = PaneModel::empty();
        model.open_terminal(id);
        let (new_pane, _) = self.grid.split(axis, pane, model)?;
        self.focused = new_pane;
        Some(id)
    }

    /// Cria e foca um pane vazio à direita/abaixo para abrir uma aba não-terminal.
    pub fn split_empty(&mut self, pane: Pane, axis: Axis) -> Option<Pane> {
        self.grid.get(pane)?;
        let (new_pane, _) = self.grid.split(axis, pane, PaneModel::empty())?;
        self.focused = new_pane;
        Some(new_pane)
    }

    /// Fecha `pane` (`closePane` do Flutter): o irmão expande; sendo o único, ele
    /// fica com um terminal novo. Foco inválido vai para o primeiro pane do layout.
    pub fn close(&mut self, pane: Pane) -> Option<ClosedPane> {
        let closed = if self.grid.len() == 1 {
            let fresh = self.alloc_term_id();
            let mut model = PaneModel::empty();
            model.open_terminal(fresh);
            let old = std::mem::replace(self.grid.get_mut(pane)?, model);
            ClosedPane {
                tabs: old.tabs().to_vec(),
                fresh_terminal: Some(fresh),
            }
        } else {
            let (old, _sibling) = self.grid.close(pane)?;
            ClosedPane {
                tabs: old.tabs().to_vec(),
                fresh_terminal: None,
            }
        };
        self.ensure_focus_valid();
        Some(closed)
    }

    /// Pane que ficou sem abas some, se houver outro (`_closeTabIn` do Flutter).
    pub fn close_if_empty(&mut self, pane: Pane) {
        let empty = self.grid.get(pane).is_some_and(|m| m.tabs().is_empty());
        if empty && self.grid.len() > 1 {
            let _ = self.grid.close(pane);
            self.ensure_focus_valid();
        }
    }

    /// Redimensiona uma divisão (arrasto do divisor), com a proporção limitada.
    pub fn resize(&mut self, split: Split, ratio: f32) {
        self.grid.resize(split, ratio.clamp(MIN_RATIO, MAX_RATIO));
    }

    fn ensure_focus_valid(&mut self) {
        if self.grid.get(self.focused).is_none() {
            if let Some(first) = self.leaves().first() {
                self.focused = *first;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_split(tree: &PaneTree) -> Option<(Split, Axis, f32)> {
        match tree.grid.layout() {
            Node::Split {
                id, axis, ratio, ..
            } => Some((*id, *axis, *ratio)),
            Node::Pane(_) => None,
        }
    }

    #[test]
    fn nasce_com_um_pane_focado_com_terminal_1() {
        let tree = PaneTree::default();
        assert_eq!(tree.leaves(), vec![tree.focused()]);
        assert_eq!(tree.focused_model().tabs(), &[PaneTab::Terminal(1)]);
    }

    #[test]
    fn dividir_poe_o_novo_pane_depois_com_terminal_proprio_e_foca_o_novo() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        assert_eq!(tree.split_with_terminal(first, Axis::Vertical), Some(2));
        let leaves = tree.leaves();
        assert_eq!(leaves.len(), 2);
        assert_eq!(leaves.first(), Some(&first));
        assert_eq!(leaves.get(1), Some(&tree.focused()));
        assert_ne!(tree.focused(), first);
        assert_eq!(tree.focused_model().tabs(), &[PaneTab::Terminal(2)]);
        assert_eq!(
            tree.model(first).map(PaneModel::tabs),
            Some(&[PaneTab::Terminal(1)][..])
        );
        let (_, axis, ratio) = root_split(&tree).expect("raiz dividida");
        assert!(matches!(axis, Axis::Vertical));
        assert_eq!(ratio, 0.5);
    }

    #[test]
    fn split_empty_foca_pane_vazio_sem_abrir_terminal() {
        let mut tree = PaneTree::default();
        let original = tree.focused();
        let empty = tree.split_empty(original, Axis::Vertical);
        assert_eq!(empty, Some(tree.focused()));
        assert_ne!(tree.focused(), original);
        assert!(tree.focused_model().tabs().is_empty());
        assert_eq!(
            tree.model(original).map(PaneModel::tabs),
            Some(&[PaneTab::Terminal(1)][..])
        );
    }

    #[test]
    fn divisoes_aninham_nos_dois_eixos_com_ids_de_terminal_unicos() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.split_with_terminal(first, Axis::Vertical);
        let second = tree.focused();
        assert_eq!(tree.split_with_terminal(second, Axis::Horizontal), Some(3));
        assert_eq!(tree.split_with_terminal(first, Axis::Horizontal), Some(4));
        let ids: Vec<_> = tree
            .models()
            .iter()
            .flat_map(|m| m.tabs().to_vec())
            .collect();
        assert_eq!(
            ids,
            vec![
                PaneTab::Terminal(1),
                PaneTab::Terminal(4),
                PaneTab::Terminal(2),
                PaneTab::Terminal(3)
            ]
        );
        assert_eq!(tree.leaves().len(), 4);
    }

    #[test]
    fn fechar_pane_devolve_as_abas_e_o_irmao_expande() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.split_with_terminal(first, Axis::Vertical);
        let second = tree.focused();
        let closed = tree.close(second);
        assert_eq!(
            closed,
            Some(ClosedPane {
                tabs: vec![PaneTab::Terminal(2)],
                fresh_terminal: None
            })
        );
        assert_eq!(tree.leaves(), vec![first]);
        assert!(root_split(&tree).is_none());
        assert_eq!(tree.focused(), first);
    }

    #[test]
    fn fechar_pane_nao_focado_mantem_o_foco() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.split_with_terminal(first, Axis::Vertical);
        let second = tree.focused();
        tree.focus(first);
        tree.close(second);
        assert_eq!(tree.focused(), first);
    }

    #[test]
    fn fechar_o_focado_foca_o_primeiro_pane_do_layout() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.split_with_terminal(first, Axis::Vertical);
        let second = tree.focused();
        tree.split_with_terminal(second, Axis::Horizontal);
        let third = tree.focused();
        tree.focus(second);
        tree.close(second);
        // layout: [first | third]; o foco caiu e vai para o primeiro, não para o irmão
        assert_eq!(tree.leaves(), vec![first, third]);
        assert_eq!(tree.focused(), first);
    }

    #[test]
    fn fechar_o_unico_pane_troca_as_abas_por_um_terminal_novo() {
        let mut tree = PaneTree::default();
        let only = tree.focused();
        tree.focused_model_mut().open_file("/proj/a.rs");
        let closed = tree.close(only);
        assert_eq!(
            closed,
            Some(ClosedPane {
                tabs: vec![PaneTab::Terminal(1), PaneTab::File("/proj/a.rs".into())],
                fresh_terminal: Some(2)
            })
        );
        assert_eq!(tree.leaves(), vec![only]);
        assert_eq!(tree.focused_model().tabs(), &[PaneTab::Terminal(2)]);
    }

    #[test]
    fn fechar_pane_inexistente_nao_faz_nada() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.split_with_terminal(first, Axis::Vertical);
        let second = tree.focused();
        tree.close(second);
        assert_eq!(tree.close(second), None);
        assert_eq!(tree.split_with_terminal(second, Axis::Vertical), None);
        assert_eq!(tree.leaves().len(), 1);
    }

    #[test]
    fn pane_sem_abas_some_so_quando_ha_outro() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.focused_model_mut().close(0);
        tree.close_if_empty(first);
        assert_eq!(tree.leaves(), vec![first]);

        tree.focused_model_mut().open_terminal(9);
        tree.split_with_terminal(first, Axis::Horizontal);
        let second = tree.focused();
        tree.focus(first);
        tree.focused_model_mut().close(0);
        tree.close_if_empty(first);
        assert_eq!(tree.leaves(), vec![second]);
        assert_eq!(tree.focused(), second);
    }

    #[test]
    fn redimensionar_limita_a_proporcao() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.split_with_terminal(first, Axis::Vertical);
        let (split, _, _) = root_split(&tree).expect("raiz dividida");
        tree.resize(split, 0.3);
        assert_eq!(root_split(&tree).map(|s| s.2), Some(0.3));
        tree.resize(split, 0.0);
        assert_eq!(root_split(&tree).map(|s| s.2), Some(MIN_RATIO));
        tree.resize(split, 1.0);
        assert_eq!(root_split(&tree).map(|s| s.2), Some(MAX_RATIO));
    }

    #[test]
    fn pane_of_acha_o_pane_dono_da_aba() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.split_with_terminal(first, Axis::Vertical);
        assert_eq!(tree.pane_of(&PaneTab::Terminal(1)), Some(first));
        assert_eq!(tree.pane_of(&PaneTab::Terminal(2)), Some(tree.focused()));
        assert_eq!(tree.pane_of(&PaneTab::Settings), None);
    }

    #[test]
    fn mover_aba_entre_panes_preserva_label_de_terminal_osc() {
        let mut tree = PaneTree::default();
        let first = tree.focused();
        tree.focused_model_mut().open_terminal(9);
        tree.focused_model_mut()
            .set_terminal_label(9, "Agente".to_owned());
        tree.split_with_terminal(first, Axis::Vertical);
        let second = tree.focused();
        assert!(tree.move_tab(first, 1, second, 0));
        assert_eq!(
            tree.model(first).map(PaneModel::tabs),
            Some(&[PaneTab::Terminal(1)][..])
        );
        let target = tree.model(second).expect("pane destino existe");
        assert_eq!(target.tabs(), &[PaneTab::Terminal(9), PaneTab::Terminal(2)]);
        assert_eq!(target.display_title(&PaneTab::Terminal(9)), "Agente");
    }

    #[test]
    fn mover_aba_para_split_novo_respeita_lado_e_preserva_title_osc() {
        let mut tree = PaneTree::default();
        let source = tree.focused();
        tree.focused_model_mut().open_terminal(9);
        tree.focused_model_mut()
            .set_terminal_label(9, "Agente".to_owned());
        tree.split_with_terminal(source, Axis::Vertical);
        let target = tree.focused();
        let new_pane = tree
            .move_tab_to_split(source, 1, target, Axis::Horizontal, true)
            .expect("pane dividido");
        assert_eq!(tree.leaves(), vec![source, new_pane, target]);
        assert_eq!(
            tree.model(new_pane).map(PaneModel::tabs),
            Some(&[PaneTab::Terminal(9)][..])
        );
        assert_eq!(
            tree.model(new_pane)
                .map(|model| model.display_title(&PaneTab::Terminal(9))),
            Some("Agente".to_owned())
        );
    }

    #[test]
    fn split_do_unico_tab_no_proprio_pane_nao_altera_o_layout() {
        let mut tree = PaneTree::default();
        let only = tree.focused();
        assert_eq!(
            tree.move_tab_to_split(only, 0, only, Axis::Vertical, false),
            None
        );
        assert_eq!(tree.leaves(), vec![only]);
        assert_eq!(
            tree.model(only).map(PaneModel::tabs),
            Some(&[PaneTab::Terminal(1)][..])
        );
    }
}
