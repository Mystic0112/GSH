//! Painel lateral direito de arquivos (`file_tree_panel.dart` do Flutter).
//!
//! Superfície `BG`, divisória esquerda de 1px `BORDER`, barra de modo de 40px,
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
    OpenGit,
    StartCreate(CreateKind),
    CreateNameChanged(String),
    SubmitCreate,
    CancelCreate,
    CreateFinished(u64, PathBuf, Result<(), FileOpError>),
}

/// Renderiza o painel de arquivos à direita com a divisória esquerda de 1px `BORDER`.
pub fn view<'a>(
    tree: &'a FileTree,
    status: Option<&'a gsh_git::GitRepoStatus>,
    repo_root: Option<&'a std::path::Path>,
    busy: bool,
    form: Option<&'a CreateForm>,
) -> Element<'a, FilesMsg> {
    let mode_bar = view_mode_bar();
    let section_header = view_section_header(busy, tree.root().is_some(), form.is_some());
    let tree_body = view_tree_body(tree, status, repo_root);
    let footer = view_footer();

    crate::panel_widgets::frame(
        column![
            mode_bar,
            crate::panel_widgets::hairline_h(),
            section_header,
            create_input(form),
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

fn view_section_header<'a>(busy: bool, has_root: bool, creating: bool) -> Element<'a, FilesMsg> {
    container(
        row![
            text(labels::FILES_SECTION_UPPER)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_SECTION_HEADER)
                .color(theme::palette().text3),
            Space::new().width(Fill),
            crate::panel_widgets::ghost(
                labels::ICON_NEW_FOLDER,
                (has_root && !creating).then_some(FilesMsg::StartCreate(CreateKind::Folder)),
                false
            ),
            crate::panel_widgets::ghost(
                labels::ICON_NEW_FILE,
                (has_root && !creating).then_some(FilesMsg::StartCreate(CreateKind::File)),
                false
            ),
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
        .style(|_, status| {
            let p = theme::palette();
            text_input::Style {
                background: p.panel2.into(),
                border: Border {
                    color: if matches!(status, text_input::Status::Focused { .. }) {
                        p.accent
                    } else {
                        p.border2
                    },
                    width: 1.0,
                    radius: theme::RADIUS_BUTTON.into(),
                },
                icon: p.text3,
                placeholder: p.text3,
                value: p.text,
                selection: p.accent_soft,
            }
        });
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
    let indent = (depth as f32) * 14.0 + 6.0;
    let chevron = if entry.is_dir {
        if expanded {
            labels::ICON_CHEVRON_DOWN
        } else {
            labels::ICON_CHEVRON_RIGHT
        }
    } else {
        ""
    };
    let icon = if entry.is_dir {
        labels::ICON_FOLDER
    } else {
        labels::ICON_FILE_ITEM
    };
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

    let mut row_content = row![
        Space::new().width(indent),
        container(
            text(chevron)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TOPBAR_TITLE)
                .color(theme::palette().text3)
        )
        .width(14),
        text(icon)
            .font(theme::FONT_ICON)
            .size(theme::SIZE_TOPBAR_TITLE)
            .color(theme::palette().text3),
        Space::new().width(6.0),
        text(entry.name)
            .font(theme::FONT_BODY)
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
        .height(22)
        .padding([0, 6])
        .on_press(msg)
        .style(move |_: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let bg = if selected {
                Some(Background::Color(theme::palette().panel2))
            } else if hovered {
                Some(Background::Color(theme::palette().panel3))
            } else {
                None
            };
            button::Style {
                background: bg,
                text_color: name_color,
                border: Border {
                    color: theme::palette().border,
                    width: 0.0,
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
            .color(theme::palette().text3),
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
