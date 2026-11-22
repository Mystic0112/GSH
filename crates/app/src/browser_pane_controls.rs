use iced::widget::{button, container, text};
use iced::{Element, Fill, Length};

use super::{theme, BrowserMsg};

pub(super) fn nav_button<'a>(
    label: &'static str,
    on_press: Option<BrowserMsg>,
) -> Element<'a, BrowserMsg> {
    let enabled = on_press.is_some();
    button(
        container(
            text(label)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(if enabled {
                    theme::palette().text
                } else {
                    theme::palette().text4
                }),
        )
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(28.0))
    .height(Length::Fixed(28.0))
    .padding(0)
    .on_press_maybe(on_press)
    .style(theme::ghost_button)
    .into()
}

pub(super) fn action_button<'a>(label: &'static str, msg: BrowserMsg) -> Element<'a, BrowserMsg> {
    button(
        text(label)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text),
    )
    .padding([6, 10])
    .on_press(msg)
    .style(theme::ghost_button)
    .into()
}
