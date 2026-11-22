//! Painéis das abas secundárias do grupo Claude: Uso, Recursos, Memória, Design Systems,
//! Cowork, Sistema, Extensões e Desenvolvedor.

use super::cli::{self, McpServerInfo, McpStatus, Usage, UsageWindow, UsageWindowKind};
use super::files::MemoryScope;
use super::themes;
use super::widgets::{
    catalog_matches, chip, field_style, mcp_status_chip, note, refresh_button, section,
    section_with_desc, setting_row, small_button, value, Emphasis,
};
use super::{Message, Pref, Switch, Tab};
use crate::panel_widgets;
use crate::settings::view::switch;
use crate::settings::Settings;
use crate::{labels, theme};
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

type El<'a> = Element<'a, Message>;

/// Aba **Uso** (`usage_panel.dart`).
pub fn usage(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let p = theme::palette();
    let now = cli::now_epoch_secs();
    let account_email = match &c.account {
        Some(Ok(acc)) if acc.logged_in => acc.email.as_deref(),
        _ => None,
    };
    let plan_label = match &c.usage {
        Some(Ok(u)) => u.subscription_type.as_deref().map(labels::usage_plan_label),
        _ => None,
    };
    let mut header_trailing = row![].spacing(8.0).align_y(Alignment::Center);
    if let Some(email) = account_email {
        header_trailing = header_trailing.push(
            text(email)
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_DESC_SIZE)
                .color(p.text2),
        );
    }
    if let Some(plan) = plan_label {
        header_trailing = header_trailing.push(
            text(plan)
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_DESC_SIZE)
                .color(p.text3),
        );
    }
    header_trailing = header_trailing.push(small_button(
        labels::SETTINGS_ACCOUNT_SWITCH,
        None,
        Emphasis::Outline,
        Some(Message::SwitchAccountAsk),
    ));
    let plan_badge: Option<El<'_>> = Some(header_trailing.into());
    let session_rows: Vec<El<'_>> = match &c.usage {
        Some(Err(err)) => vec![note(labels::claude_info_error(err), true)],
        None => vec![note(labels::SETTINGS_CLAUDE_LOADING.to_owned(), false)],
        Some(Ok(u)) if !u.available => {
            vec![note(labels::SETTINGS_USAGE_UNAVAILABLE.to_owned(), false)]
        }
        Some(Ok(u)) => {
            let rows: Vec<El<'_>> = u
                .windows
                .iter()
                .filter(|w| w.kind == UsageWindowKind::Session)
                .map(|w| usage_window_row(w, now))
                .collect();
            if rows.is_empty() {
                vec![note(String::from("—"), false)]
            } else {
                rows
            }
        }
    };
    let mut page = column![section(
        labels::SETTINGS_USAGE_TITLE,
        plan_badge,
        session_rows
    )];
    if let Some(Ok(Usage {
        available: true,
        windows,
        ..
    })) = &c.usage
    {
        let weekly: Vec<El<'_>> = windows
            .iter()
            .filter(|w| w.kind != UsageWindowKind::Session)
            .map(|w| usage_window_row(w, now))
            .collect();
        if !weekly.is_empty() {
            page = page.push(section(labels::SETTINGS_USAGE_WEEKLY_TITLE, None, weekly));
        }
    }
    if let Some(at) = c.usage_at {
        let mins = ((now - at) / 60).max(0);
        let when = if mins < 1 {
            labels::SETTINGS_USAGE_JUST_NOW.to_owned()
        } else {
            labels::usage_minutes_ago(mins)
        };
        page = page.push(
            row![
                text(labels::usage_updated(&when))
                    .font(theme::FONT_BODY)
                    .size(13.0)
                    .color(p.text3),
                Space::new().width(8.0),
                refresh_button(c.usage_loading, Message::RefreshUsage),
            ]
            .align_y(Alignment::Center),
        );
    }
    page.width(Fill).into()
}

fn usage_window_row(w: &UsageWindow, now_secs: i64) -> El<'static> {
    let p = theme::palette();
    let title = match w.kind {
        UsageWindowKind::Session => labels::SETTINGS_USAGE_WINDOW_SESSION.to_owned(),
        UsageWindowKind::Weekly => labels::SETTINGS_USAGE_WINDOW_WEEKLY.to_owned(),
        UsageWindowKind::WeeklyModel => w.label.clone().unwrap_or_else(|| String::from("—")),
    };
    let mut left = column![text(title)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_NAV_TEXT_SIZE)
        .color(p.text)];
    if let Some(resets_at) = w.resets_at {
        let sub = if w.kind == UsageWindowKind::Session {
            labels::usage_resets_in(&cli::duration_until(resets_at, now_secs))
        } else {
            labels::usage_resets_at(&cli::weekday_time(resets_at))
        };
        left = left.push(text(sub).font(theme::FONT_BODY).size(13.0).color(p.text3));
    }
    let pct = w.percent;
    let fill_color = match pct {
        None => p.text4,
        Some(v) if v >= 100.0 => p.error,
        Some(v) if v >= 80.0 => p.warn,
        Some(_) => p.accent,
    };
    let pct_clamped = pct.unwrap_or(0.0).clamp(0.0, 100.0) as f32;
    let bar = iced::widget::progress_bar(0.0..=100.0, pct_clamped)
        .length(Length::FillPortion(5))
        .girth(theme::SETTINGS_USAGE_BAR_HEIGHT)
        .style(move |_| iced::widget::progress_bar::Style {
            background: Background::Color(p.panel3),
            bar: Background::Color(fill_color),
            border: Border {
                radius: theme::SETTINGS_USAGE_BAR_RADIUS.into(),
                ..Default::default()
            },
        });
    let pct_label = pct.map_or_else(
        || String::from("—"),
        |v| labels::usage_percent_used(v.round() as i64),
    );
    row![
        left.width(Length::FillPortion(4)),
        bar,
        container(
            text(pct_label)
                .font(theme::FONT_BODY)
                .size(13.0)
                .color(p.text2),
        )
        .width(Length::Fixed(theme::SETTINGS_USAGE_PCT_WIDTH))
        .align_x(iced::alignment::Horizontal::Right),
    ]
    .width(Fill)
    .align_y(Alignment::Center)
    .padding([theme::SETTINGS_ROW_VERTICAL, 0])
    .into()
}

/// Aba **Recursos** (`resources_panel.dart`).
pub fn resources(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let p = theme::palette();
    let on = Switch::SwitchModels.is_on(&c.snapshot);
    let link = button(
        text(labels::SETTINGS_CLAUDE_GROUP_PERSONALIZATION)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DESC_SIZE)
            .color(p.accent_text),
    )
    .padding(0)
    .on_press(Message::Select(Tab::Skills))
    .style(|_: &Theme, _| button::Style {
        background: None,
        text_color: theme::palette().accent_text,
        ..Default::default()
    });
    column![
        section(
            labels::SETTINGS_CC_SECTION_GENERAL,
            None,
            vec![setting_row(
                labels::SETTINGS_CC_SWITCH_MODELS_TITLE,
                Some(labels::SETTINGS_CC_SWITCH_MODELS_DESC),
                Some(switch(on, Message::Switch(Switch::SwitchModels, !on))),
                None,
            )],
        ),
        section(
            labels::SETTINGS_NAV_SKILLS,
            None,
            vec![container(
                row![
                    text(labels::SETTINGS_RESOURCES_SKILLS_MOVED)
                        .font(theme::FONT_BODY)
                        .size(theme::SETTINGS_DESC_SIZE)
                        .color(p.text2),
                    link,
                ]
                .align_y(Alignment::Center),
            )
            .padding([8, 0])
            .into()],
        ),
    ]
    .width(Fill)
    .into()
}

/// Aba **Memória** (`memory_panel.dart`).
pub fn memory(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let home_str = c.home.to_string_lossy();
    let short_path = |path: &std::path::Path| -> String {
        let raw = path.to_string_lossy();
        if !home_str.is_empty() && raw.starts_with(home_str.as_ref()) {
            format!("~{}", raw.get(home_str.len()..).unwrap_or(""))
        } else {
            raw.into_owned()
        }
    };
    let instr_rows: Vec<El<'_>> = if c.memory.is_empty() {
        vec![note(labels::SETTINGS_MEMORY_EMPTY.to_owned(), false)]
    } else {
        c.memory
            .iter()
            .map(|f| {
                let scope_label = match f.scope {
                    MemoryScope::User => labels::SETTINGS_MEMORY_SCOPE_USER,
                    MemoryScope::Project => labels::SETTINGS_MEMORY_SCOPE_PROJECT,
                };
                memory_row(
                    short_path(&f.path),
                    Some(scope_label.to_owned()),
                    None,
                    f.path.clone(),
                )
            })
            .collect()
    };
    let auto_rows: Vec<El<'_>> = if c.auto_memory.is_empty() {
        vec![note(labels::SETTINGS_MEMORY_AUTO_EMPTY.to_owned(), false)]
    } else {
        c.auto_memory
            .iter()
            .map(|m| {
                let date = m
                    .updated_at
                    .map(|ts| labels::memory_updated(&cli::short_date(ts)));
                memory_row(m.title.clone(), m.hook.clone(), date, m.path.clone())
            })
            .collect()
    };
    column![
        section_with_desc(
            labels::SETTINGS_MEMORY_INSTRUCTIONS_TITLE,
            Some(labels::SETTINGS_MEMORY_INTRO),
            None,
            true,
            instr_rows,
        ),
        section(labels::SETTINGS_MEMORY_AUTO_TITLE, None, auto_rows),
    ]
    .width(Fill)
    .into()
}

fn memory_row(
    title: String,
    desc: Option<String>,
    date: Option<String>,
    path: std::path::PathBuf,
) -> El<'static> {
    let p = theme::palette();
    let mut line = row![
        container(
            text(title)
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_DESC_SIZE)
                .color(p.text),
        )
        .width(Length::Fixed(theme::SETTINGS_MEMORY_TITLE_WIDTH)),
        Space::new().width(12.0),
        text(desc.unwrap_or_default())
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DESC_SIZE)
            .color(p.text2)
            .width(Fill),
    ]
    .align_y(Alignment::Center);
    if let Some(date) = date {
        line = line.push(
            container(
                text(date)
                    .font(theme::FONT_BODY)
                    .size(theme::SETTINGS_DESC_SIZE)
                    .color(p.text3),
            )
            .width(Length::Fixed(theme::SETTINGS_MEMORY_DATE_WIDTH)),
        );
    }
    button(line)
        .width(Fill)
        .padding([11, 6])
        .on_press(Message::OpenFile(path))
        .style(|_: &Theme, status| {
            let p = theme::palette();
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: hovered.then_some(Background::Color(p.panel3)),
                text_color: p.text,
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

/// Aba **Design systems** (`design_systems_panel.dart`).
pub fn design_systems(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let p = theme::palette();
    let all = &c.themes;
    let shown: Vec<&themes::ThemeEntry> = all
        .iter()
        .filter(|t| catalog_matches(&c.ds_search, &[Some(&t.name), t.author.as_deref()]))
        .collect();

    let header = row![
        column![
            text(labels::SETTINGS_DS_TITLE)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SETTINGS_SECTION_TITLE_SIZE)
                .color(p.text),
            Space::new().height(4.0),
            text(labels::SETTINGS_DS_DESC)
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_DESC_SIZE)
                .color(p.text3),
        ]
        .width(Fill),
        Space::new().width(24.0),
        small_button(
            labels::SETTINGS_DS_ADD,
            Some(labels::ICON_PLUS),
            Emphasis::Outline,
            Some(Message::ImportThemePick),
        ),
    ]
    .align_y(Alignment::Start);

    let search_bar = row![
        text_input(labels::SETTINGS_DS_SEARCH, &c.ds_search)
            .on_input(Message::DsSearch)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DROPDOWN_TEXT)
            .padding([6, 10])
            .width(Length::Fixed(theme::SETTINGS_DS_SEARCH_WIDTH))
            .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS)),
        Space::new().width(Fill),
        text(labels::ds_showing(shown.len(), all.len()))
            .font(theme::FONT_BODY)
            .size(13.0)
            .color(p.text2),
    ]
    .align_y(Alignment::Center);

    let mut table = column![ds_table_line(
        text(labels::SETTINGS_DS_COL_NAME)
            .font(theme::FONT_BODY)
            .size(13.0)
            .color(p.text2)
            .into(),
        text(labels::SETTINGS_DS_COL_ORIGIN)
            .font(theme::FONT_BODY)
            .size(13.0)
            .color(p.text2)
            .into(),
        text(labels::SETTINGS_DS_COL_AUTHOR)
            .font(theme::FONT_BODY)
            .size(13.0)
            .color(p.text2)
            .into(),
        Space::new().into(),
    )];

    for t in shown {
        let active = s.current.theme_id == t.id;
        let mut name_cell = row![text(&t.name)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_NAV_TEXT_SIZE)
            .color(p.text)]
        .align_y(Alignment::Center);
        if active {
            name_cell = name_cell
                .push(Space::new().width(8.0))
                .push(chip(labels::SETTINGS_DS_ACTIVE.to_owned()));
        }
        let origin_label = if t.imported {
            labels::SETTINGS_DS_IMPORTED
        } else {
            labels::SETTINGS_DS_BUILT_IN
        };
        let mut actions = row![small_button(
            labels::SETTINGS_DS_ACTIVATE,
            Some(labels::ICON_CHECK),
            Emphasis::Outline,
            (!active).then(|| Message::Pref(Pref::ThemeId(t.id.clone()))),
        )]
        .spacing(6.0)
        .align_y(Alignment::Center);
        if t.imported {
            actions = actions.push(small_button(
                labels::SETTINGS_DS_REMOVE,
                Some(labels::ICON_DELETE_OUTLINE),
                Emphasis::Outline,
                Some(Message::DeleteTheme(t.id.clone())),
            ));
        }
        table = table.push(ds_table_line(
            name_cell.into(),
            chip(origin_label.to_owned()),
            text(t.author.clone().unwrap_or_else(|| String::from("—")))
                .font(theme::FONT_BODY)
                .size(13.0)
                .color(p.text2)
                .into(),
            actions.into(),
        ));
    }

    column![
        header,
        Space::new().height(20.0),
        search_bar,
        Space::new().height(18.0),
        table,
    ]
    .width(Fill)
    .into()
}

fn ds_table_line<'a>(name: El<'a>, origin: El<'a>, author: El<'a>, actions: El<'a>) -> El<'a> {
    column![
        row![
            container(name).width(Length::FillPortion(4)),
            container(origin).width(Length::FillPortion(2)),
            container(author).width(Length::FillPortion(3)),
            container(actions)
                .width(Length::FillPortion(3))
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .align_y(Alignment::Center)
        .padding([9, 0]),
        panel_widgets::hairline_h(),
    ]
    .into()
}

/// Aba **Cowork** (`placeholder_panel.dart`).
pub fn cowork() -> El<'static> {
    column![section(
        labels::SETTINGS_NAV_COWORK,
        None,
        vec![
            note(labels::SETTINGS_PH_COWORK_1.to_owned(), false),
            note(labels::SETTINGS_PH_COWORK_2.to_owned(), false),
        ],
    )]
    .width(Fill)
    .into()
}

/// Aba **Extensões** (`placeholder_panel.dart` + varredura segura do Claude Desktop `1dfnhq1n9fd`).
pub fn extensions(s: &Settings) -> El<'_> {
    let p = theme::palette();
    let discovered = crate::claude_desktop_scanner::scan_home(&s.claude.home);
    let mut rows: Vec<El<'_>> = Vec::new();
    for ext in &discovered {
        let kind_label = match ext.kind {
            crate::claude_desktop_scanner::DiscoveredKind::McpStdio => "MCP stdio",
            crate::claude_desktop_scanner::DiscoveredKind::McpHttp => "MCP HTTP",
            crate::claude_desktop_scanner::DiscoveredKind::DesktopBundle => "Bundle (.mcpb/.dxt)",
        };
        let mut desc = format!("{kind_label} · {}", ext.command_or_url);
        if !ext.args.is_empty() {
            desc.push_str(&format!(" {}", ext.args.join(" ")));
        }
        if !ext.env_keys.is_empty() {
            desc.push_str(&format!(
                " · env: {} (valores ocultos)",
                ext.env_keys.join(", ")
            ));
        }
        let trailing: El<'_> = match &ext.verdict {
            crate::claude_desktop_scanner::SecurityVerdict::SafeToReview => {
                if let Ok(cmd) = crate::claude_desktop_scanner::approved_claude_mcp_add_command(ext)
                {
                    small_button(
                        "Importar após revisão",
                        Some(labels::ICON_PLUS),
                        Emphasis::Outline,
                        Some(Message::RunInTerminal(cmd)),
                    )
                } else {
                    chip(String::from("Somente revisão"))
                }
            }
            crate::claude_desktop_scanner::SecurityVerdict::Blocked(reason) => {
                note(format!("Bloqueado: {reason}"), true)
            }
        };
        let item_row = row![
            column![
                text(ext.name.clone())
                    .font(theme::FONT_BODY)
                    .size(theme::SETTINGS_NAV_TEXT_SIZE)
                    .color(p.text),
                Space::new().height(theme::SETTINGS_ROW_TITLE_GAP),
                text(desc)
                    .font(theme::FONT_BODY)
                    .size(theme::SETTINGS_DESC_SIZE)
                    .color(p.text3),
            ]
            .width(Fill),
            Space::new().width(theme::SETTINGS_ROW_GAP),
            trailing,
        ]
        .align_y(Alignment::Center)
        .padding([theme::SETTINGS_ROW_VERTICAL, 0]);
        rows.push(item_row.into());
    }
    let mut page = column![section_with_desc(
        labels::SETTINGS_NAV_EXTENSIONS,
        Some(labels::SETTINGS_PH_EXTENSIONS_DESC),
        None,
        true,
        rows,
    )];
    if discovered.is_empty() {
        page = page.push(
            container(
                column![
                    text(labels::ICON_EXTENSION)
                        .font(theme::FONT_ICON)
                        .size(theme::SETTINGS_PLACEHOLDER_ICON)
                        .color(p.text3),
                    Space::new().height(14.0),
                    text(labels::SETTINGS_PH_EXTENSIONS_EMPTY)
                        .font(theme::FONT_BODY)
                        .size(14.5)
                        .color(p.text2),
                    Space::new().height(8.0),
                    text(labels::SETTINGS_PH_EXTENSIONS)
                        .font(theme::FONT_BODY)
                        .size(theme::SETTINGS_DESC_SIZE)
                        .color(p.text3),
                ]
                .align_x(Alignment::Center),
            )
            .width(Fill)
            .center_x(Fill),
        );
    }
    page.width(Fill).into()
}

/// Aba **Sistema** (`system_panel.dart`).
pub fn system(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let cfg = &s.current;
    let mut sys_rows = vec![
        setting_row(
            labels::SETTINGS_SYSTEM_VERSION,
            None,
            Some(value(String::from(env!("CARGO_PKG_VERSION")), false)),
            None,
        ),
        setting_row(
            labels::SETTINGS_SYSTEM_LAUNCH_TITLE,
            Some(labels::SETTINGS_SYSTEM_LAUNCH_DESC),
            Some(switch(
                cfg.launch_at_startup,
                Message::Pref(Pref::LaunchAtStartup(!cfg.launch_at_startup)),
            )),
            None,
        ),
        setting_row(
            labels::SETTINGS_SYSTEM_KEEP_AWAKE_TITLE,
            Some(labels::SETTINGS_SYSTEM_KEEP_AWAKE_DESC),
            Some(switch(
                c.keep_awake_active,
                Message::KeepAwakeToggle(!c.keep_awake_active),
            )),
            None,
        ),
    ];

    if cfg!(target_os = "linux") {
        let mut lid_controls = row![].spacing(6.0).align_y(Alignment::Center);
        let selected_duration = c.keep_awake.lid_duration();
        if let Some(remaining_mins) = c.keep_awake.lid_remaining_minutes() {
            lid_controls = lid_controls
                .push(chip(labels::settings_lid_active_remaining(remaining_mins)))
                .push(small_button(
                    labels::SETTINGS_SYSTEM_LID_STOP,
                    None,
                    Emphasis::Outline,
                    Some(Message::LidSessionStop),
                ));
        } else {
            for duration in themes::LidSessionDuration::ALL {
                let emphasis = if duration == selected_duration {
                    Emphasis::Primary
                } else {
                    Emphasis::Outline
                };
                lid_controls = lid_controls.push(small_button(
                    duration.label(),
                    None,
                    emphasis,
                    Some(Message::LidSessionDuration(duration)),
                ));
            }
            lid_controls = lid_controls.push(small_button(
                labels::SETTINGS_SYSTEM_LID_START,
                None,
                Emphasis::Outline,
                Some(Message::LidSessionStart),
            ));
        }
        sys_rows.push(setting_row(
            labels::SETTINGS_SYSTEM_LID_SESSION_TITLE,
            Some(labels::SETTINGS_SYSTEM_LID_SESSION_DESC),
            Some(lid_controls.into()),
            None,
        ));
        if let Some(reason) = c.keep_awake.lid_stop_reason() {
            let reason_text = match reason {
                themes::LidSafetyStop::LowBattery(pct) => labels::settings_lid_stopped_battery(pct),
                themes::LidSafetyStop::HighTemp(temp_c) => {
                    labels::settings_lid_stopped_temp(temp_c)
                }
            };
            sys_rows.push(note(reason_text, true));
        }

        let bag_editable = c.bag_mode.is_editable();
        let hotspot = text_input("SSID do hotspot", &c.bag_mode.config.hotspot_ssid)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DROPDOWN_TEXT)
            .padding([6, 10])
            .width(Length::Fixed(150.0))
            .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS));
        let preferred = text_input("Wi-Fi separados por vírgula", &c.bag_mode.preferred_draft)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DROPDOWN_TEXT)
            .padding([6, 10])
            .width(Length::Fixed(170.0))
            .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS));
        let (hotspot, preferred) = if bag_editable {
            (
                hotspot.on_input(Message::BagHotspotInput),
                preferred.on_input(Message::BagPreferredInput),
            )
        } else {
            (hotspot, preferred)
        };
        let bag_controls = row![
            hotspot,
            preferred,
            small_button(
                "Aplicar",
                None,
                Emphasis::Outline,
                bag_editable.then_some(Message::BagApplyHotspot),
            ),
        ]
        .spacing(6.0)
        .align_y(Alignment::Center);
        let bag_controls = if bag_editable || c.bag_mode.config.enabled {
            bag_controls.push(switch(
                c.bag_mode.config.enabled,
                Message::BagModeToggle(!c.bag_mode.config.enabled),
            ))
        } else {
            bag_controls
        };

        sys_rows.push(setting_row(
            "Modo bolsa (hotspot + Wi-Fi)",
            Some("Durante a sessão de tampa fechada, monitora a internet e alterna entre o hotspot e as redes Wi-Fi preferidas. Ligue o hotspot no celular e mantenha o notebook ventilado."),
            Some(bag_controls.into()),
            None,
        ));
        if let Some(status) = &c.bag_mode.status {
            let text = match status {
                Ok(text) | Err(text) => text,
            };
            sys_rows.push(note(text.clone(), status.is_err()));
        }
    }

    let sys_sec = section(labels::SETTINGS_SYSTEM_TITLE, None, sys_rows);

    let root = themes::effective_root(&c.home);
    let root_str = if root.as_os_str().is_empty() {
        String::from("—")
    } else {
        root.to_string_lossy().into_owned()
    };
    let loc_desc = labels::storage_location_desc(&root_str);
    let mut loc_actions = row![].spacing(8.0).align_y(Alignment::Center);
    if themes::is_custom_storage(&c.home) {
        loc_actions = loc_actions.push(small_button(
            labels::SETTINGS_STORAGE_USE_DEFAULT,
            None,
            Emphasis::Outline,
            Some(Message::StorageUseDefault),
        ));
    }
    loc_actions = loc_actions.push(small_button(
        labels::SETTINGS_STORAGE_CHANGE,
        None,
        Emphasis::Outline,
        Some(Message::StorageChangePick),
    ));

    let p = theme::palette();
    let loc_row = row![
        column![
            text(labels::SETTINGS_STORAGE_LOCATION_TITLE)
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_NAV_TEXT_SIZE)
                .color(p.text),
            Space::new().height(theme::SETTINGS_ROW_TITLE_GAP),
            text(loc_desc)
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_DESC_SIZE)
                .color(p.text3),
        ]
        .width(Fill),
        Space::new().width(theme::SETTINGS_ROW_GAP),
        loc_actions,
    ]
    .align_y(Alignment::Center)
    .padding([theme::SETTINGS_ROW_VERTICAL, 0]);

    let storage_sec = section(
        labels::SETTINGS_STORAGE_SECTION_TITLE,
        None,
        vec![
            loc_row.into(),
            setting_row(
                labels::SETTINGS_STORAGE_RESET_TITLE,
                Some(labels::SETTINGS_STORAGE_RESET_DESC),
                Some(small_button(
                    labels::SETTINGS_STORAGE_RESET_BUTTON,
                    None,
                    Emphasis::Outline,
                    Some(Message::StorageResetAsk),
                )),
                None,
            ),
        ],
    );

    column![sys_sec, storage_sec].width(Fill).into()
}

/// Aba **Desenvolvedor** (`developer_panel.dart`).
pub fn developer(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let p = theme::palette();
    let mut top_buttons = row![].spacing(8.0).align_y(Alignment::Center);
    if !c.home.as_os_str().is_empty() {
        top_buttons = top_buttons.push(small_button(
            labels::SETTINGS_DEV_EDIT_CONFIG,
            None,
            Emphasis::Outline,
            Some(Message::OpenFile(c.home.join(".claude.json"))),
        ));
    }
    top_buttons = top_buttons.push(refresh_button(c.mcp_loading, Message::RefreshMcp));

    let mut mcp_rows: Vec<El<'_>> = vec![container(top_buttons)
        .padding(iced::Padding {
            bottom: 12.0,
            ..Default::default()
        })
        .into()];

    match &c.mcp {
        Some(Err(err)) => mcp_rows.push(note(labels::claude_info_error(err), true)),
        None => mcp_rows.push(note(labels::SETTINGS_CLAUDE_LOADING.to_owned(), false)),
        Some(Ok(servers)) => {
            let local: Vec<&McpServerInfo> =
                servers.iter().filter(|srv| !srv.is_remote()).collect();
            if local.is_empty() {
                mcp_rows.push(note(labels::SETTINGS_DEV_MCP_EMPTY.to_owned(), false));
            } else {
                mcp_rows.push(mcp_master_detail(&local, c.dev_selected_mcp.as_deref()));
            }
        }
    }

    let mut dev_rows = vec![
        setting_row(
            labels::SETTINGS_DEV_MODE_TITLE,
            Some(labels::SETTINGS_DEV_MODE_DESC),
            Some(switch(
                s.current.developer_mode,
                Message::Pref(Pref::DeveloperMode(!s.current.developer_mode)),
            )),
            None,
        ),
        setting_row(
            labels::SETTINGS_DEV_TELEMETRY_TITLE,
            Some(labels::SETTINGS_DEV_TELEMETRY_DESC),
            None,
            None,
        ),
    ];
    if !c.home.as_os_str().is_empty() {
        dev_rows.push(setting_row(
            labels::SETTINGS_DEV_CLAUDE_SETTINGS_TITLE,
            Some(labels::SETTINGS_DEV_CLAUDE_SETTINGS_DESC),
            Some(small_button(
                labels::SETTINGS_COMMON_OPEN,
                None,
                Emphasis::Outline,
                Some(Message::OpenFile(c.settings_path())),
            )),
            None,
        ));
    }

    let api_card = container(
        row![
            text(labels::ICON_CODE)
                .font(theme::FONT_ICON)
                .size(17.0)
                .color(p.text2),
            Space::new().width(10.0),
            column![
                text(labels::SETTINGS_DEV_API_KEY_TITLE)
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(theme::SETTINGS_NAV_TEXT_SIZE)
                    .color(p.text),
                Space::new().height(4.0),
                text(labels::SETTINGS_DEV_API_KEY_DESC)
                    .font(theme::FONT_BODY)
                    .size(theme::SETTINGS_DESC_SIZE)
                    .color(p.text2),
                Space::new().height(10.0),
                small_button(
                    labels::SETTINGS_DEV_API_KEY_BUTTON,
                    Some(labels::ICON_NORTH_EAST),
                    Emphasis::Outline,
                    Some(Message::OpenUrl(String::from(labels::API_KEYS_URL))),
                ),
            ]
            .width(Fill),
        ]
        .align_y(Alignment::Start),
    )
    .padding([14, 16])
    .width(Fill)
    .style(|_: &Theme| container::Style {
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    });

    column![
        section_with_desc(
            labels::SETTINGS_DEV_MCP_TITLE,
            Some(labels::SETTINGS_DEV_MCP_DESC),
            None,
            false,
            mcp_rows,
        ),
        section(labels::SETTINGS_DEV_TITLE, None, dev_rows),
        api_card,
    ]
    .width(Fill)
    .into()
}

fn mcp_master_detail<'a>(servers: &[&'a McpServerInfo], selected_name: Option<&str>) -> El<'a> {
    let p = theme::palette();
    let selected = servers
        .iter()
        .copied()
        .find(|s| Some(s.name.as_str()) == selected_name)
        .or_else(|| servers.first().copied());
    let Some(sel) = selected else {
        return Space::new().into();
    };
    let list_col = servers.iter().fold(column![].spacing(2.0), |col, srv| {
        let is_sel = srv.name == sel.name;
        col.push(
            button(
                text(srv.display_name())
                    .font(theme::FONT_BODY)
                    .size(theme::SETTINGS_DESC_SIZE)
                    .color(p.text),
            )
            .width(Fill)
            .padding([7, 8])
            .on_press(Message::SelectDevMcp(srv.name.clone()))
            .style(move |_: &Theme, status| {
                let p = theme::palette();
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: (is_sel || hovered).then_some(Background::Color(p.panel3)),
                    text_color: p.text,
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
        )
    });

    let (cmd, args) = sel.command_line();
    let field = |label: &'static str, val: &'a str| -> El<'a> {
        column![
            Space::new().height(14.0),
            text(label)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SETTINGS_DESC_SIZE)
                .color(p.text),
            Space::new().height(2.0),
            text(if val.is_empty() { "—" } else { val })
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_DESC_SIZE)
                .color(p.text3),
        ]
        .into()
    };

    let mut detail = column![
        row![
            text(sel.display_name())
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(16.0)
                .color(p.text),
            Space::new().width(10.0),
            mcp_status_chip(sel.status),
        ]
        .align_y(Alignment::Center),
        field(labels::SETTINGS_DEV_COMMAND, cmd),
        field(labels::SETTINGS_DEV_ARGUMENTS, args),
    ];
    if sel.status == McpStatus::Failed {
        detail = detail.push(field(labels::SETTINGS_DEV_ERROR, &sel.status_text));
    }

    container(row![
        container(list_col)
            .width(Length::Fixed(theme::SETTINGS_DEV_MCP_LIST_WIDTH))
            .padding(8),
        crate::pane_strip::hairline_v(),
        container(detail).width(Fill).padding([14, 20]),
    ])
    .width(Fill)
    .style(|_: &Theme| container::Style {
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    })
    .into()
}
