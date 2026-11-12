//! Criação segura de itens na raiz do workspace.
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
    pub name: String,
    pub error: Option<FileOpError>,
    pub busy: bool,
}

impl CreateForm {
    pub fn new(kind: CreateKind) -> Self {
        Self {
            kind,
            name: String::new(),
            error: None,
            busy: false,
        }
    }
}

pub fn create(root: &Path, name: &str, kind: CreateKind) -> Result<(), FileOpError> {
    let name = name.trim();
    let mut components = Path::new(name).components();
    if name.is_empty()
        || name.contains(['/', '\\', '\0'])
        || !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
    {
        return Err(FileOpError::InvalidName);
    }
    let path = root.join(name);
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
}
