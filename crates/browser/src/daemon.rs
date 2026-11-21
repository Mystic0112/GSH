use crate::{
    actions::{run_action, Action, BrowserState},
    obscura::start_obscura,
    BrowserError, Cdp,
};
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};
#[derive(Clone)]
pub struct Endpoint;
pub fn daemon_main() -> Result<(), BrowserError> {
    let dir = dirs::data_dir()
        .ok_or_else(|| BrowserError::Message("cannot locate data directory".into()))?
        .join("work.gorila.gshdesktop/browser");
    std::fs::create_dir_all(&dir)?;
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(false)?;
    std::fs::write(
        dir.join("endpoint"),
        listener.local_addr()?.port().to_string(),
    )?;
    let obscura = start_obscura(false)?;
    let cdp = Cdp::connect(
        &format!("ws://127.0.0.1:{}/devtools/browser", obscura.port),
        &obscura.token,
    )?;
    let mut state = BrowserState {
        obscura,
        cdp,
        pages: Default::default(),
        refs: Default::default(),
        temp: std::env::temp_dir(),
    };
    for stream in listener.incoming() {
        let mut s = stream?;
        let mut line = String::new();
        BufReader::new(s.try_clone()?).read_line(&mut line)?;
        let req: Value = serde_json::from_str(&line)?;
        let a = Action {
            verb: req["verb"].as_str().unwrap_or("").into(),
            args: req["args"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect(),
            page: req["page"].as_str().unwrap_or("main").into(),
        };
        let response = run_action(&mut state, &a);
        let out = match response {
            Ok(s) => json!({"ok":true,"result":s}),
            Err(e) => json!({"ok":false,"error":e.to_string()}),
        };
        writeln!(s, "{out}")?;
    }
    #[allow(unreachable_code)]
    Ok(())
}
pub fn client_command(args: &[String]) -> Result<String, BrowserError> {
    let dir = dirs::data_dir()
        .ok_or_else(|| BrowserError::Message("cannot locate data directory".into()))?
        .join("work.gorila.gshdesktop/browser");
    std::fs::create_dir_all(&dir)?;
    let endpoint = dir.join("endpoint");
    if !endpoint.is_file() {
        let exe = std::env::current_exe()?;
        let mut c = std::process::Command::new(exe);
        c.args(["browser", "daemon"]);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x08000000);
        }
        c.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let end = std::time::Instant::now() + Duration::from_secs(20);
        while !endpoint.is_file() {
            if std::time::Instant::now() > end {
                return Err(BrowserError::Message(
                    "timed out starting browser daemon".into(),
                ));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    let port = std::fs::read_to_string(endpoint)?
        .trim()
        .parse::<u16>()
        .map_err(|e| BrowserError::Message(e.to_string()))?;
    let mut s = TcpStream::connect(("127.0.0.1", port))?;
    s.set_read_timeout(Some(Duration::from_secs(45)))?;
    let mut i = 0;
    let mut verb = String::new();
    let mut page = "main".to_owned();
    let mut jsonout = false;
    let mut rest: Vec<String> = Vec::new();
    while i < args.len() {
        match args[i].as_str() {
            "--page" => {
                i += 1;
                if let Some(x) = args.get(i) {
                    page = x.clone()
                }
            }
            "--json" => jsonout = true,
            "daemon" => return Err(BrowserError::Message("daemon is internal".into())),
            x if verb.is_empty() => verb = x.into(),
            x => rest.push(x.into()),
        }
        i += 1;
    }
    if verb.is_empty() {
        return Err(BrowserError::Message(
            "usage: gsh browser <goto|snapshot|...>".into(),
        ));
    }
    let req = json!({"verb":verb,"args":rest,"page":page});
    writeln!(s, "{req}")?;
    let mut response = String::new();
    BufReader::new(s).read_line(&mut response)?;
    let v: Value = serde_json::from_str(&response)?;
    if v["ok"] == true {
        let result = v["result"].as_str().unwrap_or("");
        if jsonout {
            Ok(v.to_string())
        } else {
            Ok(result.to_owned())
        }
    } else {
        Err(BrowserError::Message(
            v["error"].as_str().unwrap_or("browser error").into(),
        ))
    }
}
