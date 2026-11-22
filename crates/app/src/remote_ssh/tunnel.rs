use std::fs::Permissions;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use super::{closed_error, timeout};

pub(super) struct Endpoint {
    target: String,
    socket: String,
}

impl Endpoint {
    pub(super) fn new(target: String, socket: String) -> io::Result<Self> {
        let valid_target = |s: &str| {
            !s.is_empty()
                && !s.starts_with('-')
                && s.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_.-:".contains(&b))
        };
        let valid = match target.split_once('@') {
            Some((user, host)) => valid_target(user) && valid_target(host) && !user.contains(':'),
            None => valid_target(&target),
        };
        if !valid || target.len() > 255 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Alvo SSH inválido",
            ));
        }
        if !socket.starts_with('/')
            || socket.len() > 107
            || socket
                .bytes()
                .any(|b| b.is_ascii_control() || b":%".contains(&b))
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Informe um caminho absoluto válido para o socket do gsh-server",
            ));
        }
        Ok(Self { target, socket })
    }

    pub(super) fn connect(
        &self,
        closed: &AtomicBool,
        deadline: Instant,
    ) -> io::Result<(UnixStream, Tunnel)> {
        let dir = tempfile::Builder::new()
            .prefix("gsh-ssh-")
            .permissions(Permissions::from_mode(0o700))
            .tempdir_in("/tmp")?;
        let path = dir.path().join("remote.sock");
        let mut command = Command::new("ssh");
        command
            .args(["-N", "-T"])
            .args([
                "-oBatchMode=yes",
                "-oStrictHostKeyChecking=yes",
                "-oUpdateHostKeys=no",
                "-oVerifyHostKeyDNS=no",
                "-oNoHostAuthenticationForLocalhost=no",
                "-oPermitLocalCommand=no",
                "-oConnectTimeout=8",
                "-oExitOnForwardFailure=yes",
                "-oServerAliveInterval=5",
                "-oServerAliveCountMax=2",
                "-oForwardAgent=no",
                "-oForwardX11=no",
                "-oControlMaster=no",
                "-oControlPath=none",
                "-oForkAfterAuthentication=no",
                "-oStreamLocalBindMask=0177",
                "-oStreamLocalBindUnlink=no",
            ])
            .arg("-L")
            .arg(format!("{}:{}", path.display(), self.socket))
            .arg("--")
            .arg(&self.target)
            .stdin(Stdio::null())
            .stdout(Stdio::null());
        let mut tunnel = Tunnel {
            child: command.spawn()?,
            _dir: dir,
        };
        let stream = loop {
            if closed.load(Ordering::Acquire) {
                return Err(closed_error());
            }
            if let Some(status) = tunnel.child.try_wait()? {
                return Err(io::Error::other(format!(
                    "O processo SSH encerrou ({status}). Confira a autenticação e o known_hosts."
                )));
            }
            match UnixStream::connect(&path) {
                Ok(stream) => break stream,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
                    ) => {}
                Err(error) => return Err(error),
            }
            if Instant::now() >= deadline {
                return Err(timeout("Tempo esgotado ao abrir o túnel SSH"));
            }
            thread::sleep(Duration::from_millis(30));
        };
        Ok((stream, tunnel))
    }
}

pub(super) struct Tunnel {
    child: Child,
    _dir: tempfile::TempDir,
}

impl Drop for Tunnel {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
