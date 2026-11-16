//! Abas-catálogo (`settings_catalog.dart`, `plugin_directory.dart`, `skills_panel.dart`,
//! `connectors_panel.dart`, `plugins_panel.dart`).

use super::cli::{self, MarketplacePlugin, McpOrigin, McpServerInfo, McpStatus, PluginInfo};
use super::files::{SkillInfo, SkillSource};
use super::widgets::{
    catalog_matches, chip, field_style, mcp_status_chip, note, refresh_button, section,
    setting_row, shell_quote, small_button, Emphasis,
};
use super::{ClaudeState, Message, NamePromptKind};
use crate::panel_widgets;
use crate::settings::view::{choice, segmented};
use crate::settings::Settings;
use crate::{labels, theme};
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};
use std::collections::BTreeSet;

type El<'a> = Element<'a, Message>;

fn catalog_header<'a>(
    title: &'static str,
    discover: bool,
    search: Option<(&'a str, &'static str)>,
    actions: Vec<El<'a>>,
) -> El<'a> {
    let p = theme::palette();
    let mut bar = row![
        text(title)
            .font(theme::FONT_TITLE)
            .size(theme::SETTINGS_CATALOG_TITLE_SIZE)
            .color(p.text),
        Space::new().width(16.0),
        segmented(row![
            choice(
                labels::SETTINGS_CAT_MINE,
                !discover,
                Message::CatalogDiscover(false)
            ),
            choice(
                labels::SETTINGS_CAT_DISCOVER,
                discover,
                Message::CatalogDiscover(true)
            ),
        ]),
    ]
    .width(Fill)
    .align_y(Alignment::Center);

    if let Some((query, hint)) = search {
        let input = text_input(hint, query)
            .icon(text_input::Icon {
                font: theme::FONT_ICON,
                code_point: labels::ICON_SEARCH,
                size: Some(iced::Pixels(15.0)),
                spacing: 8.0,
                side: text_input::Side::Left,
            })
            .on_input(Message::CatalogSearch)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DROPDOWN_TEXT)
            .padding([6, 10])
            .width(Fill)
            .style(field_style(8.0));
        let search_slot = container(
            container(input)
                .max_width(theme::SETTINGS_CATALOG_SEARCH_WIDTH)
                .width(Fill),
        )
        .width(Fill)
        .align_x(iced::alignment::Horizontal::Right);
        bar = bar.push(Space::new().width(12.0)).push(search_slot);
    } else {
        bar = bar.push(Space::new().width(Fill));
    }
    for action in actions {
        bar = bar.push(Space::new().width(10.0)).push(action);
    }
    container(bar)
        .padding(iced::Padding {
            bottom: 14.0,
            ..Default::default()
        })
        .width(Fill)
        .into()
}

/// `CatalogAddButton`: `"+ Adicionar ⌄"` com seta para baixo (`settings_catalog.dart`).
fn catalog_add_dropdown_button(open: bool) -> El<'static> {
    let color = theme::ON_ACCENT;
    let content = row![
        text(labels::ICON_PLUS)
            .font(theme::FONT_ICON)
            .size(theme::SETTINGS_BUTTON_ICON)
            .color(color),
        text(labels::SETTINGS_CAT_ADD)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::SETTINGS_BUTTON_TEXT)
            .color(color),
        text(labels::ICON_CHEVRON_DOWN)
            .font(theme::FONT_ICON)
            .size(theme::SETTINGS_BUTTON_ICON)
            .color(color),
    ]
    .spacing(theme::SETTINGS_BUTTON_GAP)
    .align_y(Alignment::Center);
    button(content)
        .padding(theme::SETTINGS_BUTTON_PADDING)
        .on_press(Message::CatalogAddMenu(!open))
        .style(|_: &Theme, status| {
            let p = theme::palette();
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let mut fill = p.accent;
            if hovered {
                fill.a = 0.9;
            }
            button::Style {
                background: Some(Background::Color(fill)),
                text_color: theme::ON_ACCENT,
                border: Border {
                    color: fill,
                    width: 1.0,
                    radius: theme::SETTINGS_BUTTON_RADIUS.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn skills_add_menu() -> El<'static> {
    let item = |icon: &'static str, label: &'static str, msg: Message| -> El<'static> {
        let p = theme::palette();
        button(
            row![
                text(icon).font(theme::FONT_ICON).size(15.0).color(p.text2),
                Space::new().width(8.0),
                text(label).font(theme::FONT_BODY).size(13.5).color(p.text),
            ]
            .align_y(Alignment::Center),
        )
        .width(Fill)
        .padding([7, 10])
        .on_press(msg)
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
    };
    let card = container(
        column![
            item(
                labels::ICON_PLUS,
                labels::SETTINGS_CAT_NEW_SKILL,
                Message::NamePromptOpen(NamePromptKind::Skill),
            ),
            item(
                labels::ICON_STOREFRONT,
                labels::SETTINGS_CAT_ADD_MARKETPLACE,
                Message::NamePromptOpen(NamePromptKind::Marketplace),
            ),
        ]
        .spacing(2.0),
    )
    .width(Length::Fixed(220.0))
    .padding(4)
    .style(|_: &Theme| {
        let p = theme::palette();
        container::Style {
            background: Some(Background::Color(p.panel2)),
            border: Border {
                color: p.border2,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        }
    });
    container(card)
        .width(Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .padding(iced::Padding {
            bottom: 10.0,
            ..Default::default()
        })
        .into()
}

fn group_title(title: &'static str, count: usize) -> El<'static> {
    let p = theme::palette();
    container(
        row![
            text(title)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(14.5)
                .color(p.text),
            Space::new().width(8.0),
            chip(count.to_string()),
        ]
        .align_y(Alignment::Center),
    )
    .padding(iced::Padding {
        top: 18.0,
        bottom: 6.0,
        ..Default::default()
    })
    .into()
}

fn catalog_row<'a>(
    icon: &'static str,
    title: String,
    badge: Option<String>,
    by: Option<String>,
    desc: Option<String>,
    trailing: Option<El<'a>>,
    on_tap: Option<Message>,
) -> El<'a> {
    let p = theme::palette();
    let icon_box = container(text(icon).font(theme::FONT_ICON).size(18.0).color(p.text2))
        .width(Length::Fixed(36.0))
        .height(Length::Fixed(36.0))
        .center_x(Length::Fixed(36.0))
        .center_y(Length::Fixed(36.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().bg)),
            border: Border {
                color: theme::palette().border,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

    let mut title_line = row![text(title)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_NAV_TEXT_SIZE)
        .color(p.text)]
    .align_y(Alignment::Center);
    if let Some(badge) = badge {
        title_line = title_line.push(Space::new().width(8.0)).push(chip(badge));
    }

    let mut sub_parts = Vec::new();
    if let Some(by) = by.filter(|s| !s.is_empty()) {
        sub_parts.push(by);
    }
    if let Some(desc) = desc.filter(|s| !s.is_empty()) {
        sub_parts.push(desc);
    }
    let sub = sub_parts.join("  ·  ");

    let mut mid = column![title_line];
    if !sub.is_empty() {
        mid = mid.push(text(sub).font(theme::FONT_BODY).size(12.5).color(p.text3));
    }

    let mut line =
        row![icon_box, Space::new().width(16.0), mid.width(Fill)].align_y(Alignment::Center);
    if let Some(trailing) = trailing {
        line = line.push(Space::new().width(16.0)).push(trailing);
    }

    let interactive: El<'a> = if let Some(msg) = on_tap {
        button(line)
            .width(Fill)
            .padding([10, 4])
            .on_press(msg)
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
    } else {
        container(line).width(Fill).padding([10, 4]).into()
    };

    column![interactive, panel_widgets::hairline_h()].into()
}

/// `PluginDirectory` (`plugin_directory.dart`): filtro por marketplace e grade em 2 colunas.
fn plugin_directory<'a>(c: &'a ClaudeState, query: &str) -> El<'a> {
    let all = match &c.directory {
        Some(Err(err)) => return note(labels::claude_info_error(err), true),
        None => return note(labels::SETTINGS_CLAUDE_LOADING.to_owned(), false),
        Some(Ok(list)) => list,
    };
    let markets: BTreeSet<&str> = all.iter().map(|p| p.marketplace.as_str()).collect();
    let mut filter_row = row![filter_chip(
        labels::SETTINGS_CAT_ALL.to_owned(),
        c.selected_marketplace.is_none(),
        Message::SelectMarketplace(None),
    )]
    .spacing(8.0)
    .align_y(Alignment::Center);
    for m in markets {
        let selected = c.selected_marketplace.as_deref() == Some(m);
        filter_row = filter_row.push(filter_chip(
            m.to_owned(),
            selected,
            Message::SelectMarketplace(Some(m.to_owned())),
        ));
    }

    let shown: Vec<&MarketplacePlugin> = all
        .iter()
        .filter(|p| {
            c.selected_marketplace
                .as_deref()
                .is_none_or(|m| p.marketplace == m)
                && catalog_matches(
                    query,
                    &[Some(&p.name), p.description.as_deref(), p.author.as_deref()],
                )
        })
        .collect();

    let mut body = column![container(filter_row).padding(iced::Padding {
        bottom: 14.0,
        ..Default::default()
    })];

    if shown.is_empty() {
        body = body.push(note(labels::SETTINGS_NAV_NO_RESULTS.to_owned(), false));
    } else {
        let mut grid = column![].spacing(12.0);
        let mut iter = shown.into_iter();
        while let Some(first) = iter.next() {
            let left = directory_card(first);
            let right: El<'a> = match iter.next() {
                Some(second) => directory_card(second),
                None => Space::new().width(Fill).into(),
            };
            grid = grid.push(
                row![
                    container(left).width(Fill),
                    Space::new().width(12.0),
                    container(right).width(Fill),
                ]
                .width(Fill),
            );
        }
        body = body.push(grid);
    }
    body.width(Fill).into()
}

fn filter_chip(label: String, selected: bool, on_press: Message) -> El<'static> {
    button(
        text(label)
            .font(theme::FONT_BODY)
            .size(13.0)
            .color(if selected {
                theme::palette().text
            } else {
                theme::palette().text2
            }),
    )
    .padding([5, 10])
    .on_press(on_press)
    .style(move |_: &Theme, status| {
        let p = theme::palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: (selected || hovered).then_some(Background::Color(p.panel3)),
            text_color: if selected { p.text } else { p.text2 },
            border: Border {
                color: if selected { p.border2 } else { p.border },
                width: 1.0,
                radius: 7.0.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn directory_card(plugin: &MarketplacePlugin) -> El<'_> {
    let p = theme::palette();
    let sub = match &plugin.author {
        Some(author) => format!("{}  ·  {}", labels::catalog_by(author), plugin.marketplace),
        None => plugin.marketplace.clone(),
    };
    let trailing: El<'_> = if plugin.installed {
        text(labels::ICON_CHECK)
            .font(theme::FONT_ICON)
            .size(18.0)
            .color(p.ok)
            .into()
    } else {
        small_button(
            labels::SETTINGS_COMMON_ADD,
            Some(labels::ICON_PLUS),
            Emphasis::Outline,
            Some(Message::RunInTerminal(format!(
                "claude plugin install {}",
                shell_quote(&plugin.id())
            ))),
        )
    };
    container(
        row![
            text(labels::ICON_POWER)
                .font(theme::FONT_ICON)
                .size(20.0)
                .color(p.text2),
            Space::new().width(12.0),
            column![
                text(&plugin.name)
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(theme::SETTINGS_NAV_TEXT_SIZE)
                    .color(p.text),
                text(sub)
                    .font(theme::FONT_BODY_MEDIUM)
                    .size(theme::SIZE_LABEL)
                    .color(p.text3),
                Space::new().height(4.0),
                text(plugin.description.as_deref().unwrap_or(""))
                    .font(theme::FONT_BODY)
                    .size(13.0)
                    .color(p.text2),
            ]
            .width(Fill),
            Space::new().width(8.0),
            trailing,
        ]
        .align_y(Alignment::Start),
    )
    .padding(14)
    .height(Length::Fixed(theme::SETTINGS_PLUGIN_CARD_HEIGHT))
    .width(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel2)),
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Aba **Skills** (`skills_panel.dart`).
pub fn skills(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let mut actions = Vec::new();
    if !c.catalog_discover {
        actions.push(refresh_button(c.skills_loading, Message::RefreshSkills));
    }
    actions.push(catalog_add_dropdown_button(c.catalog_add_menu));

    let raw_header = catalog_header(
        labels::SETTINGS_NAV_SKILLS,
        c.catalog_discover,
        Some((&c.catalog_search, labels::SETTINGS_NAV_SEARCH)),
        actions,
    );
    let header: El<'_> = if c.catalog_add_menu {
        column![raw_header, skills_add_menu()].width(Fill).into()
    } else {
        raw_header
    };

    if c.catalog_discover {
        return column![header, plugin_directory(c, &c.catalog_search)]
            .width(Fill)
            .into();
    }

    let list = match &c.skills {
        Some(Err(err)) => {
            return column![header, note(labels::claude_info_error(err), true)]
                .width(Fill)
                .into()
        }
        None => {
            return column![
                header,
                note(labels::SETTINGS_CLAUDE_LOADING.to_owned(), false)
            ]
            .width(Fill)
            .into()
        }
        Some(Ok(items)) => items,
    };

    let filtered: Vec<&SkillInfo> = list
        .iter()
        .filter(|sk| {
            catalog_matches(
                &c.catalog_search,
                &[
                    Some(&sk.name),
                    sk.description.as_deref(),
                    sk.plugin.as_deref(),
                ],
            )
        })
        .collect();

    let mut body = column![header];
    if filtered.is_empty() {
        body = body.push(note(labels::SETTINGS_SKILLS_EMPTY.to_owned(), false));
        return body.width(Fill).into();
    }

    for (src, title) in [
        (SkillSource::User, labels::SETTINGS_SKILLS_GROUP_USER),
        (SkillSource::Project, labels::SETTINGS_SKILLS_GROUP_PROJECT),
        (SkillSource::Plugin, labels::SETTINGS_SKILLS_GROUP_PLUGINS),
    ] {
        let group: Vec<&SkillInfo> = filtered
            .iter()
            .copied()
            .filter(|s| s.source == src)
            .collect();
        if group.is_empty() {
            continue;
        }
        body = body.push(group_title(title, group.len()));
        for sk in group {
            let by = match sk.source {
                SkillSource::User => Some(labels::SETTINGS_SKILLS_BY_YOU.to_owned()),
                SkillSource::Project => Some(labels::SETTINGS_SKILLS_BY_PROJECT.to_owned()),
                SkillSource::Plugin => sk
                    .plugin
                    .as_deref()
                    .and_then(|p| p.split('@').next())
                    .map(str::to_owned),
            };
            let trailing = sk.updated_at.map(|ts| {
                text(cli::short_date(ts))
                    .font(theme::FONT_BODY)
                    .size(13.0)
                    .color(theme::palette().text3)
                    .into()
            });
            body = body.push(catalog_row(
                labels::ICON_HISTORY_EDU,
                sk.name.clone(),
                None,
                by,
                sk.description.clone(),
                trailing,
                Some(Message::OpenFile(sk.path.clone())),
            ));
        }
    }
    body.width(Fill).into()
}

/// Aba **Conectores** (`connectors_panel.dart`).
pub fn connectors(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let mut actions = Vec::new();
    if !c.catalog_discover {
        actions.push(refresh_button(c.mcp_loading, Message::RefreshMcp));
    }
    actions.push(small_button(
        labels::SETTINGS_CAT_ADD,
        Some(labels::ICON_PLUS),
        Emphasis::Primary,
        Some(Message::CatalogDiscover(true)),
    ));

    let search =
        (!c.catalog_discover).then_some((c.catalog_search.as_str(), labels::SETTINGS_NAV_SEARCH));
    let header = catalog_header(
        labels::SETTINGS_NAV_CONNECTORS,
        c.catalog_discover,
        search,
        actions,
    );

    if c.catalog_discover {
        return column![header, add_server_form(c)].width(Fill).into();
    }

    if c.mcp_loading && c.mcp.is_none() {
        return column![
            header,
            note(labels::SETTINGS_CONN_CHECKING.to_owned(), false)
        ]
        .width(Fill)
        .into();
    }
    let servers = match &c.mcp {
        Some(Err(err)) => {
            return column![header, note(labels::claude_info_error(err), true)]
                .width(Fill)
                .into()
        }
        None => {
            return column![
                header,
                note(labels::SETTINGS_CLAUDE_LOADING.to_owned(), false)
            ]
            .width(Fill)
            .into()
        }
        Some(Ok(list)) => list,
    };

    let filtered: Vec<&McpServerInfo> = servers
        .iter()
        .filter(|srv| catalog_matches(&c.catalog_search, &[Some(&srv.name), Some(&srv.target)]))
        .collect();

    if filtered.is_empty() {
        return column![header, note(labels::SETTINGS_CONN_EMPTY.to_owned(), false)]
            .width(Fill)
            .into();
    }

    let p = theme::palette();
    let mut table = column![
        header,
        connector_table_line(
            text(labels::SETTINGS_CONN_COL_CONNECTOR)
                .font(theme::FONT_BODY)
                .size(13.0)
                .color(p.text2)
                .into(),
            text(labels::SETTINGS_CONN_COL_TYPE)
                .font(theme::FONT_BODY)
                .size(13.0)
                .color(p.text2)
                .into(),
            text(labels::SETTINGS_CONN_COL_STATUS)
                .font(theme::FONT_BODY)
                .size(13.0)
                .color(p.text2)
                .into(),
        )
    ];

    for srv in filtered {
        table = table.push(connector_server_row(srv));
    }
    table.width(Fill).into()
}

fn connector_server_row(srv: &McpServerInfo) -> El<'_> {
    let p = theme::palette();
    let icon = if srv.is_remote() {
        labels::ICON_LANGUAGE
    } else {
        labels::ICON_TERMINAL
    };
    let sub = match srv.plugin_name() {
        Some(pl) => labels::connectors_from_plugin(pl),
        None => srv.target.clone(),
    };
    let name_col = row![
        text(icon).font(theme::FONT_ICON).size(17.0).color(p.text2),
        Space::new().width(12.0),
        column![
            text(srv.display_name())
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_NAV_TEXT_SIZE)
                .color(p.text),
            text(sub).font(theme::FONT_BODY).size(12.5).color(p.text3),
        ]
        .width(Fill),
    ]
    .align_y(Alignment::Center);

    let type_label = if srv.is_remote() {
        labels::SETTINGS_CONN_TYPE_WEB
    } else {
        labels::SETTINGS_CONN_TYPE_LOCAL
    };
    let mut type_col = row![text(type_label)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_DESC_SIZE)
        .color(p.text2)]
    .align_y(Alignment::Center);
    let badge = match srv.origin() {
        McpOrigin::Plugin => Some(labels::SETTINGS_CONN_ORIGIN_PLUGIN),
        McpOrigin::ClaudeAi => Some("claude.ai"),
        McpOrigin::Local => None,
    };
    if let Some(b) = badge {
        type_col = type_col
            .push(Space::new().width(6.0))
            .push(chip(b.to_owned()));
    }

    let status_col: El<'_> = if srv.status == McpStatus::NeedsAuth {
        small_button(
            labels::SETTINGS_CONN_LINK,
            None,
            Emphasis::Outline,
            Some(Message::RunInTerminal(format!(
                "claude mcp login {}",
                shell_quote(&srv.name)
            ))),
        )
    } else {
        mcp_status_chip(srv.status)
    };

    connector_table_line(name_col.into(), type_col.into(), status_col)
}

fn connector_table_line<'a>(name: El<'a>, kind: El<'a>, status: El<'a>) -> El<'a> {
    column![
        row![
            container(name).width(Length::FillPortion(5)),
            Space::new().width(12.0),
            container(kind).width(Length::FillPortion(2)),
            container(status).width(Length::FillPortion(2)),
        ]
        .align_y(Alignment::Center)
        .padding([9, 0]),
        panel_widgets::hairline_h(),
    ]
    .into()
}

fn add_server_form(c: &ClaudeState) -> El<'_> {
    let p = theme::palette();
    let name_trim = c.mcp_form_name.trim();
    let target_trim = c.mcp_form_target.trim();
    let ready = !name_trim.is_empty() && !target_trim.is_empty();
    let scope = if c.mcp_form_local_scope {
        "local"
    } else {
        "user"
    };
    let command = if c.mcp_form_http {
        format!(
            "claude mcp add -s {scope} -t http {} {}",
            shell_quote(name_trim),
            shell_quote(target_trim)
        )
    } else {
        format!(
            "claude mcp add -s {scope} {} -- {target_trim}",
            shell_quote(name_trim)
        )
    };

    let name_input = text_input("github", &c.mcp_form_name)
        .on_input(Message::McpFormName)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_DROPDOWN_TEXT)
        .padding(theme::SETTINGS_TEXT_FIELD_PADDING)
        .width(Length::Fixed(theme::SETTINGS_CONN_FIELD_WIDTH))
        .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS));

    let target_hint = if c.mcp_form_http {
        "https://…/mcp"
    } else {
        "npx -y @scope/server"
    };
    let target_input = text_input(target_hint, &c.mcp_form_target)
        .on_input(Message::McpFormTarget)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_DROPDOWN_TEXT)
        .padding(theme::SETTINGS_TEXT_FIELD_PADDING)
        .width(Length::Fixed(theme::SETTINGS_CONN_FIELD_WIDTH))
        .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS));

    let footer = container(
        row![
            text(if ready {
                command.clone()
            } else {
                String::new()
            })
            .font(theme::FONT_MONO)
            .size(theme::SIZE_MONO)
            .color(p.text3)
            .width(Fill),
            small_button(
                labels::SETTINGS_CONN_DOCS,
                Some(labels::ICON_NORTH_EAST),
                Emphasis::Outline,
                Some(Message::OpenUrl(String::from(labels::MCP_DOCS_URL))),
            ),
            Space::new().width(8.0),
            small_button(
                labels::SETTINGS_CONN_ADD_IN_TERMINAL,
                None,
                Emphasis::Primary,
                ready.then_some(Message::RunInTerminal(command)),
            ),
        ]
        .align_y(Alignment::Center),
    )
    .padding([theme::SETTINGS_ROW_VERTICAL, 0]);

    column![
        note(labels::SETTINGS_CONN_DISCOVER_INTRO.to_owned(), false),
        section(
            labels::SETTINGS_CONN_ADD_SERVER,
            None,
            vec![
                setting_row(
                    labels::SETTINGS_CONN_FORM_NAME,
                    None,
                    Some(name_input.into()),
                    None,
                ),
                setting_row(
                    labels::SETTINGS_CONN_FORM_TYPE,
                    None,
                    Some(segmented(row![
                        choice(
                            labels::SETTINGS_CONN_TYPE_WEB,
                            c.mcp_form_http,
                            Message::McpFormHttp(true)
                        ),
                        choice(
                            labels::SETTINGS_CONN_TYPE_LOCAL,
                            !c.mcp_form_http,
                            Message::McpFormHttp(false)
                        ),
                    ])),
                    None,
                ),
                setting_row(
                    if c.mcp_form_http {
                        labels::SETTINGS_CONN_FORM_URL
                    } else {
                        labels::SETTINGS_CONN_FORM_COMMAND
                    },
                    None,
                    Some(target_input.into()),
                    None,
                ),
                setting_row(
                    labels::SETTINGS_CONN_FORM_SCOPE,
                    Some(labels::SETTINGS_CONN_FORM_SCOPE_DESC),
                    Some(segmented(row![
                        choice(
                            labels::SETTINGS_CONN_SCOPE_USER,
                            !c.mcp_form_local_scope,
                            Message::McpFormLocalScope(false)
                        ),
                        choice(
                            labels::SETTINGS_CONN_SCOPE_LOCAL,
                            c.mcp_form_local_scope,
                            Message::McpFormLocalScope(true)
                        ),
                    ])),
                    None,
                ),
                footer.into(),
            ],
        ),
    ]
    .width(Fill)
    .into()
}

/// Aba **Plugins** (`plugins_panel.dart`).
pub fn plugins(s: &Settings) -> El<'_> {
    let c = &s.claude;
    let mut actions = Vec::new();
    if !c.catalog_discover {
        actions.push(refresh_button(c.plugins_loading, Message::RefreshPlugins));
    }
    actions.push(small_button(
        labels::SETTINGS_CAT_ADD,
        Some(labels::ICON_PLUS),
        Emphasis::Primary,
        Some(Message::NamePromptOpen(NamePromptKind::Marketplace)),
    ));

    let header = catalog_header(
        labels::SETTINGS_NAV_PLUGINS,
        c.catalog_discover,
        Some((&c.catalog_search, labels::SETTINGS_PLUGINS_SEARCH)),
        actions,
    );

    if c.catalog_discover {
        return column![header, plugin_directory(c, &c.catalog_search)]
            .width(Fill)
            .into();
    }

    let mut body = column![
        header,
        group_title(
            labels::SETTINGS_PLUGINS_GROUP_PROJECT,
            c.project_plugins.len()
        ),
    ];

    let all_installed = match &c.plugins {
        Some(Ok(list)) => list.as_slice(),
        _ => &[],
    };

    if c.workspace.is_none() {
        body = body.push(note(
            labels::SETTINGS_PLUGINS_NO_WORKSPACE.to_owned(),
            false,
        ));
    } else if c.project_plugins.is_empty() {
        body = body.push(note(
            labels::SETTINGS_PLUGINS_PROJECT_EMPTY.to_owned(),
            false,
        ));
    } else {
        for id in &c.project_plugins {
            let found = all_installed
                .iter()
                .find(|p| p.id == *id)
                .cloned()
                .unwrap_or(PluginInfo {
                    id: id.clone(),
                    enabled: true,
                    version: None,
                    scope: None,
                    install_path: None,
                    updated_at: None,
                    display_name: None,
                    author: None,
                    description: None,
                });
            body = body.push(plugin_row(&found));
        }
    }

    match &c.plugins {
        Some(Err(err)) => {
            body = body.push(note(labels::claude_info_error(err), true));
        }
        None => {
            body = body.push(note(labels::SETTINGS_CLAUDE_LOADING.to_owned(), false));
        }
        Some(Ok(list)) => {
            let filtered: Vec<&PluginInfo> = list
                .iter()
                .filter(|p| {
                    catalog_matches(
                        &c.catalog_search,
                        &[
                            Some(p.name()),
                            p.description.as_deref(),
                            p.author.as_deref(),
                        ],
                    )
                })
                .collect();
            body = body.push(group_title(
                labels::SETTINGS_PLUGINS_GROUP_MARKETPLACES,
                filtered.len(),
            ));
            if filtered.is_empty() {
                body = body.push(note(labels::SETTINGS_PLUGINS_EMPTY.to_owned(), false));
            } else {
                for pl in filtered {
                    body = body.push(plugin_row(pl));
                }
            }
        }
    }

    body.width(Fill).into()
}

fn plugin_row(pl: &PluginInfo) -> El<'static> {
    let p = theme::palette();
    let badge = (!pl.enabled).then(|| labels::SETTINGS_PLUGINS_DISABLED.to_owned());
    let by = match &pl.author {
        Some(author) => Some(labels::catalog_by(author)),
        None => {
            let mkt = pl.marketplace();
            (!mkt.is_empty()).then(|| mkt.to_owned())
        }
    };
    let mut right = column![].align_x(Alignment::End);
    let mut has_right = false;
    if let Some(ts) = pl.updated_at {
        has_right = true;
        right = right.push(
            text(cli::short_date(ts))
                .font(theme::FONT_BODY)
                .size(13.0)
                .color(p.text3),
        );
    }
    if let Some(ver) = &pl.version {
        has_right = true;
        let short_ver = if ver.len() > 10 {
            ver.get(..7).unwrap_or(ver).to_owned()
        } else {
            ver.clone()
        };
        right = right.push(
            text(short_ver)
                .font(theme::FONT_MONO)
                .size(11.5)
                .color(p.text4),
        );
    }
    catalog_row(
        labels::ICON_POWER,
        pl.name().to_owned(),
        badge,
        by,
        pl.description.clone(),
        has_right.then(|| right.into()),
        None,
    )
}
