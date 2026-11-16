//! Driver MySQL (`mysql_async` sobre `rustls`). Modelo efêmero: abre, executa e fecha.

use std::fmt;
use std::time::{Duration, Instant};

use mysql_async::consts::ColumnType;
use mysql_async::prelude::Queryable as _;
use mysql_async::{Column, Conn, OptsBuilder, Row, SslOpts, Value};
use zeroize::Zeroize;

/// ID da collation `binary` no protocolo MySQL (`BINARY`/`VARBINARY`/`BLOB`).
const BINARY_COLLATION_ID: u16 = 63;

use crate::error::DbError;
use crate::result::{Cell, QueryResult};

/// Destino de uma conexão MySQL, já com a senha resolvida.
#[derive(Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub database: String,
    /// `?ssl-mode=` da URL (minúsculo: `disabled`, `preferred`, `required`, `verify_ca`,
    /// `verify_identity`).
    pub ssl_mode: Option<String>,
}

impl Drop for Target {
    fn drop(&mut self) {
        self.password.zeroize();
    }
}

impl fmt::Debug for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Target")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("user", &self.user)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .field("database", &self.database)
            .field("ssl_mode", &self.ssl_mode)
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedSslMode {
    Disabled,
    Preferred,
    Required,
    VerifyCa,
    VerifyIdentity,
}

impl ParsedSslMode {
    fn from_opt(raw: Option<&str>) -> Result<Self, DbError> {
        match raw.map(str::to_ascii_lowercase).as_deref() {
            None | Some("") | Some("preferred") => Ok(Self::Preferred),
            Some("disabled") => Ok(Self::Disabled),
            Some("required") => Ok(Self::Required),
            Some("verify_ca") => Ok(Self::VerifyCa),
            Some("verify_identity") => Ok(Self::VerifyIdentity),
            Some(other) => Err(DbError::ConnectionFailed(format!(
                "unsupported MySQL ssl-mode: {other}"
            ))),
        }
    }

    fn ssl_opts(self) -> Option<SslOpts> {
        match self {
            Self::Disabled => None,
            Self::Preferred | Self::Required => Some(
                SslOpts::default()
                    .with_danger_accept_invalid_certs(true)
                    .with_danger_skip_domain_validation(true),
            ),
            Self::VerifyCa => Some(SslOpts::default().with_danger_skip_domain_validation(true)),
            Self::VerifyIdentity => Some(SslOpts::default()),
        }
    }
}

fn opts(target: &Target, ssl: Option<SslOpts>) -> OptsBuilder {
    let mut builder = OptsBuilder::default()
        .ip_or_hostname(&target.host)
        .tcp_port(target.port)
        .ssl_opts(ssl);
    if !target.user.is_empty() {
        builder = builder.user(Some(target.user.as_str()));
    }
    if let Some(password) = &target.password {
        if !password.is_empty() {
            builder = builder.pass(Some(password.as_str()));
        }
    }
    if !target.database.is_empty() {
        builder = builder.db_name(Some(target.database.as_str()));
    }
    builder
}

async fn connect(target: &Target) -> Result<Conn, DbError> {
    let mode = ParsedSslMode::from_opt(target.ssl_mode.as_deref())?;
    let primary = opts(target, mode.ssl_opts());
    match Conn::new(primary).await {
        Ok(conn) => Ok(conn),
        Err(_err) if mode == ParsedSslMode::Preferred => {
            // `preferred`: tenta TLS e cai para claro só se o servidor recusar SSL.
            let plain = opts(target, None);
            Conn::new(plain)
                .await
                .map_err(|e| DbError::ConnectionFailed(e.to_string()))
        }
        Err(err) => Err(DbError::ConnectionFailed(err.to_string())),
    }
}

fn block_on<T>(
    timeout: Duration,
    work: impl std::future::Future<Output = Result<T, DbError>>,
) -> Result<T, DbError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
    runtime.block_on(async {
        tokio::time::timeout(timeout, work)
            .await
            .map_err(|_| DbError::Timeout(timeout.as_secs()))?
    })
}

fn cell(column: &Column, value: Option<Value>) -> Cell {
    let Some(value) = value else {
        return Cell::Null;
    };
    match value {
        Value::NULL => Cell::Null,
        Value::Int(v) => Cell::Int(v),
        Value::UInt(v) => i64::try_from(v).map_or_else(|_| Cell::Text(v.to_string()), Cell::Int),
        Value::Float(v) => Cell::Real(f64::from(v)),
        Value::Double(v) => Cell::Real(v),
        Value::Date(y, m, d, h, min, s, us) => {
            if h == 0 && min == 0 && s == 0 && us == 0 {
                Cell::Text(format!("{y:04}-{m:02}-{d:02}"))
            } else if us == 0 {
                Cell::Text(format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02}:{s:02}"))
            } else {
                Cell::Text(format!(
                    "{y:04}-{m:02}-{d:02} {h:02}:{min:02}:{s:02}.{us:06}"
                ))
            }
        }
        Value::Time(neg, d, h, min, s, us) => {
            let sign = if neg { "-" } else { "" };
            let total_hours = u64::from(d) * 24 + u64::from(h);
            if us == 0 {
                Cell::Text(format!("{sign}{total_hours:02}:{min:02}:{s:02}"))
            } else {
                Cell::Text(format!("{sign}{total_hours:02}:{min:02}:{s:02}.{us:06}"))
            }
        }
        Value::Bytes(bytes) => decode_bytes(column, bytes),
    }
}

fn decode_bytes(column: &Column, bytes: Vec<u8>) -> Cell {
    let ty = column.column_type();
    if matches!(
        ty,
        ColumnType::MYSQL_TYPE_TINY
            | ColumnType::MYSQL_TYPE_SHORT
            | ColumnType::MYSQL_TYPE_LONG
            | ColumnType::MYSQL_TYPE_LONGLONG
            | ColumnType::MYSQL_TYPE_INT24
            | ColumnType::MYSQL_TYPE_YEAR
    ) {
        if let Ok(text) = std::str::from_utf8(&bytes) {
            if let Ok(n) = text.parse::<i64>() {
                return Cell::Int(n);
            }
        }
    } else if matches!(
        ty,
        ColumnType::MYSQL_TYPE_FLOAT | ColumnType::MYSQL_TYPE_DOUBLE
    ) {
        if let Ok(text) = std::str::from_utf8(&bytes) {
            if let Ok(n) = text.parse::<f64>() {
                return Cell::Real(n);
            }
        }
    } else if column.character_set() == BINARY_COLLATION_ID
        && matches!(
            ty,
            ColumnType::MYSQL_TYPE_BLOB
                | ColumnType::MYSQL_TYPE_TINY_BLOB
                | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
                | ColumnType::MYSQL_TYPE_LONG_BLOB
                | ColumnType::MYSQL_TYPE_STRING
                | ColumnType::MYSQL_TYPE_VAR_STRING
                | ColumnType::MYSQL_TYPE_BIT
        )
    {
        return Cell::Blob(bytes);
    }

    match String::from_utf8(bytes) {
        Ok(text) => Cell::Text(text),
        Err(err) => Cell::Blob(err.into_bytes()),
    }
}

async fn run_query(target: &Target, sql: &str, limit: usize) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let mut conn = connect(target).await?;
    let result = conn
        .query_iter(sql)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
    let columns: Vec<Column> = result.columns_ref().to_vec();
    if columns.is_empty() {
        let _ = result.drop_result().await;
        let _ = conn.disconnect().await;
        return Ok(QueryResult::empty(started.elapsed()));
    }
    let names: Vec<String> = columns.iter().map(|c| c.name_str().into_owned()).collect();
    let mut rows_out: Vec<Vec<Cell>> = Vec::new();
    let mut truncated = false;
    let raw_rows: Vec<Row> = result
        .collect_and_drop()
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
    let _ = conn.disconnect().await;
    for row in raw_rows {
        if rows_out.len() >= limit {
            truncated = true;
            break;
        }
        let values = row.unwrap_raw();
        let cells = columns
            .iter()
            .zip(values.into_iter().chain(std::iter::repeat(None)))
            .map(|(col, val)| cell(col, val))
            .collect();
        rows_out.push(cells);
    }
    Ok(QueryResult::from_rows(
        names,
        rows_out,
        truncated,
        started.elapsed(),
    ))
}

async fn run_execute(target: &Target, sql: &str) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let mut conn = connect(target).await?;
    conn.query_drop(sql)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
    let affected = conn.affected_rows();
    let _ = conn.disconnect().await;
    Ok(QueryResult::affected(affected, started.elapsed()))
}

/// Executa `sql` e devolve até `limit` linhas; uma a mais marca `truncated`.
pub fn query(
    target: &Target,
    sql: &str,
    limit: usize,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    block_on(timeout, run_query(target, sql, limit))
}

/// Executa DML/DDL e devolve as linhas afetadas.
pub fn execute(target: &Target, sql: &str, timeout: Duration) -> Result<QueryResult, DbError> {
    block_on(timeout, run_execute(target, sql))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(ssl_mode: Option<&str>) -> Target {
        Target {
            host: "127.0.0.1".into(),
            port: 1,
            user: "root".into(),
            password: Some("mysql-secret".into()),
            database: "app".into(),
            ssl_mode: ssl_mode.map(str::to_string),
        }
    }

    #[test]
    fn ssl_modes_map_to_expected_options() {
        assert!(ParsedSslMode::from_opt(None).unwrap().ssl_opts().is_some());
        assert!(ParsedSslMode::from_opt(Some("disabled"))
            .unwrap()
            .ssl_opts()
            .is_none());
        assert!(ParsedSslMode::from_opt(Some("REQUIRED"))
            .unwrap()
            .ssl_opts()
            .unwrap()
            .accept_invalid_certs());
        assert!(ParsedSslMode::from_opt(Some("verify_ca"))
            .unwrap()
            .ssl_opts()
            .unwrap()
            .skip_domain_validation());
        assert!(!ParsedSslMode::from_opt(Some("verify_identity"))
            .unwrap()
            .ssl_opts()
            .unwrap()
            .skip_domain_validation());
        assert!(ParsedSslMode::from_opt(Some("invalid")).is_err());
    }

    #[test]
    fn debug_redacts_password() {
        let t = target(Some("required"));
        let dbg = format!("{t:?}");
        assert!(!dbg.contains("mysql-secret"), "{dbg}");
        assert!(dbg.contains("[REDACTED]"), "{dbg}");
    }

    #[test]
    fn unreachable_server_is_a_connection_failure() {
        let error = query(
            &target(Some("disabled")),
            "SELECT 1",
            1,
            Duration::from_secs(3),
        )
        .unwrap_err();
        assert!(matches!(error, DbError::ConnectionFailed(_)), "{error:?}");
        assert_eq!(error.kind(), "connection_failed");
    }
}
