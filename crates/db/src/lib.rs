//! Consultas SQL no PostgreSQL, sem UI (MVP).
//!
//! Usa `simple_query`, que devolve cada valor como texto: funciona para qualquer
//! tipo de coluna sem mapear tipos. Limite de linhas e de tempo protegem a UI.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

use std::time::Duration;

use thiserror::Error;
use tokio_postgres::{NoTls, SimpleQueryMessage};

/// Máximo de linhas devolvidas; o resto é descartado e sinalizado em `truncated`.
pub const MAX_ROWS: usize = 1_000;

/// Tempo máximo para conectar e executar a consulta.
pub const QUERY_TIMEOUT: Duration = Duration::from_secs(30);

/// Resultado de uma consulta: nomes das colunas e linhas como texto (`None` = NULL).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub truncated: bool,
    /// Linhas afetadas por comandos sem resultado (INSERT, UPDATE...).
    pub affected: Option<u64>,
}

/// Erro tipado; a UI escolhe a frase. Detalhes vêm do driver, sem a senha.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DbError {
    #[error("invalid connection string")]
    InvalidConfig,
    #[error("connection failed: {0}")]
    Connect(String),
    #[error("query failed: {0}")]
    Query(String),
    #[error("timed out")]
    Timeout,
    #[error("runtime failed: {0}")]
    Runtime(String),
}

/// Conecta em `conn` (formato libpq ou URL `postgres://`) e executa `sql`.
/// Bloqueante: chame fora da thread de UI.
pub fn run_query(conn: &str, sql: &str) -> Result<QueryResult, DbError> {
    let config: tokio_postgres::Config = conn.parse().map_err(|_| DbError::InvalidConfig)?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| DbError::Runtime(e.to_string()))?;
    runtime.block_on(async {
        tokio::time::timeout(QUERY_TIMEOUT, execute(config, sql))
            .await
            .map_err(|_| DbError::Timeout)?
    })
}

async fn execute(config: tokio_postgres::Config, sql: &str) -> Result<QueryResult, DbError> {
    let (client, connection) = config
        .connect(NoTls)
        .await
        .map_err(|e| DbError::Connect(e.to_string()))?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let messages = client
        .simple_query(sql)
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;
    Ok(collect(messages))
}

fn collect(messages: Vec<SimpleQueryMessage>) -> QueryResult {
    let mut result = QueryResult::default();
    for message in messages {
        match message {
            SimpleQueryMessage::Row(row) => {
                if result.columns.is_empty() {
                    result.columns = row.columns().iter().map(|c| c.name().to_string()).collect();
                }
                if result.rows.len() >= MAX_ROWS {
                    result.truncated = true;
                    continue;
                }
                result.rows.push(
                    (0..row.len())
                        .map(|i| row.get(i).map(str::to_string))
                        .collect(),
                );
            }
            SimpleQueryMessage::CommandComplete(n) => result.affected = Some(n),
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_de_conexao_invalida_falha_sem_rede() {
        let err = run_query("port=nao-e-numero", "select 1").unwrap_err();
        assert_eq!(err, DbError::InvalidConfig);
    }

    #[test]
    fn servidor_inacessivel_falha_como_connect() {
        let conn = "host=127.0.0.1 port=1 user=x dbname=x connect_timeout=2";
        let err = run_query(conn, "select 1").unwrap_err();
        assert!(matches!(err, DbError::Connect(_)), "{err:?}");
    }
}
