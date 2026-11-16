// Shell da UI do GSH no Linux (iced). Layout de três colunas com a barra de
// título e ilha de agentes no topo, espelhando o GSH Desktop (Flutter):
// rail de workspaces à esquerda, pane central com abas (terminais e viewers
// de arquivo) no centro e painel de arquivos à direita.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod agent_file_marks;
pub mod agent_island_panel;
pub mod agent_status;
pub mod chat_composer;
pub mod chat_pane;
pub mod chat_view;
pub mod claude_hook_installer;
pub mod clock;
pub mod content_search_panel;
pub mod content_search_state;
pub mod db_pane;
pub mod editor_gutter;
pub mod editor_highlight;
pub mod editor_keys;
pub mod editor_pane;
pub mod editor_scroll;
pub mod editor_state;
pub mod editor_view;
pub mod farm;
mod file_icons;
pub mod file_ops;
pub mod file_tree;
pub mod file_viewer;
pub mod files_panel;
pub mod gallery_create;
pub mod gallery_panel;
pub mod gallery_template;
pub mod git_changes_view;
pub mod git_diff_view;
pub mod git_file_row;
pub mod git_header;
pub mod git_history_view;
pub mod git_ops;
pub mod git_panel;
pub mod git_presenter;
pub mod git_state;
pub mod gorilla;
pub mod harness_pane;
pub mod island;
pub mod labels;
pub mod pane_chrome;
pub mod pane_close_dialog;
pub mod pane_focus;
pub mod pane_model;
pub mod pane_strip;
pub mod pane_tab;
pub mod pane_tool_icon;
pub mod pane_tree;
pub mod panel_widgets;
pub mod rail;
pub mod right_tabs;
pub mod settings;
pub mod terminal_canvas;
pub mod terminal_grid;
pub mod terminal_input;
pub mod terminal_pane;
pub mod theme;
pub mod topbar;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::Duration;

use gsh_core::workspace::WorkspaceList;
use iced::widget::pane_grid::{self, Axis, Pane};
use iced::widget::{column, container, mouse_area, row, stack, Space};
use iced::{Background, Element, Fill, Padding, Subscription, Task, Theme};

use crate::agent_file_marks::{AgentFiles, AgentTab};
use crate::agent_status::{
    now_epoch_ms, pane_id_for_term, term_id_from_pane_id, PaneDescriptor, StatusRuntime,
};
use crate::chat_pane::{ChatMsg, ChatPane};
use crate::content_search_panel::SearchMsg;
use crate::content_search_state::SearchState;
use crate::file_tree::{FileTree, ViewerHook};
use crate::file_viewer::FileViewerState;
use crate::files_panel::FilesMsg;
use crate::gallery_create::GalleryError;
use crate::gallery_panel::GalleryMsg;
use crate::island::{AgentIsland, IslandMsg, PermissionDecision};
use crate::pane_chrome::ChromeCallbacks;
use crate::pane_model::{ChatId, PaneModel, PaneTab, TermId};
use crate::pane_tree::PaneTree;
use crate::right_tabs::RightTab;
use crate::terminal_input::{classify_key_press, KeyAction};
use crate::terminal_pane::{TerminalMsg, TerminalPane};
use crate::topbar::TopbarProps;

/// Intervalo de polling do canal de status na `Subscription` do `Shell` (50 ms).
const STATUS_POLL_INTERVAL_MS: u64 = 50;

/// Quadro do spinner da busca (~30 fps).
const SEARCH_SPIN_TICK_MS: u64 = 33;

/// Espera do debounce da busca (200 ms) sem ocupar a thread da UI.
async fn search_debounce() {
    let _ = tokio::task::spawn_blocking(|| {
        std::thread::sleep(Duration::from_millis(content_search_state::DEBOUNCE_MS));
    })
    .await;
}

#[derive(Debug, Clone)]
enum Msg {
    Island(IslandMsg),
    /// Evento da `Subscription` (tique e teclado): o teclado vale para o terminal focado.
    Terminal(TerminalMsg),
    /// Evento do canvas de um terminal específico (redimensionar, rolar, selecionar).
    TerminalIn(TermId, TerminalMsg),
    Chat(ChatId, ChatMsg),
    Files(FilesMsg),
    Git(git_panel::GitMsg),
    SelectRightTab(RightTab),
    Search(SearchMsg),
    /// Ctrl/Cmd+Shift+F: mostra o painel direito na aba Busca e foca o campo.
    FocusSearch,
    Gallery(GalleryMsg),
    GalleryCreated(Result<PathBuf, GalleryError>),
    DismissGalleryError,
    GitResult(git_state::ResultMsg),
    Editor(PathBuf, editor_pane::EditorMsg),
    WindowCloseRequested(iced::window::Id),
    ConfirmWindowClose,
    CancelWindowClose,
    WindowUnfocused,
    StatusTick,
    TabHover(Option<(Pane, usize)>),
    SelectTab(usize),
    CloseTab(usize),
    NewTerminalTab,
    NewChatTab,
    /// Variantes de aba que primeiro focam o pane de origem do clique.
    SelectTabIn(Pane, usize),
    CloseTabIn(Pane, usize),
    NewTerminalIn(Pane),
    FocusPane(Pane),
    SplitPane(Pane, Axis),
    /// Pede para fechar o pane; abre a confirmação.
    ClosePane(Pane),
    ConfirmClosePane,
    CancelClosePane,
    ResizePane(pane_grid::ResizeEvent),
    OpenFarm,
    OpenSettings,
    Settings(settings::Message),
    Farm(farm::Message),
    OpenDatabase,
    Database(db_pane::Message),
    OpenHarness,
    Harness(harness_pane::Msg),
    SelectWorkspace(usize),
    PickWorkspaceFolder,
    WorkspaceFolderPicked(Option<PathBuf>),
    ToggleRail,
    ToggleFiles,
    Noop,
    WindowDrag,
    WindowMinimize,
    WindowToggleMaximize,
    WindowCloseButton,
}

struct Shell {
    status_runtime: StatusRuntime,
    island: AgentIsland,
    farm: farm::FarmPane,
    database: db_pane::DbPane,
    harness: harness_pane::HarnessState,
    settings: settings::Settings,
    panes: PaneTree,
    /// Pane aguardando a resposta do diálogo "Fechar painel?".
    pending_close_pane: Option<Pane>,
    hovered_tab: Option<(Pane, usize)>,
    terminals: BTreeMap<TermId, TerminalPane>,
    chats: BTreeMap<ChatId, ChatPane>,
    file_viewers: BTreeMap<PathBuf, FileViewerState>,
    editors: BTreeMap<PathBuf, editor_pane::EditorPane>,
    pending_window_close: Option<iced::window::Id>,
    workspaces: WorkspaceList,
    file_tree: FileTree,
    file_create: Option<file_ops::CreateForm>,
    file_create_generation: u64,
    git: git_state::GitState,
    /// Aba escolhida do painel direito; a exibida é `right_tab()` (cai em Arquivos se sumiu).
    right_tab: RightTab,
    /// Agente escolhido no filtro de Arquivos (`None` = todos); a aba pode ter fechado, ver
    /// `agent_file_marks::effective_filter`.
    agent_filter: Option<String>,
    /// Menu do botão robô aberto.
    agent_menu_open: bool,
    /// Aba Busca: campo, toggles e resultados (somem ao sair da aba, como no Flutter).
    search: SearchState,
    /// Falha da criação de um modelo da Galeria, mostrada num diálogo.
    gallery_error: Option<GalleryError>,
    rail_open: bool,
    files_open: bool,
    static_test_mode: bool,
}

impl Default for Shell {
    fn default() -> Self {
        let mut settings = settings::Settings::default();
        settings.reload();
        let mut status_runtime = StatusRuntime::start_default();
        let mut island = AgentIsland::default();
        island.set_server_error(status_runtime.server_error().cloned());

        let workspaces = rail::default_workspace_list();
        let active_path = workspaces.active().and_then(|ws| ws.path.clone());
        let panes = PaneTree::default();

        let initial_pane_id = pane_id_for_term(1);
        status_runtime.register_pane(initial_pane_id.clone());
        let initial_env = status_runtime.hook_env_for_pane(&initial_pane_id);
        let mut terminals = BTreeMap::new();
        terminals.insert(
            1,
            TerminalPane::new_with_config(
                initial_pane_id,
                active_path.as_deref(),
                initial_env,
                settings.current.clone(),
            ),
        );

        let mut file_tree = FileTree::new();
        file_tree.set_root(active_path.clone());
        let mut git = git_state::GitState::default();
        git.set_repo(active_path);
        Self {
            status_runtime,
            island,
            farm: farm::FarmPane::default(),
            database: db_pane::DbPane::default(),
            harness: harness_pane::HarnessState::default(),
            settings,
            panes,
            pending_close_pane: None,
            hovered_tab: None,
            terminals,
            chats: BTreeMap::new(),
            file_viewers: BTreeMap::new(),
            editors: BTreeMap::new(),
            pending_window_close: None,
            workspaces,
            file_tree,
            file_create: None,
            file_create_generation: 0,
            git,
            right_tab: RightTab::default(),
            agent_filter: None,
            agent_menu_open: false,
            search: SearchState::default(),
            gallery_error: None,
            rail_open: true,
            files_open: true,
            static_test_mode: false,
        }
    }
}

async fn pick_workspace_folder() -> Option<PathBuf> {
    rfd::AsyncFileDialog::new()
        .pick_folder()
        .await
        .map(|handle| handle.path().to_path_buf())
}

impl Shell {
    /// Quais abas do painel direito existem agora (git: a pasta é um repo; galeria: há workspace).
    fn right_available(&self) -> right_tabs::Available {
        right_tabs::Available {
            search: self.workspaces.active().is_some(),
            git: self.git.root.is_some(),
            gallery: self.workspaces.active().is_some(),
        }
    }

    /// Aba do painel direito que aparece: a escolhida, ou Arquivos se a condição dela sumiu.
    fn right_tab(&self) -> RightTab {
        self.right_tab.effective(self.right_available())
    }

    /// Carrega o arquivo (editor ou visualizador) e o abre no pane focado; devolve o caminho
    /// canônico que indexa `editors` e `file_viewers`.
    fn open_in_editor(&mut self, target: PathBuf) -> PathBuf {
        let target = target.canonicalize().unwrap_or(target);
        if !self.editors.contains_key(&target) && !self.file_viewers.contains_key(&target) {
            match FileViewerState::load(&target) {
                FileViewerState::Loaded(doc) => {
                    self.editors.insert(
                        target.clone(),
                        editor_pane::EditorPane::new(target.clone(), doc.into_source())
                            .with_config(self.settings.current.editor.clone()),
                    );
                }
                fallback => {
                    self.file_viewers.insert(target.clone(), fallback);
                }
            }
        }
        self.panes.focused_model_mut().open_file(target.clone());
        target
    }

    /// Clique num resultado da busca: abre o arquivo e revela a linha (`openSearchResult`).
    fn open_search_result(&mut self, relative: &str, line: usize) -> Task<Msg> {
        let Some(path) = self.file_tree.root().map(|root| root.join(relative)) else {
            return Task::none();
        };
        let target = self.open_in_editor(path);
        match self.editors.get_mut(&target) {
            Some(editor) => editor
                .reveal_line(line)
                .map(move |message| Msg::Editor(target.clone(), message)),
            None => Task::none(),
        }
    }

    /// Dispara a busca do estado atual numa thread própria; os lotes voltam por mensagem.
    fn run_search(&mut self) -> Task<Msg> {
        let root = self.file_tree.root().map(Path::to_path_buf);
        let Some(launch) = self.search.begin(root) else {
            return Task::none();
        };
        let generation = launch.generation;
        let (sender, receiver) = iced::futures::channel::mpsc::unbounded();
        let spawned = std::thread::Builder::new()
            .name("gsh-content-search".into())
            .spawn(move || {
                let outcome = gsh_search::search(&launch.query, &launch.cancel, |batch| {
                    let _ = sender.unbounded_send(SearchMsg::Batch(generation, batch));
                });
                let _ = sender.unbounded_send(SearchMsg::Finished(generation, outcome));
            });
        if spawned.is_err() {
            self.search.finish(generation, Ok(()));
            return Task::none();
        }
        Task::run(receiver, Msg::Search)
    }

    fn update_search(&mut self, msg: SearchMsg) -> Task<Msg> {
        match msg {
            SearchMsg::TermChanged(term) => {
                let token = self.search.type_term(term);
                Task::perform(search_debounce(), move |()| {
                    Msg::Search(SearchMsg::DebounceElapsed(token))
                })
            }
            SearchMsg::DebounceElapsed(token) if self.search.debounce_due(token) => {
                self.run_search()
            }
            SearchMsg::DebounceElapsed(_) => Task::none(),
            SearchMsg::Toggle(toggle) => {
                self.search.toggle(toggle);
                self.run_search()
            }
            SearchMsg::ToggleGroup(path) => {
                self.search.toggle_group(path);
                Task::none()
            }
            SearchMsg::Clear => {
                self.search.type_term(String::new());
                Task::batch([
                    self.run_search(),
                    iced::widget::operation::focus(content_search_panel::INPUT_ID),
                ])
            }
            SearchMsg::Open(relative, line) => self.open_search_result(&relative, line),
            SearchMsg::Batch(generation, batch) => {
                self.search.apply_batch(generation, batch);
                Task::none()
            }
            SearchMsg::Finished(generation, outcome) => {
                self.search.finish(generation, outcome);
                Task::none()
            }
            SearchMsg::Spin => {
                self.search.spin_ms = self.search.spin_ms.wrapping_add(SEARCH_SPIN_TICK_MS);
                Task::none()
            }
        }
    }

    /// Botão Harness da topbar: recurso ligado e workspace com pasta local.
    fn shows_harness(&self) -> bool {
        self.settings
            .current
            .shows_feature(gsh_config::schema::AppFeature::Harness)
            && self
                .active_workspace_dir()
                .is_some_and(|dir| !dir.as_os_str().is_empty())
    }

    fn active_workspace_dir(&self) -> Option<&Path> {
        self.workspaces.active().and_then(|ws| ws.path.as_deref())
    }

    fn spawn_terminal_for_dir(&mut self, id: TermId, cwd: Option<&Path>) -> TerminalPane {
        let pane_id = pane_id_for_term(id);
        self.status_runtime.register_pane(pane_id.clone());
        let hook_env = self.status_runtime.hook_env_for_pane(&pane_id);

        if self.static_test_mode {
            let snap = gsh_term::TermSnapshot {
                cols: terminal_pane::DEFAULT_COLS,
                rows: terminal_pane::DEFAULT_ROWS,
                lines: Vec::new(),
                cursor: gsh_term::Cursor {
                    row: 0,
                    col: 0,
                    visible: true,
                },
                display_offset: 0,
                history_size: 0,
            };
            TerminalPane::from_snapshot_in_dir(snap, cwd.map(Path::to_path_buf))
                .with_pane_id(pane_id)
                .with_spawn_env(hook_env)
        } else {
            TerminalPane::new_with_config(pane_id, cwd, hook_env, self.settings.current.clone())
        }
    }

    fn pane_descriptors(&self) -> Vec<PaneDescriptor> {
        let workspace_name = self
            .workspaces
            .active()
            .map(|ws| ws.name.clone())
            .unwrap_or_else(|| labels::WORKSPACE_GSH.to_string());

        self.panes
            .models()
            .into_iter()
            .flat_map(PaneModel::tabs)
            .filter_map(|tab| match tab {
                PaneTab::Terminal(id) => Some(PaneDescriptor::new(
                    pane_id_for_term(*id),
                    labels::DEFAULT_TAB_TITLE,
                    workspace_name.clone(),
                )),
                PaneTab::Chat(_)
                | PaneTab::File(_)
                | PaneTab::Farm
                | PaneTab::Settings
                | PaneTab::Database
                | PaneTab::Harness(_) => None,
            })
            .collect()
    }

    /// Abas do menu do filtro por agente: terminais com harness ativo ou com arquivos tocados
    /// (`agentTabs` em `gsh_page.dart`).
    fn agent_tabs(&self) -> Vec<AgentTab> {
        let activity = self.status_runtime.file_activity();
        self.panes
            .models()
            .into_iter()
            .flat_map(PaneModel::tabs)
            .filter_map(|tab| {
                let PaneTab::Terminal(id) = tab else {
                    return None;
                };
                let pane_id = pane_id_for_term(*id);
                let by_hook = self
                    .status_runtime
                    .pane_record(&pane_id)
                    .is_some_and(|record| record.harness().is_some());
                (by_hook || activity.touched_count(&pane_id) > 0).then(|| AgentTab {
                    id: pane_id,
                    label: tab.title(),
                    by_hook,
                })
            })
            .collect()
    }

    fn sync_island_from_status(&mut self) {
        let descriptors = self.pane_descriptors();
        let agents = self.status_runtime.island_agents(&descriptors);
        self.island.set_agents(agents);
        let agents = descriptors
            .iter()
            .filter_map(|pane| {
                let record = self.status_runtime.pane_record(&pane.pane_id)?;
                if !record.observed() {
                    return None;
                }
                let name = record
                    .harness()
                    .map(agent_status::harness_label)
                    .unwrap_or(labels::DEFAULT_TAB_TITLE);
                Some(farm::AgentInput {
                    pane_id: pane.pane_id.clone(),
                    name: format!("{name} · {}", pane.pane_id),
                    state: farm::AgentState::from_status(record.status(), record.unseen_finish()),
                })
            })
            .collect();
        self.farm.sync_agents(agents);
    }

    fn reveal_terminal_by_pane_id(&mut self, pane_id: &str) {
        for editor in self.editors.values_mut() {
            editor.state.cancel_close();
        }
        let Some(target_id) = term_id_from_pane_id(pane_id) else {
            return;
        };
        let tab = PaneTab::Terminal(target_id);
        if let Some(pane) = self.panes.pane_of(&tab) {
            self.panes.focus(pane);
            let model = self.panes.focused_model_mut();
            if let Some(idx) = model.tabs().iter().position(|t| t == &tab) {
                let _ = model.activate(idx);
            }
            if let Some(term) = self.terminals.get_mut(&target_id) {
                term.sync_snapshot();
            }
        }
        let _ = self.status_runtime.mark_pane_seen(pane_id);
        self.sync_island_from_status();
    }

    fn activate_workspace_path(&mut self, next_path: Option<PathBuf>) {
        self.file_create = None;
        self.file_tree.set_root(next_path.clone());
        self.git.set_repo(next_path.clone());
        if let Some(dir) = next_path.as_deref() {
            for term in self.terminals.values_mut() {
                term.respawn_in_dir(Some(dir));
            }
        }
        let next_branch = self
            .workspaces
            .active()
            .and_then(|ws| ws.git_branch.clone());
        for chat in self.chats.values_mut() {
            chat.set_workspace(next_path.clone(), next_branch.clone());
        }
        self.sync_island_from_status();
    }

    fn active_terminal_mut(&mut self) -> Option<&mut TerminalPane> {
        let Some(PaneTab::Terminal(id)) = self.panes.focused_model().active_tab().cloned() else {
            return None;
        };
        self.terminals.get_mut(&id)
    }

    /// Encerra a sessão de uma aba que já saiu da árvore de panes.
    fn dispose_tab(&mut self, tab: PaneTab) {
        match tab {
            PaneTab::Terminal(id) => {
                if let Some(mut term) = self.terminals.remove(&id) {
                    term.close_session();
                }
                let pane_id = pane_id_for_term(id);
                if self.status_runtime.remove_pane(&pane_id) {
                    self.sync_island_from_status();
                }
            }
            PaneTab::Chat(id) => {
                if let Some(mut chat) = self.chats.remove(&id) {
                    chat.close();
                }
            }
            PaneTab::File(path) => {
                // o mesmo arquivo pode estar aberto em outro pane
                if self.panes.pane_of(&PaneTab::File(path.clone())).is_none() {
                    self.file_viewers.remove(&path);
                    self.editors.remove(&path);
                }
            }
            PaneTab::Farm | PaneTab::Settings | PaneTab::Database | PaneTab::Harness(_) => {}
        }
    }

    /// Abre um terminal novo no pane focado, no diretório do workspace.
    fn open_terminal_in_focused(&mut self) {
        let cwd = self.active_workspace_dir().map(Path::to_path_buf);
        self.open_terminal_for_dir(cwd);
    }

    fn open_terminal_for_dir(&mut self, cwd: Option<PathBuf>) {
        let id = self.panes.alloc_term_id();
        self.panes.focused_model_mut().open_terminal(id);
        let new_term = self.spawn_terminal_for_dir(id, cwd.as_deref());
        self.terminals.insert(id, new_term);
    }

    /// Abas Harness visíveis (a ativa de algum pane).
    fn visible_harnesses(&self) -> BTreeSet<PathBuf> {
        self.panes
            .models()
            .into_iter()
            .filter_map(|model| match model.active_tab() {
                Some(PaneTab::Harness(workspace)) => Some(workspace.clone()),
                _ => None,
            })
            .collect()
    }

    /// O harness recarrega ao montar a aba e quando ela volta a ficar ativa (o usuário pode
    /// ter editado os arquivos), qualquer que seja o jeito de chegar nela.
    fn update(&mut self, msg: Msg) -> Task<Msg> {
        let before = self.visible_harnesses();
        let task = self.update_inner(msg);
        let loads: Vec<Task<Msg>> = self
            .visible_harnesses()
            .into_iter()
            .filter(|workspace| !before.contains(workspace))
            .map(|workspace| self.harness.load(&workspace).map(Msg::Harness))
            .collect();
        if loads.is_empty() {
            return task;
        }
        Task::batch(std::iter::once(task).chain(loads))
    }

    fn update_inner(&mut self, msg: Msg) -> Task<Msg> {
        if matches!(
            &msg,
            Msg::SelectTab(_)
                | Msg::Files(FilesMsg::SelectFile(_))
                | Msg::NewTerminalTab
                | Msg::NewChatTab
                | Msg::WindowUnfocused
        ) {
            for editor in self.editors.values_mut() {
                editor.state.cancel_close();
            }
        }
        match msg {
            Msg::TabHover(tab) => {
                self.hovered_tab = tab;
                Task::none()
            }
            Msg::WindowCloseRequested(id) => {
                if self.editors.values().any(|e| e.state.buffer.is_dirty()) {
                    self.pending_window_close = Some(id);
                    Task::none()
                } else {
                    iced::window::close(id)
                }
            }
            Msg::ConfirmWindowClose => self
                .pending_window_close
                .take()
                .map_or_else(Task::none, iced::window::close),
            Msg::CancelWindowClose => {
                self.pending_window_close = None;
                Task::none()
            }
            Msg::WindowUnfocused => {
                self.pending_window_close = None;
                self.hovered_tab = None;
                Task::none()
            }
            Msg::StatusTick => {
                if self.status_runtime.drain_deliveries(now_epoch_ms()) {
                    self.sync_island_from_status();
                }
                for chat in self.chats.values_mut() {
                    chat.poll_events();
                }
                if self.git.should_refresh() {
                    return self
                        .git
                        .start(git_state::Action::Refresh)
                        .map(Msg::GitResult);
                }
                Task::none()
            }
            Msg::Island(island_msg) => {
                match &island_msg {
                    IslandMsg::RevealTab(tab_id) => {
                        let target = tab_id.clone();
                        self.island.update(island_msg);
                        self.reveal_terminal_by_pane_id(&target);
                    }
                    IslandMsg::DecidePermission { tab_id, decision } => {
                        let target = tab_id.clone();
                        let chosen = *decision;
                        self.island.update(island_msg);
                        let _ = self.status_runtime.apply_permission_decision(
                            &target,
                            chosen,
                            now_epoch_ms(),
                        );
                        if chosen == PermissionDecision::Pass {
                            self.reveal_terminal_by_pane_id(&target);
                        } else {
                            self.sync_island_from_status();
                        }
                    }
                    IslandMsg::TogglePanel | IslandMsg::ClosePanel => {
                        self.island.update(island_msg);
                    }
                }
                Task::none()
            }
            // diálogo modal aberto: o teclado não chega ao terminal
            Msg::Terminal(TerminalMsg::Keyboard(_)) if self.pending_close_pane.is_some() => {
                Task::none()
            }
            // Ctrl/Cmd+Shift+F é do shell (`FocusSearch`), não vai para o PTY
            Msg::Terminal(TerminalMsg::Keyboard(iced::keyboard::Event::KeyPressed {
                ref key,
                modifiers,
                ..
            })) if content_search_panel::is_focus_shortcut(key, modifiers) => Task::none(),
            Msg::Terminal(term_msg) => {
                if matches!(term_msg, TerminalMsg::Tick)
                    && self.status_runtime.drain_deliveries(now_epoch_ms())
                {
                    self.sync_island_from_status();
                }
                if matches!(term_msg, TerminalMsg::Tick) {
                    // todo terminal visível (aba ativa de cada pane) atualiza, não só o focado
                    let visible: Vec<TermId> = self
                        .panes
                        .models()
                        .into_iter()
                        .filter_map(|model| match model.active_tab() {
                            Some(PaneTab::Terminal(id)) => Some(*id),
                            _ => None,
                        })
                        .collect();
                    let ticks = self
                        .terminals
                        .iter_mut()
                        .filter(|(id, _)| visible.contains(id))
                        .map(|(_, term)| term.update(TerminalMsg::Tick).map(Msg::Terminal));
                    return Task::batch(ticks);
                }
                if let TerminalMsg::Keyboard(iced::keyboard::Event::KeyPressed {
                    ref key,
                    ref modified_key,
                    modifiers,
                    ..
                }) = term_msg
                {
                    if matches!(
                        classify_key_press(key, modified_key, modifiers),
                        Some(KeyAction::Send(ref bytes)) if bytes.as_slice() == b"\x1b"
                    ) {
                        if let Some(PaneTab::Terminal(id)) =
                            self.panes.focused_model().active_tab().cloned()
                        {
                            let pane_id = pane_id_for_term(id);
                            if self.status_runtime.interrupt_pane(&pane_id, now_epoch_ms()) {
                                self.sync_island_from_status();
                            }
                        }
                    }
                }
                if let Some(term) = self.active_terminal_mut() {
                    term.update(term_msg).map(Msg::Terminal)
                } else {
                    Task::none()
                }
            }
            Msg::TerminalIn(id, term_msg) => {
                self.terminals.get_mut(&id).map_or_else(Task::none, |term| {
                    term.update(term_msg)
                        .map(move |message| Msg::TerminalIn(id, message))
                })
            }
            Msg::Chat(id, chat_msg) => {
                if let Some(chat) = self.chats.get_mut(&id) {
                    let task = chat.update(chat_msg).map(move |m| Msg::Chat(id, m));
                    let handoff = chat.take_handoff_requested().then(|| {
                        (
                            chat.session_id().map(ToOwned::to_owned),
                            chat.cwd().map(Path::to_path_buf),
                        )
                    });
                    if let Some((Some(session_id), cwd)) = handoff.filter(|(sid, _)| {
                        sid.as_deref().is_some_and(gsh_chat::history::is_session_id)
                    }) {
                        self.open_terminal_for_dir(cwd);
                        if let Some(PaneTab::Terminal(term_id)) =
                            self.panes.focused_model().active_tab().cloned()
                        {
                            if let Some(term) = self.terminals.get_mut(&term_id) {
                                let command =
                                    format!("claude --resume {session_id} --remote-control\n");
                                let _ = term.send_text(&command);
                            }
                        }
                    }
                    task
                } else {
                    Task::none()
                }
            }
            Msg::Editor(path, msg) => {
                if let Some(editor) = self.editors.get_mut(&path) {
                    editor
                        .update(msg)
                        .map(move |message| Msg::Editor(path.clone(), message))
                } else {
                    Task::none()
                }
            }
            Msg::Files(files_msg) => {
                match files_msg {
                    FilesMsg::ToggleDir(dir) => self.file_tree.toggle_dir(&dir),
                    FilesMsg::SelectFile(file_path) => {
                        let ViewerHook::OpenFile(target) = self.file_tree.select_file(file_path);
                        self.open_in_editor(target);
                    }
                    FilesMsg::CollapseAll => self.file_tree.collapse_all(),
                    FilesMsg::Refresh => {
                        self.file_tree.refresh();
                        return self
                            .git
                            .start(git_state::Action::Refresh)
                            .map(Msg::GitResult);
                    }
                    FilesMsg::StartCreate(kind) => {
                        if self.file_tree.root().is_some() && self.file_create.is_none() {
                            self.file_create_generation =
                                self.file_create_generation.wrapping_add(1);
                            self.file_create = Some(file_ops::CreateForm::new(kind));
                            return iced::widget::operation::focus("files-create");
                        }
                    }
                    FilesMsg::CreateNameChanged(name) => {
                        if let Some(form) = self.file_create.as_mut().filter(|form| !form.busy) {
                            form.name = name;
                            form.error = None;
                        }
                    }
                    FilesMsg::CancelCreate => {
                        if self.file_create.as_ref().is_some_and(|form| !form.busy) {
                            self.file_create = None;
                        }
                    }
                    FilesMsg::SubmitCreate => {
                        if let (Some(root), Some(form)) = (
                            self.file_tree.root(),
                            self.file_create.as_mut().filter(|form| !form.busy),
                        ) {
                            form.busy = true;
                            form.error = None;
                            let generation = self.file_create_generation;
                            return Task::perform(
                                file_ops::create_async(
                                    root.to_path_buf(),
                                    form.name.clone(),
                                    form.kind,
                                ),
                                move |(root, result)| {
                                    Msg::Files(FilesMsg::CreateFinished(generation, root, result))
                                },
                            );
                        }
                    }
                    FilesMsg::ToggleAgentMenu => self.agent_menu_open = !self.agent_menu_open,
                    FilesMsg::CloseAgentMenu => self.agent_menu_open = false,
                    FilesMsg::SetAgentFilter(tab) => {
                        self.agent_filter = tab;
                        self.agent_menu_open = false;
                    }
                    FilesMsg::CreateFinished(generation, root, result) => {
                        if generation == self.file_create_generation
                            && self.file_tree.root() == Some(root.as_path())
                        {
                            if let Some(form) = self.file_create.as_mut() {
                                match result {
                                    Ok(()) => {
                                        self.file_create = None;
                                        self.file_tree.refresh();
                                    }
                                    Err(error) => {
                                        form.busy = false;
                                        form.error = Some(error);
                                        return iced::widget::operation::focus("files-create");
                                    }
                                }
                            }
                        }
                    }
                }
                Task::none()
            }
            Msg::SelectRightTab(tab) => {
                if self.right_tab() == RightTab::Search && tab != RightTab::Search {
                    self.search.reset();
                }
                self.right_tab = tab;
                if tab == RightTab::Git && !self.git.busy {
                    return self
                        .git
                        .start(git_state::Action::Refresh)
                        .map(Msg::GitResult);
                }
                Task::none()
            }
            Msg::Search(msg) => self.update_search(msg),
            Msg::FocusSearch => {
                if self.workspaces.active().is_none() {
                    return Task::none();
                }
                self.files_open = true;
                self.right_tab = RightTab::Search;
                Task::batch([
                    iced::widget::operation::focus(content_search_panel::INPUT_ID),
                    iced::widget::operation::select_all(content_search_panel::INPUT_ID),
                ])
            }
            Msg::Gallery(GalleryMsg::Create(template)) => match self.file_tree.root() {
                Some(root) => Task::perform(
                    gallery_create::create_async(root.to_path_buf(), template),
                    Msg::GalleryCreated,
                ),
                None => {
                    self.gallery_error = Some(GalleryError::NoWorkspace);
                    Task::none()
                }
            },
            Msg::GalleryCreated(Ok(path)) => {
                self.file_tree.refresh();
                self.update(Msg::Files(FilesMsg::SelectFile(path)))
            }
            Msg::GalleryCreated(Err(error)) => {
                self.gallery_error = Some(error);
                Task::none()
            }
            Msg::DismissGalleryError => {
                self.gallery_error = None;
                Task::none()
            }
            Msg::GitResult(result) => {
                if self.git.finish(result) {
                    self.git
                        .start(git_state::Action::Refresh)
                        .map(Msg::GitResult)
                } else {
                    Task::none()
                }
            }
            Msg::Git(msg) => {
                use git_panel::GitMsg;
                let action = match msg {
                    GitMsg::CloseDiff => {
                        self.git.diff = None;
                        None
                    }
                    GitMsg::Changes => {
                        self.git.history_open = false;
                        None
                    }
                    GitMsg::History => {
                        self.git.history_open = true;
                        None
                    }
                    GitMsg::Message(value) => {
                        self.git.message = value;
                        None
                    }
                    GitMsg::Refresh => Some(git_state::Action::Refresh),
                    GitMsg::Stage(path) => Some(git_state::Action::Stage(path)),
                    GitMsg::Unstage(path) => Some(git_state::Action::Unstage(path)),
                    GitMsg::Diff(path, staged) => Some(git_state::Action::Diff(path, staged)),
                    GitMsg::Commit if self.git.can_commit() => {
                        Some(git_state::Action::Commit(self.git.message.clone()))
                    }
                    GitMsg::Commit => None,
                };
                action.map_or_else(Task::none, |action| {
                    self.git.start(action).map(Msg::GitResult)
                })
            }
            Msg::SelectTab(idx) => {
                if let Some(tab) = self.panes.focused_model_mut().activate(idx).cloned() {
                    match tab {
                        PaneTab::Terminal(id) => {
                            if let Some(term) = self.terminals.get_mut(&id) {
                                term.sync_snapshot();
                            }
                            let pane_id = pane_id_for_term(id);
                            if self.status_runtime.mark_pane_seen(&pane_id) {
                                self.sync_island_from_status();
                            }
                        }
                        PaneTab::Chat(id) => {
                            if let Some(chat) = self.chats.get_mut(&id) {
                                chat.poll_events();
                            }
                        }
                        PaneTab::File(path) => {
                            let _ = self.file_tree.select_file(path);
                        }
                        PaneTab::Farm
                        | PaneTab::Settings
                        | PaneTab::Database
                        | PaneTab::Harness(_) => {}
                    }
                }
                Task::none()
            }
            Msg::CloseTab(idx) => {
                if self.panes.focused_model().active_index() != Some(idx) {
                    for editor in self.editors.values_mut() {
                        editor.state.cancel_close();
                    }
                }
                if let Some(PaneTab::File(path)) = self.panes.focused_model().tabs().get(idx) {
                    if let Some(editor) = self.editors.get_mut(path) {
                        if !editor.state.request_close() {
                            self.panes.focused_model_mut().activate(idx);
                            return Task::none();
                        }
                    }
                }
                if let Some(removed) = self.panes.focused_model_mut().close(idx) {
                    self.dispose_tab(removed);
                }
                // pane que perdeu a última aba some, se houver outro (como `_closeTabIn`)
                self.panes.close_if_empty(self.panes.focused());
                Task::none()
            }
            Msg::OpenSettings => {
                if !self
                    .panes
                    .focused_model()
                    .tabs()
                    .contains(&PaneTab::Settings)
                {
                    self.settings.reload();
                }
                self.panes.focused_model_mut().open_settings();
                Task::none()
            }
            Msg::Settings(message) => {
                self.settings.update(message);
                if self.settings.saved {
                    for editor in self.editors.values_mut() {
                        editor.set_config(self.settings.current.editor.clone());
                    }
                }
                Task::none()
            }
            Msg::OpenFarm => {
                self.panes.focused_model_mut().open_farm();
                Task::none()
            }
            Msg::Farm(farm::Message::Focus(pane_id)) => {
                self.reveal_terminal_by_pane_id(&pane_id);
                Task::none()
            }
            Msg::OpenHarness => {
                if let Some(workspace) = self.active_workspace_dir().map(Path::to_path_buf) {
                    if let Some(pane) = self.panes.pane_of(&PaneTab::Harness(workspace.clone())) {
                        self.panes.focus(pane);
                    }
                    self.panes.focused_model_mut().open_harness(&workspace);
                }
                Task::none()
            }
            Msg::Harness(message) => {
                let (task, open) = self.harness.update(message);
                if let Some(path) = open {
                    self.open_in_editor(path);
                }
                task.map(Msg::Harness)
            }
            Msg::OpenDatabase => {
                self.panes.focused_model_mut().open_database();
                Task::none()
            }
            Msg::Database(message) => self.database.update(message).map(Msg::Database),
            Msg::Farm(message) => {
                self.farm.update(message);
                Task::none()
            }
            Msg::SelectTabIn(pane, idx) => {
                self.panes.focus(pane);
                self.update(Msg::SelectTab(idx))
            }
            Msg::CloseTabIn(pane, idx) => {
                self.panes.focus(pane);
                self.update(Msg::CloseTab(idx))
            }
            Msg::NewTerminalIn(pane) => {
                self.panes.focus(pane);
                self.update(Msg::NewTerminalTab)
            }
            Msg::FocusPane(pane) => {
                self.panes.focus(pane);
                Task::none()
            }
            Msg::SplitPane(pane, axis) => {
                let cwd = self.active_workspace_dir().map(Path::to_path_buf);
                if let Some(id) = self.panes.split_with_terminal(pane, axis) {
                    let term = self.spawn_terminal_for_dir(id, cwd.as_deref());
                    self.terminals.insert(id, term);
                }
                Task::none()
            }
            Msg::ClosePane(pane) => {
                if self.panes.model(pane).is_some() {
                    self.panes.focus(pane);
                    self.pending_close_pane = Some(pane);
                }
                Task::none()
            }
            Msg::CancelClosePane => {
                self.pending_close_pane = None;
                Task::none()
            }
            Msg::ConfirmClosePane => {
                if let Some(closed) = self
                    .pending_close_pane
                    .take()
                    .and_then(|pane| self.panes.close(pane))
                {
                    for tab in closed.tabs {
                        self.dispose_tab(tab);
                    }
                    if let Some(id) = closed.fresh_terminal {
                        let cwd = self.active_workspace_dir().map(Path::to_path_buf);
                        let term = self.spawn_terminal_for_dir(id, cwd.as_deref());
                        self.terminals.insert(id, term);
                    }
                }
                Task::none()
            }
            Msg::ResizePane(event) => {
                self.panes.resize(event.split, event.ratio);
                Task::none()
            }
            Msg::NewTerminalTab => {
                self.open_terminal_in_focused();
                Task::none()
            }
            Msg::NewChatTab => {
                let cwd = self.active_workspace_dir().map(Path::to_path_buf);
                let branch = self
                    .workspaces
                    .active()
                    .and_then(|ws| ws.git_branch.clone());
                let id = self.panes.alloc_chat_id();
                self.panes.focused_model_mut().open_chat(id);
                let chat = ChatPane::new(cwd, branch).with_static_test_mode(self.static_test_mode);
                self.chats.insert(id, chat);
                Task::none()
            }
            Msg::SelectWorkspace(idx) => {
                if idx != self.workspaces.selected_index() {
                    if let Some(ws) = self.workspaces.select(idx).cloned() {
                        self.activate_workspace_path(ws.path);
                    }
                }
                Task::none()
            }
            Msg::PickWorkspaceFolder => {
                Task::perform(pick_workspace_folder(), Msg::WorkspaceFolderPicked)
            }
            Msg::WorkspaceFolderPicked(Some(folder)) => {
                let _ = self.workspaces.open_folder(folder.clone());
                self.files_open = true;
                self.activate_workspace_path(Some(folder));
                Task::none()
            }
            Msg::WorkspaceFolderPicked(None) => Task::none(),
            Msg::ToggleRail => {
                self.rail_open = !self.rail_open;
                Task::none()
            }
            Msg::ToggleFiles => {
                self.files_open = !self.files_open;
                Task::none()
            }
            Msg::Noop => Task::none(),
            Msg::WindowDrag => iced::window::oldest().then(|id| match id {
                Some(id) => iced::window::drag(id),
                None => Task::none(),
            }),
            Msg::WindowMinimize => iced::window::oldest().then(|id| match id {
                Some(id) => iced::window::minimize(id, true),
                None => Task::none(),
            }),
            Msg::WindowToggleMaximize => iced::window::oldest().then(|id| match id {
                Some(id) => iced::window::toggle_maximize(id),
                None => Task::none(),
            }),
            Msg::WindowCloseButton => iced::window::oldest().then(|id| match id {
                Some(id) => Task::done(Msg::WindowCloseRequested(id)),
                None => Task::none(),
            }),
        }
    }

    fn subscription(&self) -> Subscription<Msg> {
        let status_sub = iced::time::every(Duration::from_millis(STATUS_POLL_INTERVAL_MS))
            .map(|_| Msg::StatusTick);

        let pane_subs = self
            .panes
            .models()
            .into_iter()
            .map(|model| self.tab_subscription(model));

        let windows = iced::window::events().filter_map(|(id, event)| match event {
            iced::window::Event::CloseRequested => Some(Msg::WindowCloseRequested(id)),
            iced::window::Event::Unfocused => Some(Msg::WindowUnfocused),
            _ => None,
        });
        let cancel_create = if self.files_open
            && self.right_tab() == RightTab::Files
            && self.file_create.as_ref().is_some_and(|form| !form.busy)
        {
            iced::event::listen_with(|event, _, _| match event {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                }) => Some(Msg::Files(FilesMsg::CancelCreate)),
                _ => None,
            })
        } else {
            Subscription::none()
        };
        let search_shortcut = iced::event::listen_with(|event, _, _| match event {
            iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. })
                if content_search_panel::is_focus_shortcut(&key, modifiers) =>
            {
                Some(Msg::FocusSearch)
            }
            _ => None,
        });
        let search_visible = self.files_open && self.right_tab() == RightTab::Search;
        // Esc no campo (o text_input captura e desfoca): limpa a busca
        let search_clear = if search_visible {
            iced::event::listen_with(|event, status, _| match event {
                iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                }) if status == iced::event::Status::Captured => {
                    Some(Msg::Search(SearchMsg::Clear))
                }
                _ => None,
            })
        } else {
            Subscription::none()
        };
        let search_spinner = if search_visible && self.search.searching {
            iced::time::every(Duration::from_millis(SEARCH_SPIN_TICK_MS))
                .map(|_| Msg::Search(SearchMsg::Spin))
        } else {
            Subscription::none()
        };
        let harness_spinner = if self.harness.any_loading() {
            iced::time::every(Duration::from_millis(harness_pane::SPIN_TICK_MS))
                .map(|_| Msg::Harness(harness_pane::Msg::Spin))
        } else {
            Subscription::none()
        };
        Subscription::batch(
            [
                status_sub,
                harness_spinner,
                windows,
                cancel_create,
                search_shortcut,
                search_clear,
                search_spinner,
            ]
            .into_iter()
            .chain(pane_subs),
        )
    }

    /// Assinatura de atualização da aba ativa de um pane (terminal ou chat).
    fn tab_subscription(&self, model: &PaneModel) -> Subscription<Msg> {
        match model.active_tab() {
            Some(PaneTab::Terminal(id)) => self
                .terminals
                .get(id)
                .map(|term| term.subscription().map(Msg::Terminal))
                .unwrap_or_else(Subscription::none),
            Some(&PaneTab::Chat(id)) => self
                .chats
                .get(&id)
                .map(|chat| chat.subscription().with(id).map(|(id, m)| Msg::Chat(id, m)))
                .unwrap_or_else(Subscription::none),
            _ => Subscription::none(),
        }
    }

    /// Conteúdo da aba ativa de um pane.
    fn pane_content<'a>(&'a self, pane: Pane, model: &'a PaneModel) -> Element<'a, Msg> {
        match model.active_tab() {
            Some(&PaneTab::Terminal(id)) => self
                .terminals
                .get(&id)
                .map(|term| term.view().map(move |m| Msg::TerminalIn(id, m)))
                .unwrap_or_else(pane_chrome::view_empty_body),
            Some(&PaneTab::Chat(id)) => self
                .chats
                .get(&id)
                .map(|chat| {
                    chat.view(self.settings.current.chat.transcript_width.px() as f32)
                        .map(move |m| Msg::Chat(id, m))
                })
                .unwrap_or_else(pane_chrome::view_empty_body),
            Some(PaneTab::File(path)) => {
                if let Some(editor) = self.editors.get(path) {
                    let path = path.clone();
                    editor
                        .view()
                        .map(move |message| Msg::Editor(path.clone(), message))
                } else {
                    self.file_viewers
                        .get(path)
                        .map(file_viewer::view)
                        .unwrap_or_else(pane_chrome::view_empty_body)
                }
            }
            Some(PaneTab::Farm) => self.farm.view().map(Msg::Farm),
            Some(PaneTab::Database) => self.database.view().map(Msg::Database),
            Some(PaneTab::Harness(workspace)) => {
                harness_pane::view(&self.harness, workspace).map(Msg::Harness)
            }
            Some(PaneTab::Settings) => {
                let index = model.active_index().unwrap_or(0);
                settings::view::view(&self.settings).map(move |message| match message {
                    settings::Message::Back => Msg::CloseTabIn(pane, index),
                    other => Msg::Settings(other),
                })
            }
            None => pane_chrome::view_empty_body(),
        }
    }

    /// Um pane inteiro (barra de abas + conteúdo); todo pane tem os botões de dividir e fechar.
    fn pane_view<'a>(&'a self, pane: Pane, model: &'a PaneModel) -> Element<'a, Msg> {
        // pointer-down no conteúdo foca o pane mesmo que o filho capture o clique
        let on_focus = (self.panes.focused() != pane).then_some(Msg::FocusPane(pane));
        let content =
            pane_focus::FocusOnPress::new(self.pane_content(pane, model), on_focus.clone()).into();
        let dirty = |path: &Path| {
            self.editors
                .get(path)
                .is_some_and(|editor| editor.state.buffer.is_dirty())
        };
        let hovered = self
            .hovered_tab
            .and_then(|(hovered_pane, index)| (hovered_pane == pane).then_some(index));
        pane_chrome::view(
            model,
            content,
            ChromeCallbacks {
                on_select_tab: move |index| Msg::SelectTabIn(pane, index),
                on_close_tab: move |index| Msg::CloseTabIn(pane, index),
                on_new_terminal: Msg::NewTerminalIn(pane),
                noop_msg: Msg::Noop,
                on_split_right: Msg::SplitPane(pane, Axis::Vertical),
                on_split_down: Msg::SplitPane(pane, Axis::Horizontal),
                on_close_pane: Msg::ClosePane(pane),
                // a dica é uma camada acima de tudo: não pode vazar por cima do diálogo
                show_tooltips: self.pending_close_pane.is_none(),
                on_focus,
            },
            &dirty,
            hovered,
            move |index| Msg::TabHover(index.map(|index| (pane, index))),
        )
    }

    fn view(&self) -> Element<'_, Msg> {
        let workspace_title = self
            .workspaces
            .active()
            .map(|ws| ws.name.as_str())
            .unwrap_or(labels::WORKSPACE_GSH);
        let top = topbar::view(
            TopbarProps {
                workspace_title,
                rail_open: self.rail_open,
                files_open: self.files_open,
                counts: self.farm.counts(),
            },
            || self.island.view_pill().map(Msg::Island),
            topbar::TopbarActions {
                on_toggle_rail: Msg::ToggleRail,
                on_toggle_files: Msg::ToggleFiles,
                on_chat: Msg::NewChatTab,
                on_farm: Msg::OpenFarm,
                on_harness: self.shows_harness().then_some(Msg::OpenHarness),
            },
        );

        let window_controls = row![
            window_button(labels::ICON_WINDOW_MINIMIZE, Msg::WindowMinimize),
            window_button(labels::ICON_WINDOW_MAXIMIZE, Msg::WindowToggleMaximize),
            window_button(labels::ICON_CLOSE, Msg::WindowCloseButton),
        ]
        .spacing(2);
        // Mesma divisória inferior da topbar, para a linha seguir até a borda da janela.
        let window_controls = column![
            container(window_controls).center_y(theme::TOPBAR_HEIGHT - 1.0),
            container(Space::new().width(Fill).height(1.0))
                .width(theme::WINDOW_CONTROLS_WIDTH)
                .style(|_: &Theme| container::Style {
                    background: Some(Background::Color(theme::palette().border)),
                    ..Default::default()
                }),
        ];
        let top = row![
            mouse_area(container(top).width(Fill)).on_press(Msg::WindowDrag),
            window_controls,
        ]
        .align_y(iced::Alignment::Center);

        // o fundo `border` aparece só no vão de 1px entre os panes: é a linha do divisor
        let center_pane: Element<'_, Msg> = container(
            pane_grid::PaneGrid::new(self.panes.grid(), |pane, model, _| {
                pane_grid::Content::new(self.pane_view(pane, model))
            })
            .width(Fill)
            .height(Fill)
            .spacing(theme::PANE_DIVIDER_LINE)
            .on_resize(
                theme::PANE_DIVIDER_HANDLE - theme::PANE_DIVIDER_LINE,
                Msg::ResizePane,
            )
            .style(pane_chrome::grid_style),
        )
        .width(Fill)
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        })
        .into();

        let agent_tabs = self.agent_tabs();
        let agents = AgentFiles {
            activity: self.status_runtime.file_activity(),
            tabs: &agent_tabs,
            filter: agent_file_marks::effective_filter(
                self.agent_filter.as_deref(),
                self.status_runtime.file_activity(),
                &agent_tabs,
            ),
            now_ms: now_epoch_ms(),
        };
        let mut body = row![].height(Fill).width(Fill);
        if self.rail_open {
            body = body.push(rail::view(
                &self.workspaces,
                Msg::SelectWorkspace,
                Msg::PickWorkspaceFolder,
                Msg::OpenDatabase,
                Msg::OpenSettings,
            ));
        }
        body = body.push(center_pane);
        if self.files_open {
            let available = self.right_available();
            let tab = self.right_tab();
            let panel: Element<'_, Msg> = match tab {
                RightTab::Files => files_panel::view(
                    &self.file_tree,
                    self.git.status.as_ref(),
                    self.git.root.as_deref(),
                    self.git.busy,
                    self.file_create.as_ref(),
                    agents,
                )
                .map(Msg::Files),
                RightTab::Search => {
                    content_search_panel::view(&self.search, self.file_tree.root().is_some())
                        .map(Msg::Search)
                }
                RightTab::Git => git_panel::view(&self.git).map(Msg::Git),
                RightTab::Gallery => {
                    gallery_panel::view(self.file_tree.root().is_some()).map(Msg::Gallery)
                }
            };
            body = body.push(crate::panel_widgets::frame(
                column![
                    right_tabs::bar(tab, available).map(Msg::SelectRightTab),
                    crate::panel_widgets::hairline_h(),
                    panel
                ]
                .into(),
            ));
        }

        let mut root = column![top].width(Fill).height(Fill);
        if self.pending_window_close.is_some() {
            root = root.push(
                row![
                    iced::widget::text(labels::EDITOR_CONFIRM_WINDOW).color(theme::palette().warn),
                    iced::widget::button(labels::EDITOR_DISCARD).on_press(Msg::ConfirmWindowClose),
                    iced::widget::button(labels::EDITOR_CANCEL).on_press(Msg::CancelWindowClose),
                ]
                .spacing(12),
            );
        }
        let base = container(root.push(body))
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::palette().bg)),
                ..Default::default()
            });

        let layered: Element<'_, Msg> = if self.island.is_expanded() {
            let panel_overlay = container(self.island.view_panel().map(Msg::Island))
                .width(Fill)
                .center_x(Fill)
                .padding(Padding {
                    top: theme::TOPBAR_HEIGHT + 6.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                });

            let backdrop = mouse_area(
                container(Space::new().width(Fill).height(Fill))
                    .width(Fill)
                    .height(Fill),
            )
            .on_press(Msg::Island(IslandMsg::ClosePanel));

            stack![base, backdrop, panel_overlay].into()
        } else {
            base.into()
        };

        // Menu do filtro por agente: popover sob o botão robô, fecha ao clicar fora.
        let layered: Element<'_, Msg> = if self.agent_menu_open
            && self.files_open
            && self.right_tab() == RightTab::Files
            && self.file_tree.root().is_some()
        {
            let menu = container(agent_file_marks::menu(agents).map(Msg::Files))
                .width(Fill)
                .height(Fill)
                .align_x(iced::alignment::Horizontal::Right)
                .padding(Padding {
                    top: theme::TOPBAR_HEIGHT
                        + theme::TAB_STRIP_HEIGHT
                        + theme::FILE_PANEL_HEADER_TOP
                        + theme::AGENT_FILTER_BUTTON_HEIGHT
                        + theme::AGENT_MENU_OFFSET,
                    right: theme::AGENT_MENU_EDGE_MARGIN,
                    bottom: 0.0,
                    left: 0.0,
                });
            stack![
                layered,
                agent_file_marks::menu_backdrop().map(Msg::Files),
                menu
            ]
            .into()
        } else {
            layered
        };

        let layered: Element<'_, Msg> = match self
            .pending_close_pane
            .and_then(|pane| self.panes.model(pane))
        {
            Some(model) => stack![
                layered,
                pane_close_dialog::view(
                    model.tabs().len(),
                    Msg::CancelClosePane,
                    Msg::ConfirmClosePane,
                    Msg::Noop,
                )
            ]
            .into(),
            None => layered,
        };
        let layered: Element<'_, Msg> = match self.harness.name_dialog() {
            Some(dialog) => stack![layered, dialog.map(Msg::Harness)].into(),
            None => layered,
        };
        match &self.gallery_error {
            Some(error) => stack![
                layered,
                pane_close_dialog::notice(
                    labels::GALLERY_CREATE_ERROR_TITLE,
                    error.message(),
                    Msg::DismissGalleryError,
                    Msg::Noop,
                )
            ]
            .into(),
            None => layered,
        }
    }
}

/// Botão de controle de janela (minimizar, maximizar, fechar) da barra própria.
fn window_button<'a>(glyph: &'static str, msg: Msg) -> Element<'a, Msg> {
    crate::panel_widgets::ghost(glyph, Some(msg), false)
}

fn main() -> iced::Result {
    let mut app = iced::application(Shell::default, Shell::update, Shell::view)
        .window(iced::window::Settings {
            exit_on_close_request: false,
            decorations: false,
            ..Default::default()
        })
        .subscription(Shell::subscription)
        .scale_factor(|shell: &Shell| shell.settings.scale)
        .default_font(theme::FONT_BODY)
        .title(labels::APP_TITLE);

    for font_bytes in theme::EMBEDDED_FONTS {
        app = app.font(font_bytes);
    }

    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_core::status::{AgentHarness, HookEvent, StatusUpdate, TurnStatus};
    use gsh_term::{Cell, Cursor, TermSnapshot};
    use iced::keyboard;

    use crate::island::IslandAgentState;

    fn test_shell() -> Shell {
        let snap = TermSnapshot {
            cols: 80,
            rows: 24,
            lines: vec![vec![Cell::default(); 80]; 24],
            cursor: Cursor {
                row: 0,
                col: 0,
                visible: true,
            },
            display_offset: 0,
            history_size: 0,
        };
        let mut status_runtime = StatusRuntime::without_server(None);
        status_runtime.register_pane("t1");
        let workspaces = rail::default_workspace_list();
        let active_path = workspaces.active().and_then(|ws| ws.path.clone());
        let mut terminals = BTreeMap::new();
        terminals.insert(
            1,
            TerminalPane::from_snapshot_in_dir(snap, active_path).with_pane_id("t1"),
        );
        Shell {
            status_runtime,
            island: AgentIsland::default(),
            farm: farm::FarmPane::default(),
            database: db_pane::DbPane::default(),
            harness: harness_pane::HarnessState::default(),
            settings: settings::Settings::default(),
            panes: PaneTree::default(),
            pending_close_pane: None,
            hovered_tab: None,
            terminals,
            chats: BTreeMap::new(),
            file_viewers: BTreeMap::new(),
            editors: BTreeMap::new(),
            pending_window_close: None,
            workspaces,
            file_tree: FileTree::new(),
            file_create: None,
            file_create_generation: 0,
            git: git_state::GitState::default(),
            right_tab: RightTab::default(),
            agent_filter: None,
            agent_menu_open: false,
            search: SearchState::default(),
            gallery_error: None,
            rail_open: true,
            files_open: true,
            static_test_mode: true,
        }
    }

    #[test]
    fn settings_abre_e_reativa_sem_duplicar() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::OpenSettings);
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Settings)
        );
        let _ = shell.update(Msg::SelectTab(0));
        let _ = shell.update(Msg::OpenSettings);
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Settings)
        );
        assert_eq!(shell.panes.focused_model().tabs().len(), 2);
    }

    #[test]
    fn janela_suja_pede_confirmacao_e_perda_de_foco_cancela() {
        use iced::widget::text_editor::{Action, Edit};
        let mut shell = test_shell();
        let path = std::env::temp_dir().join(format!("gsh-window-editor-{}", std::process::id()));
        std::fs::write(&path, "original").unwrap();
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(path.clone())));
        let _ = shell.update(Msg::Editor(
            path.clone(),
            editor_pane::EditorMsg::Action(Action::Edit(Edit::Insert('x'))),
        ));
        let id = iced::window::Id::unique();
        let _ = shell.update(Msg::WindowCloseRequested(id));
        assert_eq!(shell.pending_window_close, Some(id));
        let _ = shell.update(Msg::WindowUnfocused);
        assert!(shell.pending_window_close.is_none());
        assert!(shell.editors.get(&path).unwrap().state.buffer.is_dirty());
        let _ = std::fs::remove_file(path);
    }

    #[cfg(unix)]
    #[test]
    fn alias_de_arquivo_abre_uma_unica_aba() {
        let dir = std::env::temp_dir().join(format!("gsh-alias-editor-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("file.rs");
        let alias = dir.join("alias.rs");
        std::fs::write(&file, "abc").unwrap();
        std::os::unix::fs::symlink(&file, &alias).unwrap();
        let mut shell = test_shell();
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(file.clone())));
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(alias)));
        assert_eq!(shell.editors.len(), 1);
        assert_eq!(shell.panes.focused_model().tabs().len(), 2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn editor_confirma_fechamento_e_preserva_arquivo_sem_salvar() {
        use iced::widget::text_editor::{Action, Edit};
        let mut shell = test_shell();
        let path = std::env::temp_dir().join(format!("gsh-shell-editor-{}", std::process::id()));
        std::fs::write(&path, "original\n").expect("arquivo de teste");
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(path.clone())));
        let _ = shell.update(Msg::Editor(
            path.clone(),
            editor_pane::EditorMsg::Action(Action::Edit(Edit::Insert('x'))),
        ));
        assert!(shell.editors.get(&path).unwrap().state.buffer.is_dirty());
        let _ = shell.update(Msg::CloseTab(1));
        assert_eq!(shell.panes.focused_model().tabs().len(), 2);
        assert!(matches!(
            shell.editors.get(&path).unwrap().state.notice,
            Some(editor_state::EditorNotice::ConfirmClose)
        ));
        let _ = shell.update(Msg::SelectTab(0));
        let _ = shell.update(Msg::CloseTab(1));
        assert!(shell.editors.contains_key(&path));
        let _ = shell.update(Msg::CloseTab(1));
        assert!(!shell.editors.contains_key(&path));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original\n");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn shell_abre_arquivo_como_aba_troca_fecha_e_cria_novo_terminal_no_cwd() {
        let mut shell = test_shell();
        assert!(shell.rail_open);
        assert!(shell.files_open);
        assert_eq!(shell.workspaces.selected_index(), 0);
        assert_eq!(shell.panes.focused_model().tabs(), &[PaneTab::Terminal(1)]);

        // Abre uma pasta escolhida no seletor
        let picked =
            std::env::temp_dir().join(format!("gsh-shell-phase3-test-{}", std::process::id()));
        std::fs::create_dir_all(&picked).expect("create test workspace dir");
        let target_file = picked.join("README.md");
        std::fs::write(&target_file, b"# GSH\nLinha 2\n").expect("write test readme");

        let _ = shell.update(Msg::WorkspaceFolderPicked(Some(picked.clone())));
        assert_eq!(shell.workspaces.selected_index(), 2);
        assert_eq!(shell.file_tree.root(), Some(picked.as_path()));

        // Clica no arquivo na árvore -> abre aba File e carrega o viewer
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(target_file.clone())));
        assert_eq!(shell.file_tree.selected_file(), Some(target_file.as_path()));
        assert_eq!(
            shell.panes.focused_model().tabs(),
            &[PaneTab::Terminal(1), PaneTab::File(target_file.clone())]
        );
        assert_eq!(shell.panes.focused_model().active_index(), Some(1));
        assert_eq!(
            shell.editors.get(&target_file).unwrap().state.buffer.text(),
            "# GSH\nLinha 2\n"
        );

        // Clica novamente no mesmo arquivo depois de voltar ao terminal -> só ativa a aba existente
        let _ = shell.update(Msg::SelectTab(0));
        assert_eq!(shell.panes.focused_model().active_index(), Some(0));
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(target_file.clone())));
        assert_eq!(shell.panes.focused_model().tabs().len(), 2);
        assert_eq!(shell.panes.focused_model().active_index(), Some(1));

        // Clica em "+" -> abre novo terminal no cwd do workspace ativo
        let _ = shell.update(Msg::NewTerminalTab);
        assert_eq!(shell.panes.focused_model().tabs().len(), 3);
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Terminal(2))
        );
        assert_eq!(
            shell.terminals.get(&2).and_then(|t| t.cwd()),
            Some(picked.as_path())
        );

        // Fecha a aba de arquivo (índice 1) -> remove do modelo e do cache de viewers
        let _ = shell.update(Msg::CloseTab(1));
        assert_eq!(
            shell.panes.focused_model().tabs(),
            &[PaneTab::Terminal(1), PaneTab::Terminal(2)]
        );
        assert!(!shell.file_viewers.contains_key(&target_file));

        // Fecha a aba de terminal (índice 1 = Terminal(2)) -> encerra e remove a sessão
        let _ = shell.update(Msg::CloseTab(1));
        assert_eq!(shell.panes.focused_model().tabs(), &[PaneTab::Terminal(1)]);
        assert!(!shell.terminals.contains_key(&2));

        let _ = shell.update(Msg::ToggleRail);
        assert!(!shell.rail_open);

        let _ = shell.update(Msg::ToggleFiles);
        assert!(!shell.files_open);

        let _ = std::fs::remove_dir_all(&picked);
    }

    #[test]
    fn farm_recebe_status_real_e_clique_foca_terminal() {
        let mut shell = test_shell();
        shell.sync_island_from_status();
        assert_eq!(shell.farm.agent_states(), Vec::new());
        let update = StatusUpdate {
            pane_id: "t1".into(),
            status: TurnStatus::Idle,
            event: None,
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Codex),
            files: Vec::new(),
        };
        assert!(shell.status_runtime.apply_update(update, 1_000));
        shell.sync_island_from_status();
        assert_eq!(shell.farm.agent_states(), vec![farm::AgentState::Idle]);
        let _ = shell.update(Msg::OpenFarm);
        let _ = shell.update(Msg::Farm(farm::Message::Focus("t1".into())));
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Terminal(1))
        );
        let _ = shell.update(Msg::CloseTab(0));
        assert!(shell.farm.agent_states().is_empty());
    }

    #[test]
    fn shell_propaga_status_de_turno_dos_terminais_para_a_ilha_e_reveal_tab() {
        let mut shell = test_shell();
        // Abre segundo terminal (t2)
        let _ = shell.update(Msg::NewTerminalTab);
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Terminal(2))
        );

        // t1 entra em Working
        let u1 = StatusUpdate {
            pane_id: "t1".into(),
            status: TurnStatus::Working,
            event: Some(HookEvent::UserPromptSubmit),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Claude),
            files: Vec::new(),
        };
        assert!(shell.status_runtime.apply_update(u1, 1_000));
        shell.sync_island_from_status();

        assert_eq!(shell.island.agents().len(), 1);
        assert_eq!(shell.island.agents()[0].tab_id, "t1");
        assert_eq!(shell.island.agents()[0].state, IslandAgentState::Working);

        // t1 termina (Stop -> Finished)
        let u1_stop = StatusUpdate {
            pane_id: "t1".into(),
            status: TurnStatus::Idle,
            event: Some(HookEvent::Stop),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Claude),
            files: Vec::new(),
        };
        assert!(shell.status_runtime.apply_update(u1_stop, 2_000));
        shell.sync_island_from_status();

        assert_eq!(shell.island.agents()[0].state, IslandAgentState::Finished);

        // Clicar na linha do agente na ilha (RevealTab("t1")) foca o Terminal(1) e limpa o Finished
        let _ = shell.update(Msg::Island(IslandMsg::RevealTab("t1".to_string())));
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Terminal(1))
        );
        assert!(shell.island.agents().is_empty());

        // t1 volta a trabalhar e o usuário aperta ESC cru -> interrompe para Idle sem Finished
        let u1_work = StatusUpdate {
            pane_id: "t1".into(),
            status: TurnStatus::Working,
            event: Some(HookEvent::UserPromptSubmit),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Claude),
            files: Vec::new(),
        };
        assert!(shell.status_runtime.apply_update(u1_work, 3_000));
        shell.sync_island_from_status();
        assert_eq!(shell.island.agents().len(), 1);

        let esc_event = keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(keyboard::key::Named::Escape),
            modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::NONE,
            text: None,
            repeat: false,
        };
        let _ = shell.update(Msg::Terminal(TerminalMsg::Keyboard(esc_event)));
        assert!(shell.island.agents().is_empty());
    }

    #[test]
    fn new_chat_tab_opens_and_closes_cleanly_without_spawning_process() {
        let mut shell = test_shell();

        let _ = shell.update(Msg::NewChatTab);
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Chat(1))
        );
        assert!(shell.chats.contains_key(&1));

        let _ = shell.update(Msg::Chat(1, ChatMsg::InputChanged("oi".to_string())));
        assert_eq!(shell.chats.get(&1).map(ChatPane::input), Some("oi"));

        let _ = shell.update(Msg::CloseTab(1));
        assert!(!shell.chats.contains_key(&1));
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Terminal(1))
        );
    }

    #[test]
    fn dividir_cria_pane_com_terminal_novo_nos_dois_eixos_e_foca_o_novo() {
        let mut shell = test_shell();
        let first = shell.panes.focused();
        let _ = shell.update(Msg::SplitPane(first, Axis::Vertical));
        assert_eq!(shell.panes.leaves().len(), 2);
        assert_ne!(shell.panes.focused(), first);
        assert_eq!(shell.panes.focused_model().tabs(), &[PaneTab::Terminal(2)]);
        assert!(shell.terminals.contains_key(&2));
        let second = shell.panes.focused();
        let _ = shell.update(Msg::SplitPane(second, Axis::Horizontal));
        assert_eq!(shell.panes.leaves().len(), 3);
        assert_eq!(shell.panes.focused_model().tabs(), &[PaneTab::Terminal(3)]);
        // o pane de origem não perdeu nada
        assert_eq!(
            shell.panes.model(first).map(PaneModel::tabs),
            Some(&[PaneTab::Terminal(1)][..])
        );
    }

    #[test]
    fn fechar_pane_pede_confirmacao_e_so_mata_os_terminais_ao_confirmar() {
        let mut shell = test_shell();
        let first = shell.panes.focused();
        let _ = shell.update(Msg::SplitPane(first, Axis::Vertical));
        let second = shell.panes.focused();

        let _ = shell.update(Msg::ClosePane(second));
        assert_eq!(shell.pending_close_pane, Some(second));
        assert_eq!(shell.panes.leaves().len(), 2);
        assert!(shell.terminals.contains_key(&2));

        let _ = shell.update(Msg::CancelClosePane);
        assert_eq!(shell.pending_close_pane, None);
        assert_eq!(shell.panes.leaves().len(), 2);

        let _ = shell.update(Msg::ClosePane(second));
        let _ = shell.update(Msg::ConfirmClosePane);
        assert_eq!(shell.pending_close_pane, None);
        assert_eq!(shell.panes.leaves(), vec![first]);
        assert_eq!(shell.panes.focused(), first);
        assert!(!shell.terminals.contains_key(&2));
        assert!(shell.terminals.contains_key(&1));
    }

    #[test]
    fn fechar_o_unico_pane_encerra_as_abas_e_abre_um_terminal_novo() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::OpenSettings);
        let only = shell.panes.focused();
        let _ = shell.update(Msg::ClosePane(only));
        let _ = shell.update(Msg::ConfirmClosePane);
        assert_eq!(shell.panes.leaves(), vec![only]);
        assert_eq!(shell.panes.focused_model().tabs(), &[PaneTab::Terminal(2)]);
        assert!(!shell.terminals.contains_key(&1));
        assert!(shell.terminals.contains_key(&2));
    }

    #[test]
    fn selecionar_aba_de_pane_nao_focado_move_o_foco() {
        let mut shell = test_shell();
        let first = shell.panes.focused();
        let _ = shell.update(Msg::SplitPane(first, Axis::Vertical));
        let _ = shell.update(Msg::SelectTabIn(first, 0));
        assert_eq!(shell.panes.focused(), first);
        assert_eq!(
            shell.panes.focused_model().active_tab(),
            Some(&PaneTab::Terminal(1))
        );
    }

    #[test]
    fn terminal_novo_em_outro_pane_nao_reaproveita_id() {
        let mut shell = test_shell();
        let first = shell.panes.focused();
        let _ = shell.update(Msg::SplitPane(first, Axis::Vertical));
        let _ = shell.update(Msg::NewTerminalIn(first));
        assert_eq!(shell.panes.focused(), first);
        assert_eq!(
            shell.panes.focused_model().tabs(),
            &[PaneTab::Terminal(1), PaneTab::Terminal(3)]
        );
        assert_eq!(shell.terminals.len(), 3);
    }

    #[test]
    fn fechar_ultima_aba_do_pane_remove_o_pane_quando_ha_outro() {
        let mut shell = test_shell();
        let first = shell.panes.focused();
        let _ = shell.update(Msg::SplitPane(first, Axis::Horizontal));
        let second = shell.panes.focused();
        let _ = shell.update(Msg::CloseTab(0));
        assert_eq!(shell.panes.leaves(), vec![first]);
        assert_eq!(shell.panes.focused(), first);
        assert!(!shell.terminals.contains_key(&2));
        let _ = shell.update(Msg::ClosePane(second));
        assert_eq!(shell.pending_close_pane, None);
    }

    #[test]
    fn clicar_em_fechar_de_pane_nao_focado_foca_e_abre_a_confirmacao() {
        let mut shell = test_shell();
        let first = shell.panes.focused();
        let _ = shell.update(Msg::SplitPane(first, Axis::Vertical));
        assert_ne!(shell.panes.focused(), first);
        let _ = shell.update(Msg::ClosePane(first));
        assert_eq!(shell.panes.focused(), first);
        assert_eq!(shell.pending_close_pane, Some(first));
    }

    #[test]
    fn aba_do_painel_direito_volta_para_arquivos_quando_a_condicao_some() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::SelectRightTab(RightTab::Gallery));
        assert_eq!(shell.right_tab(), RightTab::Gallery);
        let _ = shell.update(Msg::SelectRightTab(RightTab::Git));
        assert_eq!(shell.right_tab, RightTab::Git);
        assert_eq!(shell.right_tab(), RightTab::Files);
        shell.git.root = Some(PathBuf::from("/repo"));
        assert_eq!(shell.right_tab(), RightTab::Git);
        shell.git.root = None;
        assert_eq!(shell.right_tab(), RightTab::Files);
    }

    #[test]
    fn galeria_sem_pasta_acusa_erro_e_o_dialogo_fecha() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::Gallery(GalleryMsg::Create(
            gallery_template::GalleryTemplate::Kanban,
        )));
        assert_eq!(shell.gallery_error, Some(GalleryError::NoWorkspace));
        let _ = shell.update(Msg::DismissGalleryError);
        assert_eq!(shell.gallery_error, None);
        let _ = shell.update(Msg::GalleryCreated(Err(GalleryError::WriteFailed)));
        assert_eq!(shell.gallery_error, Some(GalleryError::WriteFailed));
    }

    #[test]
    fn galeria_criada_abre_o_arquivo_numa_aba() {
        let mut shell = test_shell();
        let dir = std::env::temp_dir().join(format!("gsh-shell-gallery-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        shell.file_tree.set_root(Some(dir.clone()));
        let path =
            gallery_create::create_from_template(&dir, gallery_template::GalleryTemplate::Kanban)
                .unwrap();
        let _ = shell.update(Msg::GalleryCreated(Ok(path.clone())));
        let opened = path.canonicalize().unwrap();
        assert!(shell
            .panes
            .focused_model()
            .tabs()
            .contains(&PaneTab::File(opened.clone())));
        assert!(shell.editors.contains_key(&opened));
        assert_eq!(shell.gallery_error, None);
        let _ = std::fs::remove_dir_all(dir);
    }

    fn search_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("gsh-shell-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn atalho_de_busca_mostra_o_painel_e_troca_para_a_aba_busca() {
        let mut shell = test_shell();
        shell.files_open = false;
        let _ = shell.update(Msg::FocusSearch);
        assert!(shell.files_open);
        assert_eq!(shell.right_tab(), RightTab::Search);
        // sem workspace: nada acontece
        let mut shell = test_shell();
        shell.files_open = false;
        shell.workspaces = WorkspaceList::new(Vec::new());
        let _ = shell.update(Msg::FocusSearch);
        assert!(!shell.files_open);
        assert_eq!(shell.right_tab(), RightTab::Files);
    }

    #[test]
    fn sair_da_aba_busca_limpa_campo_e_resultados() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::SelectRightTab(RightTab::Search));
        let _ = shell.update(Msg::Search(SearchMsg::TermChanged("needle".into())));
        assert_eq!(shell.search.term, "needle");
        let _ = shell.update(Msg::SelectRightTab(RightTab::Search));
        assert_eq!(shell.search.term, "needle");
        let _ = shell.update(Msg::SelectRightTab(RightTab::Gallery));
        assert!(shell.search.term.is_empty());
    }

    #[test]
    fn busca_so_dispara_no_ultimo_debounce() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::Search(SearchMsg::TermChanged("a".into())));
        let stale = shell.search.type_term("ab".into()) - 1;
        let _ = shell.update(Msg::Search(SearchMsg::DebounceElapsed(stale)));
        assert!(!shell.search.searching);
    }

    #[test]
    fn clicar_no_resultado_abre_o_arquivo_na_linha() {
        let mut shell = test_shell();
        let dir = search_dir("open-result");
        std::fs::write(dir.join("a.txt"), "um\ndois\ntres\n").unwrap();
        shell.file_tree.set_root(Some(dir.clone()));
        let _ = shell.update(Msg::Search(SearchMsg::Open("a.txt".into(), 3)));
        let opened = dir.join("a.txt").canonicalize().unwrap();
        assert!(shell
            .panes
            .focused_model()
            .tabs()
            .contains(&PaneTab::File(opened.clone())));
        let cursor = shell.editors.get(&opened).unwrap().content.cursor();
        assert_eq!((cursor.position.line, cursor.position.column), (2, 0));
        let _ = std::fs::remove_dir_all(dir);
    }
    #[test]
    fn harness_tab_is_unique_per_workspace_and_reloads_when_it_becomes_active() {
        let mut shell = test_shell();
        let a = PathBuf::from("/tmp/gsh-harness-a");
        let b = PathBuf::from("/tmp/gsh-harness-b");
        shell.workspaces.open_folder(a.clone());
        let _ = shell.update(Msg::OpenHarness);
        assert!(shell.harness.is_loading(&a));
        let _ = shell.update(Msg::OpenHarness);
        shell.workspaces.open_folder(b.clone());
        let _ = shell.update(Msg::OpenHarness);
        let harness_tabs = |shell: &Shell| {
            shell
                .panes
                .focused_model()
                .tabs()
                .iter()
                .filter(|tab| matches!(tab, PaneTab::Harness(_)))
                .count()
        };
        assert_eq!(harness_tabs(&shell), 2);
        assert!(shell.harness.is_loading(&b));

        // lido: sair da aba e voltar relê
        let _ = shell.update(Msg::Harness(harness_pane::Msg::Loaded(
            a.clone(),
            Some(gsh_harness::ProjectHarness::default()),
        )));
        let _ = shell.update(Msg::Harness(harness_pane::Msg::Loaded(b.clone(), None)));
        assert!(!shell.harness.is_loading(&a));
        let tabs = shell.panes.focused_model().tabs().to_vec();
        let index_a = tabs
            .iter()
            .position(|tab| tab == &PaneTab::Harness(a.clone()))
            .expect("aba a");
        let _ = shell.update(Msg::SelectTab(index_a));
        assert!(shell.harness.is_loading(&a));
        assert!(shell.harness.of(&a).is_some());
    }

    #[test]
    fn harness_button_needs_the_feature_and_a_local_folder() {
        let mut shell = test_shell();
        shell.workspaces.open_folder("/tmp/gsh-harness-a");
        assert!(shell.shows_harness());
        shell
            .settings
            .current
            .features
            .hidden
            .insert(gsh_config::schema::AppFeature::Harness);
        assert!(!shell.shows_harness());
        shell.settings.current.features.hidden.clear();
        // o workspace de exemplo não tem pasta
        shell.workspaces.select(1);
        assert!(!shell.shows_harness());
    }
}
