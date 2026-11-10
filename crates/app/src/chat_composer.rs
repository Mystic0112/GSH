//! Componente visual do composer do chat (`chat_composer.dart` do Flutter).
//!
//! Exibe a faixa de contexto (pasta + branch), a caixa de texto com atalho
//! `Enter` para envio e botão de Enviar/Parar, e a barra inferior de status.

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

use crate::chat_pane::{ChatMsg, ChatPane};
use crate::labels;
use crate::theme;

/// Renderiza o composer completo na parte inferior do [`ChatPane`].
pub fn view(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let context_strip = view_context_strip(pane);
    let input_box = view_input_box(pane);
    let bottom_bar = view_bottom_bar(pane);

    column![
        context_strip,
        Space::new().height(6.0),
        input_box,
        Space::new().height(4.0),
        bottom_bar,
    ]
    .width(Fill)
    .into()
}

fn view_context_strip(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut strip_row = row![text(pane.folder_label())
        .font(theme::FONT_MONO)
        .size(theme::SIZE_TAB)
        .color(theme::TEXT3)]
    .align_y(Alignment::Center);

    if let Some(branch) = pane.branch().filter(|b| !b.is_empty()) {
        strip_row = strip_row.push(Space::new().width(10.0)).push(
            text(branch)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_TAB)
                .color(theme::TEXT3),
        );
    }

    container(strip_row)
        .width(Fill)
        .padding([9, 12])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::PANEL2)),
            border: Border {
                radius: theme::RADIUS_CARD.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn view_input_box(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut input = text_input(labels::CHAT_PLACEHOLDER, pane.input())
        .on_input(ChatMsg::InputChanged)
        .font(theme::FONT_BODY)
        .size(theme::SIZE_BODY)
        .padding([6, 8])
        .style(|_: &Theme, _status| text_input::Style {
            background: Background::Color(theme::PANEL2),
            border: Border::default(),
            icon: theme::TEXT3,
            placeholder: theme::TEXT4,
            value: theme::TEXT,
            selection: theme::ACCENT_SOFT,
        });

    if !pane.is_busy() {
        input = input.on_submit(ChatMsg::Send);
    }

    let action_btn = if pane.is_busy() {
        view_stop_button()
    } else {
        view_send_button(!pane.input().trim().is_empty())
    };

    container(
        row![input, Space::new().width(4.0), action_btn]
            .width(Fill)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([4, 6])
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::PANEL2)),
        border: Border {
            color: theme::BORDER2,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    })
    .into()
}

fn view_send_button<'a>(enabled: bool) -> Element<'a, ChatMsg> {
    let mut btn = button(
        container(
            row![
                text(labels::ICON_SEND)
                    .font(theme::FONT_MONO_BOLD)
                    .size(theme::SIZE_LABEL)
                    .color(if enabled {
                        theme::ON_ACCENT
                    } else {
                        theme::TEXT4
                    }),
                Space::new().width(4.0),
                text(labels::CHAT_SEND)
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(theme::SIZE_LABEL)
                    .color(if enabled {
                        theme::ON_ACCENT
                    } else {
                        theme::TEXT4
                    }),
            ]
            .align_y(Alignment::Center),
        )
        .padding([5, 10]),
    )
    .padding(0)
    .style(move |_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let bg = if !enabled {
            theme::PANEL3
        } else if hovered {
            theme::ACCENT_HOVER
        } else {
            theme::ACCENT
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: if enabled {
                theme::ON_ACCENT
            } else {
                theme::TEXT4
            },
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    if enabled {
        btn = btn.on_press(ChatMsg::Send);
    }
    btn.into()
}

fn view_stop_button<'a>() -> Element<'a, ChatMsg> {
    button(
        container(
            row![
                text(labels::ICON_STOP)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_SECTION_HEADER)
                    .color(theme::ERROR),
                Space::new().width(5.0),
                text(labels::CHAT_STOP)
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(theme::SIZE_LABEL)
                    .color(theme::TEXT),
            ]
            .align_y(Alignment::Center),
        )
        .padding([5, 10]),
    )
    .padding(0)
    .on_press(ChatMsg::Cancel)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if hovered {
                theme::PANEL3
            } else {
                theme::BG
            })),
            text_color: theme::TEXT,
            border: Border {
                color: theme::ERROR,
                width: 1.0,
                radius: theme::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn view_bottom_bar(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut bar = row![
        text(labels::CHAT_MODE_DEFAULT)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::TEXT2),
        Space::new().width(Fill),
    ]
    .height(Length::Fixed(22.0))
    .align_y(Alignment::Center);

    if pane.cost_usd() > 0.0 {
        bar = bar
            .push(
                text(labels::format_chat_cost(pane.cost_usd()))
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_SECTION_HEADER)
                    .color(theme::TEXT3),
            )
            .push(Space::new().width(10.0));
    }

    if let Some(model) = pane.model().filter(|m| !m.is_empty()) {
        bar = bar.push(
            text(model)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::TEXT2),
        );
    }

    if pane.is_busy() {
        bar = bar.push(Space::new().width(10.0)).push(
            text(labels::CHAT_WORKING)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(theme::ACCENT_TEXT),
        );
    }

    container(bar).padding([0, 6]).width(Fill).into()
}
