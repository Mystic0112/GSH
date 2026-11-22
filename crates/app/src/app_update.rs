//! Verificação e aviso de atualização do GSH Desktop (`update_viewmodel.dart`,
//! `update_info.dart`, `semver.dart` e `update_checker_impl.dart` do Flutter).
//!
//! No Linux, opera no modo `notify`: busca `latest.json` (com override por
//! `GSH_UPDATE_MANIFEST_URL` para testes offline), compara `semver` com a versão
//! em execução e, se houver versão mais nova não dispensada, exibe o mini card
//! no rodapé do rail lateral. Clicar no card abre o artefato da plataforma no
//! navegador; clicar no `×` dispensa e persiste a versão dispensada.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gsh_config::schema::UpdateCheckFrequency;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_MANIFEST_URL: &str = "https://rp-s3.jacobmoura.work/downloads/gsh/latest.json";
pub const FALLBACK_DOWNLOAD_URL: &str = "https://remote-pi.jacobmoura.work/download";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateArtifact {
    pub platform: String,
    pub arch: String,
    pub format: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub version: String,
    pub date: String,
    pub notes: String,
    pub artifacts: Vec<UpdateArtifact>,
}

impl UpdateInfo {
    pub fn parse_manifest(source: &str) -> Option<Self> {
        let root: Value = serde_json::from_str(source).ok()?;
        Self::from_json(&root)
    }

    pub fn from_json(value: &Value) -> Option<Self> {
        let obj = value.as_object()?;
        let version = obj
            .get("version")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())?
            .to_owned();
        let raw_artifacts = obj.get("artifacts").and_then(Value::as_array)?;
        let mut artifacts = Vec::with_capacity(raw_artifacts.len());
        for raw in raw_artifacts {
            let item = raw.as_object()?;
            let url = item.get("url").and_then(Value::as_str)?.to_owned();
            let platform = item.get("platform").and_then(Value::as_str)?.to_owned();
            let format = item.get("format").and_then(Value::as_str)?.to_owned();
            let arch = item
                .get("arch")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let sha256 = item
                .get("sha256")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let size = item.get("size").and_then(Value::as_u64).unwrap_or(0);
            artifacts.push(UpdateArtifact {
                platform,
                arch,
                format,
                url,
                sha256,
                size,
            });
        }
        let date = obj
            .get("date")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let notes = obj
            .get("notes")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        Some(Self {
            version,
            date,
            notes,
            artifacts,
        })
    }

    pub fn artifact_for(
        &self,
        platform: &str,
        format: &str,
        arch: &str,
    ) -> Option<&UpdateArtifact> {
        let mut first_match = None;
        for artifact in &self.artifacts {
            if artifact.platform == platform && artifact.format == format {
                if artifact.arch == arch {
                    return Some(artifact);
                }
                if first_match.is_none() {
                    first_match = Some(artifact);
                }
            }
        }
        first_match
    }

    pub fn download_url_for_current(&self) -> &str {
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x64"
        };
        let platform = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };
        let primary_format = if cfg!(target_os = "macos") {
            "dmg"
        } else if cfg!(target_os = "windows") {
            "exe"
        } else {
            "deb"
        };
        self.artifact_for(platform, primary_format, arch)
            .or_else(|| self.artifact_for(platform, "tar.gz", arch))
            .map(|a| a.url.as_str())
            .unwrap_or(FALLBACK_DOWNLOAD_URL)
    }
}

fn parse_semver(version: &str) -> [u64; 3] {
    let core = version.trim().split(['-', '+']).next().unwrap_or_default();
    let mut parts = core.split('.');
    let mut next_num = || {
        parts
            .next()
            .and_then(|p| p.trim().parse::<u64>().ok())
            .unwrap_or(0)
    };
    [next_num(), next_num(), next_num()]
}

pub fn compare_semver(a: &str, b: &str) -> Ordering {
    parse_semver(a).cmp(&parse_semver(b))
}

pub fn is_newer_version(candidate: &str, current: &str) -> bool {
    compare_semver(candidate, current) == Ordering::Greater
}

pub fn should_check(
    freq: UpdateCheckFrequency,
    last_check_secs: Option<u64>,
    now_secs: u64,
    force: bool,
) -> bool {
    if force {
        return true;
    }
    if freq == UpdateCheckFrequency::Never {
        return false;
    }
    let Some(last) = last_check_secs else {
        return true;
    };
    let diff_days = now_secs.saturating_sub(last) / 86_400;
    match freq {
        UpdateCheckFrequency::Daily => diff_days >= 1,
        UpdateCheckFrequency::Weekly => diff_days >= 7,
        UpdateCheckFrequency::Monthly => diff_days >= 30,
        UpdateCheckFrequency::Never => false,
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateStore {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dismissed_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_check_secs: Option<u64>,
}

impl UpdateStore {
    pub fn read(path: &Path) -> Self {
        fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    pub fn write_atomic(&self, path: &Path) {
        let Some(parent) = path.parent() else {
            return;
        };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let Ok(bytes) = serde_json::to_vec(self) else {
            return;
        };
        let temp = path.with_extension("json.tmp");
        if fs::write(&temp, bytes).is_ok() {
            let _ = fs::rename(temp, path);
        }
    }

    pub fn default_path() -> Option<PathBuf> {
        std::env::var_os("GSH_UPDATE_STATE_PATH")
            .map(PathBuf::from)
            .or_else(|| {
                dirs::data_dir().map(|d| d.join("work.gorila.gshdesktop").join("update_state.json"))
            })
    }
}

#[derive(Debug, Default, Clone)]
pub struct UpdateState {
    pub available: Option<UpdateInfo>,
    pub store: UpdateStore,
    pub store_path: Option<PathBuf>,
    check_in_flight: Option<bool>,
}

impl UpdateState {
    pub fn load_default() -> Self {
        let store_path = UpdateStore::default_path();
        let store = store_path
            .as_deref()
            .map(UpdateStore::read)
            .unwrap_or_default();
        Self {
            store,
            store_path,
            ..Self::default()
        }
    }

    pub fn in_memory() -> Self {
        Self::default()
    }

    pub fn available_version(&self) -> Option<&str> {
        self.available.as_ref().map(|info| info.version.as_str())
    }

    pub fn download_url(&self) -> Option<String> {
        self.available
            .as_ref()
            .map(|info| info.download_url_for_current().to_owned())
    }

    pub fn begin_check(&mut self, freq: UpdateCheckFrequency, force: bool) -> bool {
        if let Some(pending_force) = &mut self.check_in_flight {
            *pending_force |= force;
            return false;
        }
        if !should_check(freq, self.store.last_check_secs, now_epoch_secs(), force) {
            return false;
        }
        self.check_in_flight = Some(force);
        true
    }

    pub fn finish_check(&mut self, current_version: &str, fetched: Option<UpdateInfo>) {
        if let Some(force) = self.check_in_flight.take() {
            self.apply_fetched(current_version, fetched, force);
        }
    }

    fn apply_fetched(&mut self, current_version: &str, fetched: Option<UpdateInfo>, force: bool) {
        if force {
            self.store.dismissed_version = None;
        }
        self.store.last_check_secs = Some(now_epoch_secs());
        self.persist();
        let Some(info) = fetched else {
            return;
        };
        if !is_newer_version(&info.version, current_version) {
            self.available = None;
            return;
        }
        if self.store.dismissed_version.as_deref() == Some(info.version.as_str()) {
            return;
        }
        self.available = Some(info);
    }

    pub fn dismiss(&mut self) {
        if let Some(info) = self.available.take() {
            self.store.dismissed_version = Some(info.version);
            self.persist();
        }
    }

    fn persist(&self) {
        if let Some(path) = self.store_path.as_deref() {
            self.store.write_atomic(path);
        }
    }
}

pub async fn fetch_latest() -> Option<UpdateInfo> {
    let target = std::env::var("GSH_UPDATE_MANIFEST_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_MANIFEST_URL.to_owned());
    if let Some(local) = target.strip_prefix("file://") {
        return fs::read_to_string(local)
            .ok()
            .and_then(|s| UpdateInfo::parse_manifest(&s));
    }
    if Path::new(&target).is_file() {
        return fs::read_to_string(&target)
            .ok()
            .and_then(|s| UpdateInfo::parse_manifest(&s));
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .ok()?;
    let response = client.get(&target).send().await.ok()?;
    if !response.status().is_success() {
        return None;
    }
    let body = response.text().await.ok()?;
    UpdateInfo::parse_manifest(&body)
}

fn now_epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_manifest_and_matches_artifact_by_platform_format_and_arch() {
        let json = r#"{
            "version": "1.2.0",
            "date": "2026-06-12",
            "notes": "resumo",
            "artifacts": [
                { "platform": "macos", "arch": "universal", "format": "dmg", "url": "https://x/a.dmg", "sha256": "abc", "size": 123 },
                { "platform": "linux", "arch": "x64", "format": "deb", "url": "https://x/x64.deb" },
                { "platform": "linux", "arch": "arm64", "format": "deb", "url": "https://x/arm.deb", "sha256": "def", "size": 456 }
            ]
        }"#;
        let info = UpdateInfo::parse_manifest(json).unwrap();
        assert_eq!(info.version, "1.2.0");
        assert_eq!(info.date, "2026-06-12");
        assert_eq!(info.notes, "resumo");
        assert_eq!(
            info.artifact_for("linux", "deb", "arm64")
                .map(|a| a.url.as_str()),
            Some("https://x/arm.deb")
        );
        assert_eq!(
            info.artifact_for("linux", "deb", "x64")
                .map(|a| a.url.as_str()),
            Some("https://x/x64.deb")
        );
        assert_eq!(
            info.artifact_for("macos", "dmg", "universal")
                .map(|a| a.url.as_str()),
            Some("https://x/a.dmg")
        );
        assert!(info.artifact_for("windows", "exe", "x64").is_none());

        for invalid in [
            "[]",
            r#"{"artifacts":[]}"#,
            r#"{"version":"","artifacts":[]}"#,
            r#"{"version":"1.0.0"}"#,
            r#"{"version":"1.0.0","artifacts":[{"platform":"macos"}]}"#,
        ] {
            assert!(UpdateInfo::parse_manifest(invalid).is_none(), "{invalid}");
        }
    }

    #[test]
    fn semver_and_schedule_rules_match_flutter() {
        assert!(is_newer_version("1.2.0", "0.1.0"));
        assert!(is_newer_version("1.2.1-beta.1", "1.2.0+99"));
        assert!(!is_newer_version("1.2.0", "1.2"));
        assert!(!is_newer_version("0.1.0", "0.2.0"));

        assert!(!should_check(
            UpdateCheckFrequency::Never,
            None,
            1_000,
            false
        ));
        assert!(should_check(UpdateCheckFrequency::Never, None, 1_000, true));
        for (frequency, days) in [
            (UpdateCheckFrequency::Daily, 1),
            (UpdateCheckFrequency::Weekly, 7),
            (UpdateCheckFrequency::Monthly, 30),
        ] {
            let due = 1_000 + days * 86_400;
            assert!(should_check(frequency, None, 1_000, false));
            assert!(!should_check(frequency, Some(1_000), due - 1, false));
            assert!(should_check(frequency, Some(1_000), due, false));
        }
    }

    #[test]
    fn dismiss_persists_version_and_force_check_resurfaces_it() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("update_state.json");
        let mut state = UpdateState {
            store_path: Some(store_path.clone()),
            ..UpdateState::default()
        };
        let info = UpdateInfo::parse_manifest(r#"{"version":"1.9.0","artifacts":[]}"#).unwrap();
        state.apply_fetched("0.1.0", Some(info.clone()), false);
        assert_eq!(state.available_version(), Some("1.9.0"));

        state.dismiss();
        assert_eq!(state.available_version(), None);
        assert_eq!(
            UpdateStore::read(&store_path).dismissed_version.as_deref(),
            Some("1.9.0")
        );

        // Background check respects dismissed version
        state.apply_fetched("0.1.0", Some(info.clone()), false);
        assert_eq!(state.available_version(), None);

        // User-initiated check (force = true) clears dismissal and resurfaces the card
        state.apply_fetched("0.1.0", Some(info), true);
        assert_eq!(state.available_version(), Some("1.9.0"));
    }
}
