use std::io::{BufRead, BufReader, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;

use gsh_core::status::{AgentHarness, HookEvent, StatusUpdate};

use super::*;
use crate::{now_epoch_ms, tests::test_shell, Msg, StatusRuntime, TerminalPane};

const TEST_TIMEOUT: Duration = Duration::from_secs(3);

struct Fixture {
    shell: Shell,
    socket: std::path::PathBuf,
    terminal: UnixStream,
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let status = StatusRuntime::start_with_config(&gsh_ipc::ServerConfig {
            gsh_dir: dir.path().to_owned(),
            debug: true,
            transport: gsh_ipc::Transport::Unix,
        });
        let gsh_ipc::Endpoint::Unix(socket) = status.endpoint().unwrap() else {
            panic!("expected Unix socket");
        };
        let socket = socket.clone();
        let (session_socket, terminal) = UnixStream::pair().unwrap();
        terminal.set_read_timeout(Some(TEST_TIMEOUT)).unwrap();
        let shutdown = session_socket.try_clone().unwrap();
        let session = gsh_term::TermSession::from_stream(
            session_socket.try_clone().unwrap(),
            session_socket,
            |_, _| Ok(()),
            move || {
                let _ = shutdown.shutdown(Shutdown::Both);
            },
            24,
            3,
            100,
        )
        .unwrap();
        let mut shell = test_shell();
        shell.status_runtime = status;
        shell.status_runtime.register_pane("t1");
        shell.terminals.insert(
            1,
            TerminalPane::with_session_config(session, gsh_config::Config::default())
                .with_pane_id("t1"),
        );
        Self {
            shell,
            socket,
            terminal,
            _dir: dir,
        }
    }

    fn send(&mut self, command: &str, tab_id: &str, args: Value) -> BufReader<UnixStream> {
        let mut socket = UnixStream::connect(&self.socket).unwrap();
        socket.set_read_timeout(Some(TEST_TIMEOUT)).unwrap();
        writeln!(
            socket,
            "{}",
            json!({"type": "cmd", "cmd": command, "tabId": tab_id, "args": args})
        )
        .unwrap();
        let deadline = Instant::now() + TEST_TIMEOUT;
        loop {
            self.shell.status_runtime.drain_deliveries(now_epoch_ms());
            let commands = self.shell.status_runtime.take_commands();
            if !commands.is_empty() {
                for command in commands {
                    let _ = self.shell.update(Msg::TeamCliCommand(command));
                }
                return BufReader::new(socket);
            }
            assert!(Instant::now() < deadline, "command did not reach app");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn call(&mut self, command: &str, tab_id: &str, args: Value) -> Value {
        receive(self.send(command, tab_id, args))
    }

    fn status(&mut self, status: TurnStatus, event: HookEvent) {
        assert!(self.shell.status_runtime.apply_update(
            StatusUpdate {
                pane_id: "t1".into(),
                status,
                event: Some(event),
                session_id: None,
                transcript_path: None,
                harness: Some(AgentHarness::Claude),
                files: Vec::new(),
            },
            now_epoch_ms()
        ));
    }
}

fn receive(mut reader: BufReader<UnixStream>) -> Value {
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn writes_utf8_to_the_focused_terminal_through_ipc() {
    let mut fixture = Fixture::new();
    let payload = "olá";
    assert_eq!(
        fixture.call(
            "write",
            "@focused",
            json!({"data": BASE64_STANDARD.encode(payload)})
        )["ok"],
        true
    );
    let mut bytes = [0; 4];
    fixture.terminal.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, payload.as_bytes());
    assert_eq!(
        fixture.call("write", "t1", json!({"data": "!!!"}))["ok"],
        false
    );
}

#[test]
fn writes_send_key_escape_and_control_bytes_to_terminal() {
    let mut fixture = Fixture::new();
    let payload = b"\x1b[A\x01\r";
    assert_eq!(
        fixture.call(
            "write",
            "t1",
            json!({"data": BASE64_STANDARD.encode(payload)})
        )["ok"],
        true
    );
    let mut bytes = [0; 5];
    fixture.terminal.read_exact(&mut bytes).unwrap();
    assert_eq!(&bytes, payload);
}

#[test]
fn reads_scrollback_order_and_offsets_through_ipc() {
    let mut fixture = Fixture::new();
    fixture
        .terminal
        .write_all(b"first\r\nsecond\r\nthird\r\nfourth\x1b[5n")
        .unwrap();
    let mut acknowledge = [0; 4];
    fixture.terminal.read_exact(&mut acknowledge).unwrap();
    assert_eq!(&acknowledge, b"\x1b[0n");
    let page = fixture.call("read-pane", "t1", json!({"lines": 2, "offset": 1}));
    assert_eq!(
        page["data"],
        json!({"text": BASE64_STANDARD.encode("second\nthird"), "lines": 2, "total": 4, "truncated": false})
    );
    let beginning = fixture.call("read-pane", "t1", json!({"lines": 1, "fromStart": true}));
    assert_eq!(beginning["data"]["text"], BASE64_STANDARD.encode("first"));
    let past_end = fixture.call("read-pane", "t1", json!({"lines": 2001, "offset": 40}));
    assert_eq!(
        past_end["data"],
        json!({"text": "", "lines": 0, "total": 4, "truncated": true})
    );
}

#[test]
fn renames_both_model_and_lookup_label_through_ipc() {
    let mut fixture = Fixture::new();
    assert_eq!(
        fixture.call("rename-tab", "@focused", json!({"title": "Operação"}))["data"]["tabId"],
        "t1"
    );
    assert_eq!(
        fixture.call(
            "rename-tab",
            "",
            json!({"target": "OPERAÇÃO", "title": "Revisor"})
        )["ok"],
        true
    );
    assert_eq!(
        fixture.shell.panes.focused_model().terminal_label(1),
        Some("Revisor")
    );
    assert_eq!(
        fixture.shell.terminal_labels.get(&1).map(String::as_str),
        Some("Revisor")
    );
}

#[test]
fn sends_escape_for_an_agent_and_control_c_when_forced_through_ipc() {
    let mut fixture = Fixture::new();
    fixture.status(TurnStatus::Working, HookEvent::UserPromptSubmit);
    assert_eq!(
        fixture.call("interrupt", "t1", json!({}))["data"]["tabId"],
        "t1"
    );
    let mut key = [0_u8];
    fixture.terminal.read_exact(&mut key).unwrap();
    assert_eq!(key, [0x1b]);
    assert_eq!(
        fixture.call("interrupt", "t1", json!({"force": true}))["ok"],
        true
    );
    fixture.terminal.read_exact(&mut key).unwrap();
    assert_eq!(key, [0x03]);
}

#[test]
fn waits_for_a_working_agent_and_replies_after_idle_through_ipc() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.call("wait-idle", "t1", json!({}))["ok"], false);
    fixture.status(TurnStatus::Working, HookEvent::UserPromptSubmit);
    let reader = fixture.send("wait-idle", "t1", json!({}));
    assert_eq!(fixture.shell.idle_waiters.len(), 1);
    fixture.status(TurnStatus::Idle, HookEvent::Stop);
    let _ = fixture.shell.update(Msg::StatusTick);
    assert_eq!(
        receive(reader),
        json!({"ok": true, "data": {"tabId": "t1"}})
    );
}
