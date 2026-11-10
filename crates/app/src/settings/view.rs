//! Interface da aba de configurações.

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Fill};

use super::{Error, Message, Settings};
use crate::{labels, theme};

pub fn view(settings: &Settings) -> Element<'_, Message> {
    let navigation = column![
        text(labels::SETTINGS_GENERAL).color(theme::TEXT),
        section(labels::SETTINGS_APPEARANCE),
        section(labels::SETTINGS_TERMINAL),
        section(labels::SETTINGS_EDITOR),
        section(labels::SETTINGS_NOTIFICATIONS),
    ]
    .spacing(20)
    .width(200);

    let mut content = column![
        text(labels::SETTINGS_GENERAL).size(24),
        text(labels::SETTINGS_LOCALE),
        text_input(labels::SETTINGS_LOCALE_HINT, &settings.locale).on_input(Message::LocaleChanged),
        button(labels::SETTINGS_SAVE).on_press(Message::Save),
    ]
    .spacing(16)
    .width(Fill);
    if let Some(error) = &settings.error {
        let message = match error {
            Error::InvalidLocale => labels::SETTINGS_INVALID_LOCALE,
            Error::Config(_) | Error::Io(_) | Error::Document(_) => labels::SETTINGS_CONFIG_ERROR,
        };
        content = content.push(text(message).color(theme::WARN));
    } else if settings.saved {
        content = content.push(text(labels::SETTINGS_SAVED));
    }
    container(row![navigation, content].spacing(32))
        .padding(24)
        .width(Fill)
        .height(Fill)
        .into()
}

fn section(name: &'static str) -> Element<'static, Message> {
    row![
        text(name).color(theme::TEXT3),
        text(labels::SETTINGS_COMING_SOON)
            .size(11)
            .color(theme::TEXT3)
    ]
    .spacing(8)
    .into()
}
