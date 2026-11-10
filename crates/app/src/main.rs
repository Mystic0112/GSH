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

pub mod file_tree;
pub mod file_viewer;
pub mod files_panel;
pub mod gorilla;
pub mod island;
pub mod labels;
pub mod pane_chrome;
pub mod pane_model;
pub mod rail;
pub mod terminal_canvas;
pub mod terminal_grid;
pub mod terminal_input;
pub mod terminal_pane;
pub mod theme;
pub mod topbar;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use gsh_core::workspace::WorkspaceList;
use iced::widget::{column, container, mouse_area, row, stack, Space};
use iced::{Background, Element, Fill, Padding, Subscription, Task, Theme};

use crate::file_tree::{FileTree, ViewerHook};
use crate::file_viewer::FileViewerState;
use crate::files_panel::FilesMsg;
use crate::island::{AgentIsland, IslandMsg};
use crate::pane_chrome::ChromeCallbacks;
use crate::pane_model::{PaneModel, PaneTab, TermId};
use crate::terminal_pane::{TerminalMsg, TerminalPane};
use crate::topbar::TopbarProps;

#[derive(Debug, Clone)]
enum Msg {
    Island(IslandMsg),
    Terminal(TerminalMsg),
    Files(FilesMsg),
    SelectTab(usize),
    CloseTab(usize),
    NewTerminalTab,
    SelectWorkspace(usize),
    PickWorkspaceFolder,
    WorkspaceFolderPicked(Option<PathBuf>),
    ToggleRail,
    ToggleFiles,
    Noop,
}

struct Shell {
    island: AgentIsland,
    pane_model: PaneModel,
    terminals: BTreeMap<TermId, TerminalPane>,
    file_viewers: BTreeMap<PathBuf, FileViewerState>,
    workspaces: WorkspaceList,
    file_tree: FileTree,
    rail_open: bool,
    files_open: bool,
    static_test_mode: bool,
}

impl Default for Shell {
    fn default() -> Self {
        let workspaces = rail::default_workspace_list();
        let active_path = workspaces.active().and_then(|ws| ws.path.clone());
        let pane_model = PaneModel::new();
        let mut terminals = BTreeMap::new();
        terminals.insert(1, TerminalPane::new_in_dir(active_path.as_deref()));
        let mut file_tree = FileTree::new();
        file_tree.set_root(active_path);
        Self {
            island: AgentIsland::default(),
            pane_model,
            terminals,
            file_viewers: BTreeMap::new(),
            workspaces,
            file_tree,
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

    fn spawn_terminal_for_dir(&self, cwd: Option<&Path>) -> TerminalPane {
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
        } else {
            TerminalPane::new_in_dir(cwd)
        }
    }

    fn activate_workspace_path(&mut self, next_path: Option<PathBuf>) {
        self.file_tree.set_root(next_path.clone());
        if let Some(dir) = next_path.as_deref() {
            for term in self.terminals.values_mut() {
                term.respawn_in_dir(Some(dir));
            }
        }
    }

    fn active_terminal_mut(&mut self) -> Option<&mut TerminalPane> {
        let Some(PaneTab::Terminal(id)) = self.pane_model.active_tab().cloned() else {
            return None;
        };
        self.terminals.get_mut(&id)
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Island(island_msg) => {
                self.island.update(island_msg);
                Task::none()
            }
            Msg::Terminal(term_msg) => {
                if let Some(term) = self.active_terminal_mut() {
                    term.update(term_msg).map(Msg::Terminal)
                } else {
                    Task::none()
                }
            }
            Msg::Files(files_msg) => {
                match files_msg {
                    FilesMsg::ToggleDir(dir) => self.file_tree.toggle_dir(&dir),
                    FilesMsg::SelectFile(file_path) => {
                        let ViewerHook::OpenFile(target) = self.file_tree.select_file(file_path);
                        self.file_viewers
                            .entry(target.clone())
                            .or_insert_with(|| FileViewerState::load(&target));
                        self.pane_model.open_file(target);
                    }
                    FilesMsg::CollapseAll => self.file_tree.collapse_all(),
                    FilesMsg::Refresh => self.file_tree.refresh(),
                }
                Task::none()
            }
            Msg::SelectTab(idx) => {
                if let Some(tab) = self.pane_model.activate(idx).cloned() {
                    match tab {
                        PaneTab::Terminal(id) => {
                            if let Some(term) = self.terminals.get_mut(&id) {
                                term.sync_snapshot();
                            }
                        }
                        PaneTab::File(path) => {
                            let _ = self.file_tree.select_file(path);
                        }
                    }
                }
                Task::none()
            }
            Msg::CloseTab(idx) => {
                if let Some(removed) = self.pane_model.close(idx) {
                    match removed {
                        PaneTab::Terminal(id) => {
                            if let Some(mut term) = self.terminals.remove(&id) {
                                term.close_session();
                            }
                        }
                        PaneTab::File(path) => {
                            self.file_viewers.remove(&path);
                        }
                    }
                }
                Task::none()
            }
            Msg::NewTerminalTab => {
                let cwd = self.active_workspace_dir().map(Path::to_path_buf);
                let new_term = self.spawn_terminal_for_dir(cwd.as_deref());
                let id = self.pane_model.open_terminal();
                self.terminals.insert(id, new_term);
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
        }
    }

    fn subscription(&self) -> Subscription<Msg> {
        let Some(PaneTab::Terminal(id)) = self.pane_model.active_tab() else {
            return Subscription::none();
        };
        let Some(term) = self.terminals.get(id) else {
            return Subscription::none();
        };
        term.subscription().map(Msg::Terminal)
    }

    fn view(&self) -> Element<'_, Msg> {
        let workspace_title = self
            .workspaces
            .active()
            .map(|ws| ws.name.as_str())
            .unwrap_or(labels::WORKSPACE_GSH);
        let island_pill = self.island.view_pill().map(Msg::Island);

        let top = topbar::view(
            TopbarProps {
                workspace_title,
                rail_open: self.rail_open,
                files_open: self.files_open,
            },
            island_pill,
            Msg::ToggleRail,
            Msg::ToggleFiles,
        );

        let active_content: Element<'_, Msg> = match self.pane_model.active_tab() {
            Some(PaneTab::Terminal(id)) => self
                .terminals
                .get(id)
                .map(|term| term.view().map(Msg::Terminal))
                .unwrap_or_else(pane_chrome::view_empty_body),
            Some(PaneTab::File(path)) => self
                .file_viewers
                .get(path)
                .map(file_viewer::view)
                .unwrap_or_else(pane_chrome::view_empty_body),
            None => pane_chrome::view_empty_body(),
        };

        let center_pane = pane_chrome::view(
            &self.pane_model,
            active_content,
            ChromeCallbacks {
                on_select_tab: Msg::SelectTab,
                on_close_tab: Msg::CloseTab,
                on_new_terminal: Msg::NewTerminalTab,
                noop_msg: Msg::Noop,
            },
        );

        let mut body = row![].height(Fill).width(Fill);
        if self.rail_open {
            body = body.push(rail::view(
                &self.workspaces,
                Msg::SelectWorkspace,
                Msg::PickWorkspaceFolder,
            ));
        }
        body = body.push(center_pane);
        if self.files_open {
            body = body.push(files_panel::view(&self.file_tree).map(Msg::Files));
        }

        let base = container(column![top, body].width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::BG)),
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

fn main() -> iced::Result {
    let mut app = iced::application(Shell::default, Shell::update, Shell::view)
        .subscription(Shell::subscription)
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
    use gsh_term::{Cell, Cursor, TermSnapshot};

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
        let workspaces = rail::default_workspace_list();
        let active_path = workspaces.active().and_then(|ws| ws.path.clone());
        let mut terminals = BTreeMap::new();
        terminals.insert(1, TerminalPane::from_snapshot_in_dir(snap, active_path));
        Shell {
            island: AgentIsland::default(),
            pane_model: PaneModel::new(),
            terminals,
            file_viewers: BTreeMap::new(),
            workspaces,
            file_tree: FileTree::new(),
            rail_open: true,
            files_open: true,
            static_test_mode: true,
        }
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
        assert!(matches!(
            shell.file_viewers.get(&target_file),
            Some(FileViewerState::Loaded(doc)) if doc.line_count() == 2
        ));

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
}
