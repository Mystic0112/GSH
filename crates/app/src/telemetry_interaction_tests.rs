//! Testes headless (`iced_test::simulator`, tiny-skia) para a aba Telemetria
//! (`telemetry_panel`), visão de caso (`telemetry_case_view`) e integração no `Shell`.

use std::collections::HashMap;
use std::path::PathBuf;

use gsh_core::workspace::{Workspace, WorkspaceList};
use gsh_telemetry::{
    now_ms, TelemetryCase, TelemetryCaseRun, TelemetryError, TelemetryEvent, TelemetryFrame,
    TelemetryRun, TelemetryRunSource, TelemetrySeverity, TelemetryStream, TelemetryTriageStatus,
};
use iced::{Element, Size};
use iced_test::simulator::{click, Simulator};
use serde_json::{Map, Value};

use crate::pane_model::PaneTab;
use crate::right_tabs::RightTab;
use crate::telemetry_state::{
    TelemetryBlame, TelemetryCaseDetail, TelemetryFilter, TelemetryMsg, TelemetryState,
};
use crate::{labels, telemetry_case_view, telemetry_panel, theme};

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

fn sample_run() -> TelemetryRun {
    TelemetryRun {
        id: "r_1".to_string(),
        key: "cmd:/proj::pytest".to_string(),
        project: "/proj".to_string(),
        source: TelemetryRunSource::Wrapper,
        cwd: "/proj".to_string(),
        command: "pytest".to_string(),
        name: None,
        task_key: None,
        pane_id: Some("t1".to_string()),
        pid: Some(4242),
        started_at_ms: now_ms() - 12_000,
        ended_at_ms: None,
        exit_code: None,
    }
}

fn sample_case(status: TelemetryTriageStatus) -> TelemetryCase {
    let now = now_ms();
    TelemetryCase {
        fingerprint: "abcd1234ef567890".to_string(),
        run_key: "cmd:/proj::pytest".to_string(),
        project: "/proj".to_string(),
        severity: TelemetrySeverity::Error,
        err_type: "PanicError".to_string(),
        message: "connection refused on port 5432".to_string(),
        location: Some("src/main.rs:42".to_string()),
        count: 3,
        first_at_ms: now - 30_000,
        last_at_ms: now - 2_000,
        status,
        triage: None,
        is_new: true,
        is_regression: false,
        runs: vec![TelemetryCaseRun {
            run_id: "r_1".to_string(),
            count: 3,
            first_at_ms: now - 30_000,
            last_at_ms: now - 2_000,
            live: true,
        }],
        last_event_id: Some("ev-1".to_string()),
    }
}

fn sample_detail(status: TelemetryTriageStatus) -> TelemetryCaseDetail {
    let now = now_ms();
    let run = sample_run();
    let mut runs_by_id = HashMap::new();
    runs_by_id.insert(run.id.clone(), run.clone());

    let mut log_attrs = Map::new();
    log_attrs.insert(
        "request_id".to_string(),
        Value::String("req_99".to_string()),
    );
    log_attrs.insert(
        "route".to_string(),
        Value::String("/api/orders".to_string()),
    );

    let correlated_log = TelemetryEvent {
        id: Some("ev-0".to_string()),
        run_id: "r_1".to_string(),
        ts_ms: now - 2_500,
        stream: TelemetryStream::Out,
        severity: TelemetrySeverity::Info,
        body: "handling request".to_string(),
        attrs: log_attrs,
        error: None,
        fingerprint: None,
        trace_id: None,
        span_id: None,
        request_id: Some("req_99".to_string()),
        raw_line_offset: Some(0),
        repeat: 1,
    };

    let last_event = TelemetryEvent {
        id: Some("ev-1".to_string()),
        run_id: "r_1".to_string(),
        ts_ms: now - 2_000,
        stream: TelemetryStream::Err,
        severity: TelemetrySeverity::Error,
        body: "PanicError: connection refused on port 5432".to_string(),
        attrs: Map::new(),
        error: Some(TelemetryError {
            err_type: "PanicError".to_string(),
            message: "connection refused on port 5432".to_string(),
            stack: None,
            frames: vec![
                TelemetryFrame {
                    raw: "at worker (src/worker.rs:88:5)".to_string(),
                    file: Some("src/worker.rs".to_string()),
                    line: Some(88),
                    column: Some(5),
                    function: Some("worker".to_string()),
                    in_project: true,
                },
                TelemetryFrame {
                    raw: "at std::rt::lang_start".to_string(),
                    file: None,
                    line: None,
                    column: None,
                    function: Some("std::rt::lang_start".to_string()),
                    in_project: false,
                },
            ],
        }),
        fingerprint: Some("abcd1234ef567890".to_string()),
        trace_id: None,
        span_id: None,
        request_id: None,
        raw_line_offset: Some(1),
        repeat: 1,
    };

    TelemetryCaseDetail {
        case: sample_case(status),
        last_event: Some(last_event.clone()),
        correlated_log: Some(correlated_log.clone()),
        context: vec![correlated_log, last_event],
        runs_by_id,
        rep_run: Some(run),
        blame: Some(TelemetryBlame {
            sha: Some("deadbee".to_string()),
            at_ms: Some(now - 3_600_000),
            author: Some("Dev".to_string()),
        }),
    }
}

fn populated_state(status: TelemetryTriageStatus) -> TelemetryState {
    let mut state = TelemetryState::in_memory();
    state.workspace_id = Some("ws-1".to_string());
    state.workspace_name = Some("proj".to_string());
    state.workspace_root = Some(PathBuf::from("/proj"));
    state.workspace_roots = vec![PathBuf::from("/proj")];
    state.developer_mode = true;
    state.db_size_bytes = 4096;
    state.runs = vec![sample_run()];
    state.cases = vec![sample_case(status)];
    state
        .case_details
        .insert("abcd1234ef567890".to_string(), sample_detail(status));
    state
}

#[test]
fn panel_clicks_chips_run_toggle_search_and_case_row() {
    let state = populated_state(TelemetryTriageStatus::Open);

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    ui.click(labels::TELEMETRY_CHIP_NEW).expect("chip Novos");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::SetFilter(TelemetryFilter::New))));

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    ui.click(labels::TELEMETRY_CHIP_WARNINGS_WITH_ICON)
        .expect("chip Avisos");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::ToggleWarnings)));

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    ui.click(labels::TELEMETRY_CHIP_APP)
        .expect("chip GSH Desktop");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::ToggleAppSource)));

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    ui.click("pytest").expect("header do run");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::ToggleRun(k) if k == "r:/proj:cmd:/proj::pytest")));

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    assert!(ui.find("connection refused on p…").is_ok());
    ui.click("src/main.rs:42").expect("linha do caso");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::OpenCase(fp) if fp == "abcd1234ef567890")));

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    ui.click(iced::widget::Id::new(telemetry_panel::SEARCH_INPUT_ID))
        .expect("campo de busca");
    let _ = ui.typewrite("timeout");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::SearchChanged(q) if !q.is_empty())));
}

#[test]
fn panel_quick_actions_when_hovered_emit_triage_and_clear() {
    let mut state = populated_state(TelemetryTriageStatus::Open);
    state.hovered_case = Some("abcd1234ef567890".to_string());

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    ui.click(labels::ICON_CHECK).expect("ação rápida resolver");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::ResolveCase(fp) if fp == "abcd1234ef567890")));

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    ui.click(labels::ICON_VISIBILITY_OFF)
        .expect("ação rápida ignorar");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::IgnoreCase(fp) if fp == "abcd1234ef567890")));

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    ui.click("connection refused on port 5432").ok();
    let _ = ui.into_messages().count();

    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    // O segundo botão de lixeira pertence ao caso em hover; o primeiro é do cabeçalho do run.
    ui.click(labels::ICON_DELETE_OUTLINE).expect("ação limpar");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::ClearRun(_) | TelemetryMsg::ClearCase(_))));
}

#[test]
fn case_view_buttons_and_project_stack_frame_clicks() {
    let state = populated_state(TelemetryTriageStatus::Open);
    let fp = "abcd1234ef567890";

    let mut ui = simulator(
        Size::new(980.0, 900.0),
        telemetry_case_view::view(&state, fp),
    );
    ui.click(labels::telemetry_open_file("src/main.rs:42"))
        .expect("botão abrir arquivo");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs.iter().any(|m| matches!(
        m,
        TelemetryMsg::OpenLocation {
            path,
            line: Some(42)
        } if path == "src/main.rs"
    )));

    let mut ui = simulator(
        Size::new(980.0, 900.0),
        telemetry_case_view::view(&state, fp),
    );
    ui.click("gsh telemetry show e_abcd")
        .expect("copiar comando da CLI");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs.iter().any(
        |m| matches!(m, TelemetryMsg::CopyCliCommand { fingerprint, .. } if fingerprint == fp)
    ));

    let mut ui = simulator(
        Size::new(980.0, 900.0),
        telemetry_case_view::view(&state, fp),
    );
    ui.click(labels::TELEMETRY_SHOW_IN_TERMINAL)
        .expect("ver no terminal");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::ShowInTerminal(p) if p == "t1")));

    let mut ui = simulator(
        Size::new(980.0, 900.0),
        telemetry_case_view::view(&state, fp),
    );
    ui.click(labels::TELEMETRY_RESOLVE)
        .expect("marcar resolvido");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::ResolveCase(id) if id == fp)));

    let mut ui = simulator(
        Size::new(980.0, 900.0),
        telemetry_case_view::view(&state, fp),
    );
    ui.click("src/worker.rs:88")
        .expect("frame clicável do projeto");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs.iter().any(|m| matches!(
        m,
        TelemetryMsg::OpenLocation {
            path,
            line: Some(88)
        } if path == "src/worker.rs"
    )));
}

#[test]
fn case_view_resolved_shows_reopen_button() {
    let state = populated_state(TelemetryTriageStatus::Resolved);
    let fp = "abcd1234ef567890";
    let mut ui = simulator(
        Size::new(980.0, 900.0),
        telemetry_case_view::view(&state, fp),
    );
    ui.click(labels::TELEMETRY_REOPEN).expect("botão Reabrir");
    let msgs: Vec<_> = ui.into_messages().collect();
    assert!(msgs
        .iter()
        .any(|m| matches!(m, TelemetryMsg::ReopenCase(id) if id == fp)));
    let _ = click;
}

#[test]
fn multiline_run_command_applies_flutter_ellipsis_in_panel_and_renders_dart_detail_sections() {
    let mut state = populated_state(TelemetryTriageStatus::Open);
    let multi = "sh -c '\necho a\nprint(\"WARN [cache] slow miss\", file=sys.stderr)\ndef load_user():\n    raise RuntimeError(\"failed\")\nload_user()\n'";
    state.runs[0].ended_at_ms = Some(now_ms());
    state.runs[0].command = multi.to_string();
    if let Some(detail) = state.case_details.get_mut("abcd1234ef567890") {
        detail.rep_run.as_mut().expect("rep run").command = multi.to_string();
        detail.runs_by_id.get_mut("r_1").expect("run map").command = multi.to_string();
    }

    // Painel: mantém primeiras linhas que cabem e corta na primeira linha mais larga com '…', descartando def load_user()
    let mut ui = simulator(Size::new(260.0, 620.0), telemetry_panel::view(&state));
    let rendered = crate::telemetry_state::flutter_ellipsis(multi, 10, None);
    assert!(rendered.starts_with("sh -c '\necho a\n"));
    assert!(rendered.ends_with('…'));
    assert!(ui.find(rendered.as_str()).is_ok());
    assert!(ui.find("def load_user():").is_err());

    // Detalhe: card ORIGEM mostra apenas "gsh telemetry" + "stdout" (não o comando), e Ocorrências por run mostra "run r_1" + "ATUAL"
    let mut detail_ui = simulator(
        Size::new(980.0, 900.0),
        telemetry_case_view::view(&state, "abcd1234ef567890"),
    );
    assert!(detail_ui.find("gsh telemetry").is_ok());
    assert!(detail_ui.find("stdout").is_ok());
    assert!(detail_ui.find("run r_1").is_ok());
    assert!(detail_ui.find("ATUAL").is_ok());
}

#[test]
fn shell_opens_single_case_tab_per_fingerprint_and_gates_right_tab() {
    let mut shell = super::tests::test_shell();
    assert!(shell.right_available().telemetry);

    shell.workspaces = WorkspaceList::new(vec![Workspace::system("sys", "Terminal", None)]);
    assert!(!shell.right_available().telemetry);

    let dir = std::env::temp_dir().join(format!("gsh-telemetry-ws-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    shell.workspaces = WorkspaceList::new(vec![Workspace::from_path("ws-1", &dir)]);
    assert!(shell.right_available().telemetry);

    let _ = shell.update(super::Msg::Telemetry(TelemetryMsg::OpenCase(
        "fp123".to_string(),
    )));
    let _ = shell.update(super::Msg::Telemetry(TelemetryMsg::OpenCase(
        "fp123".to_string(),
    )));
    let count = shell
        .panes
        .focused_model()
        .tabs()
        .iter()
        .filter(
            |t| matches!(t, PaneTab::TelemetryCase { fingerprint, .. } if fingerprint == "fp123"),
        )
        .count();
    assert_eq!(count, 1);

    let _ = shell.update(super::Msg::SelectRightTab(RightTab::Telemetry));
    assert_eq!(shell.right_tab(), RightTab::Telemetry);

    let _ = std::fs::remove_dir_all(dir);
}
