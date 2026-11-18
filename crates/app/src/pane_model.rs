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

/// Identificador numérico estável de uma aba de query (`.dbq`).
pub type QueryId = u64;

/// Identificador numérico estável de um navegador Redis ou MongoDB.
pub type BrowserId = u64;

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
    /// Grafo de agentes do workspace.
    AgentGraph(PathBuf),
    /// Configurações do aplicativo.
    Settings,
    /// Aba de query SQL (`.dbq`, arquivo ou untitled). O título muda quando o untitled vira arquivo.
    Query { id: QueryId, title: String },
    /// Navegador de chaves Redis.
    Redis { id: BrowserId, conn: String },
    /// Navegador de coleção MongoDB.
    Mongo {
        id: BrowserId,
        conn: String,
        collection: String,
    },
    /// Harness do workspace de `PathBuf`: o que o Claude Code carrega ali. Uma por workspace.
    Harness(PathBuf),
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
            Self::AgentGraph(_) => labels::AGENT_GRAPH_TAB_TITLE.to_string(),
            Self::Settings => labels::SETTINGS_TITLE.to_string(),
            Self::Query { title, .. } => title.clone(),
            Self::Redis { conn, .. } => conn.clone(),
            Self::Mongo {
                conn, collection, ..
            } => format!("{collection} ({conn})"),
            Self::Harness(_) => labels::HARNESS_TAB_TITLE.to_string(),
        }
    }

    /// Ícone tipográfico da aba (`>_` para terminal; `✦` para chat; resolvido pelo nome para arquivo).
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Terminal(_) => labels::ICON_TERMINAL,
            Self::Chat(_) => labels::ICON_CHAT,
            Self::Farm => labels::ICON_FARM,
            Self::AgentGraph(_) => labels::ICON_HUB_OUTLINED,
            Self::Settings => labels::ICON_SETTINGS,
            Self::Query { .. } => labels::ICON_DATABASE,
            Self::Redis { .. } => labels::ICON_GRID_ON,
            Self::Mongo { .. } => labels::ICON_DATA_OBJECT,
            Self::Harness(_) => labels::ICON_TUNE,
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
    terminal_labels: std::collections::BTreeMap<TermId, String>,
}

impl PaneModel {
    /// Cria um pane inicialmente sem abas. Os ids de terminal e chat vêm de
    /// `PaneTree`, que os aloca para a árvore inteira (sem colisão entre panes).
    pub fn empty() -> Self {
        Self {
            tabs: Vec::new(),
            active: None,
            terminal_labels: std::collections::BTreeMap::new(),
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

    pub fn display_title(&self, tab: &PaneTab) -> String {
        match tab {
            PaneTab::Terminal(id) => self
                .terminal_labels
                .get(id)
                .cloned()
                .unwrap_or_else(|| tab.title()),
            _ => tab.title(),
        }
    }

    pub fn set_terminal_label(&mut self, id: TermId, label: String) {
        if label.trim().is_empty() {
            self.terminal_labels.remove(&id);
        } else {
            self.terminal_labels.insert(id, label);
        }
    }

    /// Abre uma aba de terminal `id` ao final da lista e a torna ativa.
    pub fn open_terminal(&mut self, id: TermId) {
        self.tabs.push(PaneTab::Terminal(id));
        self.active = Some(self.tabs.len().saturating_sub(1));
    }

    /// Abre uma aba de chat `id` ao final da lista e a torna ativa.
    pub fn open_chat(&mut self, id: ChatId) {
        self.tabs.push(PaneTab::Chat(id));
        self.active = Some(self.tabs.len().saturating_sub(1));
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

    /// Abre ou ativa o grafo de agentes do workspace.
    pub fn open_agent_graph(&mut self, workspace: &Path) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| matches!(tab, PaneTab::AgentGraph(path) if path == workspace))
        {
            self.active = Some(index);
        } else {
            self.tabs.push(PaneTab::AgentGraph(workspace.to_path_buf()));
            self.active = Some(self.tabs.len().saturating_sub(1));
        }
    }

    /// Abre uma aba de query ao final da lista e a torna ativa.
    pub fn open_query(&mut self, id: QueryId, title: String) {
        self.tabs.push(PaneTab::Query { id, title });
        self.active = Some(self.tabs.len().saturating_sub(1));
    }

    /// Abre uma aba de navegador Redis ao final da lista e a torna ativa.
    pub fn open_redis(&mut self, id: BrowserId, conn: String) {
        self.tabs.push(PaneTab::Redis { id, conn });
        self.active = Some(self.tabs.len().saturating_sub(1));
    }

    /// Abre uma aba de coleção MongoDB ao final da lista e a torna ativa.
    pub fn open_mongo(&mut self, id: BrowserId, conn: String, collection: String) {
        self.tabs.push(PaneTab::Mongo {
            id,
            conn,
            collection,
        });
        self.active = Some(self.tabs.len().saturating_sub(1));
    }

    /// Índice da aba da query `id`, se estiver neste pane.
    pub fn query_index(&self, id: QueryId) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| matches!(tab, PaneTab::Query { id: open, .. } if *open == id))
    }

    /// Atualiza o título da aba da query `id` (o untitled virou arquivo).
    pub fn retitle_query(&mut self, id: QueryId, new_title: &str) {
        for tab in &mut self.tabs {
            if let PaneTab::Query { id: open, title } = tab {
                if *open == id && title != new_title {
                    new_title.clone_into(title);
                }
            }
        }
    }

    /// Abre ou ativa a aba Harness do workspace em `workspace`.
    pub fn open_harness(&mut self, workspace: &Path) {
        if let Some(index) = self
            .tabs
            .iter()
            .position(|tab| matches!(tab, PaneTab::Harness(path) if path == workspace))
        {
            self.active = Some(index);
        } else {
            self.tabs.push(PaneTab::Harness(workspace.to_path_buf()));
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
        if let PaneTab::Terminal(id) = &removed {
            self.terminal_labels.remove(id);
        }
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

    fn with_terminal() -> PaneModel {
        let mut model = PaneModel::empty();
        model.open_terminal(1);
        model
    }

    #[test]
    fn abre_terminais_com_o_id_informado_e_ativa_o_ultimo() {
        let mut model = with_terminal();
        assert_eq!(model.tabs(), &[PaneTab::Terminal(1)]);
        assert_eq!(model.active_index(), Some(0));
        assert_eq!(model.active_tab(), Some(&PaneTab::Terminal(1)));

        model.open_terminal(2);
        assert_eq!(model.tabs().len(), 2);
        assert_eq!(model.active_index(), Some(1));
        assert_eq!(model.active_tab(), Some(&PaneTab::Terminal(2)));
    }

    #[test]
    fn abrir_arquivo_ja_aberto_apenas_ativa_sem_duplicar() {
        let mut model = with_terminal();
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
        let mut model = with_terminal();
        model.open_file("/proj/Cargo.toml");
        model.open_file("/proj/README.md");
        model.open_terminal(2);
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

        let mut model = with_terminal();
        model.open_chat(1);
        assert_eq!(model.active_tab(), Some(&PaneTab::Chat(1)));

        let harness = PaneTab::Harness(PathBuf::from("/proj"));
        assert_eq!(harness.title(), "Harness");
        assert_eq!(harness.icon(), labels::ICON_TUNE);
    }

    #[test]
    fn harness_abre_uma_aba_por_workspace() {
        let mut model = with_terminal();
        model.open_harness(Path::new("/a"));
        model.open_harness(Path::new("/b"));
        assert_eq!(model.tabs().len(), 3);
        model.activate(0);
        model.open_harness(Path::new("/a"));
        assert_eq!(model.tabs().len(), 3);
        assert_eq!(
            model.active_tab(),
            Some(&PaneTab::Harness(PathBuf::from("/a")))
        );
    }
}
