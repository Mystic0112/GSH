//! Moldura do pane central (`pane_view.dart` e `pane_tab_leading.dart` do Flutter).
//!
//! Barra de abas de 40px em `BG` com divisória inferior `BORDER` (aba ativa em
//! `PANEL` com indicador inferior `ACCENT` de 2px e hover `PANEL3`, abas
//! inativas em `BG` com texto `TEXT3`, botão `×` por aba, botão `+` que abre
//! novo terminal e controles de split/fechar) e corpo em `PANEL` com borda `BORDER`.

use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

use crate::labels;
use crate::pane_model::{PaneModel, PaneTab};
use crate::theme;

/// Callbacks de interação da barra de abas do pane central.
pub struct ChromeCallbacks<M, FSelect, FClose> {
    pub on_select_tab: FSelect,
    pub on_close_tab: FClose,
    pub on_new_terminal: M,
    pub on_new_chat: M,
    pub noop_msg: M,
}

/// Envolve o conteúdo ativo do pane (terminal ou viewer) com a barra de abas.
pub fn view<'a, M, FSelect, FClose>(
    model: &'a PaneModel,
    active_content: Element<'a, M>,
    callbacks: ChromeCallbacks<M, FSelect, FClose>,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FSelect: Fn(usize) -> M + 'a,
    FClose: Fn(usize) -> M + 'a,
{
    let tab_strip = view_tab_strip(model, callbacks);

    let body = container(active_content)
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

/// Estado visual vazio quando todas as abas do pane foram fechadas.
pub fn view_empty_body<'a, M: 'a>() -> Element<'a, M> {
    container(
        text(labels::PANE_EMPTY_STATE)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::TEXT3),
    )
    .padding([20, 24])
    .width(Fill)
    .height(Fill)
    .into()
}

fn view_tab_strip<'a, M, FSelect, FClose>(
    model: &'a PaneModel,
    callbacks: ChromeCallbacks<M, FSelect, FClose>,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FSelect: Fn(usize) -> M + 'a,
    FClose: Fn(usize) -> M + 'a,
{
    let active_idx = model.active_index();
    let mut tabs_row = row![].height(Fill).align_y(Alignment::Center);
    for (idx, tab) in model.tabs().iter().enumerate() {
        let is_active = active_idx == Some(idx);
        tabs_row = tabs_row.push(view_tab(
            tab,
            is_active,
            (callbacks.on_select_tab)(idx),
            (callbacks.on_close_tab)(idx),
        ));
    }

    let add_tab = view_add_tab_button(callbacks.on_new_terminal);
    let add_chat = view_add_chat_button(callbacks.on_new_chat);
    let noop = callbacks.noop_msg;

    let right_tools = row![
        pane_tool_button(labels::ICON_SPLIT_H, noop.clone()),
        pane_tool_button(labels::ICON_SPLIT_V, noop.clone()),
        pane_tool_button(labels::ICON_CLOSE, noop),
        Space::new().width(6.0),
    ]
    .spacing(2.0)
    .align_y(Alignment::Center);

    let bar = container(
        row![
            tabs_row,
            add_tab,
            add_chat,
            Space::new().width(Fill),
            right_tools
        ]
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

fn view_tab<'a, M: Clone + 'a>(
    tab: &PaneTab,
    is_active: bool,
    on_select: M,
    on_close: M,
) -> Element<'a, M> {
    let icon_color = if is_active {
        theme::ACCENT_TEXT
    } else {
        theme::TEXT3
    };
    let title_color = if is_active { theme::TEXT } else { theme::TEXT3 };

    let select_btn = button(
        row![
            text(tab.icon())
                .font(theme::FONT_MONO_BOLD)
                .size(theme::SIZE_SECTION_HEADER)
                .color(icon_color),
            Space::new().width(8.0),
            container(
                text(tab.title())
                    .font(theme::FONT_TAB)
                    .size(theme::SIZE_TAB)
                    .color(title_color)
                    .wrapping(text::Wrapping::None),
            )
            .width(Fill)
            .clip(true),
        ]
        .padding([0, 10])
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Fill)
    .padding(0)
    .on_press(on_select)
    .style(move |_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let bg = if hovered {
            theme::PANEL3
        } else if is_active {
            theme::PANEL
        } else {
            theme::BG
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: title_color,
            border: Border::default(),
            ..Default::default()
        }
    });

    let close_btn = button(
        container(
            text(labels::ICON_CLOSE)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_TITLE)
                .color(theme::TEXT3),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(24.0))
    .height(Length::Fixed(24.0))
    .padding(0)
    .on_press(on_close)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::PANEL3))
            } else {
                None
            },
            text_color: if hovered { theme::TEXT } else { theme::TEXT3 },
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let top_area = container(
        row![select_btn, close_btn, Space::new().width(6.0)]
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Fill)
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(if is_active {
            theme::PANEL
        } else {
            theme::BG
        })),
        ..Default::default()
    });

    let bottom_bar: Element<'a, M> = if is_active {
        container(Space::new().width(Fill).height(2.0))
            .width(Fill)
            .height(Length::Fixed(2.0))
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::ACCENT)),
                ..Default::default()
            })
            .into()
    } else {
        Space::new().width(Fill).height(2.0).into()
    };

    let tab_col = column![top_area, bottom_bar]
        .width(Length::Fixed(theme::TAB_WIDTH - 1.0))
        .height(Fill);

    row![tab_col, hairline_v()]
        .width(Length::Fixed(theme::TAB_WIDTH))
        .height(Fill)
        .into()
}

fn view_add_tab_button<'a, M: Clone + 'a>(on_new_terminal: M) -> Element<'a, M> {
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
    .on_press(on_new_terminal)
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

fn view_add_chat_button<'a, M: Clone + 'a>(on_new_chat: M) -> Element<'a, M> {
    let btn = button(
        container(
            row![
                text(labels::ICON_CHAT)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_LABEL)
                    .color(theme::TEXT3),
                Space::new().width(5.0),
                text(labels::CHAT_TAB_TITLE)
                    .font(theme::FONT_BODY_MEDIUM)
                    .size(theme::SIZE_LABEL)
                    .color(theme::TEXT3),
            ]
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(58.0))
    .height(Fill)
    .padding(0)
    .on_press(on_new_chat)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::PANEL3))
            } else {
                None
            },
            text_color: if hovered { theme::TEXT } else { theme::TEXT3 },
            border: Border::default(),
            ..Default::default()
        }
    });

    row![btn, hairline_v()]
        .width(Length::Fixed(59.0))
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
