use super::super::{labels, rail, theme, Msg};
use super::test_shell;
use iced::keyboard;
use iced::{Event, Point, Size};
use iced_test::simulator::Simulator;

fn simulator_settings() -> iced::Settings {
    iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    }
}

#[test]
fn channel_popup_toggle_escape_and_outside_click_close() {
    let mut shell = test_shell();
    let _ = shell.update_inner(Msg::ToggleChannelPopup);
    assert!(shell.channel_popup_open);
    let _ = shell.update_inner(Msg::ToggleChannelPopup);
    assert!(!shell.channel_popup_open);

    let _ = shell.update_inner(Msg::ToggleChannelPopup);
    let mut escape_ui =
        Simulator::with_size(simulator_settings(), Size::new(1280.0, 720.0), shell.view());
    let escape = keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(keyboard::key::Named::Escape),
        modified_key: keyboard::Key::Named(keyboard::key::Named::Escape),
        physical_key: keyboard::key::Physical::Code(keyboard::key::Code::Escape),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::NONE,
        text: None,
        repeat: false,
    };
    let _ = escape_ui.simulate([Event::Keyboard(escape)]);
    for message in escape_ui.into_messages() {
        let _ = shell.update_inner(message);
    }
    assert!(!shell.channel_popup_open);

    let _ = shell.update_inner(Msg::ToggleChannelPopup);
    let mut outside_ui =
        Simulator::with_size(simulator_settings(), Size::new(1280.0, 720.0), shell.view());
    outside_ui.point_at(Point::new(800.0, 300.0));
    let _ = outside_ui.simulate([Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);
    for message in outside_ui.into_messages() {
        let _ = shell.update_inner(message);
    }
    assert!(!shell.channel_popup_open);
}

#[test]
fn channel_popup_simulator_and_messages_cover_selection_new_and_cancel() {
    let mut shell = test_shell();
    let channel = shell
        .workspaces
        .context_channels()
        .first()
        .expect("test workspace has a channel");
    let channel_id = channel.id.clone();

    let mut trigger =
        Simulator::with_size(simulator_settings(), Size::new(1280.0, 720.0), shell.view());
    assert!(trigger.click(labels::CHANNELS_ALL).is_ok());
    for message in trigger.into_messages() {
        let _ = shell.update_inner(message);
    }
    assert!(shell.channel_popup_open);

    let _ = shell.update_inner(Msg::ChannelAction(rail::ChannelAction::Select(Some(
        channel_id.clone(),
    ))));
    assert!(!shell.channel_popup_open);
    assert_eq!(
        shell
            .workspaces
            .active_context_channel()
            .map(|active| &active.id),
        Some(&channel_id)
    );

    let _ = shell.update_inner(Msg::ToggleChannelPopup);
    let _ = shell.update_inner(Msg::ChannelAction(rail::ChannelAction::New));
    assert!(shell.channel_popup_open);
    assert!(shell.channel_editor.is_some());

    let _ = shell.update_inner(Msg::ChannelAction(rail::ChannelAction::Close));
    assert!(!shell.channel_popup_open);
    assert!(shell.channel_editor.is_none());
}

#[test]
fn moving_active_workspace_to_channel_applies_and_closes_popup() {
    let mut shell = test_shell();
    let current = shell
        .workspaces
        .active()
        .and_then(|workspace| workspace.channel_id())
        .cloned();
    let target = shell
        .workspaces
        .context_channels()
        .iter()
        .find(|channel| Some(&channel.id) != current.as_ref())
        .map(|channel| channel.id.clone())
        .expect("test workspace has another channel");

    let _ = shell.update_inner(Msg::ToggleChannelPopup);
    let _ = shell.update_inner(Msg::ChannelAction(rail::ChannelAction::AssignWorkspace(
        target.clone(),
    )));
    assert!(!shell.channel_popup_open);
    assert_eq!(
        shell
            .workspaces
            .active()
            .and_then(|workspace| workspace.channel_id()),
        Some(&target)
    );
}
