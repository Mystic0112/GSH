//! Persistência da seleção de database por conexão MongoDB (`json_mongo_database_store.dart`),
//! em `~/.gsh/mongo-databases.json`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::error::StoreError;

/// Mapa persistido `<workspaceRoot>::<connName> -> <database>`.
#[derive(Debug, Clone)]
pub struct MongoDatabaseStore {
    path: PathBuf,
}

impl MongoDatabaseStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> PathBuf {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        home.join(".gsh").join("mongo-databases.json")
    }

    pub fn system() -> Self {
        Self::new(Self::default_path())
    }

    /// Chave `<workspace>::<connName>` (`::` evita colisão com `:` no nome da conexão).
    pub fn entry_key(workspace_root: &str, conn_name: &str) -> String {
        format!("{workspace_root}::{conn_name}")
    }

    fn load(&self) -> BTreeMap<String, String> {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return BTreeMap::new();
        };
        let Ok(Value::Object(map)) = serde_json::from_str::<Value>(&text) else {
            return BTreeMap::new();
        };
        map.into_iter()
            .filter_map(|(k, v)| match v {
                Value::String(s) => Some((k, s)),
                _ => None,
            })
            .collect()
    }

    fn save(&self, data: &BTreeMap<String, String>) -> Result<(), StoreError> {
        if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| StoreError::io("create mongo store dir", &e))?;
        }
        let mut text = serde_json::to_string_pretty(data).map_err(|_| StoreError::Encrypt)?;
        text.push('\n');
        fs::write(&self.path, text).map_err(|e| StoreError::io("write mongo store", &e))
    }

    pub fn selected(&self, workspace_root: &str, conn_name: &str) -> Option<String> {
        self.load()
            .get(&Self::entry_key(workspace_root, conn_name))
            .cloned()
    }

    pub fn select(
        &self,
        workspace_root: &str,
        conn_name: &str,
        database: &str,
    ) -> Result<(), StoreError> {
        let mut data = self.load();
        data.insert(
            Self::entry_key(workspace_root, conn_name),
            database.to_string(),
        );
        self.save(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_persists_selected_database() {
        let dir = tempfile::tempdir().unwrap();
        let store = MongoDatabaseStore::new(dir.path().join("mongo.json"));
        assert_eq!(store.selected("/w", "atlas"), None);
        store.select("/w", "atlas", "prod").unwrap();
        assert_eq!(store.selected("/w", "atlas").as_deref(), Some("prod"));
        assert_eq!(store.selected("/w2", "atlas"), None);
    }
}
