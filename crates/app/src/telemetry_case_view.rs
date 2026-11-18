//! Aba central com os detalhes de um caso de Telemetria (`telemetry_case_view.dart`).
//!
//! Layout espelhando o Flutter: fundo `panel`, padding `(26, 22, 26, 40)`,
//! `maxWidth: 980`, `_Crumbs` em `Wrap`, `_KindPill` + `TelemetryTag`, mensagem
//! em `FONT_MONO` 17, `_Code` (`short_id`) + `_Status` + hint + `_BlameChip`,
//! divisor, `_Btn`, `_Stats` (`panel2`, `radius: 9`), `Stack`, `Log correlacionado`,
//! `Ocorrências por run` e `Contexto cru`.

use gsh_telemetry::{
    now_ms, TelemetryCase, TelemetryEvent, TelemetryFrame, TelemetryRun, TelemetryRunSource,
    TelemetrySeverity, TelemetryTriageStatus,
};
use iced::widget::{button, column, container, row, scrollable, text, tooltip, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Padding, Theme};

use crate::telemetry_panel::{telemetry_tag, TagKind};
use crate::telemetry_state::{
    flutter_ellipsis, format_relative_ms, run_display_label, split_location, TelemetryBlame,
    TelemetryCaseDetail, TelemetryMsg, TelemetryState,
};
use crate::{labels, pane_chrome, panel_widgets, theme};

/// Largura máxima aproximada em caracteres dentro de `maxWidth: 980 - 52 = 928px`.
const CRUMB_MAX_CHARS: usize = 148;
const CONTEXT_BODY_MAX_CHARS: usize = 136;

pub fn view<'a>(state: &'a TelemetryState, fingerprint: &'a str) -> Element<'a, TelemetryMsg> {
    let Some(detail) = state.case_details.get(fingerprint) else {
        if let Some(case) = state.cases.iter().find(|c| c.fingerprint == fingerprint) {
            return render_case(state, case, None);
        }
        return container(
            text(labels::TELEMETRY_EMPTY_FILTERED)
                .font(theme::FONT_BODY)
                .size(theme::TELEMETRY_EMPTY_TEXT)
                .color(theme::palette().text3),
        )
        .center_x(Fill)
        .center_y(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::palette().panel)),
            ..Default::default()
        })
        .into();
    };
    render_case(state, &detail.case, Some(detail))
}

fn render_case<'a>(
    state: &'a TelemetryState,
    case: &'a TelemetryCase,
    detail: Option<&'a TelemetryCaseDetail>,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let now = now_ms();
    let rep_run = detail.and_then(|d| d.rep_run.as_ref());
    let last_event = detail.and_then(|d| d.last_event.as_ref());
    let open = case.status == TelemetryTriageStatus::Open;

    let mut kind_row = row![kind_pill(
        &case.err_type,
        case.severity == TelemetrySeverity::Warn,
    )]
    .spacing(10)
    .align_y(Alignment::Center);

    if case.is_regression && open {
        kind_row = kind_row.push(telemetry_tag(
            labels::TELEMETRY_TAG_REGRESSION,
            TagKind::Regression,
        ));
    } else if case.is_new && open {
        kind_row = kind_row.push(telemetry_tag(labels::TELEMETRY_TAG_NEW, TagKind::Fresh));
    }

    let message = text(&case.message)
        .font(theme::FONT_MONO)
        .size(theme::TELEMETRY_CASE_HEADLINE_SIZE)
        .line_height(theme::TELEMETRY_CASE_HEADLINE_LINE_HEIGHT)
        .color(p.text);

    let mut content = column![
        breadcrumbs(case, rep_run),
        Space::new().height(10),
        kind_row,
        Space::new().height(10),
        message,
        Space::new().height(10),
        meta_wrap(case, detail.and_then(|d| d.blame.as_ref()), now),
        Space::new().height(16),
        panel_widgets::hairline_h(),
        Space::new().height(10),
        action_bar(state, case, rep_run),
        Space::new().height(14),
        stats_bar(case, rep_run, now),
        Space::new().height(theme::TELEMETRY_VIEW_SECTION_GAP),
        stack_section(last_event),
        Space::new().height(theme::TELEMETRY_VIEW_SECTION_GAP),
        correlated_log_section(detail.and_then(|d| d.correlated_log.as_ref())),
        Space::new().height(theme::TELEMETRY_VIEW_SECTION_GAP),
        runs_section(case, detail.map(|d| &d.runs_by_id), now),
        Space::new().height(theme::TELEMETRY_VIEW_SECTION_GAP),
        context_section(
            detail.map_or(&[], |d| d.context.as_slice()),
            last_event.and_then(|e| e.id.as_deref()),
        ),
    ]
    .width(Fill);

    content = content.max_width(theme::TELEMETRY_VIEW_MAX_WIDTH);

    let padded = container(content)
        .width(Fill)
        .padding(theme::TELEMETRY_VIEW_PADDING);

    container(scrollable(padded).width(Fill).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::palette().panel)),
            ..Default::default()
        })
        .into()
}

fn breadcrumbs<'a>(
    case: &'a TelemetryCase,
    rep_run: Option<&'a TelemetryRun>,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let crumb_text = |value: String, color: Color| -> Element<'a, TelemetryMsg> {
        text(value)
            .font(theme::FONT_BODY)
            .size(theme::TELEMETRY_BREADCRUMB_SIZE)
            .color(color)
            .into()
    };

    let run_label = flutter_ellipsis(
        run_display_label(rep_run, &case.run_key),
        CRUMB_MAX_CHARS,
        None,
    );
    let mut items: Vec<Element<'a, TelemetryMsg>> = vec![
        crumb_text(case.project.clone(), p.text2),
        crumb_text(labels::TELEMETRY_CRUMB_SEP.to_string(), p.text4),
        crumb_text(run_label, p.text2),
    ];

    if let Some(run) = rep_run {
        let src_cmd = match run.source {
            TelemetryRunSource::App => format!("{} {}", labels::TELEMETRY_SRC_APP, run.command),
            TelemetryRunSource::Task => labels::TELEMETRY_SRC_TASK.to_string(),
            TelemetryRunSource::Wrapper => {
                format!("{} {}", labels::TELEMETRY_SRC_WRAPPER, run.command)
            }
        };
        items.push(crumb_text(labels::TELEMETRY_CRUMB_SEP.to_string(), p.text4));
        items.push(crumb_text(labels::telemetry_run(&run.id), p.text3));
        items.push(crumb_text(labels::TELEMETRY_CRUMB_DOT.to_string(), p.text4));
        items.push(crumb_text(
            flutter_ellipsis(&src_cmd, CRUMB_MAX_CHARS, None),
            p.text3,
        ));
        if let Some(pane) = run.pane_id.as_ref() {
            items.push(crumb_text(labels::TELEMETRY_CRUMB_DOT.to_string(), p.text4));
            items.push(crumb_text(pane.clone(), p.text3));
        }
    }

    row(items).wrap().into()
}

fn kind_pill<'a>(err_type: &str, is_warn: bool) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let color = if is_warn { p.warn } else { p.error };
    container(
        text(err_type.to_uppercase())
            .font(theme::FONT_BODY)
            .size(theme::TELEMETRY_KIND_PILL_TEXT)
            .color(color),
    )
    .padding([
        theme::TELEMETRY_KIND_PILL_PADDING[0],
        theme::TELEMETRY_KIND_PILL_PADDING[1],
    ])
    .style(move |_| container::Style {
        background: Some(Background::Color(theme::with_alpha(color, 0.15))),
        border: Border {
            radius: theme::TELEMETRY_KIND_PILL_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn meta_wrap<'a>(
    case: &'a TelemetryCase,
    blame: Option<&'a TelemetryBlame>,
    now: i64,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let code_pill = container(
        text(case.short_id())
            .font(theme::FONT_MONO)
            .size(theme::TELEMETRY_CODE_PILL_TEXT)
            .color(p.text2),
    )
    .padding([
        theme::TELEMETRY_CODE_PILL_PADDING[0],
        theme::TELEMETRY_CODE_PILL_PADDING[1],
    ])
    .style(|_| {
        let p = theme::palette();
        container::Style {
            background: Some(Background::Color(p.panel2)),
            border: Border {
                color: p.border,
                width: 1.0,
                radius: theme::TELEMETRY_CODE_PILL_RADIUS.into(),
            },
            ..Default::default()
        }
    });

    let (status_label, status_color) = match case.status {
        TelemetryTriageStatus::Open => (labels::TELEMETRY_STATUS_OPEN, p.error),
        TelemetryTriageStatus::Resolved => (labels::TELEMETRY_STATUS_RESOLVED, p.ok),
        TelemetryTriageStatus::Ignored => (labels::TELEMETRY_STATUS_IGNORED, p.text4),
    };

    let status_row = row![
        container(Space::new())
            .width(theme::TELEMETRY_STATUS_DOT)
            .height(theme::TELEMETRY_STATUS_DOT)
            .style(move |_| container::Style {
                background: Some(Background::Color(status_color)),
                border: Border {
                    radius: (theme::TELEMETRY_STATUS_DOT / 2.0).into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        text(status_label)
            .font(theme::FONT_BODY)
            .size(theme::TELEMETRY_STATUS_TEXT)
            .color(p.text2),
    ]
    .spacing(6)
    .align_y(Alignment::Center);

    let loc = case
        .location
        .as_deref()
        .unwrap_or(labels::TELEMETRY_EMPTY_SET);
    let hint = text(labels::telemetry_fingerprint_hint(loc))
        .font(theme::FONT_BODY)
        .size(theme::TELEMETRY_FINGERPRINT_HINT_SIZE)
        .color(p.text3);

    let mut items: Vec<Element<'a, TelemetryMsg>> =
        vec![code_pill.into(), status_row.into(), hint.into()];
    if let Some(b) = blame {
        items.push(blame_chip(b, now));
    }

    row(items).spacing(14).wrap().vertical_spacing(8).into()
}

fn blame_chip<'a>(blame: &TelemetryBlame, now: i64) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let label = match &blame.sha {
        None => labels::TELEMETRY_BLAME_UNCOMMITTED.to_string(),
        Some(sha) => {
            let ago = blame
                .at_ms
                .map(|ms| format_relative_ms(ms, now))
                .unwrap_or_else(|| "?".to_string());
            labels::telemetry_blame_commit(&ago, sha)
        }
    };
    container(
        row![
            text(labels::ICON_EDIT)
                .font(theme::FONT_ICON)
                .size(theme::TELEMETRY_BLAME_ICON)
                .color(p.edited),
            text(label)
                .font(theme::FONT_BODY)
                .size(theme::TELEMETRY_BLAME_TEXT)
                .color(p.edited),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([
        theme::TELEMETRY_BLAME_PADDING[0],
        theme::TELEMETRY_BLAME_PADDING[1],
    ])
    .style(|_| {
        let p = theme::palette();
        container::Style {
            background: Some(Background::Color(p.edited_bg)),
            border: Border {
                color: theme::with_alpha(p.edited, 0.35),
                width: 1.0,
                radius: theme::TELEMETRY_BLAME_RADIUS.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

fn action_bar<'a>(
    state: &'a TelemetryState,
    case: &'a TelemetryCase,
    rep_run: Option<&'a TelemetryRun>,
) -> Element<'a, TelemetryMsg> {
    let mut items: Vec<Element<'a, TelemetryMsg>> = Vec::new();
    let fp = case.fingerprint.clone();
    let short_id = case.short_id();

    if let Some(loc) = case.location.as_deref() {
        let (path, line, _) = split_location(loc);
        let lbl = labels::telemetry_open_file(loc);
        items.push(action_btn(
            labels::ICON_OPEN_IN_NEW,
            lbl.clone(),
            lbl,
            None,
            TelemetryMsg::OpenLocation { path, line },
            BtnStyle::Primary,
        ));
    }

    if let Some(pane_id) = rep_run.and_then(|r| r.pane_id.as_ref()) {
        items.push(action_btn(
            labels::ICON_TERMINAL,
            labels::TELEMETRY_SHOW_IN_TERMINAL.to_string(),
            labels::TELEMETRY_SHOW_IN_TERMINAL.to_string(),
            None,
            TelemetryMsg::ShowInTerminal(pane_id.clone()),
            BtnStyle::Normal,
        ));
    }

    let copied = state.copied_fingerprint.as_deref() == Some(fp.as_str());
    let cli_cmd = format!("gsh telemetry show {short_id}");
    items.push(action_btn(
        if copied {
            labels::ICON_CHECK
        } else {
            labels::ICON_CONTENT_COPY
        },
        labels::TELEMETRY_COPY_COMMAND.to_string(),
        if copied {
            labels::TELEMETRY_COPIED.to_string()
        } else {
            cli_cmd.clone()
        },
        Some(cli_cmd),
        TelemetryMsg::CopyCliCommand {
            fingerprint: fp.clone(),
            short_id,
        },
        BtnStyle::Normal,
    ));

    if case.status == TelemetryTriageStatus::Open {
        items.push(action_btn(
            labels::ICON_CHECK,
            labels::TELEMETRY_RESOLVE.to_string(),
            labels::TELEMETRY_RESOLVE.to_string(),
            None,
            TelemetryMsg::ResolveCase(fp.clone()),
            BtnStyle::Normal,
        ));
        items.push(action_btn(
            labels::ICON_VISIBILITY_OFF,
            labels::TELEMETRY_IGNORE.to_string(),
            labels::TELEMETRY_IGNORE.to_string(),
            None,
            TelemetryMsg::IgnoreCase(fp.clone()),
            BtnStyle::Ghost,
        ));
    } else {
        items.push(action_btn(
            labels::ICON_REFRESH,
            labels::TELEMETRY_REOPEN.to_string(),
            labels::TELEMETRY_REOPEN.to_string(),
            None,
            TelemetryMsg::ReopenCase(fp.clone()),
            BtnStyle::Normal,
        ));
    }

    items.push(action_btn(
        labels::ICON_DELETE_OUTLINE,
        labels::TELEMETRY_CLEAR.to_string(),
        labels::TELEMETRY_CLEAR.to_string(),
        None,
        TelemetryMsg::ClearCase(fp),
        BtnStyle::Ghost,
    ));

    row(items).spacing(8).wrap().vertical_spacing(8).into()
}

#[derive(Clone, Copy)]
enum BtnStyle {
    Primary,
    Normal,
    Ghost,
}

fn action_btn<'a>(
    glyph: &'static str,
    tip: String,
    text_label: String,
    mono: Option<String>,
    msg: TelemetryMsg,
    kind: BtnStyle,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let fg = match kind {
        BtnStyle::Primary => theme::on_color(p.accent),
        BtnStyle::Normal | BtnStyle::Ghost => p.text2,
    };
    let label_el = if mono.is_some() {
        text(text_label)
            .font(theme::FONT_MONO)
            .size(theme::TELEMETRY_ACTION_BTN_MONO_TEXT)
            .color(p.accent_text)
    } else {
        text(text_label)
            .font(theme::FONT_BODY)
            .size(theme::TELEMETRY_ACTION_BTN_TEXT)
            .color(fg)
    };

    let btn = button(
        row![
            text(glyph)
                .font(theme::FONT_ICON)
                .size(theme::TELEMETRY_ACTION_BTN_ICON)
                .color(fg),
            label_el,
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .padding([
        theme::TELEMETRY_ACTION_BTN_PADDING[0],
        theme::TELEMETRY_ACTION_BTN_PADDING[1],
    ])
    .on_press(msg)
    .style(move |_: &Theme, status| {
        let p = theme::palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        match kind {
            BtnStyle::Primary => {
                let bg = if hovered {
                    theme::with_alpha(p.accent, 0.85)
                } else {
                    p.accent
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: theme::on_color(p.accent),
                    border: Border {
                        color: p.accent,
                        width: 1.0,
                        radius: theme::TELEMETRY_ACTION_BTN_RADIUS.into(),
                    },
                    ..Default::default()
                }
            }
            BtnStyle::Normal => button::Style {
                background: Some(Background::Color(if hovered { p.panel3 } else { p.panel2 })),
                text_color: p.text2,
                border: Border {
                    color: p.border2,
                    width: 1.0,
                    radius: theme::TELEMETRY_ACTION_BTN_RADIUS.into(),
                },
                ..Default::default()
            },
            BtnStyle::Ghost => button::Style {
                background: hovered.then_some(Background::Color(p.panel3)),
                text_color: p.text2,
                border: Border {
                    color: p.border2,
                    width: 1.0,
                    radius: theme::TELEMETRY_ACTION_BTN_RADIUS.into(),
                },
                ..Default::default()
            },
        }
    });

    tooltip(btn, pane_chrome::tip_bubble(tip), tooltip::Position::Top).into()
}

fn stats_bar<'a>(
    case: &'a TelemetryCase,
    rep_run: Option<&'a TelemetryRun>,
    now: i64,
) -> Element<'a, TelemetryMsg> {
    let origin_val = match rep_run {
        None => labels::TELEMETRY_DASH.to_string(),
        Some(r) => match r.source {
            TelemetryRunSource::App => labels::TELEMETRY_SRC_APP.to_string(),
            TelemetryRunSource::Task => {
                let name = r.name.as_deref().unwrap_or("");
                flutter_ellipsis(&format!("{} {name}", labels::TELEMETRY_SRC_TASK), 22, None)
            }
            TelemetryRunSource::Wrapper => labels::TELEMETRY_SRC_WRAPPER.to_string(),
        },
    };
    let stream_label = if rep_run.is_some_and(|r| r.source == TelemetryRunSource::Task) {
        labels::TELEMETRY_PTY
    } else {
        labels::TELEMETRY_STDOUT
    };

    let bar = row![
        stat_cell(
            labels::TELEMETRY_OCCURRENCES,
            case.count.to_string(),
            Some(labels::telemetry_in_runs(case.runs.len())),
        ),
        vertical_divider(),
        stat_cell(
            labels::TELEMETRY_FIRST,
            format_relative_ms(case.first_at_ms, now),
            None,
        ),
        vertical_divider(),
        stat_cell(
            labels::TELEMETRY_LAST,
            format_relative_ms(case.last_at_ms, now),
            None,
        ),
        vertical_divider(),
        stat_cell(
            labels::TELEMETRY_ORIGIN,
            origin_val,
            Some(stream_label.to_string()),
        ),
    ]
    .width(Fill)
    .align_y(Alignment::Start);

    container(bar)
        .width(Fill)
        .style(|_| {
            let p = theme::palette();
            container::Style {
                background: Some(Background::Color(p.panel2)),
                border: Border {
                    color: p.border,
                    width: 1.0,
                    radius: theme::TELEMETRY_CARD_RADIUS.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

fn stat_cell<'a>(
    label: &'static str,
    value: String,
    small: Option<String>,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let mut val_row = row![text(value)
        .font(theme::FONT_TITLE)
        .size(theme::TELEMETRY_STATS_BIG_SIZE)
        .color(p.text)]
    .align_y(Alignment::End);

    if let Some(sub) = small {
        val_row = val_row.push(Space::new().width(4)).push(
            text(sub)
                .font(theme::FONT_BODY)
                .size(theme::TELEMETRY_STATS_SUB_SIZE)
                .color(p.text3),
        );
    }

    container(
        column![
            text(label.to_uppercase())
                .font(theme::FONT_BODY)
                .size(theme::TELEMETRY_STATS_LABEL_SIZE)
                .color(p.text3),
            val_row,
        ]
        .spacing(3),
    )
    .width(Length::FillPortion(1))
    .padding([
        theme::TELEMETRY_STATS_CELL_PADDING[0],
        theme::TELEMETRY_STATS_CELL_PADDING[1],
    ])
    .into()
}

fn vertical_divider<'a>() -> Element<'a, TelemetryMsg> {
    container(Space::new().width(1).height(56))
        .width(1)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        })
        .into()
}

fn stack_section<'a>(last_event: Option<&'a TelemetryEvent>) -> Element<'a, TelemetryMsg> {
    let frames: &[TelemetryFrame] = last_event
        .and_then(|ev| ev.error.as_ref())
        .map_or(&[], |err| err.frames.as_slice());

    let body: Element<'a, TelemetryMsg> = if frames.is_empty() {
        empty_card(labels::TELEMETRY_NONE)
    } else {
        let mut list = column![];
        let mut seen_first_project = false;
        for frame in frames {
            let top = frame.in_project && !seen_first_project;
            if frame.in_project {
                seen_first_project = true;
            }
            list = list.push(frame_row(frame, top));
        }
        card(list.width(Fill).into(), [8.0, 0.0].into())
    };

    section(
        labels::TELEMETRY_SECTION_STACK,
        labels::TELEMETRY_STACK_HINT,
        body,
    )
}

fn frame_row<'a>(frame: &'a TelemetryFrame, top: bool) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let base_text = frame.raw.trim();
    let display = if base_text.is_empty() {
        frame.location().unwrap_or_default()
    } else {
        base_text.to_string()
    };
    let text_color = if frame.in_project { p.text } else { p.text3 };
    let pad = Padding {
        top: 2.0,
        right: 14.0,
        bottom: 2.0,
        left: if top { 12.0 } else { 14.0 },
    };

    let row_el: Element<'a, TelemetryMsg> = if frame.in_project {
        if let Some(loc) = frame.location() {
            let (path, line, _) = split_location(&loc);
            let mut r = row![text(display)
                .font(theme::FONT_MONO)
                .size(theme::TELEMETRY_FRAME_LINE_SIZE)
                .line_height(theme::TELEMETRY_FRAME_LINE_HEIGHT)
                .color(text_color)]
            .spacing(8)
            .align_y(Alignment::Center);
            r = r.push(
                text(loc)
                    .font(theme::FONT_MONO)
                    .size(theme::TELEMETRY_FRAME_LINE_SIZE)
                    .line_height(theme::TELEMETRY_FRAME_LINE_HEIGHT)
                    .color(p.accent_text),
            );
            button(r)
                .width(Fill)
                .padding(pad)
                .on_press(TelemetryMsg::OpenLocation { path, line })
                .style(move |_: &Theme, status| {
                    let p = theme::palette();
                    let hovered =
                        matches!(status, button::Status::Hovered | button::Status::Pressed);
                    let bg = if hovered {
                        Some(Background::Color(p.panel2))
                    } else if top {
                        Some(Background::Color(theme::with_alpha(p.error, 0.08)))
                    } else {
                        None
                    };
                    button::Style {
                        background: bg,
                        text_color,
                        border: Border::default(),
                        ..Default::default()
                    }
                })
                .into()
        } else {
            container(
                text(display)
                    .font(theme::FONT_MONO)
                    .size(theme::TELEMETRY_FRAME_LINE_SIZE)
                    .line_height(theme::TELEMETRY_FRAME_LINE_HEIGHT)
                    .color(text_color),
            )
            .width(Fill)
            .padding(pad)
            .into()
        }
    } else {
        container(
            text(display)
                .font(theme::FONT_MONO)
                .size(theme::TELEMETRY_FRAME_LINE_SIZE)
                .line_height(theme::TELEMETRY_FRAME_LINE_HEIGHT)
                .color(text_color),
        )
        .width(Fill)
        .padding(pad)
        .into()
    };

    if !top {
        return row_el;
    }

    row![
        container(Space::new().width(2).height(Fill))
            .width(2)
            .style(|_| container::Style {
                background: Some(Background::Color(theme::palette().error)),
                ..Default::default()
            }),
        row_el,
    ]
    .width(Fill)
    .into()
}

fn correlated_log_section<'a>(log: Option<&'a TelemetryEvent>) -> Element<'a, TelemetryMsg> {
    let body: Element<'a, TelemetryMsg> = match log {
        None => empty_card(labels::TELEMETRY_NONE),
        Some(ev) => {
            let p = theme::palette();
            let mut entries: Vec<(&str, String, bool)> =
                vec![("msg", format!("\"{}\"", ev.body), true)];
            for (k, v) in &ev.attrs {
                let (s, is_str) = match v {
                    serde_json::Value::String(text_val) => (format!("\"{text_val}\""), true),
                    other => (other.to_string(), false),
                };
                entries.push((k.as_str(), s, is_str));
            }
            let max_key = entries
                .iter()
                .map(|(k, _, _)| k.chars().count())
                .max()
                .unwrap_or(4)
                .clamp(4, 24);
            let key_w = (max_key as f32 * 7.5) + 14.0;

            let mut table = column![].spacing(4);
            for (k, val_str, is_str) in entries {
                table = table.push(
                    row![
                        container(
                            text(k)
                                .font(theme::FONT_MONO)
                                .size(theme::TELEMETRY_LOG_ROW_SIZE)
                                .color(p.text3),
                        )
                        .width(key_w),
                        text(val_str)
                            .font(theme::FONT_MONO)
                            .size(theme::TELEMETRY_LOG_ROW_SIZE)
                            .color(if is_str { p.ok } else { p.warn }),
                    ]
                    .align_y(Alignment::Start),
                );
            }
            card(
                table.width(Fill).into(),
                [
                    theme::TELEMETRY_CARD_PADDING_V,
                    theme::TELEMETRY_CARD_PADDING_H,
                ]
                .into(),
            )
        }
    };

    section(
        labels::TELEMETRY_SECTION_CORRELATED,
        labels::TELEMETRY_CORRELATED_HINT,
        body,
    )
}

fn runs_section<'a>(
    case: &'a TelemetryCase,
    runs_by_id: Option<&'a std::collections::HashMap<String, TelemetryRun>>,
    now: i64,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let max_count = case.runs.iter().map(|r| r.count).max().unwrap_or(1).max(1);
    let mut rows = column![];
    for (idx, cr) in case.runs.iter().enumerate() {
        let is_current = idx == 0;
        if idx > 0 {
            rows = rows.push(panel_widgets::hairline_h());
        }
        let exit_code = runs_by_id
            .and_then(|map| map.get(&cr.run_id))
            .and_then(|r| r.exit_code);
        let left_label = flutter_ellipsis(&labels::telemetry_run(&cr.run_id), 14, None);

        let mut left_col = row![text(left_label)
            .font(theme::FONT_MONO)
            .size(12.0)
            .color(p.text2)]
        .align_y(Alignment::Center);

        if is_current {
            left_col = left_col
                .push(Space::new().width(8))
                .push(telemetry_tag(labels::TELEMETRY_CURRENT, TagKind::Fresh));
        }

        let fill_pct = ((cr.count as f32 / max_count as f32) * 100.0).clamp(4.0, 100.0);
        let fill_part = fill_pct.round() as u16;
        let rem_part = 100_u16.saturating_sub(fill_part);

        let bar_track = container(
            row![
                container(Space::new())
                    .width(Length::FillPortion(fill_part.max(1)))
                    .height(theme::TELEMETRY_RUN_BAR_HEIGHT)
                    .style(|_| container::Style {
                        background: Some(Background::Color(theme::palette().error)),
                        border: Border {
                            radius: theme::TELEMETRY_RUN_BAR_RADIUS.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                Space::new().width(Length::FillPortion(rem_part.max(1))),
            ]
            .width(Fill),
        )
        .width(Fill)
        .height(theme::TELEMETRY_RUN_BAR_HEIGHT)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::palette().panel3)),
            border: Border {
                radius: theme::TELEMETRY_RUN_BAR_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let mut meta = format_relative_ms(cr.first_at_ms, now);
        if cr.live {
            meta.push_str(" →");
        }
        if let Some(code) = exit_code {
            meta.push_str(&format!(" · exit {code}"));
        }

        rows = rows.push(
            container(
                row![
                    container(left_col).width(theme::TELEMETRY_RUN_BAR_LABEL_WIDTH),
                    bar_track,
                    Space::new().width(14),
                    text(format!("×{}", cr.count))
                        .font(theme::FONT_MONO)
                        .size(12.0)
                        .color(p.text2),
                    Space::new().width(8),
                    text(meta).font(theme::FONT_BODY).size(11.5).color(p.text3),
                ]
                .align_y(Alignment::Center),
            )
            .width(Fill)
            .padding([9.0, 14.0]),
        );
    }

    section(
        labels::TELEMETRY_SECTION_RUNS,
        labels::TELEMETRY_RUNS_HINT,
        card(rows.width(Fill).into(), Padding::ZERO),
    )
}

fn context_section<'a>(
    lines: &'a [TelemetryEvent],
    target_id: Option<&'a str>,
) -> Element<'a, TelemetryMsg> {
    let body: Element<'a, TelemetryMsg> = if lines.is_empty() {
        empty_card(labels::TELEMETRY_NONE)
    } else {
        let p = theme::palette();
        let mut list = column![];
        for ev in lines {
            let hit = target_id.is_some() && ev.id.as_deref() == target_id;
            let is_json = !ev.attrs.is_empty() && ev.error.is_none();
            let line_no = ev
                .raw_line_offset
                .map(|o| (o + 1).to_string())
                .unwrap_or_default();
            let line_color = if hit {
                p.text
            } else if is_json {
                p.accent_text
            } else {
                p.text3
            };
            let body_text = flutter_ellipsis(&ev.body, CONTEXT_BODY_MAX_CHARS, Some(3));

            let line_row = container(
                row![
                    container(
                        text(line_no)
                            .font(theme::FONT_MONO)
                            .size(theme::TELEMETRY_CONTEXT_LINE_SIZE)
                            .line_height(theme::TELEMETRY_CONTEXT_LINE_HEIGHT)
                            .color(p.text4),
                    )
                    .width(theme::TELEMETRY_CONTEXT_LINE_NUM_WIDTH),
                    text(body_text)
                        .font(theme::FONT_MONO)
                        .size(theme::TELEMETRY_CONTEXT_LINE_SIZE)
                        .line_height(theme::TELEMETRY_CONTEXT_LINE_HEIGHT)
                        .color(line_color),
                ]
                .align_y(Alignment::Start),
            )
            .width(Fill)
            .padding(Padding {
                top: 0.0,
                right: 14.0,
                bottom: 0.0,
                left: if hit { 12.0 } else { 14.0 },
            })
            .style(move |_| container::Style {
                background: hit.then_some(Background::Color(theme::with_alpha(
                    theme::palette().error,
                    0.08,
                ))),
                ..Default::default()
            });

            if hit {
                list = list.push(
                    row![
                        container(Space::new().width(2).height(Fill))
                            .width(2)
                            .style(|_| container::Style {
                                background: Some(Background::Color(theme::palette().error)),
                                ..Default::default()
                            }),
                        line_row,
                    ]
                    .width(Fill),
                );
            } else {
                list = list.push(line_row);
            }
        }
        card(list.width(Fill).into(), [8.0, 0.0].into())
    };

    section(
        labels::TELEMETRY_SECTION_CONTEXT,
        labels::TELEMETRY_CONTEXT_HINT,
        body,
    )
}

fn empty_card<'a>(label: &'static str) -> Element<'a, TelemetryMsg> {
    card(
        text(label)
            .font(theme::FONT_MONO)
            .size(12.0)
            .color(theme::palette().text4)
            .into(),
        [
            theme::TELEMETRY_CARD_PADDING_V,
            theme::TELEMETRY_CARD_PADDING_H,
        ]
        .into(),
    )
}

fn section<'a>(
    title: &'static str,
    hint: &'static str,
    body: Element<'a, TelemetryMsg>,
) -> Element<'a, TelemetryMsg> {
    let p = theme::palette();
    let header = row![
        text(title.to_uppercase())
            .font(theme::FONT_TITLE)
            .size(theme::TELEMETRY_SECTION_TITLE_SIZE)
            .color(p.text3),
        Space::new().width(10),
        container(
            text(hint)
                .font(theme::FONT_BODY)
                .size(11.5)
                .color(p.text4)
                .wrapping(text::Wrapping::None),
        )
        .clip(true),
    ]
    .align_y(Alignment::Center);

    column![header, body]
        .spacing(theme::TELEMETRY_VIEW_HEADER_GAP)
        .width(Fill)
        .into()
}

fn card<'a>(content: Element<'a, TelemetryMsg>, padding: Padding) -> Element<'a, TelemetryMsg> {
    container(content)
        .width(Fill)
        .padding(padding)
        .style(|_| {
            let p = theme::palette();
            container::Style {
                background: Some(Background::Color(p.bg)),
                border: Border {
                    color: p.border,
                    width: 1.0,
                    radius: theme::TELEMETRY_CARD_RADIUS.into(),
                },
                ..Default::default()
            }
        })
        .into()
}
