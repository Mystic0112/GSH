use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::model::TelemetryStoreError;
use crate::store::TelemetryStore;

pub type TelemetryStoreOpener =
    Arc<dyn Fn(&str) -> Result<TelemetryStore, TelemetryStoreError> + Send + Sync>;

/// Um `TelemetryStore` por workspace (cacheado). Mesmo diretório de cache que o
/// app Flutter (`<applicationSupport>/telemetry/<safeId>.sqlite`).
pub struct TelemetryStoreRegistry {
    opener: TelemetryStoreOpener,
    stores: Mutex<HashMap<String, TelemetryStore>>,
}

impl TelemetryStoreRegistry {
    pub fn new() -> Self {
        Self::with_opener(Arc::new(open_on_disk))
    }

    pub fn with_directory(dir: impl Into<PathBuf>) -> Self {
        let dir = dir.into();
        Self::with_opener(Arc::new(move |ws_id| open_in_dir(&dir, ws_id)))
    }

    pub fn with_opener(opener: TelemetryStoreOpener) -> Self {
        Self {
            opener,
            stores: Mutex::new(HashMap::new()),
        }
    }

    pub fn memory() -> Self {
        Self::with_opener(Arc::new(|_| TelemetryStore::open_memory()))
    }

    /// Diretório das bases: `<applicationSupport>/telemetry/`, igual ao Flutter.
    pub fn directory() -> Result<PathBuf, TelemetryStoreError> {
        #[cfg(target_os = "linux")]
        let base = dirs::data_local_dir().map(|d| d.join("work.gorila.gshdesktop"));
        #[cfg(target_os = "macos")]
        let base = dirs::data_dir().map(|d| d.join("work.gorila.gshdesktop"));
        #[cfg(target_os = "windows")]
        let base = dirs::data_dir().map(|d| d.join("Gorila Software House").join("GSH Desktop"));
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        let base = dirs::data_local_dir().map(|d| d.join("work.gorila.gshdesktop"));

        let base = base.ok_or_else(|| {
            TelemetryStoreError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "cannot resolve application support directory",
            ))
        })?;
        Ok(base.join("telemetry"))
    }

    pub fn db_path_in(dir: &Path, workspace_id: &str) -> PathBuf {
        let safe: String = workspace_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        dir.join(format!("{safe}.sqlite"))
    }

    pub async fn for_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<TelemetryStore, TelemetryStoreError> {
        let mut guard = self.stores.lock().await;
        if let Some(existing) = guard.get(workspace_id) {
            return Ok(existing.clone());
        }
        let opened = (self.opener)(workspace_id)?;
        guard.insert(workspace_id.to_string(), opened.clone());
        Ok(opened)
    }

    pub async fn open_ids(&self) -> Vec<String> {
        self.stores.lock().await.keys().cloned().collect()
    }

    pub async fn close_all(&self) {
        let drained: Vec<TelemetryStore> = {
            let mut guard = self.stores.lock().await;
            guard.drain().map(|(_, s)| s).collect()
        };
        for store in drained {
            let _ = store.close().await;
        }
    }
}

impl Default for TelemetryStoreRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn open_on_disk(workspace_id: &str) -> Result<TelemetryStore, TelemetryStoreError> {
    let dir = TelemetryStoreRegistry::directory()?;
    open_in_dir(&dir, workspace_id)
}

fn open_in_dir(dir: &Path, workspace_id: &str) -> Result<TelemetryStore, TelemetryStoreError> {
    std::fs::create_dir_all(dir)?;
    let path = TelemetryStoreRegistry::db_path_in(dir, workspace_id);
    TelemetryStore::open(path)
}
