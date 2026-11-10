//! Interface da aba de configurações.
use super::{Error, Message, Section, Settings};
use crate::{labels, theme};
use gsh_config::schema::AppThemeMode;
use iced::widget::{button, checkbox, column, container, radio, row, text, text_input};
use iced::{Element, Fill};

fn theme_choice(
    label: &'static str,
    mode: AppThemeMode,
    selected: AppThemeMode,
) -> Element<'static, Message> {
    radio(label, mode, Some(selected), Message::ThemeSelected).into()
}

pub fn view(s: &Settings) -> Element<'_, Message> {
    // `system` é exibido como escuro, que é o que ele resolve.
    let selected = match s.theme_mode {
        AppThemeMode::Light => AppThemeMode::Light,
        AppThemeMode::Dark | AppThemeMode::System => AppThemeMode::Dark,
    };
    let navigation = column![
        button(labels::SETTINGS_GENERAL).on_press(Message::Select(Section::General)),
        button(labels::SETTINGS_APPEARANCE).on_press(Message::Select(Section::Appearance)),
        button(labels::SETTINGS_TERMINAL).on_press(Message::Select(Section::Terminal)),
        button(labels::SETTINGS_EDITOR).on_press(Message::Select(Section::Editor)),
        button(labels::SETTINGS_NOTIFICATIONS).on_press(Message::Select(Section::Notifications)),
    ]
    .spacing(20)
    .width(200);
    let mut content = match s.section {
        Section::General => column![
            text(labels::SETTINGS_GENERAL).size(24),
            text(labels::SETTINGS_LOCALE),
            text_input(labels::SETTINGS_LOCALE_HINT, &s.locale).on_input(Message::LocaleChanged),
            button(labels::SETTINGS_SAVE).on_press(Message::Save)
        ],
        Section::Appearance => column![
            text(labels::SETTINGS_APPEARANCE).size(24),
            text(labels::SETTINGS_THEME),
            theme_choice(labels::SETTINGS_DARK, AppThemeMode::Dark, selected),
            theme_choice(labels::SETTINGS_LIGHT, AppThemeMode::Light, selected),
            text(labels::SETTINGS_INTERFACE_SIZE),
            text_input(labels::SETTINGS_INTERFACE_SIZE_HINT, &s.interface_size)
                .on_input(Message::InterfaceSizeChanged),
            button(labels::SETTINGS_SAVE).on_press(Message::SaveControls)
        ],
        Section::Terminal => column![
            text(labels::SETTINGS_TERMINAL).size(24),
            text(labels::SETTINGS_SHELL),
            text_input(labels::SETTINGS_SHELL, &s.shell).on_input(Message::ShellChanged),
            text(labels::SETTINGS_SCROLLBACK),
            text_input(labels::SETTINGS_SCROLLBACK, &s.scrollback)
                .on_input(Message::ScrollbackChanged),
            text(labels::SETTINGS_TERMINAL_SIZE),
            text_input(labels::SETTINGS_TERMINAL_SIZE, &s.terminal_size)
                .on_input(Message::TerminalSizeChanged),
            text(labels::SETTINGS_TERMINAL_RESTART),
            button(labels::SETTINGS_SAVE).on_press(Message::SaveControls)
        ],
        Section::Editor => column![
            text(labels::SETTINGS_EDITOR).size(24),
            text(labels::SETTINGS_TAB_SIZE),
            text_input(labels::SETTINGS_TAB_SIZE, &s.tab_size).on_input(Message::TabSizeChanged),
            checkbox(s.highlight)
                .label(labels::SETTINGS_HIGHLIGHT)
                .on_toggle(Message::HighlightChanged),
            checkbox(s.wrap)
                .label(labels::SETTINGS_WRAP)
                .on_toggle(Message::WrapChanged),
            button(labels::SETTINGS_SAVE).on_press(Message::SaveControls)
        ],
        Section::Notifications => column![
            text(labels::SETTINGS_NOTIFICATIONS).size(24),
            checkbox(s.notifications)
                .label(labels::SETTINGS_AGENT_NOTIFICATIONS)
                .on_toggle(Message::NotificationsChanged),
            text(labels::SETTINGS_NOTIFICATIONS_PENDING),
            button(labels::SETTINGS_SAVE).on_press(Message::SaveControls)
        ],
    }
    .spacing(16)
    .width(Fill);
    if let Some(error) = &s.error {
        let message = match error {
            Error::InvalidLocale => labels::SETTINGS_INVALID_LOCALE,
            Error::InvalidInterfaceSize => labels::SETTINGS_INVALID_INTERFACE_SIZE,
            Error::InvalidTheme => labels::SETTINGS_INVALID_THEME,
            Error::InvalidShell => labels::SETTINGS_INVALID_SHELL,
            Error::InvalidScrollback => labels::SETTINGS_INVALID_SCROLLBACK,
            Error::InvalidTerminalSize => labels::SETTINGS_INVALID_TERMINAL_SIZE,
            Error::InvalidTabSize => labels::SETTINGS_INVALID_TAB_SIZE,
            Error::InvalidSection | Error::Config(_) | Error::Io(_) | Error::Document(_) => {
                labels::SETTINGS_CONFIG_ERROR
            }
        };
        content = content.push(text(message).color(theme::palette().warn));
    } else if s.saved {
        content = content.push(text(labels::SETTINGS_SAVED));
    }
    container(row![navigation, iced::widget::scrollable(content).height(Fill)].spacing(32))
        .padding(24)
        .width(Fill)
        .height(Fill)
        .into()
}
