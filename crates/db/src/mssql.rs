//! Driver SQL Server (`tiberius` sobre `rustls`). Modelo efêmero: abre, executa e fecha.

use std::fmt;
use std::time::{Duration, Instant};

use futures_util::TryStreamExt as _;
use tiberius::{AuthMethod, Client, ColumnData, Config, EncryptionLevel, QueryItem};
use tokio::net::TcpStream;
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt as _};
use zeroize::Zeroize;

use crate::error::DbError;
use crate::result::{Cell, QueryResult};

/// Destino de uma conexão SQL Server, já com a senha resolvida.
#[derive(Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: Option<String>,
    pub database: String,
    /// `?encrypt=true|false` (`None` = padrão do driver).
    pub encrypt: Option<bool>,
    /// `?trustcert=true` para desenvolvimento com certificado autoassinado.
    pub trust_cert: bool,
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
            .field("encrypt", &self.encrypt)
            .field("trust_cert", &self.trust_cert)
            .finish()
    }
}

fn config(target: &Target) -> Config {
    let mut config = Config::new();
    config.host(&target.host);
    config.port(target.port);
    if !target.database.is_empty() {
        config.database(&target.database);
    }
    if !target.user.is_empty() || target.password.is_some() {
        config.authentication(AuthMethod::sql_server(
            &target.user,
            target.password.as_deref().unwrap_or_default(),
        ));
    }
    if target.trust_cert {
        config.trust_cert();
    }
    match target.encrypt {
        Some(true) => config.encryption(EncryptionLevel::Required),
        Some(false) => config.encryption(EncryptionLevel::Off),
        None => {}
    }
    config
}

async fn connect(target: &Target, timeout: Duration) -> Result<Client<Compat<TcpStream>>, DbError> {
    let cfg = config(target);
    let tcp = tokio::time::timeout(timeout, TcpStream::connect(cfg.get_addr()))
        .await
        .map_err(|_| DbError::Timeout(timeout.as_secs()))?
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
    let _ = tcp.set_nodelay(true);
    Client::connect(cfg, tcp.compat_write())
        .await
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))
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

fn cell(data: ColumnData<'_>) -> Cell {
    match data {
        ColumnData::U8(Some(v)) => Cell::Int(i64::from(v)),
        ColumnData::I16(Some(v)) => Cell::Int(i64::from(v)),
        ColumnData::I32(Some(v)) => Cell::Int(i64::from(v)),
        ColumnData::I64(Some(v)) => Cell::Int(v),
        ColumnData::F32(Some(v)) => Cell::Real(f64::from(v)),
        ColumnData::F64(Some(v)) => Cell::Real(v),
        ColumnData::Bit(Some(v)) => Cell::Bool(v),
        ColumnData::String(Some(s)) => Cell::Text(s.into_owned()),
        ColumnData::Guid(Some(u)) => Cell::Text(u.to_string()),
        ColumnData::Binary(Some(b)) => Cell::Blob(b.into_owned()),
        ColumnData::Numeric(Some(n)) => Cell::Text(n.to_string()),
        ColumnData::Xml(Some(x)) => Cell::Text(x.to_string()),
        ColumnData::DateTime(Some(dt)) => Cell::Text(format_days_and_time(
            1900,
            i64::from(dt.days()),
            u64::from(dt.seconds_fragments()) * 10_000_000 / 3,
        )),
        ColumnData::SmallDateTime(Some(dt)) => Cell::Text(format_days_and_time(
            1900,
            i64::from(dt.days()),
            u64::from(dt.seconds_fragments()) * 60_000_000_000,
        )),
        ColumnData::Date(Some(d)) => Cell::Text(format_days(1, i64::from(d.days()))),
        ColumnData::Time(Some(t)) => Cell::Text(format_time_ns(time_ns(t))),
        ColumnData::DateTime2(Some(dt)) => {
            let date = format_days(1, i64::from(dt.date().days()));
            let time = format_time_ns(time_ns(dt.time()));
            Cell::Text(format!("{date} {time}"))
        }
        ColumnData::DateTimeOffset(Some(dto)) => {
            let dt = dto.datetime2();
            let date = format_days(1, i64::from(dt.date().days()));
            let time = format_time_ns(time_ns(dt.time()));
            let off = dto.offset();
            let sign = if off >= 0 { '+' } else { '-' };
            let abs = off.abs();
            Cell::Text(format!(
                "{date} {time} {sign}{:02}:{:02}",
                abs / 60,
                abs % 60
            ))
        }
        ColumnData::U8(None)
        | ColumnData::I16(None)
        | ColumnData::I32(None)
        | ColumnData::I64(None)
        | ColumnData::F32(None)
        | ColumnData::F64(None)
        | ColumnData::Bit(None)
        | ColumnData::String(None)
        | ColumnData::Guid(None)
        | ColumnData::Binary(None)
        | ColumnData::Numeric(None)
        | ColumnData::Xml(None)
        | ColumnData::DateTime(None)
        | ColumnData::SmallDateTime(None)
        | ColumnData::Date(None)
        | ColumnData::Time(None)
        | ColumnData::DateTime2(None)
        | ColumnData::DateTimeOffset(None) => Cell::Null,
    }
}

fn time_ns(t: tiberius::time::Time) -> u64 {
    let scale = 10_u64.saturating_pow(u32::from(9_u8.saturating_sub(t.scale())));
    t.increments().saturating_mul(scale)
}

fn format_time_ns(ns: u64) -> String {
    let total_secs = ns / 1_000_000_000;
    let frac = ns % 1_000_000_000;
    let h = total_secs / 3600;
    let m = (total_secs % 3600) / 60;
    let s = total_secs % 60;
    if frac == 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        let text = format!("{frac:09}");
        let trimmed = text.trim_end_matches('0');
        format!("{h:02}:{m:02}:{s:02}.{trimmed}")
    }
}

fn format_days(base_year: i64, days: i64) -> String {
    // Civil-from-days (Howard Hinnant): 1900-01-01 é day 693595 após 0001-01-01, e 1970-01-01 é
    // day 719162 após 0001-01-01.
    let epoch_offset = if base_year == 1900 { -25567 } else { -719_162 };
    let z = days + epoch_offset + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    format!("{year:04}-{m:02}-{d:02}")
}

fn format_days_and_time(base_year: i64, days: i64, ns: u64) -> String {
    format!("{} {}", format_days(base_year, days), format_time_ns(ns))
}

async fn run_query(
    target: &Target,
    sql: &str,
    limit: usize,
    timeout: Duration,
) -> Result<QueryResult, DbError> {
    let started = Instant::now();
    let mut client = connect(target, timeout).await?;
    let mut stream = client
        .simple_query(sql)
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
    let mut names: Vec<String> = Vec::new();
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut truncated = false;
    while let Some(item) = stream
        .try_next()
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?
    {
        match item {
            QueryItem::Metadata(meta) => {
                if names.is_empty() {
                    names = meta
                        .columns()
                        .iter()
                        .map(|c| c.name().to_string())
                        .collect();
                }
            }
            QueryItem::Row(row) => {
                if rows.len() >= limit {
                    truncated = true;
                    break;
                }
                if names.is_empty() {
                    names = row.columns().iter().map(|c| c.name().to_string()).collect();
                }
                rows.push(row.into_iter().map(cell).collect());
            }
        }
    }
    drop(stream);
    let _ = client.close().await;
    if names.is_empty() || rows.is_empty() {
        return Ok(QueryResult::empty(started.elapsed()));
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
    let mut client = connect(target, timeout).await?;
    let result = client
        .execute(sql, &[])
        .await
        .map_err(|e| DbError::QueryFailed(e.to_string()))?;
    let affected = result.total();
    let _ = client.close().await;
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

    fn target(encrypt: Option<bool>, trust_cert: bool) -> Target {
        Target {
            host: "127.0.0.1".into(),
            port: 1,
            user: "sa".into(),
            password: Some("mssql-secret".into()),
            database: "master".into(),
            encrypt,
            trust_cert,
        }
    }

    #[test]
    fn date_math_matches_sql_server_epochs() {
        assert_eq!(format_days(1, 0), "0001-01-01");
        assert_eq!(format_days(1900, 0), "1900-01-01");
        assert_eq!(format_days(1900, 25567), "1970-01-01");
        assert_eq!(format_days(1, 719_162), "1970-01-01");
    }

    #[test]
    fn debug_redacts_password() {
        let t = target(Some(true), true);
        let dbg = format!("{t:?}");
        assert!(!dbg.contains("mssql-secret"), "{dbg}");
        assert!(dbg.contains("[REDACTED]"), "{dbg}");
    }

    #[test]
    fn unreachable_server_is_a_connection_failure() {
        let error = query(&target(None, true), "SELECT 1", 1, Duration::from_secs(3)).unwrap_err();
        assert!(matches!(error, DbError::ConnectionFailed(_)), "{error:?}");
        assert_eq!(error.kind(), "connection_failed");
    }
}
