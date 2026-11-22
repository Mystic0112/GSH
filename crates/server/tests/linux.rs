#![cfg(target_os = "linux")]

use std::{
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};

const SERVER: &str = env!("CARGO_BIN_EXE_gsh-server");

struct Daemon {
    path: PathBuf,
    child: Child,
    _directory: tempfile::TempDir,
}

impl Daemon {
    fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("run/server.sock");
        let mut child = server(&path).spawn().unwrap();
        wait_until(|| {
            assert!(
                child.try_wait().unwrap().is_none(),
                "server exited at startup"
            );
            UnixStream::connect(&path).is_ok()
        });
        Self {
            path,
            child,
            _directory: directory,
        }
    }

    fn client(&self) -> Client {
        let mut client = Client::connect(&self.path);
        client.send(json!({"t":"hello","v":2,"client":"gsh-server-test"}));
        let hello = client.receive();
        assert_eq!(hello["t"], "hello.ack");
        assert_eq!(hello["v"], 2);
        client
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct Client(BufReader<UnixStream>);

impl Client {
    fn connect(path: &Path) -> Self {
        let stream = UnixStream::connect(path).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        Self(BufReader::new(stream))
    }

    fn send(&mut self, message: Value) {
        let stream = self.0.get_mut();
        serde_json::to_writer(&mut *stream, &message).unwrap();
        stream.write_all(b"\n").unwrap();
    }

    fn receive(&mut self) -> Value {
        let mut frame = String::new();
        assert!(
            self.0.read_line(&mut frame).unwrap() > 0,
            "server closed connection"
        );
        serde_json::from_str(&frame).unwrap()
    }
}

fn server(path: &Path) -> Command {
    let mut command = Command::new(SERVER);
    command.arg("--socket").arg(path).stdout(Stdio::null());
    command
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(4);
    while !ready() {
        assert!(Instant::now() < deadline, "timed out waiting for server");
        thread::sleep(Duration::from_millis(10));
    }
}

fn decode(frame: &Value) -> Vec<u8> {
    STANDARD.decode(frame["d"].as_str().unwrap()).unwrap()
}

#[test]
fn pty_survives_disconnect_and_resumes_with_real_resize_and_exit() {
    let daemon = Daemon::start();
    let mut client = daemon.client();
    client.send(json!({
        "t":"pty.open","rid":41, "cmd":"/bin/sh", "rows":24, "cols":80, "cwd":"/tmp",
        "args":["-c", "stty -echo; printf 'ready\n'; IFS= read -r line; pwd; stty size; printf 'done:%s\n' \"$line\"; exit 7"]
    }));
    let opened = client.receive();
    assert_eq!(opened["t"], "pty.opened");
    assert_eq!(opened["rid"], 41);
    let id = opened["id"].as_str().unwrap();
    client.send(json!({"t":"pty.attach","id":id}));
    let mut offset = 0;
    let mut output = Vec::new();
    while !output.ends_with(b"ready\r\n") {
        let frame = client.receive();
        assert_eq!(frame["t"], "pty.output");
        assert_eq!(frame["off"], offset);
        output.extend(decode(&frame));
        offset = output.len();
    }
    drop(client);

    let mut client = daemon.client();
    client.send(json!({"t":"pty.list","rid":42}));
    let list = client.receive();
    assert_eq!(list["rid"], 42);
    assert_eq!(list["sessions"][0]["id"], id);
    assert_eq!(list["sessions"][0]["len"], offset);
    client.send(json!({"t":"pty.attach","id":id,"from":offset}));
    client.send(json!({"t":"pty.resize","id":id,"rows":41,"cols":111}));
    client.send(json!({"t":"pty.input","id":id,"d":STANDARD.encode("from-reconnect\n")}));

    let mut replay = Vec::new();
    loop {
        let frame = client.receive();
        assert_eq!(frame["id"], id);
        if frame["t"] == "pty.exited" {
            assert_eq!(frame["code"], 7);
            break;
        }
        assert_eq!(frame["t"], "pty.output");
        assert_eq!(frame["off"], offset + replay.len());
        replay.extend(decode(&frame));
    }
    assert_eq!(replay, b"/tmp\r\n41 111\r\ndone:from-reconnect\r\n");
}

#[test]
fn replay_uses_absolute_offset_after_history_evicts_while_detached() {
    let daemon = Daemon::start();
    let mut client = daemon.client();
    const TOTAL: usize = 4_300_000;
    client.send(json!({
        "t":"pty.open","rid":1,"cmd":"/bin/sh","args":["-c",format!("exec head -c {TOTAL} /dev/zero")],
        "flow":true
    }));
    let opened = client.receive();
    assert_eq!(opened["t"], "pty.opened");
    let id = opened["id"].as_str().unwrap();
    wait_until(|| {
        client.send(json!({"t":"pty.list","rid":2}));
        let session = &client.receive()["sessions"][0];
        if session["exit"] == 0 {
            assert_eq!(session["len"], TOTAL);
            return true;
        }
        false
    });
    client.send(json!({"t":"pty.attach","id":id,"from":0}));
    let first = client.receive();
    assert_eq!(first["t"], "pty.output");
    let start = first["off"].as_u64().unwrap();
    assert!(start > 0 && start < TOTAL as u64);
    let first_count = decode(&first).len();
    let mut next = start + first_count as u64;
    const WINDOW: u64 = 256 * 1024;
    while next - start < WINDOW {
        let frame = client.receive();
        assert_eq!(frame["t"], "pty.output");
        assert_eq!(frame["off"], next);
        next += decode(&frame).len() as u64;
    }
    assert_eq!(next - start, WINDOW);
    client
        .0
        .get_mut()
        .set_read_timeout(Some(Duration::from_millis(120)))
        .unwrap();
    let error = client.0.read_line(&mut String::new()).unwrap_err();
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
    client
        .0
        .get_mut()
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    client.send(json!({"t":"pty.ack","id":id,"n":WINDOW}));
    loop {
        let frame = client.receive();
        if frame["t"] == "pty.exited" {
            assert_eq!(frame["code"], 0);
            assert_eq!(next, TOTAL as u64);
            break;
        }
        assert_eq!(frame["t"], "pty.output");
        assert_eq!(frame["off"], next);
        let bytes = decode(&frame);
        assert!(bytes.iter().all(|&byte| byte == 0));
        next += bytes.len() as u64;
        client.send(json!({"t":"pty.ack","id":id,"n":bytes.len()}));
    }
}

#[test]
fn protocol_bounds_frames_and_preserves_correlation() {
    let daemon = Daemon::start();
    let mut client = Client::connect(&daemon.path);
    client.send(json!({"t":"hello","v":1,"client":"legacy"}));
    assert_eq!(client.receive()["code"], "version_mismatch");

    let mut client = Client::connect(&daemon.path);
    client.0.get_mut().write_all(br#"{"t":"hel"#).unwrap();
    thread::sleep(Duration::from_millis(50));
    client
        .0
        .get_mut()
        .write_all(
            b"lo\",\"v\":2,\"client\":\"test\"}\n{\x22t\x22:\x22pty.list\x22,\x22rid\x22:9}\n",
        )
        .unwrap();
    assert_eq!(client.receive()["t"], "hello.ack");
    assert_eq!(
        client.receive(),
        json!({"t":"pty.sessions","rid":9,"sessions":[]})
    );

    client.send(json!({"t":"pty.open","rid":77,"cmd":"", "rows":0}));
    let error = client.receive();
    assert_eq!(error["rid"], 77);
    assert_eq!(error["code"], "spawn_failed");

    let payload = vec![b' '; 64 * 1024 + 1];
    client.0.get_mut().write_all(&payload).unwrap();
    assert_eq!(client.receive()["code"], "frame_too_large");
    assert_eq!(daemon.client().receive_with_list()["sessions"], json!([]));
}

impl Client {
    fn receive_with_list(&mut self) -> Value {
        self.send(json!({"t":"pty.list"}));
        self.receive()
    }
}

#[test]
fn private_socket_rejects_duplicate_and_shutdown_reaps_session() {
    let mut daemon = Daemon::start();
    assert_eq!(
        fs::metadata(&daemon.path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(daemon.path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let duplicate = server(&daemon.path).output().unwrap();
    assert!(!duplicate.status.success());

    let mut client = daemon.client();
    client.send(json!({"t":"pty.open","rid":44,"cmd":"/bin/sleep","args":["300"]}));
    let opened = client.receive();
    let pid = opened["pid"].as_u64().unwrap() as libc::pid_t;
    assert!(pid > 1);
    assert_eq!(
        unsafe { libc::kill(daemon.child.id() as libc::pid_t, libc::SIGTERM) },
        0
    );
    wait_until(|| daemon.child.try_wait().unwrap().is_some());
    assert!(!daemon.path.exists());
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
}

#[test]
fn stale_socket_is_recovered_but_public_parent_and_regular_file_are_rejected() {
    let directory = tempfile::Builder::new()
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir()
        .unwrap();
    let path = directory.path().join("stale.sock");
    drop(UnixListener::bind(&path).unwrap());
    let mut child = server(&path).spawn().unwrap();
    wait_until(|| UnixStream::connect(&path).is_ok());
    child.kill().unwrap();
    child.wait().unwrap();

    let public = directory.path().join("public");
    fs::create_dir(&public).unwrap();
    fs::set_permissions(&public, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(!server(&public.join("server.sock"))
        .output()
        .unwrap()
        .status
        .success());

    let regular = directory.path().join("regular");
    fs::write(&regular, b"keep").unwrap();
    assert!(!server(&regular).output().unwrap().status.success());
    assert_eq!(fs::read(regular).unwrap(), b"keep");
}

#[test]
fn kill_discards_the_session_and_reaps_the_child() {
    let daemon = Daemon::start();
    let mut client = daemon.client();
    client.send(json!({"t":"pty.open","rid":10,"cmd":"/bin/sleep","args":["300"]}));
    let opened = client.receive();
    let id = opened["id"].as_str().unwrap();
    let pid = opened["pid"].as_u64().unwrap() as libc::pid_t;
    assert!(pid > 1);
    client.send(json!({"t":"pty.kill","id":id}));
    client.send(json!({"t":"pty.list","rid":11}));
    assert_eq!(client.receive()["sessions"], json!([]));
    client.send(json!({"t":"pty.attach","id":id,"from":0}));
    assert_eq!(client.receive()["code"], "session_not_found");
    wait_until(|| unsafe { libc::kill(pid, 0) } == -1);
}
