//! Estado de apresentação e operações assíncronas do controle de versão.
use crate::git_harness;
use crate::git_ops;
use gsh_config::schema::Automation;
use gsh_git::{FileDiff, GitCommitInfo, GitError, GitRepoStatus};
use iced::Task;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[path = "git_diff_notes.rs"]
mod diff_notes;
use diff_notes::{NoteFile, Notes};
const HISTORY_LIMIT: usize = 100;
const REFRESH_TICKS: u16 = 60;

#[derive(Debug, Clone)]
pub enum Action {
    Refresh,
    Stage(String),
    Unstage(String),
    Commit(String),
    Diff(String, bool),
    SwitchBranch(String),
    SuggestCommit(Automation),
    FetchPr,
    FetchPrLog(usize, crate::git_pr::LogRequest),
}
#[derive(Debug, Clone)]
pub enum Value {
    Snapshot {
        root: PathBuf,
        status: Box<GitRepoStatus>,
        history: Result<Vec<GitCommitInfo>, GitError>,
        renames: BTreeMap<String, String>,
        selected_diff: Option<Result<FileDiff, GitError>>,
        branches: Vec<String>,
    },
    Diff {
        root: PathBuf,
        diff: FileDiff,
    },
    CommitMessage(String),
    Pr(Box<crate::git_pr::PrFeedback>),
    PrLog(usize, Result<String, String>),
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
    pub branches: Vec<String>,
    pub diff: Option<FileDiff>,
    pub diff_notes: Notes,
    pub active_note: Option<((usize, usize), String)>,
    pub note_agent: Option<String>,
    pub error: Option<GitError>,
    pub message: String,
    pub busy: bool,
    pub history_open: bool,
    pub branches_open: bool,
    pub committing: bool,
    pub suggesting: bool,
    pub fetching_pr: bool,
    pub pr: Option<crate::git_pr::PrFeedback>,
    pub pr_revision: u64,
    pub pr_agent: Option<String>,
    pub pr_notice: Option<&'static str>,
    pub renames: BTreeMap<String, String>,
    generation: u64,
    ticks: u16,
    notes_directory: Option<PathBuf>,
    note_file: Option<NoteFile>,
}
impl GitState {
    pub fn enable_note_storage(&mut self, directory: Option<PathBuf>) {
        self.notes_directory = directory;
    }

    pub fn start_note(&mut self, hunk: usize, line: usize) {
        if !self
            .diff
            .as_ref()
            .and_then(|diff| diff.hunks.get(hunk))
            .and_then(|hunk| hunk.lines.get(line))
            .is_some_and(|line| line.kind != gsh_git::DiffLineKind::Context)
        {
            return;
        }
        let draft = self
            .diff_notes
            .get(&(hunk, line))
            .cloned()
            .unwrap_or_default();
        self.active_note = Some(((hunk, line), draft));
    }

    pub fn update_note_draft(&mut self, text: String) {
        if let Some((_, draft)) = &mut self.active_note {
            *draft = text;
        }
    }

    pub fn save_note(&mut self) -> bool {
        if let Some((key, draft)) = &self.active_note {
            let mut notes = self.diff_notes.clone();
            let trimmed = draft.trim();
            if trimmed.is_empty() {
                notes.remove(key);
            } else {
                notes.insert(*key, trimmed.to_owned());
            }
            if !self.store_notes(notes) {
                return false;
            }
            self.active_note = None;
        }
        true
    }

    pub fn cancel_note(&mut self) {
        self.active_note = None;
    }

    pub fn remove_note(&mut self, hunk: usize, line: usize) {
        let mut notes = self.diff_notes.clone();
        notes.remove(&(hunk, line));
        if self.store_notes(notes)
            && self
                .active_note
                .as_ref()
                .is_some_and(|(key, _)| *key == (hunk, line))
        {
            self.active_note = None;
        }
    }

    pub fn close_diff(&mut self) {
        self.diff = None;
        self.note_file = None;
        self.diff_notes.clear();
        self.active_note = None;
    }

    pub fn store_notes(&mut self, notes: Notes) -> bool {
        let result = match (&self.note_file, &self.notes_directory) {
            (Some(file), _) => file.save(&self.diff_notes, &notes),
            (None, Some(_)) => Err(std::io::Error::other(
                "armazenamento das notas indisponível",
            )),
            (None, None) => Ok(()),
        };
        if let Err(error) = result {
            self.error = Some(GitError::Io {
                detail: format!("Falha ao salvar notas do diff: {error}"),
            });
            return false;
        }
        self.diff_notes = notes;
        self.error = None;
        true
    }

    fn show_diff(&mut self, root: &Path, diff: FileDiff) -> Result<(), GitError> {
        if self.root.as_deref() == Some(root) && self.diff.as_ref() == Some(&diff) {
            return Ok(());
        }
        self.close_diff();
        self.diff = Some(diff);
        let result = (|| -> std::io::Result<()> {
            if let (Some(directory), Some(diff)) = (&self.notes_directory, &self.diff) {
                let file = NoteFile::new(directory, root, diff)?;
                self.diff_notes = file.load()?;
                self.note_file = Some(file);
            }
            Ok(())
        })();
        result.map_err(|error| GitError::Io {
            detail: format!("Falha ao carregar notas do diff: {error}"),
        })
    }

    pub fn set_repo(&mut self, repo: Option<PathBuf>) {
        let generation = self.generation.wrapping_add(1);
        *self = Self {
            repo,
            generation,
            notes_directory: self.notes_directory.take(),
            ..Self::default()
        };
    }
    pub fn can_commit(&self) -> bool {
        !self.busy
            && !self.message.trim().is_empty()
            && self.status.as_ref().is_some_and(|s| !s.staged.is_empty())
    }
    pub fn can_suggest_commit(&self) -> bool {
        !self.busy
            && self
                .status
                .as_ref()
                .is_some_and(|status| !status.staged.is_empty())
    }
    pub fn can_switch_branch(&self) -> bool {
        !self.busy && self.status.as_ref().is_some_and(GitRepoStatus::is_clean)
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
        self.suggesting = matches!(action, Action::SuggestCommit(_));
        self.fetching_pr = matches!(action, Action::FetchPr);
        if self.fetching_pr {
            self.pr = None;
            self.pr_notice = None;
        }
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
        self.suggesting = false;
        self.fetching_pr = false;
        match msg.result {
            Ok(Value::Snapshot {
                root,
                status,
                history,
                renames,
                selected_diff,
                branches,
            }) => {
                let mut diff_error = None;
                if self.diff.is_some() {
                    match selected_diff {
                        Some(Ok(diff)) => diff_error = self.show_diff(&root, diff).err(),
                        Some(Err(error)) => {
                            self.close_diff();
                            diff_error = Some(error);
                        }
                        None => {
                            self.close_diff();
                        }
                    }
                }
                self.root = Some(root);
                if self
                    .pr
                    .as_ref()
                    .is_some_and(|pr| pr.branch != status.branch)
                {
                    self.pr = None;
                    self.pr_notice = None;
                }
                self.status = Some(*status);
                self.renames = renames;
                self.branches = branches;
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
            Ok(Value::Diff { root, diff }) => {
                self.error = self.show_diff(&root, diff).err();
                self.root = Some(root);
            }
            Ok(Value::CommitMessage(message)) => {
                self.message = message;
                self.error = None;
                return false;
            }
            Ok(Value::Pr(pr)) => {
                self.pr = Some(*pr);
                self.pr_revision = msg.generation;
                self.error = None;
            }
            Ok(Value::PrLog(index, log)) => {
                if let Some(check) = self.pr.as_mut().and_then(|pr| pr.checks.get_mut(index)) {
                    check.log = Some(log);
                }
                self.error = None;
            }
            Ok(Value::Mutated) => {
                if matches!(msg.action, Action::Commit(_)) {
                    self.message.clear();
                }
                if matches!(msg.action, Action::SwitchBranch(_)) {
                    self.pr = None;
                    self.pr_notice = None;
                }
                self.error = None;
                return true;
            }
            Err(error) => {
                if matches!(msg.action, Action::Diff(..)) {
                    self.close_diff();
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
            let diff = if conflict {
                git_ops::conflict_diff(&repo, &path)
            } else if let (true, Some(old)) = (staged, old) {
                git_ops::rename_diff(&repo, &path, &old)
            } else {
                gsh_git::diff_file(&repo, path.as_ref(), staged)
            }?;
            return Ok(Value::Diff { root: repo, diff });
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
        Action::SwitchBranch(branch) => {
            gsh_git::switch_branch(&repo, &branch)?;
            return Ok(Value::Mutated);
        }
        Action::SuggestCommit(automation) => {
            return git_harness::suggest_commit_message(&repo, &automation)
                .map(Value::CommitMessage);
        }
        Action::FetchPr => {
            return crate::git_pr::fetch_current_pr(&repo, git_ops::current_branch(&repo)?)
                .map(|pr| Value::Pr(Box::new(pr)));
        }
        Action::FetchPrLog(index, request) => {
            if git_ops::current_branch(&repo)? != request.branch {
                return Err(GitError::Io {
                    detail: "A branch mudou. Consulte o PR da branch atual.".to_owned(),
                });
            }
            return Ok(Value::PrLog(
                index,
                crate::git_pr::fetch_log(&repo, &request).map_err(|error| error.to_string()),
            ));
        }
        Action::Refresh => {}
    }
    let status = gsh_git::status(&repo)?;
    let history = gsh_git::log(&repo, HISTORY_LIMIT);
    let renames = git_ops::renames(&repo)?;
    let branches = gsh_git::branches(&repo)?;
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
        branches,
    })
}

#[cfg(test)]
#[path = "git_state_tests.rs"]
mod tests;
