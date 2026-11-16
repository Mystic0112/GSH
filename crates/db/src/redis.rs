//! Driver Redis (`redis` crate, TCP e TLS via `rediss://`). Modelo efêmero por chamada/lote.

use std::fmt;
use std::sync::Once;
use std::time::Duration;

use redis::{
    Client, ConnectionAddr, ConnectionInfo, IntoConnectionInfo, RedisConnectionInfo, Value,
};
use serde_json::{Map, Value as JsonValue};
use zeroize::Zeroize;

use crate::error::DbError;

static RUSTLS_INIT: Once = Once::new();

pub(crate) fn ensure_rustls_provider() {
    RUSTLS_INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

/// Destino de uma conexão Redis, já com a senha resolvida.
#[derive(Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub port: u16,
    pub user: Option<String>,
    pub password: Option<String>,
    pub db: i64,
    pub tls: bool,
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
            .field("db", &self.db)
            .field("tls", &self.tls)
            .finish()
    }
}

fn connection_info(target: &Target) -> Result<ConnectionInfo, DbError> {
    let addr = if target.tls {
        ConnectionAddr::TcpTls {
            host: target.host.clone(),
            port: target.port,
            insecure: false,
            tls_params: None,
        }
    } else {
        ConnectionAddr::Tcp(target.host.clone(), target.port)
    };
    let mut redis = RedisConnectionInfo::default().set_db(target.db);
    if let Some(user) = target.user.as_deref().filter(|u| !u.is_empty()) {
        redis = redis.set_username(user);
    }
    if let Some(password) = target.password.as_deref().filter(|p| !p.is_empty()) {
        redis = redis.set_password(password);
    }
    Ok(addr
        .into_connection_info()
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?
        .set_redis_settings(redis))
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

async fn connect(target: &Target) -> Result<redis::aio::MultiplexedConnection, DbError> {
    if target.tls {
        ensure_rustls_provider();
    }
    let client = Client::open(connection_info(target)?)
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
    client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| DbError::ConnectionFailed(e.to_string()))
}

fn build_cmd(parts: &[String]) -> Result<redis::Cmd, DbError> {
    let Some((first, rest)) = parts.split_first() else {
        return Err(DbError::QueryFailed("Empty Redis command.".into()));
    };
    let mut cmd = redis::cmd(first);
    for arg in rest {
        cmd.arg(arg.as_str());
    }
    Ok(cmd)
}

fn to_json(value: Value) -> Result<JsonValue, DbError> {
    match value {
        Value::Nil => Ok(JsonValue::Null),
        Value::Int(v) => Ok(JsonValue::from(v)),
        Value::Double(v) => {
            Ok(serde_json::Number::from_f64(v).map_or(JsonValue::Null, JsonValue::Number))
        }
        Value::Boolean(v) => Ok(JsonValue::Bool(v)),
        Value::SimpleString(s) => Ok(JsonValue::String(s)),
        Value::Okay => Ok(JsonValue::String("OK".to_string())),
        Value::BulkString(bytes) => Ok(JsonValue::String(
            String::from_utf8_lossy(&bytes).into_owned(),
        )),
        Value::VerbatimString { text, .. } => Ok(JsonValue::String(text)),
        Value::BigNumber(n) => Ok(JsonValue::String(n.to_string())),
        Value::Array(items) | Value::Set(items) | Value::Push { data: items, .. } => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                out.push(to_json(item)?);
            }
            Ok(JsonValue::Array(out))
        }
        Value::Map(pairs) => {
            let mut map = Map::new();
            for (k, v) in pairs {
                let key = json_key(to_json(k)?);
                map.insert(key, to_json(v)?);
            }
            Ok(JsonValue::Object(map))
        }
        Value::Attribute { data, .. } => to_json(*data),
        Value::ServerError(err) => Err(DbError::QueryFailed(err.to_string())),
        _ => Ok(JsonValue::Null),
    }
}

fn json_key(value: JsonValue) -> String {
    match value {
        JsonValue::String(s) => s,
        JsonValue::Null => "null".to_string(),
        other => other.to_string(),
    }
}

/// Executa um único comando Redis (`["GET", "foo"]`) e devolve o reply em JSON.
pub fn command(target: &Target, parts: &[String], timeout: Duration) -> Result<JsonValue, DbError> {
    let mut replies = batch(target, std::slice::from_ref(&parts.to_vec()), timeout)?;
    replies
        .pop()
        .ok_or_else(|| DbError::QueryFailed("Empty Redis reply.".into()))
}

/// Executa uma lista de comandos na mesma conexão Redis e devolve os replies na mesma ordem.
pub fn batch(
    target: &Target,
    commands: &[Vec<String>],
    timeout: Duration,
) -> Result<Vec<JsonValue>, DbError> {
    if commands.is_empty() {
        return Ok(Vec::new());
    }
    let cmds: Vec<redis::Cmd> = commands
        .iter()
        .map(|parts| build_cmd(parts))
        .collect::<Result<_, _>>()?;
    block_on(timeout, async move {
        let mut conn = connect(target).await?;
        let mut out = Vec::with_capacity(cmds.len());
        for cmd in cmds {
            let reply: Value = cmd
                .query_async(&mut conn)
                .await
                .map_err(|e| DbError::QueryFailed(e.to_string()))?;
            out.push(to_json(reply)?);
        }
        Ok(out)
    })
}

/// `PING` de teste de conexão.
pub fn ping(target: &Target, timeout: Duration) -> Result<(), DbError> {
    command(target, &["PING".to_string()], timeout).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(tls: bool) -> Target {
        Target {
            host: "127.0.0.1".into(),
            port: 1,
            user: None,
            password: Some("redis-secret".into()),
            db: 0,
            tls,
        }
    }

    #[test]
    fn empty_command_fails_before_connecting() {
        let err = command(&target(false), &[], Duration::from_secs(1)).unwrap_err();
        assert_eq!(err.kind(), "query_failed");
    }

    #[test]
    fn debug_redacts_password() {
        let t = target(true);
        let dbg = format!("{t:?}");
        assert!(!dbg.contains("redis-secret"), "{dbg}");
        assert!(dbg.contains("[REDACTED]"), "{dbg}");
    }

    #[test]
    fn unreachable_server_is_a_connection_failure() {
        let err = ping(&target(false), Duration::from_secs(3)).unwrap_err();
        assert!(matches!(err, DbError::ConnectionFailed(_)), "{err:?}");
        assert_eq!(err.kind(), "connection_failed");
    }
}
