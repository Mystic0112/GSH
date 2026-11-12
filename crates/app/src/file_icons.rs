//! File type icons from the Flutter material-icon-theme mapping.
use std::collections::HashMap;
use std::sync::OnceLock;

use iced::widget::{image, svg, text};
use iced::Element;

use crate::{labels, theme};

mod file_icon_assets_0;
mod file_icon_assets_1;
mod file_icon_assets_2;

fn lookup(
    data: &'static str,
    cell: &'static OnceLock<HashMap<&'static str, &'static str>>,
    key: &str,
) -> Option<&'static str> {
    cell.get_or_init(|| {
        data.split('|')
            .filter_map(|entry| entry.split_once('\t'))
            .collect()
    })
    .get(key)
    .copied()
}

fn file_name(name: &str) -> Option<&'static str> {
    static NAMES: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    static EXTENSIONS: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    let name = name.to_lowercase();
    if let Some(icon) = lookup(
        include_str!("../assets/file_icons/kFileNameIcons.map"),
        &NAMES,
        &name,
    ) {
        return Some(icon);
    }
    if let Some((stem, ext)) = name.rsplit_once('.') {
        if !stem.is_empty() {
            match ext {
                "dbq" => return Some("database"),
                "kanban" => return Some("todo"),
                "http" | "rest" => return Some("http"),
                "notebook" => return Some("gsh-notebook"),
                "panel" => return Some("gsh-panel"),
                _ => {}
            }
        }
    }
    for (index, _) in name.match_indices('.') {
        if let Some(ext) = name.get(index + 1..) {
            if let Some(icon) = lookup(
                include_str!("../assets/file_icons/kFileExtensionIcons.map"),
                &EXTENSIONS,
                ext,
            ) {
                return Some(icon);
            }
        }
    }
    None
}

fn folder_name(name: &str, open: bool) -> &'static str {
    static CLOSED: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    static OPEN: OnceLock<HashMap<&str, &str>> = OnceLock::new();
    let name = name.to_lowercase();
    let name = if name.len() > 4 {
        name.strip_prefix("__")
            .and_then(|s| s.strip_suffix("__"))
            .unwrap_or(&name)
    } else {
        &name
    };
    let name = name.trim_start_matches(['.', '_', '-']);
    let (data, cell, default) = if open {
        (
            include_str!("../assets/file_icons/kFolderOpenIcons.map"),
            &OPEN,
            "folder-open",
        )
    } else {
        (
            include_str!("../assets/file_icons/kFolderIcons.map"),
            &CLOSED,
            "folder",
        )
    };
    lookup(data, cell, name).unwrap_or(default)
}

fn asset(name: &str) -> Option<&'static [u8]> {
    file_icon_assets_0::get(name)
        .or_else(|| file_icon_assets_1::get(name))
        .or_else(|| file_icon_assets_2::get(name))
}

fn resolve(name: &str, is_dir: bool, open: bool) -> Option<&'static [u8]> {
    if is_dir {
        asset(folder_name(name, open))
    } else {
        file_name(name).and_then(asset)
    }
}

pub fn view<Message: 'static>(name: &str, is_dir: bool, open: bool) -> Element<'static, Message> {
    if uses_gsh_logo(name, is_dir) {
        image(image::Handle::from_bytes(
            &include_bytes!("../assets/branding/gsh_logo.png")[..],
        ))
        .width(theme::FILE_TREE_ICON_SIZE)
        .height(theme::FILE_TREE_ICON_SIZE)
        .into()
    } else if let Some(bytes) = resolve(name, is_dir, open) {
        svg(svg::Handle::from_memory(bytes))
            .width(theme::FILE_TREE_ICON_SIZE)
            .height(theme::FILE_TREE_ICON_SIZE)
            .into()
    } else {
        text(if is_dir {
            labels::ICON_FOLDER
        } else {
            labels::ICON_FILE_ITEM
        })
        .font(theme::FONT_ICON)
        .size(theme::FILE_TREE_ICON_SIZE)
        .color(theme::palette().text3)
        .into()
    }
}

fn uses_gsh_logo(name: &str, is_dir: bool) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".ckp") || lower == ".env.gsh" || (is_dir && lower == ".gsh")
}

#[cfg(test)]
mod tests {
    use super::{folder_name, resolve, uses_gsh_logo};

    #[test]
    fn gsh_logo_matches_flutter_special_cases() {
        assert!(uses_gsh_logo("layout.CKP", false));
        assert!(uses_gsh_logo(".env.gsh", false));
        assert!(uses_gsh_logo(".gsh", true));
        assert!(!uses_gsh_logo(".gsh", false));
        assert!(!uses_gsh_logo("project.gsh", false));
    }

    #[test]
    fn known_file_has_embedded_svg_and_unknown_file_uses_fallback() {
        assert!(resolve("main.rs", false, false).is_some());
        assert!(resolve("unknown.unknown_gsh_type", false, false).is_none());
    }

    #[test]
    fn folders_resolve_closed_and_open_assets() {
        assert_eq!(folder_name("unknown_gsh_folder", false), "folder");
        assert_eq!(folder_name("unknown_gsh_folder", true), "folder-open");
        assert_ne!(
            resolve("unknown_gsh_folder", true, false),
            resolve("unknown_gsh_folder", true, true)
        );
    }
}
