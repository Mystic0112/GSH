//! Cofre de senhas desta máquina: `~/.gsh/db-secrets.json` (`DbSecretStore` do `gsh_core`).
//!
//! É o mesmo arquivo que o Flutter e o `gsh-server` leem, então o formato é idêntico: envelope
//! `{"v":1,"n":<nonce>,"d":<dados>}` em base64, AES-256-GCM com nonce de 96 bits, tag de 128 bits,
//! sem dado associado, e chave fixa do produto (`SHA-256("gsh.db-secrets.v1")`).
//!
//! A cifra é ofuscação, não blindagem: a chave está no binário. Compra não deixar a senha em
//! texto claro no disco (backup, `grep`, pasta sincronizada), e nada além disso — quem executa
//! como o dono da conta também a obtém. O arquivo fica em `0600` e o diretório em `0700`.
//! O cofre em claro (formato anterior à cifra) ainda é lido; a próxima escrita o cifra.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::StoreError;

const KEY_SEED: &str = "gsh.db-secrets.v1";
const NONCE_BYTES: usize = 12;

#[derive(Serialize)]
struct Envelope {
    v: u8,
    n: String,
    d: String,
}

/// Cofre de segredos de conexão.
#[derive(Debug, Clone)]
pub struct SecretStore {
    path: PathBuf,
}

impl SecretStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// `~/.gsh/db-secrets.json`.
    pub fn default_path() -> PathBuf {
        let home = std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map_or_else(|| PathBuf::from("."), PathBuf::from);
        home.join(".gsh").join("db-secrets.json")
    }

    /// Cofre no caminho padrão.
    pub fn system() -> Self {
        Self::new(Self::default_path())
    }

    /// Chave de uma senha: a raiz do workspace entra porque dois workspaces podem ter conexões
    /// homônimas. O separador é o NUL, como no Flutter; cofre e cliente precisam concordar byte
    /// a byte.
    pub fn key_for(workspace_root: &str, conn_name: &str) -> String {
        format!("{workspace_root}\u{0}{conn_name}")
    }

    /// Senha guardada de `conn_name`.
    pub fn read(&self, workspace_root: &str, conn_name: &str) -> Option<String> {
        self.load()
            .remove(&Self::key_for(workspace_root, conn_name))
    }

    /// Guarda a senha de `conn_name`.
    pub fn write(
        &self,
        workspace_root: &str,
        conn_name: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        let mut data = self.load();
        data.insert(Self::key_for(workspace_root, conn_name), value.to_string());
        self.persist(&data)
    }

    /// Apaga a senha de `conn_name` (sem escrita quando não havia).
    pub fn delete(&self, workspace_root: &str, conn_name: &str) -> Result<(), StoreError> {
        let mut data = self.load();
        if data
            .remove(&Self::key_for(workspace_root, conn_name))
            .is_none()
        {
            return Ok(());
        }
        self.persist(&data)
    }

    /// Move a senha de `from` para `to` numa escrita só (renomear a conexão não perde a senha).
    pub fn rename(&self, workspace_root: &str, from: &str, to: &str) -> Result<(), StoreError> {
        let from_key = Self::key_for(workspace_root, from);
        let to_key = Self::key_for(workspace_root, to);
        if from_key == to_key {
            return Ok(());
        }
        let mut data = self.load();
        let Some(value) = data.remove(&from_key) else {
            return Ok(());
        };
        data.insert(to_key, value);
        self.persist(&data)
    }

    /// Arquivo ausente, corrompido ou de outra chave é um cofre vazio; a próxima escrita o refaz.
    fn load(&self) -> BTreeMap<String, String> {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return BTreeMap::new();
        };
        let Ok(Value::Object(root)) = serde_json::from_str::<Value>(&text) else {
            return BTreeMap::new();
        };
        let encrypted = root.get("v").is_some_and(Value::is_i64)
            && root.get("n").is_some_and(Value::is_string)
            && root.get("d").is_some_and(Value::is_string);
        if !encrypted {
            return entries(&root);
        }
        let Some(plain) = decrypt(&root) else {
            return BTreeMap::new();
        };
        match serde_json::from_str::<Value>(&plain) {
            Ok(Value::Object(inner)) => entries(&inner),
            _ => BTreeMap::new(),
        }
    }

    fn persist(&self, data: &BTreeMap<String, String>) -> Result<(), StoreError> {
        let dir = self.path.parent().map(PathBuf::from).unwrap_or_default();
        fs::create_dir_all(&dir).map_err(|e| StoreError::io("create secrets dir", &e))?;
        restrict(&dir, 0o700);
        let plain = serde_json::to_string(data).map_err(|_| StoreError::Encrypt)?;
        let envelope = encrypt(&plain)?;
        let mut text = serde_json::to_string(&envelope).map_err(|_| StoreError::Encrypt)?;
        text.push('\n');
        // arquivo vazio e restrito antes do conteúdo: não existe janela com o segredo em `0644`
        if !self.path.exists() {
            fs::File::create(&self.path).map_err(|e| StoreError::io("create secrets file", &e))?;
        }
        restrict(&self.path, 0o600);
        fs::write(&self.path, text).map_err(|e| StoreError::io("write secrets file", &e))
    }
}

fn entries(map: &serde_json::Map<String, Value>) -> BTreeMap<String, String> {
    map.iter()
        .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_string())))
        .collect()
}

fn cipher() -> Aes256Gcm {
    let key = Sha256::digest(KEY_SEED.as_bytes());
    Aes256Gcm::new(&key)
}

fn encrypt(plain: &str) -> Result<Envelope, StoreError> {
    // nonce novo a cada escrita: o arquivo é regravado inteiro, então (chave, nonce) nunca repete
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let data = cipher()
        .encrypt(&nonce, plain.as_bytes())
        .map_err(|_| StoreError::Encrypt)?;
    Ok(Envelope {
        v: 1,
        n: STANDARD.encode(nonce),
        d: STANDARD.encode(data),
    })
}

fn decrypt(envelope: &serde_json::Map<String, Value>) -> Option<String> {
    let nonce = STANDARD.decode(envelope.get("n")?.as_str()?).ok()?;
    let data = STANDARD.decode(envelope.get("d")?.as_str()?).ok()?;
    if nonce.len() != NONCE_BYTES {
        return None;
    }
    let plain = cipher()
        .decrypt(Nonce::from_slice(&nonce), data.as_slice())
        .ok()?;
    String::from_utf8(plain).ok()
}

/// `0600`/`0700` no Unix. Fora dele não há modo POSIX e a ACL do diretório do usuário vale.
#[cfg(unix)]
fn restrict(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    // defesa em profundidade: não poder apertar a permissão não é motivo para perder o segredo
    let _ = fs::set_permissions(path, fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn restrict(_path: &std::path::Path, _mode: u32) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (tempfile::TempDir, SecretStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = SecretStore::new(dir.path().join(".gsh/db-secrets.json"));
        (dir, store)
    }

    #[test]
    fn write_read_delete_round_trip() {
        let (_dir, store) = store();
        assert_eq!(store.read("/w", "pg"), None);
        store.write("/w", "pg", "s3nh@ çom acento").unwrap();
        store.write("/w", "other", "x").unwrap();
        assert_eq!(store.read("/w", "pg").as_deref(), Some("s3nh@ çom acento"));
        // mesma conexão em outro workspace é outra entrada
        assert_eq!(store.read("/outro", "pg"), None);
        store.delete("/w", "pg").unwrap();
        assert_eq!(store.read("/w", "pg"), None);
        assert_eq!(store.read("/w", "other").as_deref(), Some("x"));
    }

    #[test]
    fn rename_moves_the_secret() {
        let (_dir, store) = store();
        store.write("/w", "old", "pw").unwrap();
        store.rename("/w", "old", "new").unwrap();
        assert_eq!(store.read("/w", "old"), None);
        assert_eq!(store.read("/w", "new").as_deref(), Some("pw"));
    }

    #[test]
    fn the_file_is_the_flutter_envelope_and_never_plain_text() {
        let (_dir, store) = store();
        store.write("/w", "pg", "hunter2").unwrap();
        let text = fs::read_to_string(&store.path).unwrap();
        assert!(!text.contains("hunter2") && !text.contains("/w"));
        let envelope: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(envelope["v"], 1);
        assert!(text.starts_with("{\"v\":1,\"n\":\""), "{text}");
        let nonce = STANDARD.decode(envelope["n"].as_str().unwrap()).unwrap();
        assert_eq!(nonce.len(), 12);
    }

    #[test]
    fn nonce_is_fresh_on_every_write() {
        let (_dir, store) = store();
        store.write("/w", "pg", "a").unwrap();
        let first: Value = serde_json::from_str(&fs::read_to_string(&store.path).unwrap()).unwrap();
        store.write("/w", "pg", "a").unwrap();
        let second: Value =
            serde_json::from_str(&fs::read_to_string(&store.path).unwrap()).unwrap();
        assert_ne!(first["n"], second["n"]);
    }

    /// Envelope produzido pelo `DbSecretStore` do Dart (pointycastle) com `{"/work pg":"hunter2"}`:
    /// prova a compatibilidade byte a byte da cifra.
    #[test]
    fn reads_an_envelope_written_by_the_dart_implementation() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path.parent().unwrap()).unwrap();
        fs::write(&store.path, DART_ENVELOPE).unwrap();
        assert_eq!(store.read("/work", "pg").as_deref(), Some("hunter2"));
    }

    const DART_ENVELOPE: &str = include_str!("../tests/dart-db-secrets.json");

    #[test]
    fn legacy_plain_vault_is_read_and_encrypted_on_next_write() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path.parent().unwrap()).unwrap();
        fs::write(&store.path, "{\"/w\\u0000pg\":\"old\"}").unwrap();
        assert_eq!(store.read("/w", "pg").as_deref(), Some("old"));
        store.write("/w", "new", "n").unwrap();
        let text = fs::read_to_string(&store.path).unwrap();
        assert!(!text.contains("old"));
        assert_eq!(store.read("/w", "pg").as_deref(), Some("old"));
    }

    #[test]
    fn a_corrupted_file_is_an_empty_vault() {
        let (_dir, store) = store();
        fs::create_dir_all(store.path.parent().unwrap()).unwrap();
        fs::write(&store.path, "{\"v\":1,\"n\":\"AAAA\",\"d\":\"AAAA\"}").unwrap();
        assert_eq!(store.read("/w", "pg"), None);
        store.write("/w", "pg", "again").unwrap();
        assert_eq!(store.read("/w", "pg").as_deref(), Some("again"));
    }

    #[cfg(unix)]
    #[test]
    fn permissions_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let (_dir, store) = store();
        store.write("/w", "pg", "x").unwrap();
        let file = fs::metadata(&store.path).unwrap().permissions().mode() & 0o777;
        let dir = fs::metadata(store.path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!((file, dir), (0o600, 0o700));
    }
}
