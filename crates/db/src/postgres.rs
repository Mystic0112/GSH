//! Driver PostgreSQL (`tokio-postgres`, sem TLS por enquanto). Modelo efêmero: cada chamada
//! abre a conexão, executa e fecha, num runtime próprio — o chamador usa `spawn_blocking`.
//!
//! Os valores vêm do protocolo de texto (`simple_query`), que serve para qualquer tipo; os
//! tipos de coluna vêm de um `prepare` e só bool, inteiros, floats e bytea viram tipados.

use std::error::Error as _;
use std::time::{Duration, Instant};

use tokio_postgres::types::Type;
use tokio_postgres::{Client, Config, NoTls, SimpleQueryMessage};

use crate::error::DbError;
use crate::result::{Cell, QueryResult};

/// Destino de uma conexão Postgres, já com a senha resolvida.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub database: String,
    /// `?sslmode=` da URL.
    pub sslmode: Option<String>,
}

fn describe(error: &tokio_postgres::Error) -> String {
    if let Some(db) = error.as_db_error() {
        return match db.detail() {
            Some(detail) => format!("{}\n{detail}", db.message()),
            None => db.message().to_string(),
        };
    }
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

fn query_error(error: &tokio_postgres::Error) -> DbError {
    DbError::QueryFailed(describe(error))
}

fn config(target: &Target, timeout: Duration) -> Result<Config, DbError> {
    // sem TLS neste build: pedir TLS à força falha alto, nunca cai em conexão aberta
    if let Some(mode @ ("require" | "verify-ca" | "verify-full")) = target.sslmode.as_deref() {
        return Err(DbError::TlsUnavailable(mode.to_string()));
    }
    let mut config = Config::new();
    config
        .host(&target.host)
        .port(target.port)
        .user(&target.user)
        .connect_timeout(timeout);
    if !target.database.is_empty() {
        config.dbname(&target.database);
    }
    if let Some(password) = &target.password {
        config.password(password);
    }
    if target.sslmode.as_deref() == Some("disable") {
        config.ssl_mode(tokio_postgres::config::SslMode::Disable);
    }
    Ok(config)
}

async fn connect(target: &Target, timeout: Duration) -> Result<Client, DbError> {
    let (client, connection) = config(target, timeout)?
        .connect(NoTls)
        .await
        .map_err(|e| DbError::ConnectionFailed(describe(&e)))?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    Ok(client)
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

/// Valor de texto do protocolo na célula tipada do resultado.
fn cell(ty: &Type, text: Option<&str>) -> Cell {
    let Some(text) = text else {
        return Cell::Null;
    };
    let typed = if *ty == Type::BOOL {
        match text {
            "t" => Some(Cell::Bool(true)),
            "f" => Some(Cell::Bool(false)),
            _ => None,
        }
    } else if [Type::INT2, Type::INT4, Type::INT8, Type::OID].contains(ty) {
        text.parse().ok().map(Cell::Int)
    } else if [Type::FLOAT4, Type::FLOAT8].contains(ty) {
        text.parse().ok().map(Cell::Real)
    } else if *ty == Type::BYTEA {
        text.strip_prefix("\\x").and_then(hex).map(Cell::Blob)
    } else {
        None
    };
    typed.unwrap_or_else(|| Cell::Text(text.to_string()))
}

fn hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    (0..text.len() / 2)
        .map(|index| {
            let pair = text.get(index * 2..index * 2 + 2)?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect()
}

async fn run_query(
    target: &Target,
    sql: &str,
    limit: usize,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let client = connect(target, timeout).await?;
    let statement = client.prepare(sql).await.map_err(|e| query_error(&e))?;
    let types: Vec<Type> = statement
        .columns()
        .iter()
        .map(|column| column.type_().clone())
        .collect();
    let messages = client
        .simple_query(sql)
        .await
        .map_err(|e| query_error(&e))?;
    if types.is_empty() {
        return Ok(QueryResult::empty(started.elapsed()));
    }
    let mut names: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut truncated = false;
    for message in messages {
        let SimpleQueryMessage::Row(row) = message else {
            continue;
        };
        if rows.len() >= limit {
            truncated = true;
            break;
        }
        if names.is_empty() {
            names = row.columns().iter().map(|c| c.name().to_string()).collect();
        }
        rows.push(
            types
                .iter()
                .enumerate()
                .map(|(index, ty)| cell(ty, row.get(index)))
                .collect(),
        );
    }
    Ok(QueryResult::from_rows(
        names,
        rows,
        truncated,
        started.elapsed(),
    ))
}

async fn run_execute(
    target: &Target,
    sql: &str,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let client = connect(target, timeout).await?;
    let messages = client
        .simple_query(sql)
        .await
        .map_err(|e| query_error(&e))?;
    let affected = messages
        .iter()
        .filter_map(|message| match message {
            SimpleQueryMessage::CommandComplete(count) => Some(*count),
            _ => None,
        })
        .last()
        .unwrap_or(0);
    Ok(QueryResult::affected(affected, started.elapsed()))
}

/// Executa `sql` e devolve até `limit` linhas; uma a mais marca `truncated`.
pub fn query(
    target: &Target,
    sql: &str,
    limit: usize,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    block_on(timeout, run_query(target, sql, limit, timeout))
}

/// Executa DML/DDL e devolve as linhas afetadas.
pub fn execute(target: &Target, sql: &str, timeout: Duration) -> Result<QueryResult, DbError> {
    block_on(timeout, run_execute(target, sql, timeout))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(sslmode: Option<&str>) -> Target {
        Target {
            host: "127.0.0.1".into(),
            port: 1,
            user: "x".into(),
            password: None,
            database: "x".into(),
            sslmode: sslmode.map(str::to_string),
        }
    }

    #[test]
    fn text_values_become_typed_cells() {
        assert_eq!(cell(&Type::BOOL, Some("t")), Cell::Bool(true));
        assert_eq!(cell(&Type::INT4, Some("-7")), Cell::Int(-7));
        assert_eq!(
            cell(&Type::INT8, Some("9000000000")),
            Cell::Int(9_000_000_000)
        );
        assert_eq!(cell(&Type::FLOAT8, Some("2.5")), Cell::Real(2.5));
        assert_eq!(
            cell(&Type::BYTEA, Some("\\x0aff")),
            Cell::Blob(vec![10, 255])
        );
        assert_eq!(
            cell(&Type::NUMERIC, Some("1.10")),
            Cell::Text("1.10".into())
        );
        assert_eq!(cell(&Type::TEXT, None), Cell::Null);
        // valor que não casa com o tipo declarado não se perde
        assert_eq!(cell(&Type::INT4, Some("oops")), Cell::Text("oops".into()));
        assert_eq!(
            cell(&Type::BYTEA, Some("\\xzz")),
            Cell::Text("\\xzz".into())
        );
    }

    #[test]
    fn tls_modes_fail_loudly_instead_of_connecting_in_clear() {
        let t = Duration::from_secs(1);
        for mode in ["require", "verify-ca", "verify-full"] {
            let error = query(&target(Some(mode)), "select 1", 1, t).unwrap_err();
            assert_eq!(error, DbError::TlsUnavailable(mode.into()));
        }
    }

    #[test]
    fn unreachable_server_is_a_connection_failure() {
        let error = query(&target(None), "select 1", 1, Duration::from_secs(3)).unwrap_err();
        assert!(matches!(error, DbError::ConnectionFailed(_)), "{error:?}");
        assert_eq!(error.kind(), "connection_failed");
    }
}
