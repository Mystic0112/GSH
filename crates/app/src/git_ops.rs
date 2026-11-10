//! Consultas Git complementares que a API de domínio ainda não expõe.
use gsh_git::{FileDiff, GitError};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
};

fn command(repo: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let binary = gsh_git::resolve_git_binary();
    let output = Command::new(&binary)
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("LC_ALL", "C")
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GitError::GitNotInstalled {
                    binary: binary.display().to_string(),
                }
            } else {
                GitError::Io {
                    detail: e.to_string(),
                }
            }
        })?;
    if !output.status.success() {
        return Err(GitError::CommandFailed {
            command: args.join(" "),
            exit_code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(output.stdout)
}
pub fn root(workspace: &Path) -> Result<PathBuf, GitError> {
    let out =
        command(workspace, &["rev-parse", "--show-toplevel"]).map_err(|error| match error {
            GitError::CommandFailed { .. } => GitError::NotARepository {
                path: workspace.to_owned(),
            },
            other => other,
        })?;
    let path = String::from_utf8(out).map_err(|e| GitError::Io {
        detail: e.to_string(),
    })?;
    Ok(PathBuf::from(path.trim_end_matches(['\r', '\n'])))
}
pub fn renames(repo: &Path) -> Result<BTreeMap<String, String>, GitError> {
    let output = command(
        repo,
        &["diff", "--cached", "--name-status", "-z", "--find-renames"],
    )?;
    let output = String::from_utf8(output).map_err(|e| GitError::Io {
        detail: e.to_string(),
    })?;
    let mut fields = output.split('\0');
    let mut result = BTreeMap::new();
    while let Some(status) = fields.next().filter(|s| !s.is_empty()) {
        let Some(first) = fields.next() else {
            break;
        };
        if status.starts_with('R') || status.starts_with('C') {
            let Some(next) = fields.next() else {
                break;
            };
            if status.starts_with('R') {
                result.insert(next.to_owned(), first.to_owned());
            }
        }
    }
    Ok(result)
}
pub fn rename_diff(repo: &Path, new: &str, old: &str) -> Result<FileDiff, GitError> {
    let output = command(
        repo,
        &["diff", "--cached", "--find-renames", "--", old, new],
    )?;
    Ok(gsh_git::parse_unified_diff(
        &String::from_utf8_lossy(&output),
        new,
        true,
    ))
}

pub fn conflict_diff(repo: &Path, path: &str) -> Result<FileDiff, GitError> {
    let output = command(repo, &["diff", "--ours", "--", path])?;
    Ok(gsh_git::parse_unified_diff(
        &String::from_utf8_lossy(&output),
        path,
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Repo(PathBuf);
    impl Repo {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "gsh-git-ui-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(path.join("sub")).unwrap();
            let repo = Self(path);
            repo.git(&["init"]);
            repo.git(&["config", "user.name", "Test"]);
            repo.git(&["config", "user.email", "test@example.test"]);
            repo
        }
        fn git(&self, args: &[&str]) {
            let result = Command::new(gsh_git::resolve_git_binary())
                .arg("-C")
                .arg(&self.0)
                .args(args)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
    }
    impl Drop for Repo {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn subdirectory_resolves_root_and_unstages_both_sides_of_rename() {
        let repo = Repo::new();
        fs::write(repo.0.join("old"), "first\nsecond\n").unwrap();
        repo.git(&["add", "old"]);
        repo.git(&["commit", "-m", "initial"]);
        repo.git(&["mv", "old", "new"]);
        let root = root(&repo.0.join("sub")).unwrap();
        assert_eq!(root, repo.0);
        let names = renames(&root).unwrap();
        assert_eq!(names.get("new").map(String::as_str), Some("old"));
        let diff = rename_diff(&root, "new", "old").unwrap();
        assert_eq!(diff.path, "new");
        gsh_git::unstage(&root, &["new", "old"]).unwrap();
        assert!(gsh_git::status(&root).unwrap().staged.is_empty());
    }
}
