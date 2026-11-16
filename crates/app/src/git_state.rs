//! Estado de apresentação e operações assíncronas do controle de versão.
use crate::git_ops;
use gsh_git::{FileDiff, GitCommitInfo, GitError, GitRepoStatus};
use iced::Task;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
const HISTORY_LIMIT: usize = 100;
const REFRESH_TICKS: u16 = 60;

#[derive(Debug, Clone)]
pub enum Action {
    Refresh,
    Stage(String),
    Unstage(String),
    Commit(String),
    Diff(String, bool),
}
#[derive(Debug, Clone)]
pub enum Value {
    Snapshot {
        root: PathBuf,
        status: Box<GitRepoStatus>,
        history: Result<Vec<GitCommitInfo>, GitError>,
        renames: BTreeMap<String, String>,
        selected_diff: Option<Result<FileDiff, GitError>>,
    },
    Diff(FileDiff),
    Mutated,
}
#[derive(Debug, Clone)]
pub struct ResultMsg {
    pub repo: PathBuf,
    pub generation: u64,
    pub result: Result<Value, GitError>,
    pub action: Action,
}
#[derive(Debug, Default)]
pub struct GitState {
    pub repo: Option<PathBuf>,
    pub root: Option<PathBuf>,
    pub status: Option<GitRepoStatus>,
    pub history: Vec<GitCommitInfo>,
    pub diff: Option<FileDiff>,
    pub error: Option<GitError>,
    pub message: String,
    pub busy: bool,
    pub history_open: bool,
    pub committing: bool,
    pub renames: BTreeMap<String, String>,
    generation: u64,
    ticks: u16,
}
impl GitState {
    pub fn set_repo(&mut self, repo: Option<PathBuf>) {
        let generation = self.generation.wrapping_add(1);
        *self = Self {
            repo,
            generation,
            ..Self::default()
        };
    }
    pub fn can_commit(&self) -> bool {
        !self.busy
            && !self.message.trim().is_empty()
            && self.status.as_ref().is_some_and(|s| !s.staged.is_empty())
    }
    pub fn should_refresh(&mut self) -> bool {
        if self.repo.is_none() || self.busy {
            return false;
        }
        if self.status.is_none() && self.error.is_none() {
            return true;
        }
        self.ticks = self.ticks.saturating_add(1);
        self.ticks >= REFRESH_TICKS
    }
    pub fn begin(&mut self, action: &Action) -> Option<(PathBuf, u64)> {
        if self.busy {
            return None;
        }
        let repo = self.repo.clone()?;
        self.busy = true;
        self.committing = matches!(action, Action::Commit(_));
        self.ticks = 0;
        self.generation = self.generation.wrapping_add(1);
        Some((repo, self.generation))
    }
    pub fn start(&mut self, action: Action) -> Task<ResultMsg> {
        let Some((repo, generation)) = self.begin(&action) else {
            return Task::none();
        };
        let old = match &action {
            Action::Unstage(path) | Action::Diff(path, true) => self.renames.get(path).cloned(),
            _ => None,
        };
        let conflict = match &action {
            Action::Diff(path, _) => self
                .status
                .as_ref()
                .is_some_and(|s| s.files.get(path) == Some(&gsh_git::GitFileStatus::Conflict)),
            _ => false,
        };
        let selected = if matches!(action, Action::Refresh) {
            self.diff.as_ref().map(|d| (d.path.clone(), d.staged))
        } else {
            None
        };
        Task::perform(
            async move {
                let task_repo = repo.clone();
                let task_action = action.clone();
                let result = tokio::task::spawn_blocking(move || {
                    run(&task_repo, task_action, old, conflict, selected)
                })
                .await
                .unwrap_or_else(|error| {
                    Err(GitError::Io {
                        detail: error.to_string(),
                    })
                });
                ResultMsg {
                    repo,
                    generation,
                    result,
                    action,
                }
            },
            |result| result,
        )
    }
    pub fn finish(&mut self, msg: ResultMsg) -> bool {
        if self.repo.as_ref() != Some(&msg.repo) || self.generation != msg.generation {
            return false;
        }
        self.busy = false;
        self.committing = false;
        match msg.result {
            Ok(Value::Snapshot {
                root,
                status,
                history,
                renames,
                selected_diff,
            }) => {
                let mut diff_error = None;
                if self.diff.is_some() {
                    match selected_diff {
                        Some(Ok(diff)) => self.diff = Some(diff),
                        Some(Err(error)) => {
                            self.diff = None;
                            diff_error = Some(error);
                        }
                        None => self.diff = None,
                    }
                }
                self.root = Some(root);
                self.status = Some(*status);
                self.renames = renames;
                match history {
                    Ok(history) => {
                        self.history = history;
                        self.error = None;
                    }
                    Err(error) => {
                        self.history.clear();
                        self.error = Some(error);
                    }
                }
                if diff_error.is_some() {
                    self.error = diff_error;
                }
            }
            Ok(Value::Diff(diff)) => {
                self.diff = Some(diff);
                self.error = None;
            }
            Ok(Value::Mutated) => {
                if matches!(msg.action, Action::Commit(_)) {
                    self.message.clear();
                }
                self.error = None;
                return true;
            }
            Err(error) => {
                if matches!(msg.action, Action::Diff(..)) {
                    self.diff = None;
                }
                self.error = Some(error);
            }
        }
        false
    }
}
fn run(
    workspace: &Path,
    action: Action,
    old: Option<String>,
    conflict: bool,
    selected: Option<(String, bool)>,
) -> Result<Value, GitError> {
    let repo = git_ops::root(workspace)?;
    match action {
        Action::Diff(path, staged) => {
            return if conflict {
                git_ops::conflict_diff(&repo, &path).map(Value::Diff)
            } else if let (true, Some(old)) = (staged, old) {
                git_ops::rename_diff(&repo, &path, &old).map(Value::Diff)
            } else {
                gsh_git::diff_file(&repo, path.as_ref(), staged).map(Value::Diff)
            }
        }
        Action::Stage(path) => {
            gsh_git::stage(&repo, &[path])?;
            return Ok(Value::Mutated);
        }
        Action::Unstage(path) => {
            let mut paths = vec![path];
            if let Some(old) = old {
                paths.push(old);
            }
            gsh_git::unstage(&repo, &paths)?;
            return Ok(Value::Mutated);
        }
        Action::Commit(message) => {
            gsh_git::commit(&repo, &message)?;
            return Ok(Value::Mutated);
        }
        Action::Refresh => {}
    }
    let status = gsh_git::status(&repo)?;
    let history = gsh_git::log(&repo, HISTORY_LIMIT);
    let renames = git_ops::renames(&repo)?;
    let selected_diff = selected.and_then(|(path, staged)| {
        let files = if staged {
            &status.staged
        } else {
            &status.unstaged
        };
        files.get(&path).map(|file_status| {
            if *file_status == gsh_git::GitFileStatus::Conflict {
                git_ops::conflict_diff(&repo, &path)
            } else if let Some(old) = renames.get(&path).filter(|_| staged) {
                git_ops::rename_diff(&repo, &path, old)
            } else {
                gsh_git::diff_file(&repo, path.as_ref(), staged)
            }
        })
    });
    Ok(Value::Snapshot {
        root: repo,
        status: Box::new(status),
        history,
        renames,
        selected_diff,
    })
}

#[cfg(test)]
#[path = "git_state_tests.rs"]
mod tests;
