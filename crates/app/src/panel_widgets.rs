//! Superfícies e ações compartilhadas pelos painéis laterais.
use crate::theme;
use iced::widget::{button, container, row, text, Space};
use iced::{Background, Border, Element, Fill, Length, Theme};

pub fn ghost<'a, M: Clone + 'a>(
    glyph: &'static str,
    message: Option<M>,
    active: bool,
) -> Element<'a, M> {
    button(
        container(text(glyph).font(theme::FONT_MONO).size(theme::SIZE_TITLE))
            .center_x(Fill)
            .center_y(Fill),
    )
    .width(28)
    .height(28)
    .padding(0)
    .on_press_maybe(message)
    .style(move |_: &Theme, status| button::Style {
        background: if active {
            Some(Background::Color(theme::ACCENT_SOFT))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(Background::Color(theme::PANEL3))
        } else {
            None
        },
        text_color: if matches!(status, button::Status::Disabled) && !active {
            theme::TEXT4
        } else {
            theme::TEXT2
        },
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
pub fn hairline_h<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(Fill).height(1))
        .width(Fill)
        .height(1)
        .style(|_| container::Style {
            background: Some(theme::BORDER.into()),
            ..Default::default()
        })
        .into()
}
pub fn frame<'a, M: 'a>(content: Element<'a, M>) -> Element<'a, M> {
    let line = container(Space::new().width(1).height(Fill))
        .width(1)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(theme::BORDER.into()),
            ..Default::default()
        });
    row![
        line,
        container(content)
            .width(Length::Fixed(theme::FILES_PANEL_WIDTH - 1.0))
            .height(Fill)
            .style(|_| container::Style {
                background: Some(theme::BG.into()),
                ..Default::default()
            })
    ]
    .width(theme::FILES_PANEL_WIDTH)
    .height(Fill)
    .into()
}
