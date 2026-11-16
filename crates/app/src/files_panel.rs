//! Aba Arquivos do painel direito (`file_tree_panel.dart` do Flutter).
//!
//! Corpo da aba (a moldura e a barra de abas ficam em `right_tabs`):
//! cabeçalho "ARQUIVOS" com ações de recolher/atualizar, árvore de arquivos
//! interativa (pastas expansíveis de forma lazy e seleção de arquivo) e rodapé
//! de status LSP de 34px.

use std::path::PathBuf;

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

use crate::file_ops::{CreateForm, CreateKind, FileOpError};
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
    StartCreate(CreateKind),
    CreateNameChanged(String),
    SubmitCreate,
    CancelCreate,
    CreateFinished(u64, PathBuf, Result<(), FileOpError>),
}

/// Corpo da aba Arquivos: cabeçalho (só com pasta), árvore e rodapé de status LSP.
pub fn view<'a>(
    tree: &'a FileTree,
    status: Option<&'a gsh_git::GitRepoStatus>,
    repo_root: Option<&'a std::path::Path>,
    busy: bool,
    form: Option<&'a CreateForm>,
) -> Element<'a, FilesMsg> {
    let has_root = tree.root().is_some();
    let mut body = column![];
    if has_root {
        body = body.push(view_section_header(busy, form.is_some()));
    }
    body.push(create_input(form))
        .push(view_tree_body(tree, status, repo_root))
        .push(crate::panel_widgets::hairline_h())
        .push(view_footer())
        .into()
}

fn view_section_header<'a>(busy: bool, creating: bool) -> Element<'a, FilesMsg> {
    crate::panel_widgets::section_header(
        labels::FILES_SECTION_UPPER,
        vec![
            header_action(
                labels::ICON_NEW_FILE,
                (!creating).then_some(FilesMsg::StartCreate(CreateKind::File)),
            ),
            header_action(
                labels::ICON_NEW_FOLDER,
                (!creating).then_some(FilesMsg::StartCreate(CreateKind::Folder)),
            ),
            header_action(labels::ICON_COLLAPSE, Some(FilesMsg::CollapseAll)),
            header_action(labels::ICON_REFRESH, (!busy).then_some(FilesMsg::Refresh)),
        ],
    )
}

fn header_action<'a>(glyph: &'static str, message: Option<FilesMsg>) -> Element<'a, FilesMsg> {
    button(
        text(glyph)
            .font(theme::FONT_ICON)
            .size(theme::FILE_PANEL_HEADER_ACTION_SIZE),
    )
    .padding(theme::FILE_PANEL_HEADER_ACTION_PADDING)
    .on_press_maybe(message)
    .style(|_, status| button::Style {
        background: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(theme::palette().panel3.into())
        } else {
            None
        },
        text_color: theme::palette().text3,
        border: Border {
            radius: theme::RADIUS_BADGE.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn create_input(form: Option<&CreateForm>) -> Element<'_, FilesMsg> {
    let Some(form) = form else {
        return Space::new().height(0).into();
    };
    let hint = match form.kind {
        CreateKind::Folder => labels::FILES_NEW_FOLDER_HINT,
        CreateKind::File => labels::FILES_NEW_FILE_HINT,
    };
    let input = text_input(hint, &form.name)
        .id("files-create")
        .on_input_maybe(
            (!form.busy).then_some(FilesMsg::CreateNameChanged as fn(String) -> FilesMsg),
        )
        .on_submit_maybe((!form.busy).then_some(FilesMsg::SubmitCreate))
        .style(crate::panel_widgets::input_style);
    let mut content = column![input].spacing(4);
    if let Some(error) = form.error {
        content = content.push(
            text(error.label())
                .size(theme::SIZE_LABEL)
                .color(theme::palette().error),
        );
    }
    container(content).padding([0, 12]).width(Fill).into()
}

fn view_tree_body<'a>(
    tree: &'a FileTree,
    status: Option<&'a gsh_git::GitRepoStatus>,
    repo_root: Option<&'a std::path::Path>,
) -> Element<'a, FilesMsg> {
    if tree.root().is_none() {
        return crate::panel_widgets::empty_label(labels::FILES_NO_FOLDER);
    }

    let rows = tree.visible_rows();
    let mut list = column![].spacing(0).padding([
        theme::FILE_TREE_SCROLL_PADDING_VERTICAL,
        theme::FILE_TREE_ROW_PADDING,
    ]);
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
                let indent = (depth as f32) * 12.0 + 8.0;
                list = list.push(
                    container(
                        text(error.ui_message())
                            .font(theme::FONT_BODY)
                            .size(theme::SIZE_LABEL)
                            .color(theme::palette().error),
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
    let indent = (depth as f32) * theme::FILE_TREE_ROW_INDENT;
    let chevron = if entry.is_dir {
        if expanded {
            labels::ICON_CHEVRON_DOWN
        } else {
            labels::ICON_CHEVRON_RIGHT
        }
    } else {
        ""
    };
    let icon = crate::file_icons::view(&entry.name, entry.is_dir, expanded);
    let name_color = if let Some(status) = git_status {
        crate::git_presenter::status_color(status)
    } else if selected {
        theme::palette().text
    } else {
        theme::palette().text2
    };
    let msg = if entry.is_dir {
        FilesMsg::ToggleDir(entry.path)
    } else {
        FilesMsg::SelectFile(entry.path)
    };

    let row_content = row![
        Space::new().width(indent),
        container(
            text(chevron)
                .font(theme::FONT_ICON)
                .size(theme::FILE_TREE_CHEVRON_SIZE)
                .color(theme::palette().text4)
        )
        .width(theme::FILE_TREE_CHEVRON_WIDTH),
        Space::new().width(theme::FILE_TREE_CHEVRON_GAP),
        icon,
        Space::new().width(theme::FILE_TREE_ICON_GAP),
        container(
            text(entry.name)
                .font(theme::FONT_BODY)
                .size(theme::FILE_TREE_NAME_SIZE)
                .color(name_color)
                .wrapping(iced::widget::text::Wrapping::None)
        )
        .width(Fill)
        .clip(true),
    ]
    .align_y(Alignment::Center);

    button(row_content)
        .width(Fill)
        .height(theme::FILE_TREE_ROW_HEIGHT)
        .padding([0.0, theme::FILE_TREE_ROW_PADDING])
        .on_press(msg)
        .style(move |_: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let bg = if selected {
                Some(Background::Color(theme::palette().panel2))
            } else if hovered {
                Some(Background::Color(theme::palette().panel))
            } else {
                None
            };
            button::Style {
                background: bg,
                text_color: name_color,
                border: Border {
                    color: theme::palette().border,
                    width: 0.0,
                    radius: theme::FILE_TREE_ROW_RADIUS.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn view_footer<'a>() -> Element<'a, FilesMsg> {
    container(
        row![
            text(labels::NO_LSP_AVAILABLE)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text4),
            Space::new().width(Fill),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::FOOTER_HEIGHT))
    .padding([0, 12])
    .into()
}
