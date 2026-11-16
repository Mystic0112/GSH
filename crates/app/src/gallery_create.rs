//! Criação do documento de um modelo da Galeria na raiz do workspace
//! (`createFromTemplate` de `gsh_viewmodel.dart`, só o caminho local).
//!
//! Cria a subpasta (`relative_dir`) se faltar; nome fixo já existente é
//! devolvido sem reescrever; senão escolhe o primeiro nome livre e grava. O
//! `.env.gsh` sai do git via `.git/info/exclude`. Abrir o arquivo e atualizar a
//! árvore é com quem chama.

use std::path::{Path, PathBuf};

use crate::gallery_template::GalleryTemplate;
use crate::{git_ops, labels};

/// Erro tipado da criação (`FileOperationError` do Flutter); a frase nasce em
/// [`GalleryError::message`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GalleryError {
    /// Nenhuma pasta aberta no workspace (`noWorkspace`).
    NoWorkspace,
    /// Falha do SO ao listar ou criar a pasta, com a mensagem dele (`osFailure`).
    Os(String),
    /// Não foi possível gravar o arquivo (`writeFailed`).
    WriteFailed,
}

impl GalleryError {
    pub fn message(&self) -> String {
        match self {
            Self::NoWorkspace => labels::GALLERY_ERR_NO_WORKSPACE.to_owned(),
            Self::Os(detail) => detail.clone(),
            Self::WriteFailed => labels::GALLERY_ERR_WRITE_FAILED.to_owned(),
        }
    }
}

/// Cria (ou localiza) o arquivo do modelo e devolve o caminho dele.
pub fn create_from_template(
    root: &Path,
    template: GalleryTemplate,
) -> Result<PathBuf, GalleryError> {
    let dir = match template.relative_dir() {
        "" => root.to_path_buf(),
        relative => root.join(relative),
    };
    let mut taken = Vec::new();
    if dir.is_dir() {
        let entries = std::fs::read_dir(&dir).map_err(os_error)?;
        for entry in entries {
            taken.push(
                entry
                    .map_err(os_error)?
                    .file_name()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    } else if !template.relative_dir().is_empty() {
        std::fs::create_dir_all(&dir).map_err(os_error)?;
    }
    let file_name = template.file_name();
    if template.fixed_name()
        && taken
            .iter()
            .any(|n| n.to_lowercase() == file_name.to_lowercase())
    {
        return Ok(dir.join(file_name));
    }
    let name = if template.fixed_name() {
        file_name
    } else {
        template.unique_file_name(&taken)
    };
    let path = dir.join(&name);
    std::fs::write(&path, template.content()).map_err(|_| GalleryError::WriteFailed)?;
    if template == GalleryTemplate::WorkspaceEnv {
        git_ops::exclude_from_git(root, &name);
    }
    Ok(path)
}

pub async fn create_async(
    root: PathBuf,
    template: GalleryTemplate,
) -> Result<PathBuf, GalleryError> {
    tokio::task::spawn_blocking(move || create_from_template(&root, template))
        .await
        .unwrap_or_else(|error| Err(GalleryError::Os(error.to_string())))
}

fn os_error(error: std::io::Error) -> GalleryError {
    GalleryError::Os(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "gsh-gallery-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn writes_the_content_and_suffixes_when_the_name_is_taken() {
        let dir = TempDir::new();
        let first = create_from_template(&dir.0, GalleryTemplate::DbQuery).unwrap();
        assert_eq!(first, dir.0.join("query.dbq"));
        assert_eq!(
            std::fs::read_to_string(&first).unwrap(),
            "-- limit: 100\nSELECT 1;\n"
        );
        let second = create_from_template(&dir.0, GalleryTemplate::DbQuery).unwrap();
        assert_eq!(second, dir.0.join("query-2.dbq"));
        let third = create_from_template(&dir.0, GalleryTemplate::DbQuery).unwrap();
        assert_eq!(third, dir.0.join("query-3.dbq"));
    }

    #[test]
    fn tasks_creates_the_folder_and_reopens_the_existing_file_untouched() {
        let dir = TempDir::new();
        let path = create_from_template(&dir.0, GalleryTemplate::Tasks).unwrap();
        assert_eq!(path, dir.0.join(".gsh").join("tasks.json"));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            GalleryTemplate::Tasks.content()
        );
        std::fs::write(&path, "mine").unwrap();
        let again = create_from_template(&dir.0, GalleryTemplate::Tasks).unwrap();
        assert_eq!(again, path);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine");
        assert!(!dir.0.join(".gsh").join("tasks-2.json").exists());
    }

    #[test]
    fn notebook_writes_welcome_inside_its_folder() {
        let dir = TempDir::new();
        let path = create_from_template(&dir.0, GalleryTemplate::Notebook).unwrap();
        assert_eq!(path, dir.0.join("notes.notebook").join("welcome.md"));
        assert!(std::fs::read_to_string(path)
            .unwrap()
            .starts_with("---\ntitle: Welcome\n"));
    }

    #[test]
    fn write_failure_is_typed() {
        let dir = TempDir::new();
        let file_as_root = dir.0.join("not-a-dir");
        std::fs::write(&file_as_root, "x").unwrap();
        assert_eq!(
            create_from_template(&file_as_root, GalleryTemplate::Html),
            Err(GalleryError::WriteFailed)
        );
        assert!(matches!(
            create_from_template(&file_as_root, GalleryTemplate::Tasks),
            Err(GalleryError::Os(_))
        ));
    }

    #[test]
    fn error_messages_come_from_labels() {
        assert_eq!(
            GalleryError::NoWorkspace.message(),
            labels::GALLERY_ERR_NO_WORKSPACE
        );
        assert_eq!(GalleryError::Os("boom".into()).message(), "boom");
    }
}
