// Shell da UI do GSH no Linux (iced). Layout de três colunas com a barra de
// título e ilha de agentes no topo, espelhando o GSH Desktop (Flutter):
// rail de workspaces à esquerda, pane de terminal com abas no centro e
// painel de arquivos à direita.
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
pub mod files_panel;
pub mod gorilla;
pub mod island;
pub mod labels;
pub mod pane_chrome;
pub mod rail;
pub mod terminal_canvas;
pub mod terminal_grid;
pub mod terminal_input;
pub mod terminal_pane;
pub mod theme;
pub mod topbar;

use std::path::PathBuf;

use gsh_core::workspace::WorkspaceList;
use iced::widget::{column, container, mouse_area, row, stack, Space};
use iced::{Background, Element, Fill, Padding, Subscription, Task, Theme};

use crate::file_tree::{FileTree, ViewerHook};
use crate::files_panel::FilesMsg;
use crate::island::{AgentIsland, IslandMsg};
use crate::terminal_pane::{TerminalMsg, TerminalPane};
use crate::topbar::TopbarProps;

#[derive(Debug, Clone)]
enum Msg {
    Island(IslandMsg),
    Terminal(TerminalMsg),
    Files(FilesMsg),
    SelectWorkspace(usize),
    PickWorkspaceFolder,
    WorkspaceFolderPicked(Option<PathBuf>),
    ToggleRail,
    ToggleFiles,
    Noop,
}

struct Shell {
    island: AgentIsland,
    terminal: TerminalPane,
    workspaces: WorkspaceList,
    file_tree: FileTree,
    /// Ponto de integração para a próxima fase (abertura do viewer de arquivo).
    pending_viewer_file: Option<PathBuf>,
    rail_open: bool,
    files_open: bool,
}

impl Default for Shell {
    fn default() -> Self {
        let workspaces = rail::default_workspace_list();
        let active_path = workspaces.active().and_then(|ws| ws.path.clone());
        let terminal = TerminalPane::new_in_dir(active_path.as_deref());
        let mut file_tree = FileTree::new();
        file_tree.set_root(active_path);
        Self {
            island: AgentIsland::default(),
            terminal,
            workspaces,
            file_tree,
            pending_viewer_file: None,
            rail_open: true,
            files_open: true,
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
    fn activate_workspace_path(&mut self, next_path: Option<PathBuf>) {
        self.file_tree.set_root(next_path.clone());
        self.pending_viewer_file = None;
        if let Some(dir) = next_path.as_deref() {
            self.terminal.respawn_in_dir(Some(dir));
        }
    }

    fn update(&mut self, msg: Msg) -> Task<Msg> {
        match msg {
            Msg::Island(island_msg) => {
                self.island.update(island_msg);
                Task::none()
            }
            Msg::Terminal(term_msg) => self.terminal.update(term_msg).map(Msg::Terminal),
            Msg::Files(files_msg) => {
                match files_msg {
                    FilesMsg::ToggleDir(dir) => self.file_tree.toggle_dir(&dir),
                    FilesMsg::SelectFile(file_path) => {
                        // Ponto de integração do viewer (próxima fase):
                        // por enquanto destaca a linha selecionada e registra o caminho alvo.
                        let ViewerHook::PendingViewer(target) =
                            self.file_tree.select_file(file_path);
                        self.pending_viewer_file = Some(target);
                    }
                    FilesMsg::CollapseAll => self.file_tree.collapse_all(),
                    FilesMsg::Refresh => self.file_tree.refresh(),
                }
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
        self.terminal.subscription().map(Msg::Terminal)
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

        let center_pane = pane_chrome::view(self.terminal.view().map(Msg::Terminal), Msg::Noop);

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
        Shell {
            island: AgentIsland::default(),
            terminal: TerminalPane::from_snapshot(snap),
            workspaces: rail::default_workspace_list(),
            file_tree: FileTree::new(),
            pending_viewer_file: None,
            rail_open: true,
            files_open: true,
        }
    }

    #[test]
    fn shell_toggles_panels_switches_and_opens_project_workspace() {
        let mut shell = test_shell();
        assert!(shell.rail_open);
        assert!(shell.files_open);
        assert_eq!(shell.workspaces.selected_index(), 0);

        let _ = shell.update(Msg::SelectWorkspace(1));
        assert_eq!(shell.workspaces.selected_index(), 1);

        // Índice fora dos limites é ignorado
        let _ = shell.update(Msg::SelectWorkspace(99));
        assert_eq!(shell.workspaces.selected_index(), 1);

        // Abre uma pasta escolhida no seletor
        let picked = std::env::temp_dir();
        let _ = shell.update(Msg::WorkspaceFolderPicked(Some(picked.clone())));
        assert_eq!(shell.workspaces.selected_index(), 2);
        assert_eq!(shell.file_tree.root(), Some(picked.as_path()));

        // Seleciona um arquivo na árvore e verifica o ponto de integração do viewer
        let target_file = picked.join("README.md");
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(target_file.clone())));
        assert_eq!(shell.pending_viewer_file, Some(target_file));

        let _ = shell.update(Msg::ToggleRail);
        assert!(!shell.rail_open);

        let _ = shell.update(Msg::ToggleFiles);
        assert!(!shell.files_open);
    }
}
