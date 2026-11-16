//! Banco de dados do GSH, sem UI: conexões do workspace (`.gsh/databases*.json`), cofre de
//! senhas, drivers SQLite e PostgreSQL, introspecção de schema e execução com limites.
//!
//! MySQL, SQL Server, Redis, MongoDB e túnel SSH não têm driver ainda: as conexões desses motores
//! são lidas, mantidas ao regravar o arquivo e recusadas com [`DbError::UnsupportedEngine`].

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod connection;
pub mod dbq;
pub mod driver;
pub mod error;
pub mod postgres;
pub mod result;
pub mod schema;
pub mod secrets;
pub mod sqlite;
pub mod statements;
pub mod store;
pub mod url;

pub use connection::{Access, Connection, Engine, Origin, SshTunnel};
pub use dbq::DbqDocument;
pub use driver::{Endpoint, DEFAULT_LIMIT};
pub use error::{DbError, StoreError};
pub use result::{Cell, Column, QueryResult};
pub use schema::{SchemaColumn, SchemaTable};
pub use secrets::SecretStore;
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
