//! Cifra do time (`team_crypto.dart`): AES-256-GCM com a chave derivada da senha por
//! PBKDF2-HMAC-SHA256, salgada com o nome do time. Parametros identicos aos do Dart.
//!
//! Formato do frame: `[2][nonce de 12 bytes][cifrado + tag de 16 bytes]`, AAD `gsh-team-v2`.
//! Primitivas sao as do RustCrypto; aqui so ha a cola do formato.

use crate::error::TeamError;
use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use sha2::Sha256;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

const VERSION: u8 = 2;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const ITERATIONS: u32 = 100_000;
const AAD: &[u8] = b"gsh-team-v2";
const KEY_CACHE_CAP: usize = 16;

/// Chave de 32 bytes derivada de (time, senha). Barata de clonar; nunca vai para log.
#[derive(Clone)]
pub struct TeamKey(Arc<[u8; 32]>);

impl fmt::Debug for TeamKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TeamKey(<redacted>)")
    }
}

impl TeamKey {
    #[cfg(test)]
    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

type KeyCache = Mutex<HashMap<(String, String), TeamKey>>;

/// PBKDF2 e lento de proposito: deriva uma vez por (time, senha), como o `_keys` do Dart.
pub fn derive_key(team: &str, secret: &str) -> TeamKey {
    static CACHE: OnceLock<KeyCache> = OnceLock::new();
    let cache = CACHE.get_or_init(KeyCache::default);
    let id = (team.trim().to_owned(), secret.to_owned());
    if let Some(k) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&id)
    {
        return k.clone();
    }
    // fora do lock: a derivacao leva tempo e nao deve travar quem ja tem chave
    let mut key = [0_u8; 32];
    let salt = format!("gsh-team\n{}", id.0);
    pbkdf2::pbkdf2_hmac::<Sha256>(secret.as_bytes(), salt.as_bytes(), ITERATIONS, &mut key);
    let key = TeamKey(Arc::new(key));
    let mut map = cache.lock().unwrap_or_else(PoisonError::into_inner);
    if map.len() >= KEY_CACHE_CAP {
        map.clear();
    }
    map.entry(id).or_insert(key).clone()
}

/// Cifra `plain` para o time, com nonce novo do RNG do SO a cada chamada.
pub fn seal(key: &TeamKey, plain: &[u8]) -> Result<Vec<u8>, TeamError> {
    let mut nonce = [0_u8; NONCE_LEN];
    getrandom::fill(&mut nonce).map_err(|e| TeamError::Random {
        detail: e.to_string(),
    })?;
    seal_with_nonce(key, nonce, plain)
}

/// So para teste (vetor do Dart com nonce fixo): em producao o nonce nunca e escolhido por fora.
pub(crate) fn seal_with_nonce(
    key: &TeamKey,
    nonce: [u8; NONCE_LEN],
    plain: &[u8],
) -> Result<Vec<u8>, TeamError> {
    let cipher = Aes256Gcm::new_from_slice(key.0.as_slice()).map_err(|_| TeamError::Seal)?;
    let sealed = cipher
        .encrypt(
            &Nonce::from(nonce),
            Payload {
                msg: plain,
                aad: AAD,
            },
        )
        .map_err(|_| TeamError::Seal)?;
    let mut frame = Vec::with_capacity(1 + NONCE_LEN + sealed.len());
    frame.push(VERSION);
    frame.extend_from_slice(&nonce);
    frame.extend_from_slice(&sealed);
    Ok(frame)
}

/// Abre o que [`seal`] cifrou. `None` = outra senha/time, adulterado ou nao e deste formato.
/// O tamanho e a versao sao conferidos antes de qualquer decifragem.
pub fn open(key: &TeamKey, data: &[u8]) -> Option<Vec<u8>> {
    if data.len() < 1 + NONCE_LEN + TAG_LEN {
        return None;
    }
    let (&version, rest) = data.split_first()?;
    if version != VERSION {
        return None;
    }
    let (nonce, sealed) = rest.split_at_checked(NONCE_LEN)?;
    let nonce: [u8; NONCE_LEN] = nonce.try_into().ok()?;
    let cipher = Aes256Gcm::new_from_slice(key.0.as_slice()).ok()?;
    cipher
        .decrypt(
            &Nonce::from(nonce),
            Payload {
                msg: sealed,
                aad: AAD,
            },
        )
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_e_volta_e_nonce_novo_a_cada_frame() {
        let key = derive_key("gorila", "segredo");
        let a = seal(&key, b"oi").unwrap();
        let b = seal(&key, b"oi").unwrap();
        assert_ne!(a, b, "nonce nao pode repetir");
        assert_eq!(open(&key, &a).as_deref(), Some(&b"oi"[..]));
    }

    #[test]
    fn adulterado_ou_truncado_ou_outra_senha_nao_abre() {
        let key = derive_key("gorila", "segredo");
        let other = derive_key("gorila", "outra");
        let frame = seal(&key, b"conteudo").unwrap();
        assert!(open(&other, &frame).is_none());
        for i in 0..frame.len() {
            let mut bad = frame.clone();
            bad[i] ^= 1;
            assert!(open(&key, &bad).is_none(), "byte {i}");
        }
        for len in 0..frame.len() {
            assert!(open(&key, &frame[..len]).is_none(), "len {len}");
        }
        assert!(open(&key, &[]).is_none());
        let mut v1 = frame.clone();
        v1[0] = 1;
        assert!(open(&key, &v1).is_none());
    }

    #[test]
    fn time_aparado_no_sal_e_senha_exata() {
        assert_eq!(
            derive_key("  gorila  ", "s").as_bytes(),
            derive_key("gorila", "s").as_bytes()
        );
        assert_ne!(
            derive_key("gorila", "s").as_bytes(),
            derive_key("gorila", "s ").as_bytes()
        );
        assert_ne!(
            derive_key("a", "s").as_bytes(),
            derive_key("b", "s").as_bytes()
        );
    }
}
