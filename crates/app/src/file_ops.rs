//! Operações seguras em itens da árvore do workspace.
use std::path::{Component, Path, PathBuf};

use crate::labels;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateKind {
    Folder,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOpError {
    InvalidName,
    AlreadyExists,
    PermissionDenied,
    Io,
}

impl FileOpError {
    pub fn label(self) -> &'static str {
        match self {
            Self::InvalidName => labels::FILES_INVALID_NAME,
            Self::AlreadyExists => labels::FILES_ALREADY_EXISTS,
            Self::PermissionDenied => labels::FILES_CREATE_PERMISSION,
            Self::Io => labels::FILES_CREATE_FAILED,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateForm {
    pub kind: CreateKind,
    /// Pasta onde o novo item será criado; vazia preserva o fluxo legado na raiz.
    pub target_dir: PathBuf,
    pub name: String,
    pub error: Option<FileOpError>,
    pub busy: bool,
}

#[derive(Debug, Clone)]
pub struct RenameForm {
    pub path: PathBuf,
    pub name: String,
    pub error: Option<FileOpError>,
    pub busy: bool,
}

impl RenameForm {
    pub fn new(path: impl Into<PathBuf>, name: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            name: name.into(),
            error: None,
            busy: false,
        }
    }
}

impl CreateForm {
    pub fn new(kind: CreateKind) -> Self {
        Self::new_at(kind, PathBuf::new())
    }

    pub fn new_at(kind: CreateKind, target_dir: impl Into<PathBuf>) -> Self {
        Self {
            kind,
            target_dir: target_dir.into(),
            name: String::new(),
            error: None,
            busy: false,
        }
    }
}

pub fn create(root: &Path, name: &str, kind: CreateKind) -> Result<(), FileOpError> {
    create_in(root, root, name, kind)
}

/// Cria um item diretamente em `target_dir`, validando que o destino permaneça
/// sob a raiz e que o nome seja um único componente.
pub fn create_in(
    root: &Path,
    target_dir: &Path,
    name: &str,
    kind: CreateKind,
) -> Result<(), FileOpError> {
    let name = name.trim();
    let mut components = Path::new(name).components();
    if name.is_empty()
        || name.contains(['/', '\\', '\0'])
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(FileOpError::InvalidName);
    }
    let root = root.canonicalize().map_err(map_io_error)?;
    let target_dir = target_dir.canonicalize().map_err(map_io_error)?;
    if !target_dir.starts_with(&root) || !target_dir.is_dir() {
        return Err(FileOpError::InvalidName);
    }
    let path = target_dir.join(name);
    let result = match kind {
        CreateKind::Folder => std::fs::create_dir(path),
        CreateKind::File => std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map(drop),
    };
    result.map_err(|error| match error.kind() {
        std::io::ErrorKind::AlreadyExists => FileOpError::AlreadyExists,
        std::io::ErrorKind::PermissionDenied => FileOpError::PermissionDenied,
        _ => FileOpError::Io,
    })
}

fn map_io_error(error: std::io::Error) -> FileOpError {
    match error.kind() {
        std::io::ErrorKind::PermissionDenied => FileOpError::PermissionDenied,
        _ => FileOpError::Io,
    }
}

/// Renames one workspace item without allowing the new name to escape its parent.
/// Returns the resulting path so the UI can keep the renamed item selected.
pub fn rename(root: &Path, path: &Path, new_name: &str) -> Result<PathBuf, FileOpError> {
    validate_name(new_name)?;
    let (root, path) = checked_item(root, path)?;
    if path == root {
        return Err(FileOpError::InvalidName);
    }
    let parent = path.parent().ok_or(FileOpError::InvalidName)?;
    let target = parent.join(new_name.trim());
    if path_exists(&target) {
        return Err(FileOpError::AlreadyExists);
    }
    std::fs::rename(&path, &target).map_err(map_io_error)?;
    Ok(target)
}

/// Removes a workspace item. macOS uses Finder's trash; other platforms delete permanently.
pub fn delete(root: &Path, path: &Path) -> Result<(), FileOpError> {
    let (root, path) = checked_item(root, path)?;
    if path == root {
        return Err(FileOpError::InvalidName);
    }
    #[cfg(target_os = "macos")]
    {
        let path = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let script = format!("tell application \"Finder\" to delete POSIX file \"{path}\"");
        let output = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|_| FileOpError::Io)?;
        return output.status.success().then_some(()).ok_or(FileOpError::Io);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let metadata = std::fs::symlink_metadata(&path).map_err(map_io_error)?;
        if metadata.file_type().is_dir() {
            std::fs::remove_dir_all(path).map_err(map_io_error)
        } else {
            std::fs::remove_file(path).map_err(map_io_error)
        }
    }
}

/// Copies or moves a tree item into a workspace directory without replacing
/// any existing destination. Symlinks are rejected to avoid copying outside root.
pub fn paste(
    root: &Path,
    source: &Path,
    target_dir: &Path,
    cut: bool,
) -> Result<PathBuf, FileOpError> {
    let (root, source) = checked_item(root, source)?;
    let target_dir = checked_directory(&root, target_dir)?;
    if source == root {
        return Err(FileOpError::InvalidName);
    }
    let name = source.file_name().ok_or(FileOpError::InvalidName)?;
    let destination = unique_destination(&target_dir, name)?;
    let source_metadata = std::fs::symlink_metadata(&source).map_err(map_io_error)?;
    if source_metadata.file_type().is_symlink() {
        return Err(FileOpError::InvalidName);
    }
    if source_metadata.is_dir() && target_dir.starts_with(&source) {
        return Err(FileOpError::InvalidName);
    }
    if cut {
        std::fs::rename(&source, &destination).map_err(map_io_error)?;
    } else {
        if let Err(error) = copy_item(&source, &destination) {
            let _ = std::fs::remove_dir_all(&destination);
            let _ = std::fs::remove_file(&destination);
            return Err(error);
        }
    }
    Ok(destination)
}

fn unique_destination(directory: &Path, name: &std::ffi::OsStr) -> Result<PathBuf, FileOpError> {
    let name = name.to_string_lossy();
    let original = directory.join(name.as_ref());
    if !path_exists(&original) {
        return Ok(original);
    }
    let dot = name.rfind('.').filter(|dot| *dot > 0);
    let (stem, extension) = dot.map_or((name.as_ref(), ""), |dot| name.split_at(dot));
    let mut number = 1_u64;
    loop {
        let suffix = if number == 1 {
            " copy".to_owned()
        } else {
            format!(" copy {number}")
        };
        let candidate = directory.join(format!("{stem}{suffix}{extension}"));
        if !path_exists(&candidate) {
            return Ok(candidate);
        }
        number = number.checked_add(1).ok_or(FileOpError::Io)?;
    }
}

fn path_exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn validate_name(name: &str) -> Result<(), FileOpError> {
    let name = name.trim();
    let mut components = Path::new(name).components();
    if name.is_empty()
        || name.contains(['/', '\\', '\0'])
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(FileOpError::InvalidName);
    }
    Ok(())
}

fn checked_item(root: &Path, path: &Path) -> Result<(PathBuf, PathBuf), FileOpError> {
    let root = root.canonicalize().map_err(map_io_error)?;
    let parent = path.parent().ok_or(FileOpError::InvalidName)?;
    let parent = parent.canonicalize().map_err(map_io_error)?;
    if !parent.starts_with(&root) {
        return Err(FileOpError::InvalidName);
    }
    let path = parent.join(path.file_name().ok_or(FileOpError::InvalidName)?);
    std::fs::symlink_metadata(&path).map_err(map_io_error)?;
    Ok((root, path))
}

fn checked_directory(root: &Path, path: &Path) -> Result<PathBuf, FileOpError> {
    let target = path.canonicalize().map_err(map_io_error)?;
    if !target.starts_with(root) || !target.is_dir() {
        return Err(FileOpError::InvalidName);
    }
    Ok(target)
}

fn copy_item(source: &Path, target: &Path) -> Result<(), FileOpError> {
    let metadata = std::fs::symlink_metadata(source).map_err(map_io_error)?;
    if metadata.file_type().is_symlink() {
        return Err(FileOpError::InvalidName);
    }
    if metadata.is_dir() {
        std::fs::create_dir(target).map_err(map_io_error)?;
        for entry in std::fs::read_dir(source).map_err(map_io_error)? {
            let entry = entry.map_err(map_io_error)?;
            let child_source = entry.path();
            let child_target = target.join(entry.file_name());
            copy_item(&child_source, &child_target)?;
        }
        return Ok(());
    }
    let mut input = std::fs::File::open(source).map_err(map_io_error)?;
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(map_io_error)?;
    std::io::copy(&mut input, &mut output).map_err(map_io_error)?;
    Ok(())
}

pub async fn create_async(
    root: PathBuf,
    name: String,
    kind: CreateKind,
) -> (PathBuf, Result<(), FileOpError>) {
    let target = root.clone();
    let result = tokio::task::spawn_blocking(move || create(&target, &name, kind))
        .await
        .unwrap_or(Err(FileOpError::Io));
    (root, result)
}

pub async fn create_in_async(
    root: PathBuf,
    target_dir: PathBuf,
    name: String,
    kind: CreateKind,
) -> (PathBuf, PathBuf, Result<(), FileOpError>) {
    let root_for_operation = root.clone();
    let target_for_operation = target_dir.clone();
    let result = tokio::task::spawn_blocking(move || {
        create_in(&root_for_operation, &target_for_operation, &name, kind)
    })
    .await
    .unwrap_or(Err(FileOpError::Io));
    (root, target_dir, result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn creates_items_without_overwriting_or_escaping_root() {
        let dir = std::env::temp_dir().join(format!(
            "gsh-create-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&dir).unwrap();
        assert_eq!(create(&dir, "folder", CreateKind::Folder), Ok(()));
        assert!(dir.join("folder").is_dir());
        assert_eq!(create(&dir, "file", CreateKind::File), Ok(()));
        std::fs::write(dir.join("file"), b"preserve").unwrap();
        assert_eq!(
            create(&dir, "file", CreateKind::File),
            Err(FileOpError::AlreadyExists)
        );
        assert_eq!(std::fs::read(dir.join("file")).unwrap(), b"preserve");
        for name in ["", ".", "..", "../escape", "a/b", "a\\b"] {
            assert_eq!(
                create(&dir, name, CreateKind::File),
                Err(FileOpError::InvalidName)
            );
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn creates_inline_in_a_descendant_and_rejects_a_destination_outside_root() {
        let dir = std::env::temp_dir().join(format!(
            "gsh-create-in-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join("root");
        let nested = root.join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        assert_eq!(
            create_in(&root, &nested, "file.md", CreateKind::File),
            Ok(())
        );
        assert!(nested.join("file.md").is_file());
        assert_eq!(
            create_in(&root, &dir, "escape", CreateKind::File),
            Err(FileOpError::InvalidName)
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rename_delete_copy_and_cut_stay_inside_root_and_preserve_collisions() {
        let dir = std::env::temp_dir().join(format!(
            "gsh-file-ops-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join("root");
        let source_dir = root.join("source");
        let target_dir = root.join("target");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::create_dir_all(&target_dir).unwrap();
        let source = source_dir.join("report.md");
        std::fs::write(&source, b"content").unwrap();

        let renamed = rename(&root, &source, "renamed.md").unwrap();
        assert_eq!(renamed, source_dir.join("renamed.md"));
        let first_copy = paste(&root, &renamed, &target_dir, false).unwrap();
        let second_copy = paste(&root, &renamed, &target_dir, false).unwrap();
        assert_eq!(first_copy.file_name().unwrap(), "renamed.md");
        assert_eq!(second_copy.file_name().unwrap(), "renamed copy.md");
        assert!(renamed.is_file());

        let moved = paste(&root, &renamed, &target_dir, true).unwrap();
        assert_eq!(moved.file_name().unwrap(), "renamed copy 2.md");
        assert!(!renamed.exists());
        assert_eq!(std::fs::read(&moved).unwrap(), b"content");
        assert_eq!(delete(&root, &moved), Ok(()));
        assert!(!moved.exists());
        assert_eq!(delete(&root, &root), Err(FileOpError::InvalidName));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn recursive_copy_rejects_symlinks_without_following_them() {
        use std::os::unix::fs::symlink;

        let dir = std::env::temp_dir().join(format!(
            "gsh-file-copy-symlink-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = dir.join("root");
        let folder = root.join("folder");
        let outside = dir.join("outside");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(&outside, b"secret").unwrap();
        symlink(&outside, folder.join("link")).unwrap();
        assert_eq!(
            paste(&root, &folder, &root, false),
            Err(FileOpError::InvalidName)
        );
        assert!(!root.join("folder copy").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
