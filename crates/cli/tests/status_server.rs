//! Integração cliente x servidor: o `gsh hook` de verdade (o binário) manda o
//! envelope do harness, e o servidor de `gsh-ipc` entrega o evento tipado.
//! Cobre o socket Unix e o TCP loopback com token (o caminho do Windows), este
//! em qualquer plataforma.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use gsh_core::status::{AgentHarness, HookEvent, StatusUpdate, TurnStatus, TurnTracker};
use gsh_core::wire;
use gsh_ipc::{Delivery, ServerConfig, StatusServer, Transport};
use serde_json::{json, Value};

const BIN: &str = env!("CARGO_BIN_EXE_gsh");
const WAIT: Duration = Duration::from_secs(5);
const QUIET: Duration = Duration::from_millis(300);

fn unique_dir() -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::temp_dir().join(format!(
        "gss{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn transports() -> Vec<Transport> {
    let mut all = vec![Transport::Tcp];
    #[cfg(unix)]
    all.push(Transport::Unix);
    all
}

struct App {
    home: PathBuf,
    server: StatusServer,
    rx: Receiver<Delivery>,
}

impl App {
    fn start(transport: Transport) -> App {
        let home = unique_dir();
        let mut config = ServerConfig::new(home.join(".gsh"));
        config.transport = transport;
        let (server, rx) = StatusServer::start(&config).unwrap();
        App { home, server, rx }
    }

    /// Roda `gsh hook` como uma aba do GSH: com o env que o PTY injetaria.
    fn hook(&self, harness: Option<&str>, envelope: &Value) -> Output {
        self.hook_with_env(harness, envelope, &self.server.hook_env_owned(), Some("t3"))
    }

    fn hook_with_env(
        &self,
        harness: Option<&str>,
        envelope: &Value,
        env: &[(String, String)],
        pane: Option<&str>,
    ) -> Output {
        let mut cmd = Command::new(BIN);
        cmd.arg("hook");
        if let Some(h) = harness {
            cmd.args(["--harness", h]);
        }
        cmd.current_dir(&self.home)
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env_remove(wire::ENV_PANE_ID)
            .env_remove(wire::ENV_TAB_ID)
            .env_remove(wire::ENV_STATUS_SOCK)
            .env_remove(wire::ENV_STATUS_PORT)
            .env_remove(wire::ENV_STATUS_TOKEN)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(pane) = pane {
            cmd.env(wire::ENV_PANE_ID, pane);
        }
        for (k, v) in env {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(envelope.to_string().as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }

    fn status(&self) -> StatusUpdate {
        match self.rx.recv_timeout(WAIT).expect("nada entregue") {
            Delivery::Status(u) => u,
            other => panic!("esperava Status, veio {other:?}"),
        }
    }

    fn assert_quiet(&self) {
        match self.rx.recv_timeout(QUIET) {
            Err(RecvTimeoutError::Timeout) => {}
            other => panic!("não devia entregar nada: {other:?}"),
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

/// `hook_env` sem o `&'static str` (para passar a `Command`).
trait HookEnvOwned {
    fn hook_env_owned(&self) -> Vec<(String, String)>;
}

impl HookEnvOwned for StatusServer {
    fn hook_env_owned(&self) -> Vec<(String, String)> {
        self.hook_env()
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }
}

fn abs(name: &str) -> String {
    std::env::temp_dir()
        .join("repo")
        .join(name)
        .to_string_lossy()
        .into_owned()
}

fn envelope(event: &str, extra: Value) -> Value {
    let mut env = json!({
        "hook_event_name": event,
        "session_id": "sess-1",
        "transcript_path": "/home/u/.claude/projects/x/sess-1.jsonl",
    });
    if let (Some(base), Some(extra)) = (env.as_object_mut(), extra.as_object()) {
        base.extend(extra.clone());
    }
    env
}

#[test]
fn ciclo_de_turno_do_claude_chega_tipado_e_move_a_maquina() {
    for t in transports() {
        let app = App::start(t);
        let file = abs("a.rs");
        let steps: Vec<(Value, Option<&str>, TurnStatus, HookEvent)> = vec![
            (
                envelope("SessionStart", json!({})),
                None,
                TurnStatus::Idle,
                HookEvent::SessionStart,
            ),
            (
                envelope("UserPromptSubmit", json!({})),
                None,
                TurnStatus::Working,
                HookEvent::UserPromptSubmit,
            ),
            (
                envelope("PreToolUse", json!({"tool_name": "Bash"})),
                None,
                TurnStatus::Working,
                HookEvent::PreToolUse,
            ),
            (
                envelope("PreToolUse", json!({"tool_name": "AskUserQuestion"})),
                None,
                TurnStatus::Waiting,
                HookEvent::PreToolUse,
            ),
            (
                envelope(
                    "PostToolUse",
                    json!({"tool_name": "Write", "cwd": abs(""), "tool_input": {"file_path": file}}),
                ),
                None,
                TurnStatus::Working,
                HookEvent::PostToolUse,
            ),
            (
                envelope(
                    "Notification",
                    json!({"message": "Claude needs your approval"}),
                ),
                None,
                TurnStatus::Waiting,
                HookEvent::Notification,
            ),
            (
                envelope("Notification", json!({"notification_type": "idle_prompt"})),
                None,
                TurnStatus::Idle,
                HookEvent::Notification,
            ),
            (
                envelope("Stop", json!({})),
                None,
                TurnStatus::Idle,
                HookEvent::Stop,
            ),
            (
                envelope("SessionEnd", json!({})),
                Some("claude"),
                TurnStatus::Idle,
                HookEvent::SessionEnd,
            ),
        ];
        let mut got = Vec::new();
        for (env, harness, status, event) in steps {
            let out = app.hook(harness, &env);
            assert!(out.status.success(), "{t:?}: {out:?}");
            assert!(out.stdout.is_empty(), "o hook não escreve no stdout");
            let u = app.status();
            assert_eq!(u.pane_id, "t3", "{t:?}");
            assert_eq!(u.status, status, "{t:?} {event:?}");
            assert_eq!(u.event, Some(event), "{t:?}");
            assert_eq!(u.session_id.as_deref(), Some("sess-1"));
            assert_eq!(
                u.transcript_path.as_deref(),
                Some("/home/u/.claude/projects/x/sess-1.jsonl")
            );
            assert_eq!(u.harness, Some(AgentHarness::Claude), "default é claude");
            got.push(u);
        }
        // Arquivo tocado só no PostToolUse de edição.
        assert_eq!(got.get(4).map(|u| u.files.clone()), Some(vec![file]));
        assert!(got
            .iter()
            .enumerate()
            .all(|(i, u)| i == 4 || u.files.is_empty()));

        // Alimenta a máquina com o que chegou, em tempo sintético (10 ms entre eventos).
        use gsh_core::status::Applied::{Changed, Unchanged};
        use TurnStatus::{Idle, Waiting, Working};
        let mut turn = TurnTracker::new();
        let applied: Vec<_> = got
            .iter()
            .zip((1..).map(|i| i * 10))
            .map(|(u, now)| turn.apply(u, now))
            .collect();
        assert_eq!(
            applied,
            [
                Unchanged,                 // SessionStart: já era Idle
                Changed { from: Idle },    // UserPromptSubmit
                Unchanged,                 // PreToolUse Bash
                Changed { from: Working }, // PreToolUse AskUserQuestion -> waiting
                Changed { from: Waiting }, // PostToolUse
                Changed { from: Working }, // Notification de aprovação
                Changed { from: Waiting }, // Notification idle
                Unchanged,                 // Stop: já era Idle
                Unchanged,                 // SessionEnd
            ]
        );
        assert_eq!(turn.status(), Idle);
        assert!(!turn.turn_active());
        // O Idle assentou 50 ms depois da última mudança (a Notification, em 70 ms).
        assert_eq!(turn.poll_finished(70 + 50), Some(Idle));
        app.assert_quiet();
    }
}

#[test]
fn codex_marca_o_harness_e_permission_request_e_waiting() {
    for t in transports() {
        let app = App::start(t);
        let env = envelope(
            "PermissionRequest",
            json!({"tool_name": "shell", "turn_id": "u1", "tool_input": {"command": ["ls"]}}),
        );
        let out = app.hook(Some("codex"), &env);
        assert!(out.status.success());
        // Só o Claude pergunta pela ilha; no Codex não há `permission-ask`.
        assert!(out.stdout.is_empty());
        let u = app.status();
        assert_eq!(u.status, TurnStatus::Waiting);
        assert_eq!(u.event, Some(HookEvent::PermissionRequest));
        assert_eq!(u.harness, Some(AgentHarness::Codex));
        app.assert_quiet();
    }
}

#[test]
fn permission_request_do_claude_sem_handler_deixa_o_terminal_perguntar() {
    for t in transports() {
        let app = App::start(t);
        let env = envelope(
            "PermissionRequest",
            json!({"tool_name": "Bash", "tool_input": {"command": "ls"}}),
        );
        let out = app.hook(None, &env);
        // O app responde `commands unavailable` ao `permission-ask`: o hook não
        // imprime decisão e o Claude mostra o prompt de sempre.
        assert!(out.status.success(), "{out:?}");
        assert!(
            out.stdout.is_empty(),
            "{:?}",
            String::from_utf8_lossy(&out.stdout)
        );
        let u = app.status();
        assert_eq!(u.status, TurnStatus::Waiting);
        app.assert_quiet();
    }
}

#[test]
fn evento_inerte_e_sessao_fora_do_gsh_nao_entregam_nada() {
    for t in transports() {
        let app = App::start(t);
        // Subagente do Codex: sem status.
        app.hook(Some("codex"), &envelope("SubagentStart", json!({})));
        // Sem GSH_PANE_ID: não é sessão do GSH, o hook é no-op.
        let out = app.hook_with_env(
            None,
            &envelope("Stop", json!({})),
            &app.server.hook_env_owned(),
            None,
        );
        assert!(out.status.success());
        app.assert_quiet();
        // E o servidor segue entregando.
        app.hook(None, &envelope("Stop", json!({})));
        assert_eq!(app.status().status, TurnStatus::Idle);
    }
}

#[test]
fn tcp_com_token_errado_ou_ausente_e_descartado() {
    let app = App::start(Transport::Tcp);
    let good = app.server.hook_env_owned();
    let with_token = |tok: Option<&str>| -> Vec<(String, String)> {
        good.iter()
            .filter(|(k, _)| k != wire::ENV_STATUS_TOKEN)
            .cloned()
            .chain(tok.map(|t| (wire::ENV_STATUS_TOKEN.to_string(), t.to_string())))
            .collect()
    };
    let stop = envelope("Stop", json!({}));
    for env in [with_token(None), with_token(Some("0".repeat(32).as_str()))] {
        let out = app.hook_with_env(None, &stop, &env, Some("t3"));
        assert!(out.status.success());
    }
    app.assert_quiet();
    app.hook_with_env(
        None,
        &stop,
        &with_token(
            good.iter()
                .find(|(k, _)| k == wire::ENV_STATUS_TOKEN)
                .map(|(_, v)| v.as_str()),
        ),
        Some("t9"),
    );
    assert_eq!(app.status().pane_id, "t9");
    app.assert_quiet();
}
