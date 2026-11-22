//! Validação e aprovação humana de envios LocalSend iniciados pela CLI.

use std::path::{Path, PathBuf};
use std::sync::mpsc::SyncSender;

use iced::widget::{column, row, text};
use iced::{Alignment, Element};
use serde_json::Value;

pub struct ApprovedFile {
    pub path: PathBuf,
    pub size: u64,
    pub snapshot: std::fs::File,
}

pub struct PendingSend {
    pub caller: String,
    pub peer: gsh_localsend::Peer,
    pub sender: crate::localsend_runtime::LocalSendSender,
    pub files: Vec<ApprovedFile>,
    pub reply: SyncSender<Value>,
    pub transfer_deadline: std::time::Instant,
    pub max_file_size: u64,
    pub deadline: std::time::Instant,
    pub busy: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

fn escape_peer_text(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_control()
                || matches!(
                    ch as u32,
                    0x061C
                        | 0x200E..=0x200F
                        | 0x2028..=0x202E
                        | 0x2066..=0x2069
                )
            {
                format!("\\u{{{:04X}}}", ch as u32)
            } else {
                ch.to_string()
            }
        })
        .collect()
}

pub fn approval_view<'a, M: Clone + 'a>(
    pending: &PendingSend,
    allow_once: M,
    deny: M,
    noop: M,
) -> Element<'a, M> {
    use crate::pane_close_dialog::{self, ButtonKind};
    use crate::theme;

    let mut files = column![].spacing(5);
    for file in &pending.files {
        files = files.push(
            text(format!(
                "{} · {} bytes",
                escape_peer_text(&file.path.display().to_string()),
                file.size
            ))
            .size(theme::SIZE_LABEL),
        );
    }
    let body = column![
        text(format!("Aba/agente: {}", escape_peer_text(&pending.caller)))
            .size(theme::DIALOG_MESSAGE_SIZE),
        text(format!(
            "Destinatário: {} ({})",
            escape_peer_text(&pending.peer.info.alias),
            pending.peer.ip
        ))
        .size(theme::DIALOG_MESSAGE_SIZE),
        text(format!(
            "Certificado SHA-256: {}",
            escape_peer_text(&pending.peer.info.fingerprint)
        ))
        .size(theme::SIZE_LABEL),
        files,
    ]
    .spacing(theme::DIALOG_TITLE_GAP);
    let actions = row![
        pane_close_dialog::action("Negar", ButtonKind::Outline, Some(deny.clone())),
        pane_close_dialog::action("Permitir", ButtonKind::Primary, Some(allow_once)),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center);
    let title = text("Um agente quer enviar arquivos")
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text)
        .into();
    pane_close_dialog::form(
        title,
        body.into(),
        actions.into(),
        Some(theme::DIALOG_CONTENT_WIDTH),
        deny,
        noop,
    )
}

#[cfg(target_os = "linux")]
#[cfg(test)]
pub fn snapshot_files(
    paths: &[String],
    workdir: &Path,
    home: &Path,
    max_files: usize,
    max_file_size: u64,
) -> Result<Vec<ApprovedFile>, String> {
    snapshot_files_with_policy(
        paths,
        workdir,
        home,
        max_files,
        max_file_size,
        std::time::Instant::now() + std::time::Duration::from_secs(300),
        &[],
    )
}

#[cfg(target_os = "linux")]
pub fn snapshot_files_with_policy(
    paths: &[String],
    workdir: &Path,
    home: &Path,
    max_files: usize,
    max_file_size: u64,
    deadline: std::time::Instant,
    protected_roots: &[PathBuf],
) -> Result<Vec<ApprovedFile>, String> {
    use std::io::{Read, Seek, Write};
    const MAX_TOTAL_BYTES: u64 = 512 * 1024 * 1024;
    if paths.is_empty() {
        return Err("no files were provided".into());
    }
    if paths.len() > max_files {
        return Err(format!("too many files (maximum {max_files})"));
    }
    let workdir = workdir
        .canonicalize()
        .map_err(|e| format!("cannot resolve agent working directory: {e}"))?;
    let home = home
        .canonicalize()
        .map_err(|e| format!("cannot resolve home directory: {e}"))?;
    let protected_roots = protected_roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .collect::<Vec<_>>();
    let mut total = 0_u64;
    paths
        .iter()
        .map(|raw| {
            if std::time::Instant::now() >= deadline {
                return Err("LocalSend preparation timed out".into());
            }
            let input = workdir.join(raw);
            let path = input
                .canonicalize()
                .map_err(|_| format!("file not found: {}", input.display()))?;
            if !path.starts_with(&workdir) && !path.starts_with(&home) {
                return Err(format!(
                    "refusing a path outside the agent working directory and home: {}",
                    input.display()
                ));
            }
            if is_sensitive(&path, &home)
                || protected_roots.iter().any(|root| path.starts_with(root))
            {
                return Err(format!(
                    "refusing to send a sensitive file: {}",
                    input.display()
                ));
            }
            let mut snapshot = || -> std::io::Result<ApprovedFile> {
                let source = open_canonical_file(&path)?;
                let before = source.metadata()?;
                if !before.is_file() || before.len() > max_file_size {
                    return Err(std::io::Error::other(
                        "not a regular file within the size limit",
                    ));
                }
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if before.nlink() != 1 {
                        return Err(std::io::Error::other(
                            "refusing to send a file with multiple hard links",
                        ));
                    }
                }
                if total.saturating_add(before.len()) > MAX_TOTAL_BYTES {
                    return Err(std::io::Error::other(
                        "total transfer exceeds the 512 MiB preparation limit",
                    ));
                }
                let mut output = sealed_snapshot_file()?;
                let mut input = source;
                let mut size = 0_u64;
                let mut buffer = [0_u8; 64 * 1024];
                loop {
                    if std::time::Instant::now() >= deadline {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::TimedOut,
                            "LocalSend preparation timed out",
                        ));
                    }
                    let read = input.read(&mut buffer)?;
                    if read == 0 {
                        break;
                    }
                    size = size.saturating_add(read as u64);
                    if size > max_file_size || total.saturating_add(size) > MAX_TOTAL_BYTES {
                        return Err(std::io::Error::other(
                            "total transfer exceeds its size limit",
                        ));
                    }
                    let chunk = buffer.get(..read).ok_or_else(|| {
                        std::io::Error::other("invalid LocalSend snapshot read length")
                    })?;
                    output.write_all(chunk)?;
                }
                let after = input.metadata()?;
                if size != before.len()
                    || size != after.len()
                    || before.modified()? != after.modified()?
                {
                    return Err(std::io::Error::other(
                        "file changed while preparing the transfer",
                    ));
                }
                seal_snapshot(&output)?;
                output.rewind()?;
                total = total.saturating_add(size);
                Ok(ApprovedFile {
                    path: path.clone(),
                    size,
                    snapshot: output,
                })
            };
            snapshot().map_err(|error| format!("cannot snapshot {}: {error}", path.display()))
        })
        .collect()
}

#[cfg(target_os = "linux")]
fn sealed_snapshot_file() -> std::io::Result<std::fs::File> {
    use std::os::fd::FromRawFd;
    let name = std::ffi::CString::new("gsh-localsend-snapshot")
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let fd =
        unsafe { libc::memfd_create(name.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(unsafe { std::fs::File::from_raw_fd(fd) })
}

#[cfg(target_os = "linux")]
fn seal_snapshot(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::fd::AsRawFd;
    let seals = libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE;
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, seals) } < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn open_canonical_file(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;

    let name = std::ffi::CString::new(path.as_os_str().as_bytes())?;
    // The canonical path was checked against the roots and denylist. Refuse
    // symlinks in every component if any part is replaced before opening it.
    let mut how: libc::open_how = unsafe { std::mem::zeroed() };
    how.flags = (libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NONBLOCK) as u64;
    how.resolve = libc::RESOLVE_NO_SYMLINKS;
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            libc::AT_FDCWD,
            name.as_ptr(),
            &how,
            std::mem::size_of_val(&how),
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error());
    }
    let file = unsafe { std::fs::File::from_raw_fd(fd as std::os::fd::RawFd) };
    let opened_path = std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))?;
    if opened_path != path {
        return Err(std::io::Error::other("file moved while opening it"));
    }
    Ok(file)
}

#[cfg(not(target_os = "linux"))]
#[cfg(test)]
pub fn snapshot_files(
    _paths: &[String],
    _workdir: &Path,
    _home: &Path,
    _max_files: usize,
    _max_file_size: u64,
) -> Result<Vec<ApprovedFile>, String> {
    Err("secure LocalSend agent file approval requires Linux".into())
}

#[cfg(not(target_os = "linux"))]
pub fn snapshot_files_with_policy(
    _paths: &[String],
    _workdir: &Path,
    _home: &Path,
    _max_files: usize,
    _max_file_size: u64,
    _deadline: std::time::Instant,
    _protected_roots: &[PathBuf],
) -> Result<Vec<ApprovedFile>, String> {
    Err("secure LocalSend agent file approval requires Linux".into())
}

#[cfg(target_os = "linux")]
fn is_sensitive(path: &Path, home: &Path) -> bool {
    let relative_home = path.strip_prefix(home).ok();
    if relative_home.is_some_and(|relative| {
        let mut components = relative.components();
        matches!(components.next(), Some(std::path::Component::Normal(name)) if name == ".ssh" || name == ".gnupg")
    }) {
        return true;
    }
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    let lower = name.to_ascii_lowercase();
    lower == ".env"
        || matches!(
            lower.as_str(),
            ".credentials.json" | ".claude.json" | "credentials.json"
        )
        || lower.starts_with(".env.")
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
        || lower.starts_with("id_rsa")
        || (relative_home.is_some()
            && path.components().any(|part| part.as_os_str() == ".gsh")
            && matches!(
                lower.as_str(),
                "secrets.json" | "db-secrets.json" | "credentials.json" | "team-secrets.json"
            ))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn path_validator_blocks_sensitive_files_and_symlink_escape() {
        let root = tempfile::tempdir().unwrap();
        let work = root.path().join("work");
        let home = root.path().join("home");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&work).unwrap();
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        std::fs::create_dir_all(home.join(".gsh")).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        let normal = work.join("readme.txt");
        let secret = home.join(".ssh/id_rsa");
        let gsh_secret = home.join(".gsh/db-secrets.json");
        let env_file = work.join(".env.local");
        let escaped = work.join("outside.pem");
        std::fs::write(&normal, "ok").unwrap();
        std::fs::write(&secret, "private").unwrap();
        std::fs::write(&gsh_secret, "private").unwrap();
        std::fs::write(&env_file, "private").unwrap();
        std::fs::write(outside.join("payload"), "outside").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.join("payload"), &escaped).unwrap();
        let allowed = snapshot_files(
            &[normal.to_string_lossy().into_owned()],
            &work,
            &home,
            10,
            1024,
        )
        .unwrap();
        assert_eq!(allowed[0].size, 2);
        assert!(snapshot_files(
            &[secret.to_string_lossy().into_owned()],
            &work,
            &home,
            10,
            1024
        )
        .is_err());
        assert!(snapshot_files(
            &[gsh_secret.to_string_lossy().into_owned()],
            &work,
            &home,
            10,
            1024,
        )
        .is_err());
        assert!(snapshot_files(
            &[env_file.to_string_lossy().into_owned()],
            &work,
            &home,
            10,
            1024
        )
        .is_err());
        #[cfg(unix)]
        assert!(snapshot_files(
            &[escaped.to_string_lossy().into_owned()],
            &work,
            &home,
            10,
            1024
        )
        .is_err());
    }

    #[test]
    fn snapshots_preserve_bytes_across_write_and_symlink_replacement() {
        use std::io::{Read, Seek, Write};

        let work = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let source = work.path().join("report.txt");
        let secret = other.path().join("secret.txt");
        std::fs::write(&source, b"original report").unwrap();
        std::fs::write(&secret, b"secret contents").unwrap();
        let mut files =
            snapshot_files(&["report.txt".into()], work.path(), work.path(), 1, 1024).unwrap();

        std::fs::write(&source, b"modified report").unwrap();
        std::fs::remove_file(&source).unwrap();
        std::os::unix::fs::symlink(&secret, &source).unwrap();
        assert!(open_canonical_file(&source).is_err());
        assert!(files[0].snapshot.write_all(b"overwrite").is_err());
        files[0].snapshot.rewind().unwrap();
        let mut bytes = Vec::new();
        files[0].snapshot.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"original report");
        assert_eq!(files[0].path, source);
    }

    #[test]
    fn path_validator_rejects_directories() {
        let root = tempfile::tempdir().unwrap();
        let work = root.path();
        let home = root.path();
        assert!(
            snapshot_files(&[work.to_string_lossy().into_owned()], work, home, 10, 1024).is_err()
        );
    }
}
