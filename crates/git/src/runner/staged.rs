use std::io::{self, Read};
use std::process::{Child, Output};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use crate::GitError;

pub(super) fn read_output(
    mut child: Child,
    max_bytes: usize,
    timeout: Duration,
) -> Result<Output, GitError> {
    let deadline = Instant::now() + timeout;
    let result = collect_output(&mut child, max_bytes, deadline);
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}

fn collect_output(
    child: &mut Child,
    max_bytes: usize,
    deadline: Instant,
) -> Result<Output, GitError> {
    let stdout = child.stdout.take().ok_or(GitError::Io {
        detail: "stdout do git indisponível".to_owned(),
    })?;
    let stderr = child.stderr.take().ok_or(GitError::Io {
        detail: "stderr do git indisponível".to_owned(),
    })?;
    let stdout = read_pipe(stdout, (max_bytes as u64).saturating_add(1))?;
    let stderr = read_pipe(stderr, 64 * 1024)?;
    let stdout = await_pipe(stdout, deadline)?;
    if stdout.len() > max_bytes {
        return Err(GitError::StagedDiffTooLarge { max_bytes });
    }
    let stderr = await_pipe(stderr, deadline)?;
    loop {
        if let Some(status) = child.try_wait().map_err(io_failure)? {
            return Ok(Output {
                status,
                stdout,
                stderr,
            });
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(GitError::StagedDiffTimeout);
        }
        thread::sleep(remaining.min(Duration::from_millis(5)));
    }
}

fn read_pipe(
    pipe: impl Read + Send + 'static,
    limit: u64,
) -> Result<Receiver<io::Result<Vec<u8>>>, GitError> {
    let (send, receive) = mpsc::channel();
    thread::Builder::new()
        .spawn(move || {
            let mut bytes = Vec::new();
            let result = pipe.take(limit).read_to_end(&mut bytes).map(|_| bytes);
            let _ = send.send(result);
        })
        .map_err(io_failure)?;
    Ok(receive)
}

fn await_pipe(pipe: Receiver<io::Result<Vec<u8>>>, deadline: Instant) -> Result<Vec<u8>, GitError> {
    match pipe.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(result) => result.map_err(io_failure),
        Err(RecvTimeoutError::Timeout) => Err(GitError::StagedDiffTimeout),
        Err(RecvTimeoutError::Disconnected) => Err(GitError::Io {
            detail: "leitura da saída do git interrompida".to_owned(),
        }),
    }
}

fn io_failure(error: io::Error) -> GitError {
    GitError::Io {
        detail: error.to_string(),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    #[test]
    fn staged_diff_deadline_covers_blocked_output_and_process_exit() {
        for script in ["exec sleep 5", "exec 1>&- 2>&-; exec sleep 5"] {
            let child = Command::new("sh")
                .args(["-c", script])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let start = Instant::now();
            assert_eq!(
                read_output(child, 1024, Duration::from_millis(50)).unwrap_err(),
                GitError::StagedDiffTimeout
            );
            assert!(start.elapsed() < Duration::from_secs(2));
        }
    }
}
