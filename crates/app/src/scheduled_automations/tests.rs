use super::*;

#[test]
fn add_persist_and_take_due_runs_once_per_local_day() {
    let dir = tempfile::tempdir().unwrap();
    let store_path = dir.path().join("scheduled_routines.json");
    let repo_dir = dir.path().join("repo");
    std::fs::create_dir(&repo_dir).unwrap();

    let mut store = ScheduledStore::load_from(Some(store_path.clone()));
    let added = store
        .add(
            "Revisão matinal",
            "9:00",
            every_day(),
            &repo_dir.to_string_lossy(),
            "claude",
            "Revisar PRs abertos e rodar testes",
        )
        .unwrap();
    assert_eq!(added.time_hhmm, "09:00");

    // 08:30 local -> not due yet
    let (today, _, weekday) = local_schedule_time(1_767_268_800_000);
    let mut reloaded = ScheduledStore::load_from(Some(store_path.clone()));
    assert_eq!(reloaded.routines().len(), 1);
    reloaded.routines[0].time_hhmm = String::from("23:59");
    assert!(reloaded.take_due(1_767_236_700_000).is_empty());

    reloaded.routines[0].time_hhmm = String::from("00:00");
    reloaded.routines[0].weekdays = [false; 7];
    assert!(reloaded.take_due(1_767_268_800_000).is_empty());
    assert!(reloaded
        .manual_run(&added.id)
        .unwrap()
        .last_run_date
        .is_none());
    reloaded.routines[0].weekdays[weekday] = true;
    let due = reloaded.take_due(1_767_268_800_000);
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].last_run_date.as_deref(), Some(today.as_str()));

    // Calling take_due again on the same day does not re-trigger
    assert!(reloaded.take_due(1_767_268_800_000).is_empty());
}

#[test]
fn legacy_routines_remain_daily() {
    let routine: ScheduledRoutine = serde_json::from_str(
        r#"{"id":"routine-1","name":"Teste","timeHhmm":"09:00","workspace":"/tmp","harness":"pi","prompt":"revisar"}"#,
    ).unwrap();
    assert_eq!(routine.weekdays, every_day());
    assert!(routine.enabled);
}

#[cfg(target_os = "linux")]
#[test]
fn records_an_interrupted_run_and_preserves_its_partial_log() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scheduled_routines.json");
    let mut store = ScheduledStore::load_from(Some(path.clone()));
    let routine = store
        .add(
            "Teste",
            "09:00",
            every_day(),
            dir.path().to_str().unwrap(),
            "codex",
            "Revisar",
        )
        .unwrap();
    let run = store.begin_run(&routine, false, 1_767_268_800_000).unwrap();
    run.terminal = Some((1, 42));
    std::fs::write(run.log(), "resultado parcial").unwrap();
    assert!(store.manual_run(&routine.id).is_none());
    assert_eq!(
        store.poll_runs(|_, _| false).next().unwrap().status,
        RunStatus::Interrupted
    );
    let history = ScheduledStore::load_from(Some(path));
    let run = &history.runs()[0];
    assert_eq!(run.status, RunStatus::Interrupted);
    assert_eq!(
        std::fs::read_to_string(run.log()).unwrap(),
        "resultado parcial"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn terminal_captures_literal_prompt_output_and_status_for_history() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("scheduled_routines.json");
    let program = dir.path().join("agy");
    std::fs::write(
        &program,
        "#!/bin/bash\nprintf '%s\\n' \"${@: -1}\"\necho 'erro de teste' >&2\nexit 23\n",
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let mut store = ScheduledStore::load_from(Some(path.clone()));
    let prompt = "-teste 'aspas' \"texto\" $(/usr/bin/touch escaped)\noutra linha";
    let routine = store
        .add(
            "Teste",
            "09:00",
            every_day(),
            dir.path().to_str().unwrap(),
            "agy",
            prompt,
        )
        .unwrap();
    let run = store.begin_run(&routine, true, 1_767_268_800_000).unwrap();
    let command = run
        .command(&routine)
        .unwrap()
        .env("PATH", dir.path().as_os_str());
    let terminal = gsh_term::TermSession::spawn(command, 80, 24).unwrap();
    run.terminal = terminal.child_pid().map(|pid| (1, pid));

    let deadline = Instant::now() + Duration::from_secs(3);
    while store.poll_runs(|_, _| true).next().is_none() {
        assert!(
            Instant::now() < deadline,
            "scheduled terminal did not finish"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let history = ScheduledStore::load_from(Some(path));
    let run = &history.runs()[0];
    assert_eq!(run.status, RunStatus::Failed(23));
    assert_eq!(
        std::fs::read_to_string(run.log()).unwrap(),
        format!("{prompt}\nerro de teste\n")
    );
    assert!(!dir.path().join("escaped").exists());
}
