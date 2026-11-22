use super::{
    document::{validate_size, MAX_DOCUMENT},
    wire::invalid,
};
use std::{
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::process::{Child, Command};

pub(super) fn spawn(command: &str, root: &Path, file: Option<&Path>) -> io::Result<Child> {
    let args = shlex::split(command).ok_or_else(|| invalid("invalid command quoting"))?;
    let (program, args) = args.split_first().ok_or_else(|| invalid("empty command"))?;
    if file.is_some() && !args.iter().any(|arg| arg.contains("%FILE%")) {
        return Err(invalid("formatter command requires a %FILE% argument"));
    }
    let args = args.iter().map(|arg| match file.and_then(Path::to_str) {
        Some(file) => arg.replace("%FILE%", file),
        None => arg.clone(),
    });
    let program = if program.contains('/') {
        std::path::absolute(root.join(program))?
    } else {
        executable_on_path(program).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!("{program} not found on PATH"),
            )
        })?
    };
    Command::new(program)
        .args(args)
        .current_dir(root)
        .stdin(if file.is_some() {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stdout(if file.is_some() {
            Stdio::null()
        } else {
            Stdio::piped()
        })
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
}

pub fn executable_on_path(program: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).find_map(|dir| {
        let candidate = dir.join(program);
        candidate
            .metadata()
            .ok()
            .filter(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)?;
        // Keep the entry name for executables such as rustup's tool proxies.
        std::path::absolute(candidate).ok()
    })
}

pub async fn format_external(
    command: &str,
    root: &Path,
    file: &Path,
    text: &str,
) -> io::Result<String> {
    use std::io::{Read, Write};

    validate_size(text)?;
    // shortcut: the file has a temporary basename, use stdin when a formatter supports a virtual filename.
    let parent = file
        .parent()
        .ok_or_else(|| invalid("missing file directory"))?;
    let suffix = file
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();
    let mut temporary = tempfile::Builder::new()
        .prefix(".gsh-format-")
        .suffix(&suffix)
        .tempfile_in(parent)?;
    temporary.write_all(text.as_bytes())?;
    let temporary = temporary.into_temp_path();
    if temporary.to_str().is_none() {
        return Err(invalid("formatter needs a UTF-8 path"));
    }
    let mut child = spawn(command, root, Some(&temporary))?;
    let status = match tokio::time::timeout(Duration::from_secs(30), child.wait()).await {
        Ok(status) => status?,
        Err(timeout) => {
            let _ = child.kill().await;
            return Err(timeout.into());
        }
    };
    if !status.success() {
        return Err(io::Error::other(format!("formatter exited with {status}")));
    }
    // Formatter output must still be a regular file, even if rewritten with rename.
    let metadata = std::fs::symlink_metadata(&temporary)?;
    if !metadata.is_file() || metadata.len() > MAX_DOCUMENT as u64 {
        return Err(invalid("invalid formatter output"));
    }
    let mut result = String::new();
    std::fs::File::open(&temporary)?
        .take(MAX_DOCUMENT as u64 + 1)
        .read_to_string(&mut result)?;
    if result.len() > MAX_DOCUMENT {
        return Err(invalid("formatter output exceeds size limit"));
    }
    Ok(result)
}
