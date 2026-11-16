//! Driver MongoDB (`mongodb` crate com `rustls`, `dns-resolver` para `mongodb+srv://` e
//! `socks5-proxy` para túnel SSH).
//!
//! Documentos trafegam em relaxed extended JSON (`{"$oid": …}`, `{"$date": …}`), mantendo
//! round-trip sem perda de tipos BSON.

use std::fmt;
use std::time::Duration;

use mongodb::bson::{Bson, Document};
use mongodb::options::{ClientOptions, Socks5Proxy};
use mongodb::Client;
use serde_json::{Map, Value};
use zeroize::Zeroize;

use crate::connection::{redact_url, Connection};
use crate::error::DbError;
use crate::url;

/// Destino de uma conexão MongoDB, já com URI/senha resolvidas.
#[derive(Clone, PartialEq, Eq)]
pub struct Target {
    pub uri: String,
    pub database: String,
    pub socks_proxy: Option<(String, u16)>,
}

impl Drop for Target {
    fn drop(&mut self) {
        self.uri.zeroize();
    }
}

impl fmt::Debug for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Target")
            .field("uri", &redact_url(&self.uri))
            .field("database", &self.database)
            .field("socks_proxy", &self.socks_proxy)
            .finish()
    }
}

/// URI com a senha do cofre injetada no `userinfo`, se ainda não houver senha na URL.
/// Espelha `NoSqlRunnerImpl._mongoUri`.
pub fn mongo_uri(conn: &Connection, password: Option<&str>) -> String {
    let Some(password) = password.filter(|p| !p.is_empty()) else {
        return conn.url.clone();
    };
    let Ok(parsed) = url::parse(&conn.url) else {
        return conn.url.clone();
    };
    if parsed.user_info.contains(':') {
        return conn.url.clone();
    }
    let Some(sep) = conn.url.find("://") else {
        return conn.url.clone();
    };
    let start = sep + 3;
    let tail = conn.url.get(start..).unwrap_or_default();
    let user_enc = url::encode_component(&conn.user());
    let pass_enc = url::encode_component(password);
    let auth = format!("{user_enc}:{pass_enc}@");
    if parsed.user_info.is_empty() {
        format!("{}{auth}{tail}", conn.url.get(..start).unwrap_or_default())
    } else if let Some(at) = tail.find('@') {
        let after_at = tail.get(at + 1..).unwrap_or_default();
        format!(
            "{}{auth}{after_at}",
            conn.url.get(..start).unwrap_or_default()
        )
    } else {
        conn.url.clone()
    }
}

/// Banco padrão do deployment: path da URL, senão `authSource`, senão `admin`.
/// Espelha `NoSqlRunnerImpl._mongoDatabase`.
pub fn default_database(conn: &Connection) -> String {
    let db = conn.database();
    if !db.is_empty() {
        return db;
    }
    conn.query_param("authSource")
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "admin".to_string())
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

fn classify_error(err: &mongodb::error::Error) -> DbError {
    let msg = err.to_string();
    let lower = msg.to_ascii_lowercase();
    if lower.contains("connect")
        || lower.contains("server selection")
        || lower.contains("connection refused")
        || lower.contains("timed out")
    {
        DbError::ConnectionFailed(msg)
    } else {
        DbError::QueryFailed(msg)
    }
}

/// Roda um comando (`{find: "users", filter: {...}}`) via `runCommand` e devolve relaxed extended
/// JSON.
pub fn run_command(
    target: &Target,
    command: &Map<String, Value>,
    database: Option<&str>,
    timeout: Duration,
) -> Result<Value, DbError> {
    let doc = Document::try_from(command.clone())
        .map_err(|e| DbError::QueryFailed(format!("Invalid command BSON: {e}")))?;
    run_document(target, doc, database, timeout)
}

/// Executa um comando Mongo preservando a ordem exata das chaves (exigida pelo wire protocol,
/// pois o nome do comando precisa ser a primeira chave do documento).
pub fn run_ordered_command(
    target: &Target,
    entries: &[(&str, Value)],
    database: Option<&str>,
    timeout: Duration,
) -> Result<Value, DbError> {
    let mut doc = Document::new();
    for (k, v) in entries {
        let bson = Bson::try_from(v.clone())
            .map_err(|e| DbError::QueryFailed(format!("Invalid command BSON: {e}")))?;
        doc.insert((*k).to_string(), bson);
    }
    run_document(target, doc, database, timeout)
}

fn run_document(
    target: &Target,
    doc: Document,
    database: Option<&str>,
    timeout: Duration,
) -> Result<Value, DbError> {
    let db_name = database
        .filter(|s| !s.is_empty())
        .unwrap_or(&target.database)
        .to_string();
    let uri = target.uri.clone();
    let socks = target.socks_proxy.clone();

    block_on(timeout, async move {
        crate::redis::ensure_rustls_provider();
        let mut options = ClientOptions::parse(&uri)
            .await
            .map_err(|e| DbError::ConnectionFailed(e.to_string()))?;
        options.connect_timeout = Some(timeout.min(Duration::from_secs(10)));
        options.server_selection_timeout = Some(timeout.min(Duration::from_secs(10)));
        if let Some((host, port)) = socks {
            options.socks5_proxy = Some(Socks5Proxy::builder().host(host).port(port).build());
        }
        let client = Client::with_options(options).map_err(|e| classify_error(&e))?;
        let reply = client
            .database(&db_name)
            .run_command(doc)
            .await
            .map_err(|e| classify_error(&e));
        client.shutdown().await;
        let doc = reply?;
        Ok(Bson::Document(doc).into_relaxed_extjson())
    })
}

/// `{"ping": 1}` de teste de conexão.
pub fn ping(target: &Target, timeout: Duration) -> Result<(), DbError> {
    let mut cmd = Map::new();
    cmd.insert("ping".into(), Value::from(1));
    let reply = run_command(target, &cmd, None, timeout)?;
    if reply.get("ok").and_then(Value::as_f64) == Some(0.0) {
        let msg = reply
            .get("errmsg")
            .and_then(Value::as_str)
            .unwrap_or("MongoDB ping failed.");
        return Err(DbError::ConnectionFailed(msg.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Origin;
    use serde_json::json;

    fn mongo(url: &str) -> Connection {
        Connection::from_json(&json!({"name": "m", "url": url}), Origin::Registered).unwrap()
    }

    #[test]
    fn injects_vault_password_preserving_query_and_encoding_specials() {
        let c = mongo("mongodb://ana@db.local:27017/app?authSource=admin");
        assert_eq!(
            mongo_uri(&c, Some("s:cr@t")),
            "mongodb://ana:s%3Acr%40t@db.local:27017/app?authSource=admin"
        );
        let with_inline = mongo("mongodb://ana:inline@db.local:27017/app");
        assert_eq!(
            mongo_uri(&with_inline, Some("other")),
            "mongodb://ana:inline@db.local:27017/app"
        );
    }

    #[test]
    fn default_database_falls_back_to_auth_source_then_admin() {
        assert_eq!(default_database(&mongo("mongodb://h/sales")), "sales");
        assert_eq!(
            default_database(&mongo("mongodb+srv://u@cluster.example/?authSource=myauth")),
            "myauth"
        );
        assert_eq!(default_database(&mongo("mongodb://h/")), "admin");
    }

    #[test]
    fn extended_json_round_trip_preserves_oid_and_date() {
        let input: Map<String, Value> = serde_json::from_str(
            r#"{"_id":{"$oid":"507f1f77bcf86cd799439011"},"at":{"$date":"2026-01-02T03:04:05Z"}}"#,
        )
        .unwrap();
        let doc = Document::try_from(input.clone()).unwrap();
        let out = Bson::Document(doc).into_relaxed_extjson();
        assert_eq!(out["_id"]["$oid"], "507f1f77bcf86cd799439011");
        assert!(out["at"]["$date"].is_string());
    }

    #[test]
    fn debug_redacts_uri_password() {
        let t = Target {
            uri: "mongodb://ana:mongo-secret@127.0.0.1:1/app".into(),
            database: "app".into(),
            socks_proxy: None,
        };
        let dbg = format!("{t:?}");
        assert!(!dbg.contains("mongo-secret"), "{dbg}");
        assert!(dbg.contains("ana:***"), "{dbg}");
    }

    #[test]
    fn unreachable_server_is_a_connection_failure() {
        let t = Target {
            uri: "mongodb://127.0.0.1:1/app".into(),
            database: "app".into(),
            socks_proxy: None,
        };
        let err = ping(&t, Duration::from_secs(2)).unwrap_err();
        assert!(
            matches!(err, DbError::ConnectionFailed(_) | DbError::Timeout(_)),
            "{err:?}"
        );
    }
}
