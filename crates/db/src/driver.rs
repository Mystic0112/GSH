//! Execução e introspecção sobre um [`Endpoint`] já resolvido (motor, caminho, senha, túnel).
//!
//! Cada função é bloqueante: o chamador as roda fora da thread de UI.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Map, Value};

use crate::connection::{Connection, Engine};
use crate::error::DbError;
use crate::mongo;
use crate::mssql;
use crate::mysql;
use crate::postgres;
use crate::redis;
use crate::result::QueryResult;
use crate::schema::{self, SchemaColumn, SchemaTable};
use crate::sqlite;
use crate::ssh::SshTunnelManager;

/// Linhas devolvidas quando nem a chamada nem o `.dbq` dizem o limite.
pub const DEFAULT_LIMIT: usize = 200;

/// Teto de linhas das consultas de schema.
const SCHEMA_LIMIT: usize = 10_000;

/// Tempo máximo de uma consulta (conectar incluso).
pub const TIMEOUT: Duration = Duration::from_secs(30);

/// Para onde a conexão aponta, com caminho, senha e eventual túnel resolvidos.
#[derive(Clone, PartialEq, Eq)]
pub struct Endpoint {
    engine: Engine,
    sqlite_path: PathBuf,
    postgres: postgres::Target,
    mysql: mysql::Target,
    mssql: mssql::Target,
    redis: redis::Target,
    mongo: mongo::Target,
}

impl fmt::Debug for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Endpoint");
        s.field("engine", &self.engine);
        match self.engine {
            Engine::Sqlite => s.field("sqlite_path", &self.sqlite_path),
            Engine::Postgres => s.field("postgres", &self.postgres),
            Engine::Mysql => s.field("mysql", &self.mysql),
            Engine::Mssql => s.field("mssql", &self.mssql),
            Engine::Redis => s.field("redis", &self.redis),
            Engine::Mongo => s.field("mongo", &self.mongo),
        };
        s.finish()
    }
}

impl Endpoint {
    /// Resolve `conn` contra a raiz do workspace (SQLite relativo é relativo à raiz) e `password`.
    pub fn resolve(
        conn: &Connection,
        root: &Path,
        password: Option<String>,
    ) -> Result<Self, DbError> {
        let raw = conn.sqlite_path();
        let sqlite_path = if is_absolute(raw) {
            PathBuf::from(raw)
        } else {
            root.join(raw)
        };
        let host = conn.host();
        let port = conn.port();
        let user = conn.user();
        let database = conn.database();
        let socks_proxy = match (
            conn.query_param("proxyHost"),
            conn.query_param("proxyPort")
                .and_then(|p| p.parse::<u16>().ok()),
        ) {
            (Some(h), Some(p)) if !h.is_empty() => Some((h, p)),
            _ => None,
        };
        let mongo_uri = if conn.engine == Engine::Mongo {
            mongo::mongo_uri(conn, password.as_deref())
        } else {
            String::new()
        };
        let mongo_db = if conn.engine == Engine::Mongo {
            mongo::default_database(conn)
        } else {
            String::new()
        };

        Ok(Self {
            engine: conn.engine,
            sqlite_path,
            postgres: postgres::Target {
                host: host.clone(),
                port,
                user: user.clone(),
                password: (conn.engine == Engine::Postgres)
                    .then(|| password.clone())
                    .flatten(),
                database: database.clone(),
                sslmode: conn.query_param("sslmode"),
            },
            mysql: mysql::Target {
                host: host.clone(),
                port,
                user: user.clone(),
                password: (conn.engine == Engine::Mysql)
                    .then(|| password.clone())
                    .flatten(),
                database: database.clone(),
                ssl_mode: conn.query_param("ssl-mode").map(|s| s.to_ascii_lowercase()),
            },
            mssql: mssql::Target {
                host: host.clone(),
                port,
                user: user.clone(),
                password: (conn.engine == Engine::Mssql)
                    .then(|| password.clone())
                    .flatten(),
                database: database.clone(),
                encrypt: match conn.query_param("encrypt").as_deref() {
                    Some("true") => Some(true),
                    Some("false") => Some(false),
                    _ => None,
                },
                trust_cert: conn.query_param("trustcert").as_deref() == Some("true"),
            },
            redis: redis::Target {
                host,
                port,
                user: (!user.is_empty()).then_some(user),
                password: (conn.engine == Engine::Redis).then_some(password).flatten(),
                db: database.parse().unwrap_or(0),
                tls: conn.use_tls(),
            },
            mongo: mongo::Target {
                uri: mongo_uri,
                database: mongo_db,
                socks_proxy,
            },
        })
    }

    /// Resolve a conexão passando pelo túnel SSH quando configurado (`_tunneled` do Flutter).
    pub fn resolve_with_tunnel(
        conn: &Connection,
        root: &Path,
        password: Option<String>,
        ssh_passphrase: Option<&str>,
        tunnels: &SshTunnelManager,
    ) -> Result<Self, DbError> {
        let Some(ssh) = conn
            .ssh
            .as_ref()
            .filter(|_| conn.engine.supports_ssh_tunnel())
        else {
            return Self::resolve(conn, root, password);
        };
        if conn.engine == Engine::Mongo {
            let proxy = tunnels.ensure_socks(ssh, ssh_passphrase)?;
            let proxied = conn.with_socks_proxy(&proxy.host, proxy.port);
            return Self::resolve(&proxied, root, password);
        }
        let local = tunnels.ensure(ssh, &conn.host(), conn.port(), ssh_passphrase)?;
        let redirected = conn.with_endpoint(&local.host, local.port);
        Self::resolve(&redirected, root, password)
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
        Engine::Mysql => mysql::query(&endpoint.mysql, sql, limit, TIMEOUT),
        Engine::Mssql => mssql::query(&endpoint.mssql, sql, limit, TIMEOUT),
        other => Err(DbError::UnsupportedEngine(other)),
    }
}

/// Executa DML/DDL e devolve as linhas afetadas.
pub fn execute(endpoint: &Endpoint, sql: &str) -> Result<QueryResult, DbError> {
    match endpoint.engine {
        Engine::Sqlite => sqlite::execute(&endpoint.sqlite_path, sql, TIMEOUT),
        Engine::Postgres => postgres::execute(&endpoint.postgres, sql, TIMEOUT),
        Engine::Mysql => mysql::execute(&endpoint.mysql, sql, TIMEOUT),
        Engine::Mssql => mssql::execute(&endpoint.mssql, sql, TIMEOUT),
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

/// Testa a conexão com o ping mínimo do motor.
pub fn ping(endpoint: &Endpoint) -> Result<(), DbError> {
    match endpoint.engine {
        Engine::Sqlite | Engine::Postgres | Engine::Mysql | Engine::Mssql => {
            query(endpoint, "SELECT 1", 1).map(|_| ())
        }
        Engine::Redis => redis::ping(&endpoint.redis, TIMEOUT),
        Engine::Mongo => mongo::ping(&endpoint.mongo, TIMEOUT),
    }
}

/// Executa um comando Redis.
pub fn redis_command(endpoint: &Endpoint, parts: &[String]) -> Result<Value, DbError> {
    if endpoint.engine != Engine::Redis {
        return Err(DbError::UnsupportedEngine(endpoint.engine));
    }
    redis::command(&endpoint.redis, parts, TIMEOUT)
}

/// Executa comandos Redis em lote numa única conexão.
pub fn redis_batch(endpoint: &Endpoint, commands: &[Vec<String>]) -> Result<Vec<Value>, DbError> {
    if endpoint.engine != Engine::Redis {
        return Err(DbError::UnsupportedEngine(endpoint.engine));
    }
    redis::batch(&endpoint.redis, commands, TIMEOUT)
}

/// Executa um `runCommand` no MongoDB.
pub fn mongo_command(
    endpoint: &Endpoint,
    command: &Map<String, Value>,
    database: Option<&str>,
) -> Result<Value, DbError> {
    if endpoint.engine != Engine::Mongo {
        return Err(DbError::UnsupportedEngine(endpoint.engine));
    }
    mongo::run_command(&endpoint.mongo, command, database, TIMEOUT)
}

/// Executa um comando Mongo preservando a ordem exata das chaves.
pub fn mongo_ordered_command(
    endpoint: &Endpoint,
    entries: &[(&str, Value)],
    database: Option<&str>,
) -> Result<Value, DbError> {
    if endpoint.engine != Engine::Mongo {
        return Err(DbError::UnsupportedEngine(endpoint.engine));
    }
    mongo::run_ordered_command(&endpoint.mongo, entries, database, TIMEOUT)
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
    fn every_engine_resolves_and_redacts_secrets_in_debug() {
        for url in [
            "postgres://u:super-secret@h:5432/db",
            "mysql://u:super-secret@h:3306/db",
            "mssql://u:super-secret@h:1433/db",
            "redis://u:super-secret@h:6379/0",
            "mongodb://u:super-secret@h:27017/db",
        ] {
            let conn = Connection::from_json(
                &serde_json::json!({"name": "c", "url": url}),
                Origin::Registered,
            )
            .unwrap();
            let ep = Endpoint::resolve(&conn, Path::new("/"), Some("super-secret".into())).unwrap();
            let dbg = format!("{ep:?}");
            assert!(!dbg.contains("super-secret"), "{dbg}");
        }
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
