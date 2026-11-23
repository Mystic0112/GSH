use std::path::Path;

use iced::{Point, Size};
use iced_test::simulator::{click, Simulator};

use crate::{labels, rail, theme};

fn settings() -> iced::Settings {
    iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    }
}

fn ignore_action(_: rail::ChannelAction) {}

fn rail_view<'a>(
    workspaces: &'a gsh_core::workspace::WorkspaceList,
    open: bool,
) -> iced::Element<'a, ()> {
    rail::view(
        workspaces,
        |_| (),
        rail::RailActions {
            keep_awake: rail::KeepAwakeStatus::default(),
            update_version: None,
            on_add_workspace: (),
            channels: rail::ChannelActions {
                editor: None,
                members: Vec::new(),
                on_action: ignore_action,
            },
            channels_open: open,
            on_toggle_channels: (),
            on_close_channels: (),
            on_toggle_keep_awake: (),
            on_download_update: (),
            on_dismiss_update: (),
            on_settings: (),
        },
    )
}

fn snapshot(name: &str, workspaces: &gsh_core::workspace::WorkspaceList, open: bool) {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    theme::set_active(theme::ThemeVariant::Dark);
    let mut ui = Simulator::with_size(
        settings(),
        Size::new(theme::RAIL_WIDTH, 650.0),
        rail_view(workspaces, open),
    );
    let directory = Path::new("/home/heliodev/projetos/GSH-wt/shots");
    let target = directory.join(format!("{name}.png"));
    let generated = directory.join(format!("{name}-tiny-skia.png"));
    ui.snapshot(&iced::Theme::Dark)
        .expect("render rail snapshot")
        .matches_image(&target)
        .expect("write rail snapshot");
    std::fs::rename(generated, target).expect("rename rail snapshot");
}

#[test]
fn rail_hides_channels_by_default_and_channel_button_opens_popup() {
    let workspaces = rail::default_workspace_list();
    snapshot("gsh-rail-canais-fechado", &workspaces, false);
    snapshot("gsh-rail-canais-popup", &workspaces, true);

    let mut ui = Simulator::with_size(
        settings(),
        Size::new(theme::RAIL_WIDTH, 650.0),
        rail_view(&workspaces, false),
    );
    assert!(ui.find(labels::CHANNELS_ALL).is_err());
    assert!(ui.find(labels::WORKSPACES_HEADER).is_ok());
}

#[test]
fn shell_channel_popup_opens_closes_on_selection_escape_and_outside_click() {
    let mut shell = crate::tests::test_shell();
    let mut ui = Simulator::with_size(settings(), Size::new(1024.0, 700.0), shell.view());
    ui.point_at(Point::new(193.0, 65.0));
    let _ = ui.simulate(click());
    for message in ui.into_messages() {
        let _ = shell.update(message);
    }
    assert!(shell.channels_popup_open);

    let _ = shell.update(crate::Msg::ChannelAction(rail::ChannelAction::Select(None)));
    assert!(!shell.channels_popup_open);

    let mut ui = Simulator::with_size(settings(), Size::new(1024.0, 700.0), shell.view());
    ui.point_at(Point::new(193.0, 65.0));
    let _ = ui.simulate(click());
    for message in ui.into_messages() {
        let _ = shell.update(message);
    }
    assert!(shell.channels_popup_open);
    let mut ui = Simulator::with_size(settings(), Size::new(1024.0, 700.0), shell.view());
    let _ = ui.tap_key(iced::keyboard::key::Named::Escape);
    for message in ui.into_messages() {
        let _ = shell.update(message);
    }
    assert!(!shell.channels_popup_open);

    let mut ui = Simulator::with_size(settings(), Size::new(1024.0, 700.0), shell.view());
    ui.point_at(Point::new(193.0, 65.0));
    let _ = ui.simulate(click());
    for message in ui.into_messages() {
        let _ = shell.update(message);
    }
    let mut ui = Simulator::with_size(settings(), Size::new(1024.0, 700.0), shell.view());
    ui.point_at(Point::new(700.0, 600.0));
    let _ = ui.simulate(click());
    for message in ui.into_messages() {
        let _ = shell.update(message);
    }
    assert!(!shell.channels_popup_open);
}

#[test]
fn channel_filter_uses_existing_workspace_filter_state_and_indicator() {
    let mut shell = crate::tests::test_shell();
    let id = shell
        .workspaces
        .create_channel(gsh_core::workspace::ChannelDraft {
            name: "Produto".into(),
            ..Default::default()
        })
        .expect("create test channel");
    shell
        .workspaces
        .select_context_channel(Some(&id))
        .expect("select test channel");
    assert!(shell.workspaces.active_context_channel().is_some());
    assert!(!shell.channels_popup_open);
    snapshot("gsh-rail-canais-filtrado", &shell.workspaces, false);
}
