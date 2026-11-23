//! Janela LocalSend do GSH: recebimento, envio e configurações funcionais.

mod settings;

use crate::settings::LocalSendState;
use crate::{labels, theme};
use iced::widget::{
    button, column, container, progress_bar, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Border, Element, Fill, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopupTab {
    Receive,
    Send,
    SettingsReceive,
    SettingsNetwork,
}

/// Ações da janela; o shell converte cada gesto em mensagem de domínio.
pub struct PopupActions<M> {
    pub close: M,
    pub receive: M,
    pub send: M,
    pub settings_receive: M,
    pub settings_network: M,
    pub pick_files: M,
    pub pick_folder: M,
    pub text_changed: fn(String) -> M,
    pub send_text: M,
    pub paste: Option<M>,
    pub refresh: M,
    pub send_to: fn(String) -> M,
    pub choose_receive_dir: M,
    pub alias_changed: fn(String) -> M,
    pub always_on_changed: fn(bool) -> M,
    pub limits_changed: fn(String, String) -> M,
    pub server_restart: M,
    pub server_stop: M,
    pub cancel_send: M,
}

/// Conteúdo da janela modal; o shell controla scrim, centralização e fechamento exterior.
pub fn view<'a, M: Clone + 'static + 'a>(
    state: &'a LocalSendState,
    selected: PopupTab,
    draft_text: &'a str,
    actions: PopupActions<M>,
) -> Element<'a, M> {
    let sidebar = column![
        text(labels::LOCALSEND_TITLE)
            .font(theme::FONT_TITLE)
            .size(27)
            .color(theme::palette().text),
        Space::new().height(18),
        nav(
            labels::LOCALSEND_POPUP_RECEIVE,
            selected == PopupTab::Receive,
            actions.receive.clone()
        ),
        nav(
            labels::LOCALSEND_POPUP_SEND,
            selected == PopupTab::Send,
            actions.send.clone()
        ),
        nav(
            labels::LOCALSEND_POPUP_SETTINGS,
            matches!(
                selected,
                PopupTab::SettingsReceive | PopupTab::SettingsNetwork
            ),
            actions.settings_receive.clone()
        ),
    ]
    .spacing(9)
    .padding([28, 16])
    .width(232)
    .height(Fill);

    let page = match selected {
        PopupTab::Receive => receive(state),
        PopupTab::Send => send(state, draft_text, &actions),
        PopupTab::SettingsReceive => settings::receive(state, &actions),
        PopupTab::SettingsNetwork => settings::network(state, &actions),
    };
    let content = column![
        row![
            text(tab_title(selected))
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text)
                .width(Fill),
            button(labels::LOCALSEND_CLOSE)
                .on_press(actions.close)
                .style(button_style),
        ]
        .align_y(Alignment::Center),
        page,
    ]
    .spacing(20)
    .padding([18, 28])
    .width(Fill)
    .height(Fill);

    container(row![sidebar, container(content).width(Fill).height(Fill)])
        .width(900)
        .height(620)
        .style(|_| container::Style {
            background: Some(theme::palette().bg.into()),
            border: Border {
                color: theme::palette().border2,
                width: 1.0,
                radius: theme::DIALOG_RADIUS.into(),
            },
            ..Default::default()
        })
        .into()
}

fn tab_title(tab: PopupTab) -> &'static str {
    match tab {
        PopupTab::Receive => labels::LOCALSEND_POPUP_RECEIVE,
        PopupTab::Send => labels::LOCALSEND_POPUP_SEND,
        PopupTab::SettingsReceive | PopupTab::SettingsNetwork => labels::LOCALSEND_POPUP_SETTINGS,
    }
}

fn receive<'a, M: 'a>(state: &'a LocalSendState) -> Element<'a, M> {
    let configured_alias = if state.alias.trim().is_empty() {
        labels::LOCALSEND_EMPTY_ALIAS
    } else {
        state.alias.trim()
    };
    let alias = state.server_alias.as_deref().unwrap_or(configured_alias);
    let status = state
        .server_status
        .as_deref()
        .unwrap_or(labels::LOCALSEND_STOPPED);
    let mut content = column![
        text(labels::ICON_HUB_OUTLINED)
            .font(theme::FONT_ICON)
            .size(112)
            .color(theme::palette().accent),
        text(alias.to_owned())
            .font(theme::FONT_TITLE)
            .size(38)
            .color(theme::palette().text),
        text(status.to_owned())
            .size(theme::SIZE_LABEL)
            .color(if state.server_status.is_some() {
                theme::palette().online
            } else {
                theme::palette().text3
            }),
    ]
    .spacing(14)
    .align_x(Alignment::Center);
    if let Some(error) = &state.error {
        content = content.push(
            text(error.clone())
                .size(theme::SIZE_LABEL)
                .color(theme::palette().warn),
        );
    }
    container(content)
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}

fn send<'a, M: Clone + 'static + 'a>(
    state: &'a LocalSendState,
    draft: &'a str,
    actions: &PopupActions<M>,
) -> Element<'a, M> {
    let mut choices = row![
        choice(
            labels::ICON_DESCRIPTION,
            labels::LOCALSEND_FILE,
            actions.pick_files.clone()
        ),
        choice(
            labels::ICON_FOLDER,
            labels::LOCALSEND_FOLDER,
            actions.pick_folder.clone()
        ),
        choice(
            labels::ICON_FILE_DOC,
            labels::LOCALSEND_TEXT,
            actions.send_text.clone()
        ),
    ]
    .spacing(10);
    if let Some(paste) = &actions.paste {
        choices = choices.push(choice(
            labels::ICON_CONTENT_COPY,
            labels::LOCALSEND_PASTE,
            paste.clone(),
        ));
    }

    let mut devices = column![row![
        text(labels::LOCALSEND_NEARBY)
            .font(theme::FONT_TITLE)
            .size(theme::SETTINGS_NAV_TEXT_SIZE)
            .color(theme::palette().text)
            .width(Fill),
        button(labels::ICON_REFRESH)
            .on_press(actions.refresh.clone())
            .style(button_style),
    ]
    .align_y(Alignment::Center),]
    .spacing(10);
    if state.peers.is_empty() {
        devices = devices.push(
            text(labels::LOCALSEND_NO_DEVICES)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        );
    }
    for peer in &state.peers {
        devices = devices.push(
            button(
                row![
                    text(peer.alias.clone())
                        .font(theme::FONT_BODY_MEDIUM)
                        .size(theme::SETTINGS_NAV_TEXT_SIZE)
                        .color(theme::palette().text),
                    text(format!("{} · {}", peer.device_type, peer.ip))
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().text3)
                        .width(Fill),
                    text(labels::ICON_SEND)
                        .font(theme::FONT_ICON)
                        .size(20)
                        .color(theme::palette().accent),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .padding([14, 16]),
            )
            .width(Fill)
            .on_press((actions.send_to)(peer.ip.clone()))
            .style(button_style),
        );
    }

    let mut body = column![
        text(labels::LOCALSEND_SELECTION)
            .font(theme::FONT_TITLE)
            .size(theme::SETTINGS_NAV_TEXT_SIZE)
            .color(theme::palette().text),
        choices,
        text_input(labels::LOCALSEND_TEXT_HINT, draft)
            .on_input(actions.text_changed)
            .style(crate::panel_widgets::input_style)
            .padding([10, 12]),
        devices,
    ]
    .spacing(14);
    if !state.selected_files.is_empty() {
        let mut selected = column![text(labels::LOCALSEND_SELECTED_FILES)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text)];
        for path in &state.selected_files {
            let name = path
                .file_name()
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            selected = selected.push(
                text(name)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text2),
            );
        }
        body = body.push(selected.spacing(4));
    }
    if let Some(status) = &state.transfer_status {
        body = body.push(
            text(status.clone())
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text2),
        );
    }
    if let Some(transfer) = &state.transfer {
        body = body.push(
            column![
                text(format!("{} · {}", transfer.peer_alias, transfer.file_name))
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text),
                progress_bar(
                    0.0..=transfer.total_bytes.max(1) as f32,
                    transfer.bytes.min(transfer.total_bytes.max(1)) as f32
                ),
                button(labels::LOCALSEND_CANCEL)
                    .on_press(actions.cancel_send.clone())
                    .style(button_style),
            ]
            .spacing(8),
        );
    }
    body = body.push(
        text(labels::LOCALSEND_WIFI_HELP)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3),
    );
    if let Some(error) = &state.error {
        body = body.push(
            text(error.clone())
                .size(theme::SIZE_LABEL)
                .color(theme::palette().warn),
        );
    }
    scrollable(body.spacing(16)).height(Fill).into()
}

pub(super) fn nav<M: Clone + 'static>(
    label: &'static str,
    active: bool,
    message: M,
) -> Element<'static, M> {
    button(
        text(label)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::SETTINGS_NAV_TEXT_SIZE),
    )
    .width(Fill)
    .padding([10, 13])
    .on_press(message)
    .style(move |_, status| button::Style {
        background: Some(
            if active {
                theme::palette().panel3
            } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                theme::palette().panel2
            } else {
                theme::palette().panel
            }
            .into(),
        ),
        text_color: theme::palette().text,
        border: Border {
            color: theme::palette().border2,
            width: 1.0,
            radius: theme::RADIUS_CARD.into(),
        },
        ..Default::default()
    })
    .into()
}

fn choice<M: Clone + 'static>(
    icon: &'static str,
    label: &'static str,
    message: M,
) -> Element<'static, M> {
    button(
        column![
            text(icon)
                .font(theme::FONT_ICON)
                .size(23)
                .color(theme::palette().accent),
            text(label)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text),
        ]
        .spacing(7)
        .align_x(Alignment::Center)
        .padding([12, 16]),
    )
    .on_press(message)
    .width(106)
    .style(button_style)
    .into()
}

pub(super) fn button_style(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: Some(
            if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                theme::palette().panel3
            } else {
                theme::palette().panel2
            }
            .into(),
        ),
        text_color: theme::palette().text,
        border: Border {
            color: theme::palette().border2,
            width: 1.0,
            radius: theme::RADIUS_CARD.into(),
        },
        ..Default::default()
    }
}
