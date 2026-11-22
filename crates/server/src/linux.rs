use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{self, ErrorKind},
    os::{
        fd::AsRawFd,
        unix::{
            ffi::OsStrExt,
            fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            net::UnixListener,
        },
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use crate::{protocol, pty::Session};

const MAX_CONNECTIONS: usize = 16;
const MAX_SESSIONS: usize = 32;
const TICK: Duration = Duration::from_millis(30);

#[derive(Default)]
pub(crate) struct Sessions(Mutex<SessionSet>);

#[derive(Default)]
struct SessionSet {
    visible: Vec<Arc<Session>>,
    stopping: Vec<Arc<Session>>,
    shutting_down: bool,
}

impl Sessions {
    pub(crate) fn open(&self, spec: crate::pty::Open) -> io::Result<Arc<Session>> {
        let mut sessions = self.0.lock().unwrap();
        if sessions.shutting_down {
            return Err(io::Error::other("server is shutting down"));
        }
        sessions
            .stopping
            .retain(|session| session.finished().is_none());
        let replace = if sessions.visible.len() + sessions.stopping.len() >= MAX_SESSIONS {
            let (_, index) = sessions
                .visible
                .iter()
                .enumerate()
                .filter_map(|(index, session)| session.finished().map(|time| (time, index)))
                .min()
                .ok_or_else(|| io::Error::other("terminal session limit reached"))?;
            Some(index)
        } else {
            None
        };
        let session = Arc::new(Session::open(spec)?);
        if let Some(index) = replace {
            sessions.visible.remove(index);
        }
        sessions.visible.push(session.clone());
        Ok(session)
    }

    pub(crate) fn with_sessions<T>(&self, f: impl FnOnce(&[Arc<Session>]) -> T) -> T {
        f(&self.0.lock().unwrap().visible)
    }

    pub(crate) fn kill(&self, id: &str) -> bool {
        let mut sessions = self.0.lock().unwrap();
        let Some(index) = sessions.visible.iter().position(|session| session.id == id) else {
            return false;
        };
        let session = sessions.visible.remove(index);
        session.kill();
        sessions
            .stopping
            .retain(|session| session.finished().is_none());
        if session.finished().is_none() {
            // Keep a stopping child under the session cap until the worker reaps it.
            sessions.stopping.push(session);
        }
        true
    }

    fn stop_all(&self) -> Vec<Arc<Session>> {
        let mut sessions = self.0.lock().unwrap();
        sessions.shutting_down = true;
        sessions
            .visible
            .iter()
            .chain(&sessions.stopping)
            .map(|session| {
                session.kill();
                session.clone()
            })
            .collect()
    }
}

pub(crate) fn serve(path: &Path) -> io::Result<()> {
    let stop = Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(libc::SIGTERM, stop.clone())?;
    signal_hook::flag::register(libc::SIGINT, stop.clone())?;
    let socket = Socket::bind(path)?;
    socket.listener.set_nonblocking(true)?;
    println!("gsh-server listening on {}", path.display());

    let sessions = Arc::new(Sessions::default());
    let connections = Arc::new(AtomicUsize::new(0));
    let result = loop {
        if stop.load(Ordering::Acquire) {
            break Ok(());
        }
        match socket.listener.accept() {
            Ok((stream, _)) if connections.load(Ordering::Relaxed) < MAX_CONNECTIONS => {
                connections.fetch_add(1, Ordering::Relaxed);
                let permit = ConnectionPermit(connections.clone());
                let sessions = sessions.clone();
                let stop = stop.clone();
                if let Err(error) =
                    thread::Builder::new()
                        .name("gsh-client".into())
                        .spawn(move || {
                            let _permit = permit;
                            let _ = protocol::connect(stream, &sessions, &stop);
                        })
                {
                    break Err(error);
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::WouldBlock => thread::sleep(TICK),
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => break Err(error),
        }
    };
    stop.store(true, Ordering::Release);
    let sessions = sessions.stop_all();
    let until = Instant::now() + Duration::from_secs(2);
    while Instant::now() < until
        && (sessions.iter().any(|session| session.finished().is_none())
            || connections.load(Ordering::Relaxed) > 0)
    {
        thread::sleep(TICK);
    }
    result
}

struct ConnectionPermit(Arc<AtomicUsize>);

impl Drop for ConnectionPermit {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

struct Socket {
    listener: UnixListener,
    path: PathBuf,
    inode: (u64, u64),
    _lock: File,
    _dir: File,
}

impl Socket {
    fn bind(path: &Path) -> io::Result<Self> {
        let name = path
            .file_name()
            .filter(|_| path.is_absolute())
            .ok_or_else(|| {
                io::Error::other("socket path must be an absolute path with a file name")
            })?;
        if path.as_os_str().as_bytes().len() > 107 {
            return Err(io::Error::other(
                "socket path exceeds the Linux limit of 107 bytes",
            ));
        }
        let parent = path.parent().unwrap();
        let mut builder = DirBuilder::new();
        builder.mode(0o700).recursive(true).create(parent)?;
        let dir = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(parent)?;
        let metadata = dir.metadata()?;
        let uid = unsafe { libc::geteuid() };
        if metadata.uid() != uid || metadata.mode() & 0o777 != 0o700 {
            return Err(io::Error::new(
                ErrorKind::PermissionDenied,
                "socket directory must be owned by the current user with mode 0700",
            ));
        }
        // Keep path operations in the validated directory even when its ancestors move.
        let dir_fd = PathBuf::from(format!("/proc/self/fd/{}", dir.as_raw_fd()));
        let mut lock_name = name.to_os_string();
        lock_name.push(".lock");
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(dir_fd.join(lock_name))?;
        let metadata = lock.metadata()?;
        if !metadata.is_file() || metadata.uid() != uid || metadata.mode() & 0o777 != 0o600 {
            return Err(io::Error::other("invalid server lock file"));
        }
        // The lock inode stays in place so all competing starts lock the same file.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == ErrorKind::WouldBlock {
                Err(io::Error::new(
                    ErrorKind::AddrInUse,
                    "gsh-server is already running",
                ))
            } else {
                Err(error)
            };
        }
        let pinned = dir_fd.join(name);
        match fs::symlink_metadata(&pinned) {
            Ok(metadata) if metadata.file_type().is_socket() && metadata.uid() == uid => {
                let probe =
                    socket2::Socket::new(socket2::Domain::UNIX, socket2::Type::STREAM, None)?;
                probe.set_nonblocking(true)?;
                match probe.connect(&socket2::SockAddr::unix(&pinned)?) {
                    Err(error) if error.kind() == ErrorKind::ConnectionRefused => {
                        fs::remove_file(&pinned)?
                    }
                    _ => {
                        return Err(io::Error::new(
                            ErrorKind::AddrInUse,
                            "socket already exists",
                        ))
                    }
                }
            }
            Ok(_) => {
                return Err(io::Error::new(
                    ErrorKind::AddrInUse,
                    "socket path is occupied",
                ))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let listener = UnixListener::bind(&pinned)?;
        let inode = identity(&fs::symlink_metadata(&pinned)?);
        let socket = Self {
            listener,
            path: pinned,
            inode,
            _lock: lock,
            _dir: dir,
        };
        fs::set_permissions(&socket.path, fs::Permissions::from_mode(0o600))?;
        Ok(socket)
    }
}

impl Drop for Socket {
    fn drop(&mut self) {
        if fs::symlink_metadata(&self.path).is_ok_and(|metadata| identity(&metadata) == self.inode)
        {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn identity(metadata: &fs::Metadata) -> (u64, u64) {
    (metadata.dev(), metadata.ino())
}
