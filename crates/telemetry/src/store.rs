use std::path::Path;
use std::sync::mpsc;

use tokio::sync::oneshot;

use crate::db::TelemetryDb;
use crate::model::{
    now_ms, TelemetryCase, TelemetryEvent, TelemetryQuery, TelemetryRetention, TelemetryRun,
    TelemetryRunSource, TelemetryStoreError, TelemetryTriageActor, TelemetryTriageStatus,
};

type BoxResult<T> = Result<T, TelemetryStoreError>;
type MarkRange = (i64, Option<i64>);

enum Op {
    OpenRun {
        key: String,
        project: String,
        source: TelemetryRunSource,
        cwd: String,
        command: String,
        name: Option<String>,
        task_key: Option<String>,
        pane_id: Option<String>,
        pid: Option<i64>,
        started_at_ms: Option<i64>,
        tx: oneshot::Sender<BoxResult<TelemetryRun>>,
    },
    CloseRun {
        run_id: String,
        exit_code: Option<i32>,
        ended_at_ms: Option<i64>,
        tx: oneshot::Sender<BoxResult<()>>,
    },
    Run {
        run_id: String,
        tx: oneshot::Sender<BoxResult<Option<TelemetryRun>>>,
    },
    Runs {
        project: Option<String>,
        key: Option<String>,
        only_live: bool,
        limit: usize,
        tx: oneshot::Sender<BoxResult<Vec<TelemetryRun>>>,
    },
    Append {
        events: Vec<TelemetryEvent>,
        tx: oneshot::Sender<BoxResult<Vec<TelemetryEvent>>>,
    },
    Event {
        event_id: String,
        tx: oneshot::Sender<BoxResult<Option<TelemetryEvent>>>,
    },
    Events {
        query: TelemetryQuery,
        tx: oneshot::Sender<BoxResult<Vec<TelemetryEvent>>>,
    },
    Context {
        event_id: String,
        before: usize,
        after: usize,
        tx: oneshot::Sender<BoxResult<Vec<TelemetryEvent>>>,
    },
    Cases {
        query: TelemetryQuery,
        tx: oneshot::Sender<BoxResult<Vec<TelemetryCase>>>,
    },
    CaseById {
        id_or_fp: String,
        tx: oneshot::Sender<BoxResult<Option<TelemetryCase>>>,
    },
    Triage {
        fingerprint: String,
        status: TelemetryTriageStatus,
        by: TelemetryTriageActor,
        reason: Option<String>,
        tx: oneshot::Sender<BoxResult<()>>,
    },
    MarkStart {
        name: String,
        at_ms: Option<i64>,
        tx: oneshot::Sender<BoxResult<()>>,
    },
    MarkEnd {
        name: String,
        at_ms: Option<i64>,
        tx: oneshot::Sender<BoxResult<()>>,
    },
    Mark {
        name: String,
        tx: oneshot::Sender<BoxResult<Option<MarkRange>>>,
    },
    Clear {
        fingerprint: Option<String>,
        run_id: Option<String>,
        project: Option<String>,
        tx: oneshot::Sender<BoxResult<()>>,
    },
    Vacuum {
        retention: TelemetryRetention,
        tx: oneshot::Sender<BoxResult<u64>>,
    },
    SizeInBytes {
        tx: oneshot::Sender<BoxResult<u64>>,
    },
    Close {
        tx: oneshot::Sender<BoxResult<()>>,
    },
}

/// Handle para um store SQLite dedicado numa thread própria (espelho do
/// `SqliteTelemetryStore` em isolate do Flutter). Clonável e thread-safe.
#[derive(Clone)]
pub struct TelemetryStore {
    tx: mpsc::Sender<Op>,
}

impl std::fmt::Debug for TelemetryStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TelemetryStore").finish_non_exhaustive()
    }
}

impl TelemetryStore {
    pub fn open_memory() -> Result<Self, TelemetryStoreError> {
        Self::spawn_worker(None)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, TelemetryStoreError> {
        let path_buf = path.as_ref().to_path_buf();
        Self::spawn_worker(Some(path_buf))
    }

    fn spawn_worker(path: Option<std::path::PathBuf>) -> Result<Self, TelemetryStoreError> {
        let (init_tx, init_rx) = mpsc::sync_channel::<BoxResult<()>>(1);
        let (tx, rx) = mpsc::channel::<Op>();
        std::thread::Builder::new()
            .name("gsh-telemetry-db".into())
            .spawn(move || {
                let mut db = match path {
                    None => match TelemetryDb::memory() {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = init_tx.send(Err(e));
                            return;
                        }
                    },
                    Some(p) => match TelemetryDb::open(&p) {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = init_tx.send(Err(e));
                            return;
                        }
                    },
                };
                let _ = init_tx.send(Ok(()));
                while let Ok(op) = rx.recv() {
                    if dispatch(&mut db, op) {
                        break;
                    }
                }
            })?;
        init_rx.recv().map_err(|_| TelemetryStoreError::Closed)??;
        Ok(Self { tx })
    }

    async fn call<T>(
        &self,
        build: impl FnOnce(oneshot::Sender<BoxResult<T>>) -> Op,
    ) -> BoxResult<T> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(build(tx))
            .map_err(|_| TelemetryStoreError::Closed)?;
        rx.await.map_err(|_| TelemetryStoreError::Closed)?
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn open_run(
        &self,
        key: impl Into<String>,
        project: impl Into<String>,
        source: TelemetryRunSource,
        cwd: impl Into<String>,
        command: impl Into<String>,
        name: Option<String>,
        task_key: Option<String>,
        pane_id: Option<String>,
        pid: Option<i64>,
        started_at_ms: Option<i64>,
    ) -> BoxResult<TelemetryRun> {
        let key = key.into();
        let project = project.into();
        let cwd = cwd.into();
        let command = command.into();
        self.call(|tx| Op::OpenRun {
            key,
            project,
            source,
            cwd,
            command,
            name,
            task_key,
            pane_id,
            pid,
            started_at_ms,
            tx,
        })
        .await
    }

    pub async fn close_run(
        &self,
        run_id: impl Into<String>,
        exit_code: Option<i32>,
        ended_at_ms: Option<i64>,
    ) -> BoxResult<()> {
        let run_id = run_id.into();
        self.call(|tx| Op::CloseRun {
            run_id,
            exit_code,
            ended_at_ms,
            tx,
        })
        .await
    }

    pub async fn run(&self, run_id: impl Into<String>) -> BoxResult<Option<TelemetryRun>> {
        let run_id = run_id.into();
        self.call(|tx| Op::Run { run_id, tx }).await
    }

    pub async fn runs(
        &self,
        project: Option<String>,
        key: Option<String>,
        only_live: bool,
        limit: usize,
    ) -> BoxResult<Vec<TelemetryRun>> {
        self.call(|tx| Op::Runs {
            project,
            key,
            only_live,
            limit,
            tx,
        })
        .await
    }

    pub async fn append(&self, events: Vec<TelemetryEvent>) -> BoxResult<Vec<TelemetryEvent>> {
        if events.is_empty() {
            return Ok(Vec::new());
        }
        self.call(|tx| Op::Append { events, tx }).await
    }

    pub async fn event(&self, event_id: impl Into<String>) -> BoxResult<Option<TelemetryEvent>> {
        let event_id = event_id.into();
        self.call(|tx| Op::Event { event_id, tx }).await
    }

    pub async fn events(&self, query: TelemetryQuery) -> BoxResult<Vec<TelemetryEvent>> {
        self.call(|tx| Op::Events { query, tx }).await
    }

    pub async fn context(
        &self,
        event_id: impl Into<String>,
        before: usize,
        after: usize,
    ) -> BoxResult<Vec<TelemetryEvent>> {
        let event_id = event_id.into();
        self.call(|tx| Op::Context {
            event_id,
            before,
            after,
            tx,
        })
        .await
    }

    pub async fn cases(&self, query: TelemetryQuery) -> BoxResult<Vec<TelemetryCase>> {
        self.call(|tx| Op::Cases { query, tx }).await
    }

    pub async fn case_by_id(
        &self,
        id_or_fp: impl Into<String>,
    ) -> BoxResult<Option<TelemetryCase>> {
        let id_or_fp = id_or_fp.into();
        self.call(|tx| Op::CaseById { id_or_fp, tx }).await
    }

    pub async fn triage(
        &self,
        fingerprint: impl Into<String>,
        status: TelemetryTriageStatus,
        by: TelemetryTriageActor,
        reason: Option<String>,
    ) -> BoxResult<()> {
        let fingerprint = fingerprint.into();
        self.call(|tx| Op::Triage {
            fingerprint,
            status,
            by,
            reason,
            tx,
        })
        .await
    }

    pub async fn mark_start(&self, name: impl Into<String>, at_ms: Option<i64>) -> BoxResult<()> {
        let name = name.into();
        self.call(|tx| Op::MarkStart { name, at_ms, tx }).await
    }

    pub async fn mark_end(&self, name: impl Into<String>, at_ms: Option<i64>) -> BoxResult<()> {
        let name = name.into();
        self.call(|tx| Op::MarkEnd { name, at_ms, tx }).await
    }

    pub async fn mark(&self, name: impl Into<String>) -> BoxResult<Option<(i64, Option<i64>)>> {
        let name = name.into();
        self.call(|tx| Op::Mark { name, tx }).await
    }

    pub async fn clear(
        &self,
        fingerprint: Option<String>,
        run_id: Option<String>,
        project: Option<String>,
    ) -> BoxResult<()> {
        self.call(|tx| Op::Clear {
            fingerprint,
            run_id,
            project,
            tx,
        })
        .await
    }

    pub async fn vacuum(&self, retention: TelemetryRetention) -> BoxResult<u64> {
        self.call(|tx| Op::Vacuum { retention, tx }).await
    }

    pub async fn size_in_bytes(&self) -> BoxResult<u64> {
        self.call(|tx| Op::SizeInBytes { tx }).await
    }

    pub async fn close(&self) -> BoxResult<()> {
        self.call(|tx| Op::Close { tx }).await
    }
}

fn dispatch(db: &mut TelemetryDb, op: Op) -> bool {
    match op {
        Op::OpenRun {
            key,
            project,
            source,
            cwd,
            command,
            name,
            task_key,
            pane_id,
            pid,
            started_at_ms,
            tx,
        } => {
            let _ = tx.send(db.open_run(
                &key,
                &project,
                source,
                &cwd,
                &command,
                name.as_deref(),
                task_key.as_deref(),
                pane_id.as_deref(),
                pid,
                started_at_ms,
            ));
        }
        Op::CloseRun {
            run_id,
            exit_code,
            ended_at_ms,
            tx,
        } => {
            let _ = tx.send(db.close_run(&run_id, exit_code, ended_at_ms));
        }
        Op::Run { run_id, tx } => {
            let _ = tx.send(db.run(&run_id));
        }
        Op::Runs {
            project,
            key,
            only_live,
            limit,
            tx,
        } => {
            let _ = tx.send(db.runs(project.as_deref(), key.as_deref(), only_live, limit));
        }
        Op::Append { events, tx } => {
            let _ = tx.send(db.append(&events));
        }
        Op::Event { event_id, tx } => {
            let _ = tx.send(db.event(&event_id));
        }
        Op::Events { query, tx } => {
            let _ = tx.send(db.events(&query));
        }
        Op::Context {
            event_id,
            before,
            after,
            tx,
        } => {
            let _ = tx.send(db.context(&event_id, before, after));
        }
        Op::Cases { query, tx } => {
            let _ = tx.send(db.cases(&query));
        }
        Op::CaseById { id_or_fp, tx } => {
            let _ = tx.send(db.case_by_id(&id_or_fp));
        }
        Op::Triage {
            fingerprint,
            status,
            by,
            reason,
            tx,
        } => {
            let _ = tx.send(db.triage(&fingerprint, status, by, reason.as_deref(), Some(now_ms())));
        }
        Op::MarkStart { name, at_ms, tx } => {
            let _ = tx.send(db.mark_start(&name, at_ms));
        }
        Op::MarkEnd { name, at_ms, tx } => {
            let _ = tx.send(db.mark_end(&name, at_ms));
        }
        Op::Mark { name, tx } => {
            let _ = tx.send(db.mark(&name));
        }
        Op::Clear {
            fingerprint,
            run_id,
            project,
            tx,
        } => {
            let _ = tx.send(db.clear(
                fingerprint.as_deref(),
                run_id.as_deref(),
                project.as_deref(),
            ));
        }
        Op::Vacuum { retention, tx } => {
            let _ = tx.send(db.vacuum(retention, now_ms()));
        }
        Op::SizeInBytes { tx } => {
            let _ = tx.send(db.size_in_bytes());
        }
        Op::Close { tx } => {
            let _ = tx.send(Ok(()));
            return true;
        }
    }
    false
}
