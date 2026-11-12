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

pub mod agent_island_panel;
pub mod agent_status;
pub mod chat_composer;
pub mod chat_pane;
pub mod chat_view;
pub mod claude_hook_installer;
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
pub mod island;
pub mod labels;
pub mod pane_chrome;
pub mod pane_model;
pub mod pane_strip;
pub mod pane_tab;
pub mod panel_widgets;
pub mod rail;
pub mod settings;
pub mod terminal_canvas;
pub mod terminal_grid;
pub mod terminal_input;
pub mod terminal_pane;
pub mod theme;
pub mod topbar;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gsh_core::workspace::WorkspaceList;
use iced::widget::{column, container, mouse_area, row, stack, Space};
use iced::{Background, Element, Fill, Padding, Subscription, Task, Theme};

use crate::agent_status::{
    now_epoch_ms, pane_id_for_term, term_id_from_pane_id, PaneDescriptor, StatusRuntime,
};
use crate::chat_pane::{ChatMsg, ChatPane};
use crate::file_tree::{FileTree, ViewerHook};
use crate::file_viewer::FileViewerState;
use crate::files_panel::FilesMsg;
use crate::island::{AgentIsland, IslandMsg, PermissionDecision};
use crate::pane_chrome::ChromeCallbacks;
use crate::pane_model::{ChatId, PaneModel, PaneTab, TermId};
use crate::terminal_input::{classify_key_press, KeyAction};
use crate::terminal_pane::{TerminalMsg, TerminalPane};
use crate::topbar::TopbarProps;

/// Intervalo de polling do canal de status na `Subscription` do `Shell` (50 ms).
const STATUS_POLL_INTERVAL_MS: u64 = 50;

#[derive(Debug, Clone)]
enum Msg {
    Island(IslandMsg),
    Terminal(TerminalMsg),
    Chat(ChatId, ChatMsg),
    Files(FilesMsg),
    Git(git_panel::GitMsg),
    GitResult(git_state::ResultMsg),
    Editor(PathBuf, editor_pane::EditorMsg),
    WindowCloseRequested(iced::window::Id),
    ConfirmWindowClose,
    CancelWindowClose,
    WindowUnfocused,
    StatusTick,
    TabHover(Option<(bool, usize)>),
    SelectTab(usize),
    CloseTab(usize),
    NewTerminalTab,
    NewChatTab,
    // ponytail: sem botão na barra; o menu do `+` volta a construir esta mensagem
    #[allow(dead_code)]
    SplitRight,
    CloseSplit,
    SelectOther(usize),
    CloseOther(usize),
    NewTerminalOther,
    OpenFarm,
    OpenSettings,
    Settings(settings::Message),
    Farm(farm::Message),
    OpenDatabase,
    Database(db_pane::Message),
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
    settings: settings::Settings,
    pane_model: PaneModel,
    /// Segundo pane quando o central está dividido; `pane_model` é sempre o focado.
    split_pane: Option<PaneModel>,
    /// `true` quando o pane focado está à direita (o outro, à esquerda).
    split_focus_right: bool,
    hovered_tab: Option<(bool, usize)>,
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
        let pane_model = PaneModel::new();

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
            settings,
            pane_model,
            split_pane: None,
            split_focus_right: false,
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

        self.pane_model
            .tabs()
            .iter()
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
                | PaneTab::Database => None,
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
        let tab_idx = self
            .pane_model
            .tabs()
            .iter()
            .position(|tab| matches!(tab, PaneTab::Terminal(id) if *id == target_id));
        if let Some(idx) = tab_idx {
            let _ = self.pane_model.activate(idx);
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
        let Some(PaneTab::Terminal(id)) = self.pane_model.active_tab().cloned() else {
            return None;
        };
        self.terminals.get_mut(&id)
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        let task = self.update_inner(msg);
        self.normalize_split();
        task
    }

    /// Divisão sem abas some; pane focado vazio cede lugar ao outro.
    fn normalize_split(&mut self) {
        if self
            .split_pane
            .as_ref()
            .is_some_and(|o| o.tabs().is_empty())
        {
            self.split_pane = None;
            self.split_focus_right = false;
        }
        if self.pane_model.tabs().is_empty() {
            if let Some(other) = self.split_pane.take() {
                self.pane_model = other;
                self.split_focus_right = false;
            }
        }
    }

    /// Troca o pane focado pelo outro (o focado passa a ser o que estava ao lado).
    fn focus_other(&mut self) {
        if let Some(other) = self.split_pane.as_mut() {
            std::mem::swap(&mut self.pane_model, other);
            self.split_focus_right = !self.split_focus_right;
        }
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
            Msg::Terminal(term_msg) => {
                if matches!(term_msg, TerminalMsg::Tick)
                    && self.status_runtime.drain_deliveries(now_epoch_ms())
                {
                    self.sync_island_from_status();
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
                        if let Some(PaneTab::Terminal(id)) = self.pane_model.active_tab().cloned() {
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
            Msg::Chat(id, chat_msg) => {
                if let Some(chat) = self.chats.get_mut(&id) {
                    chat.update(chat_msg).map(move |m| Msg::Chat(id, m))
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
                        let target = target.canonicalize().unwrap_or(target);
                        if !self.editors.contains_key(&target)
                            && !self.file_viewers.contains_key(&target)
                        {
                            match FileViewerState::load(&target) {
                                FileViewerState::Loaded(doc) => {
                                    self.editors.insert(
                                        target.clone(),
                                        editor_pane::EditorPane::new(
                                            target.clone(),
                                            doc.into_source(),
                                        )
                                        .with_config(self.settings.current.editor.clone()),
                                    );
                                }
                                fallback => {
                                    self.file_viewers.insert(target.clone(), fallback);
                                }
                            }
                        }
                        self.pane_model.open_file(target);
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
                    FilesMsg::OpenGit => {
                        self.git.git_open = true;
                        if self.git.busy {
                            return Task::none();
                        }
                        return self
                            .git
                            .start(git_state::Action::Refresh)
                            .map(Msg::GitResult);
                    }
                }
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
                    GitMsg::Files => {
                        self.git.git_open = false;
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
                if let Some(tab) = self.pane_model.activate(idx).cloned() {
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
                        PaneTab::Farm | PaneTab::Settings | PaneTab::Database => {}
                    }
                }
                Task::none()
            }
            Msg::CloseTab(idx) => {
                if self.pane_model.active_index() != Some(idx) {
                    for editor in self.editors.values_mut() {
                        editor.state.cancel_close();
                    }
                }
                if let Some(PaneTab::File(path)) = self.pane_model.tabs().get(idx) {
                    if let Some(editor) = self.editors.get_mut(path) {
                        if !editor.state.request_close() {
                            self.pane_model.activate(idx);
                            return Task::none();
                        }
                    }
                }
                if let Some(removed) = self.pane_model.close(idx) {
                    match removed {
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
                            self.file_viewers.remove(&path);
                            self.editors.remove(&path);
                        }
                        PaneTab::Farm | PaneTab::Settings | PaneTab::Database => {}
                    }
                }
                Task::none()
            }
            Msg::OpenSettings => {
                if !self.pane_model.tabs().contains(&PaneTab::Settings) {
                    self.settings.reload();
                }
                self.pane_model.open_settings();
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
                self.pane_model.open_farm();
                Task::none()
            }
            Msg::Farm(farm::Message::Focus(pane_id)) => {
                self.reveal_terminal_by_pane_id(&pane_id);
                Task::none()
            }
            Msg::OpenDatabase => {
                self.pane_model.open_database();
                Task::none()
            }
            Msg::Database(message) => self.database.update(message).map(Msg::Database),
            Msg::Farm(message) => {
                self.farm.update(message);
                Task::none()
            }
            Msg::SplitRight => {
                if self.split_pane.is_none() {
                    if let Some(idx) = self.pane_model.active_index() {
                        if let Some(tab) = self.pane_model.close(idx) {
                            let mut moved = PaneModel::empty();
                            moved.adopt(tab);
                            let remaining = std::mem::replace(&mut self.pane_model, moved);
                            self.split_pane = Some(remaining);
                            self.split_focus_right = true;
                        }
                    }
                }
                Task::none()
            }
            Msg::CloseSplit => {
                if let Some(other) = self.split_pane.take() {
                    for tab in other.tabs().iter().cloned() {
                        self.pane_model.adopt(tab);
                    }
                }
                self.split_focus_right = false;
                Task::none()
            }
            Msg::SelectOther(idx) => {
                self.focus_other();
                self.update(Msg::SelectTab(idx))
            }
            Msg::CloseOther(idx) => {
                self.focus_other();
                self.update(Msg::CloseTab(idx))
            }
            Msg::NewTerminalOther => {
                self.focus_other();
                self.update(Msg::NewTerminalTab)
            }
            Msg::NewTerminalTab => {
                let cwd = self.active_workspace_dir().map(Path::to_path_buf);
                let id = self.pane_model.open_terminal();
                let new_term = self.spawn_terminal_for_dir(id, cwd.as_deref());
                self.terminals.insert(id, new_term);
                Task::none()
            }
            Msg::NewChatTab => {
                let cwd = self.active_workspace_dir().map(Path::to_path_buf);
                let branch = self
                    .workspaces
                    .active()
                    .and_then(|ws| ws.git_branch.clone());
                let id = self.pane_model.open_chat();
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

        let active_sub = self.tab_subscription(&self.pane_model);
        let other_sub = self
            .split_pane
            .as_ref()
            .map(|other| self.tab_subscription(other))
            .unwrap_or_else(Subscription::none);

        let windows = iced::window::events().filter_map(|(id, event)| match event {
            iced::window::Event::CloseRequested => Some(Msg::WindowCloseRequested(id)),
            iced::window::Event::Unfocused => Some(Msg::WindowUnfocused),
            _ => None,
        });
        let cancel_create = if self.files_open
            && !self.git.git_open
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
        Subscription::batch([status_sub, active_sub, other_sub, windows, cancel_create])
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
    fn pane_content<'a>(&'a self, model: &'a PaneModel) -> Element<'a, Msg> {
        match model.active_tab() {
            Some(PaneTab::Terminal(id)) => self
                .terminals
                .get(id)
                .map(|term| term.view().map(Msg::Terminal))
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
            Some(PaneTab::Settings) => {
                let index = model.active_index().unwrap_or(0);
                let focused = std::ptr::eq(model, &self.pane_model);
                settings::view::view(&self.settings).map(move |message| match message {
                    settings::Message::Back => {
                        if focused {
                            Msg::CloseTab(index)
                        } else {
                            Msg::CloseOther(index)
                        }
                    }
                    other => Msg::Settings(other),
                })
            }
            None => pane_chrome::view_empty_body(),
        }
    }

    /// Um pane inteiro (barra de abas + conteúdo). O focado recebe os controles de divisão.
    fn pane_view<'a>(&'a self, model: &'a PaneModel, focused: bool) -> Element<'a, Msg> {
        let content = self.pane_content(model);
        let dirty = |path: &Path| {
            self.editors
                .get(path)
                .is_some_and(|editor| editor.state.buffer.is_dirty())
        };
        let split_open = self.split_pane.is_some();
        let right = split_open && (focused == self.split_focus_right);
        let hovered = self
            .hovered_tab
            .and_then(|(side, index)| (side == right).then_some(index));
        if focused {
            pane_chrome::view(
                model,
                content,
                ChromeCallbacks {
                    on_select_tab: Msg::SelectTab,
                    on_close_tab: Msg::CloseTab,
                    on_new_terminal: Msg::NewTerminalTab,
                    noop_msg: Msg::Noop,
                    // ponytail: botão de dividir sai da barra a pedido; volta no menu do `+`
                    on_split: None,
                    on_close_split: split_open.then_some(Msg::CloseSplit),
                },
                &dirty,
                hovered,
                move |index| Msg::TabHover(index.map(|index| (right, index))),
            )
        } else {
            pane_chrome::view(
                model,
                content,
                ChromeCallbacks {
                    on_select_tab: Msg::SelectOther,
                    on_close_tab: Msg::CloseOther,
                    on_new_terminal: Msg::NewTerminalOther,
                    noop_msg: Msg::Noop,
                    on_split: None,
                    on_close_split: None,
                },
                &dirty,
                hovered,
                move |index| Msg::TabHover(index.map(|index| (right, index))),
            )
        }
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
                counts: self.island.counts(),
            },
            || self.island.view_pill().map(Msg::Island),
            topbar::TopbarActions {
                on_toggle_rail: Msg::ToggleRail,
                on_toggle_files: Msg::ToggleFiles,
                on_chat: Msg::NewChatTab,
                on_farm: Msg::OpenFarm,
            },
        );

        let window_controls = row![
            window_button(labels::ICON_WINDOW_MINIMIZE, Msg::WindowMinimize),
            window_button(labels::ICON_WINDOW_MAXIMIZE, Msg::WindowToggleMaximize),
            window_button(labels::ICON_CLOSE, Msg::WindowCloseButton),
        ]
        .spacing(2);
        let top = row![
            mouse_area(container(top).width(Fill)).on_press(Msg::WindowDrag),
            window_controls,
        ]
        .align_y(iced::Alignment::Center);

        let focused_el = self.pane_view(&self.pane_model, true);
        let center_pane: Element<'_, Msg> = match self.split_pane.as_ref() {
            Some(other) => {
                let other_el = self.pane_view(other, false);
                let (left, right) = if self.split_focus_right {
                    (other_el, focused_el)
                } else {
                    (focused_el, other_el)
                };
                let divider = container(iced::widget::Space::new())
                    .width(1)
                    .height(Fill)
                    .style(|_: &Theme| container::Style {
                        background: Some(Background::Color(theme::BORDER)),
                        ..Default::default()
                    });
                row![left, divider, right].height(Fill).width(Fill).into()
            }
            None => focused_el,
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
            if self.git.git_open {
                body = body.push(git_panel::view(&self.git).map(Msg::Git));
            } else {
                body = body.push(
                    files_panel::view(
                        &self.file_tree,
                        self.git.status.as_ref(),
                        self.git.root.as_deref(),
                        self.git.busy,
                        self.file_create.as_ref(),
                    )
                    .map(Msg::Files),
                );
            }
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

        if self.island.is_expanded() {
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
            settings: settings::Settings::default(),
            pane_model: PaneModel::new(),
            split_pane: None,
            split_focus_right: false,
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
            rail_open: true,
            files_open: true,
            static_test_mode: true,
        }
    }

    #[test]
    fn settings_abre_e_reativa_sem_duplicar() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::OpenSettings);
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Settings));
        let _ = shell.update(Msg::SelectTab(0));
        let _ = shell.update(Msg::OpenSettings);
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Settings));
        assert_eq!(shell.pane_model.tabs().len(), 2);
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
        assert_eq!(shell.pane_model.tabs().len(), 2);
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
        assert_eq!(shell.pane_model.tabs().len(), 2);
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
        assert_eq!(shell.pane_model.tabs(), &[PaneTab::Terminal(1)]);

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
            shell.pane_model.tabs(),
            &[PaneTab::Terminal(1), PaneTab::File(target_file.clone())]
        );
        assert_eq!(shell.pane_model.active_index(), Some(1));
        assert_eq!(
            shell.editors.get(&target_file).unwrap().state.buffer.text(),
            "# GSH\nLinha 2\n"
        );

        // Clica novamente no mesmo arquivo depois de voltar ao terminal -> só ativa a aba existente
        let _ = shell.update(Msg::SelectTab(0));
        assert_eq!(shell.pane_model.active_index(), Some(0));
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(target_file.clone())));
        assert_eq!(shell.pane_model.tabs().len(), 2);
        assert_eq!(shell.pane_model.active_index(), Some(1));

        // Clica em "+" -> abre novo terminal no cwd do workspace ativo
        let _ = shell.update(Msg::NewTerminalTab);
        assert_eq!(shell.pane_model.tabs().len(), 3);
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Terminal(2)));
        assert_eq!(
            shell.terminals.get(&2).and_then(|t| t.cwd()),
            Some(picked.as_path())
        );

        // Fecha a aba de arquivo (índice 1) -> remove do modelo e do cache de viewers
        let _ = shell.update(Msg::CloseTab(1));
        assert_eq!(
            shell.pane_model.tabs(),
            &[PaneTab::Terminal(1), PaneTab::Terminal(2)]
        );
        assert!(!shell.file_viewers.contains_key(&target_file));

        // Fecha a aba de terminal (índice 1 = Terminal(2)) -> encerra e remove a sessão
        let _ = shell.update(Msg::CloseTab(1));
        assert_eq!(shell.pane_model.tabs(), &[PaneTab::Terminal(1)]);
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
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Terminal(1)));
        let _ = shell.update(Msg::CloseTab(0));
        assert!(shell.farm.agent_states().is_empty());
    }

    #[test]
    fn shell_propaga_status_de_turno_dos_terminais_para_a_ilha_e_reveal_tab() {
        let mut shell = test_shell();
        // Abre segundo terminal (t2)
        let _ = shell.update(Msg::NewTerminalTab);
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Terminal(2)));

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
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Terminal(1)));
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
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Chat(1)));
        assert!(shell.chats.contains_key(&1));

        let _ = shell.update(Msg::Chat(1, ChatMsg::InputChanged("oi".to_string())));
        assert_eq!(shell.chats.get(&1).map(ChatPane::input), Some("oi"));

        let _ = shell.update(Msg::CloseTab(1));
        assert!(!shell.chats.contains_key(&1));
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Terminal(1)));
    }

    #[test]
    fn dividir_move_aba_ativa_para_o_painel_novo_e_fechar_junta() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::OpenSettings);
        let before = shell.pane_model.tabs().len();
        let _ = shell.update(Msg::SplitRight);
        assert!(shell.split_pane.is_some());
        assert!(shell.split_focus_right);
        assert_eq!(shell.pane_model.tabs(), &[PaneTab::Settings]);
        assert_eq!(
            shell.split_pane.as_ref().map(|p| p.tabs().len()),
            Some(before - 1)
        );
        let _ = shell.update(Msg::CloseSplit);
        assert!(shell.split_pane.is_none());
        assert_eq!(shell.pane_model.tabs().len(), before);
    }

    #[test]
    fn selecionar_aba_do_outro_painel_troca_o_foco() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::OpenSettings);
        let _ = shell.update(Msg::SplitRight);
        let _ = shell.update(Msg::SelectOther(0));
        assert!(!shell.split_focus_right);
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Terminal(1)));
    }

    #[test]
    fn fechar_ultima_aba_do_painel_focado_cede_lugar_ao_outro() {
        let mut shell = test_shell();
        let _ = shell.update(Msg::OpenSettings);
        let _ = shell.update(Msg::SplitRight);
        let _ = shell.update(Msg::CloseTab(0));
        assert!(shell.split_pane.is_none());
        assert_eq!(shell.pane_model.active_tab(), Some(&PaneTab::Terminal(1)));
    }
}
