//! Modelo puro de abas do pane central (`LeafPane` em `pane_node.dart` e
//! `PaneItem` em `pane_item.dart` do app Flutter).
//!
//! Mantém a lista ordenada de abas tipadas (`Terminal(id)` | `File(path)`),
//! o índice da aba ativa e as transições puras de abrir, ativar e fechar.

use std::path::{Path, PathBuf};

use gsh_core::tabs::basename;

use crate::labels;

/// Identificador numérico estável de uma sessão de terminal no pane.
pub type TermId = u64;

/// Identificador numérico estável de uma sessão de chat no pane.
pub type ChatId = u64;

/// Aba aberta no pane central.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneTab {
    /// Sessão de terminal identificada por [`TermId`].
    Terminal(TermId),
    /// Sessão de chat com o agente identificada por [`ChatId`].
    Chat(ChatId),
    /// Viewer de arquivo identificado pelo caminho no sistema de arquivos.
    File(PathBuf),
    /// Cenário estático da fazenda ou escritório.
    Farm,
    /// Configurações do aplicativo.
    Settings,
}

impl PaneTab {
    /// Retorna `true` quando a aba é de terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal(_))
    }

    /// Retorna `true` quando a aba é de chat.
    pub fn is_chat(&self) -> bool {
        matches!(self, Self::Chat(_))
    }

    /// Retorna `true` quando a aba é de arquivo.
    pub fn is_file(&self) -> bool {
        matches!(self, Self::File(_))
    }

    /// Título exibido na barra de abas (`"Terminal"`, `"Chat"` ou o basename do arquivo).
    pub fn title(&self) -> String {
        match self {
            Self::Terminal(_) => labels::DEFAULT_TAB_TITLE.to_string(),
            Self::Chat(_) => labels::CHAT_TAB_TITLE.to_string(),
            Self::File(path) => file_display_name(path),
            Self::Farm => labels::FARM_TAB_TITLE.to_string(),
            Self::Settings => labels::SETTINGS_TITLE.to_string(),
        }
    }

    /// Ícone tipográfico da aba (`>_` para terminal; `✦` para chat; resolvido pelo nome para arquivo).
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Terminal(_) => labels::ICON_TERMINAL,
            Self::Chat(_) => labels::ICON_CHAT,
            Self::Farm => labels::ICON_PLUS,
            Self::Settings => labels::ICON_SETTINGS,
            Self::File(path) => {
                let name = file_display_name(path);
                labels::file_icon_for_name(&name)
            }
        }
    }
}

/// Extrai o nome do arquivo para exibição na aba.
pub fn file_display_name(path: &Path) -> String {
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

/// Estado puro da faixa de abas do pane central.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneModel {
    tabs: Vec<PaneTab>,
    active: Option<usize>,
    next_term_id: TermId,
    next_chat_id: ChatId,
}

impl Default for PaneModel {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneModel {
    /// Inicia o pane com uma aba de terminal ativa (`Terminal(1)`).
    pub fn new() -> Self {
        Self {
            tabs: vec![PaneTab::Terminal(1)],
            active: Some(0),
            next_term_id: 2,
            next_chat_id: 1,
        }
    }

    /// Cria um pane inicialmente sem abas.
    pub fn empty() -> Self {
        Self {
            tabs: Vec::new(),
            active: None,
            next_term_id: 1,
            next_chat_id: 1,
        }
    }

    /// Lista ordenada de abas abertas no pane.
    pub fn tabs(&self) -> &[PaneTab] {
        &self.tabs
    }

    /// Índice da aba atualmente ativa, ou `None` quando o pane está vazio.
    pub fn active_index(&self) -> Option<usize> {
        self.active
    }

    /// Referência para a aba atualmente ativa.
    pub fn active_tab(&self) -> Option<&PaneTab> {
        self.active.and_then(|idx| self.tabs.get(idx))
    }

    /// Abre uma nova aba de terminal ao final da lista, torna-a ativa e devolve seu [`TermId`].
    pub fn open_terminal(&mut self) -> TermId {
        let id = self.next_term_id;
        self.next_term_id = self.next_term_id.saturating_add(1);
        self.tabs.push(PaneTab::Terminal(id));
        self.active = Some(self.tabs.len().saturating_sub(1));
        id
    }

    /// Abre uma nova aba de chat ao final da lista, torna-a ativa e devolve seu [`ChatId`].
    pub fn open_chat(&mut self) -> ChatId {
        let id = self.next_chat_id;
        self.next_chat_id = self.next_chat_id.saturating_add(1);
        self.tabs.push(PaneTab::Chat(id));
        self.active = Some(self.tabs.len().saturating_sub(1));
        id
    }

    /// Abre ou ativa a aba única do cenário.
    pub fn open_farm(&mut self) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| matches!(tab, PaneTab::Farm))
        {
            self.active = Some(index);
        } else {
            self.tabs.push(PaneTab::Farm);
            self.active = Some(self.tabs.len().saturating_sub(1));
        }
    }

    /// Abre ou ativa a aba única de configurações.
    pub fn open_settings(&mut self) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| matches!(tab, PaneTab::Settings))
        {
            self.active = Some(index);
        } else {
            self.tabs.push(PaneTab::Settings);
            self.active = Some(self.tabs.len().saturating_sub(1));
        }
    }

    /// Abre um arquivo como aba: se já estiver aberto, apenas ativa a aba existente;
    /// caso contrário adiciona ao final e ativa. Devolve o índice ativo.
    pub fn open_file(&mut self, path: impl Into<PathBuf>) -> usize {
        let path_buf = path.into();
        if let Some(existing_idx) = self
            .tabs
            .iter()
            .position(|tab| matches!(tab, PaneTab::File(p) if p == &path_buf))
        {
            self.active = Some(existing_idx);
            return existing_idx;
        }

        self.tabs.push(PaneTab::File(path_buf));
        let idx = self.tabs.len().saturating_sub(1);
        self.active = Some(idx);
        idx
    }

    /// Ativa a aba em `index` se estiver dentro dos limites.
    pub fn activate(&mut self, index: usize) -> Option<&PaneTab> {
        if index < self.tabs.len() {
            self.active = Some(index);
            self.tabs.get(index)
        } else {
            None
        }
    }

    /// Fecha a aba em `index`, ajustando a aba ativa e retornando a aba removida.
    pub fn close(&mut self, index: usize) -> Option<PaneTab> {
        if index >= self.tabs.len() {
            return None;
        }
        let removed = self.tabs.remove(index);
        self.active = if self.tabs.is_empty() {
            None
        } else if let Some(curr) = self.active {
            if index < curr {
                Some(curr.saturating_sub(1))
            } else if index == curr {
                Some(curr.min(self.tabs.len().saturating_sub(1)))
            } else {
                Some(curr)
            }
        } else {
            Some(0)
        };
        Some(removed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inicia_com_terminal_ativo_e_abre_novos_terminais_com_id_incremental() {
        let mut model = PaneModel::new();
        assert_eq!(model.tabs(), &[PaneTab::Terminal(1)]);
        assert_eq!(model.active_index(), Some(0));
        assert_eq!(model.active_tab(), Some(&PaneTab::Terminal(1)));

        let id2 = model.open_terminal();
        assert_eq!(id2, 2);
        assert_eq!(model.tabs().len(), 2);
        assert_eq!(model.active_index(), Some(1));
        assert_eq!(model.active_tab(), Some(&PaneTab::Terminal(2)));
    }

    #[test]
    fn abrir_arquivo_ja_aberto_apenas_ativa_sem_duplicar() {
        let mut model = PaneModel::new();
        let idx1 = model.open_file("/proj/src/main.rs");
        assert_eq!(idx1, 1);
        assert_eq!(model.tabs().len(), 2);
        assert_eq!(model.active_index(), Some(1));

        // Volta para o terminal e reabre o mesmo arquivo
        model.activate(0);
        assert_eq!(model.active_index(), Some(0));

        let idx_reopen = model.open_file("/proj/src/main.rs");
        assert_eq!(idx_reopen, 1);
        assert_eq!(model.tabs().len(), 2);
        assert_eq!(model.active_index(), Some(1));
    }

    #[test]
    fn fechar_abas_preserva_ou_ajusta_aba_ativa_corretamente() {
        let mut model = PaneModel::new();
        model.open_file("/proj/Cargo.toml");
        model.open_file("/proj/README.md");
        model.open_terminal();
        // Abas: [0: Terminal(1), 1: Cargo.toml, 2: README.md, 3: Terminal(2)], ativa = 3
        assert_eq!(model.tabs().len(), 4);
        assert_eq!(model.active_index(), Some(3));

        // Fecha aba anterior à ativa (1: Cargo.toml) -> ativa desloca de 3 para 2
        let removed = model.close(1);
        assert_eq!(
            removed,
            Some(PaneTab::File(PathBuf::from("/proj/Cargo.toml")))
        );
        assert_eq!(model.active_index(), Some(2));
        assert_eq!(model.active_tab(), Some(&PaneTab::Terminal(2)));

        // Fecha a própria aba ativa no fim (2: Terminal(2)) -> ativa recua para 1 (README.md)
        let removed_term = model.close(2);
        assert_eq!(removed_term, Some(PaneTab::Terminal(2)));
        assert_eq!(model.active_index(), Some(1));
        assert_eq!(
            model.active_tab(),
            Some(&PaneTab::File(PathBuf::from("/proj/README.md")))
        );

        // Fecha as restantes até esvaziar
        assert_eq!(model.close(0), Some(PaneTab::Terminal(1)));
        assert_eq!(model.active_index(), Some(0));
        assert_eq!(
            model.close(0),
            Some(PaneTab::File(PathBuf::from("/proj/README.md")))
        );
        assert_eq!(model.active_index(), None);
        assert!(model.tabs().is_empty());
        assert_eq!(model.close(0), None);
    }

    #[test]
    fn titulo_e_icone_seguem_tipo_da_aba_e_nome_do_arquivo() {
        let term = PaneTab::Terminal(1);
        assert_eq!(term.title(), labels::DEFAULT_TAB_TITLE);
        assert_eq!(term.icon(), labels::ICON_TERMINAL);
        assert!(term.is_terminal());

        let rs_file = PaneTab::File(PathBuf::from("/proj/src/lib.rs"));
        assert_eq!(rs_file.title(), "lib.rs");
        assert_eq!(rs_file.icon(), labels::ICON_FILE_CODE);
        assert!(rs_file.is_file());

        let toml_file = PaneTab::File(PathBuf::from("/proj/Cargo.toml"));
        assert_eq!(toml_file.title(), "Cargo.toml");
        assert_eq!(toml_file.icon(), labels::ICON_FILE_CONFIG);

        let chat = PaneTab::Chat(1);
        assert_eq!(chat.title(), labels::CHAT_TAB_TITLE);
        assert_eq!(chat.icon(), labels::ICON_CHAT);
        assert!(chat.is_chat());

        let mut model = PaneModel::new();
        let cid = model.open_chat();
        assert_eq!(cid, 1);
        assert_eq!(model.active_tab(), Some(&PaneTab::Chat(1)));
    }
}
