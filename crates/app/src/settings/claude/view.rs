//! Renderização do painel ativo do lado Claude e dos overlays (diálogos e toast).

use super::widgets::field_style;
use super::{catalog, fonts, panels, secondary, ClaudeState, Message, NamePromptKind, Tab};
use crate::settings::Settings;
use crate::{labels, theme};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{
    button, column, container, mouse_area, opaque, row, stack, text, text_input, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Theme};

/// Painel da aba selecionada no lado Claude.
pub fn panel(s: &Settings) -> Element<'_, Message> {
    match s.claude.tab {
        Tab::General => panels::general(s),
        Tab::Account => panels::account(s),
        Tab::Privacy => panels::privacy(s),
        Tab::Usage => secondary::usage(s),
        Tab::Resources => secondary::resources(s),
        Tab::Memory => secondary::memory(s),
        Tab::DesignSystems => secondary::design_systems(s),
        Tab::ClaudeCode => panels::claude_code(s),
        Tab::Cowork => secondary::cowork(),
        Tab::System => secondary::system(s),
        Tab::Extensions => secondary::extensions(),
        Tab::Developer => secondary::developer(s),
        Tab::Skills => catalog::skills(s),
        Tab::Connectors => catalog::connectors(s),
        Tab::Plugins => catalog::plugins(s),
    }
}

/// Empilha o toast e os diálogos sobre a tela de Configurações.
pub fn overlay<'a>(
    c: &'a ClaudeState,
    base: Element<'a, crate::settings::Message>,
) -> Element<'a, crate::settings::Message> {
    let mut layered = base;
    if let Some((id, message)) = &c.toast {
        let toast = toast_card(*id, message).map(crate::settings::Message::Claude);
        layered = stack![layered, toast].into();
    }
    if let Some(picker) = &c.font_picker {
        let modal = fonts::dialog(picker).map(crate::settings::Message::Claude);
        layered = stack![layered, modal].into();
    }
    if let Some((kind, value)) = &c.name_prompt {
        let modal = name_prompt_dialog(*kind, value).map(crate::settings::Message::Claude);
        layered = stack![layered, modal].into();
    }
    if c.storage_reset_ask {
        let modal = confirm_storage_reset_dialog().map(crate::settings::Message::Claude);
        layered = stack![layered, modal].into();
    }
    if let Some((msg, quit_only)) = &c.storage_restart_msg {
        let modal = storage_restart_dialog(msg, *quit_only).map(crate::settings::Message::Claude);
        layered = stack![layered, modal].into();
    }
    if c.logout_ask {
        let modal = confirm_logout_dialog().map(crate::settings::Message::Claude);
        layered = stack![layered, modal].into();
    }
    if c.notice {
        let modal = no_workspace_dialog().map(crate::settings::Message::Claude);
        layered = stack![layered, modal].into();
    }
    layered
}

fn toast_card(id: u64, message: &str) -> Element<'_, Message> {
    let card = button(
        container(
            text(message)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text),
        )
        .max_width(theme::SETTINGS_TOAST_MAX_WIDTH),
    )
    .padding(theme::SETTINGS_TOAST_PADDING)
    .on_press(Message::ToastClose(id))
    .style(|_: &Theme, _| {
        let p = theme::palette();
        button::Style {
            background: Some(Background::Color(p.panel2)),
            text_color: p.text,
            border: Border {
                color: p.border2,
                width: 1.0,
                radius: theme::SETTINGS_TOAST_RADIUS.into(),
            },
            ..Default::default()
        }
    });
    container(card)
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Right)
        .align_y(Vertical::Bottom)
        .padding(theme::SETTINGS_TOAST_MARGIN)
        .into()
}

fn name_prompt_dialog(kind: NamePromptKind, value: &str) -> Element<'_, Message> {
    let (title, placeholder, confirm) = match kind {
        NamePromptKind::Skill => (
            labels::SETTINGS_CAT_NEW_SKILL,
            "minha-skill",
            labels::HARNESS_CREATE,
        ),
        NamePromptKind::Marketplace => (
            labels::SETTINGS_CAT_ADD_MARKETPLACE,
            "owner/repo",
            labels::SETTINGS_COMMON_ADD,
        ),
    };
    let input = text_input(placeholder, value)
        .on_input(Message::NamePromptInput)
        .on_submit(Message::NamePromptSubmit)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_DROPDOWN_TEXT)
        .padding(theme::SETTINGS_TEXT_FIELD_PADDING)
        .width(Length::Fixed(theme::HARNESS_NAME_DIALOG_WIDTH))
        .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS));

    let actions = row![
        dialog_button(labels::DIALOG_CANCEL, Message::NamePromptCancel, None),
        dialog_button(confirm, Message::NamePromptSubmit, Some(false)),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center);

    modal_custom(
        title,
        input.into(),
        actions.into(),
        Message::NamePromptCancel,
    )
}

fn confirm_storage_reset_dialog() -> Element<'static, Message> {
    let actions = row![
        dialog_button(labels::DIALOG_CANCEL, Message::StorageResetCancel, None),
        dialog_button(
            labels::SETTINGS_STORAGE_RESET_CONFIRM,
            Message::StorageResetConfirm,
            Some(true),
        ),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center);

    modal_shell(
        labels::SETTINGS_STORAGE_RESET_DIALOG_TITLE,
        labels::SETTINGS_STORAGE_RESET_DIALOG_CONTENT.to_owned(),
        actions.into(),
        Message::StorageResetCancel,
    )
}

fn storage_restart_dialog(message: &str, quit_only: bool) -> Element<'static, Message> {
    let mut actions = row![]
        .spacing(theme::DIALOG_ACTIONS_GAP)
        .align_y(Alignment::Center);
    if !quit_only {
        actions = actions.push(dialog_button(
            labels::SETTINGS_STORAGE_LATER,
            Message::StorageRestartDismiss,
            None,
        ));
    }
    actions = actions.push(dialog_button(
        labels::SETTINGS_STORAGE_QUIT,
        Message::StorageRestartQuit,
        Some(false),
    ));

    modal_shell(
        labels::SETTINGS_STORAGE_RESTART_TITLE,
        message.to_owned(),
        actions.into(),
        if quit_only {
            Message::StorageRestartQuit
        } else {
            Message::StorageRestartDismiss
        },
    )
}

fn confirm_logout_dialog() -> Element<'static, Message> {
    let actions = row![
        dialog_button(labels::DIALOG_CANCEL, Message::LogoutCancel, None),
        dialog_button(
            labels::SETTINGS_ACCOUNT_LOGOUT,
            Message::LogoutConfirm,
            Some(true),
        ),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center);

    modal_shell(
        labels::SETTINGS_ACCOUNT_LOGOUT_CONFIRM_TITLE,
        labels::SETTINGS_ACCOUNT_LOGOUT_CONFIRM_MESSAGE.to_owned(),
        actions.into(),
        Message::LogoutCancel,
    )
}

fn no_workspace_dialog() -> Element<'static, Message> {
    let actions = row![dialog_button(
        labels::DIALOG_OK,
        Message::NoticeClose,
        Some(false),
    )]
    .align_y(Alignment::Center);

    modal_shell(
        labels::SETTINGS_CLAUDE_TERMINAL_TITLE,
        labels::SETTINGS_CLAUDE_NO_WORKSPACE.to_owned(),
        actions.into(),
        Message::NoticeClose,
    )
}

fn modal_shell(
    title: &'static str,
    body_text: String,
    actions: Element<'static, Message>,
    on_dismiss: Message,
) -> Element<'static, Message> {
    let body = container(
        text(body_text)
            .font(theme::FONT_BODY)
            .size(theme::DIALOG_MESSAGE_SIZE)
            .color(theme::palette().text2),
    )
    .max_width(theme::DIALOG_CONTENT_WIDTH);
    modal_custom(title, body.into(), actions, on_dismiss)
}

fn modal_custom<'a>(
    title: &'static str,
    body: Element<'a, Message>,
    actions: Element<'a, Message>,
    on_dismiss: Message,
) -> Element<'a, Message> {
    let heading = text(title)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text);

    let card = container(
        column![
            column![heading, body].spacing(theme::DIALOG_TITLE_GAP),
            actions
        ]
        .spacing(theme::DIALOG_GAP)
        .align_x(Alignment::End),
    )
    .padding(theme::DIALOG_PADDING)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        border: Border {
            color: theme::palette().panel2,
            width: 1.0,
            radius: theme::DIALOG_RADIUS.into(),
        },
        ..Default::default()
    });

    let scrim = mouse_area(
        container(Space::new().width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::palette().scrim)),
                ..Default::default()
            }),
    )
    .on_press(on_dismiss);

    stack![
        scrim,
        container(opaque(card))
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
    ]
    .into()
}

fn dialog_button(
    label: &'static str,
    on_press: Message,
    danger: Option<bool>,
) -> Element<'static, Message> {
    let fg = match danger {
        Some(true) => theme::on_color(theme::palette().error),
        Some(false) => theme::ON_ACCENT,
        None => theme::palette().text,
    };
    button(
        text(label)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::DIALOG_BUTTON_SIZE)
            .color(fg),
    )
    .padding([8, 16])
    .on_press(on_press)
    .style(move |_: &Theme, status| {
        let p = theme::palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let fill = |color: Color| {
            let color = if hovered {
                Color { a: 0.9, ..color }
            } else {
                color
            };
            (Some(Background::Color(color)), color)
        };
        let (background, border_color) = match danger {
            Some(true) => fill(p.error),
            Some(false) => fill(p.accent),
            None => (hovered.then_some(Background::Color(p.panel3)), p.border),
        };
        button::Style {
            background,
            text_color: fg,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: theme::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }
    })
    .into()
}
