//! Execução e introspecção sobre um [`Endpoint`] já resolvido (motor, caminho, senha).
//!
//! Cada função é bloqueante: o chamador as roda fora da thread de UI.

use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::connection::{Connection, Engine};
use crate::error::DbError;
use crate::postgres;
use crate::result::QueryResult;
use crate::schema::{self, SchemaColumn, SchemaTable};
use crate::sqlite;

/// Linhas devolvidas quando nem a chamada nem o `.dbq` dizem o limite.
pub const DEFAULT_LIMIT: usize = 200;

/// Teto de linhas das consultas de schema.
const SCHEMA_LIMIT: usize = 10_000;

/// Tempo máximo de uma consulta (conectar incluso).
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// Para onde a conexão aponta, com caminho e senha resolvidos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    engine: Engine,
    sqlite_path: PathBuf,
    postgres: postgres::Target,
}

impl Endpoint {
    /// Resolve `conn` contra a raiz do workspace (SQLite relativo é relativo à raiz) e `password`.
    pub fn resolve(
        conn: &Connection,
        root: &Path,
        password: Option<String>,
    ) -> Result<Self, DbError> {
        if !conn.engine.is_supported() {
            return Err(DbError::UnsupportedEngine(conn.engine));
        }
        let raw = conn.sqlite_path();
        let sqlite_path = if is_absolute(raw) {
            PathBuf::from(raw)
        } else {
            root.join(raw)
        };
        Ok(Self {
            engine: conn.engine,
            sqlite_path,
            postgres: postgres::Target {
                host: conn.host(),
                port: conn.port(),
                user: conn.user(),
                password,
                database: conn.database(),
                sslmode: conn.query_param("sslmode"),
            },
        })
    }

    pub fn engine(&self) -> Engine {
        self.engine
    }
}

fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    path.starts_with('/')
        || (bytes.len() >= 3
            && bytes.first().is_some_and(u8::is_ascii_alphabetic)
            && bytes.get(1) == Some(&b':')
            && matches!(bytes.get(2), Some(b'/' | b'\\')))
}

/// Executa `sql` e devolve até `limit` linhas.
pub fn query(endpoint: &Endpoint, sql: &str, limit: usize) -> Result<QueryResult, DbError> {
    match endpoint.engine {
        Engine::Sqlite => sqlite::query(&endpoint.sqlite_path, sql, limit, TIMEOUT),
        Engine::Postgres => postgres::query(&endpoint.postgres, sql, limit, TIMEOUT),
        other => Err(DbError::UnsupportedEngine(other)),
    }
}

/// Executa DML/DDL e devolve as linhas afetadas.
pub fn execute(endpoint: &Endpoint, sql: &str) -> Result<QueryResult, DbError> {
    match endpoint.engine {
        Engine::Sqlite => sqlite::execute(&endpoint.sqlite_path, sql, TIMEOUT),
        Engine::Postgres => postgres::execute(&endpoint.postgres, sql, TIMEOUT),
        other => Err(DbError::UnsupportedEngine(other)),
    }
}

/// Roda os statements em sequência e devolve o resultado do **último** ("run script" dos
/// clients). Cada statement abre a própria conexão: temp table e transação não atravessam.
pub fn run_statements(
    endpoint: &Endpoint,
    statements: &[String],
    limit: Option<usize>,
) -> Result<QueryResult, DbError> {
    let mut last = None;
    for sql in statements {
        last = Some(query(endpoint, sql, limit.unwrap_or(DEFAULT_LIMIT))?);
    }
    last.ok_or(DbError::NothingToRun)
}

/// Tabelas e views do banco.
pub fn tables(endpoint: &Endpoint) -> Result<Vec<SchemaTable>, DbError> {
    let sql = schema::schema_sql(endpoint.engine, None, None)?;
    Ok(schema::tables_from(&query(endpoint, &sql, SCHEMA_LIMIT)?))
}

/// Colunas de `table` (em `schema_name`, quando o motor tem schemas).
pub fn columns(
    endpoint: &Endpoint,
    table: &str,
    schema_name: Option<&str>,
) -> Result<Vec<SchemaColumn>, DbError> {
    let sql = schema::schema_sql(endpoint.engine, Some(table), schema_name)?;
    Ok(schema::columns_from(&query(endpoint, &sql, SCHEMA_LIMIT)?))
}

/// Testa a conexão com o ping mínimo.
pub fn ping(endpoint: &Endpoint) -> Result<(), DbError> {
    query(endpoint, "SELECT 1", 1).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::{Access, Origin};

    fn sqlite_connection(path: &str) -> Connection {
        Connection::sqlite("app", path, Origin::Registered, Access::Read, true)
    }

    #[test]
    fn relative_sqlite_paths_resolve_against_the_workspace_root() {
        let root = Path::new("/work/space");
        let relative = Endpoint::resolve(&sqlite_connection("data/app.db"), root, None).unwrap();
        assert_eq!(relative.sqlite_path, Path::new("/work/space/data/app.db"));
        let absolute = Endpoint::resolve(&sqlite_connection("/tmp/x.db"), root, None).unwrap();
        assert_eq!(absolute.sqlite_path, Path::new("/tmp/x.db"));
        let drive = Endpoint::resolve(&sqlite_connection("C:\\x.db"), root, None).unwrap();
        assert_eq!(drive.sqlite_path, Path::new("C:\\x.db"));
    }

    #[test]
    fn engines_without_a_driver_are_refused_up_front() {
        let conn = Connection::from_json(
            &serde_json::json!({"name": "m", "url": "mysql://u@h/db"}),
            Origin::Registered,
        )
        .unwrap();
        let error = Endpoint::resolve(&conn, Path::new("/"), None).unwrap_err();
        assert_eq!(error, DbError::UnsupportedEngine(Engine::Mysql));
        assert_eq!(error.kind(), "unsupported_engine");
    }

    #[test]
    fn run_statements_keeps_the_last_result_and_stops_on_the_first_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.db");
        rusqlite::Connection::open(&path).unwrap();
        let endpoint =
            Endpoint::resolve(&sqlite_connection(path.to_str().unwrap()), dir.path(), None)
                .unwrap();
        let script = [
            "CREATE TABLE t (n INTEGER)".to_string(),
            "INSERT INTO t VALUES (1), (2)".to_string(),
            "SELECT n FROM t ORDER BY n".to_string(),
        ];
        let result = run_statements(&endpoint, &script, None).unwrap();
        assert_eq!(result.rows.len(), 2);

        let broken = [
            "DELETE FROM t".to_string(),
            "SELEC".to_string(),
            "SELECT 1".to_string(),
        ];
        assert!(matches!(
            run_statements(&endpoint, &broken, None),
            Err(DbError::QueryFailed(_))
        ));
        assert_eq!(
            run_statements(&endpoint, &[], None),
            Err(DbError::NothingToRun)
        );
        // a primeira parte do script já tinha rodado
        assert!(query(&endpoint, "SELECT n FROM t", 5)
            .unwrap()
            .rows
            .is_empty());
    }
}
