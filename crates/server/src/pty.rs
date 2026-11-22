use std::{
    collections::{BTreeMap, VecDeque},
    io::{self, Read, Write},
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use serde::{Deserialize, Serialize};

const HISTORY_BYTES: usize = 4 * 1024 * 1024;
const PUMP_BYTES: usize = 64 * 1024;
pub(crate) const MAX_INPUT_BYTES: usize = 16 * 1024;
const TICK: Duration = Duration::from_millis(8);

#[derive(Deserialize)]
pub(crate) struct Open {
    cmd: String,
    #[serde(default)]
    args: Vec<String>,
    cwd: Option<PathBuf>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default = "rows")]
    rows: u16,
    #[serde(default = "cols")]
    cols: u16,
    #[serde(default)]
    flow: bool,
}

fn rows() -> u16 {
    24
}

fn cols() -> u16 {
    80
}

pub(crate) fn dimensions(rows: u16, cols: u16) -> io::Result<PtySize> {
    if !(1..=500).contains(&rows) || !(1..=500).contains(&cols) {
        return Err(io::Error::other(
            "terminal dimensions must be between 1 and 500",
        ));
    }
    Ok(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })
}

#[derive(Debug, Serialize)]
pub(crate) struct Info {
    pub(crate) id: String,
    pub(crate) pid: u32,
    cmd: String,
    rows: u16,
    cols: u16,
    len: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit: Option<u32>,
}

pub(crate) struct Session {
    pub(crate) id: String,
    pub(crate) pid: u32,
    pub(crate) flow: bool,
    command: String,
    output: Arc<Mutex<Output>>,
    commands: mpsc::SyncSender<Command>,
    stop: Arc<AtomicBool>,
}

enum Command {
    Input(Vec<u8>),
    Resize(PtySize),
}

struct Output {
    retained: VecDeque<u8>,
    end: u64,
    size: PtySize,
    exit: Option<(u32, Instant)>,
}

pub(crate) enum Event {
    Output(u64, Vec<u8>),
    Exit(u32),
}

impl Session {
    pub(crate) fn open(spec: Open) -> io::Result<Self> {
        let size = dimensions(spec.rows, spec.cols)?;
        let valid = |s: &str, max| s.len() <= max && !s.contains('\0');
        if !valid(&spec.cmd, 512)
            || spec.args.len() > 64
            || !spec.args.iter().all(|arg| valid(arg, 8192))
            || spec.env.len() > 256
        {
            return Err(io::Error::other("invalid terminal parameters"));
        }
        let cwd = spec
            .cwd
            .filter(|path| !path.as_os_str().is_empty())
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .ok_or_else(|| io::Error::other("HOME is unset; specify cwd"))?;
        if !cwd.is_absolute() || cwd.as_os_str().as_bytes().contains(&0) || !cwd.is_dir() {
            return Err(io::Error::other(
                "cwd is not an absolute directory on the host",
            ));
        }
        let login_shell = spec.cmd.is_empty();
        let command = if login_shell {
            std::env::var("SHELL")
                .ok()
                .filter(|shell| !shell.trim().is_empty())
                .unwrap_or_else(|| "/bin/sh".into())
        } else {
            spec.cmd
        };
        if !valid(&command, 512) {
            return Err(io::Error::other("invalid host login shell"));
        }
        let mut cmd = CommandBuilder::new(&command);
        cmd.cwd(cwd);
        cmd.env("GSH_REMOTE", "1");
        cmd.env_remove("GSH_STATUS_SOCK");
        cmd.env_remove("GSH_STATUS_PORT");
        cmd.env_remove("GSH_STATUS_TOKEN");
        cmd.env("TERM", "xterm-256color");
        for key in ["TERM", "COLORTERM"] {
            if let Some(value) = spec.env.get(key).filter(|value| !value.is_empty()) {
                if !valid(value, 128) {
                    return Err(io::Error::other("invalid terminal environment value"));
                }
                cmd.env(key, value);
            }
        }
        if login_shell && spec.args.is_empty() {
            cmd.arg("-l");
        } else {
            cmd.args(spec.args);
        }

        let pair = native_pty_system()
            .openpty(size)
            .map_err(io::Error::other)?;
        let fd = pair
            .master
            .as_raw_fd()
            .ok_or_else(|| io::Error::other("PTY has no fd"))?;
        // The master and its duplicated I/O handles share file status flags.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(io::Error::last_os_error());
        }
        let mut reader = pair.master.try_clone_reader().map_err(io::Error::other)?;
        let mut writer = pair.master.take_writer().map_err(io::Error::other)?;
        let output = Arc::new(Mutex::new(Output {
            retained: VecDeque::new(),
            end: 0,
            size,
            exit: None,
        }));
        let (commands, inbox) = mpsc::sync_channel(16);
        let stop = Arc::new(AtomicBool::new(false));

        let (started, ready) = mpsc::sync_channel(0);
        let worker_output = output.clone();
        let worker_stop = stop.clone();
        thread::Builder::new()
            .name("gsh-pty".into())
            .spawn(move || {
                let master = pair.master;
                let child = pair.slave.spawn_command(cmd).map_err(io::Error::other);
                drop(pair.slave);
                let mut child = match child {
                    Ok(child) => child,
                    Err(error) => {
                        let _ = started.send(Err(error));
                        return;
                    }
                };
                let Some(pid) = child.process_id() else {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = started.send(Err(io::Error::other("PTY child has no pid")));
                    return;
                };
                let mut pending: Option<Vec<u8>> = None;
                let mut offset = 0;
                if started.send(Ok(pid)).is_err() {
                    worker_stop.store(true, Ordering::Release);
                }
                let code = loop {
                    if worker_stop.load(Ordering::Acquire) {
                        // portable-pty creates a process group with the unreaped child as its leader.
                        unsafe { libc::kill(-(pid as libc::pid_t), libc::SIGHUP) };
                        let _ = child.kill();
                        break child.wait().map_or(1, |status| status.exit_code());
                    }
                    let read = drain(&mut reader, &worker_output);
                    match child.try_wait() {
                        Ok(Some(status)) => break status.exit_code(),
                        Ok(None) => {}
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break 1,
                    }
                    let mut worked = read;
                    if let Some(bytes) = pending.as_ref() {
                        match writer.write(&bytes[offset..]) {
                            Ok(0) => pending = None,
                            Ok(count) => {
                                worked = true;
                                offset += count;
                                if offset == bytes.len() {
                                    pending = None;
                                }
                            }
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                                ) => {}
                            Err(_) => pending = None,
                        }
                    }
                    if pending.is_none() {
                        match inbox.try_recv() {
                            Ok(Command::Input(bytes)) => {
                                pending = Some(bytes);
                                offset = 0;
                                worked = true;
                            }
                            Ok(Command::Resize(size)) => {
                                if master.resize(size).is_ok() {
                                    worker_output.lock().unwrap().size = size;
                                }
                                worked = true;
                            }
                            Err(mpsc::TryRecvError::Empty) => {}
                            Err(mpsc::TryRecvError::Disconnected) => {
                                worker_stop.store(true, Ordering::Release);
                                continue;
                            }
                        }
                    }
                    if !worked {
                        thread::sleep(TICK);
                    }
                };
                // The child can exit between a nonblocking read and try_wait.
                drain(&mut reader, &worker_output);
                drop((writer, reader, master));
                worker_output.lock().unwrap().exit = Some((code, Instant::now()));
            })?;
        let pid = ready.recv().map_err(io::Error::other)??;
        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            pid,
            flow: spec.flow,
            command,
            output,
            commands,
            stop,
        })
    }

    pub(crate) fn info(&self) -> Info {
        let output = self.output.lock().unwrap();
        Info {
            id: self.id.clone(),
            pid: self.pid,
            cmd: self.command.clone(),
            rows: output.size.rows,
            cols: output.size.cols,
            len: output.end,
            exit: output.exit.map(|(code, _)| code),
        }
    }

    pub(crate) fn finished(&self) -> Option<Instant> {
        self.output.lock().unwrap().exit.map(|(_, time)| time)
    }

    pub(crate) fn has_offset(&self, next: u64) -> bool {
        next <= self.output.lock().unwrap().end
    }

    pub(crate) fn event(&self, next: u64, limit: usize) -> Option<Event> {
        let output = self.output.lock().unwrap();
        let start = output.end - output.retained.len() as u64;
        let offset = next.max(start);
        if offset >= output.end {
            return output.exit.map(|(code, _)| Event::Exit(code));
        }
        if limit == 0 {
            return None;
        }
        let from = (offset - start) as usize;
        let to = (from + limit).min(output.retained.len());
        Some(Event::Output(
            offset,
            output.retained.range(from..to).copied().collect(),
        ))
    }

    pub(crate) fn input(&self, bytes: Vec<u8>) -> io::Result<()> {
        self.enqueue(Command::Input(bytes))
    }

    pub(crate) fn resize(&self, size: PtySize) -> io::Result<()> {
        self.enqueue(Command::Resize(size))
    }

    fn enqueue(&self, command: Command) -> io::Result<()> {
        self.commands
            .try_send(command)
            .map_err(|error| match error {
                mpsc::TrySendError::Full(_) => io::Error::other("terminal input queue is full"),
                mpsc::TrySendError::Disconnected(_) => io::Error::other("terminal has exited"),
            })
    }

    pub(crate) fn kill(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.kill();
    }
}

fn drain(reader: &mut dyn Read, target: &Mutex<Output>) -> bool {
    let mut chunk = [0; 8192];
    let mut total = 0;
    while total < PUMP_BYTES {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(count) => {
                let mut output = target.lock().unwrap();
                let excess = (output.retained.len() + count).saturating_sub(HISTORY_BYTES);
                output.retained.drain(..excess);
                output.retained.extend(&chunk[..count]);
                output.end += count as u64;
                total += count;
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            // Linux reports EIO when the last slave closes.
            Err(_) => break,
        }
    }
    total > 0
}
