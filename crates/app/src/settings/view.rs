//! Interface da aba de configurações.
use super::{Error, Message, Section, Settings};
use crate::{labels, theme};
use gsh_config::schema::{AppThemeMode, TranscriptWidth};
use iced::widget::{button, checkbox, column, container, radio, row, text, text_input};
use iced::{Alignment, Background, Border, Element, Fill, Theme};

fn theme_choice(
    label: &'static str,
    mode: AppThemeMode,
    selected: AppThemeMode,
) -> Element<'static, Message> {
    radio(label, mode, Some(selected), Message::ThemeSelected)
        .style(radio_style)
        .into()
}

fn radio_style(_: &Theme, _: radio::Status) -> radio::Style {
    let p = theme::palette();
    radio::Style {
        background: p.panel2.into(),
        dot_color: p.accent,
        border_width: 1.0,
        border_color: p.border2,
        text_color: Some(p.text2),
    }
}

pub fn view(s: &Settings) -> Element<'_, Message> {
    // `system` é exibido como escuro, que é o que ele resolve.
    let selected = match s.theme_mode {
        AppThemeMode::Light => AppThemeMode::Light,
        AppThemeMode::Dark | AppThemeMode::System => AppThemeMode::Dark,
    };
    let navigation = navigation(s);
    let content = match s.section {
        Section::General => column![
            text(labels::SETTINGS_APPEARANCE)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SETTINGS_SECTION_TITLE_SIZE)
                .color(theme::palette().text),
            setting_row(
                labels::SETTINGS_THEME,
                row![
                    theme_choice(labels::SETTINGS_DARK, AppThemeMode::Dark, selected),
                    theme_choice(labels::SETTINGS_LIGHT, AppThemeMode::Light, selected)
                ]
                .spacing(theme::SETTINGS_NAV_ITEM_GAP)
                .into()
            ),
            iced::widget::Space::new().height(theme::SETTINGS_SECTION_GAP),
            text(labels::SETTINGS_GSH_DESKTOP)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SETTINGS_SECTION_TITLE_SIZE)
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
            setting_row(
                labels::SETTINGS_TRANSCRIPT_WIDTH,
                row![
                    radio(
                        labels::SETTINGS_WIDTH_NARROW,
                        TranscriptWidth::Narrow,
                        Some(s.transcript_width),
                        Message::TranscriptWidthChanged
                    )
                    .style(radio_style),
                    radio(
                        labels::SETTINGS_WIDTH_MEDIUM,
                        TranscriptWidth::Medium,
                        Some(s.transcript_width),
                        Message::TranscriptWidthChanged
                    )
                    .style(radio_style),
                    radio(
                        labels::SETTINGS_WIDTH_WIDE,
                        TranscriptWidth::Wide,
                        Some(s.transcript_width),
                        Message::TranscriptWidthChanged
                    )
                    .style(radio_style)
                ]
                .spacing(theme::SETTINGS_NAV_ITEM_GAP)
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
                labels::SETTINGS_NOTIFY_TURN_DONE,
                checkbox(s.notifications)
                    .on_toggle(Message::NotificationsChanged)
                    .style(check_style)
                    .into()
            ),
            setting_row(
                labels::SETTINGS_NOTIFY_PERMISSION,
                checkbox(s.permission_sound)
                    .on_toggle(Message::PermissionSoundChanged)
                    .style(check_style)
                    .into()
            ),
            text(labels::SETTINGS_NOTIFICATIONS_PENDING),
            button(labels::SETTINGS_SAVE)
                .on_press(Message::SaveControls)
                .style(primary_button)
        ],
    }
    .spacing(theme::SETTINGS_SECTION_TITLE_GAP)
    .width(Fill);
    // ponytail: o lado Claude lê ~/.claude (fora do escopo); mostra o aviso até portar
    let mut content = if s.nav_claude {
        column![text(labels::SETTINGS_CLAUDE_UNAVAILABLE).color(theme::palette().text3)].width(Fill)
    } else {
        content
    };
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
    let panel = container(content)
        .max_width(theme::SETTINGS_PANEL_MAX_WIDTH)
        .width(Fill);
    let panel = container(panel)
        .width(Fill)
        .center_x(Fill)
        .padding(iced::Padding {
            top: f32::from(theme::SETTINGS_PANEL_TOP),
            right: f32::from(theme::SETTINGS_PANEL_HORIZONTAL),
            bottom: f32::from(theme::SETTINGS_PANEL_BOTTOM),
            left: f32::from(theme::SETTINGS_PANEL_HORIZONTAL),
        });
    let nav_border =
        container(iced::widget::Space::new().width(1).height(Fill)).style(|_| container::Style {
            background: Some(theme::palette().border.into()),
            ..Default::default()
        });
    let body = row![
        navigation,
        nav_border,
        iced::widget::scrollable(panel).height(Fill)
    ]
    .height(Fill);
    container(column![
        row![
            button(
                container(
                    text(labels::ICON_BACK)
                        .font(theme::FONT_ICON)
                        .size(theme::SETTINGS_HEADER_ICON),
                )
                .center_x(Fill)
                .center_y(Fill)
            )
            .width(theme::SETTINGS_HEADER_BUTTON)
            .height(theme::SETTINGS_HEADER_BUTTON)
            .padding(0)
            .on_press(Message::Back)
            .style(header_button),
            text(labels::SETTINGS_TITLE)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TOPBAR_TITLE)
                .color(theme::palette().text)
        ]
        .spacing(theme::SETTINGS_HEADER_GAP)
        .padding([0, theme::SETTINGS_NAV_PADDING])
        .align_y(Alignment::Center),
        crate::panel_widgets::hairline_h(),
        body
    ])
    .width(Fill)
    .height(Fill)
    .into()
}

fn matches_search(label: &str, query: &str) -> bool {
    label.to_lowercase().contains(&query.trim().to_lowercase())
}

fn navigation(s: &Settings) -> Element<'_, Message> {
    let entries = [
        (
            labels::ICON_SETTINGS,
            labels::SETTINGS_GENERAL,
            Section::General,
        ),
        ("", labels::SETTINGS_APPEARANCE, Section::Appearance),
        (
            labels::ICON_TERMINAL,
            labels::SETTINGS_TERMINAL,
            Section::Terminal,
        ),
        (
            labels::ICON_FILE_ITEM,
            labels::SETTINGS_EDITOR,
            Section::Editor,
        ),
        ("", labels::SETTINGS_NOTIFICATIONS, Section::Notifications),
    ];
    let mut items = column![].spacing(theme::SETTINGS_NAV_ITEM_SPACING);
    let mut found = false;
    for (icon, label, section) in entries {
        if !s.nav_claude && matches_search(label, &s.nav_search) {
            found = true;
            items = items.push(nav_item(icon, label, section, s.section));
        }
    }
    let group = if s.nav_claude {
        labels::SETTINGS_NAV_CLAUDE
    } else if found {
        labels::SETTINGS_GSH_DESKTOP
    } else {
        labels::SETTINGS_NAV_NO_RESULTS
    };
    let heading = container(
        text(group)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_NAV_GROUP_SIZE)
            .color(theme::palette().text3),
    )
    .padding(iced::Padding {
        top: theme::SETTINGS_NAV_GROUP_TOP,
        right: f32::from(theme::SETTINGS_NAV_ITEM_HORIZONTAL),
        bottom: theme::SETTINGS_NAV_GROUP_BOTTOM,
        left: f32::from(theme::SETTINGS_NAV_ITEM_HORIZONTAL),
    });
    let search = text_input(labels::SETTINGS_NAV_SEARCH, &s.nav_search)
        .on_input(Message::SearchChanged)
        .padding([
            theme::SETTINGS_NAV_SEARCH_VERTICAL,
            theme::SETTINGS_NAV_SEARCH_HORIZONTAL,
        ])
        .size(theme::SETTINGS_NAV_TEXT_SIZE)
        .style(nav_search_style);
    let segments = container(row![
        segment(labels::SETTINGS_GSH_DESKTOP, false, s.nav_claude),
        segment(labels::SETTINGS_NAV_CLAUDE, true, s.nav_claude)
    ])
    .padding(theme::SETTINGS_NAV_SEGMENT_PADDING)
    .style(|_| container::Style {
        background: Some(theme::palette().panel2.into()),
        border: Border {
            radius: theme::SETTINGS_NAV_SEARCH_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let list =
        iced::widget::scrollable(column![heading, items].padding([0, theme::SETTINGS_NAV_PADDING]))
            .height(Fill);
    container(column![
        container(segments).padding(iced::Padding {
            top: f32::from(theme::SETTINGS_NAV_PADDING),
            right: f32::from(theme::SETTINGS_NAV_PADDING),
            bottom: theme::SETTINGS_NAV_SEGMENT_BOTTOM,
            left: f32::from(theme::SETTINGS_NAV_PADDING)
        }),
        container(search).padding(iced::Padding {
            top: 0.0,
            right: f32::from(theme::SETTINGS_NAV_PADDING),
            bottom: theme::SETTINGS_NAV_SEARCH_BOTTOM,
            left: f32::from(theme::SETTINGS_NAV_PADDING)
        }),
        list,
        container(
            text(concat!("GSH Desktop ", env!("CARGO_PKG_VERSION")))
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text4)
        )
        .padding([
            theme::SETTINGS_NAV_FOOTER_VERTICAL,
            theme::SETTINGS_NAV_FOOTER_HORIZONTAL
        ])
    ])
    .width(theme::SETTINGS_NAV_WIDTH)
    .height(Fill)
    .style(|_| container::Style {
        background: Some(theme::palette().bg.into()),
        ..Default::default()
    })
    .into()
}

fn segment(label: &'static str, claude: bool, current: bool) -> Element<'static, Message> {
    let selected = claude == current;
    button(
        text(label)
            .font(if selected {
                theme::FONT_BODY_MEDIUM
            } else {
                theme::FONT_BODY
            })
            .size(theme::SETTINGS_NAV_SEGMENT_TEXT),
    )
    .padding([
        theme::SETTINGS_NAV_SEGMENT_VERTICAL,
        theme::SETTINGS_NAV_SEGMENT_HORIZONTAL,
    ])
    .on_press(Message::SideChanged(claude))
    .style(move |_, status| button::Style {
        background: if selected {
            Some(theme::palette().panel3.into())
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(theme::palette().panel.into())
        } else {
            None
        },
        text_color: if selected {
            theme::palette().text
        } else {
            theme::palette().text2
        },
        border: Border {
            color: if selected {
                theme::palette().border2
            } else {
                theme::palette().panel2
            },
            width: if selected { 1.0 } else { 0.0 },
            radius: theme::SETTINGS_NAV_SEGMENT_RADIUS.into(),
        },
        ..Default::default()
    })
    .into()
}

fn nav_search_style(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = input_style(theme, status);
    style.border.radius = theme::SETTINGS_NAV_SEARCH_RADIUS.into();
    style
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
                .size(theme::SETTINGS_NAV_ICON_SIZE),
            text(label)
                .font(if selected {
                    theme::FONT_BODY_SEMIBOLD
                } else {
                    theme::FONT_BODY
                })
                .size(theme::SETTINGS_NAV_TEXT_SIZE)
        ]
        .spacing(theme::SETTINGS_NAV_ITEM_GAP)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([
        theme::SETTINGS_NAV_ITEM_VERTICAL,
        theme::SETTINGS_NAV_ITEM_HORIZONTAL,
    ])
    .on_press(Message::Select(section))
    .style(move |_, status| button::Style {
        background: if selected
            || matches!(status, button::Status::Hovered | button::Status::Pressed)
        {
            Some(Background::Color(theme::palette().panel3))
        } else {
            None
        },
        text_color: if selected {
            theme::palette().text
        } else {
            theme::palette().text2
        },
        border: Border {
            radius: theme::SETTINGS_NAV_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn header_button(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(theme::palette().panel3.into())
        } else {
            None
        },
        text_color: theme::palette().text2,
        border: Border {
            radius: theme::SETTINGS_HEADER_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    }
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
                .size(theme::SETTINGS_NAV_TEXT_SIZE)
                .color(theme::palette().text)
                .width(Fill),
            iced::widget::Space::new().width(theme::SETTINGS_ROW_GAP),
            control
        ]
        .align_y(Alignment::Center)
        .padding([theme::SETTINGS_ROW_VERTICAL, 0]),
        crate::panel_widgets::hairline_h()
    ]
    .into()
}

#[cfg(test)]
mod nav_tests {
    use super::matches_search;
    #[test]
    fn filter_matches_case_insensitively_and_trims_query() {
        assert!(matches_search("Aparência", "  ARÊN  "));
        assert!(matches_search("Terminal", "  "));
        assert!(!matches_search("Terminal", "editor"));
    }
}
