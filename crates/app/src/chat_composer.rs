//! Componente visual do composer do chat (`chat_composer.dart` do Flutter).
//!
//! Exibe a faixa de contexto (pasta + branch), a caixa de texto com atalho
//! `Enter` para envio e botão de Enviar/Parar, e a barra inferior de status.

use iced::widget::{button, column, container, row, text, text_editor, tooltip, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

mod bar;
mod details;

#[cfg(test)]
use crate::chat_pane::ComposerMenu;
use crate::chat_pane::{ChatMsg, ChatPane};
use crate::labels;
use crate::theme;
use bar::view_bottom_bar;
use details::{view_peer_picker, view_usage_details};

/// Renderiza o composer completo na parte inferior do [`ChatPane`].
pub fn view(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let context_strip = view_context_strip(pane);
    let input_box = view_input_box(pane);
    let bottom_bar = view_bottom_bar(pane);

    let mut content = column![context_strip, Space::new().height(6.0)];
    if !pane.slash_matches().is_empty() {
        let mut commands = column![].spacing(2);
        for (index, command) in pane.slash_matches().into_iter().enumerate() {
            let label = if command.argument_hint.is_empty() {
                format!("/{}  {}", command.name, command.description)
            } else {
                format!(
                    "/{} {}  {}",
                    command.name, command.argument_hint, command.description
                )
            };
            let item = button(text(label).size(theme::SIZE_LABEL))
                .on_press(ChatMsg::SelectCommand(format!("/{} ", command.name)))
                .style(theme::ghost_button);
            commands = commands.push(if index == pane.slash_index() {
                item.style(|_: &Theme, _status| button::Style {
                    background: Some(Background::Color(theme::palette().panel3)),
                    text_color: theme::palette().text,
                    border: Border {
                        radius: theme::RADIUS_BUTTON.into(),
                        ..Default::default()
                    },
                    ..button::Style::default()
                })
            } else {
                item
            });
        }
        content = content.push(container(commands).width(Fill).padding(6));
    }

    content
        .push(input_box)
        .push(Space::new().height(4.0))
        .push(bottom_bar)
        .push(if pane.peer_picker_open() {
            view_peer_picker(pane)
        } else {
            Space::new().height(0.0).into()
        })
        .push(if pane.usage_open() {
            view_usage_details(pane)
        } else {
            Space::new().height(0.0).into()
        })
        .width(Fill)
        .into()
}

fn view_context_strip(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut strip_row = row![text(pane.folder_label())
        .font(theme::FONT_MONO)
        .size(theme::SIZE_TAB)
        .color(theme::palette().text3)]
    .align_y(Alignment::Center);

    if let Some(branch) = pane.branch().filter(|b| !b.is_empty()) {
        strip_row = strip_row.push(Space::new().width(10.0)).push(
            text(branch)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_TAB)
                .color(theme::palette().text3),
        );
    }

    container(strip_row)
        .width(Fill)
        .padding([9, 12])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn view_input_box(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let lines = pane.input_content().line_count().clamp(1, 8);
    let input = text_editor(pane.input_content())
        .placeholder(labels::CHAT_PLACEHOLDER)
        .on_action(ChatMsg::InputAction)
        .key_binding(move |press| chat_editor_key_binding(pane, press))
        .font(theme::FONT_BODY)
        .size(theme::SIZE_BODY)
        .line_height(iced::widget::text::LineHeight::Relative(
            theme::LINE_HEIGHT_DISPLAY,
        ))
        .padding([6, 8])
        .height(Length::Fixed(
            lines as f32 * theme::SIZE_BODY * theme::LINE_HEIGHT_DISPLAY + 12.0,
        ))
        .style(|_: &Theme, _status| text_editor::Style {
            background: Background::Color(iced::Color::TRANSPARENT),
            border: Border::default(),
            placeholder: theme::palette().text4,
            value: theme::palette().text,
            selection: theme::palette().accent_soft,
        });

    let action_btn = if pane.is_busy() {
        view_stop_button()
    } else {
        view_send_button(!pane.input().trim().is_empty())
    };

    let mut controls = row![input, Space::new().width(4.0), action_btn]
        .width(Fill)
        .align_y(Alignment::End);
    if !pane.attachments().is_empty() {
        let names =
            pane.attachments()
                .iter()
                .enumerate()
                .fold(row![], |row, (index, attachment)| {
                    row.push(
                        button(text(format!("{} ×", attachment.name)).size(theme::SIZE_TAB))
                            .on_press(ChatMsg::RemoveAttachment(index))
                            .style(theme::ghost_button),
                    )
                });
        controls = row![column![names, controls].spacing(3)];
    }
    container(controls)
        .width(Fill)
        .padding([4, 6])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                color: theme::palette().border2,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn chat_editor_key_binding(
    pane: &ChatPane,
    press: text_editor::KeyPress,
) -> Option<text_editor::Binding<ChatMsg>> {
    use iced::keyboard::{key::Named, Key};
    use text_editor::Binding;

    if (press.modifiers.control() || press.modifiers.command())
        && matches!(&press.key, Key::Character(value) if value.as_str().eq_ignore_ascii_case("v"))
    {
        return Some(Binding::Custom(ChatMsg::PasteImage));
    }
    let slash_open = !pane.slash_matches().is_empty();
    match &press.key {
        Key::Named(Named::Enter) if press.modifiers.shift() => Binding::from_key_press(press),
        Key::Named(Named::Enter) if slash_open => Some(Binding::Custom(ChatMsg::SlashKey(
            crate::chat_pane::SlashKey::Enter,
        ))),
        Key::Named(Named::Enter) => Some(Binding::Custom(ChatMsg::Send)),
        Key::Named(Named::Tab) if slash_open => Some(Binding::Custom(ChatMsg::SlashKey(
            crate::chat_pane::SlashKey::Tab,
        ))),
        Key::Named(Named::Escape) if slash_open => Some(Binding::Custom(ChatMsg::SlashKey(
            crate::chat_pane::SlashKey::Escape,
        ))),
        Key::Named(Named::ArrowUp) if slash_open => Some(Binding::Custom(ChatMsg::SlashKey(
            crate::chat_pane::SlashKey::Up,
        ))),
        Key::Named(Named::ArrowDown) if slash_open => Some(Binding::Custom(ChatMsg::SlashKey(
            crate::chat_pane::SlashKey::Down,
        ))),
        _ => Binding::from_key_press(press),
    }
}

fn view_send_button<'a>(enabled: bool) -> Element<'a, ChatMsg> {
    let mut send = button(
        text(labels::ICON_KEYBOARD_RETURN)
            .font(theme::FONT_ICON)
            .size(theme::CHAT_ICON_ACTION_SIZE)
            .color(theme::palette().text2),
    )
    .padding(theme::CHAT_ICON_ACTION_PADDING)
    .style(theme::ghost_button);
    if enabled {
        send = send.on_press(ChatMsg::Send);
    }
    tooltip(
        send,
        crate::pane_chrome::tip_bubble(labels::CHAT_SEND),
        tooltip::Position::Top,
    )
    .into()
}

fn view_stop_button<'a>() -> Element<'a, ChatMsg> {
    let stop = button(
        text(labels::ICON_STOP)
            .font(theme::FONT_ICON)
            .size(theme::CHAT_ICON_ACTION_SIZE)
            .color(theme::palette().text2),
    )
    .padding(theme::CHAT_ICON_ACTION_PADDING)
    .on_press(ChatMsg::Cancel)
    .style(theme::ghost_button);
    tooltip(
        stop,
        crate::pane_chrome::tip_bubble(labels::CHAT_STOP),
        tooltip::Position::Top,
    )
    .into()
}

#[cfg(test)]
#[test]
fn permission_menu_opens_and_applies_selection() {
    use iced::{Settings, Size};
    use iced_test::simulator::Simulator;

    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let mut pane = ChatPane::default();
    let mut ui = Simulator::with_size(settings.clone(), Size::new(800.0, 120.0), view(&pane));
    assert!(ui.click(labels::CHAT_MODE_DEFAULT).is_ok());
    for message in ui.into_messages() {
        let _ = pane.update(message);
    }
    assert_eq!(pane.composer_menu(), Some(ComposerMenu::Mode));
    let _ = pane.update(ChatMsg::SetMode("plan".into()));
    assert_eq!(pane.mode(), "plan");
    assert_eq!(pane.composer_menu(), None);
}

#[cfg(test)]
#[test]
#[ignore = "gera o snapshot do composer para revisão visual"]
fn capture_chat_composer_snapshot() {
    use iced::widget::{column, container};
    use iced::{Settings, Size};
    use iced_test::simulator::Simulator;
    use std::path::{Path, PathBuf};

    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    theme::set_active(theme::ThemeVariant::Dark);
    let pane = ChatPane::new(Some(PathBuf::from("style")), Some("main".into()));
    let content = container(column![
        Space::new().height(70.0),
        row![
            Space::new().width(75.0),
            container(view(&pane)).width(760.0),
            Space::new().width(Fill),
        ]
        .width(Fill),
    ])
    .width(Fill)
    .height(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        ..Default::default()
    });
    let settings = Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let mut simulator = Simulator::with_size(settings, Size::new(862.0, 198.0), content);
    let directory = Path::new("/tmp");
    let target = directory.join("rust-chat-composer-fixed-2x.png");
    let generated = directory.join("rust-chat-composer-fixed-2x-tiny-skia.png");
    let _ = std::fs::remove_file(&target);
    let _ = std::fs::remove_file(&generated);
    simulator
        .snapshot(&Theme::Dark)
        .expect("snapshot render")
        .matches_image(&target)
        .expect("snapshot write");
    std::fs::rename(generated, target).expect("rename snapshot");
}
