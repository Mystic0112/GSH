use crate::{
    labels,
    pane_chrome::ChromeCallbacks,
    pane_model::{PaneModel, PaneTab},
    pane_tab::view_tab,
    theme,
};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

pub(crate) fn view_tab_strip<'a, M, FSelect, FClose>(
    model: &'a PaneModel,
    callbacks: ChromeCallbacks<M, FSelect, FClose>,
    dirty: &dyn Fn(&std::path::Path) -> bool,
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
            matches!(tab, PaneTab::File(path) if dirty(path)),
            (callbacks.on_select_tab)(idx),
            (callbacks.on_close_tab)(idx),
        ));
    }

    let add_tab = view_add_tab_button(callbacks.on_new_terminal);
    let add_chat = view_add_chat_button(callbacks.on_new_chat);
    let farm =
        button(text(labels::FARM_TAB_TITLE).size(theme::SIZE_LABEL)).on_press(callbacks.on_farm);
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
            farm,
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

pub(crate) fn hairline_v<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(1.0).height(Fill))
        .width(Length::Fixed(1.0))
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::BORDER)),
            ..Default::default()
        })
        .into()
}
