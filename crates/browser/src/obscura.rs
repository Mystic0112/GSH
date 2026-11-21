use crate::BrowserError;
use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
pub struct Obscura {
    pub child: Child,
    pub port: u16,
    pub token: String,
}
impl Drop for Obscura {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
pub fn find_binary() -> Result<std::path::PathBuf, BrowserError> {
    let name = if cfg!(windows) {
        "obscura.exe"
    } else {
        "obscura"
    };
    if let Some(p) = std::env::var_os("GSH_OBSCURA") {
        let p = std::path::PathBuf::from(p);
        if p.is_file() {
            return Ok(p);
        }
        return Err(BrowserError::Message(
            "GSH_OBSCURA does not point to a file".into(),
        ));
    }
    let dir = dirs::data_dir()
        .ok_or_else(|| BrowserError::Message("cannot locate data directory".into()))?;
    let p = dir.join("work.gorila.gshdesktop/obscura/0.2.7").join(name);
    if p.is_file() {
        return Ok(p);
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&paths) {
            let p = d.join(name);
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err(BrowserError::Message(
        "Obscura is not installed; run `gsh browser install`".into(),
    ))
}
pub fn start_obscura(allow_private: bool) -> Result<Obscura, BrowserError> {
    let bin = find_binary()?;
    let mut rng = [0u8; 32];
    getrandom::fill(&mut rng).map_err(|e| BrowserError::Message(e.to_string()))?;
    let token = rng.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let l = TcpListener::bind("127.0.0.1:0")?;
    let port = l.local_addr()?.port();
    drop(l);
    let ready =
        std::env::temp_dir().join(format!("gsh-obscura-{}-{port}.ready", std::process::id()));
    let mut cmd = Command::new(bin);
    cmd.args([
        "serve",
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
        "--ready-file",
    ])
    .arg(&ready)
    .arg("--quiet")
    .env("OBSCURA_CDP_TOKEN", &token)
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    if allow_private {
        cmd.arg("--allow-private-network");
    }
    let child = cmd.spawn()?;
    let mut o = Obscura { child, port, token };
    let end = Instant::now() + Duration::from_secs(20);
    while Instant::now() < end {
        if o.child.try_wait()?.is_some() {
            return Err(BrowserError::Message(
                "Obscura exited before becoming ready".into(),
            ));
        }
        if ready.is_file() {
            let _ = std::fs::remove_file(ready);
            return Ok(o);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(BrowserError::Message(
        "timed out waiting for Obscura".into(),
    ))
}
