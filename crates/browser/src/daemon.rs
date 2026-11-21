use crate::{
    actions::{run_action, Action, BrowserState},
    obscura::start_obscura,
    BrowserError, Cdp,
};
use serde_json::{json, Value};
#[cfg(windows)]
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::{
    fs::{OpenOptionsExt, PermissionsExt},
    net::UnixStream,
    process::CommandExt,
};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Endpoint;

fn runtime_dir() -> Result<PathBuf, BrowserError> {
    if let Some(dir) = std::env::var_os("GSH_BROWSER_DIR") {
        return Ok(PathBuf::from(dir));
    }
    dirs::data_dir()
        .map(|d| d.join("work.gorila.gshdesktop/browser"))
        .ok_or_else(|| BrowserError::Message("cannot locate data directory".into()))
}
fn endpoint_path(dir: &Path) -> PathBuf {
    dir.join("endpoint.json")
}
fn socket_path(dir: &Path) -> PathBuf {
    dir.join("browser.sock")
}
#[cfg(windows)]
fn ready_path(dir: &Path) -> PathBuf {
    dir.join("ready")
}
fn lock_path(dir: &Path) -> PathBuf {
    dir.join("daemon.lock")
}
#[cfg(unix)]
fn daemon_alive(dir: &Path) -> bool {
    let Ok(pid) = fs::read_to_string(lock_path(dir)) else {
        return false;
    };
    let Ok(pid) = pid.trim().parse::<i32>() else {
        return false;
    };
    // SAFETY: kill(pid, 0) only probes process existence and sends no signal.
    unsafe {
        libc::kill(pid, 0) == 0
            || std::io::Error::last_os_error().kind() == std::io::ErrorKind::PermissionDenied
    }
}
#[cfg(unix)]
fn lock_is_stale(dir: &Path) -> bool {
    if daemon_alive(dir) {
        return false;
    }
    fs::metadata(lock_path(dir))
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .is_some_and(|age| age > Duration::from_secs(5))
}
#[cfg(windows)]
fn daemon_alive(dir: &Path) -> bool {
    lock_path(dir).is_file()
}
struct LockCleanup(PathBuf);
impl Drop for LockCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
struct FileCleanup(PathBuf);
impl Drop for FileCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn is_ready(dir: &Path) -> bool {
    #[cfg(unix)]
    {
        endpoint_path(dir).is_file() && socket_path(dir).exists() && daemon_alive(dir)
    }
    #[cfg(windows)]
    {
        endpoint_path(dir).is_file() && ready_path(dir).is_file() && daemon_alive(dir)
    }
}

#[cfg(unix)]
fn create_lock(path: &Path) -> Result<std::fs::File, BrowserError> {
    let mut opts = OpenOptions::new();
    opts.write(true).create_new(true).mode(0o600);
    opts.open(path).map_err(BrowserError::from)
}
#[cfg(windows)]
fn create_lock(path: &Path) -> Result<std::fs::File, BrowserError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(BrowserError::from)
}

#[cfg(unix)]
fn serve(
    dir: &Path,
    state: &mut BrowserState,
    token: &str,
    idle: Duration,
) -> Result<(), BrowserError> {
    let path = socket_path(dir);
    let listener = gsh_ipc::bind_private_unix(&path)
        .map_err(|error| BrowserError::Message(error.to_string()))?;
    let _socket_cleanup = FileCleanup(path.clone());
    listener.set_nonblocking(true)?;
    let mut active = Instant::now();
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_secs(10)))?;
                let mut line = String::new();
                BufReader::new(stream.try_clone()?).read_line(&mut line)?;
                let req: Value = serde_json::from_str(&line)?;
                if !req["token"]
                    .as_str()
                    .is_some_and(|received| gsh_ipc::token_matches(token, received))
                {
                    writeln!(stream, "{{\"ok\":false,\"error\":\"invalid token\"}}")?;
                    continue;
                }
                active = Instant::now();
                let verb = req["verb"].as_str().unwrap_or_default();
                let result = if verb == "stop" {
                    Ok("stopped".to_owned())
                } else {
                    run_action(
                        state,
                        &Action {
                            verb: verb.to_owned(),
                            args: req["args"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect(),
                            page: req["page"].as_str().unwrap_or("main").to_owned(),
                        },
                    )
                };
                let stop = verb == "stop" && result.is_ok();
                let response = match result {
                    Ok(s) => json!({"ok":true,"result":s}),
                    Err(e) => json!({"ok":false,"error":e.to_string()}),
                };
                writeln!(stream, "{response}")?;
                if stop {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if active.elapsed() >= idle {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.into()),
        }
    }
    let _ = fs::remove_file(path);
    Ok(())
}

#[cfg(windows)]
fn serve(
    dir: &Path,
    state: &mut BrowserState,
    token: &str,
    idle: Duration,
) -> Result<(), BrowserError> {
    let port: u16 = std::fs::read_to_string(endpoint_path(dir))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .and_then(|v| v["port"].as_u64())
        .unwrap_or(0) as u16;
    let listener = TcpListener::bind(("127.0.0.1", port))?;
    let _ready_cleanup = FileCleanup(ready_path(dir));
    fs::write(ready_path(dir), b"ready")?;
    listener.set_nonblocking(true)?;
    let mut active = Instant::now();
    loop {
        match listener.accept() {
            Ok((mut stream, _)) => {
                let mut line = String::new();
                BufReader::new(stream.try_clone()?).read_line(&mut line)?;
                let req: Value = serde_json::from_str(&line)?;
                if !req["token"]
                    .as_str()
                    .is_some_and(|received| gsh_ipc::token_matches(token, received))
                {
                    writeln!(stream, "{{\"ok\":false,\"error\":\"invalid token\"}}")?;
                    continue;
                }
                active = Instant::now();
                let verb = req["verb"].as_str().unwrap_or_default();
                let result = if verb == "stop" {
                    Ok("stopped".into())
                } else {
                    run_action(
                        state,
                        &Action {
                            verb: verb.into(),
                            args: req["args"]
                                .as_array()
                                .into_iter()
                                .flatten()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect(),
                            page: req["page"].as_str().unwrap_or("main").into(),
                        },
                    )
                };
                let stop = verb == "stop" && result.is_ok();
                let out = match result {
                    Ok(s) => json!({"ok":true,"result":s}),
                    Err(e) => json!({"ok":false,"error":e.to_string()}),
                };
                writeln!(stream, "{out}")?;
                if stop {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if active.elapsed() >= idle {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

pub fn daemon_main() -> Result<(), BrowserError> {
    let dir = runtime_dir()?;
    fs::create_dir_all(&dir)?;
    let mut lock = create_lock(&lock_path(&dir)).map_err(|e| {
        BrowserError::Message(format!(
            "browser daemon already running or lock unavailable: {e}"
        ))
    })?;
    let _cleanup = LockCleanup(lock_path(&dir));
    writeln!(lock, "{}", std::process::id())?;
    let mut rng = [0u8; 32];
    getrandom::fill(&mut rng).map_err(|e| BrowserError::Message(e.to_string()))?;
    let token = rng.iter().map(|b| format!("{b:02x}")).collect::<String>();
    #[cfg(unix)]
    let endpoint = json!({"transport":"unix","path":socket_path(&dir),"token":token});
    #[cfg(windows)]
    let endpoint = {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        drop(listener);
        json!({"transport":"tcp","port":port,"token":token})
    };
    let ep = endpoint_path(&dir);
    fs::write(&ep, endpoint.to_string())?;
    let _endpoint_cleanup = FileCleanup(ep.clone());
    #[cfg(unix)]
    fs::set_permissions(&ep, fs::Permissions::from_mode(0o600))?;
    let obscura = start_obscura(false)?;
    fs::write(dir.join("obscura.pid"), obscura.child.id().to_string())?;
    let cdp = Cdp::connect(&obscura.websocket_endpoint(), &obscura.token)?;
    let mut state = BrowserState {
        obscura,
        cdp,
        pages: Default::default(),
        refs: Default::default(),
        temp: std::env::temp_dir(),
    };
    let secs = std::env::var("GSH_BROWSER_IDLE_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1800);
    let result = serve(&dir, &mut state, &token, Duration::from_secs(secs));
    drop(state);
    let _ = fs::remove_file(dir.join("obscura.pid"));
    let _ = fs::remove_file(ep);
    result
}

pub fn client_command(args: &[String]) -> Result<String, BrowserError> {
    let jsonout = args.iter().any(|arg| arg == "--json");
    if args.first().map(String::as_str) == Some("status") {
        let status = if is_ready(&runtime_dir()?) {
            "running"
        } else {
            "stopped"
        };
        return Ok(if jsonout {
            json!({"ok":true,"result":status}).to_string()
        } else {
            status.into()
        });
    }
    if args.first().map(String::as_str) == Some("start") {
        ensure_daemon()?;
        return Ok(if jsonout {
            json!({"ok":true,"result":"started"}).to_string()
        } else {
            "started".into()
        });
    }
    let dir = runtime_dir()?;
    fs::create_dir_all(&dir)?;
    let mut i = 0;
    let mut verb = String::new();
    let mut page = "main".to_owned();
    let mut jsonout = false;
    let mut rest: Vec<String> = Vec::new();
    while i < args.len() {
        match args[i].as_str() {
            "--page" => {
                i += 1;
                page = args
                    .get(i)
                    .ok_or_else(|| BrowserError::Message("--page requires a name".into()))?
                    .clone();
            }
            "--json" => jsonout = true,
            "daemon" => return Err(BrowserError::Message("daemon is internal".into())),
            x if verb.is_empty() => verb = x.into(),
            x => rest.push(x.into()),
        }
        i += 1;
    }
    if verb.is_empty() {
        return Err(BrowserError::Message("usage: gsh browser <goto|back|forward|reload|snapshot|click|fill|press|eval|text|shot|wait|pages|close|start|stop|status|install>".into()));
    }
    if verb == "goto" && rest.first().is_some_and(|s| s.starts_with("file:")) {
        return Err(BrowserError::Message("file: URLs are blocked".into()));
    }
    if verb != "stop" {
        ensure_daemon()?;
    }
    let ep: Value = serde_json::from_str(&fs::read_to_string(endpoint_path(&dir))?)?;
    let req = json!({"token":ep["token"],"verb":verb,"args":rest,"page":page});
    #[cfg(unix)]
    let mut stream = {
        let path = ep["path"]
            .as_str()
            .ok_or_else(|| BrowserError::Message("invalid daemon endpoint".into()))?;
        UnixStream::connect(path)?
    };
    #[cfg(windows)]
    let mut stream = TcpStream::connect(("127.0.0.1", ep["port"].as_u64().unwrap_or(0) as u16))?;
    stream.set_read_timeout(Some(Duration::from_secs(45)))?;
    writeln!(stream, "{req}")?;
    let mut response = String::new();
    BufReader::new(stream).read_line(&mut response)?;
    let v: Value = serde_json::from_str(&response)?;
    if v["ok"] == true {
        if jsonout {
            Ok(v.to_string())
        } else {
            Ok(v["result"].as_str().unwrap_or("").to_owned())
        }
    } else {
        Err(BrowserError::Message(
            v["error"].as_str().unwrap_or("browser error").into(),
        ))
    }
}

fn ensure_daemon() -> Result<(), BrowserError> {
    crate::find_binary()?;
    let dir = runtime_dir()?;
    fs::create_dir_all(&dir)?;
    if is_ready(&dir) {
        return Ok(());
    }
    if lock_path(&dir).exists() {
        #[cfg(unix)]
        if lock_is_stale(&dir) {
            let _ = fs::remove_file(lock_path(&dir));
        }
        if lock_path(&dir).exists() {
            let end = Instant::now() + Duration::from_secs(20);
            while Instant::now() < end {
                if is_ready(&dir) {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(50));
            }
            return Err(BrowserError::Message(
                "timed out waiting for browser daemon startup".into(),
            ));
        }
    }
    let exe = std::env::current_exe()?;
    let mut c = Command::new(exe);
    c.args(["browser", "daemon"]);
    #[cfg(unix)]
    unsafe {
        c.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x00000008 | 0x00000200);
    }
    c.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let end = Instant::now() + Duration::from_secs(20);
    while Instant::now() < end {
        if is_ready(&dir) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Err(BrowserError::Message(
        "timed out starting browser daemon".into(),
    ))
}
