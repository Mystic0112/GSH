//! Menu de contexto da aba, fechar em lote e renomear inline (`_showTabMenu`, `_closeTabs`,
//! `_handleTap` e `_commitEdit` de `pane_view.dart`).
//!
//! A parte pura (quais itens aparecem, quais abas cada "fechar" alcança, o nome digitado) fica
//! em funções soltas; o que mexe no [`Shell`] fica no `impl` ao final.

use std::time::{Duration, Instant};

use iced::widget::pane_grid::Pane;
use iced::Task;

use crate::pane_model::{PaneModel, PaneTab};
use crate::pane_strip::TabMenuEntry;
use crate::{labels, pane_id_for_term, DbQuery, Msg, Shell, TermId};

/// Janela do duplo clique (`Duration(milliseconds: 300)` de `_handleTap`).
const DOUBLE_CLICK: Duration = Duration::from_millis(300);

/// Ação escolhida no menu da aba (os `value` de `_showTabMenu`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabAction {
    Rename,
    ResetLabel,
    CopyId,
    Restart,
    Close,
    CloseOthers,
    CloseToTheRight,
    CloseAll,
}

impl TabAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Rename => labels::PANE_TAB_RENAME,
            Self::ResetLabel => labels::PANE_TAB_RESET_TITLE,
            Self::CopyId => labels::PANE_TAB_COPY_ID,
            Self::Restart => labels::PANE_TAB_RESTART,
            Self::Close => labels::PANE_TAB_CLOSE,
            Self::CloseOthers => labels::PANE_TAB_CLOSE_OTHERS,
            Self::CloseToTheRight => labels::PANE_TAB_CLOSE_TO_THE_RIGHT,
            Self::CloseAll => labels::PANE_TAB_CLOSE_ALL,
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Self::Rename => labels::ICON_EDIT,
            Self::ResetLabel | Self::Restart => labels::ICON_RESTART_ALT,
            Self::CopyId => labels::ICON_CONTENT_COPY,
            Self::Close => labels::ICON_CLOSE,
            Self::CloseOthers => labels::ICON_TAB_CLOSE_INACTIVE,
            Self::CloseToTheRight => labels::ICON_TAB_CLOSE_RIGHT,
            Self::CloseAll => labels::ICON_TAB_CLOSE,
        }
    }
}

/// Itens do menu da aba `index`, na ordem do Flutter. Item que não se aplica some em vez de
/// ficar cinza (`if (widget.onCloseOthers != null)` etc.).
///
/// `label_locked` é o `titleLocked` (só terminal com nome manual oferece "Redefinir título").
pub fn menu_actions(
    tab: &PaneTab,
    index: usize,
    tab_count: usize,
    label_locked: bool,
) -> Vec<TabAction> {
    let mut actions = Vec::new();
    if tab.is_terminal() {
        actions.push(TabAction::Rename);
        if label_locked {
            actions.push(TabAction::ResetLabel);
        }
        actions.push(TabAction::CopyId);
        actions.push(TabAction::Restart);
    }
    actions.push(TabAction::Close);
    // na única aba "fechar as outras" não faria nada e "fechar todas" seria o próprio "fechar"
    if tab_count > 1 {
        actions.push(TabAction::CloseOthers);
    }
    if index + 1 < tab_count {
        actions.push(TabAction::CloseToTheRight);
    }
    if tab_count > 1 {
        actions.push(TabAction::CloseAll);
    }
    actions
}

/// Abas que cada ação de fechar em lote alcança, na ordem visual.
pub fn batch_targets(action: TabAction, index: usize, tabs: &[PaneTab]) -> Vec<PaneTab> {
    tabs.iter()
        .enumerate()
        .filter(|(position, _)| match action {
            TabAction::CloseOthers => *position != index,
            TabAction::CloseToTheRight => *position > index,
            TabAction::CloseAll => true,
            _ => false,
        })
        .map(|(_, tab)| tab.clone())
        .collect()
}

/// Nome digitado: sem bordas e com `-` no lugar do espaço (`_commitEdit`). Vazio = `None`.
pub fn committed_name(text: &str) -> Option<String> {
    let name = text.trim().replace(' ', "-");
    (!name.is_empty()).then_some(name)
}

/// Edição inline do nome de uma aba de terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabRename {
    pub pane: Pane,
    pub term: TermId,
    pub text: String,
}

/// Fechar em lote aguardando a confirmação "Fechar abas?".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCloseTabs {
    pub pane: Pane,
    pub tabs: Vec<PaneTab>,
}

/// Entradas do menu prontas para a barra de abas.
pub fn menu_entries(pane: Pane, index: usize, model: &PaneModel) -> Vec<TabMenuEntry<Msg>> {
    let Some(tab) = model.tabs().get(index) else {
        return Vec::new();
    };
    let locked = match tab {
        PaneTab::Terminal(id) => model.terminal_label(*id).is_some(),
        _ => false,
    };
    menu_actions(tab, index, model.tabs().len(), locked)
        .into_iter()
        .map(|action| TabMenuEntry {
            label: action.label(),
            glyph: action.glyph(),
            message: Msg::TabMenuAction(pane, index, action),
        })
        .collect()
}

impl Shell {
    /// Terminal em edição do nome ou diálogo de fechar abas aberto: o teclado não chega ao
    /// terminal.
    pub(crate) fn tab_modal_open(&self) -> bool {
        self.tab_rename.is_some() || self.pending_close_tabs.is_some()
    }

    /// Duplo clique numa aba de terminal (`_handleTap`): o segundo clique em menos de 300 ms
    /// na mesma aba renomeia em vez de selecionar. Devolve `true` quando consumiu o clique.
    pub(crate) fn register_tab_click(&mut self, pane: Pane, index: usize, now: Instant) -> bool {
        let Some(PaneTab::Terminal(id)) = self
            .panes
            .model(pane)
            .and_then(|model| model.tabs().get(index))
        else {
            self.last_tab_click = None;
            return false;
        };
        let id = *id;
        let double = self.last_tab_click.is_some_and(|(last_pane, last_id, at)| {
            last_pane == pane && last_id == id && now.duration_since(at) < DOUBLE_CLICK
        });
        self.last_tab_click = if double { None } else { Some((pane, id, now)) };
        double
    }

    /// Título exibido da aba de terminal: nome manual, senão o título dinâmico do terminal
    /// (`displayTitle` = `manualLabel ?? title`).
    fn terminal_display_title(&self, model: &PaneModel, id: TermId) -> String {
        model
            .terminal_label(id)
            .map(str::to_owned)
            .or_else(|| self.terminals.get(&id).and_then(|term| term.title()))
            .unwrap_or_else(|| PaneTab::Terminal(id).title())
    }

    /// `_startEditing`: semeia o campo com o nome exibido, selecionado por inteiro, e foca.
    pub(crate) fn start_tab_rename(&mut self, pane: Pane, index: usize) -> Task<Msg> {
        let Some(model) = self.panes.model(pane) else {
            return Task::none();
        };
        let Some(PaneTab::Terminal(id)) = model.tabs().get(index) else {
            return Task::none();
        };
        let id = *id;
        let text = self.terminal_display_title(model, id);
        self.tab_rename = Some(TabRename {
            pane,
            term: id,
            text,
        });
        Task::batch([
            iced::widget::operation::focus(crate::pane_tab::RENAME_INPUT_ID),
            iced::widget::operation::select_all(crate::pane_tab::RENAME_INPUT_ID),
        ])
    }

    /// Grava (ou, com `None`, descarta) o nome manual do terminal `id` nos dois lugares onde o
    /// app o guarda: o modelo do pane (layout salvo) e o mapa que a CLI usa para achar a aba.
    pub(crate) fn apply_terminal_label(&mut self, id: TermId, label: Option<String>) {
        let Some(pane) = self.panes.pane_of(&PaneTab::Terminal(id)) else {
            return;
        };
        let Some(model) = self.panes.model_mut(pane) else {
            return;
        };
        match label {
            Some(label) => {
                model.set_terminal_label(id, label.clone());
                self.terminal_labels.insert(id, label);
            }
            None => {
                model.set_terminal_label(id, String::new());
                self.terminal_labels.remove(&id);
            }
        }
        self.sync_island_from_status();
    }

    /// `_commitEdit`: nome vazio não muda nada.
    pub(crate) fn commit_tab_rename(&mut self) {
        let Some(rename) = self.tab_rename.take() else {
            return;
        };
        if let Some(name) = committed_name(&rename.text) {
            self.apply_terminal_label(rename.term, Some(name));
        }
    }

    /// Aplica a ação escolhida no menu da aba `index` do `pane`.
    pub(crate) fn tab_menu_action(
        &mut self,
        pane: Pane,
        index: usize,
        action: TabAction,
    ) -> Task<Msg> {
        self.tab_menu = None;
        let Some(model) = self.panes.model(pane) else {
            return Task::none();
        };
        let Some(tab) = model.tabs().get(index).cloned() else {
            return Task::none();
        };
        match (action, &tab) {
            (TabAction::Rename, _) => self.start_tab_rename(pane, index),
            (TabAction::ResetLabel, PaneTab::Terminal(id)) => {
                self.apply_terminal_label(*id, None);
                Task::none()
            }
            (TabAction::CopyId, PaneTab::Terminal(id)) => {
                iced::clipboard::write(pane_id_for_term(*id))
            }
            (TabAction::Restart, PaneTab::Terminal(id)) => {
                self.restart_terminal(*id);
                Task::none()
            }
            (TabAction::Close, _) => self.update_inner(Msg::CloseTabIn(pane, index)),
            (TabAction::CloseOthers | TabAction::CloseToTheRight | TabAction::CloseAll, _) => {
                let tabs = batch_targets(action, index, model.tabs());
                if !tabs.is_empty() {
                    self.pending_close_tabs = Some(PendingCloseTabs { pane, tabs });
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// "Reiniciar": processo novo na mesma aba (mesmo id, nome manual e posição), no diretório
    /// com que a aba foi aberta, com o status do agente zerado.
    ///
    /// Diferente do Flutter, o Rust não guarda cwd vivo (OSC 7), scrollback para replay nem o
    /// id de sessão do harness: a aba volta com a tela limpa e sem `--resume`.
    pub(crate) fn restart_terminal(&mut self, id: TermId) {
        let Some(term) = self.terminals.get(&id) else {
            return;
        };
        if term.is_remote() {
            return;
        }
        let cwd = term.cwd().map(std::path::Path::to_path_buf);
        self.clear_terminal_runtime(id);
        // o tamanho vem do canvas no próximo layout, como em qualquer terminal novo
        let fresh = self.spawn_terminal_for_dir(id, cwd.as_deref());
        // o `insert` descarta o terminal antigo, que encerra o processo dele
        self.terminals.insert(id, fresh);
        self.sync_island_from_status();
    }

    /// Fecha as abas confirmadas, em lote. As limpas saem de uma vez ([`PaneModel::close_many`]);
    /// arquivo ou query com alteração não salva fica aberto e a primeira delas pergunta o que
    /// fazer (o `requestCloseTab` do Flutter, que também para no "Cancelar").
    pub(crate) fn close_tabs(&mut self, pending: PendingCloseTabs) -> Task<Msg> {
        let PendingCloseTabs { pane, tabs } = pending;
        self.panes.focus(pane);
        for editor in self.editors.values_mut() {
            editor.state.cancel_close();
        }
        let Some(model) = self.panes.model(pane) else {
            return Task::none();
        };
        let (dirty, clean): (Vec<PaneTab>, Vec<PaneTab>) = tabs
            .into_iter()
            .filter(|tab| model.tabs().contains(tab))
            .partition(|tab| self.tab_has_unsaved_changes(tab));
        let indices = clean
            .iter()
            .filter_map(|tab| model.tabs().iter().position(|open| open == tab))
            .collect::<Vec<_>>();
        let removed = self
            .panes
            .model_mut(pane)
            .map(|model| model.close_many(&indices))
            .unwrap_or_default();
        for tab in removed {
            self.dispose_tab(tab);
        }
        let follow_up = self.after_tab_close();
        let first_dirty = dirty.first().and_then(|tab| {
            self.panes
                .model(pane)
                .and_then(|model| model.tabs().iter().position(|open| open == tab))
        });
        match first_dirty {
            Some(index) => Task::batch([follow_up, self.update_inner(Msg::CloseTab(index))]),
            None => follow_up,
        }
    }

    fn tab_has_unsaved_changes(&self, tab: &PaneTab) -> bool {
        match tab {
            PaneTab::File(path) => self
                .editors
                .get(path)
                .is_some_and(|editor| editor.state.buffer.is_dirty()),
            PaneTab::Query { id, .. } => self.queries.get(id).is_some_and(DbQuery::dirty),
            _ => false,
        }
    }

    /// Depois de fechar abas do pane focado: pane vazio some (se houver outro) ou vira terminal
    /// (`_closeTabIn`).
    pub(crate) fn after_tab_close(&mut self) -> Task<Msg> {
        self.panes.close_if_empty(self.panes.focused());
        if self
            .panes
            .model(self.panes.focused())
            .is_some_and(|model| model.tabs().is_empty())
        {
            return self.update_inner(Msg::NewTerminalTab);
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tabs() -> Vec<PaneTab> {
        vec![
            PaneTab::Terminal(1),
            PaneTab::File("/a.rs".into()),
            PaneTab::Terminal(2),
        ]
    }

    #[test]
    fn terminal_oferece_renomear_copiar_id_e_reiniciar_antes_de_fechar() {
        assert_eq!(
            menu_actions(&PaneTab::Terminal(1), 1, 3, false),
            vec![
                TabAction::Rename,
                TabAction::CopyId,
                TabAction::Restart,
                TabAction::Close,
                TabAction::CloseOthers,
                TabAction::CloseToTheRight,
                TabAction::CloseAll,
            ]
        );
    }

    #[test]
    fn redefinir_titulo_so_aparece_com_nome_manual() {
        let actions = menu_actions(&PaneTab::Terminal(1), 0, 2, true);
        assert_eq!(actions.get(1), Some(&TabAction::ResetLabel));
        assert!(!menu_actions(&PaneTab::Terminal(1), 0, 2, false).contains(&TabAction::ResetLabel));
    }

    #[test]
    fn unica_aba_so_oferece_fechar_e_ultima_nao_fecha_a_direita() {
        assert_eq!(
            menu_actions(&PaneTab::Terminal(1), 0, 1, false),
            vec![
                TabAction::Rename,
                TabAction::CopyId,
                TabAction::Restart,
                TabAction::Close
            ]
        );
        let last = menu_actions(&PaneTab::Terminal(2), 2, 3, false);
        assert!(!last.contains(&TabAction::CloseToTheRight));
        assert!(last.contains(&TabAction::CloseOthers));
    }

    #[test]
    fn aba_que_nao_e_terminal_so_tem_os_itens_de_fechar() {
        assert_eq!(
            menu_actions(&PaneTab::File("/a.rs".into()), 1, 3, false),
            vec![
                TabAction::Close,
                TabAction::CloseOthers,
                TabAction::CloseToTheRight,
                TabAction::CloseAll,
            ]
        );
    }

    #[test]
    fn rotulos_e_icones_do_menu_sao_os_do_flutter() {
        let all = [
            (TabAction::Rename, "Renomear", "\u{f097}"),
            (TabAction::ResetLabel, "Redefinir título", "\u{f053}"),
            (TabAction::CopyId, "Copiar Id", "\u{e14d}"),
            (TabAction::Restart, "Reiniciar", "\u{f053}"),
            (TabAction::Close, "Fechar", "\u{e5cd}"),
            (TabAction::CloseOthers, "Fechar as outras", "\u{f3d0}"),
            (TabAction::CloseToTheRight, "Fechar à direita", "\u{f746}"),
            (TabAction::CloseAll, "Fechar todas", "\u{f745}"),
        ];
        for (action, label, glyph) in all {
            assert_eq!((action.label(), action.glyph()), (label, glyph));
        }
    }

    #[test]
    fn alvos_do_fechar_em_lote() {
        let open = tabs();
        assert_eq!(
            batch_targets(TabAction::CloseOthers, 1, &open),
            vec![PaneTab::Terminal(1), PaneTab::Terminal(2)]
        );
        assert_eq!(
            batch_targets(TabAction::CloseToTheRight, 1, &open),
            vec![PaneTab::Terminal(2)]
        );
        assert_eq!(batch_targets(TabAction::CloseAll, 1, &open), open);
        assert!(batch_targets(TabAction::Close, 1, &open).is_empty());
    }

    #[test]
    fn nome_confirmado_apara_troca_espaco_por_hifen_e_ignora_vazio() {
        assert_eq!(
            committed_name("  meu agente  "),
            Some("meu-agente".to_owned())
        );
        assert_eq!(committed_name("   "), None);
        assert_eq!(committed_name(""), None);
    }
}
