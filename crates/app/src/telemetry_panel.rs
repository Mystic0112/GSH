//! Aba **Telemetria** do painel direito (`telemetry_panel.dart`).
//!
//! Exibe cabeçalho com busca/chips + árvore `projeto -> run -> casos`,
//! replicando exatamente fontes, tamanhos, cores, truncamento `TextPainter` e
//! ações rápidas do app Flutter.

use gsh_telemetry::{
    now_ms, TelemetryCase, TelemetryRun, TelemetryRunSource, TelemetrySeverity,
    TelemetryTriageStatus,
};
use iced::widget::{
    button, column, container, mouse_area, row, scrollable, text, text_input, tooltip, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Fill, Padding, Theme};

use crate::telemetry_state::{
    flutter_ellipsis, format_bytes, format_relative_ms, run_display_label, ProjectGroup, RunGroup,
    TelemetryFilter, TelemetryMsg, TelemetryState,
};
use crate::{labels, pane_chrome, panel_widgets, theme};

pub const SEARCH_INPUT_ID: &str = "telemetry-search-input";

/// Largura útil do painel direito dentro da moldura (`FILES_PANEL_WIDTH - 1px`).
const PANEL_INNER_WIDTH: f32 = theme::FILES_PANEL_WIDTH - 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagKind {
    Fresh,
    Regression,
    Resolved,
    Ignored,
}

pub fn telemetry_tag<'a>(label: &'static str, kind: TagKind) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let (fg, bg, border): (Color, Option<Color>, Option<Color>) = match kind {
        TagKind::Fresh => (p.accent_text, Some(p.accent_soft), None),
        TagKind::Regression => (
            p.error,
            Some(theme::with_alpha(p.error, 0.15)),
            Some(theme::with_alpha(p.error, 0.35)),
        ),
        TagKind::Resolved => (p.ok, None, Some(theme::with_alpha(p.ok, 0.35))),
        TagKind::Ignored => (p.text3, None, Some(p.border2)),
    };

    container(
        text(label.to_uppercase())
            .font(theme::FONT_BODY)
            .size(theme::TELEMETRY_TAG_PILL_TEXT)
            .color(fg),
    )
    .padding([
        theme::TELEMETRY_TAG_PILL_PADDING[0],
        theme::TELEMETRY_TAG_PILL_PADDING[1],
    ])
    .style(move |_| container::Style {
        background: bg.map(Background::Color),
        border: Border {
            color: border.unwrap_or(Color::TRANSPARENT),
            width: if border.is_some() { 1.0 } else { 0.0 },
            radius: theme::TELEMETRY_TAG_PILL_RADIUS.into(),
        },
        ..Default::default()
    })
    .into()
}

pub fn view<'a>(state: &'a TelemetryState) -> Element<'a, TelemetryMsg> {
    view_with_active(state, None)
}

pub fn view_with_active<'a>(
    state: &'a TelemetryState,
    active_case_fp: Option<&'a str>,
) -> Element<'a, TelemetryMsg> {
    if state.workspace_id.is_none() && !(state.developer_mode && state.show_app_telemetry) {
        return panel_widgets::empty_label(labels::TELEMETRY_NO_WORKSPACE);
    }

    column![header(state), body(state, active_case_fp)]
        .height(Fill)
        .into()
}

fn header<'a>(state: &'a TelemetryState) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let title = text(labels::TELEMETRY_TITLE)
        .font(theme::FONT_TITLE)
        .size(theme::TELEMETRY_HEADER_TITLE_SIZE)
        .color(p.text);

    let mut top_row = row![title, Space::new().width(8)].align_y(Alignment::Center);
    let live = state.live_run_count();
    if live > 0 {
        top_row = top_row
            .push(
                container(Space::new())
                    .width(theme::TELEMETRY_LIVE_DOT)
                    .height(theme::TELEMETRY_LIVE_DOT)
                    .style(|_| container::Style {
                        background: Some(Background::Color(theme::palette().error)),
                        border: Border {
                            radius: (theme::TELEMETRY_LIVE_DOT / 2.0).into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
            )
            .push(Space::new().width(theme::TELEMETRY_LIVE_GAP))
            .push(
                text(labels::telemetry_live_runs(live))
                    .font(theme::FONT_MONO)
                    .size(theme::TELEMETRY_LIVE_TEXT)
                    .color(p.error),
            );
    }

    top_row = top_row.push(Space::new().width(Fill)).push(
        text(format_bytes(state.db_size_bytes))
            .font(theme::FONT_MONO)
            .size(theme::TELEMETRY_DB_SIZE_TEXT)
            .color(p.text4),
    );

    let header_body = container(column![
        top_row,
        Space::new().height(9),
        search_box(state),
        Space::new().height(8),
        filter_chips(state),
    ])
    .width(Fill)
    .padding(theme::TELEMETRY_HEADER_PADDING);

    column![header_body, panel_widgets::hairline_h()]
        .width(Fill)
        .into()
}

fn search_box<'a>(state: &'a TelemetryState) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let input = text_input(labels::TELEMETRY_SEARCH_HINT, &state.search)
        .id(SEARCH_INPUT_ID)
        .on_input(TelemetryMsg::SearchChanged)
        .font(theme::FONT_BODY)
        .size(theme::TELEMETRY_SEARCH_TEXT)
        .padding([5.0, 9.0])
        .width(Fill)
        .style(|_, _| {
            let p = theme::palette();
            text_input::Style {
                background: Background::Color(p.panel2),
                border: Border {
                    color: p.border,
                    width: 1.0,
                    radius: theme::TELEMETRY_SEARCH_RADIUS.into(),
                },
                icon: p.text3,
                placeholder: p.text4,
                value: p.text,
                selection: p.accent_soft,
            }
        });

    if state.search.is_empty() {
        return input.into();
    }

    row![
        input,
        button(
            text(labels::ICON_CLOSE)
                .font(theme::FONT_ICON)
                .size(14.0)
                .color(p.text3),
        )
        .padding(4)
        .on_press(TelemetryMsg::SearchChanged(String::new()))
        .style(theme::ghost_button),
    ]
    .spacing(4)
    .align_y(Alignment::Center)
    .into()
}

fn filter_chips<'a>(state: &'a TelemetryState) -> Element<'a, TelemetryMsg> {
    let mut chips: Vec<Element<'a, TelemetryMsg>> = vec![
        filter_chip(
            labels::TELEMETRY_CHIP_OPEN,
            Some(state.counts.open),
            state.filter == TelemetryFilter::Open,
            TelemetryMsg::SetFilter(TelemetryFilter::Open),
        ),
        filter_chip(
            labels::TELEMETRY_CHIP_NEW,
            Some(state.counts.fresh),
            state.filter == TelemetryFilter::New,
            TelemetryMsg::SetFilter(TelemetryFilter::New),
        ),
        filter_chip(
            labels::TELEMETRY_CHIP_RESOLVED,
            Some(state.counts.resolved),
            state.filter == TelemetryFilter::Resolved,
            TelemetryMsg::SetFilter(TelemetryFilter::Resolved),
        ),
        filter_chip(
            labels::TELEMETRY_CHIP_IGNORED,
            Some(state.counts.ignored),
            state.filter == TelemetryFilter::Ignored,
            TelemetryMsg::SetFilter(TelemetryFilter::Ignored),
        ),
        filter_chip(
            labels::TELEMETRY_CHIP_WARNINGS_WITH_ICON,
            Some(state.counts.warnings),
            state.show_warnings,
            TelemetryMsg::ToggleWarnings,
        ),
    ];

    if state.developer_mode {
        chips.push(filter_chip(
            labels::TELEMETRY_CHIP_APP,
            None,
            state.show_app_telemetry,
            TelemetryMsg::ToggleAppSource,
        ));
    }

    row(chips)
        .spacing(theme::TELEMETRY_CHIPS_GAP)
        .wrap()
        .vertical_spacing(theme::TELEMETRY_CHIPS_GAP)
        .into()
}

fn filter_chip<'a>(
    label: &'static str,
    count: Option<usize>,
    selected: bool,
    msg: TelemetryMsg,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let mut content = row![text(label)
        .font(theme::FONT_BODY)
        .size(theme::TELEMETRY_CHIP_TEXT)
        .color(if selected { p.text } else { p.text3 })]
    .align_y(Alignment::Center);

    if let Some(n) = count {
        content = content.push(Space::new().width(6)).push(
            text(n.to_string())
                .font(theme::FONT_MONO)
                .size(theme::TELEMETRY_CHIP_COUNT_TEXT)
                .color(if selected { p.text2 } else { p.text3 }),
        );
    }

    button(content)
        .padding([
            theme::TELEMETRY_CHIP_PADDING[0],
            theme::TELEMETRY_CHIP_PADDING[1],
        ])
        .on_press(msg)
        .style(move |_: &Theme, status| {
            let p = theme::palette();
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let bg = if hovered {
                p.panel3
            } else if selected {
                p.panel2
            } else {
                p.panel
            };
            button::Style {
                background: Some(Background::Color(bg)),
                text_color: if selected { p.text } else { p.text3 },
                border: Border {
                    color: if selected { p.border2 } else { p.border },
                    width: 1.0,
                    radius: theme::TELEMETRY_CHIP_RADIUS.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn body<'a>(
    state: &'a TelemetryState,
    active_case_fp: Option<&'a str>,
) -> Element<'a, TelemetryMsg> {
    let groups = state.grouped_tree();
    if groups.is_empty() {
        let empty = if !state.search.trim().is_empty() || state.filter != TelemetryFilter::Open {
            labels::TELEMETRY_EMPTY_FILTERED
        } else {
            labels::TELEMETRY_EMPTY
        };
        return container(
            text(empty)
                .font(theme::FONT_BODY)
                .size(theme::TELEMETRY_EMPTY_TEXT)
                .color(theme::palette().text4),
        )
        .center_x(Fill)
        .center_y(Fill)
        .padding(theme::TELEMETRY_EMPTY_PADDING)
        .into();
    }

    let now = now_ms();
    let flat = state.single_root();
    let mut list = column![];
    for group in groups {
        list = list.push(project_section(state, &group, flat, active_case_fp, now));
    }

    scrollable(list.padding(Padding {
        top: 4.0,
        right: 0.0,
        bottom: 12.0,
        left: 0.0,
    }))
    .width(Fill)
    .height(Fill)
    .into()
}

fn project_section<'a>(
    state: &'a TelemetryState,
    group: &ProjectGroup<'a>,
    flat: bool,
    active_case_fp: Option<&'a str>,
    now: i64,
) -> Element<'a, TelemetryMsg> {
    let collapsed = state.collapsed_projects.contains(group.project);
    let mut col = column![];
    if !flat {
        let hot = group
            .runs
            .iter()
            .any(|rg| rg.cases.iter().any(|c| is_open_error(c)));
        col = col.push(project_header(
            group.project,
            group.total_cases,
            hot,
            !collapsed,
        ));
    }
    if flat || !collapsed {
        let run_indent = if flat {
            theme::TELEMETRY_RUN_INDENT_FLAT
        } else {
            theme::TELEMETRY_RUN_INDENT_NESTED
        };
        let case_indent = if flat {
            theme::TELEMETRY_CASE_INDENT_FLAT
        } else {
            theme::TELEMETRY_CASE_INDENT_NESTED
        };
        for run_group in &group.runs {
            col = col.push(run_section(
                state,
                group.project,
                run_group,
                run_indent,
                case_indent,
                active_case_fp,
                now,
            ));
        }
    }
    col.width(Fill).into()
}

fn project_header<'a>(
    project: &'a str,
    count: usize,
    hot: bool,
    expanded: bool,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let chevron = if expanded {
        labels::ICON_EXPAND_MORE
    } else {
        labels::ICON_CHEVRON_RIGHT
    };

    button(
        row![
            text(chevron)
                .font(theme::FONT_ICON)
                .size(theme::TELEMETRY_GROUP_CHEVRON_ICON)
                .color(p.text4),
            Space::new().width(4),
            container(
                text(flutter_ellipsis(project, 26, Some(1)))
                    .font(theme::FONT_TITLE)
                    .size(theme::TELEMETRY_GROUP_TITLE_SIZE)
                    .color(p.text2)
                    .wrapping(text::Wrapping::None),
            )
            .width(Fill)
            .clip(true),
            text(count.to_string())
                .font(theme::FONT_MONO)
                .size(11.0)
                .color(if hot { p.error } else { p.text3 }),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(Padding {
        top: theme::TELEMETRY_GROUP_HEADER_PADDING_V[0],
        right: 10.0,
        bottom: theme::TELEMETRY_GROUP_HEADER_PADDING_V[1],
        left: theme::TELEMETRY_GROUP_INDENT,
    })
    .on_press(TelemetryMsg::ToggleProject(project.to_string()))
    .style(theme::ghost_button)
    .into()
}

fn run_section<'a>(
    state: &'a TelemetryState,
    project: &'a str,
    group: &RunGroup<'a>,
    run_indent: f32,
    case_indent: f32,
    active_case_fp: Option<&'a str>,
    now: i64,
) -> Element<'a, TelemetryMsg> {
    let toggle_key = format!("r:{project}:{}", group.run_key);
    let collapsed = state.collapsed_runs.contains(&toggle_key)
        || group
            .run
            .is_some_and(|r| state.collapsed_runs.contains(&r.id));
    let mut col = column![run_header(group, toggle_key, run_indent, !collapsed)];
    if !collapsed {
        for case in &group.cases {
            col = col.push(case_row(state, case, case_indent, active_case_fp, now));
        }
    }
    col.width(Fill).into()
}

fn run_header<'a>(
    group: &RunGroup<'a>,
    toggle_key: String,
    indent: f32,
    expanded: bool,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let chevron = if expanded {
        labels::ICON_EXPAND_MORE
    } else {
        labels::ICON_CHEVRON_RIGHT
    };
    let hot = group.cases.iter().any(|c| is_open_error(c));
    let raw_label = run_display_label(group.run, group.run_key);
    let source = group.run.map_or(TelemetryRunSource::Wrapper, |r| r.source);
    let is_task = source == TelemetryRunSource::Task;
    let live = group.run.is_some_and(TelemetryRun::is_live);
    let run_id_label = group
        .run
        .map(|r| labels::telemetry_run(&r.id))
        .unwrap_or_default();

    let pill_w = match source {
        TelemetryRunSource::Task => 32.0,
        TelemetryRunSource::App => 27.0,
        TelemetryRunSource::Wrapper => 70.0,
    };
    let live_w = if live { 14.0 } else { 0.0 };
    let clear_w = if group.run.is_some() { 23.0 } else { 0.0 };
    let run_id_w = if run_id_label.is_empty() {
        0.0
    } else {
        run_id_label.chars().count() as f32 * 6.4
    };
    let fixed_w = indent
        + 10.0
        + theme::TELEMETRY_RUN_CHEVRON_ICON
        + 4.0
        + 7.0
        + pill_w
        + live_w
        + clear_w
        + run_id_w;
    let avail_px = (PANEL_INNER_WIDTH - fixed_w).max(48.0);
    let max_chars = ((avail_px / (theme::TELEMETRY_RUN_TITLE_SIZE * 0.60)).floor() as usize).max(6);
    let label = flutter_ellipsis(raw_label, max_chars, None);

    let mut header_row = row![
        text(chevron)
            .font(theme::FONT_ICON)
            .size(theme::TELEMETRY_RUN_CHEVRON_ICON)
            .color(p.text4),
        Space::new().width(4),
        container(
            text(label)
                .font(theme::FONT_MONO)
                .size(theme::TELEMETRY_RUN_TITLE_SIZE)
                .color(p.text2),
        )
        .max_width(avail_px)
        .clip(true),
        Space::new().width(7),
        source_pill(source, is_task),
    ]
    .align_y(Alignment::Center);

    if live {
        header_row = header_row.push(Space::new().width(7)).push(
            container(Space::new())
                .width(7)
                .height(7)
                .style(|_| container::Style {
                    background: Some(Background::Color(theme::palette().online)),
                    border: Border {
                        radius: 3.5.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }

    header_row = header_row.push(Space::new().width(Fill));

    if let Some(run) = group.run {
        let clear_btn = tooltip(
            button(
                text(labels::ICON_DELETE_OUTLINE)
                    .font(theme::FONT_ICON)
                    .size(13.0)
                    .color(p.text4),
            )
            .padding(3)
            .on_press(TelemetryMsg::ClearRun(run.id.clone()))
            .style(theme::ghost_button),
            pane_chrome::tip_bubble(labels::TELEMETRY_CLEAR_RUN),
            tooltip::Position::Top,
        );
        header_row = header_row.push(clear_btn).push(Space::new().width(4));
    }

    if !run_id_label.is_empty() {
        header_row = header_row.push(
            text(run_id_label)
                .font(theme::FONT_MONO)
                .size(11.0)
                .color(if hot { p.error } else { p.text3 }),
        );
    }

    button(header_row)
        .width(Fill)
        .padding(Padding {
            top: theme::TELEMETRY_RUN_HEADER_PADDING_V,
            right: 10.0,
            bottom: theme::TELEMETRY_RUN_HEADER_PADDING_V,
            left: indent,
        })
        .on_press(TelemetryMsg::ToggleRun(toggle_key))
        .style(theme::ghost_button)
        .into()
}

fn source_pill<'a>(source: TelemetryRunSource, accent: bool) -> Element<'a, TelemetryMsg> {
    let label = match source {
        TelemetryRunSource::Task => labels::TELEMETRY_SRC_TASK,
        TelemetryRunSource::Wrapper => labels::TELEMETRY_SRC_WRAPPER,
        TelemetryRunSource::App => labels::TELEMETRY_SRC_APP,
    };
    container(
        text(label.to_uppercase())
            .font(theme::FONT_BODY)
            .size(theme::TELEMETRY_SOURCE_PILL_TEXT)
            .color(if accent {
                theme::palette().accent_text
            } else {
                theme::palette().text4
            }),
    )
    .padding([
        theme::TELEMETRY_SOURCE_PILL_PADDING[0],
        theme::TELEMETRY_SOURCE_PILL_PADDING[1],
    ])
    .style(move |_| {
        let p = theme::palette();
        container::Style {
            background: None,
            border: Border {
                color: if accent {
                    theme::with_alpha(p.accent_text, 0.35)
                } else {
                    p.border
                },
                width: 1.0,
                radius: theme::TELEMETRY_SOURCE_PILL_RADIUS.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn case_row<'a>(
    state: &'a TelemetryState,
    case: &'a TelemetryCase,
    indent: f32,
    active_case_fp: Option<&'a str>,
    now: i64,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let fp = case.fingerprint.clone();
    let open = case.status == TelemetryTriageStatus::Open;
    let is_hovered = state.hovered_case.as_deref() == Some(fp.as_str());
    let is_selected = active_case_fp == Some(fp.as_str());
    let sev_color = match case.status {
        TelemetryTriageStatus::Resolved => p.ok,
        TelemetryTriageStatus::Ignored => p.text4,
        TelemetryTriageStatus::Open => {
            if case.severity == TelemetrySeverity::Warn {
                p.warn
            } else {
                p.error
            }
        }
    };

    let dot = container(
        container(Space::new())
            .width(theme::TELEMETRY_CASE_DOT_SIZE)
            .height(theme::TELEMETRY_CASE_DOT_SIZE)
            .style(move |_| container::Style {
                background: Some(Background::Color(sev_color)),
                border: Border {
                    radius: (theme::TELEMETRY_CASE_DOT_SIZE / 2.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
    )
    .padding(Padding {
        top: 5.0,
        ..Padding::ZERO
    });

    // Em Dart: Text(c.message, maxLines: 1, overflow: TextOverflow.ellipsis, style: typo.mono 12.5)
    let right_col_w = if is_hovered { 68.0 } else { 24.0 };
    let msg_avail_px = (PANEL_INNER_WIDTH - indent - 8.0 - 8.0 - 8.0 - 6.0 - right_col_w).max(40.0);
    let msg_max_chars =
        ((msg_avail_px / (theme::TELEMETRY_CASE_MSG_SIZE * 0.60)).floor() as usize).max(10);
    let msg_line = flutter_ellipsis(&case.message, msg_max_chars, Some(1));

    let msg_text = text(msg_line)
        .font(theme::FONT_MONO)
        .size(theme::TELEMETRY_CASE_MSG_SIZE)
        .color(if open { p.text } else { p.text3 })
        .wrapping(text::Wrapping::None);

    let mut meta_items: Vec<Element<'a, TelemetryMsg>> = Vec::new();
    if case.is_regression && open {
        meta_items.push(telemetry_tag(
            labels::TELEMETRY_TAG_REGRESSION,
            TagKind::Regression,
        ));
    } else if case.is_new && open {
        meta_items.push(telemetry_tag(labels::TELEMETRY_TAG_NEW, TagKind::Fresh));
    }
    if case.status == TelemetryTriageStatus::Resolved {
        meta_items.push(telemetry_tag(
            labels::TELEMETRY_TAG_RESOLVED,
            TagKind::Resolved,
        ));
    }
    if case.status == TelemetryTriageStatus::Ignored {
        meta_items.push(telemetry_tag(
            labels::TELEMETRY_TAG_IGNORED,
            TagKind::Ignored,
        ));
    }
    if let Some(loc) = case.location.as_deref() {
        meta_items.push(
            text(loc)
                .font(theme::FONT_MONO)
                .size(theme::TELEMETRY_CASE_META_SIZE)
                .color(p.text3)
                .into(),
        );
    }
    meta_items.push(
        text(format_relative_ms(case.last_at_ms, now))
            .font(theme::FONT_BODY)
            .size(theme::TELEMETRY_CASE_META_SIZE)
            .color(p.text4)
            .into(),
    );

    let meta_wrap = row(meta_items).spacing(8).wrap().vertical_spacing(3);
    let center_col = column![msg_text, meta_wrap].spacing(2).width(Fill);

    let right_widget: Element<'a, TelemetryMsg> = if is_hovered {
        quick_actions(case)
    } else {
        text(format!("×{}", case.count))
            .font(theme::FONT_MONO)
            .size(theme::TELEMETRY_CASE_META_SIZE)
            .color(p.text2)
            .into()
    };

    let content = row![
        dot,
        Space::new().width(theme::TELEMETRY_CASE_ROW_GAP),
        center_col,
        Space::new().width(6),
        right_widget,
    ]
    .align_y(Alignment::Start);

    let btn = button(content)
        .width(Fill)
        .padding(Padding {
            top: theme::TELEMETRY_CASE_ROW_PADDING_V,
            right: theme::TELEMETRY_CASE_ROW_PADDING_RIGHT,
            bottom: theme::TELEMETRY_CASE_ROW_PADDING_V,
            left: indent,
        })
        .on_press(TelemetryMsg::OpenCase(fp.clone()))
        .style(move |_: &Theme, status| {
            let p = theme::palette();
            let hovered =
                is_hovered || matches!(status, button::Status::Hovered | button::Status::Pressed);
            let bg = if is_selected || hovered {
                Some(Background::Color(p.panel2))
            } else {
                None
            };
            button::Style {
                background: bg,
                text_color: p.text,
                border: Border::default(),
                ..Default::default()
            }
        });

    mouse_area(btn)
        .on_enter(TelemetryMsg::HoverCase(Some(fp)))
        .on_exit(TelemetryMsg::HoverCase(None))
        .into()
}

fn quick_actions<'a>(case: &'a TelemetryCase) -> Element<'a, TelemetryMsg> {
    let fp = &case.fingerprint;
    let mut actions = row![].spacing(2).align_y(Alignment::Center);
    if case.status == TelemetryTriageStatus::Open {
        actions = actions.push(quick_icon_btn(
            labels::ICON_CHECK,
            labels::TELEMETRY_RESOLVE,
            TelemetryMsg::ResolveCase(fp.clone()),
        ));
        actions = actions.push(quick_icon_btn(
            labels::ICON_VISIBILITY_OFF,
            labels::TELEMETRY_IGNORE,
            TelemetryMsg::IgnoreCase(fp.clone()),
        ));
    } else {
        actions = actions.push(quick_icon_btn(
            labels::ICON_REFRESH,
            labels::TELEMETRY_REOPEN,
            TelemetryMsg::ReopenCase(fp.clone()),
        ));
    }
    actions = actions.push(quick_icon_btn(
        labels::ICON_DELETE_OUTLINE,
        labels::TELEMETRY_CLEAR,
        TelemetryMsg::ClearCase(fp.clone()),
    ));
    actions.into()
}

fn quick_icon_btn<'a>(
    glyph: &'static str,
    tip: &'static str,
    msg: TelemetryMsg,
) -> Element<'a, TelemetryMsg> {
    let btn = button(
        text(glyph)
            .font(theme::FONT_ICON)
            .size(theme::TELEMETRY_QUICK_ACTION_ICON)
            .color(theme::palette().text2),
    )
    .padding(theme::TELEMETRY_QUICK_ACTION_PADDING)
    .on_press(msg)
    .style(|_: &Theme, status| {
        let p = theme::palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if hovered { p.panel3 } else { p.panel2 })),
            text_color: p.text2,
            border: Border {
                color: p.border2,
                width: 1.0,
                radius: theme::TELEMETRY_QUICK_ACTION_RADIUS.into(),
            },
            ..Default::default()
        }
    });

    tooltip(btn, pane_chrome::tip_bubble(tip), tooltip::Position::Top).into()
}

fn is_open_error(c: &TelemetryCase) -> bool {
    c.status == TelemetryTriageStatus::Open
        && c.severity.index() >= TelemetrySeverity::Error.index()
}
