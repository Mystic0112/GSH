//! Banco de dados do GSH, sem UI: conexões do workspace (`.gsh/databases*.json`), cofre de
//! senhas, drivers SQLite, PostgreSQL (com TLS), MySQL, SQL Server, Redis, MongoDB e túnel SSH.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod access;
pub mod connection;
pub mod dbq;
pub mod driver;
pub mod error;
pub mod mongo;
pub mod mongo_browse;
pub mod mongo_store;
pub mod mssql;
pub mod mysql;
pub mod postgres;
pub mod redis;
pub mod redis_browse;
pub mod result;
pub mod schema;
pub mod secrets;
pub mod sqlite;
pub mod ssh;
pub mod ssh_keys;
pub mod ssh_socks;
pub mod statements;
pub mod store;
pub mod url;

pub use connection::{redact_url, Access, Connection, Engine, Origin, SshTunnel};
pub use dbq::DbqDocument;
pub use driver::{Endpoint, DEFAULT_LIMIT};
pub use error::{DbError, StoreError};
pub use mongo_store::MongoDatabaseStore;
pub use result::{Cell, Column, QueryResult};
pub use schema::{SchemaColumn, SchemaTable};
pub use secrets::SecretStore;
pub use ssh::{SshTunnelManager, TunnelEndpoint};
pub use ssh_keys::SshHostKeyStore;
pub use statements::SqlStatement;

use std::path::Path;

/// Senha da conexão, na ordem do Flutter: cofre (se `save_password`) e depois a embutida na URL.
/// SQLite não tem senha.
pub fn resolve_password(conn: &Connection, root: &Path, secrets: &SecretStore) -> Option<String> {
    if conn.engine == Engine::Sqlite {
        return None;
    }
    if conn.save_password {
        if let Some(saved) = secrets.read(&root.to_string_lossy(), &conn.name) {
            return Some(saved);
        }
    }
    conn.url_password()
}

/// Passphrase da chave SSH, na ordem do Flutter: override digitado -> cofre -> cache de sessão.
/// Devolve `Ok(None)` quando a chave não exige passphrase, ou `Err(DbError::SshCredentialRequired)`
/// quando exige e nenhuma está disponível.
pub fn resolve_ssh_passphrase(
    conn: &Connection,
    root: &Path,
    secrets: &SecretStore,
    override_pass: Option<&str>,
    cached: Option<&str>,
) -> Result<Option<String>, DbError> {
    let Some(ssh) = conn
        .ssh
        .as_ref()
        .filter(|_| conn.engine.supports_ssh_tunnel())
    else {
        return Ok(None);
    };
    if !ssh_keys::needs_passphrase(&ssh.key_path) {
        return Ok(None);
    }
    if let Some(typed) = override_pass.filter(|p| !p.is_empty()) {
        return Ok(Some(typed.to_string()));
    }
    if ssh.save_passphrase {
        let key = SecretStore::ssh_key_for(&root.to_string_lossy(), &conn.name);
        if let Some(saved) = secrets.read_raw(&key).filter(|p| !p.is_empty()) {
            return Ok(Some(saved));
        }
    }
    if let Some(session) = cached.filter(|p| !p.is_empty()) {
        return Ok(Some(session.to_string()));
    }
    Err(DbError::SshCredentialRequired(format!(
        "SSH passphrase required for \"{}\".",
        conn.name
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn postgres(name: &str, url: &str, save_password: bool) -> Connection {
        let mut conn = Connection::from_json(
            &serde_json::json!({"name": name, "url": url}),
            Origin::Registered,
        )
        .unwrap();
        conn.save_password = save_password;
        conn
    }

    #[test]
    fn password_comes_from_the_vault_then_the_url() {
        let dir = tempfile::tempdir().unwrap();
        let secrets = SecretStore::new(dir.path().join("s.json"));
        let root = Path::new("/w");
        let with_url = postgres("pg", "postgres://u:fromurl@h/db", true);
        assert_eq!(
            resolve_password(&with_url, root, &secrets).as_deref(),
            Some("fromurl")
        );
        secrets.write("/w", "pg", "fromvault").unwrap();
        assert_eq!(
            resolve_password(&with_url, root, &secrets).as_deref(),
            Some("fromvault")
        );
        // sem "Save Password" o cofre é ignorado
        let unsaved = postgres("pg", "postgres://u:fromurl@h/db", false);
        assert_eq!(
            resolve_password(&unsaved, root, &secrets).as_deref(),
            Some("fromurl")
        );
        let sqlite = Connection::sqlite("s", "a.db", Origin::Registered, Access::Read, true);
        assert_eq!(resolve_password(&sqlite, root, &secrets), None);
    }
}
