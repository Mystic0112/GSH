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
}

/// Renderiza o painel de arquivos à direita com a divisória esquerda de 1px `BORDER`.
pub fn view(tree: &FileTree) -> Element<'_, FilesMsg> {
    let mode_bar = view_mode_bar();
    let section_header = view_section_header();
    let tree_body = view_tree_body(tree);
    let footer = view_footer();

    let content = container(column![
        mode_bar,
        hairline_h(),
        section_header,
        tree_body,
        hairline_h(),
        footer,
    ])
    .width(Length::Fixed(theme::FILES_PANEL_WIDTH - 1.0))
    .height(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::BG)),
        ..Default::default()
    });

    row![hairline_v(), content]
        .width(Length::Fixed(theme::FILES_PANEL_WIDTH))
        .height(Fill)
        .into()
}

fn view_mode_bar<'a>() -> Element<'a, FilesMsg> {
    let mode_button = container(
        text(labels::ICON_FOLDER)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_TITLE)
            .color(theme::ACCENT_TEXT),
    )
    .width(Length::Fixed(28.0))
    .height(Length::Fixed(28.0))
    .center_x(Length::Fixed(28.0))
    .center_y(Length::Fixed(28.0))
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::ACCENT_SOFT)),
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    container(
        row![mode_button, Space::new().width(Fill)]
            .width(Fill)
            .height(Length::Fixed(theme::TAB_STRIP_HEIGHT - 1.0))
            .align_y(Alignment::Center)
            .padding([0, 14]),
    )
    .width(Fill)
    .height(Length::Fixed(theme::TAB_STRIP_HEIGHT - 1.0))
    .into()
}

fn view_section_header<'a>() -> Element<'a, FilesMsg> {
    container(
        row![
            text(labels::FILES_SECTION_UPPER)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_SECTION_HEADER)
                .color(theme::TEXT3),
            Space::new().width(Fill),
            header_action_button(labels::ICON_COLLAPSE, FilesMsg::CollapseAll),
            Space::new().width(4.0),
            header_action_button(labels::ICON_REFRESH, FilesMsg::Refresh),
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

fn view_tree_body(tree: &FileTree) -> Element<'_, FilesMsg> {
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
                list = list.push(view_entry_row(entry, depth, expanded, selected));
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
    let name_color = if selected || entry.is_dir {
        theme::TEXT
    } else {
        theme::TEXT2
    };
    let msg = if entry.is_dir {
        FilesMsg::ToggleDir(entry.path)
    } else {
        FilesMsg::SelectFile(entry.path)
    };

    let row_content = row![
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

fn header_action_button<'a>(glyph: &'static str, msg: FilesMsg) -> Element<'a, FilesMsg> {
    button(
        container(
            text(glyph)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(theme::TEXT3),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(22.0))
    .height(Length::Fixed(22.0))
    .padding(0)
    .on_press(msg)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::PANEL3))
            } else {
                None
            },
            text_color: if hovered { theme::TEXT2 } else { theme::TEXT3 },
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

fn hairline_h<'a>() -> Element<'a, FilesMsg> {
    container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::BORDER)),
            ..Default::default()
        })
        .into()
}

fn hairline_v<'a>() -> Element<'a, FilesMsg> {
    container(Space::new().width(1.0).height(Fill))
        .width(Length::Fixed(1.0))
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::BORDER)),
            ..Default::default()
        })
        .into()
}
