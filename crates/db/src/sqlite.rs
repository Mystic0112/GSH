//! Driver SQLite (`rusqlite`, SQLite embutido). Modelo efêmero: cada chamada abre, executa e fecha.

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rusqlite::types::ValueRef;
use rusqlite::{Connection, OpenFlags};

use crate::error::DbError;
use crate::result::{Cell, QueryResult};

fn open(path: &Path) -> Result<Connection, DbError> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| DbError::ConnectionFailed(e.to_string()))
}

/// Interrompe o statement quando o tempo acaba; o `Drop` encerra o vigia.
struct Watchdog {
    done: Option<mpsc::Sender<()>>,
    fired: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Watchdog {
    fn start(connection: &Connection, timeout: Duration) -> Self {
        let (done, wait) = mpsc::channel::<()>();
        let fired = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let handle = connection.get_interrupt_handle();
        let flag = std::sync::Arc::clone(&fired);
        std::thread::spawn(move || {
            if wait.recv_timeout(timeout) == Err(mpsc::RecvTimeoutError::Timeout) {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                handle.interrupt();
            }
        });
        Self {
            done: Some(done),
            fired,
        }
    }

    fn fired(&self) -> bool {
        self.fired.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl Drop for Watchdog {
    fn drop(&mut self) {
        self.done.take();
    }
}

fn failure(watchdog: &Watchdog, timeout: Duration, error: &rusqlite::Error) -> DbError {
    if watchdog.fired() {
        DbError::Timeout(timeout.as_secs())
    } else {
        DbError::QueryFailed(error.to_string())
    }
}

fn cell(value: ValueRef<'_>) -> Cell {
    match value {
        ValueRef::Null => Cell::Null,
        ValueRef::Integer(v) => Cell::Int(v),
        ValueRef::Real(v) => Cell::Real(v),
        ValueRef::Text(bytes) => Cell::Text(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => Cell::Blob(bytes.to_vec()),
    }
}

/// Executa `sql` e devolve até `limit` linhas; uma a mais marca `truncated`. Statement sem
/// colunas (DML/DDL) executa e devolve vazio, como o `rawQuery` do Flutter.
pub fn query(
    path: &Path,
    sql: &str,
    limit: usize,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let connection = open(path)?;
    let watchdog = Watchdog::start(&connection, timeout);
    let mut statement = connection
        .prepare(sql)
        .map_err(|e| failure(&watchdog, timeout, &e))?;
    let names: Vec<String> = statement.column_names().map(str::to_string).collect();
    if names.is_empty() {
        statement
            .execute([])
            .map_err(|e| failure(&watchdog, timeout, &e))?;
        return Ok(QueryResult::empty(started.elapsed()));
    }
    let mut rows = statement
        .query([])
        .map_err(|e| failure(&watchdog, timeout, &e))?;
    let mut out: Vec<Vec<Cell>> = Vec::new();
    let mut truncated = false;
    while let Some(row) = rows.next().map_err(|e| failure(&watchdog, timeout, &e))? {
        if out.len() >= limit {
            truncated = true;
            break;
        }
        let cells = (0..names.len())
            .map(|index| row.get_ref(index).map(cell))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| DbError::QueryFailed(e.to_string()))?;
        out.push(cells);
    }
    Ok(QueryResult::from_rows(
        names,
        out,
        truncated,
        started.elapsed(),
    ))
}

/// Executa DML/DDL e devolve as linhas afetadas.
pub fn execute(path: &Path, sql: &str, timeout: Duration) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let connection = open(path)?;
    let watchdog = Watchdog::start(&connection, timeout);
    let affected = connection
        .execute(sql, [])
        .map_err(|e| failure(&watchdog, timeout, &e))?;
    Ok(QueryResult::affected(
        u64::try_from(affected).unwrap_or(0),
        started.elapsed(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn database() -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("app.db");
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, score REAL, photo BLOB);
                 INSERT INTO users (name, score, photo) VALUES ('ana', 1.5, x'0102'), ('bia', NULL, NULL), ('caio', 3.0, NULL);
                 CREATE VIEW top AS SELECT name FROM users WHERE score > 1;",
            )
            .unwrap();
        (dir, path)
    }

    const T: Duration = Duration::from_secs(30);

    #[test]
    fn select_returns_typed_cells_and_inferred_types() {
        let (_dir, path) = database();
        let result = query(
            &path,
            "SELECT id, name, score, photo FROM users ORDER BY id",
            10,
            T,
        )
        .unwrap();
        let names: Vec<_> = result.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["id", "name", "score", "photo"]);
        let types: Vec<_> = result.columns.iter().map(|c| c.ty.as_str()).collect();
        assert_eq!(types, ["INTEGER", "TEXT", "REAL", ""]);
        assert_eq!(result.rows.len(), 3);
        assert_eq!(result.rows[0][1], Cell::Text("ana".into()));
        assert_eq!(result.rows[0][3], Cell::Blob(vec![1, 2]));
        assert_eq!(result.rows[1][2], Cell::Null);
        assert!(!result.truncated);
    }

    #[test]
    fn limit_truncates_and_says_so() {
        let (_dir, path) = database();
        let result = query(&path, "SELECT id FROM users ORDER BY id", 2, T).unwrap();
        assert_eq!(result.rows.len(), 2);
        assert!(result.truncated);
        let exact = query(&path, "SELECT id FROM users", 3, T).unwrap();
        assert!(!exact.truncated);
    }

    #[test]
    fn empty_result_has_no_columns() {
        let (_dir, path) = database();
        let result = query(&path, "SELECT id FROM users WHERE id = 99", 10, T).unwrap();
        assert!(result.columns.is_empty() && result.rows.is_empty());
    }

    #[test]
    fn dml_through_query_runs_and_returns_nothing() {
        let (_dir, path) = database();
        let result = query(&path, "UPDATE users SET score = 0", 10, T).unwrap();
        assert!(result.rows.is_empty() && result.affected_rows.is_none());
        let check = query(&path, "SELECT score FROM users WHERE score = 0", 10, T).unwrap();
        assert_eq!(check.rows.len(), 3);
    }

    #[test]
    fn execute_reports_affected_rows() {
        let (_dir, path) = database();
        let result = execute(&path, "DELETE FROM users WHERE id > 1", T).unwrap();
        assert_eq!(result.affected_rows, Some(2));
    }

    #[test]
    fn errors_are_typed_and_carry_the_driver_text() {
        let (_dir, path) = database();
        let syntax = query(&path, "SELEC 1", 10, T).unwrap_err();
        assert!(
            matches!(&syntax, DbError::QueryFailed(text) if text.contains("syntax")),
            "{syntax:?}"
        );
        let missing = query(&path.with_file_name("none.db"), "SELECT 1", 10, T).unwrap_err();
        assert!(
            matches!(missing, DbError::ConnectionFailed(_)),
            "{missing:?}"
        );
    }

    #[test]
    fn schema_queries_list_tables_and_columns() {
        use crate::connection::Engine;
        use crate::schema::{columns_from, schema_sql, tables_from};
        let (_dir, path) = database();
        let sql = schema_sql(Engine::Sqlite, None, None).unwrap();
        let tables = tables_from(&query(&path, &sql, 10_000, T).unwrap());
        let listed: Vec<_> = tables
            .iter()
            .map(|t| (t.name.as_str(), t.is_view))
            .collect();
        assert_eq!(listed, [("top", true), ("users", false)]);
        let sql = schema_sql(Engine::Sqlite, Some("users"), None).unwrap();
        let columns = columns_from(&query(&path, &sql, 10_000, T).unwrap());
        let listed: Vec<_> = columns
            .iter()
            .map(|c| (c.name.as_str(), c.ty.as_str(), c.primary_key))
            .collect();
        assert_eq!(
            listed,
            [
                ("id", "INTEGER", true),
                ("name", "TEXT", false),
                ("score", "REAL", false),
                ("photo", "BLOB", false)
            ]
        );
    }

    #[test]
    fn a_long_query_is_interrupted_at_the_timeout() {
        let (_dir, path) = database();
        let sql = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT count(*) FROM n";
        let error = query(&path, sql, 10, Duration::from_millis(300)).unwrap_err();
        assert_eq!(error, DbError::Timeout(0));
    }
}
