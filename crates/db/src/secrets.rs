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
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use aes_gcm::aead::{Aead, AeadCore, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use crate::error::StoreError;

const KEY_SEED: &str = "gsh.db-secrets.v1";
const NONCE_BYTES: usize = 12;
const ENVELOPE_VERSION: i64 = 1;

static TMP_SEQ: AtomicU64 = AtomicU64::new(0);

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

    /// Chave da passphrase da chave SSH (`DbQueryService.sshSecretKey` do Flutter).
    pub fn ssh_key_for(workspace_root: &str, conn_name: &str) -> String {
        Self::key_for("ssh", &format!("{workspace_root}\u{0}{conn_name}"))
    }

    /// Senha guardada de `conn_name`.
    pub fn read(&self, workspace_root: &str, conn_name: &str) -> Option<String> {
        self.read_raw(&Self::key_for(workspace_root, conn_name))
    }

    /// Guarda a senha de `conn_name`.
    pub fn write(
        &self,
        workspace_root: &str,
        conn_name: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        self.write_raw(&Self::key_for(workspace_root, conn_name), value)
    }

    /// Apaga a senha de `conn_name` (sem escrita quando não havia).
    pub fn delete(&self, workspace_root: &str, conn_name: &str) -> Result<(), StoreError> {
        self.delete_raw(&Self::key_for(workspace_root, conn_name))
    }

    /// Move a senha de `from` para `to` numa escrita só (renomear a conexão não perde a senha).
    pub fn rename(&self, workspace_root: &str, from: &str, to: &str) -> Result<(), StoreError> {
        self.rename_raw(
            &Self::key_for(workspace_root, from),
            &Self::key_for(workspace_root, to),
        )
    }

    /// Lê diretamente por chave bruta do cofre (usado também para passphrases SSH).
    pub fn read_raw(&self, key: &str) -> Option<String> {
        let mut data = self.load();
        let value = data.remove(key);
        zeroize_map(&mut data);
        value
    }

    /// Grava diretamente por chave bruta do cofre.
    pub fn write_raw(&self, key: &str, value: &str) -> Result<(), StoreError> {
        let mut data = self.load();
        data.insert(key.to_string(), value.to_string());
        let res = self.persist(&data);
        zeroize_map(&mut data);
        res
    }

    /// Apaga diretamente por chave bruta do cofre.
    pub fn delete_raw(&self, key: &str) -> Result<(), StoreError> {
        let mut data = self.load();
        if data.remove(key).is_none() {
            zeroize_map(&mut data);
            return Ok(());
        }
        let res = self.persist(&data);
        zeroize_map(&mut data);
        res
    }

    /// Move diretamente uma chave bruta do cofre.
    pub fn rename_raw(&self, from_key: &str, to_key: &str) -> Result<(), StoreError> {
        if from_key == to_key {
            return Ok(());
        }
        let mut data = self.load();
        let Some(value) = data.remove(from_key) else {
            zeroize_map(&mut data);
            return Ok(());
        };
        data.insert(to_key.to_string(), value);
        let res = self.persist(&data);
        zeroize_map(&mut data);
        res
    }

    /// Arquivo ausente, corrompido, versão desconhecida ou de outra chave vira cofre vazio; a
    /// próxima escrita o refaz.
    fn load(&self) -> BTreeMap<String, String> {
        let Ok(text) = fs::read_to_string(&self.path).map(Zeroizing::new) else {
            return BTreeMap::new();
        };
        let Ok(Value::Object(root)) = serde_json::from_str::<Value>(&text) else {
            return BTreeMap::new();
        };
        let has_envelope_shape = root.contains_key("v")
            && root.get("n").is_some_and(Value::is_string)
            && root.get("d").is_some_and(Value::is_string);
        if !has_envelope_shape {
            return entries(&root);
        }
        // Rejeita qualquer versão != 1 antes de tentar decifrar.
        if root.get("v").and_then(Value::as_i64) != Some(ENVELOPE_VERSION) {
            return BTreeMap::new();
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
        let dir = match self.path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
            _ => PathBuf::from("."),
        };
        fs::create_dir_all(&dir).map_err(|e| StoreError::io("create secrets dir", &e))?;
        restrict(&dir, 0o700)?;
        let plain = Zeroizing::new(serde_json::to_string(data).map_err(|_| StoreError::Encrypt)?);
        let envelope = encrypt(&plain)?;
        let mut text = serde_json::to_string(&envelope).map_err(|_| StoreError::Encrypt)?;
        text.push('\n');

        let tmp_path = temp_path(&dir);
        let write_result = write_atomic_0600(&tmp_path, &self.path, text.as_bytes());
        if write_result.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        write_result
    }
}

fn zeroize_map(map: &mut BTreeMap<String, String>) {
    for value in map.values_mut() {
        value.zeroize();
    }
    map.clear();
}

fn temp_path(dir: &Path) -> PathBuf {
    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(".db-secrets.{}.{seq}.tmp", std::process::id()))
}

fn write_atomic_0600(tmp: &Path, target: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(tmp)
        .map_err(|e| StoreError::io("create temp secrets file", &e))?;
    file.write_all(bytes)
        .map_err(|e| StoreError::io("write temp secrets file", &e))?;
    file.sync_all()
        .map_err(|e| StoreError::io("sync temp secrets file", &e))?;
    drop(file);
    restrict(tmp, 0o600)?;
    fs::rename(tmp, target).map_err(|e| StoreError::io("rename secrets file", &e))?;
    restrict(target, 0o600)
}

fn entries(map: &serde_json::Map<String, Value>) -> BTreeMap<String, String> {
    map.iter()
        .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_string())))
        .collect()
}

fn with_cipher<R>(f: impl FnOnce(&Aes256Gcm) -> R) -> R {
    let mut key = Sha256::digest(KEY_SEED.as_bytes());
    let cipher = Aes256Gcm::new(&key);
    key.zeroize();
    f(&cipher)
}

fn encrypt(plain: &str) -> Result<Envelope, StoreError> {
    // nonce novo a cada escrita: o arquivo é regravado inteiro, então (chave, nonce) nunca repete
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let data = with_cipher(|cipher| cipher.encrypt(&nonce, plain.as_bytes()))
        .map_err(|_| StoreError::Encrypt)?;
    Ok(Envelope {
        v: 1,
        n: STANDARD.encode(nonce),
        d: STANDARD.encode(data),
    })
}

fn decrypt(envelope: &serde_json::Map<String, Value>) -> Option<Zeroizing<String>> {
    if envelope.get("v").and_then(Value::as_i64) != Some(ENVELOPE_VERSION) {
        return None;
    }
    let nonce = STANDARD.decode(envelope.get("n")?.as_str()?).ok()?;
    let data = STANDARD.decode(envelope.get("d")?.as_str()?).ok()?;
    if nonce.len() != NONCE_BYTES {
        return None;
    }
    let plain = Zeroizing::new(
        with_cipher(|cipher| cipher.decrypt(Nonce::from_slice(&nonce), data.as_slice())).ok()?,
    );
    let text = String::from_utf8(plain.to_vec()).ok()?;
    Some(Zeroizing::new(text))
}

/// `0600`/`0700` no Unix; falha se não for possível restringir a permissão.
#[cfg(unix)]
fn restrict(path: &Path, mode: u32) -> Result<(), StoreError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode))
        .map_err(|e| StoreError::io("set permissions", &e))
}

#[cfg(not(unix))]
fn restrict(_path: &Path, _mode: u32) -> Result<(), StoreError> {
    Ok(())
}

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

    #[test]
    fn envelope_with_unknown_version_is_rejected_before_decrypting() {
        let (_dir, store) = store();
        store.write("/w", "pg", "secret").unwrap();
        let mut envelope: serde_json::Map<String, Value> =
            serde_json::from_str(&fs::read_to_string(&store.path).unwrap()).unwrap();
        for bad_version in [
            serde_json::json!(2),
            serde_json::json!(0),
            serde_json::json!("1"),
        ] {
            envelope.insert("v".into(), bad_version);
            fs::write(&store.path, serde_json::to_string(&envelope).unwrap()).unwrap();
            assert_eq!(store.read("/w", "pg"), None);
        }
        // `decrypt` diretamente também recusa versão != 1 mesmo com ciphertext v1 válido
        envelope.insert("v".into(), serde_json::json!(2));
        assert!(decrypt(&envelope).is_none());
    }

    #[test]
    fn atomic_write_leaves_no_temp_files_behind() {
        let (_dir, store) = store();
        store.write("/w", "pg", "one").unwrap();
        store.write("/w", "pg", "two").unwrap();
        let entries: Vec<_> = fs::read_dir(store.path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, ["db-secrets.json"]);
        assert_eq!(store.read("/w", "pg").as_deref(), Some("two"));
    }

    #[cfg(unix)]
    #[test]
    fn failing_to_restrict_permissions_returns_typed_error() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let err = restrict(&missing, 0o600).unwrap_err();
        assert!(
            matches!(
                err,
                StoreError::Io {
                    op: "set permissions",
                    ..
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn decrypted_plaintext_is_zeroized_on_drop() {
        let mut envelope: serde_json::Map<String, Value> =
            serde_json::from_str(DART_ENVELOPE).unwrap();
        envelope.insert("v".into(), serde_json::json!(1));
        let plain = decrypt(&envelope).unwrap();
        assert!(plain.contains("hunter2"));
        drop(plain);
        let mut map = BTreeMap::from([("k".to_string(), "super-secret".to_string())]);
        zeroize_map(&mut map);
        assert!(map.is_empty());
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
