use std::sync::Arc;

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use serde_json::{json, Map, Value};
use tokio::net::TcpListener;

use gsh_telemetry::config::TelemetryProxyConfig;
use gsh_telemetry::db::{b36, fts_query, TelemetryDb};
use gsh_telemetry::fingerprint::{fingerprint_of, normalize_message};
use gsh_telemetry::ingest::TelemetryIngest;
use gsh_telemetry::model::{
    now_ms, TelemetryError, TelemetryEvent, TelemetryFrame, TelemetryQuery, TelemetryRetention,
    TelemetryRun, TelemetryRunSource, TelemetrySeverity, TelemetryStream, TelemetryTaskDef,
    TelemetryTriageActor, TelemetryTriageStatus, TelemetryWorkspace, K_TELEMETRY_APP_WORKSPACE_ID,
};
use gsh_telemetry::otlp::{OtlpReceiver, OtlpSink};
use gsh_telemetry::parser::{TelemetryLineParser, TelemetryParserConfig};
use gsh_telemetry::proxy::HttpTelemetryProxy;
use gsh_telemetry::redaction::redact_attrs;
use gsh_telemetry::registry::TelemetryStoreRegistry;
use gsh_telemetry::{
    strip_comments, TelemetryCliHandler, TelemetryWorkspaceConfig, K_TELEMETRY_REPLY_CAP,
};

const GOLDEN_JSON: &str = include_str!("golden/telemetry_golden.json");

fn event_to_golden_json(e: &TelemetryEvent) -> Value {
    json!({
        "runId": e.run_id,
        "tsMs": e.ts_ms,
        "stream": e.stream.as_str(),
        "severity": e.severity.as_str(),
        "body": e.body,
        "attrs": e.attrs,
        "error": e.error,
        "fingerprint": e.fingerprint,
        "traceId": e.trace_id,
        "spanId": e.span_id,
        "requestId": e.request_id,
        "rawLineOffset": e.raw_line_offset,
        "repeat": e.repeat,
    })
}

fn parse_sample(text: &str, roots: &[&str]) -> Vec<Value> {
    let cfg = TelemetryParserConfig {
        project_roots: roots.iter().map(|s| (*s).to_string()).collect(),
        ..Default::default()
    };
    let mut parser = TelemetryLineParser::new("r_1", cfg);
    let at_ms = 1_758_628_800_000i64;
    let mut out = Vec::new();
    for (idx, line) in text.split('\n').enumerate() {
        out.extend(parser.feed(line, TelemetryStream::Out, at_ms, Some(idx as i64)));
    }
    out.extend(parser.flush());
    out.iter().map(event_to_golden_json).collect()
}

#[test]
fn golden_parity_with_dart() {
    let golden: Value = serde_json::from_str(GOLDEN_JSON).expect("valid golden json");

    // 1. Fingerprints & normalize_message
    for item in golden["fingerprints"].as_array().unwrap() {
        let err_type = item["type"].as_str().unwrap();
        let message = item["message"].as_str().unwrap();
        let loc = item["loc"].as_str();
        let frames = match loc {
            Some(l) => {
                let (f, ln) = l.split_once(':').unwrap();
                vec![TelemetryFrame {
                    raw: String::new(),
                    file: Some(f.to_string()),
                    line: Some(ln.parse().unwrap()),
                    column: None,
                    function: None,
                    in_project: true,
                }]
            }
            None => Vec::new(),
        };
        let err = TelemetryError {
            err_type: err_type.to_string(),
            message: message.to_string(),
            stack: None,
            frames,
        };
        assert_eq!(
            normalize_message(message),
            item["normalized"].as_str().unwrap(),
            "normalize_message mismatch for {message}"
        );
        assert_eq!(
            fingerprint_of(&err),
            item["fingerprint"].as_str().unwrap(),
            "fingerprint mismatch for {message}"
        );
    }

    // 2. Redaction
    let red_in = golden["redaction"]["input"].as_object().unwrap();
    let red_out = redact_attrs(red_in);
    assert_eq!(Value::Object(red_out), golden["redaction"]["output"]);

    // 3. Run keys
    let keys = golden["runKeys"].as_array().unwrap();
    assert_eq!(
        TelemetryRun::key_for("/ws/app", "flutter  run   --debug", None),
        keys[0].as_str().unwrap()
    );
    assert_eq!(
        TelemetryRun::key_for("/ws/app", "flutter run", Some("  api ")),
        keys[1].as_str().unwrap()
    );
    assert_eq!(
        TelemetryRun::key_for("/ws/app", "npm run dev", Some("   ")),
        keys[2].as_str().unwrap()
    );

    // 4. Severities
    for item in golden["severities"].as_array().unwrap() {
        let parsed = TelemetrySeverity::parse_value(Some(&item["raw"]));
        assert_eq!(
            parsed.as_str(),
            item["parsed"].as_str().unwrap(),
            "{item:?}"
        );
    }

    // 5. FTS queries & base36
    for item in golden["ftsQueries"].as_array().unwrap() {
        assert_eq!(
            fts_query(item["input"].as_str().unwrap()),
            item["fts"].as_str().unwrap()
        );
    }
    for item in golden["b36"].as_array().unwrap() {
        assert_eq!(
            b36(item["n"].as_u64().unwrap()),
            item["b36"].as_str().unwrap()
        );
    }

    // 6. JSONC comment stripping
    let jsonc = "{\n  // comment here\n  \"unwrap\": [\"^api-1\\\\s*\\\\| \"],\n  /* block\n     comment */\n  \"proxy\": { \"listen\": 3100, \"upstream\": \"http://127.0.0.1:3000\" }\n}";
    assert_eq!(
        strip_comments(jsonc),
        golden["jsoncStripped"].as_str().unwrap()
    );

    // 7. Line parser samples
    let samples = [
        (
            "ansi_and_alt",
            "\x1b[31mERROR\x1b[0m boom\n\x1b[?1049h(tela de TUI) Error: falso\nainda no alt\n\x1b[?1049l\nde volta",
            &[][..],
        ),
        (
            "stacked_prefixes",
            "api-1  | 2026-09-23T12:00:00.000Z {\"level\":\"warn\",\"msg\":\"slow\"}\nflutter: {\"level\":\"debug\",\"msg\":\"cart add\",\"itemId\":\"sku-88\"}\nI/flutter ( 1234): {\"level\":\"info\",\"msg\":\"android\"}\n[0] listening on :3000",
            &[][..],
        ),
        (
            "pino_json",
            "{\"level\":50,\"time\":1758632475810,\"name\":\"api\",\"err\":{\"type\":\"Error\",\"message\":\"connect ECONNREFUSED 127.0.0.1:5432\",\"code\":\"ECONNREFUSED\",\"stack\":\"Error: connect ECONNREFUSED 127.0.0.1:5432\\n    at TCPConnectWrap.afterConnect [as oncomplete] (node:net:1555:16)\\n    at pool.connect (/Users/j/shop/backend/src/db.ts:12:9)\"},\"msg\":\"db connect failed\"}",
            &["/Users/j/shop/backend"][..],
        ),
        (
            "flutter_exception_and_another",
            "flutter: ══╡ EXCEPTION CAUGHT BY GESTURE ╞═══════════════════════════════════════════════\nflutter: The following RangeError was thrown while handling a gesture:\nflutter: RangeError (index): Index out of range: index 3, length 2\nflutter:\nflutter: When the exception was thrown, this was the stack:\nflutter: #0      List.[] (dart:core-patch/growable_array.dart:264:36)\nflutter: #1      CartService.add (package:app/cart/cart_service.dart:87:22)\nflutter: #2      CartBadge._onTap (package:app/cart/cart_badge.dart:41:9)\nflutter: #3      _InkResponseState.handleTap (package:flutter/src/material/ink_well.dart:1176:21)\nflutter: ════════════════════════════════════════════════════════════════════════════════\nflutter: Another exception was thrown: RangeError (index): Index out of range: index 3, length 2\nflutter: depois",
            &[][..],
        ),
        (
            "dart_unhandled",
            "Unhandled exception:\nNull check operator used on a null value\n#0      _CheckoutPageState.build (package:app/checkout/checkout_page.dart:120:41)\n#1      StatefulElement.build (package:flutter/src/widgets/framework.dart:5592:27)\n<asynchronous suspension>\nnext line",
            &[][..],
        ),
        (
            "node_error_block",
            "Error: connect ECONNREFUSED 127.0.0.1:5432\n    at TCPConnectWrap.afterConnect [as oncomplete] (node:net:1555:16)\n    at pool.connect (/app/src/db.ts:12:9) {\n  errno: -61,\n  code: 'ECONNREFUSED'\n}\n[nodemon] app crashed",
            &["/app"][..],
        ),
        (
            "python_traceback",
            "Traceback (most recent call last):\n  File \"app.py\", line 12, in <module>\n    main()\n  File \"/usr/lib/python3.12/site-packages/flask/app.py\", line 100, in run\n    x()\n  File \"shop/orders.py\", line 41, in create\n    qty = int(payload[\"qty\"])\nValueError: invalid literal for int() with base 10: 'abc'\nok",
            &[][..],
        ),
        (
            "rust_panic",
            "thread 'main' panicked at src/main.rs:12:5:\nindex out of bounds: the len is 2 but the index is 3\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\nfim",
            &[][..],
        ),
        (
            "go_panic",
            "panic: runtime error: index out of range [3] with length 2\n\ngoroutine 1 [running]:\nmain.run(...)\n        /app/main.go:12 +0x1d\nmain.main()\n        /app/main.go:8 +0x15\nexit status 2",
            &["/app"][..],
        ),
        (
            "dart_test_fail",
            "00:08 +11 -1: CartServiceTest adds item beyond capacity [E]\n  RangeError (index): Index out of range: index 3, length 2\n  package:app/cart/cart_service.dart 87:22  CartService.add\n  test/cart/cart_service_test.dart 33:5     main.<fn>.<fn>\n\n00:08 +11 -1: Some tests failed.",
            &[][..],
        ),
        (
            "pytest_fail",
            "FAILED tests/test_x.py::test_y - AssertionError: 1 != 2",
            &[][..],
        ),
        (
            "raw_severities",
            "Compiled successfully with 0 errors\nwebpack compiled with 1 warning\n[nodemon] app crashed - waiting for file changes\nDEBUG something\nFATAL out of memory\nplain line",
            &[][..],
        ),
    ];

    for (key, input, roots) in samples {
        let got = Value::Array(parse_sample(input, roots));
        assert_eq!(got, golden["parser"][key], "parser mismatch on {key}");
    }

    let mut err_parser = TelemetryLineParser::new("r_1", TelemetryParserConfig::default());
    let warn_evs = err_parser.feed(
        "WARN [cache] slow miss",
        TelemetryStream::Err,
        1_758_628_800_000,
        Some(0),
    );
    let warn_json: Vec<Value> = warn_evs.iter().map(event_to_golden_json).collect();
    assert_eq!(
        Value::Array(warn_json),
        golden["parser"]["warn_stderr"],
        "parser mismatch on warn_stderr"
    );
}

#[test]
fn telemetry_config_parses_jsonc_and_ignores_invalid() {
    let cfg = TelemetryWorkspaceConfig::parse(
        r#"{
  // prefixos extras
  "unwrap": ["^api-1\\s*\\| ", "[invalid"],
  "ignore": ["DeprecationWarning: punycode"],
  "projectFrames": ["packages/"],
  "proxy": { "listen": 3100, "upstream": "http://127.0.0.1:3000" }
}"#,
    );
    assert_eq!(cfg.unwrap.len(), 1);
    assert!(cfg.ignore[0]
        .is_match("DeprecationWarning: punycode is old")
        .unwrap());
    assert_eq!(cfg.project_frames, vec!["packages/"]);
    let proxy = cfg.proxy.unwrap();
    assert_eq!(proxy.listen, 3100);
    assert_eq!(proxy.upstream, "http://127.0.0.1:3000");

    assert!(TelemetryWorkspaceConfig::load("/nope/never")
        .proxy
        .is_none());
    assert!(TelemetryWorkspaceConfig::parse("not json")
        .unwrap
        .is_empty());
    assert!(
        TelemetryWorkspaceConfig::parse(r#"{"proxy":{"listen":"x"}}"#)
            .proxy
            .is_none()
    );
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Runtime::new().unwrap().block_on(f)
}

#[test]
fn sqlite_store_runs_cases_triage_fts_context_marks_and_vacuum() {
    block_on(async {
        let dir = tempfile::tempdir().unwrap();
        let registry = TelemetryStoreRegistry::with_directory(dir.path());
        let store = registry.for_workspace("ws-1").await.unwrap();
        let expected_path = TelemetryStoreRegistry::db_path_in(dir.path(), "ws-1");
        assert!(expected_path.exists());

        let r1 = store
            .open_run(
                TelemetryRun::key_for("/ws/app", "flutter run", None),
                "app",
                TelemetryRunSource::Task,
                "/ws/app",
                "flutter run",
                Some("app".into()),
                Some("app".into()),
                Some("t3".into()),
                None,
                Some(1_700_000_000_000),
            )
            .await
            .unwrap();
        assert!(r1.id.starts_with("r_"));
        assert!(r1.is_live());

        let flutter_block = "══╡ EXCEPTION CAUGHT BY GESTURE ╞════════\n\
        The following RangeError was thrown while handling a gesture:\n\
        RangeError (index): Index out of range: index 3, length 2\n\
        When the exception was thrown, this was the stack:\n\
        #0      CartService.add (package:app/cart/cart_service.dart:87:22)\n\
        ════════════════════════════════════════";
        let mut parser = TelemetryLineParser::new(&r1.id, TelemetryParserConfig::default());
        let mut evs = Vec::new();
        for (i, line) in format!("{{\"level\":\"debug\",\"msg\":\"cart add\",\"probe\":\"cart\",\"items\":2}}\n{flutter_block}\nafter")
        .lines()
        .enumerate()
    {
        evs.extend(parser.feed(line, TelemetryStream::Out, 1_700_000_001_000 + i as i64, Some(i as i64)));
    }
        evs.extend(parser.flush());
        let saved = store.append(evs).await.unwrap();
        assert_eq!(saved.len(), 3);
        assert!(saved[0].id.as_deref().unwrap().starts_with("ev_"));
        assert!(saved[1].fingerprint.as_deref().is_some());

        // Probe & FTS search
        let probes = store
            .events(TelemetryQuery {
                probe: Some("cart".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(probes.len(), 1);
        assert_eq!(probes[0].attrs["items"], 2);

        let fts_hits = store
            .events(TelemetryQuery {
                text: Some("cart".into()),
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(fts_hits.len(), 1);

        // Context
        let ctx = store
            .context(saved[1].id.as_deref().unwrap(), 2, 2)
            .await
            .unwrap();
        assert_eq!(ctx.len(), 3);

        // Close r1, open r2, triage -> regression
        store
            .close_run(&r1.id, Some(0), Some(1_700_000_010_000))
            .await
            .unwrap();
        let cases1 = store.cases(TelemetryQuery::default()).await.unwrap();
        assert_eq!(cases1.len(), 1);
        assert!(cases1[0].is_new);
        let fp = cases1[0].fingerprint.clone();

        store
            .triage(
                &fp,
                TelemetryTriageStatus::Resolved,
                TelemetryTriageActor::Agent,
                Some("fixed".into()),
            )
            .await
            .unwrap();
        assert!(store
            .cases(TelemetryQuery::default())
            .await
            .unwrap()
            .is_empty());

        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let r2 = store
            .open_run(
                TelemetryRun::key_for("/ws/app", "flutter run", None),
                "app",
                TelemetryRunSource::Task,
                "/ws/app",
                "flutter run",
                Some("app".into()),
                Some("app".into()),
                Some("t3".into()),
                None,
                Some(now_ms()),
            )
            .await
            .unwrap();
        let mut p2 = TelemetryLineParser::new(&r2.id, TelemetryParserConfig::default());
        let mut evs2 = Vec::new();
        for line in flutter_block.lines() {
            evs2.extend(p2.feed(line, TelemetryStream::Out, now_ms(), None));
        }
        evs2.extend(p2.flush());
        store.append(evs2).await.unwrap();

        let reg = store.cases(TelemetryQuery::default()).await.unwrap();
        assert_eq!(reg.len(), 1);
        assert!(reg[0].is_regression);
        assert_eq!(reg[0].count, 2);

        // Marks & vacuum
        store.mark_start("m1", Some(100)).await.unwrap();
        store.mark_end("m1", Some(200)).await.unwrap();
        assert_eq!(store.mark("m1").await.unwrap(), Some((100, Some(200))));

        store
            .vacuum(TelemetryRetention {
                max_bytes: 256 * 1024 * 1024,
                max_age_ms: 1000,
            })
            .await
            .unwrap();
        let remaining_runs = store.runs(None, None, false, 10).await.unwrap();
        assert_eq!(remaining_runs.len(), 1);
        assert_eq!(remaining_runs[0].id, r2.id);

        registry.close_all().await;
    });
}

#[test]
fn telemetry_db_sync_fts_and_10k_batch() {
    let mut db = TelemetryDb::memory().unwrap();
    assert!(db.has_fts());
    let run = db
        .open_run(
            "k",
            "p",
            TelemetryRunSource::Wrapper,
            "/x",
            "c",
            None,
            None,
            None,
            None,
            Some(1000),
        )
        .unwrap();
    let batch: Vec<TelemetryEvent> = (0..10_000)
        .map(|i| TelemetryEvent {
            id: None,
            run_id: run.id.clone(),
            ts_ms: 1000 + i,
            stream: TelemetryStream::Out,
            severity: TelemetrySeverity::Info,
            body: format!("line {i} token={i}"),
            attrs: Map::new(),
            error: None,
            fingerprint: None,
            trace_id: None,
            span_id: None,
            request_id: None,
            raw_line_offset: None,
            repeat: 1,
        })
        .collect();
    let saved = db.append(&batch).unwrap();
    assert_eq!(saved.len(), 10_000);
    let found = db
        .events(&TelemetryQuery {
            text: Some("line 9999".into()),
            limit: 5,
            ..Default::default()
        })
        .unwrap();
    assert!(found.iter().any(|e| e.body == "line 9999 token=[redacted]"));
}

#[test]
fn ingest_resolves_roots_chunks_dedup_and_notices() {
    block_on(async {
        let registry = Arc::new(TelemetryStoreRegistry::memory());
        let ingest = TelemetryIngest::new(Arc::clone(&registry));
        ingest.register_workspace(TelemetryWorkspace {
            id: "ws-mono".into(),
            name: "shop".into(),
            path: "/ws/shop".into(),
            roots: vec!["/ws/shop/app".into(), "/ws/shop/backend".into()],
        });
        ingest.register_workspace(TelemetryWorkspace {
            id: "ws-single".into(),
            name: "site".into(),
            path: "/ws/site".into(),
            roots: vec!["/ws/site".into()],
        });

        assert_eq!(ingest.resolve("/ws/shop/app/lib").unwrap().1, "app");
        assert_eq!(ingest.resolve("/ws/shop/backend").unwrap().1, "backend");
        assert_eq!(ingest.resolve("/ws/site/src").unwrap().1, "site");
        assert!(ingest.resolve("/elsewhere").is_none());

        let mut rx = ingest.subscribe_notices();
        let task = TelemetryTaskDef {
            id: "flutter:run".into(),
            label: "app".into(),
            cwd: "/ws/shop/app".into(),
            telemetry_enabled: true,
        };
        let session = ingest
            .open_task_run(&task, "flutter run", None, Some("t1".into()))
            .await
            .unwrap()
            .unwrap();
        let env = session.child_environment();
        assert_eq!(env["GSH_RUN_ID"], session.run_id());
        assert_eq!(env["OTEL_SERVICE_NAME"], "app");
        assert!(env["OTEL_RESOURCE_ATTRIBUTES"].contains("gsh.project=app"));

        session.add(
            "flutter: {\"level\":\"info\",\"msg\":\"app sta",
            TelemetryStream::Out,
        );
        session.add(
            "rted\"}\r\nError: boom\r\n    at f (/ws/shop/app/lib/a.js:1:1)\r\n",
            TelemetryStream::Out,
        );
        session.add_events(vec![TelemetryEvent {
            id: None,
            run_id: session.run_id().to_string(),
            ts_ms: now_ms(),
            stream: TelemetryStream::Vm,
            severity: TelemetrySeverity::Error,
            body: "boom".into(),
            attrs: Map::new(),
            error: Some(TelemetryError {
                err_type: "Error".into(),
                message: "boom".into(),
                stack: None,
                frames: vec![TelemetryFrame {
                    raw: String::new(),
                    file: Some("lib/a.js".into()),
                    line: Some(1),
                    column: None,
                    function: None,
                    in_project: true,
                }],
            }),
            fingerprint: None,
            trace_id: None,
            span_id: None,
            request_id: None,
            raw_line_offset: None,
            repeat: 1,
        }]);
        session.close(Some(0)).await.unwrap();

        let notice = rx.try_recv().unwrap();
        assert_eq!(notice.workspace_id, "ws-mono");
        assert_eq!(notice.fingerprints.len(), 1);

        let store = registry.for_workspace("ws-mono").await.unwrap();
        let cases = store.cases(TelemetryQuery::default()).await.unwrap();
        assert_eq!(cases.len(), 1);
        assert_eq!(cases[0].count, 1); // deduped!
        assert_eq!(cases[0].location.as_deref(), Some("lib/a.js:1"));
    });
}

struct MockOtlpSink {
    events: std::sync::Mutex<Vec<(String, Vec<TelemetryEvent>)>>,
}

impl OtlpSink for MockOtlpSink {
    fn run_for(&self, run_id: Option<&str>, service_name: Option<&str>) -> Option<String> {
        if run_id == Some("r_1") {
            return Some("r_1".into());
        }
        if service_name == Some("api") {
            return Some("r_api".into());
        }
        None
    }

    fn add_events(&self, run_id: &str, events: Vec<TelemetryEvent>) {
        self.events
            .lock()
            .unwrap()
            .push((run_id.to_string(), events));
    }

    fn project_roots_of(&self, _run_id: &str) -> Vec<String> {
        vec!["/ws/backend".into()]
    }
}

#[test]
fn otlp_receiver_logs_traces_and_errors() {
    block_on(async {
        let sink = Arc::new(MockOtlpSink {
            events: std::sync::Mutex::new(Vec::new()),
        });
        let rx = OtlpReceiver::new(Arc::clone(&sink) as Arc<dyn OtlpSink>);
        rx.start().await;
        let ep = rx.endpoint();
        assert!(ep.starts_with("http://127.0.0.1:"));

        let client = reqwest::Client::new();
        let r = client
        .post(format!("{ep}/v1/logs"))
        .header("content-type", "application/json")
        .body(
            json!({
                "resourceLogs": [{
                    "resource": {"attributes": [
                        {"key": "gsh.run", "value": {"stringValue": "r_1"}},
                        {"key": "service.name", "value": {"stringValue": "api"}}
                    ]},
                    "scopeLogs": [{
                        "logRecords": [{
                            "timeUnixNano": "1758632475810000000",
                            "severityNumber": 17,
                            "body": {"stringValue": "db connect failed"},
                            "traceId": "abc",
                            "attributes": [
                                {"key": "exception.type", "value": {"stringValue": "Error"}},
                                {"key": "exception.message", "value": {"stringValue": "connect ECONNREFUSED"}},
                                {"key": "exception.stacktrace", "value": {"stringValue": "Error: connect\n    at pool.connect (/ws/backend/src/db.ts:12:9)"}}
                            ]
                        }]
                    }]
                }]
            })
            .to_string(),
        )
        .send()
        .await
        .unwrap();
        assert_eq!(r.status(), 200);

        let r405 = client.get(format!("{ep}/v1/logs")).send().await.unwrap();
        assert_eq!(r405.status(), 405);

        let r415 = client
            .post(format!("{ep}/v1/logs"))
            .header("content-type", "application/x-protobuf")
            .body(vec![1u8, 2, 3])
            .send()
            .await
            .unwrap();
        assert_eq!(r415.status(), 415);

        let got = sink.events.lock().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].0, "r_1");
        assert_eq!(
            got[0].1[0]
                .error
                .as_ref()
                .unwrap()
                .project_frame()
                .unwrap()
                .location()
                .as_deref(),
            Some("src/db.ts:12")
        );
        rx.stop();
    });
}

#[test]
fn http_proxy_forwards_injects_records_and_replays() {
    block_on(async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let up_port = listener.local_addr().unwrap().port();
        let seen = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let seen_c = Arc::clone(&seen);
        let up_task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let seen_c = Arc::clone(&seen_c);
                tokio::spawn(async move {
                    let io = TokioIo::new(stream);
                    let svc = service_fn(move |req: Request<Incoming>| {
                        let seen_c = Arc::clone(&seen_c);
                        async move {
                            let req_id = req
                                .headers()
                                .get("x-request-id")
                                .and_then(|v| v.to_str().ok())
                                .unwrap_or("")
                                .to_string();
                            seen_c.lock().unwrap().push(req_id);
                            let _ = req.into_body().collect().await;
                            Ok::<_, std::convert::Infallible>(
                                Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .header("content-type", "application/json")
                                    .body(Full::new(Bytes::from_static(b"{\"err\":\"boom\"}")))
                                    .unwrap(),
                            )
                        }
                    });
                    let _ = http1::Builder::new().serve_connection(io, svc).await;
                });
            }
        });

        // Pick a free port for proxy
        let tmp = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let listen_port = tmp.local_addr().unwrap().port();
        drop(tmp);

        let recorded = Arc::new(std::sync::Mutex::new(Vec::<TelemetryEvent>::new()));
        let rec_c = Arc::clone(&recorded);
        let proxy = HttpTelemetryProxy::new(
            TelemetryProxyConfig {
                listen: listen_port,
                upstream: format!("http://127.0.0.1:{up_port}"),
            },
            "r_proxy".into(),
            Arc::new(move |evs| rec_c.lock().unwrap().extend(evs)),
        );
        assert!(proxy.start().await);

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("http://127.0.0.1:{listen_port}/api/orders?x=1"))
            .header("authorization", "Bearer super-secret-token")
            .header("content-type", "application/json")
            .body(r#"{"password":"hunter2","item":"abc"}"#)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 500);

        tokio::time::sleep(std::time::Duration::from_millis(30)).await;
        let evs = recorded.lock().unwrap().clone();
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].severity, TelemetrySeverity::Error);
        assert_eq!(evs[0].error.as_ref().unwrap().err_type, "HTTP 500");

        let replayed = proxy.replay(&evs).await;
        assert_eq!(replayed, 1);
        let ids = seen.lock().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids[1].ends_with("-replay"));

        proxy.stop();
        up_task.abort();
    });
}

#[test]
fn cli_handler_wrapper_show_triage_and_cap() {
    block_on(async {
        let registry = Arc::new(TelemetryStoreRegistry::memory());
        let ingest = TelemetryIngest::new(Arc::clone(&registry));
        ingest.register_workspace(TelemetryWorkspace {
            id: "ws1".into(),
            name: "shop".into(),
            path: "/ws/shop".into(),
            roots: vec!["/ws/shop".into()],
        });
        let last_edit = Arc::new(std::sync::Mutex::new(None::<i64>));
        let le = Arc::clone(&last_edit);
        let cli = TelemetryCliHandler::new(
            Arc::clone(&registry),
            Arc::clone(&ingest),
            Arc::new(move |_| *le.lock().unwrap()),
            Arc::new(|_| vec!["/ws/shop".into()]),
        );

        let call = |cmd: &'static str, args: Map<String, Value>| {
            let cli_ref = &cli;
            async move {
                cli_ref
                    .handle(cmd, Some("t9"), &args, Some("ws1"), "/ws/shop")
                    .await
            }
        };

        let open = call(
            "telemetry-open",
            json!({"cwd": "/ws/shop", "command": "flutter test"})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await;
        assert_eq!(open["ok"], true);
        let run_id = open["data"]["run"].as_str().unwrap().to_string();

        let ingest_res = call(
            "telemetry-ingest",
            json!({
                "run": run_id,
                "lines": [
                    "{\"level\":\"debug\",\"msg\":\"cart add\",\"itemId\":\"sku-88\"}",
                    "RangeError (index): Index out of range: index 3, length 2",
                    "#0      CartService.add (package:app/cart/cart_service.dart:87:22)",
                    "after"
                ]
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .await;
        assert_eq!(ingest_res["ok"], true);

        let close_res = call(
            "telemetry-close",
            json!({"run": run_id, "exit_code": 1})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await;
        assert_eq!(close_res["ok"], true);
        assert_eq!(close_res["data"]["errors"], 1);

        let errors = call("telemetry-errors", Map::new()).await;
        let id = errors["data"]["cases"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let shown = call(
            "telemetry-show",
            json!({"id": id}).as_object().unwrap().clone(),
        )
        .await;
        assert_eq!(
            shown["data"]["case"]["location"],
            "lib/cart/cart_service.dart:87"
        );
        assert_eq!(shown["data"]["correlated"]["attrs"]["itemId"], "sku-88");

        // App mode (--app)
        let app_s = ingest.open_app_run("1.0.0").await.unwrap();
        app_s.close(Some(0)).await.unwrap();
        let app_runs = cli
            .handle("telemetry-runs", None, &Map::new(), None, "")
            .await;
        assert_eq!(app_runs["ok"], true);
        assert_eq!(app_runs["data"]["runs"].as_array().unwrap().len(), 1);
        assert_eq!(
            registry
                .for_workspace(K_TELEMETRY_APP_WORKSPACE_ID)
                .await
                .unwrap()
                .runs(None, None, false, 10)
                .await
                .unwrap()
                .len(),
            1
        );

        // Reply cap check
        let mut big_lines = Vec::new();
        for i in 0..400 {
            big_lines.push(format!(
                "Error: failure number {i} in a fairly long message to fill the payload {}",
                "x".repeat(120)
            ));
            big_lines.push(format!("    at fn{i} (src/file{i}.ts:{i}:1)"));
        }
        let open2 = call(
            "telemetry-open",
            json!({"cwd": "/ws/shop", "command": "big"})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await;
        let run2 = open2["data"]["run"].as_str().unwrap().to_string();
        call(
            "telemetry-ingest",
            json!({"run": run2, "lines": big_lines})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await;
        call(
            "telemetry-close",
            json!({"run": run2, "exit_code": 1})
                .as_object()
                .unwrap()
                .clone(),
        )
        .await;
        let capped = call(
            "telemetry-errors",
            json!({"limit": 400}).as_object().unwrap().clone(),
        )
        .await;
        assert_eq!(capped["data"]["truncated"], true);
        assert!(
            serde_json::to_string(&capped["data"]["cases"])
                .unwrap()
                .len()
                <= K_TELEMETRY_REPLY_CAP
        );
    });
}
