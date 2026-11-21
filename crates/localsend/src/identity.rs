use crate::protocol::{DeviceInfo, VERSION};
use rcgen::{generate_simple_self_signed, CertifiedKey, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{self, BufReader, Cursor, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IdentityError {
    #[error("LocalSend data directory is unavailable")]
    DataDir,
    #[error("cannot access identity files at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot parse persisted certificate or key: {0}")]
    Pem(String),
    #[error("certificate generation failed: {0}")]
    Certificate(String),
    #[error("secure random source failed: {0}")]
    Random(String),
}

pub struct Identity {
    pub info: DeviceInfo,
    pub cert: CertificateDer<'static>,
    pub key: PrivateKeyDer<'static>,
    pub directory: PathBuf,
}

impl Clone for Identity {
    fn clone(&self) -> Self {
        Self {
            info: self.info.clone(),
            cert: self.cert.clone(),
            key: self.key.clone_key(),
            directory: self.directory.clone(),
        }
    }
}

impl Identity {
    pub fn load_or_create(port: u16) -> Result<Self, IdentityError> {
        let directory = dirs::data_dir()
            .ok_or(IdentityError::DataDir)?
            .join("work.gorila.gshdesktop")
            .join("localsend");
        fs::create_dir_all(&directory).map_err(|source| IdentityError::Io {
            path: directory.clone(),
            source,
        })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).map_err(
                |source| IdentityError::Io {
                    path: directory.clone(),
                    source,
                },
            )?;
        }
        let identity_path = directory.join("identity.pem");
        if !identity_path.exists() {
            let hostname = hostname();
            let CertifiedKey { cert, key_pair } =
                generate_simple_self_signed(vec![hostname.clone()])
                    .map_err(|error| IdentityError::Certificate(error.to_string()))?;
            let pem = format!("{}{}", cert.pem(), key_pair.serialize_pem());
            write_identity(&directory, &identity_path, pem.as_bytes())?;
        }
        let pair = read_pair(&identity_path)?;
        let digest = Sha256::digest(pair.0.as_ref());
        let fingerprint = digest
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":");
        let model = std::env::consts::OS.to_string();
        Ok(Self {
            info: DeviceInfo {
                alias: hostname(),
                version: VERSION.into(),
                device_model: Some(model),
                device_type: Some("desktop".into()),
                fingerprint,
                port: Some(port),
                protocol: Some("https".into()),
                download: false,
            },
            cert: pair.0,
            key: PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(pair.1.serialize_der())),
            directory,
        })
    }
}

fn read_pair(path: &PathBuf) -> Result<(CertificateDer<'static>, KeyPair), IdentityError> {
    let pem = fs::read(path).map_err(|source| IdentityError::Io {
        path: path.clone(),
        source,
    })?;
    let mut cert_reader = BufReader::new(Cursor::new(&pem));
    let cert = rustls_pemfile::certs(&mut cert_reader)
        .next()
        .ok_or_else(|| IdentityError::Pem("certificate missing".into()))?
        .map_err(|error| IdentityError::Pem(error.to_string()))?;
    let mut key_reader = BufReader::new(Cursor::new(&pem));
    let key_der = rustls_pemfile::private_key(&mut key_reader)
        .map_err(|error| IdentityError::Pem(error.to_string()))?
        .ok_or_else(|| IdentityError::Pem("private key missing".into()))?;
    let key = KeyPair::try_from(key_der.secret_der())
        .map_err(|error| IdentityError::Pem(error.to_string()))?;
    Ok((cert, key))
}

fn write_identity(directory: &Path, path: &Path, bytes: &[u8]) -> Result<(), IdentityError> {
    let mut random = [0u8; 12];
    getrandom::fill(&mut random).map_err(|error| IdentityError::Random(error.to_string()))?;
    let temporary = directory.join(format!(
        ".identity-{}.tmp",
        random
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let result = options.open(&temporary).and_then(|mut file| {
        file.write_all(bytes)?;
        file.sync_all()
    });
    if let Err(source) = result {
        let _ = fs::remove_file(&temporary);
        return Err(IdentityError::Io {
            path: temporary,
            source,
        });
    }
    match fs::hard_link(&temporary, path) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(source) => {
            let _ = fs::remove_file(&temporary);
            return Err(IdentityError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    }
    fs::remove_file(&temporary).map_err(|source| IdentityError::Io {
        path: temporary,
        source,
    })?;
    Ok(())
}

fn hostname() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .ok()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "GSH".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_certificate_and_key_reload_without_changing_fingerprint() {
        let dir =
            std::env::temp_dir().join(format!("gsh-localsend-identity-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("identity.pem");
        let generated = generate_simple_self_signed(vec!["test-host".into()]).unwrap();
        let pem = format!(
            "{}{}",
            generated.cert.pem(),
            generated.key_pair.serialize_pem()
        );
        write_identity(&dir, &path, pem.as_bytes()).unwrap();
        let (first_cert, first_key) = read_pair(&path).unwrap();
        let (second_cert, second_key) = read_pair(&path).unwrap();
        assert_eq!(first_cert, second_cert);
        assert_eq!(first_key.serialize_der(), second_key.serialize_der());
        let _ = fs::remove_dir_all(dir);
    }
}
