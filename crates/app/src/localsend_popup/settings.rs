//! Telas de configuração do popup LocalSend.

use super::{button_style, nav, PopupActions, PopupTab};
use crate::settings::LocalSendState;
use crate::{labels, theme};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Border, Element, Fill};

pub(super) fn receive<'a, M: Clone + 'static + 'a>(
    state: &'a LocalSendState,
    actions: &PopupActions<M>,
) -> Element<'a, M> {
    let limits = row![
        text_input(labels::LOCALSEND_MAX_FILE_MB_HINT, &state.max_file_mb)
            .on_input({
                let update = actions.limits_changed;
                move |value| update(value, String::new())
            })
            .style(crate::panel_widgets::input_style)
            .width(105)
            .padding([8, 10]),
        text(labels::LOCALSEND_MB)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text2),
        text_input(labels::LOCALSEND_MAX_FILES_HINT, &state.max_files)
            .on_input({
                let update = actions.limits_changed;
                move |value| update(String::new(), value)
            })
            .style(crate::panel_widgets::input_style)
            .width(90)
            .padding([8, 10]),
        text(labels::LOCALSEND_FILES)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text2),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    page(
        labels::LOCALSEND_SETTINGS_RECEIVE,
        PopupTab::SettingsReceive,
        actions,
        column![
            setting(
                labels::LOCALSEND_ALWAYS_ON,
                button(if state.always_on {
                    labels::LOCALSEND_ON
                } else {
                    labels::LOCALSEND_OFF
                })
                .on_press((actions.always_on_changed)(!state.always_on))
                .style(button_style)
                .into()
            ),
            setting(
                labels::LOCALSEND_RECEIVE_DIR,
                row![
                    text(state.receive_dir.to_string_lossy().to_string())
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().text2)
                        .width(Fill),
                    button(labels::LOCALSEND_CHOOSE_RECEIVE_DIR)
                        .on_press(actions.choose_receive_dir.clone())
                        .style(button_style),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
            ),
            setting(labels::LOCALSEND_LIMITS, limits.into()),
        ]
        .spacing(8),
    )
}

pub(super) fn network<'a, M: Clone + 'static + 'a>(
    state: &'a LocalSendState,
    actions: &PopupActions<M>,
) -> Element<'a, M> {
    let status = state
        .server_status
        .as_deref()
        .unwrap_or(labels::LOCALSEND_STOPPED);
    page(
        labels::LOCALSEND_SETTINGS_NETWORK,
        PopupTab::SettingsNetwork,
        actions,
        column![
            setting(
                labels::LOCALSEND_SERVER,
                row![
                    button(labels::LOCALSEND_RESTART_SERVER)
                        .on_press(actions.server_restart.clone())
                        .style(button_style),
                    button(labels::LOCALSEND_STOP_SERVER)
                        .on_press(actions.server_stop.clone())
                        .style(button_style),
                ]
                .spacing(8)
                .into()
            ),
            setting(
                labels::LOCALSEND_ALIAS,
                text_input(labels::LOCALSEND_ALIAS_HINT, &state.alias)
                    .on_input(actions.alias_changed)
                    .style(crate::panel_widgets::input_style)
                    .width(240)
                    .padding([8, 10])
                    .into()
            ),
            setting(
                labels::LOCALSEND_PORT,
                text(gsh_localsend::protocol::PORT.to_string())
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text2)
                    .into(),
            ),
            setting(
                labels::LOCALSEND_MULTICAST,
                text(gsh_localsend::protocol::MULTICAST.to_string())
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text2)
                    .into(),
            ),
            setting(
                labels::LOCALSEND_STATE,
                text(status)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text2)
                    .into()
            ),
        ]
        .spacing(8),
    )
}

fn page<'a, M: Clone + 'static + 'a>(
    title: &'static str,
    tab: PopupTab,
    actions: &PopupActions<M>,
    body: iced::widget::Column<'a, M>,
) -> Element<'a, M> {
    column![
        row![
            nav(
                labels::LOCALSEND_SETTINGS_RECEIVE,
                tab == PopupTab::SettingsReceive,
                actions.settings_receive.clone()
            ),
            nav(
                labels::LOCALSEND_SETTINGS_NETWORK,
                tab == PopupTab::SettingsNetwork,
                actions.settings_network.clone()
            ),
        ]
        .spacing(8),
        container(
            column![
                text(title)
                    .font(theme::FONT_TITLE)
                    .size(theme::SETTINGS_NAV_TEXT_SIZE)
                    .color(theme::palette().text),
                body,
            ]
            .spacing(14)
        )
        .padding(20)
        .width(Fill)
        .style(|_| container::Style {
            background: Some(theme::palette().panel.into()),
            border: Border {
                color: theme::palette().border,
                width: 1.0,
                radius: theme::RADIUS_CARD.into()
            },
            ..Default::default()
        }),
    ]
    .spacing(14)
    .into()
}

fn setting<'a, M: 'static + 'a>(label: &'static str, control: Element<'a, M>) -> Element<'a, M> {
    row![
        text(label)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text)
            .width(Fill),
        control
    ]
    .spacing(16)
    .align_y(Alignment::Center)
    .padding([12, 0])
    .into()
}
