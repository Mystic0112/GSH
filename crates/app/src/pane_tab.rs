use crate::{labels, pane_model::PaneTab, pane_strip::hairline_v, theme};
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

pub(crate) fn view_tab<'a, M: Clone + 'a>(
    tab: &PaneTab,
    is_active: bool,
    dirty: bool,
    on_select: M,
    on_close: M,
) -> Element<'a, M> {
    let icon_color = if is_active {
        theme::palette().accent_text
    } else {
        theme::palette().text3
    };
    let title_color = if is_active {
        theme::palette().accent_text
    } else {
        theme::palette().text3
    };

    let mut tab_label = row![
        text(tab.icon())
            .font(if tab.is_file() {
                theme::FONT_MONO_BOLD
            } else {
                theme::FONT_ICON
            })
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
    ];
    if dirty {
        tab_label = tab_label.push(iced::widget::tooltip(
            text(labels::EDITOR_DIRTY)
                .size(theme::SIZE_TAB)
                .color(theme::palette().accent),
            text(labels::EDITOR_DIRTY_TOOLTIP),
            iced::widget::tooltip::Position::Bottom,
        ));
    }
    let select_btn = button(tab_label.padding([0, 10]).align_y(Alignment::Center))
        .width(Fill)
        .height(Fill)
        .padding(0)
        .on_press(on_select)
        .style(move |_: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let bg = if hovered {
                theme::palette().panel3
            } else {
                theme::palette().bg
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
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text3),
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
                Some(Background::Color(theme::palette().panel3))
            } else {
                None
            },
            text_color: if hovered {
                theme::palette().text
            } else {
                theme::palette().text3
            },
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
        background: Some(Background::Color(theme::palette().bg)),
        ..Default::default()
    });

    let bottom_bar: Element<'a, M> = if is_active {
        container(Space::new().width(Fill).height(2.0))
            .width(Fill)
            .height(Length::Fixed(2.0))
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::palette().accent)),
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
