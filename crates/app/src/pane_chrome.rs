//! Moldura do pane central (`pane_view.dart` e `pane_tab_leading.dart` do Flutter).
//!
//! Barra de abas de 40px em `BG` com divisória inferior `BORDER` (aba ativa em
//! `PANEL` com indicador inferior `ACCENT` de 2px e hover `PANEL3`, botão `+` e
//! controles de split/fechar) e corpo do terminal em `PANEL` com borda `BORDER`.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

use crate::labels;
use crate::theme;

/// Envolve o conteúdo do pane (terminal) com a barra de abas e superfícies do design-system.
pub fn view<'a, M: Clone + 'a>(terminal_content: Element<'a, M>, noop_msg: M) -> Element<'a, M> {
    let tab_strip = view_tab_strip(noop_msg);

    let body = container(terminal_content)
        .width(Fill)
        .height(Fill)
        .padding(1)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::PANEL)),
            border: Border {
                color: theme::BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

    column![tab_strip, body].width(Fill).height(Fill).into()
}

fn view_tab_strip<'a, M: Clone + 'a>(noop_msg: M) -> Element<'a, M> {
    let active_tab = view_active_tab(noop_msg.clone());
    let add_tab = view_add_tab_button(noop_msg.clone());

    let right_tools = row![
        pane_tool_button(labels::ICON_SPLIT_H, noop_msg.clone()),
        pane_tool_button(labels::ICON_SPLIT_V, noop_msg.clone()),
        pane_tool_button(labels::ICON_CLOSE, noop_msg),
        Space::new().width(6.0),
    ]
    .spacing(2.0)
    .align_y(Alignment::Center);

    let bar = container(
        row![active_tab, add_tab, Space::new().width(Fill), right_tools]
            .width(Fill)
            .height(Length::Fixed(theme::TAB_STRIP_HEIGHT - 1.0))
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::TAB_STRIP_HEIGHT - 1.0))
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::BG)),
        ..Default::default()
    });

    let bottom_hairline = container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::BORDER)),
            ..Default::default()
        });

    column![bar, bottom_hairline]
        .width(Fill)
        .height(Length::Fixed(theme::TAB_STRIP_HEIGHT))
        .into()
}

fn view_active_tab<'a, M: Clone + 'a>(noop_msg: M) -> Element<'a, M> {
    let tab_row = row![
        text(labels::ICON_TERMINAL)
            .font(theme::FONT_MONO_BOLD)
            .size(theme::SIZE_SECTION_HEADER)
            .color(theme::ACCENT_TEXT),
        Space::new().width(8.0),
        container(
            text(labels::DEFAULT_TAB_TITLE)
                .font(theme::FONT_TAB)
                .size(theme::SIZE_TAB)
                .color(theme::TEXT),
        )
        .width(Fill),
        text(labels::ICON_CLOSE)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_TITLE)
            .color(theme::TEXT3),
    ]
    .padding([0, 12])
    .align_y(Alignment::Center);

    let accent_bar = container(Space::new().width(Fill).height(2.0))
        .width(Fill)
        .height(Length::Fixed(2.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::ACCENT)),
            ..Default::default()
        });

    let tab_button = button(
        column![
            container(tab_row).width(Fill).height(Fill).center_y(Fill),
            accent_bar,
        ]
        .width(Fill)
        .height(Fill),
    )
    .width(Length::Fixed(theme::TAB_WIDTH - 1.0))
    .height(Fill)
    .padding(0)
    .on_press(noop_msg)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if hovered {
                theme::PANEL3
            } else {
                theme::PANEL
            })),
            text_color: theme::TEXT,
            border: Border::default(),
            ..Default::default()
        }
    });

    row![tab_button, hairline_v()]
        .width(Length::Fixed(theme::TAB_WIDTH))
        .height(Fill)
        .into()
}

fn view_add_tab_button<'a, M: Clone + 'a>(noop_msg: M) -> Element<'a, M> {
    let btn = button(
        container(
            text(labels::ICON_PLUS)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_TOPBAR_TITLE)
                .color(theme::TEXT4),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(33.0))
    .height(Fill)
    .padding(0)
    .on_press(noop_msg)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::PANEL3))
            } else {
                None
            },
            text_color: if hovered { theme::TEXT2 } else { theme::TEXT4 },
            border: Border::default(),
            ..Default::default()
        }
    });

    row![btn, hairline_v()]
        .width(Length::Fixed(34.0))
        .height(Fill)
        .into()
}

fn pane_tool_button<'a, M: Clone + 'a>(glyph: &'static str, noop_msg: M) -> Element<'a, M> {
    button(
        container(
            text(glyph)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(theme::TEXT4),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(24.0))
    .height(Length::Fixed(24.0))
    .padding(0)
    .on_press(noop_msg)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::PANEL3))
            } else {
                None
            },
            text_color: if hovered { theme::TEXT2 } else { theme::TEXT4 },
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

fn hairline_v<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(1.0).height(Fill))
        .width(Length::Fixed(1.0))
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::BORDER)),
            ..Default::default()
        })
        .into()
}
