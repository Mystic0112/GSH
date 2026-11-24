#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use iced::widget::{container, row, Space};
    use iced::{Alignment, Element, Fill, Settings, Size};
    use iced_test::simulator::Simulator;

    use crate::{
        farm::FarmCounts, labels, pane_close_dialog, rail, theme, topbar,
        workspace_settings_dialog as dialog,
    };
    use gsh_core::workspace::{Workspace, WorkspaceList};

    fn capture<'a, M: Clone + 'a>(name: &str, view: Element<'a, M>) {
        std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
        let settings = Settings {
            fonts: theme::EMBEDDED_FONTS
                .iter()
                .map(|font| (*font).into())
                .collect(),
            default_font: theme::FONT_BODY,
            ..Default::default()
        };
        let mut simulator = Simulator::with_size(settings, Size::new(1024.0, 720.0), view);
        let target = Path::new("/home/heliodev/projetos/GSH-wt/shots").join(name);
        let generated = target.with_file_name(format!(
            "{}-tiny-skia.png",
            target
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("snapshot")
        ));
        std::fs::create_dir_all(target.parent().unwrap_or_else(|| Path::new("/tmp")))
            .expect("snapshot directory");
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&generated);
        simulator
            .snapshot(&iced::Theme::Dark)
            .expect("snapshot render")
            .matches_image(&target)
            .expect("snapshot write");
        std::fs::rename(generated, target).expect("snapshot rename");
    }

    fn no_op(_: usize, _: rail::WorkspaceMenuAction) {}

    #[test]
    fn capture_workspace_settings_menu_close_and_avatar_snapshots() {
        theme::set_active(theme::ThemeVariant::Dark);
        let draft = dialog::Draft {
            id: Some("ws-1".into()),
            name: "GSH Desktop".into(),
            path: PathBuf::from("/home/helio/projetos/gsh"),
            color: 0xFF6E56CF,
            image: None,
        };
        let edit = pane_close_dialog::modal(dialog::view(&draft).map(|_| ()), (), ());
        capture("gsh-ws-dialog.png", edit);
        let mut create = draft.clone();
        create.id = None;
        create.name = "meu-projeto".into();
        create.path = PathBuf::from("/home/helio/projetos/meu-projeto");
        create.color = dialog::PALETTE.get(2).copied().unwrap_or_default();
        let new = pane_close_dialog::modal(dialog::view(&create).map(|_| ()), (), ());
        capture("gsh-ws-dialog-novo.png", new);

        let close = pane_close_dialog::confirm(
            labels::WORKSPACE_CLOSE_TITLE.to_owned(),
            labels::WORKSPACE_CLOSE_MESSAGE.to_owned(),
            labels::WORKSPACE_CLOSE_CONFIRM,
            (),
            (),
            (),
        );
        capture("gsh-ws-fechar.png", close);

        let avatar = PathBuf::from(
            "/home/heliodev/projetos/GSH-wt/ui3/crates/app/assets/branding/gsh_logo.png",
        );
        let workspaces = WorkspaceList::new(vec![
            Workspace::from_path("ws-1", "/workspace/azul").with_color(Some(0xFF2F6FF0)),
            Workspace::from_path("ws-2", "/workspace/verde")
                .with_color(Some(0xFF3FB868))
                .with_image_path(Some(avatar)),
            Workspace::from_path("ws-3", "/workspace/rosa").with_color(Some(0xFFD6409F)),
        ]);
        let editor = None;
        let menu = rail::view(
            &workspaces,
            |_| (),
            rail::RailActions {
                keep_awake: rail::KeepAwakeStatus::default(),
                update_version: None,
                new_workspace_menu_open: false,
                on_new_workspace_menu: |_| (),
                channels: rail::ChannelActions {
                    editor: editor.as_ref(),
                    members: Vec::new(),
                    on_action: |_| (),
                },
                channel_popup_open: false,
                on_toggle_channel_popup: (),
                on_close_channel_popup: (),
                on_toggle_keep_awake: (),
                on_download_update: (),
                on_dismiss_update: (),
                on_settings: (),
                workspace_menu_open: Some(1),
                on_workspace_menu: no_op,
            },
        );
        capture("gsh-ws-menu.png", menu);
        let rail = rail::view(
            &workspaces,
            |_| (),
            rail::RailActions {
                keep_awake: rail::KeepAwakeStatus::default(),
                update_version: None,
                new_workspace_menu_open: false,
                on_new_workspace_menu: |_| (),
                channels: rail::ChannelActions {
                    editor: editor.as_ref(),
                    members: Vec::new(),
                    on_action: |_| (),
                },
                channel_popup_open: false,
                on_toggle_channel_popup: (),
                on_close_channel_popup: (),
                on_toggle_keep_awake: (),
                on_download_update: (),
                on_dismiss_update: (),
                on_settings: (),
                workspace_menu_open: None,
                on_workspace_menu: no_op,
            },
        );
        capture("gsh-ws-rail-cores.png", rail);
        let plus = rail::view(
            &workspaces,
            |_| (),
            rail::RailActions {
                keep_awake: rail::KeepAwakeStatus::default(),
                update_version: None,
                new_workspace_menu_open: true,
                on_new_workspace_menu: |_| (),
                channels: rail::ChannelActions {
                    editor: editor.as_ref(),
                    members: Vec::new(),
                    on_action: |_| (),
                },
                channel_popup_open: false,
                on_toggle_channel_popup: (),
                on_close_channel_popup: (),
                on_toggle_keep_awake: (),
                on_download_update: (),
                on_dismiss_update: (),
                on_settings: (),
                workspace_menu_open: None,
                on_workspace_menu: no_op,
            },
        );
        capture("gsh-plus-menu.png", plus);
    }

    #[test]
    fn capture_files_menu_and_viewer_snapshots() {
        theme::set_active(theme::ThemeVariant::Dark);
        let menu = crate::file_tree_context_menu::context_menu(
            Path::new("/workspace/src"),
            true,
            false,
            true,
            true,
        );
        capture("gsh-files-menu.png", menu);

        let source = "---\nname: agente-demo\ndescription: Revisa markdown.\ntags:\n  - rust\n  - ui\n---\n# Arquivo\n\nUma prévia Markdown com [link](https://example.com).";
        let frontmatter = crate::file_viewer_assets::split_frontmatter(source).unwrap();
        let markdown = iced::widget::markdown::Content::parse(&frontmatter.body);
        let viewer = crate::file_viewer_view::source_preview(
            Path::new("/workspace/agent.md"),
            Some(Path::new("/workspace")),
            source,
            Some(&markdown),
            true,
        );
        capture("gsh-files-viewer.png", viewer);
    }

    #[test]
    fn capture_empty_rail_and_topbar_snapshots() {
        theme::set_active(theme::ThemeVariant::Dark);

        let empty_workspaces = rail::default_workspace_list();
        let empty_rail = rail::view(
            &empty_workspaces,
            |_| (),
            rail::RailActions {
                keep_awake: rail::KeepAwakeStatus::default(),
                update_version: None,
                new_workspace_menu_open: false,
                on_new_workspace_menu: |_| (),
                channels: rail::ChannelActions {
                    editor: None,
                    members: Vec::new(),
                    on_action: |_| (),
                },
                channel_popup_open: false,
                on_toggle_channel_popup: (),
                on_close_channel_popup: (),
                on_toggle_keep_awake: (),
                on_download_update: (),
                on_dismiss_update: (),
                on_settings: (),
                workspace_menu_open: None,
                on_workspace_menu: no_op,
            },
        );
        capture("gsh-rail-vazio.png", empty_rail);

        let bar = topbar::view(
            topbar::TopbarProps {
                workspace_title: labels::WORKSPACE_GSH,
                rail_open: true,
                files_open: true,
                counts: FarmCounts {
                    working: 1,
                    waiting: 1,
                    idle: 2,
                },
            },
            || Space::new().into(),
            topbar::TopbarActions {
                on_toggle_rail: (),
                on_toggle_files: (),
                on_chat: Some(()),
                on_farm: Some(()),
                on_browser: None,
                on_harness: Some(()),
                on_agent_graph: Some(()),
                on_localsend: Some(()),
                app_menus: Vec::new(),
                app_menu_open: false,
                on_toggle_app_menu: (),
                on_close_app_menu: (),
            },
        );
        let full_topbar: Element<'_, ()> = row![
            container(bar).width(Fill),
            topbar::window_controls((), (), ()),
        ]
        .align_y(Alignment::Center)
        .into();
        capture("gsh-topbar.png", full_topbar);
    }
}
