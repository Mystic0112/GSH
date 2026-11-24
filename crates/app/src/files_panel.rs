//! Aba Arquivos do painel direito, portada do `file_tree_panel.dart` do Flutter.

use std::path::PathBuf;

use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

use crate::agent_file_marks::{self, AgentFiles};
use crate::file_ops::{CreateForm, CreateKind, FileOpError, RenameForm};
use crate::file_tree::{FileEntry, FileTree, TreeRow};
use crate::labels;
use crate::theme;

pub use crate::file_tree_context_menu::FileTreeAction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesMsg {
    ToggleDir(PathBuf),
    SelectFile(PathBuf),
    ClearSelection,
    CopySelected,
    CutSelected,
    PasteSelected,
    RenameSelected,
    DeleteSelected,
    CollapseAll,
    Refresh,
    StartCreate(CreateKind),
    CreateNameChanged(String),
    SubmitCreate,
    CancelCreate,
    CreateFinished(u64, PathBuf, Result<(), FileOpError>),
    StartRename(PathBuf),
    RenameNameChanged(String),
    SubmitRename,
    CancelRename,
    RenameFinished(u64, PathBuf, Result<(), FileOpError>),
    RequestDelete(PathBuf),
    ConfirmDelete,
    CancelDelete,
    DeleteFinished(u64, PathBuf, PathBuf, Result<(), FileOpError>),
    PasteFinished(
        PathBuf,
        PathBuf,
        PathBuf,
        bool,
        Result<PathBuf, FileOpError>,
    ),
    DismissOperationNotice,
    OpenContextMenu(PathBuf),
    CloseContextMenu,
    ContextAction(PathBuf, FileTreeAction),
    ToggleAgentMenu,
    CloseAgentMenu,
    SetAgentFilter(Option<String>),
}

pub fn view<'a>(
    tree: &'a FileTree,
    status: Option<&'a gsh_git::GitRepoStatus>,
    repo_root: Option<&'a std::path::Path>,
    busy: bool,
    form: Option<&'a CreateForm>,
    rename: Option<&'a RenameForm>,
    agents: AgentFiles<'_>,
) -> Element<'a, FilesMsg> {
    let has_root = tree.root().is_some();
    let mut body = column![];
    if has_root {
        body = body.push(view_section_header(busy, form.is_some(), agents));
    }
    // Com filtro por agente a árvore vira a lista plana do que ele tocou.
    let content = match agents.filter {
        Some(tab_id) if has_root => agent_file_marks::touched_list(agents, tab_id, tree.root()),
        _ => view_tree_body(tree, status, repo_root, form, rename, agents),
    };
    body.push(content)
        .push(crate::panel_widgets::hairline_h())
        .push(view_footer())
        .into()
}

fn view_section_header<'a>(
    busy: bool,
    creating: bool,
    agents: AgentFiles<'_>,
) -> Element<'a, FilesMsg> {
    crate::panel_widgets::section_header(
        labels::FILES_SECTION_UPPER,
        vec![
            agent_file_marks::filter_button(agents),
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

fn create_input(form: &CreateForm, depth: usize) -> Element<'_, FilesMsg> {
    let hint = match form.kind {
        CreateKind::Folder => labels::FILES_NEW_FOLDER_HINT,
        CreateKind::File => labels::FILES_NEW_FILE_HINT,
    };
    let icon = match form.kind {
        CreateKind::Folder => crate::file_icons::view("new", true, false),
        CreateKind::File => crate::file_icons::view("new file", false, false),
    };
    let input = text_input(hint, &form.name)
        .id("files-create")
        .on_input_maybe(
            (!form.busy).then_some(FilesMsg::CreateNameChanged as fn(String) -> FilesMsg),
        )
        .on_submit_maybe((!form.busy).then_some(FilesMsg::SubmitCreate))
        .style(crate::panel_widgets::input_style);
    let content = row![
        Space::new().width((depth as f32) * theme::FILE_TREE_ROW_INDENT),
        Space::new().width(theme::FILE_TREE_CHEVRON_WIDTH + theme::FILE_TREE_CHEVRON_GAP),
        icon,
        Space::new().width(theme::FILE_TREE_ICON_GAP),
        input.width(Fill),
    ]
    .align_y(Alignment::Center);
    if let Some(error) = form.error {
        return column![
            content,
            text(error.label())
                .size(theme::SIZE_LABEL)
                .color(theme::palette().error)
        ]
        .padding(Padding {
            top: 0.0,
            right: theme::FILE_TREE_ROW_PADDING,
            bottom: 0.0,
            left: (depth as f32) * theme::FILE_TREE_ROW_INDENT,
        })
        .into();
    }
    container(content)
        .height(theme::FILE_TREE_ROW_HEIGHT)
        .padding([0.0, theme::FILE_TREE_ROW_PADDING])
        .width(Fill)
        .into()
}

fn view_tree_body<'a>(
    tree: &'a FileTree,
    status: Option<&'a gsh_git::GitRepoStatus>,
    repo_root: Option<&'a std::path::Path>,
    form: Option<&'a CreateForm>,
    rename: Option<&'a RenameForm>,
    agents: AgentFiles<'_>,
) -> Element<'a, FilesMsg> {
    if tree.root().is_none() {
        return crate::panel_widgets::empty_label(labels::FILES_NO_FOLDER);
    }

    let rows = tree.visible_rows();
    let mut list = column![].spacing(0).padding([
        theme::FILE_TREE_SCROLL_PADDING_VERTICAL,
        theme::FILE_TREE_ROW_PADDING,
    ]);
    let Some(root) = tree.root() else {
        return crate::panel_widgets::empty_label(labels::FILES_NO_FOLDER);
    };
    if let Some(form) = form.filter(|form| root == form.target_dir.as_path()) {
        list = list.push(create_input(form, 0));
    }
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
                // Só arquivo ganha as marcas de agente (o Flutter liga `agentMarks` na linha de arquivo).
                let marks = if entry.is_dir {
                    None
                } else {
                    agent_file_marks::marks(agents, &entry.path)
                };
                let renaming = rename.filter(|rename| rename.path == entry.path);
                let empty_dir = entry.is_dir && expanded && tree.is_expanded_empty_dir(&entry.path);
                let empty_path = entry.path.clone();
                let create_form = form.filter(|form| form.target_dir == empty_path);
                list = list.push(view_entry_row(
                    tree,
                    entry,
                    EntryRowState {
                        depth,
                        expanded,
                        selected,
                        git_status,
                        marks,
                        renaming,
                    },
                ));
                if let Some(form) = create_form {
                    list = list.push(create_input(form, depth.saturating_add(1)));
                }
                if empty_dir {
                    list = list.push(empty_folder_slot(tree, &empty_path));
                }
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

    let root_path = PathBuf::from(root);
    let root_menu = (tree.context_target() == Some(root_path.as_path())).then(|| {
        crate::file_tree_context_menu::context_menu(
            &root_path,
            true,
            false,
            tree.can_paste(&root_path),
            false,
        )
    });
    crate::anchored::Anchored::new(
        iced::widget::mouse_area(scrollable(list.width(Fill)).width(Fill).height(Fill))
            .on_press(FilesMsg::ClearSelection)
            .on_right_press(FilesMsg::OpenContextMenu(root_path)),
        root_menu,
        FilesMsg::CloseContextMenu,
    )
    .on_escape(FilesMsg::CloseContextMenu)
    .into()
}

fn empty_folder_slot<'a>(tree: &'a FileTree, path: &std::path::Path) -> Element<'a, FilesMsg> {
    let popup = (tree.context_target() == Some(path)).then(|| {
        crate::file_tree_context_menu::context_menu(path, true, false, tree.can_paste(path), false)
    });
    crate::anchored::Anchored::new(
        iced::widget::mouse_area(Space::new().width(Fill).height(theme::FILE_TREE_ROW_HEIGHT))
            .on_right_press(FilesMsg::OpenContextMenu(path.to_path_buf())),
        popup,
        FilesMsg::CloseContextMenu,
    )
    .on_escape(FilesMsg::CloseContextMenu)
    .into()
}

struct EntryRowState<'a> {
    depth: usize,
    expanded: bool,
    selected: bool,
    git_status: Option<gsh_git::GitFileStatus>,
    marks: Option<Element<'a, FilesMsg>>,
    renaming: Option<&'a RenameForm>,
}

fn view_entry_row<'a>(
    tree: &'a FileTree,
    entry: FileEntry,
    state: EntryRowState<'a>,
) -> Element<'a, FilesMsg> {
    let indent = (state.depth as f32) * theme::FILE_TREE_ROW_INDENT;
    let chevron = if entry.is_dir {
        if state.expanded {
            labels::ICON_CHEVRON_DOWN
        } else {
            labels::ICON_CHEVRON_RIGHT
        }
    } else {
        ""
    };
    let icon = crate::file_icons::view(&entry.name, entry.is_dir, state.expanded);
    let name_color = if let Some(status) = state.git_status {
        crate::git_presenter::status_color(status)
    } else if state.selected {
        theme::palette().text
    } else {
        theme::palette().text2
    };
    let msg = if entry.is_dir {
        FilesMsg::ToggleDir(entry.path.clone())
    } else {
        FilesMsg::SelectFile(entry.path.clone())
    };

    let label: Element<'a, FilesMsg> = if let Some(form) = state.renaming {
        let input = text_input("", &form.name)
            .id("files-rename")
            .on_input_maybe(
                (!form.busy).then_some(FilesMsg::RenameNameChanged as fn(String) -> FilesMsg),
            )
            .on_submit_maybe((!form.busy).then_some(FilesMsg::SubmitRename))
            .style(crate::panel_widgets::input_style);
        if let Some(error) = form.error {
            column![
                input,
                text(error.label())
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().error),
            ]
            .spacing(2.0)
            .into()
        } else {
            input.into()
        }
    } else {
        text(entry.name.clone())
            .font(theme::FONT_BODY)
            .size(theme::FILE_TREE_NAME_SIZE)
            .color(name_color)
            .wrapping(iced::widget::text::Wrapping::None)
            .into()
    };
    let mut row_content = row![
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
        container(label).width(Fill).clip(true),
    ]
    .align_y(Alignment::Center);
    if let Some(marks) = state.marks {
        row_content = row_content.push(marks);
    }

    let row = button(row_content)
        .width(Fill)
        .height(if state.renaming.is_some() {
            Length::Shrink
        } else {
            Length::Fixed(theme::FILE_TREE_ROW_HEIGHT)
        })
        .padding([0.0, theme::FILE_TREE_ROW_PADDING])
        .on_press(msg)
        .style(move |_: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let bg = if state.selected {
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
        });
    let is_dir = entry.is_dir && !entry.name.to_ascii_lowercase().ends_with(".notebook");
    let paste_dir = if is_dir {
        entry.path.as_path()
    } else {
        entry.path.parent().unwrap_or(entry.path.as_path())
    };
    let can_paste = tree.can_paste(paste_dir);
    let menu = crate::file_tree_context_menu::context_menu(
        &entry.path,
        is_dir,
        state.git_status.is_some(),
        can_paste,
        true,
    );
    let is_open = tree.context_target() == Some(entry.path.as_path());
    crate::anchored::Anchored::new(
        iced::widget::mouse_area(row).on_right_press(FilesMsg::OpenContextMenu(entry.path.clone())),
        is_open.then_some(menu),
        FilesMsg::CloseContextMenu,
    )
    .on_escape(FilesMsg::CloseContextMenu)
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
