use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use rusqlite::{params_from_iter, types::Value as SqlValue, Connection, OptionalExtension};
use serde_json::{Map, Value};

use crate::fingerprint::fingerprint_of;
use crate::model::{
    now_ms, TelemetryCase, TelemetryCaseRun, TelemetryError, TelemetryEvent, TelemetryQuery,
    TelemetryRetention, TelemetryRun, TelemetryRunSource, TelemetrySeverity, TelemetryStoreError,
    TelemetryStream, TelemetryTriage, TelemetryTriageActor, TelemetryTriageStatus,
};
use crate::redaction::{redact_attrs, redact_text};

pub fn b36(mut n: u64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut buf = Vec::new();
    while n > 0 {
        buf.push(DIGITS[(n % 36) as usize]);
        n /= 36;
    }
    buf.reverse();
    String::from_utf8(buf).unwrap_or_else(|_| "0".to_string())
}

pub fn event_id(rowid: i64) -> String {
    let n = u64::try_from(rowid.max(0)).unwrap_or(0);
    format!("ev_{}", b36(n))
}

pub fn event_rowid(id: &str) -> Option<i64> {
    let rest = id.strip_prefix("ev_")?;
    i64::from_str_radix(rest, 36).ok()
}

pub fn fts_query(text: &str) -> String {
    text.split_whitespace()
        .filter(|t| !t.is_empty())
        .map(|t| format!("\"{}\"*", t.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" ")
}

pub struct TelemetryDb {
    conn: Connection,
    has_fts: bool,
}

impl TelemetryDb {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, TelemetryStoreError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA auto_vacuum=INCREMENTAL;
             PRAGMA foreign_keys=ON;",
        )?;
        let mut db = Self {
            conn,
            has_fts: false,
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn memory() -> Result<Self, TelemetryStoreError> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        let mut db = Self {
            conn,
            has_fts: false,
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn has_fts(&self) -> bool {
        self.has_fts
    }

    fn migrate(&mut self) -> Result<(), TelemetryStoreError> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS runs (
              id TEXT PRIMARY KEY,
              key TEXT NOT NULL,
              project TEXT NOT NULL,
              source TEXT NOT NULL,
              cwd TEXT NOT NULL,
              command TEXT NOT NULL,
              name TEXT,
              task_key TEXT,
              pane_id TEXT,
              pid INTEGER,
              started_at INTEGER NOT NULL,
              ended_at INTEGER,
              exit_code INTEGER
            );
            CREATE INDEX IF NOT EXISTS runs_key ON runs(key, started_at);
            CREATE INDEX IF NOT EXISTS runs_project ON runs(project, started_at);

            CREATE TABLE IF NOT EXISTS events (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
              ts INTEGER NOT NULL,
              stream TEXT NOT NULL,
              severity INTEGER NOT NULL,
              body TEXT NOT NULL,
              attrs TEXT,
              error TEXT,
              fingerprint TEXT,
              trace_id TEXT,
              span_id TEXT,
              request_id TEXT,
              raw_offset INTEGER,
              repeat INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS events_run ON events(run_id, id);
            CREATE INDEX IF NOT EXISTS events_fp ON events(fingerprint, ts);
            CREATE INDEX IF NOT EXISTS events_ts ON events(ts);
            CREATE INDEX IF NOT EXISTS events_trace ON events(trace_id);

            CREATE TABLE IF NOT EXISTS triage (
              fingerprint TEXT PRIMARY KEY,
              status TEXT NOT NULL,
              actor TEXT NOT NULL,
              at INTEGER NOT NULL,
              reason TEXT
            );

            CREATE TABLE IF NOT EXISTS marks (
              name TEXT PRIMARY KEY,
              start_at INTEGER NOT NULL,
              end_at INTEGER
            );

            CREATE TABLE IF NOT EXISTS meta (k TEXT PRIMARY KEY, v TEXT);
            ",
        )?;

        let fts_res = self.conn.execute_batch(
            "
            CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
              body, attrs, content='events', content_rowid='id'
            );
            CREATE TRIGGER IF NOT EXISTS events_ai AFTER INSERT ON events BEGIN
              INSERT INTO events_fts(rowid, body, attrs)
              VALUES (new.id, new.body, coalesce(new.attrs, ''));
            END;
            CREATE TRIGGER IF NOT EXISTS events_ad AFTER DELETE ON events BEGIN
              INSERT INTO events_fts(events_fts, rowid, body, attrs)
              VALUES ('delete', old.id, old.body, coalesce(old.attrs, ''));
            END;
            ",
        );
        self.has_fts = fts_res.is_ok();
        Ok(())
    }

    fn next_seq(&self, k: &str) -> Result<u64, TelemetryStoreError> {
        let cur: Option<String> = self
            .conn
            .query_row("SELECT v FROM meta WHERE k = ?1", [k], |row| row.get(0))
            .optional()?;
        let cur_num: u64 = cur.and_then(|s| s.parse().ok()).unwrap_or(0);
        let next = cur_num + 1;
        self.conn.execute(
            "INSERT INTO meta(k, v) VALUES (?1, ?2) ON CONFLICT(k) DO UPDATE SET v = excluded.v",
            rusqlite::params![k, next.to_string()],
        )?;
        Ok(next)
    }

    // ---- runs --------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    pub fn open_run(
        &self,
        key: &str,
        project: &str,
        source: TelemetryRunSource,
        cwd: &str,
        command: &str,
        name: Option<&str>,
        task_key: Option<&str>,
        pane_id: Option<&str>,
        pid: Option<i64>,
        started_at_ms: Option<i64>,
    ) -> Result<TelemetryRun, TelemetryStoreError> {
        let id = format!("r_{}", b36(self.next_seq("run_seq")?));
        let started = started_at_ms.unwrap_or_else(now_ms);
        self.conn.execute(
            "INSERT INTO runs(id, key, project, source, cwd, command, name, task_key, pane_id, pid, started_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            rusqlite::params![
                id,
                key,
                project,
                source.as_str(),
                cwd,
                command,
                name,
                task_key,
                pane_id,
                pid,
                started
            ],
        )?;
        self.run(&id)?
            .ok_or_else(|| TelemetryStoreError::NotFound(id))
    }

    pub fn close_run(
        &self,
        id: &str,
        exit_code: Option<i32>,
        ended_at_ms: Option<i64>,
    ) -> Result<(), TelemetryStoreError> {
        let ended = ended_at_ms.unwrap_or_else(now_ms);
        self.conn.execute(
            "UPDATE runs SET ended_at = ?1, exit_code = ?2 WHERE id = ?3 AND ended_at IS NULL",
            rusqlite::params![ended, exit_code, id],
        )?;
        Ok(())
    }

    pub fn run(&self, id: &str) -> Result<Option<TelemetryRun>, TelemetryStoreError> {
        self.conn
            .query_row(
                "SELECT id, key, project, source, cwd, command, name, task_key, pane_id, pid, started_at, ended_at, exit_code
                 FROM runs WHERE id = ?1",
                [id],
                row_to_run,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn runs(
        &self,
        project: Option<&str>,
        key: Option<&str>,
        only_live: bool,
        limit: usize,
    ) -> Result<Vec<TelemetryRun>, TelemetryStoreError> {
        let mut where_clauses = Vec::new();
        let mut args: Vec<SqlValue> = Vec::new();
        if let Some(p) = project {
            where_clauses.push("project = ?");
            args.push(SqlValue::Text(p.to_string()));
        }
        if let Some(k) = key {
            where_clauses.push("key = ?");
            args.push(SqlValue::Text(k.to_string()));
        }
        if only_live {
            where_clauses.push("ended_at IS NULL");
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT id, key, project, source, cwd, command, name, task_key, pane_id, pid, started_at, ended_at, exit_code
             FROM runs {where_sql} ORDER BY started_at DESC LIMIT ?"
        );
        args.push(SqlValue::Integer(i64::try_from(limit).unwrap_or(50)));
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(args), row_to_run)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        Ok(out)
    }

    // ---- events ------------------------------------------------------------

    pub fn append(
        &mut self,
        events: &[TelemetryEvent],
    ) -> Result<Vec<TelemetryEvent>, TelemetryStoreError> {
        if events.is_empty() {
            return Ok(Vec::new());
        }
        let tx = self.conn.transaction()?;
        let mut saved = Vec::with_capacity(events.len());
        {
            let mut stmt = tx.prepare(
                "INSERT INTO events(run_id, ts, stream, severity, body, attrs, error,
                 fingerprint, trace_id, span_id, request_id, raw_offset, repeat)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            )?;
            for ev in events {
                let redacted_attrs =
                    if ev.attrs.is_empty() {
                        None
                    } else {
                        let m = redact_attrs(&ev.attrs);
                        Some(serde_json::to_string(&m).map_err(|e| {
                            TelemetryStoreError::Corrupt(format!("attrs json: {e}"))
                        })?)
                    };
                let mut fp = ev.fingerprint.clone();
                let mut redacted_err = ev.error.clone();
                let err_json =
                    if let Some(ref mut err) = redacted_err {
                        err.message = redact_text(&err.message);
                        if let Some(ref st) = err.stack {
                            err.stack = Some(redact_text(st));
                        }
                        if fp.is_none() {
                            fp = Some(fingerprint_of(err));
                        }
                        Some(serde_json::to_string(err).map_err(|e| {
                            TelemetryStoreError::Corrupt(format!("error json: {e}"))
                        })?)
                    } else {
                        None
                    };
                let redacted_body = redact_text(&ev.body);
                stmt.execute(rusqlite::params![
                    ev.run_id,
                    ev.ts_ms,
                    ev.stream.as_str(),
                    i64::from(ev.severity.index()),
                    redacted_body,
                    redacted_attrs,
                    err_json,
                    fp,
                    ev.trace_id,
                    ev.span_id,
                    ev.request_id,
                    ev.raw_line_offset,
                    ev.repeat.max(1),
                ])?;
                let id = event_id(tx.last_insert_rowid());
                let mut cloned = ev.clone();
                cloned.id = Some(id);
                cloned.fingerprint = fp;
                cloned.body = redacted_body;
                cloned.error = redacted_err;
                saved.push(cloned);
            }
        }
        tx.commit()?;
        Ok(saved)
    }

    pub fn event(&self, id: &str) -> Result<Option<TelemetryEvent>, TelemetryStoreError> {
        let Some(rowid) = event_rowid(id) else {
            return Ok(None);
        };
        self.conn
            .query_row(
                "SELECT id, run_id, ts, stream, severity, body, attrs, error, fingerprint, trace_id, span_id, request_id, raw_offset, repeat
                 FROM events WHERE id = ?1",
                [rowid],
                row_to_event,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn events(&self, q: &TelemetryQuery) -> Result<Vec<TelemetryEvent>, TelemetryStoreError> {
        let mut where_clauses = Vec::new();
        let mut args = Vec::new();
        apply_run_filters(q, &mut where_clauses, &mut args);
        if let Some(sev) = q.min_severity {
            where_clauses.push("e.severity >= ?".to_string());
            args.push(SqlValue::Integer(i64::from(sev.index())));
        }
        if let Some(since) = q.since_ms {
            where_clauses.push("e.ts >= ?".to_string());
            args.push(SqlValue::Integer(since));
        }
        if let Some(until) = q.until_ms {
            where_clauses.push("e.ts <= ?".to_string());
            args.push(SqlValue::Integer(until));
        }
        if q.only_errors {
            where_clauses.push("e.error IS NOT NULL".to_string());
        }
        if let Some(ref probe) = q.probe {
            if probe == "*" {
                where_clauses.push("e.attrs LIKE '%\"probe\":%'".to_string());
            } else {
                where_clauses.push("e.attrs LIKE ?".to_string());
                let enc = serde_json::to_string(probe).unwrap_or_else(|_| format!("\"{probe}\""));
                args.push(SqlValue::Text(format!("%\"probe\":{enc}%")));
            }
        }
        if let Some(ref text) = q.text {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                if self.has_fts {
                    where_clauses.push(
                        "e.id IN (SELECT rowid FROM events_fts WHERE events_fts MATCH ?)"
                            .to_string(),
                    );
                    args.push(SqlValue::Text(fts_query(trimmed)));
                } else {
                    where_clauses.push("(e.body LIKE ? OR e.attrs LIKE ?)".to_string());
                    let pat = format!("%{trimmed}%");
                    args.push(SqlValue::Text(pat.clone()));
                    args.push(SqlValue::Text(pat));
                }
            }
        }
        if !q.include_ignored {
            where_clauses.push(
                "(e.fingerprint IS NULL OR e.fingerprint NOT IN (SELECT fingerprint FROM triage WHERE status = 'ignored'))".to_string(),
            );
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_clauses.join(" AND "))
        };
        let sql = format!(
            "SELECT e.id, e.run_id, e.ts, e.stream, e.severity, e.body, e.attrs, e.error, e.fingerprint, e.trace_id, e.span_id, e.request_id, e.raw_offset, e.repeat
             FROM events e JOIN runs r ON r.id = e.run_id
             {where_sql}
             ORDER BY e.id DESC LIMIT ?"
        );
        args.push(SqlValue::Integer(i64::try_from(q.limit).unwrap_or(100)));
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(args), row_to_event)?;
        let mut out = Vec::new();
        for r in rows {
            out.push(r?);
        }
        out.reverse();
        Ok(out)
    }

    pub fn context(
        &self,
        id: &str,
        before: usize,
        after: usize,
    ) -> Result<Vec<TelemetryEvent>, TelemetryStoreError> {
        let Some(rowid) = event_rowid(id) else {
            return Ok(Vec::new());
        };
        let run_id: Option<String> = self
            .conn
            .query_row("SELECT run_id FROM events WHERE id = ?1", [rowid], |r| {
                r.get(0)
            })
            .optional()?;
        let Some(run_id) = run_id else {
            return Ok(Vec::new());
        };

        let mut prev = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, run_id, ts, stream, severity, body, attrs, error, fingerprint, trace_id, span_id, request_id, raw_offset, repeat
                 FROM events WHERE run_id = ?1 AND id < ?2 ORDER BY id DESC LIMIT ?3",
            )?;
            let rows = stmt.query_map(
                rusqlite::params![run_id, rowid, i64::try_from(before).unwrap_or(10)],
                row_to_event,
            )?;
            for r in rows {
                prev.push(r?);
            }
            prev.reverse();
        }

        let mut next = Vec::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT id, run_id, ts, stream, severity, body, attrs, error, fingerprint, trace_id, span_id, request_id, raw_offset, repeat
                 FROM events WHERE run_id = ?1 AND id >= ?2 ORDER BY id ASC LIMIT ?3",
            )?;
            let rows = stmt.query_map(
                rusqlite::params![run_id, rowid, i64::try_from(after + 1).unwrap_or(6)],
                row_to_event,
            )?;
            for r in rows {
                next.push(r?);
            }
        }

        prev.extend(next);
        Ok(prev)
    }

    // ---- cases -------------------------------------------------------------

    pub fn cases(&self, q: &TelemetryQuery) -> Result<Vec<TelemetryCase>, TelemetryStoreError> {
        let mut where_clauses = vec!["e.fingerprint IS NOT NULL".to_string()];
        let mut args = Vec::new();
        apply_run_filters(q, &mut where_clauses, &mut args);
        if let Some(sev) = q.min_severity {
            where_clauses.push("e.severity >= ?".to_string());
            args.push(SqlValue::Integer(i64::from(sev.index())));
        }
        if let Some(since) = q.since_ms {
            where_clauses.push("e.ts >= ?".to_string());
            args.push(SqlValue::Integer(since));
        }
        if let Some(until) = q.until_ms {
            where_clauses.push("e.ts <= ?".to_string());
            args.push(SqlValue::Integer(until));
        }
        if let Some(ref text) = q.text {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                if self.has_fts {
                    where_clauses.push(
                        "e.id IN (SELECT rowid FROM events_fts WHERE events_fts MATCH ?)"
                            .to_string(),
                    );
                    args.push(SqlValue::Text(fts_query(trimmed)));
                } else {
                    where_clauses.push("(e.body LIKE ? OR e.error LIKE ?)".to_string());
                    let pat = format!("%{trimmed}%");
                    args.push(SqlValue::Text(pat.clone()));
                    args.push(SqlValue::Text(pat));
                }
            }
        }

        let sql = format!(
            "SELECT e.fingerprint AS fp, r.key AS run_key, r.project AS project,
                    r.id AS run_id, r.started_at AS run_started, r.ended_at AS run_ended,
                    SUM(e.repeat) AS cnt, MIN(e.ts) AS first_at, MAX(e.ts) AS last_at,
                    MAX(e.severity) AS sev, MAX(e.id) AS last_id
             FROM events e JOIN runs r ON r.id = e.run_id
             WHERE {}
             GROUP BY e.fingerprint, r.key, r.id
             ORDER BY last_at DESC",
            where_clauses.join(" AND ")
        );

        struct AggRow {
            fp: String,
            run_key: String,
            project: String,
            run_id: String,
            run_started: i64,
            run_ended: Option<i64>,
            cnt: i64,
            first_at: i64,
            last_at: i64,
            sev: i64,
            last_id: i64,
        }

        let mut agg_rows = Vec::new();
        {
            let mut stmt = self.conn.prepare(&sql)?;
            let rows = stmt.query_map(params_from_iter(args), |row| {
                Ok(AggRow {
                    fp: row.get(0)?,
                    run_key: row.get(1)?,
                    project: row.get(2)?,
                    run_id: row.get(3)?,
                    run_started: row.get(4)?,
                    run_ended: row.get(5)?,
                    cnt: row.get(6)?,
                    first_at: row.get(7)?,
                    last_at: row.get(8)?,
                    sev: row.get(9)?,
                    last_id: row.get(10)?,
                })
            })?;
            for r in rows {
                agg_rows.push(r?);
            }
        }
        if agg_rows.is_empty() {
            return Ok(Vec::new());
        }

        let mut triage_map: HashMap<String, TelemetryTriage> = HashMap::new();
        {
            let mut stmt = self
                .conn
                .prepare("SELECT fingerprint, status, actor, at, reason FROM triage")?;
            let rows = stmt.query_map([], |row| {
                let fp: String = row.get(0)?;
                let status_s: String = row.get(1)?;
                let actor_s: String = row.get(2)?;
                let at_ms: i64 = row.get(3)?;
                let reason: Option<String> = row.get(4)?;
                Ok(TelemetryTriage {
                    fingerprint: fp,
                    status: TelemetryTriageStatus::parse(&status_s)
                        .unwrap_or(TelemetryTriageStatus::Open),
                    by: TelemetryTriageActor::parse(&actor_s)
                        .unwrap_or(TelemetryTriageActor::Human),
                    at_ms,
                    reason,
                })
            })?;
            for r in rows {
                let t = r?;
                triage_map.insert(t.fingerprint.clone(), t);
            }
        }

        let mut latest_run: HashMap<String, String> = HashMap::new();
        {
            let mut stmt = self.conn.prepare(
                "SELECT key, id FROM runs r1
                 WHERE started_at = (SELECT MAX(started_at) FROM runs r2 WHERE r2.key = r1.key)",
            )?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?;
            for r in rows {
                let (k, id) = r?;
                latest_run.insert(k, id);
            }
        }

        let mut grouped: HashMap<(String, String), Vec<AggRow>> = HashMap::new();
        for r in agg_rows {
            grouped
                .entry((r.fp.clone(), r.run_key.clone()))
                .or_default()
                .push(r);
        }

        let mut out = Vec::new();
        for ((fp, key), mut list) in grouped {
            list.sort_by_key(|a| std::cmp::Reverse(a.run_started));
            let Some(first_row) = list.first() else {
                continue;
            };
            let project = first_row.project.clone();
            let last_at = list.iter().map(|r| r.last_at).max().unwrap_or(0);
            let first_at = list.iter().map(|r| r.first_at).min().unwrap_or(0);
            let count: i64 = list.iter().map(|r| r.cnt).sum();
            let sev = list.iter().map(|r| r.sev).max().unwrap_or(2);
            let last_id = list.iter().map(|r| r.last_id).max().unwrap_or(0);

            let t = triage_map.get(&fp).cloned();
            let mut status = t.as_ref().map_or(TelemetryTriageStatus::Open, |x| x.status);
            let mut regression = false;
            if status == TelemetryTriageStatus::Resolved {
                if let Some(ref tr) = t {
                    if last_at > tr.at_ms {
                        status = TelemetryTriageStatus::Open;
                        regression = true;
                    }
                }
            }
            if status == TelemetryTriageStatus::Ignored && !q.include_ignored {
                continue;
            }
            if status == TelemetryTriageStatus::Resolved && !q.include_resolved {
                continue;
            }

            let run_ids: BTreeSet<&str> = list.iter().map(|r| r.run_id.as_str()).collect();
            let is_new = run_ids.len() == 1
                && latest_run
                    .get(&key)
                    .is_some_and(|latest| run_ids.contains(latest.as_str()));
            if q.only_new && !(is_new || regression) {
                continue;
            }

            let err_raw: String = self.conn.query_row(
                "SELECT error FROM events WHERE id = ?1",
                [last_id],
                |row| row.get(0),
            )?;
            let err: TelemetryError = serde_json::from_str(&err_raw)
                .map_err(|e| TelemetryStoreError::Corrupt(format!("case error json: {e}")))?;
            let location = err.project_frame().and_then(|f| f.location());
            let runs = list
                .into_iter()
                .map(|r| TelemetryCaseRun {
                    run_id: r.run_id,
                    count: r.cnt,
                    first_at_ms: r.first_at,
                    last_at_ms: r.last_at,
                    live: r.run_ended.is_none(),
                })
                .collect();

            out.push(TelemetryCase {
                fingerprint: fp,
                run_key: key,
                project,
                severity: TelemetrySeverity::from_index(sev),
                err_type: err.err_type,
                message: err.message,
                location,
                count,
                first_at_ms: first_at,
                last_at_ms: last_at,
                status,
                triage: t,
                is_new,
                is_regression: regression,
                runs,
                last_event_id: Some(event_id(last_id)),
            });
        }

        out.sort_by_key(|a| std::cmp::Reverse(a.last_at_ms));
        if out.len() > q.limit {
            out.truncate(q.limit);
        }
        Ok(out)
    }

    pub fn case_by_id(&self, id_or_fp: &str) -> Result<Option<TelemetryCase>, TelemetryStoreError> {
        let prefix = id_or_fp
            .strip_prefix("e_")
            .unwrap_or(id_or_fp)
            .to_lowercase();
        if prefix.is_empty() {
            return Ok(None);
        }
        let mut stmt = self
            .conn
            .prepare("SELECT DISTINCT fingerprint FROM events WHERE fingerprint LIKE ?1")?;
        let mut fps = Vec::new();
        let rows = stmt.query_map([format!("{prefix}%")], |row| row.get::<_, String>(0))?;
        for r in rows {
            fps.push(r?);
        }
        if fps.len() != 1 {
            return Ok(None);
        }
        let fp = &fps[0];
        let all = self.cases(&TelemetryQuery {
            include_ignored: true,
            include_resolved: true,
            limit: 1000,
            ..Default::default()
        })?;
        Ok(all.into_iter().find(|c| &c.fingerprint == fp))
    }

    pub fn triage(
        &self,
        fingerprint: &str,
        status: TelemetryTriageStatus,
        actor: TelemetryTriageActor,
        reason: Option<&str>,
        at_ms: Option<i64>,
    ) -> Result<(), TelemetryStoreError> {
        if status == TelemetryTriageStatus::Open {
            self.conn
                .execute("DELETE FROM triage WHERE fingerprint = ?1", [fingerprint])?;
            return Ok(());
        }
        let at = at_ms.unwrap_or_else(now_ms);
        self.conn.execute(
            "INSERT INTO triage(fingerprint, status, actor, at, reason) VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(fingerprint) DO UPDATE SET status = excluded.status,
             actor = excluded.actor, at = excluded.at, reason = excluded.reason",
            rusqlite::params![fingerprint, status.as_str(), actor.as_str(), at, reason],
        )?;
        Ok(())
    }

    // ---- marks -------------------------------------------------------------

    pub fn mark_start(&self, name: &str, at_ms: Option<i64>) -> Result<(), TelemetryStoreError> {
        let at = at_ms.unwrap_or_else(now_ms);
        self.conn.execute(
            "INSERT INTO marks(name, start_at, end_at) VALUES (?1,?2,NULL)
             ON CONFLICT(name) DO UPDATE SET start_at = excluded.start_at, end_at = NULL",
            rusqlite::params![name, at],
        )?;
        Ok(())
    }

    pub fn mark_end(&self, name: &str, at_ms: Option<i64>) -> Result<(), TelemetryStoreError> {
        let at = at_ms.unwrap_or_else(now_ms);
        self.conn.execute(
            "UPDATE marks SET end_at = ?1 WHERE name = ?2",
            rusqlite::params![at, name],
        )?;
        Ok(())
    }

    pub fn mark(&self, name: &str) -> Result<Option<(i64, Option<i64>)>, TelemetryStoreError> {
        self.conn
            .query_row(
                "SELECT start_at, end_at FROM marks WHERE name = ?1",
                [name],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .optional()
            .map_err(Into::into)
    }

    // ---- maintenance -------------------------------------------------------

    pub fn clear(
        &self,
        fingerprint: Option<&str>,
        run_id: Option<&str>,
        project: Option<&str>,
    ) -> Result<(), TelemetryStoreError> {
        if let Some(fp) = fingerprint {
            self.conn
                .execute("DELETE FROM events WHERE fingerprint = ?1", [fp])?;
        }
        if let Some(id) = run_id {
            self.conn.execute("DELETE FROM runs WHERE id = ?1", [id])?;
        }
        if let Some(proj) = project {
            self.conn
                .execute("DELETE FROM runs WHERE project = ?1", [proj])?;
        }
        if fingerprint.is_none() && run_id.is_none() && project.is_none() {
            self.conn.execute("DELETE FROM runs", [])?;
        }
        Ok(())
    }

    pub fn size_in_bytes(&self) -> Result<u64, TelemetryStoreError> {
        let s: Option<i64> = self.conn.query_row(
            "SELECT page_count * page_size AS s FROM pragma_page_count(), pragma_page_size()",
            [],
            |row| row.get(0),
        )?;
        Ok(u64::try_from(s.unwrap_or(0).max(0)).unwrap_or(0))
    }

    pub fn vacuum(
        &self,
        retention: TelemetryRetention,
        now: i64,
    ) -> Result<u64, TelemetryStoreError> {
        let before = self.size_in_bytes()?;
        self.conn.execute(
            "DELETE FROM runs WHERE ended_at IS NOT NULL AND started_at < ?1",
            [now.saturating_sub(retention.max_age_ms)],
        )?;
        let mut guard = 0usize;
        while self.size_in_bytes()? > retention.max_bytes && guard < 10_000 {
            guard += 1;
            let oldest: Option<String> = self
                .conn
                .query_row(
                    "SELECT id FROM runs WHERE ended_at IS NOT NULL ORDER BY started_at ASC LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .optional()?;
            let Some(id) = oldest else {
                break;
            };
            self.conn.execute("DELETE FROM runs WHERE id = ?1", [id])?;
            self.conn.execute_batch("PRAGMA incremental_vacuum;")?;
        }
        self.conn.execute_batch("PRAGMA incremental_vacuum;")?;
        let after = self.size_in_bytes()?;
        Ok(before.saturating_sub(after))
    }
}

fn apply_run_filters(
    q: &TelemetryQuery,
    where_clauses: &mut Vec<String>,
    args: &mut Vec<SqlValue>,
) {
    let mut eq = |col: &str, val: Option<&String>| {
        if let Some(v) = val {
            where_clauses.push(format!("{col} = ?"));
            args.push(SqlValue::Text(v.clone()));
        }
    };
    eq("r.project", q.project.as_ref());
    eq("r.id", q.run_id.as_ref());
    eq("r.key", q.run_key.as_ref());
    eq("r.task_key", q.task_key.as_ref());
    eq("r.pane_id", q.pane_id.as_ref());
}

fn row_to_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<TelemetryRun> {
    let source_s: String = row.get(3)?;
    let source = TelemetryRunSource::parse(&source_s).unwrap_or(TelemetryRunSource::Wrapper);
    Ok(TelemetryRun {
        id: row.get(0)?,
        key: row.get(1)?,
        project: row.get(2)?,
        source,
        cwd: row.get(4)?,
        command: row.get(5)?,
        name: row.get(6)?,
        task_key: row.get(7)?,
        pane_id: row.get(8)?,
        pid: row.get(9)?,
        started_at_ms: row.get(10)?,
        ended_at_ms: row.get(11)?,
        exit_code: row.get(12)?,
    })
}

fn row_to_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<TelemetryEvent> {
    let rowid: i64 = row.get(0)?;
    let stream_s: String = row.get(3)?;
    let sev_i: i64 = row.get(4)?;
    let attrs_raw: Option<String> = row.get(6)?;
    let err_raw: Option<String> = row.get(7)?;
    let attrs: Map<String, Value> = attrs_raw
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let error: Option<TelemetryError> = err_raw
        .filter(|s| !s.is_empty())
        .and_then(|s| serde_json::from_str(&s).ok());
    Ok(TelemetryEvent {
        id: Some(event_id(rowid)),
        run_id: row.get(1)?,
        ts_ms: row.get(2)?,
        stream: TelemetryStream::parse(&stream_s).unwrap_or(TelemetryStream::Out),
        severity: TelemetrySeverity::from_index(sev_i),
        body: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
        attrs,
        error,
        fingerprint: row.get(8)?,
        trace_id: row.get(9)?,
        span_id: row.get(10)?,
        request_id: row.get(11)?,
        raw_line_offset: row.get(12)?,
        repeat: row.get::<_, Option<i64>>(13)?.unwrap_or(1),
    })
}
