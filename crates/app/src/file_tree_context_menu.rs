use iced::Element;

use crate::files_panel::FilesMsg;
use crate::labels;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTreeAction {
    Open,
    OpenWith,
    OpenAsMarkdown,
    OpenAsHtml,
    OpenLayout,
    ShowGitDiff,
    NewFile,
    NewFolder,
    CreateTerminal,
    OpenFileManager,
    Rename,
    Delete,
    Copy,
    Cut,
    Paste,
    CopyRelativePath,
    CopyAbsolutePath,
}

pub fn context_menu(
    path: &std::path::Path,
    is_dir: bool,
    has_git_status: bool,
    can_paste: bool,
    has_item_actions: bool,
) -> Element<'static, FilesMsg> {
    let path = path.to_path_buf();
    let mut items = Vec::new();
    if is_dir {
        items.push(menu_item(
            labels::FILES_MENU_NEW_FILE,
            labels::ICON_NEW_FILE,
            &path,
            FileTreeAction::NewFile,
            true,
        ));
        items.push(menu_item(
            labels::FILES_MENU_NEW_FOLDER,
            labels::ICON_NEW_FOLDER,
            &path,
            FileTreeAction::NewFolder,
            true,
        ));
        items.push(menu_item(
            labels::FILES_MENU_CREATE_TERMINAL,
            labels::ICON_TERMINAL,
            &path,
            FileTreeAction::CreateTerminal,
            true,
        ));
        items.push(menu_item(
            labels::FILES_MENU_OPEN_FILE_MANAGER,
            labels::ICON_FOLDER_OPEN,
            &path,
            FileTreeAction::OpenFileManager,
            true,
        ));
    } else {
        items.push(menu_item(
            labels::FILES_MENU_OPEN,
            labels::ICON_OPEN_IN_NEW,
            &path,
            FileTreeAction::Open,
            true,
        ));
        items.push(menu_item(
            labels::FILES_MENU_OPEN_WITH,
            labels::ICON_OPEN_IN_NEW,
            &path,
            FileTreeAction::OpenWith,
            true,
        ));
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        if name.ends_with(".kanban") {
            items.push(menu_item(
                labels::FILES_MENU_OPEN_AS_MARKDOWN,
                labels::ICON_EDIT,
                &path,
                FileTreeAction::OpenAsMarkdown,
                true,
            ));
        }
        if name.ends_with(".panel") {
            items.push(menu_item(
                labels::FILES_MENU_OPEN_AS_HTML,
                labels::ICON_EDIT,
                &path,
                FileTreeAction::OpenAsHtml,
                true,
            ));
        }
        if name.ends_with(".ckp") {
            items.push(menu_item(
                labels::FILES_MENU_OPEN_LAYOUT,
                labels::ICON_GRID_ON,
                &path,
                FileTreeAction::OpenLayout,
                true,
            ));
        }
        items.push(menu_item(
            labels::FILES_MENU_SHOW_GIT_DIFF,
            labels::ICON_DIFF,
            &path,
            FileTreeAction::ShowGitDiff,
            has_git_status,
        ));
    }
    if has_item_actions {
        items.extend([
            menu_item(
                labels::FILES_MENU_RENAME,
                labels::ICON_EDIT,
                &path,
                FileTreeAction::Rename,
                true,
            ),
            menu_item(
                labels::FILES_MENU_DELETE,
                labels::ICON_DELETE_OUTLINE,
                &path,
                FileTreeAction::Delete,
                true,
            ),
            menu_item(
                labels::FILES_MENU_COPY,
                labels::ICON_CONTENT_COPY,
                &path,
                FileTreeAction::Copy,
                true,
            ),
            menu_item(
                labels::FILES_MENU_CUT,
                labels::ICON_CONTENT_CUT,
                &path,
                FileTreeAction::Cut,
                true,
            ),
        ]);
    }
    items.extend([
        menu_item(
            labels::FILES_MENU_PASTE,
            labels::ICON_CONTENT_PASTE,
            &path,
            FileTreeAction::Paste,
            can_paste,
        ),
        menu_item(
            labels::FILES_MENU_COPY_RELATIVE_PATH,
            labels::ICON_CONTENT_COPY,
            &path,
            FileTreeAction::CopyRelativePath,
            true,
        ),
        menu_item(
            labels::FILES_MENU_COPY_ABSOLUTE_PATH,
            labels::ICON_CONTENT_COPY,
            &path,
            FileTreeAction::CopyAbsolutePath,
            true,
        ),
    ]);
    crate::app_menu::menu(items, 220.0)
}

fn menu_item(
    label: &'static str,
    glyph: &'static str,
    path: &std::path::Path,
    action: FileTreeAction,
    enabled: bool,
) -> crate::app_menu::Item<FilesMsg> {
    use crate::app_menu::{Entry, Item, Leading};

    let entry = Entry {
        label: label.to_string(),
        description: None,
        leading: Some(Leading::Glyph(glyph)),
        selected: false,
        danger: action == FileTreeAction::Delete,
        shortcut: None,
        requires_credits: false,
        submenu: false,
        on_press: enabled.then(|| FilesMsg::ContextAction(path.to_path_buf(), action)),
    };
    Item::Entry(entry)
}
