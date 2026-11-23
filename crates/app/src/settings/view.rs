//! Interface da aba de configurações.
use super::{claude, Error, Message, Section, Settings};
use crate::scheduled_automations::{RunStatus, WEEKDAYS};
use crate::{labels, theme};
use gsh_config::schema::{AppFeature, AppThemeMode, HarnessKind, TranscriptWidth};
use iced::widget::{button, column, container, pick_list, row, text, text_editor, text_input};
use iced::{Alignment, Background, Border, Element, Fill, Theme};

pub(super) fn choice<M: Clone + 'static>(
    label: &'static str,
    selected: bool,
    message: M,
) -> Element<'static, M> {
    button(
        text(label)
            .font(if selected {
                theme::FONT_BODY_MEDIUM
            } else {
                theme::FONT_BODY
            })
            .size(theme::SETTINGS_SEGMENT_TEXT),
    )
    .padding([
        theme::SETTINGS_SEGMENT_VERTICAL,
        theme::SETTINGS_SEGMENT_HORIZONTAL,
    ])
    .on_press(message)
    .style(move |_, status| {
        let p = theme::palette();
        button::Style {
            background: if selected {
                Some(p.panel3.into())
            } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                Some(p.panel.into())
            } else {
                None
            },
            text_color: if selected { p.text } else { p.text2 },
            border: Border {
                color: p.border2,
                width: if selected { 1.0 } else { 0.0 },
                radius: theme::SETTINGS_SEGMENT_RADIUS.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn theme_choices(selected: AppThemeMode) -> Element<'static, Message> {
    segmented(row![
        choice(
            labels::SETTINGS_LOCALE_SYSTEM,
            selected == AppThemeMode::System,
            Message::ThemeSelected(AppThemeMode::System)
        ),
        choice(
            labels::SETTINGS_LIGHT,
            selected == AppThemeMode::Light,
            Message::ThemeSelected(AppThemeMode::Light)
        ),
        choice(
            labels::SETTINGS_DARK,
            selected == AppThemeMode::Dark,
            Message::ThemeSelected(AppThemeMode::Dark)
        )
    ])
}

pub(super) fn segmented<'a, M: 'a>(choices: iced::widget::Row<'a, M>) -> Element<'a, M> {
    container(choices)
        .padding(theme::SETTINGS_SEGMENT_PADDING)
        .style(|_| container::Style {
            background: Some(theme::palette().panel2.into()),
            border: Border {
                radius: theme::SETTINGS_SEGMENT_OUTER_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// `Switch` do shadcn (também usado nos diálogos de outras abas).
pub fn switch<M: Clone + 'static>(value: bool, message: M) -> Element<'static, M> {
    let p = theme::palette();
    let knob = container(
        iced::widget::Space::new()
            .width(theme::SETTINGS_SWITCH_KNOB)
            .height(theme::SETTINGS_SWITCH_KNOB),
    )
    .width(theme::SETTINGS_SWITCH_KNOB)
    .height(theme::SETTINGS_SWITCH_KNOB)
    .style(move |_| container::Style {
        background: Some(if value { p.bg } else { p.text2 }.into()),
        border: Border {
            radius: theme::SETTINGS_SWITCH_KNOB.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    button(
        container(knob)
            .width(theme::SETTINGS_SWITCH_WIDTH)
            .height(theme::SETTINGS_SWITCH_HEIGHT)
            .align_x(if value {
                iced::alignment::Horizontal::Right
            } else {
                iced::alignment::Horizontal::Left
            })
            .align_y(iced::alignment::Vertical::Center)
            .padding(theme::SETTINGS_SWITCH_PADDING),
    )
    .width(theme::SETTINGS_SWITCH_WIDTH)
    .height(theme::SETTINGS_SWITCH_HEIGHT)
    .padding(0)
    .on_press(message)
    .style(move |_, status| button::Style {
        background: Some(
            if value {
                p.accent
            } else if matches!(status, button::Status::Hovered) {
                p.panel3
            } else {
                p.panel2
            }
            .into(),
        ),
        border: Border {
            color: p.border2,
            width: 1.0,
            radius: theme::SETTINGS_SWITCH_HEIGHT.into(),
        },
        ..Default::default()
    })
    .into()
}

pub fn view(s: &Settings) -> Element<'_, Message> {
    let selected = s.theme_mode;
    let navigation = navigation(s);
    let content = match s.section {
        Section::General => column![
            text(labels::SETTINGS_APPEARANCE)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SETTINGS_SECTION_TITLE_SIZE)
                .color(theme::palette().text),
            setting_row(labels::SETTINGS_THEME, theme_choices(selected)),
            iced::widget::Space::new().height(theme::SETTINGS_SECTION_GAP),
            text(labels::SETTINGS_GSH_DESKTOP)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SETTINGS_SECTION_TITLE_SIZE)
                .color(theme::palette().text),
            setting_row(
                labels::SETTINGS_LOCALE,
                pick_list(
                    &labels::SETTINGS_LOCALES[..],
                    Some(match s.locale.as_str() {
                        "en" => labels::SETTINGS_LOCALES[1],
                        "es" => labels::SETTINGS_LOCALES[2],
                        "pt-BR" => labels::SETTINGS_LOCALES[3],
                        _ => labels::SETTINGS_LOCALES[0],
                    }),
                    |value| Message::LocaleChanged(
                        match value {
                            labels::SETTINGS_LOCALE_EN => "en",
                            labels::SETTINGS_LOCALE_ES => "es",
                            labels::SETTINGS_LOCALE_PT => "pt-BR",
                            _ => "",
                        }
                        .to_owned()
                    )
                )
                .style(locale_style)
                .menu_style(|_| {
                    let p = theme::palette();
                    iced::widget::overlay::menu::Style {
                        text_color: p.text,
                        background: p.panel2.into(),
                        border: Border {
                            color: p.border2,
                            width: 1.0,
                            radius: theme::SETTINGS_DROPDOWN_RADIUS.into(),
                        },
                        selected_text_color: p.text,
                        selected_background: p.panel3.into(),
                        shadow: Default::default(),
                    }
                })
                .into()
            )
        ],
        Section::Appearance => column![
            text(labels::SETTINGS_APPEARANCE)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            setting_row(labels::SETTINGS_THEME, theme_choices(selected)),
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
                segmented(row![
                    choice(
                        labels::SETTINGS_WIDTH_NARROW,
                        s.transcript_width == TranscriptWidth::Narrow,
                        Message::TranscriptWidthChanged(TranscriptWidth::Narrow)
                    ),
                    choice(
                        labels::SETTINGS_WIDTH_MEDIUM,
                        s.transcript_width == TranscriptWidth::Medium,
                        Message::TranscriptWidthChanged(TranscriptWidth::Medium)
                    ),
                    choice(
                        labels::SETTINGS_WIDTH_WIDE,
                        s.transcript_width == TranscriptWidth::Wide,
                        Message::TranscriptWidthChanged(TranscriptWidth::Wide)
                    )
                ])
            )
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
            text(labels::SETTINGS_TERMINAL_RESTART)
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
                switch(s.highlight, Message::HighlightChanged(!s.highlight))
            ),
            setting_row(
                labels::SETTINGS_WRAP,
                switch(s.wrap, Message::WrapChanged(!s.wrap))
            )
        ],
        Section::Notifications => column![
            text(labels::SETTINGS_NOTIFICATIONS)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            setting_row(
                labels::SETTINGS_AGENT_NOTIFICATIONS,
                switch(
                    s.notifications,
                    Message::NotificationsChanged(!s.notifications)
                )
            )
        ],
        Section::Team => {
            let team = &s.current.farm.team;
            let mut rows = column![
                text(labels::SETTINGS_TEAM)
                    .font(theme::FONT_TITLE)
                    .size(theme::SIZE_TITLE)
                    .color(theme::palette().text),
                setting_row_desc(
                    labels::SETTINGS_TEAM_ENABLE,
                    labels::SETTINGS_TEAM_ENABLE_DESC,
                    switch(team.enabled, Message::TeamEnabledChanged(!team.enabled))
                ),
            ];
            if team.enabled {
                rows = rows
                    .push(setting_row_desc(
                        labels::SETTINGS_TEAM_NAME,
                        labels::SETTINGS_TEAM_NAME_DESC,
                        text_input(labels::SETTINGS_TEAM_NAME_HINT, &team.name)
                            .on_input(Message::TeamNameChanged)
                            .style(input_style)
                            .width(240)
                            .into(),
                    ))
                    .push(setting_row_desc(
                        labels::SETTINGS_TEAM_ME,
                        labels::SETTINGS_TEAM_ME_DESC,
                        text_input(
                            option_env!("USER")
                                .or(option_env!("USERNAME"))
                                .unwrap_or(""),
                            &team.me,
                        )
                        .on_input(Message::TeamDisplayNameChanged)
                        .style(input_style)
                        .width(240)
                        .into(),
                    ))
                    .push(setting_row_desc(
                        labels::SETTINGS_TEAM_SHOW_TITLES,
                        labels::SETTINGS_TEAM_SHOW_TITLES_DESC,
                        switch(
                            team.show_titles,
                            Message::TeamShowTitlesChanged(!team.show_titles),
                        ),
                    ))
                    .push(setting_row_desc(
                        labels::SETTINGS_TEAM_SECRET,
                        labels::SETTINGS_TEAM_SECRET_DESC,
                        text_input(labels::SETTINGS_TEAM_SECRET_HINT, &team.secret)
                            .secure(true)
                            .on_input(Message::TeamSecretChanged)
                            .style(input_style)
                            .width(240)
                            .into(),
                    ));
                if !team.secret.is_empty() {
                    rows = rows
                        .push(setting_row_desc(
                            labels::SETTINGS_TEAM_DOOR_ALLOW,
                            labels::SETTINGS_TEAM_DOOR_ALLOW_DESC,
                            text_input(labels::SETTINGS_TEAM_DOOR_ALLOW_HINT, &team.door_allow)
                                .on_input(Message::TeamDoorAllowChanged)
                                .style(input_style)
                                .width(240)
                                .into(),
                        ))
                        .push(setting_row_desc(
                            labels::SETTINGS_TEAM_DOOR_ASK,
                            labels::SETTINGS_TEAM_DOOR_ASK_DESC,
                            switch(team.door_ask, Message::TeamDoorAskChanged(!team.door_ask)),
                        ));
                }
                rows = rows.push(setting_row_desc(
                    labels::SETTINGS_TEAM_NETWORK,
                    labels::SETTINGS_TEAM_NETWORK_DESC,
                    text_input(labels::SETTINGS_TEAM_NETWORK_HINT, &team.network)
                        .on_input(Message::TeamNetworkChanged)
                        .style(input_style)
                        .width(240)
                        .into(),
                ));
            }
            rows
        }
        Section::LocalSend => column![super::localsend::view(&s.localsend)],
        Section::Automations => {
            let ws_placeholder = s
                .claude
                .workspace
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_else(|| labels::SETTINGS_ROUTINE_WORKSPACE_HINT.to_owned());
            let mut col = column![
                text(labels::SETTINGS_AUTOMATIONS)
                    .font(theme::FONT_TITLE)
                    .size(theme::SIZE_TITLE)
                    .color(theme::palette().text),
                setting_row(
                    labels::SETTINGS_AUTOMATION_HARNESS,
                    pick_list(
                        &[
                            labels::SETTINGS_AUTOMATION_DISABLED,
                            "Antigravity",
                            "Claude Code",
                            "Codex",
                            "Cursor",
                            "GitHub Copilot",
                            "OpenCode",
                            "Pi",
                        ][..],
                        Some(harness_label(s.current.automation.harness_id)),
                        |value| Message::AutomationHarnessChanged(value.to_owned()),
                    )
                    .into(),
                ),
                setting_row(
                    labels::SETTINGS_AUTOMATION_MODEL,
                    text_input(
                        "Padrão do harness",
                        s.current.automation.model_id.as_deref().unwrap_or("")
                    )
                    .on_input(Message::AutomationModelChanged)
                    .style(input_style)
                    .width(240)
                    .into(),
                ),
                text(labels::SETTINGS_ROUTINES_TITLE)
                    .font(theme::FONT_TITLE)
                    .size(theme::SIZE_SECTION_HEADER)
                    .color(theme::palette().text),
                text(labels::SETTINGS_ROUTINES_DESC)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text3),
                row![
                    text_input(labels::SETTINGS_ROUTINE_NAME_HINT, &s.routine_name)
                        .on_input(Message::RoutineNameChanged)
                        .style(input_style)
                        .width(180),
                    text_input(labels::SETTINGS_ROUTINE_TIME_HINT, &s.routine_time)
                        .on_input(Message::RoutineTimeChanged)
                        .style(input_style)
                        .width(110),
                    pick_list(
                        &crate::scheduled_automations::ALLOWED_ROUTINE_HARNESSES[..],
                        Some(s.routine_harness.as_str()),
                        |val| Message::RoutineHarnessChanged(val.to_owned()),
                    ),
                ]
                .spacing(8),
                WEEKDAYS
                    .into_iter()
                    .zip(s.routine_weekdays)
                    .enumerate()
                    .fold(
                        row![text("Dias")].spacing(4),
                        |days, (index, (day, selected))| {
                            days.push(choice(day, selected, Message::RoutineWeekdayToggled(index)))
                        }
                    ),
                text_input(&ws_placeholder, &s.routine_workspace)
                    .on_input(Message::RoutineWorkspaceChanged)
                    .style(input_style),
                row![
                    text_input(labels::SETTINGS_ROUTINE_PROMPT_HINT, &s.routine_prompt)
                        .on_input(Message::RoutinePromptChanged)
                        .on_submit(Message::RoutineAdd)
                        .style(input_style),
                    button(text(labels::SETTINGS_ROUTINE_ADD).size(theme::SIZE_LABEL))
                        .padding([7, 12])
                        .on_press(Message::RoutineAdd)
                        .style(theme::ghost_button),
                ]
                .spacing(8),
            ]
            .spacing(10);
            if let Some(err) = &s.routine_error {
                col = col.push(
                    text(err)
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().error),
                );
            }
            if s.routines.routines().is_empty() {
                col = col.push(
                    text(labels::SETTINGS_ROUTINES_EMPTY)
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().text3),
                );
            } else {
                for routine in s.routines.routines() {
                    let last = routine
                        .last_run_date
                        .as_deref()
                        .unwrap_or("ainda não rodou");
                    let summary = format!(
                        "[{} · {}] {} · {} · {} (último disparo agendado: {last})\n{}",
                        routine.time_hhmm,
                        WEEKDAYS
                            .into_iter()
                            .zip(routine.weekdays)
                            .filter_map(|(day, selected)| selected.then_some(day))
                            .collect::<Vec<_>>()
                            .join(", "),
                        routine.name,
                        routine.harness,
                        routine.workspace,
                        routine.prompt
                    );
                    let id_toggle = routine.id.clone();
                    let id_run = routine.id.clone();
                    let id_rm = routine.id.clone();
                    col = col.push(
                        row![
                            text(summary)
                                .font(theme::FONT_BODY)
                                .size(theme::SIZE_LABEL)
                                .color(theme::palette().text)
                                .width(iced::Fill),
                            switch(
                                routine.enabled,
                                Message::RoutineToggle(id_toggle, !routine.enabled),
                            ),
                            button(text(labels::SETTINGS_ROUTINE_RUN_NOW).size(theme::SIZE_BADGE))
                                .padding([4, 8])
                                .on_press_maybe(
                                    (!s.routines.is_running(&id_run))
                                        .then_some(Message::RoutineRunNow(id_run))
                                )
                                .style(theme::ghost_button),
                            button(text(labels::SETTINGS_ROUTINE_REMOVE).size(theme::SIZE_BADGE))
                                .padding([4, 8])
                                .on_press(Message::RoutineRemove(id_rm))
                                .style(theme::ghost_button),
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                    );
                }
            }
            if !s.routines.runs().is_empty() {
                col = col.push(text("Histórico de execuções").font(theme::FONT_BODY_SEMIBOLD));
                for run in s.routines.runs().iter().rev() {
                    col = col.push(
                        row![
                            text(run.summary()).size(theme::SIZE_LABEL).width(Fill),
                            button(text("Abrir log").size(theme::SIZE_BADGE))
                                .padding([4, 8])
                                .on_press_maybe(
                                    (run.status != RunStatus::Running)
                                        .then(|| Message::RoutineOpenLog(run.log()))
                                )
                                .style(theme::ghost_button),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    );
                }
            }
            col
        }
        Section::LanguageServers => column![
            text(labels::SETTINGS_LANGUAGE_SERVERS)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            text(labels::SETTINGS_LSP_HELP).color(theme::palette().text2),
            setting_row(
                labels::SETTINGS_LSP_COMMANDS,
                text_editor(&s.lsp_commands)
                    .on_action(Message::LspCommandsAction)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_MONO)
                    .height(160)
                    .style(settings_editor_style)
                    .into(),
            ),
            setting_row(
                labels::SETTINGS_LSP_FORMATTERS,
                text_editor(&s.lsp_formatters)
                    .on_action(Message::LspFormattersAction)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_MONO)
                    .height(160)
                    .style(settings_editor_style)
                    .into(),
            ),
        ],
        Section::Features => {
            let features = [
                (AppFeature::Chat, labels::SETTINGS_FEATURE_CHAT),
                (AppFeature::Farm, labels::SETTINGS_FEATURE_FARM),
                (AppFeature::Harness, labels::SETTINGS_FEATURE_HARNESS),
                (AppFeature::AgentGraph, labels::SETTINGS_FEATURE_GRAPH),
                (AppFeature::Browser, labels::SETTINGS_FEATURE_BROWSER),
                (AppFeature::AgentIsland, labels::SETTINGS_FEATURE_ISLAND),
            ];
            let mut rows = column![text(labels::SETTINGS_FEATURES)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text)];
            for (feature, label) in features {
                rows = rows.push(setting_row(
                    label,
                    switch(
                        s.current.shows_feature(feature),
                        Message::FeatureVisibilityChanged(
                            feature,
                            !s.current.shows_feature(feature),
                        ),
                    ),
                ));
            }
            rows
        }
    }
    .spacing(theme::SETTINGS_SECTION_TITLE_GAP)
    .width(Fill);
    let mut content = if s.nav_claude {
        column![claude::view::panel(s).map(Message::Claude)].width(Fill)
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
            Error::InvalidLocalSendLimit => labels::SETTINGS_INVALID_LOCALSEND_LIMIT,
            Error::InvalidLspSettings => labels::SETTINGS_LSP_INVALID,
            Error::InvalidAutomation => labels::SETTINGS_AUTOMATION_INVALID,
            Error::InvalidSection | Error::Config(_) | Error::Io(_) | Error::Document(_) => {
                labels::SETTINGS_CONFIG_ERROR
            }
        };
        content = content.push(text(message).color(theme::palette().warn));
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
        iced::widget::scrollable(panel).width(Fill).height(Fill)
    ]
    .height(Fill);
    let page = container(column![
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
    .into();
    claude::view::overlay(&s.claude, page)
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
        (
            labels::ICON_APPEARANCE,
            labels::SETTINGS_APPEARANCE,
            Section::Appearance,
        ),
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
        (
            labels::ICON_NOTIFICATIONS,
            labels::SETTINGS_NOTIFICATIONS,
            Section::Notifications,
        ),
        (labels::ICON_FARM, labels::SETTINGS_TEAM, Section::Team),
        (
            labels::ICON_FILE_ITEM,
            labels::LOCALSEND_TITLE,
            Section::LocalSend,
        ),
        (
            labels::ICON_BOLT,
            labels::SETTINGS_AUTOMATIONS,
            Section::Automations,
        ),
        (
            labels::ICON_LANGUAGE,
            labels::SETTINGS_LANGUAGE_SERVERS,
            Section::LanguageServers,
        ),
        (
            labels::ICON_WIDGETS,
            labels::SETTINGS_FEATURES,
            Section::Features,
        ),
    ];
    let mut nav_list = column![];
    let mut found = false;
    if s.nav_claude {
        let tab_groups: [(&str, &[claude::Tab]); 3] = [
            (labels::SETTINGS_CLAUDE_GROUP, &claude::Tab::GROUP_SETTINGS),
            (
                labels::SETTINGS_CLAUDE_GROUP_COMPUTER,
                &claude::Tab::GROUP_COMPUTER,
            ),
            (
                labels::SETTINGS_CLAUDE_GROUP_PERSONALIZATION,
                &claude::Tab::GROUP_PERSONALIZATION,
            ),
        ];
        for (title, tabs) in tab_groups {
            let shown: Vec<claude::Tab> = tabs
                .iter()
                .copied()
                .filter(|t| matches_search(t.label(), &s.nav_search))
                .collect();
            if shown.is_empty() {
                continue;
            }
            nav_list = nav_list.push(group_heading(title, !found));
            found = true;
            let mut group_items = column![].spacing(theme::SETTINGS_NAV_ITEM_SPACING);
            for tab in shown {
                group_items = group_items.push(nav_entry(
                    tab.icon(),
                    tab.label(),
                    tab == s.claude.tab,
                    false,
                    Message::Claude(claude::Message::Select(tab)),
                ));
            }
            nav_list = nav_list.push(group_items);
        }
        if matches_search(labels::SETTINGS_NAV_API_KEYS, &s.nav_search) {
            nav_list = nav_list.push(group_heading(
                labels::SETTINGS_CLAUDE_GROUP_PLATFORM,
                !found,
            ));
            found = true;
            nav_list = nav_list.push(nav_entry(
                labels::ICON_VPN_KEY,
                labels::SETTINGS_NAV_API_KEYS,
                false,
                true,
                Message::Claude(claude::Message::OpenUrl(String::from(labels::API_KEYS_URL))),
            ));
        }
        if !found {
            nav_list = nav_list.push(group_heading(labels::SETTINGS_NAV_NO_RESULTS, true));
        }
    } else {
        let mut items = column![].spacing(theme::SETTINGS_NAV_ITEM_SPACING);
        for (icon, label, section) in entries {
            if (section != Section::Team || s.current.shows_feature(AppFeature::Farm))
                && matches_search(label, &s.nav_search)
            {
                found = true;
                items = items.push(nav_item(icon, label, section, s.section));
            }
        }
        let group = if found {
            labels::SETTINGS_GSH_DESKTOP
        } else {
            labels::SETTINGS_NAV_NO_RESULTS
        };
        nav_list = nav_list.push(group_heading(group, true)).push(items);
    }
    let search = text_input(labels::SETTINGS_NAV_SEARCH, &s.nav_search)
        .icon(text_input::Icon {
            font: theme::FONT_ICON,
            code_point: labels::ICON_SEARCH,
            size: Some(iced::Pixels(theme::SETTINGS_NAV_ICON_SIZE)),
            spacing: theme::SETTINGS_NAV_SEARCH_ICON_GAP,
            side: text_input::Side::Left,
        })
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
        iced::widget::scrollable(nav_list.padding([0, theme::SETTINGS_NAV_PADDING])).height(Fill);
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

fn harness_label(harness: Option<HarnessKind>) -> &'static str {
    match harness {
        None => labels::SETTINGS_AUTOMATION_DISABLED,
        Some(HarnessKind::Antigravity) => "Antigravity",
        Some(HarnessKind::ClaudeCode) => "Claude Code",
        Some(HarnessKind::Codex) => "Codex",
        Some(HarnessKind::Cursor) => "Cursor",
        Some(HarnessKind::GitHubCopilot) => "GitHub Copilot",
        Some(HarnessKind::OpenCode) => "OpenCode",
        Some(HarnessKind::Pi) => "Pi",
    }
}

fn settings_editor_style(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    text_editor::Style {
        background: theme::palette().panel.into(),
        border: Border::default(),
        placeholder: theme::palette().text4,
        value: theme::palette().text,
        selection: theme::TERM_SELECTION_BG,
    }
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

fn group_heading(title: &'static str, first: bool) -> Element<'static, Message> {
    container(
        text(title)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_NAV_GROUP_SIZE)
            .color(theme::palette().text3),
    )
    .padding(iced::Padding {
        top: if first {
            theme::SETTINGS_NAV_GROUP_TOP
        } else {
            theme::SETTINGS_NAV_GROUP_NEXT_TOP
        },
        right: f32::from(theme::SETTINGS_NAV_ITEM_HORIZONTAL),
        bottom: theme::SETTINGS_NAV_GROUP_BOTTOM,
        left: f32::from(theme::SETTINGS_NAV_ITEM_HORIZONTAL),
    })
    .into()
}

fn nav_item(
    icon: &'static str,
    label: &'static str,
    section: Section,
    current: Section,
) -> Element<'static, Message> {
    nav_entry(
        icon,
        label,
        section == current,
        false,
        Message::Select(section),
    )
}

fn nav_entry(
    icon: &'static str,
    label: &'static str,
    selected: bool,
    external: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let mut line = row![
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
            .width(Fill),
    ]
    .spacing(theme::SETTINGS_NAV_ITEM_GAP)
    .align_y(Alignment::Center);
    if external {
        line = line.push(
            text(labels::ICON_NORTH_EAST)
                .font(theme::FONT_ICON)
                .size(15.0)
                .color(theme::palette().text3),
        );
    }
    button(line)
        .width(Fill)
        .padding([
            theme::SETTINGS_NAV_ITEM_VERTICAL,
            theme::SETTINGS_NAV_ITEM_HORIZONTAL,
        ])
        .on_press(on_press)
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

pub(super) fn input_style(_: &Theme, status: text_input::Status) -> text_input::Style {
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

pub(super) fn locale_style(_: &Theme, _: pick_list::Status) -> pick_list::Style {
    let p = theme::palette();
    pick_list::Style {
        text_color: p.text,
        placeholder_color: p.text3,
        handle_color: p.text3,
        background: p.panel2.into(),
        border: Border {
            color: p.border2,
            width: 1.0,
            radius: theme::SETTINGS_DROPDOWN_RADIUS.into(),
        },
    }
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
        .width(Fill)
        .align_y(Alignment::Center)
        .padding([theme::SETTINGS_ROW_VERTICAL, 0]),
        crate::panel_widgets::hairline_h()
    ]
    .width(Fill)
    .into()
}

fn setting_row_desc<'a>(
    label: &'static str,
    description: &'static str,
    control: Element<'a, Message>,
) -> Element<'a, Message> {
    column![
        row![
            column![
                text(label)
                    .font(theme::FONT_BODY)
                    .size(theme::SETTINGS_NAV_TEXT_SIZE)
                    .color(theme::palette().text),
                text(description).size(12).color(theme::palette().text3),
            ]
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
    use crate::labels;
    use crate::settings::{claude, Message, Settings};
    use crate::theme;
    use iced::{Element, Point, Size};
    use iced_test::simulator::{click, Simulator};

    fn simulator<'a, M>(size: Size, element: impl Into<Element<'a, M>>) -> Simulator<'a, M> {
        std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
        let settings = iced::Settings {
            fonts: theme::EMBEDDED_FONTS
                .iter()
                .map(|font| (*font).into())
                .collect(),
            default_font: theme::FONT_BODY,
            ..Default::default()
        };
        Simulator::with_size(settings, size, element)
    }

    #[test]
    fn filter_matches_case_insensitively_and_trims_query() {
        assert!(matches_search("Aparência", "  ARÊN  "));
        assert!(matches_search("Terminal", "  "));
        assert!(!matches_search("Terminal", "editor"));
        for tab in claude::Tab::ALL {
            assert!(matches_search(tab.label(), &tab.label().to_uppercase()));
        }
    }

    #[test]
    fn claude_tabs_and_overlays_build_without_error() {
        let mut s = Settings::default();
        s.claude.home = std::path::PathBuf::new();
        let _ = s.update(Message::SideChanged(true));
        for tab in claude::Tab::ALL {
            let _ = s.update(Message::Claude(claude::Message::Select(tab)));
            let _ = super::view(&s);
        }
        let _ = s.update(Message::Claude(claude::Message::LogoutAsk));
        s.claude.show_no_workspace();
        let _ = super::view(&s);
    }

    #[test]
    fn general_tab_renders_and_clicks_switches() {
        let mut s = Settings::default();
        s.claude.home = std::path::PathBuf::new();
        let _ = s.update(Message::SideChanged(true));
        let _ = s.update(Message::Claude(claude::Message::Select(
            claude::Tab::General,
        )));

        let mut ui = simulator(Size::new(1100.0, 900.0), super::view(&s));
        let click_row_switch =
            |ui: &mut Simulator<'_, Message>, title: &'static str, desc: &'static str| {
                let title_bounds = ui.find(title).unwrap().visible_bounds().unwrap();
                let desc_bounds = ui.find(desc).unwrap().visible_bounds().unwrap();
                let switch_x = title_bounds.x
                    + title_bounds.width
                    + theme::SETTINGS_ROW_GAP
                    + theme::SETTINGS_SWITCH_WIDTH * 0.5;
                let switch_y = (title_bounds.y + desc_bounds.y + desc_bounds.height) * 0.5;
                ui.point_at(Point::new(switch_x, switch_y));
                let _ = ui.simulate(click());
            };
        click_row_switch(
            &mut ui,
            labels::SETTINGS_GC_REDUCED_MOTION,
            labels::SETTINGS_GC_REDUCED_MOTION_DESC,
        );
        click_row_switch(
            &mut ui,
            labels::SETTINGS_GC_NOTIFY_TURN_DONE,
            labels::SETTINGS_GC_NOTIFY_TURN_DONE_DESC,
        );
        click_row_switch(
            &mut ui,
            labels::SETTINGS_GC_PUSH_INPUT_NEEDED,
            labels::SETTINGS_GC_PUSH_INPUT_NEEDED_DESC,
        );
        let messages: Vec<Message> = ui.into_messages().collect();
        assert!(
            messages.iter().any(|m| matches!(
                m,
                Message::Claude(claude::Message::Switch(claude::Switch::ReducedMotion, true))
            )),
            "esperava clique no switch de Movimento reduzido, veio: {messages:?}"
        );
        assert!(
            messages.iter().any(|m| matches!(
                m,
                Message::Claude(claude::Message::Pref(claude::Pref::Notifications(_)))
            )),
            "esperava clique no switch de notificações de conclusão, veio: {messages:?}"
        );
        assert!(
            messages.iter().any(|m| matches!(
                m,
                Message::Claude(claude::Message::Switch(
                    claude::Switch::PushInputNeeded,
                    true
                ))
            )),
            "esperava clique no switch de push no celular, veio: {messages:?}"
        );
    }

    #[test]
    fn catalog_headers_keep_add_button_visible_and_clickable_in_900px_pane() {
        for tab in [
            claude::Tab::Connectors,
            claude::Tab::Skills,
            claude::Tab::Plugins,
        ] {
            let mut s = Settings::default();
            s.claude.home = std::path::PathBuf::new();
            let _ = s.update(Message::SideChanged(true));
            let _ = s.update(Message::Claude(claude::Message::Select(tab)));

            let mut ui = simulator(Size::new(900.0, 600.0), super::view(&s));
            let add_btn = ui.find(labels::SETTINGS_CAT_ADD).unwrap();
            let bounds = add_btn.visible_bounds().unwrap();
            assert!(
                bounds.x + bounds.width <= 900.0,
                "{tab:?}: botão '+ Adicionar' estourou 900px: {bounds:?}"
            );
            let _ = ui.click(labels::SETTINGS_CAT_ADD).unwrap();
            let messages: Vec<Message> = ui.into_messages().collect();
            assert!(
                !messages.is_empty(),
                "{tab:?}: clique em '+ Adicionar' não emitiu mensagem"
            );
        }
    }

    #[test]
    fn usage_tab_renders_logged_out_note_and_subscription_bars() {
        let mut s = Settings::default();
        s.claude.home = std::path::PathBuf::new();
        let _ = s.update(Message::SideChanged(true));
        let _ = s.update(Message::Claude(claude::Message::Select(claude::Tab::Usage)));

        // Estado não-logado / sem limites de plano (`rate_limits_available: false`):
        let _ = s.update(Message::Claude(claude::Message::UsageLoaded(
            0,
            Ok(claude::cli::Usage {
                available: false,
                subscription_type: None,
                windows: Vec::new(),
                extra: None,
            }),
        )));
        {
            let mut ui = simulator(Size::new(900.0, 600.0), super::view(&s));
            assert!(ui.find(labels::SETTINGS_USAGE_UNAVAILABLE).is_ok());
        }

        // Estado logado por assinatura (`rate_limits_available: true`):
        let _ = s.update(Message::Claude(claude::Message::UsageLoaded(
            0,
            Ok(claude::cli::Usage {
                available: true,
                subscription_type: Some(String::from("team")),
                windows: vec![
                    claude::cli::UsageWindow {
                        kind: claude::cli::UsageWindowKind::Session,
                        label: None,
                        percent: Some(4.0),
                        resets_at: Some(2_000_000_000),
                    },
                    claude::cli::UsageWindow {
                        kind: claude::cli::UsageWindowKind::Weekly,
                        label: None,
                        percent: Some(63.0),
                        resets_at: Some(2_000_100_000),
                    },
                    claude::cli::UsageWindow {
                        kind: claude::cli::UsageWindowKind::WeeklyModel,
                        label: Some(String::from("Fable")),
                        percent: Some(37.0),
                        resets_at: Some(2_000_100_000),
                    },
                ],
                extra: None,
            }),
        )));
        {
            let mut ui = simulator(Size::new(900.0, 600.0), super::view(&s));
            assert!(ui.find(labels::SETTINGS_USAGE_PLAN_TEAM).is_ok());
            assert!(ui.find(labels::SETTINGS_USAGE_WINDOW_SESSION).is_ok());
            assert!(ui.find(labels::SETTINGS_USAGE_WINDOW_WEEKLY).is_ok());
            assert!(ui.find("Fable").is_ok());
            assert!(ui.find("4% usado").is_ok());
            assert!(ui.find("63% usado").is_ok());
            assert!(ui.find("37% usado").is_ok());
        }
    }
}
