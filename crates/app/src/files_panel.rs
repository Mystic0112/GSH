//! Painel lateral direito de arquivos (`file_tree_panel.dart` do Flutter).
//!
//! Superfície `BG`, divisória esquerda de 1px `BORDER`, barra de modo de 40px,
//! cabeçalho "ARQUIVOS" com ações de recolher/atualizar, árvore de arquivos
//! interativa (pastas expansíveis de forma lazy e seleção de arquivo) e rodapé
//! de status LSP de 34px.

use std::path::PathBuf;

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

use crate::file_tree::{FileEntry, FileTree, TreeRow};
use crate::labels;
use crate::theme;

/// Mensagens emitidas pelo painel de arquivos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesMsg {
    /// Expande ou recolhe uma pasta na árvore.
    ToggleDir(PathBuf),
    /// Clique simples num arquivo na árvore: seleciona e abre/ativa sua aba no pane central.
    SelectFile(PathBuf),
    /// Recolhe todas as pastas expandidas.
    CollapseAll,
    /// Atualiza os diretórios abertos no painel.
    Refresh,
    OpenGit,
}

/// Renderiza o painel de arquivos à direita com a divisória esquerda de 1px `BORDER`.
pub fn view<'a>(
    tree: &'a FileTree,
    status: Option<&'a gsh_git::GitRepoStatus>,
    repo_root: Option<&'a std::path::Path>,
    busy: bool,
) -> Element<'a, FilesMsg> {
    let mode_bar = view_mode_bar();
    let section_header = view_section_header(busy);
    let tree_body = view_tree_body(tree, status, repo_root);
    let footer = view_footer();

    crate::panel_widgets::frame(
        column![
            mode_bar,
            crate::panel_widgets::hairline_h(),
            section_header,
            tree_body,
            crate::panel_widgets::hairline_h(),
            footer
        ]
        .into(),
    )
}

fn view_mode_bar<'a>() -> Element<'a, FilesMsg> {
    container(
        row![
            crate::panel_widgets::ghost(labels::ICON_FOLDER, None, true),
            crate::panel_widgets::ghost(labels::ICON_GIT_BRANCH, Some(FilesMsg::OpenGit), false),
            Space::new().width(Fill)
        ]
        .align_y(Alignment::Center)
        .padding([0, 14]),
    )
    .width(Fill)
    .height(Length::Fixed(theme::TAB_STRIP_HEIGHT - 1.0))
    .into()
}

fn view_section_header<'a>(busy: bool) -> Element<'a, FilesMsg> {
    container(
        row![
            text(labels::FILES_SECTION_UPPER)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_SECTION_HEADER)
                .color(theme::TEXT3),
            Space::new().width(Fill),
            crate::panel_widgets::ghost(labels::ICON_COLLAPSE, Some(FilesMsg::CollapseAll), false),
            Space::new().width(4.0),
            crate::panel_widgets::ghost(
                labels::ICON_REFRESH,
                (!busy).then_some(FilesMsg::Refresh),
                false
            ),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(Padding {
        top: 10.0,
        right: 12.0,
        bottom: 6.0,
        left: 14.0,
    })
    .into()
}

fn view_tree_body<'a>(
    tree: &'a FileTree,
    status: Option<&'a gsh_git::GitRepoStatus>,
    repo_root: Option<&'a std::path::Path>,
) -> Element<'a, FilesMsg> {
    if tree.root().is_none() {
        return empty_state_label(labels::FILES_NO_FOLDER);
    }

    let rows = tree.visible_rows();
    if rows.is_empty() {
        return empty_state_label(labels::FILES_EMPTY_FOLDER);
    }

    let mut list = column![].spacing(1.0).padding([4, 6]);
    for item in rows {
        match item {
            TreeRow::Entry {
                entry,
                depth,
                expanded,
                selected,
            } => {
                let git_status = repo_root
                    .and_then(|root| status.and_then(|s| s.status_for_abs_path(root, &entry.path)));
                list = list.push(view_entry_row(entry, depth, expanded, selected, git_status));
            }
            TreeRow::Error { depth, error } => {
                let indent = (depth as f32) * 14.0 + 8.0;
                list = list.push(
                    container(
                        text(error.ui_message())
                            .font(theme::FONT_BODY)
                            .size(theme::SIZE_LABEL)
                            .color(theme::ERROR),
                    )
                    .padding(Padding {
                        top: 4.0,
                        right: 8.0,
                        bottom: 4.0,
                        left: indent,
                    }),
                );
            }
        }
    }

    scrollable(list.width(Fill)).width(Fill).height(Fill).into()
}

fn view_entry_row<'a>(
    entry: FileEntry,
    depth: usize,
    expanded: bool,
    selected: bool,
    git_status: Option<gsh_git::GitFileStatus>,
) -> Element<'a, FilesMsg> {
    let indent = (depth as f32) * 14.0 + 6.0;
    let icon = if entry.is_dir {
        if expanded {
            labels::ICON_CHEVRON_DOWN
        } else {
            labels::ICON_CHEVRON_RIGHT
        }
    } else {
        labels::ICON_FILE_ITEM
    };
    let icon_color = if entry.is_dir {
        theme::ACCENT_TEXT
    } else if selected {
        theme::TEXT
    } else {
        theme::TEXT3
    };
    let name_font = if entry.is_dir || selected {
        theme::FONT_BODY_MEDIUM
    } else {
        theme::FONT_BODY
    };
    let name_color = if let Some(status) = git_status {
        crate::git_presenter::status_color(status)
    } else if selected || entry.is_dir {
        theme::TEXT
    } else {
        theme::TEXT2
    };
    let msg = if entry.is_dir {
        FilesMsg::ToggleDir(entry.path)
    } else {
        FilesMsg::SelectFile(entry.path)
    };

    let mut row_content = row![
        Space::new().width(indent),
        text(icon)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_LABEL)
            .color(icon_color),
        Space::new().width(6.0),
        text(entry.name)
            .font(name_font)
            .size(theme::SIZE_LABEL)
            .color(name_color),
    ]
    .align_y(Alignment::Center);
    if let Some(status) = git_status {
        row_content = row_content.push(Space::new().width(Fill)).push(
            text(status.badge_code())
                .color(crate::git_presenter::status_color(status))
                .size(theme::SIZE_LABEL),
        );
    }

    button(row_content)
        .width(Fill)
        .padding([4, 6])
        .on_press(msg)
        .style(move |_: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let bg = if selected {
                Some(Background::Color(theme::PANEL2))
            } else if hovered {
                Some(Background::Color(theme::PANEL3))
            } else {
                None
            };
            button::Style {
                background: bg,
                text_color: name_color,
                border: Border {
                    color: if selected {
                        theme::ACCENT
                    } else {
                        theme::BORDER
                    },
                    width: if selected { 1.0 } else { 0.0 },
                    radius: theme::RADIUS_BUTTON.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn empty_state_label<'a>(msg: &'static str) -> Element<'a, FilesMsg> {
    container(
        text(msg)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::TEXT3),
    )
    .width(Fill)
    .height(Fill)
    .padding([8, 14])
    .into()
}

fn view_footer<'a>() -> Element<'a, FilesMsg> {
    container(
        row![
            text(labels::NO_LSP_AVAILABLE)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::TEXT4),
            Space::new().width(Fill),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::FOOTER_HEIGHT))
    .padding([0, 12])
    .into()
}
