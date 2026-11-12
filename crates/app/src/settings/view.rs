//! Interface da aba de configurações.
use super::{Error, Message, Section, Settings};
use crate::{labels, theme};
use gsh_config::schema::AppThemeMode;
use iced::widget::{button, checkbox, column, container, radio, row, text, text_input};
use iced::{Alignment, Background, Border, Element, Fill, Theme};

fn theme_choice(
    label: &'static str,
    mode: AppThemeMode,
    selected: AppThemeMode,
) -> Element<'static, Message> {
    radio(label, mode, Some(selected), Message::ThemeSelected)
        .style(|_, _| {
            let p = theme::palette();
            radio::Style {
                background: p.panel2.into(),
                dot_color: p.accent,
                border_width: 1.0,
                border_color: p.border2,
                text_color: Some(p.text2),
            }
        })
        .into()
}

pub fn view(s: &Settings) -> Element<'_, Message> {
    // `system` é exibido como escuro, que é o que ele resolve.
    let selected = match s.theme_mode {
        AppThemeMode::Light => AppThemeMode::Light,
        AppThemeMode::Dark | AppThemeMode::System => AppThemeMode::Dark,
    };
    let navigation = column![
        nav_item(
            labels::ICON_SETTINGS,
            labels::SETTINGS_GENERAL,
            Section::General,
            s.section
        ),
        nav_item(
            labels::ICON_LAYERS,
            labels::SETTINGS_APPEARANCE,
            Section::Appearance,
            s.section
        ),
        nav_item(
            labels::ICON_TERMINAL,
            labels::SETTINGS_TERMINAL,
            Section::Terminal,
            s.section
        ),
        nav_item(
            labels::ICON_FILE_ITEM,
            labels::SETTINGS_EDITOR,
            Section::Editor,
            s.section
        ),
        nav_item(
            labels::ICON_CHAT,
            labels::SETTINGS_NOTIFICATIONS,
            Section::Notifications,
            s.section
        ),
    ]
    .spacing(4)
    .width(200);
    let mut content = match s.section {
        Section::General => column![
            text(labels::SETTINGS_GENERAL)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            setting_row(
                labels::SETTINGS_LOCALE,
                text_input(labels::SETTINGS_LOCALE_HINT, &s.locale)
                    .on_input(Message::LocaleChanged)
                    .style(input_style)
                    .width(240)
                    .into()
            ),
            button(labels::SETTINGS_SAVE)
                .on_press(Message::Save)
                .style(primary_button)
        ],
        Section::Appearance => column![
            text(labels::SETTINGS_APPEARANCE)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            setting_row(
                labels::SETTINGS_THEME,
                row![
                    theme_choice(labels::SETTINGS_DARK, AppThemeMode::Dark, selected),
                    theme_choice(labels::SETTINGS_LIGHT, AppThemeMode::Light, selected)
                ]
                .spacing(16)
                .into()
            ),
            setting_row(
                labels::SETTINGS_INTERFACE_SIZE,
                text_input(labels::SETTINGS_INTERFACE_SIZE_HINT, &s.interface_size)
                    .on_input(Message::InterfaceSizeChanged)
                    .style(input_style)
                    .width(240)
                    .into()
            ),
            button(labels::SETTINGS_SAVE)
                .on_press(Message::SaveControls)
                .style(primary_button)
        ],
        Section::Terminal => column![
            text(labels::SETTINGS_TERMINAL)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            setting_row(
                labels::SETTINGS_SHELL,
                text_input(labels::SETTINGS_SHELL, &s.shell)
                    .on_input(Message::ShellChanged)
                    .style(input_style)
                    .width(240)
                    .into()
            ),
            setting_row(
                labels::SETTINGS_SCROLLBACK,
                text_input(labels::SETTINGS_SCROLLBACK, &s.scrollback)
                    .on_input(Message::ScrollbackChanged)
                    .style(input_style)
                    .width(240)
                    .into()
            ),
            setting_row(
                labels::SETTINGS_TERMINAL_SIZE,
                text_input(labels::SETTINGS_TERMINAL_SIZE, &s.terminal_size)
                    .on_input(Message::TerminalSizeChanged)
                    .style(input_style)
                    .width(240)
                    .into()
            ),
            text(labels::SETTINGS_TERMINAL_RESTART),
            button(labels::SETTINGS_SAVE)
                .on_press(Message::SaveControls)
                .style(primary_button)
        ],
        Section::Editor => column![
            text(labels::SETTINGS_EDITOR)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            setting_row(
                labels::SETTINGS_TAB_SIZE,
                text_input(labels::SETTINGS_TAB_SIZE, &s.tab_size)
                    .on_input(Message::TabSizeChanged)
                    .style(input_style)
                    .width(240)
                    .into()
            ),
            setting_row(
                labels::SETTINGS_HIGHLIGHT,
                checkbox(s.highlight)
                    .on_toggle(Message::HighlightChanged)
                    .style(check_style)
                    .into()
            ),
            setting_row(
                labels::SETTINGS_WRAP,
                checkbox(s.wrap)
                    .on_toggle(Message::WrapChanged)
                    .style(check_style)
                    .into()
            ),
            button(labels::SETTINGS_SAVE)
                .on_press(Message::SaveControls)
                .style(primary_button)
        ],
        Section::Notifications => column![
            text(labels::SETTINGS_NOTIFICATIONS)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            setting_row(
                labels::SETTINGS_AGENT_NOTIFICATIONS,
                checkbox(s.notifications)
                    .on_toggle(Message::NotificationsChanged)
                    .style(check_style)
                    .into()
            ),
            text(labels::SETTINGS_NOTIFICATIONS_PENDING),
            button(labels::SETTINGS_SAVE)
                .on_press(Message::SaveControls)
                .style(primary_button)
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
    let body = row![navigation, iced::widget::scrollable(content).height(Fill)].spacing(32);
    container(
        column![
            row![
                crate::panel_widgets::ghost(labels::ICON_BACK, Some(Message::Back), false),
                text(labels::SETTINGS_TITLE)
                    .font(theme::FONT_TITLE)
                    .size(theme::SIZE_TOPBAR_TITLE)
                    .color(theme::palette().text)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            crate::panel_widgets::hairline_h(),
            body
        ]
        .spacing(16),
    )
    .padding(24)
    .width(Fill)
    .height(Fill)
    .into()
}

fn nav_item(
    icon: &'static str,
    label: &'static str,
    section: Section,
    current: Section,
) -> Element<'static, Message> {
    let selected = section == current;
    button(
        row![
            text(icon)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TOPBAR_TITLE),
            text(label).font(theme::FONT_BODY).size(theme::SIZE_LABEL)
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([10, 12])
    .on_press(Message::Select(section))
    .style(move |_, status| button::Style {
        background: if selected {
            Some(Background::Color(theme::palette().accent_soft))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(Background::Color(theme::palette().panel3))
        } else {
            None
        },
        text_color: if selected {
            theme::palette().accent_text
        } else {
            theme::palette().text2
        },
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn primary_button(_: &Theme, status: button::Status) -> button::Style {
    let p = theme::palette();
    button::Style {
        background: Some(Background::Color(
            if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                p.accent_text
            } else {
                p.accent
            },
        )),
        text_color: p.bg,
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn input_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let p = theme::palette();
    text_input::Style {
        background: Background::Color(p.panel2),
        border: Border {
            color: if matches!(status, text_input::Status::Focused { .. }) {
                p.accent
            } else {
                p.border2
            },
            width: 1.0,
            radius: theme::RADIUS_CARD.into(),
        },
        icon: p.text3,
        placeholder: p.text3,
        value: p.text,
        selection: p.accent_soft,
    }
}

fn check_style(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let mut style = checkbox::primary(theme, status);
    let p = theme::palette();
    style.background = p.panel2.into();
    style.icon_color = p.accent;
    style.border.color = p.border2;
    style.text_color = Some(p.text2);
    style
}

fn setting_row<'a>(label: &'static str, control: Element<'a, Message>) -> Element<'a, Message> {
    column![
        row![
            text(label)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_RAIL_ITEM)
                .color(theme::palette().text)
                .width(Fill),
            control
        ]
        .align_y(Alignment::Center)
        .padding([12, 0]),
        crate::panel_widgets::hairline_h()
    ]
    .into()
}
