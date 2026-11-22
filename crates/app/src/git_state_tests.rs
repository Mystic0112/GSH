use super::*;
fn err() -> GitError {
    GitError::Io {
        detail: "test".into(),
    }
}
fn result(
    repo: PathBuf,
    generation: u64,
    action: Action,
    result: Result<Value, GitError>,
) -> ResultMsg {
    ResultMsg {
        repo,
        generation,
        action,
        result,
    }
}
fn status() -> GitRepoStatus {
    let mut s = GitRepoStatus::default();
    s.staged.insert("a".into(), gsh_git::GitFileStatus::Staged);
    s
}

#[test]
fn saved_notes_reopen_only_for_the_exact_diff_and_repository() {
    let dir = tempfile::tempdir().unwrap();
    let repo = Path::new("/workspace/repo");
    let mut diff = FileDiff::unchanged("a", false);
    diff.kind = gsh_git::FileDiffKind::Modified;
    diff.hunks.push(gsh_git::DiffHunk {
        header: "@@ -1 +1 @@".into(),
        lines: vec![gsh_git::DiffLine {
            kind: gsh_git::DiffLineKind::Added,
            old_line: None,
            new_line: Some(1),
            text: "original change".into(),
        }],
    });
    let fresh_state = || GitState {
        notes_directory: Some(dir.path().to_path_buf()),
        ..GitState::default()
    };
    let mut state = fresh_state();
    state.show_diff(repo, diff.clone()).unwrap();
    let mut stale = fresh_state();
    stale.show_diff(repo, diff.clone()).unwrap();
    state.start_note(0, 0);
    state.update_note_draft("comentário pendente".into());
    assert!(state.save_note());
    drop(state);
    stale.start_note(0, 0);
    stale.update_note_draft("janela desatualizada".into());
    assert!(!stale.save_note());
    assert!(stale.active_note.is_some());

    let mut state = fresh_state();
    let mut different = diff.clone();
    different
        .hunks
        .first_mut()
        .unwrap()
        .lines
        .first_mut()
        .unwrap()
        .text = "changed on the same line".into();
    state.show_diff(repo, different).unwrap();
    assert!(state.diff_notes.is_empty());
    state
        .show_diff(Path::new("/workspace/other"), diff.clone())
        .unwrap();
    assert!(state.diff_notes.is_empty());
    state.show_diff(repo, diff.clone()).unwrap();
    assert_eq!(
        state.diff_notes.get(&(0, 0)).map(String::as_str),
        Some("comentário pendente")
    );

    state.remove_note(0, 0);
    let mut state = fresh_state();
    state.show_diff(repo, diff).unwrap();
    assert!(state.diff_notes.is_empty());
}
#[test]
fn busy_without_repo_and_stale_generation() {
    let mut state = GitState::default();
    assert!(state.begin(&Action::Refresh).is_none());
    state.set_repo(Some("repo".into()));
    let (repo, old) = state.begin(&Action::Refresh).unwrap();
    assert!(state.begin(&Action::Stage("a".into())).is_none());
    state.busy = false;
    let (_, current) = state.begin(&Action::Refresh).unwrap();
    assert_ne!(old, current);
    state.finish(result(repo.clone(), old, Action::Refresh, Err(err())));
    assert!(state.busy && state.error.is_none());
    state.finish(result(repo, current, Action::Refresh, Err(err())));
    assert!(!state.busy && state.error.is_some());
}
#[test]
fn errors_preserve_status_and_clear_failed_diff() {
    let mut state = GitState {
        status: Some(status()),
        diff: Some(FileDiff::unchanged("old", false)),
        ..Default::default()
    };
    state.repo = Some("repo".into());
    let (repo, generation) = state.begin(&Action::Diff("new".into(), false)).unwrap();
    state.finish(result(
        repo,
        generation,
        Action::Diff("new".into(), false),
        Err(err()),
    ));
    assert_eq!(state.status, Some(status()));
    assert!(state.diff.is_none());
}
#[test]
fn mutations_preserve_status_and_only_commit_clears_message() {
    let mut state = GitState {
        repo: Some("repo".into()),
        status: Some(status()),
        message: "message".into(),
        ..Default::default()
    };
    for action in [Action::Stage("a".into()), Action::Commit("message".into())] {
        let (repo, generation) = state.begin(&action).unwrap();
        assert!(state.finish(result(repo, generation, action.clone(), Ok(Value::Mutated))));
        assert_eq!(state.status, Some(status()));
        if matches!(action, Action::Stage(_)) {
            assert_eq!(state.message, "message");
        } else {
            assert!(state.message.is_empty());
        }
    }
}

#[test]
fn commit_message_suggestion_fills_draft_without_committing() {
    let mut state = GitState {
        repo: Some("repo".into()),
        status: Some(status()),
        ..Default::default()
    };
    let action = Action::SuggestCommit(Automation::default());
    let (repo, generation) = state.begin(&action).unwrap();
    assert!(state.suggesting && state.busy);
    assert!(!state.finish(result(
        repo,
        generation,
        action,
        Ok(Value::CommitMessage("feat: add branch picker".into())),
    )));
    assert_eq!(state.message, "feat: add branch picker");
    assert!(!state.suggesting && !state.busy);
}
#[test]
fn snapshot_degrades_history_and_preserves_unchanged_diff() {
    let mut state = GitState {
        repo: Some("repo".into()),
        status: Some(status()),
        diff: Some(FileDiff::unchanged("a", true)),
        ..Default::default()
    };
    let (repo, generation) = state.begin(&Action::Refresh).unwrap();
    let value = Value::Snapshot {
        root: "root".into(),
        status: Box::new(status()),
        history: Err(err()),
        renames: BTreeMap::new(),
        selected_diff: Some(Ok(FileDiff::unchanged("a", true))),
        branches: vec!["main".into()],
    };
    state.finish(result(repo, generation, Action::Refresh, Ok(value)));
    assert!(state.error.is_some() && state.history.is_empty() && state.diff.is_some());
    assert_eq!(state.root, Some("root".into()));
}
#[test]
fn reset_invalidates_inflight_and_clears_workspace_state() {
    let mut state = GitState {
        repo: Some("old".into()),
        status: Some(status()),
        message: "draft".into(),
        error: Some(err()),
        ..Default::default()
    };
    let (old_repo, old_generation) = state.begin(&Action::Refresh).unwrap();
    state.set_repo(Some("new".into()));
    assert!(
        state.status.is_none()
            && state.message.is_empty()
            && state.error.is_none()
            && !state.busy
            && state.root.is_none()
    );
    state.finish(result(
        old_repo,
        old_generation,
        Action::Refresh,
        Err(err()),
    ));
    assert!(state.error.is_none());
}
#[test]
fn failed_refresh_retries_after_interval() {
    let mut state = GitState {
        repo: Some("repo".into()),
        error: Some(err()),
        ..Default::default()
    };
    for _ in 1..REFRESH_TICKS {
        assert!(!state.should_refresh());
    }
    assert!(state.should_refresh());
    state.busy = true;
    assert!(!state.should_refresh());
}
#[test]
fn changed_file_status_invalidates_selected_diff() {
    let mut state = GitState {
        repo: Some("repo".into()),
        status: Some(status()),
        diff: Some(FileDiff::unchanged("a", true)),
        ..Default::default()
    };
    let (repo, generation) = state.begin(&Action::Refresh).unwrap();
    let value = Value::Snapshot {
        root: "root".into(),
        status: Box::new(GitRepoStatus::default()),
        history: Ok(vec![]),
        renames: BTreeMap::new(),
        selected_diff: None,
        branches: vec!["main".into()],
    };
    state.finish(result(repo, generation, Action::Refresh, Ok(value)));
    assert!(state.diff.is_none());
}
