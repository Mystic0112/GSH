//! Leitura/classificação de chaves privadas SSH (`ssh_key_pem.dart`) e store TOFU de host keys
//! (`json_ssh_host_key_store.dart` / `file_ssh_host_key_store.dart`, em
//! `~/.gsh/ssh-known-hosts.json`).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use russh::keys::{decode_secret_key, HashAlg, PrivateKey, PublicKey};
use serde_json::Value;
use zeroize::Zeroizing;

use crate::error::{DbError, StoreError};

/// Expande `~` usando `HOME` (ou `USERPROFILE` / `HOMEDRIVE`+`HOMEPATH` no Windows).
pub fn expand_tilde(path: &str) -> String {
    if !path.starts_with('~') {
        return path.to_string();
    }
    let Some(home) = home_dir() else {
        return path.to_string();
    };
    let home = home.to_string_lossy();
    if path == "~" {
        home.into_owned()
    } else if let Some(rest) = path.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else if let Some(rest) = path.strip_prefix("~\\") {
        format!("{home}\\{rest}")
    } else {
        path.to_string()
    }
}

/// Encurta `$HOME/...` para `~/...` ao guardar no `databases.json`.
pub fn shorten_home(path: &std::path::Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    if let Some(home) = home_dir() {
        let home_str = home.to_string_lossy().replace('\\', "/");
        let home_trimmed = home_str.trim_end_matches('/');
        if let Some(rest) = raw.strip_prefix(&format!("{home_trimmed}/")) {
            return format!("~/{rest}");
        }
    }
    raw
}

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(profile) = std::env::var_os("USERPROFILE").filter(|v| !v.is_empty()) {
            return Some(PathBuf::from(profile));
        }
        let drive = std::env::var_os("HOMEDRIVE").unwrap_or_default();
        let path = std::env::var_os("HOMEPATH").unwrap_or_default();
        let joined = format!("{}{}", drive.to_string_lossy(), path.to_string_lossy());
        if !joined.is_empty() {
            return Some(PathBuf::from(joined));
        }
        None
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    }
}

/// Sugestão de chave padrão para o diálogo (`~/.ssh/id_ed25519` depois `~/.ssh/id_rsa`), mantendo
/// o `~` literal.
pub fn default_key_path() -> Option<&'static str> {
    ["~/.ssh/id_ed25519", "~/.ssh/id_rsa"]
        .into_iter()
        .find(|&candidate| std::path::Path::new(&expand_tilde(candidate)).exists())
}

/// `true` se o arquivo de chave privada existe e exige passphrase. Arquivo ausente/ilegível
/// devolve `false` (o erro acionável aparece na conexão).
pub fn needs_passphrase(key_path: &str) -> bool {
    let expanded = expand_tilde(key_path);
    let Ok(pem) = fs::read_to_string(expanded).map(Zeroizing::new) else {
        return false;
    };
    is_encrypted_pem(&pem)
}

pub fn is_encrypted_pem(pem: &str) -> bool {
    if pem.contains("ENCRYPTED") {
        return true;
    }
    PrivateKey::from_openssh(pem.as_bytes()).is_ok_and(|key| key.is_encrypted())
}

/// Lê e decodifica a chave privada SSH, com erro tipado (`ssh_key_missing` ou
/// `ssh_credential_required`).
pub fn load_private_key(key_path: &str, passphrase: Option<&str>) -> Result<PrivateKey, DbError> {
    let expanded = expand_tilde(key_path);
    let path = std::path::Path::new(&expanded);
    if !path.exists() {
        return Err(DbError::SshKeyMissing(format!(
            "SSH key not found: {expanded}"
        )));
    }
    let pem = fs::read_to_string(path)
        .map(Zeroizing::new)
        .map_err(|e| DbError::SshKeyMissing(format!("Cannot read SSH key {expanded}: {e}")))?;

    let encrypted = is_encrypted_pem(&pem);
    let pass = passphrase.filter(|p| !p.is_empty());
    if encrypted && pass.is_none() {
        return Err(DbError::SshCredentialRequired(
            "This SSH key is passphrase-protected and no passphrase is available.".into(),
        ));
    }

    decode_secret_key(&pem, pass).map_err(|err| {
        if encrypted {
            DbError::SshCredentialRequired(
                "Could not decrypt the SSH key — wrong passphrase?".into(),
            )
        } else {
            DbError::SshKeyMissing(format!("Invalid SSH private key: {err}"))
        }
    })
}

/// Fingerprint OpenSSH SHA-256 (`SHA256:<base64 sem '='>`), idêntico ao `dartssh2`.
pub fn host_key_fingerprint(key: &PublicKey) -> String {
    key.fingerprint(HashAlg::Sha256).to_string()
}

/// Host keys confiadas (TOFU) em `~/.gsh/ssh-known-hosts.json`.
#[derive(Debug, Clone)]
pub struct SshHostKeyStore {
    path: PathBuf,
}

impl SshHostKeyStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn default_path() -> PathBuf {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".gsh")
            .join("ssh-known-hosts.json")
    }

    pub fn system() -> Self {
        Self::new(Self::default_path())
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
            fs::create_dir_all(parent).map_err(|e| StoreError::io("create ssh hosts dir", &e))?;
        }
        let mut text = serde_json::to_string_pretty(data).map_err(|_| StoreError::Encrypt)?;
        text.push('\n');
        fs::write(&self.path, text).map_err(|e| StoreError::io("write ssh hosts store", &e))
    }

    pub fn trusted(&self, endpoint: &str) -> Option<String> {
        self.load().get(endpoint).cloned()
    }

    pub fn trust(&self, endpoint: &str, fingerprint: &str) -> Result<(), StoreError> {
        let mut data = self.load();
        data.insert(endpoint.to_string(), fingerprint.to_string());
        self.save(&data)
    }

    pub fn forget(&self, endpoint: &str) -> Result<(), StoreError> {
        let mut data = self.load();
        if data.remove(endpoint).is_none() {
            return Ok(());
        }
        self.save(&data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_key_fingerprint_matches_openssh_sha256() {
        let pubkey = PublicKey::from_openssh(
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIOMqqnkVzrm0SdG6UOoqKLsabgH5C9okWi0dh2l9GKJl",
        )
        .unwrap();
        assert_eq!(
            host_key_fingerprint(&pubkey),
            "SHA256:+DiY3wvvV6TuJJhbpZisF/zLDA0zPMSvHdkr4UvCOqU"
        );
    }

    #[test]
    fn ssh_host_key_store_round_trips_and_forgets() {
        let dir = tempfile::tempdir().unwrap();
        let store = SshHostKeyStore::new(dir.path().join("known.json"));
        assert_eq!(store.trusted("u@h:22"), None);
        store.trust("u@h:22", "SHA256:abc").unwrap();
        assert_eq!(store.trusted("u@h:22").as_deref(), Some("SHA256:abc"));
        store.forget("u@h:22").unwrap();
        assert_eq!(store.trusted("u@h:22"), None);
    }

    #[test]
    fn missing_key_returns_ssh_key_missing() {
        let err = load_private_key("/nonexistent/id_ed25519", None).unwrap_err();
        assert_eq!(err.kind(), "ssh_key_missing");
    }
}
