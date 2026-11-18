//! `gsh telemetry` (plano 66): verbos de consulta da Caixa Preta e o
//! wrapper que observa um processo (`gsh telemetry <cmd>`).
//!
//! O wrapper é burro de propósito: sobe o filho (PTY aninhado quando há TTY,
//! pipes quando não há), repassa os bytes intocados pra tela e, em paralelo,
//! manda as linhas pro app em lotes. Quem interpreta é o parser do app.

use std::io::{IsTerminal, Read, Write};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use gsh_core::wire::cmd_request;
use serde_json::{json, Map, Value};

use crate::env::self_tab_id;
use crate::transport;

const HELP: &str = include_str!("../text/telemetry_help.txt");
const QUERY_TIMEOUT: Duration = Duration::from_secs(60);
const WAIT_TIMEOUT: Duration = Duration::from_secs(11 * 60);
const BATCH_LINES: usize = 200;
const BATCH_DELAY: Duration = Duration::from_millis(100);

const QUERY_VERBS: [&str; 14] = [
    "errors", "logs", "events", "runs", "show", "wait", "mark", "resolve", "ignore", "reopen",
    "clear", "probes", "perf", "replay",
];
const BOOL_FLAGS: [&str; 9] = [
    "--app",
    "--since-edit",
    "--since-run",
    "--new",
    "--include-ignored",
    "--include-resolved",
    "--live",
    "--any-new",
    "--json",
];

#[derive(Debug, PartialEq)]
pub struct QuerySpec {
    pub wire: String,
    pub args: Map<String, Value>,
    pub tab_id: Option<String>,
    pub timeout: Duration,
}

#[derive(Debug, PartialEq)]
pub struct WrapOpts {
    pub name: Option<String>,
    pub quiet: bool,
    pub argv: Vec<String>,
}

pub fn run(args: &[String]) -> u8 {
    let Some(first) = args.first().map(String::as_str) else {
        let _ = std::io::stderr().write_all(HELP.as_bytes());
        return 2;
    };
    if first == "--help" || first == "-h" {
        let _ = std::io::stdout().write_all(HELP.as_bytes());
        return 0;
    }
    if QUERY_VERBS.contains(&first) {
        let rest = args.get(1..).unwrap_or_default();
        return exec_query(first, rest);
    }
    exec_wrap(args)
}

fn fail_json(kind: &str, message: &str) -> u8 {
    println!("{}", json!({"error": {"kind": kind, "message": message}}));
    1
}

pub fn parse_query(verb: &str, rest: &[String]) -> Result<QuerySpec, (String, String)> {
    let mut a = Map::new();
    let mut positionals: Vec<String> = Vec::new();
    let mut tab_id: Option<String> = None;
    let mut i = 0usize;
    while let Some(tok) = rest.get(i) {
        if BOOL_FLAGS.contains(&tok.as_str()) {
            a.insert(key_of(tok), json!(true));
        } else if let Some(stripped) = tok.strip_prefix("--") {
            let (k, v) = match stripped.split_once('=') {
                Some((k, v)) => (k.to_string(), v.to_string()),
                None => {
                    i += 1;
                    let Some(v) = rest.get(i).cloned() else {
                        return Err(("error".into(), format!("missing value for {tok}")));
                    };
                    (stripped.to_string(), v)
                }
            };
            if k == "tab-id" {
                tab_id = Some(v);
            } else {
                a.insert(k.replace('-', "_"), json!(v));
            }
        } else {
            positionals.push(tok.clone());
        }
        i += 1;
    }

    let wire = match verb {
        "errors" | "logs" | "events" | "runs" | "probes" | "perf" | "wait" | "clear" | "replay" => {
            format!("telemetry-{verb}")
        }
        "show" => {
            let Some(id) = positionals.first().cloned() else {
                return Err((
                    "error".into(),
                    "missing id (e_xxxx | ev_xxxx | r_xx)".into(),
                ));
            };
            a.insert("id".into(), json!(id));
            "telemetry-show".into()
        }
        "mark" => {
            let action = positionals.first().map(String::as_str);
            let name = positionals.get(1);
            match (action, name) {
                (Some(act @ ("start" | "end")), Some(nm)) => {
                    a.insert("action".into(), json!(act));
                    a.insert("name".into(), json!(nm));
                    "telemetry-mark".into()
                }
                _ => {
                    return Err((
                        "error".into(),
                        "usage: gsh telemetry mark start|end <name>".into(),
                    ));
                }
            }
        }
        "resolve" | "ignore" | "reopen" => {
            let Some(id) = positionals.first().cloned() else {
                return Err(("error".into(), "missing case id (e_xxxx)".into()));
            };
            a.insert("id".into(), json!(id));
            a.insert("status".into(), json!(verb));
            "telemetry-triage".into()
        }
        _ => return Err(("error".into(), format!("unknown verb {verb}"))),
    };

    let timeout = if verb == "wait" {
        WAIT_TIMEOUT
    } else {
        QUERY_TIMEOUT
    };
    Ok(QuerySpec {
        wire,
        args: a,
        tab_id,
        timeout,
    })
}

fn exec_query(verb: &str, rest: &[String]) -> u8 {
    let spec = match parse_query(verb, rest) {
        Ok(s) => s,
        Err((kind, msg)) => return fail_json(&kind, &msg),
    };
    let tab = spec.tab_id.or_else(self_tab_id);
    let req = cmd_request(&spec.wire, tab.as_deref(), Some(Value::Object(spec.args)));
    let resp = match transport::request(req, spec.timeout) {
        Ok(r) => r,
        Err(e) => return fail_json("transport", &e.to_string()),
    };
    if resp.is_ok() {
        let data = resp.data();
        println!("{}", json!({"ok": data}));
        return 0;
    }
    let raw = resp.error_text();
    match raw.split_once(": ") {
        Some((kind, msg)) if !kind.contains(' ') => fail_json(kind, msg),
        _ => fail_json("error", &raw),
    }
}

fn key_of(flag: &str) -> String {
    flag.trim_start_matches("--").replace('-', "_")
}

pub fn parse_wrap(args: &[String]) -> Result<WrapOpts, String> {
    let mut name = None;
    let mut quiet = false;
    let mut i = 0usize;
    while let Some(arg) = args.get(i) {
        match arg.as_str() {
            "--" => {
                i += 1;
                break;
            }
            "-q" | "--quiet" => quiet = true,
            "--name" => {
                i += 1;
                let Some(v) = args.get(i).cloned() else {
                    return Err("gsh telemetry: missing value for --name".into());
                };
                name = Some(v);
            }
            s if s.starts_with("--name=") => {
                name = Some(s.strip_prefix("--name=").unwrap_or("").to_string());
            }
            _ => break,
        }
        i += 1;
    }
    let argv: Vec<String> = args.get(i..).unwrap_or_default().to_vec();
    if argv.is_empty() {
        return Err("gsh telemetry: missing command to run (see --help)".into());
    }
    Ok(WrapOpts { name, quiet, argv })
}

enum Msg {
    Lines(&'static str, Vec<String>),
    Done,
}

fn exec_wrap(args: &[String]) -> u8 {
    let opts = match parse_wrap(args) {
        Ok(o) => o,
        Err(msg) => {
            eprintln!("{msg}");
            return 2;
        }
    };
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let command = opts.argv.join(" ");

    if !transport::transport_configured() {
        eprintln!("telemetry: not inside GSH, running without capture");
        return exit_u8(passthrough(&opts.argv));
    }

    let tab = self_tab_id();
    let open = cmd_request(
        "telemetry-open",
        tab.as_deref(),
        Some(json!({
            "cwd": cwd,
            "command": command,
            "name": opts.name,
            "pid": std::process::id(),
        })),
    );
    let resp = match transport::request(open, QUERY_TIMEOUT) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("telemetry: {e} (running without capture)");
            return exit_u8(passthrough(&opts.argv));
        }
    };
    if !resp.is_ok() {
        eprintln!("telemetry: {} (running without capture)", resp.error_text());
        return exit_u8(passthrough(&opts.argv));
    }

    let data = resp.data();
    let run_id = data
        .get("run")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let mut env: Vec<(String, String)> = Vec::new();
    if let Some(m) = data.get("env").and_then(Value::as_object) {
        for (k, v) in m {
            if let Some(s) = v.as_str() {
                env.push((k.clone(), s.to_string()));
            }
        }
    }

    let (tx, rx) = mpsc::channel::<Msg>();
    let tab_sender = tab.clone();
    let run_for_sender = run_id.clone();
    let sender = thread::spawn(move || {
        let mut pending: Vec<(&'static str, Vec<String>)> = Vec::new();
        let mut last = Instant::now();
        let flush = |pending: &mut Vec<(&'static str, Vec<String>)>| {
            for (stream, lines) in pending.drain(..) {
                let req = cmd_request(
                    "telemetry-ingest",
                    tab_sender.as_deref(),
                    Some(json!({
                        "run": run_for_sender,
                        "stream": stream,
                        "lines": lines,
                    })),
                );
                let _ = transport::request(req, QUERY_TIMEOUT);
            }
        };
        loop {
            match rx.recv_timeout(BATCH_DELAY) {
                Ok(Msg::Lines(stream, lines)) => {
                    match pending.last_mut() {
                        Some((s, l)) if *s == stream => l.extend(lines),
                        _ => pending.push((stream, lines)),
                    }
                    let total: usize = pending.iter().map(|(_, l)| l.len()).sum();
                    if total >= BATCH_LINES || last.elapsed() >= BATCH_DELAY {
                        flush(&mut pending);
                        last = Instant::now();
                    }
                }
                Ok(Msg::Done) => {
                    flush(&mut pending);
                    return;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if !pending.is_empty() {
                        flush(&mut pending);
                        last = Instant::now();
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    flush(&mut pending);
                    return;
                }
            }
        }
    });

    let interactive = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let code = if interactive {
        run_pty(&opts.argv, &env, &tx)
    } else {
        run_pipes(&opts.argv, &env, &tx)
    };
    let _ = tx.send(Msg::Done);
    let _ = sender.join();

    let close = cmd_request(
        "telemetry-close",
        tab.as_deref(),
        Some(json!({"run": run_id, "exit_code": code})),
    );
    let close_data = transport::request(close, QUERY_TIMEOUT)
        .ok()
        .map(|r| r.data())
        .unwrap_or(Value::Null);
    if !opts.quiet {
        let errors = close_data
            .get("errors")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let warnings = close_data
            .get("warnings")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        eprintln!(
            "telemetry: run {run_id} · {errors} errors · {warnings} warnings · gsh telemetry errors --run {run_id}"
        );
    }
    exit_u8(code)
}

fn exit_u8(code: i32) -> u8 {
    u8::try_from(code.clamp(0, 255)).unwrap_or(1)
}

fn passthrough(argv: &[String]) -> i32 {
    let Some((prog, rest)) = argv.split_first() else {
        return 127;
    };
    match std::process::Command::new(prog).args(rest).status() {
        Ok(st) => exit_code_of(&st),
        Err(e) => {
            eprintln!("gsh telemetry: {prog}: {e}");
            127
        }
    }
}

fn exit_code_of(st: &std::process::ExitStatus) -> i32 {
    if let Some(c) = st.code() {
        return c;
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = st.signal() {
            return 128 + sig;
        }
    }
    1
}

struct LineSplitter {
    stream: &'static str,
    partial: Vec<u8>,
}

impl LineSplitter {
    fn new(stream: &'static str) -> Self {
        Self {
            stream,
            partial: Vec::new(),
        }
    }

    fn push(&mut self, data: &[u8], tx: &mpsc::Sender<Msg>) {
        self.partial.extend_from_slice(data);
        let mut lines = Vec::new();
        let mut start = 0usize;
        for (i, &b) in self.partial.iter().enumerate() {
            if b == b'\n' {
                let mut end = i;
                if end > start && self.partial.get(end - 1) == Some(&b'\r') {
                    end -= 1;
                }
                if let Some(slice) = self.partial.get(start..end) {
                    lines.push(String::from_utf8_lossy(slice).into_owned());
                }
                start = i + 1;
            }
        }
        if start > 0 {
            self.partial.drain(..start);
        }
        if !lines.is_empty() {
            let _ = tx.send(Msg::Lines(self.stream, lines));
        }
    }

    fn finish(&mut self, tx: &mpsc::Sender<Msg>) {
        if !self.partial.is_empty() {
            let rest = String::from_utf8_lossy(&self.partial).into_owned();
            self.partial.clear();
            if !rest.trim().is_empty() {
                let _ = tx.send(Msg::Lines(self.stream, vec![rest]));
            }
        }
    }
}

fn run_pipes(argv: &[String], env: &[(String, String)], tx: &mpsc::Sender<Msg>) -> i32 {
    use std::process::{Command, Stdio};
    let Some((prog, rest)) = argv.split_first() else {
        return 127;
    };
    let mut cmd = Command::new(prog);
    cmd.args(rest)
        .envs(env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("gsh telemetry: {prog}: {e}");
            return 127;
        }
    };
    let out = child.stdout.take();
    let err = child.stderr.take();
    let tx_out = tx.clone();
    let t_out = thread::spawn(move || pump(out, "out", tx_out, false));
    let tx_err = tx.clone();
    let t_err = thread::spawn(move || pump(err, "err", tx_err, true));
    let status = child.wait();
    let _ = t_out.join();
    let _ = t_err.join();
    match status {
        Ok(st) => exit_code_of(&st),
        Err(_) => 1,
    }
}

fn pump<R: Read>(src: Option<R>, stream: &'static str, tx: mpsc::Sender<Msg>, to_stderr: bool) {
    let Some(mut src) = src else { return };
    let mut split = LineSplitter::new(stream);
    let mut buf = [0u8; 8192];
    loop {
        match src.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                let slice = buf.get(..n).unwrap_or_default();
                if to_stderr {
                    let mut e = std::io::stderr().lock();
                    let _ = e.write_all(slice);
                    let _ = e.flush();
                } else {
                    let mut o = std::io::stdout().lock();
                    let _ = o.write_all(slice);
                    let _ = o.flush();
                }
                split.push(slice, &tx);
            }
        }
    }
    split.finish(&tx);
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn run_pty(argv: &[String], env: &[(String, String)], tx: &mpsc::Sender<Msg>) -> i32 {
    use std::ffi::CString;
    use std::os::unix::io::{AsRawFd, FromRawFd};

    let stdin_fd = std::io::stdin().as_raw_fd();
    let mut orig: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(stdin_fd, &mut orig) } != 0 {
        return run_pipes(argv, env, tx);
    }
    let mut ws: libc::winsize = unsafe { std::mem::zeroed() };
    unsafe { libc::ioctl(stdin_fd, libc::TIOCGWINSZ, &mut ws) };

    let cargv: Vec<CString> = argv
        .iter()
        .map(|a| CString::new(a.as_str()).unwrap_or_default())
        .collect();
    let Some(prog) = cargv.first() else {
        return 127;
    };
    let cenv: Vec<(CString, CString)> = env
        .iter()
        .map(|(k, v)| {
            (
                CString::new(k.as_str()).unwrap_or_default(),
                CString::new(v.as_str()).unwrap_or_default(),
            )
        })
        .collect();

    let mut master: libc::c_int = -1;
    let pid = unsafe {
        libc::forkpty(
            &mut master,
            std::ptr::null_mut(),
            &mut orig as *mut libc::termios,
            &mut ws as *mut libc::winsize,
        )
    };
    if pid < 0 {
        return run_pipes(argv, env, tx);
    }
    if pid == 0 {
        for (k, v) in &cenv {
            unsafe { libc::setenv(k.as_ptr(), v.as_ptr(), 1) };
        }
        let mut ptrs: Vec<*const libc::c_char> = cargv.iter().map(|c| c.as_ptr()).collect();
        ptrs.push(std::ptr::null());
        unsafe { libc::execvp(prog.as_ptr(), ptrs.as_ptr()) };
        unsafe { libc::_exit(127) };
    }

    let mut raw = orig;
    unsafe { libc::cfmakeraw(&mut raw) };
    unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &raw) };

    let master_w = unsafe { std::fs::File::from_raw_fd(libc::dup(master)) };
    thread::spawn(move || {
        let mut master_w = master_w;
        let mut stdin = std::io::stdin().lock();
        let mut buf = [0u8; 1024];
        loop {
            match stdin.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let slice = buf.get(..n).unwrap_or_default();
                    if master_w.write_all(slice).is_err() {
                        break;
                    }
                }
            }
        }
    });

    let mut split = LineSplitter::new("out");
    let mut buf = [0u8; 8192];
    let mut last_ws = (ws.ws_row, ws.ws_col);
    let mut pfd = libc::pollfd {
        fd: master,
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        pfd.revents = 0;
        let r = unsafe { libc::poll(&mut pfd, 1, 100) };
        if r < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            break;
        }
        let mut cur: libc::winsize = unsafe { std::mem::zeroed() };
        if unsafe { libc::ioctl(stdin_fd, libc::TIOCGWINSZ, &mut cur) } == 0
            && (cur.ws_row, cur.ws_col) != last_ws
        {
            last_ws = (cur.ws_row, cur.ws_col);
            unsafe { libc::ioctl(master, libc::TIOCSWINSZ, &cur) };
        }
        if r == 0 {
            continue;
        }
        if pfd.revents & libc::POLLIN != 0 {
            let n = unsafe { libc::read(master, buf.as_mut_ptr() as *mut libc::c_void, buf.len()) };
            if n <= 0 {
                break;
            }
            let n = usize::try_from(n).unwrap_or(0);
            let slice = buf.get(..n).unwrap_or_default();
            let mut o = std::io::stdout().lock();
            let _ = o.write_all(slice);
            let _ = o.flush();
            split.push(slice, tx);
        } else if pfd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            break;
        }
    }
    split.finish(tx);

    let mut status: libc::c_int = 0;
    unsafe { libc::waitpid(pid, &mut status, 0) };
    unsafe { libc::tcsetattr(stdin_fd, libc::TCSANOW, &orig) };
    unsafe { libc::close(master) };
    if libc::WIFEXITED(status) {
        libc::WEXITSTATUS(status)
    } else if libc::WIFSIGNALED(status) {
        128 + libc::WTERMSIG(status)
    } else {
        1
    }
}

#[cfg(not(unix))]
fn run_pty(argv: &[String], env: &[(String, String)], tx: &mpsc::Sender<Msg>) -> i32 {
    run_pipes(argv, env, tx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| (*x).to_string()).collect()
    }

    #[test]
    fn parse_query_handles_flags_and_verbs() {
        let q = parse_query(
            "errors",
            &v(&["--run", "r_42", "--new", "--since-edit", "--tab-id=t3"]),
        )
        .unwrap();
        assert_eq!(q.wire, "telemetry-errors");
        assert_eq!(q.args["run"], "r_42");
        assert_eq!(q.args["new"], true);
        assert_eq!(q.args["since_edit"], true);
        assert_eq!(q.tab_id.as_deref(), Some("t3"));

        let show = parse_query("show", &v(&["e_3f2a", "--app"])).unwrap();
        assert_eq!(show.wire, "telemetry-show");
        assert_eq!(show.args["id"], "e_3f2a");
        assert_eq!(show.args["app"], true);

        let triage = parse_query("resolve", &v(&["e_1234", "--reason", "fixed"])).unwrap();
        assert_eq!(triage.wire, "telemetry-triage");
        assert_eq!(triage.args["status"], "resolve");
        assert_eq!(triage.args["reason"], "fixed");
    }

    #[test]
    fn parse_wrap_handles_name_quiet_and_double_dash() {
        let w = parse_wrap(&v(&["--name", "api", "-q", "--", "npm", "run", "dev"])).unwrap();
        assert_eq!(w.name.as_deref(), Some("api"));
        assert!(w.quiet);
        assert_eq!(w.argv, vec!["npm", "run", "dev"]);

        assert!(parse_wrap(&v(&["-q"])).is_err());
        assert!(parse_wrap(&v(&["--name"])).is_err());
    }
}
