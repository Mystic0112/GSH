//! Árvore de panes do centro (`pane_node.dart` e `closePane`/`splitPane` de
//! `gsh_viewmodel.dart` no Flutter).
//!
//! Árvore binária de divisões aninháveis, cada folha com um [`PaneModel`] (abas).
//! A estrutura (eixo, proporção, "o irmão expande ao fechar") é a do
//! `pane_grid::State` do iced, que é dado puro: nada aqui precisa de janela nem
//! de renderer. `Axis::Vertical` = lado a lado (`SplitDir.vertical` do Flutter),
//! `Axis::Horizontal` = empilhado (`SplitDir.horizontal`).

use iced::widget::pane_grid::{self, Axis, Node, Pane, Split};

use crate::pane_model::{ChatId, PaneModel, PaneTab, TermId};

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
        }
    }
}

impl PaneTree {
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
}
