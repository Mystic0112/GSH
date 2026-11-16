//! Túnel SSH (`russh`) com port-forward local e proxy SOCKS5 (`ssh_tunnel_impl.dart`).
//!
//! Os túneis vivem em um runtime Tokio persistente do processo para sobreviver ao modelo efêmero
//! (open -> query -> close) dos drivers, com TTL ocioso de 5 minutos.

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use russh::client::{self, AuthResult, Config, Handle};
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::connection::SshTunnel;
use crate::error::DbError;
use crate::ssh_keys::{host_key_fingerprint, load_private_key, SshHostKeyStore};
use crate::ssh_socks::LocalSocksServer;

const IDLE_TTL: Duration = Duration::from_secs(5 * 60);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(20);
const OPEN_TIMEOUT: Duration = Duration::from_secs(30);

/// Ponta local de um túnel ativo (`127.0.0.1:<porta>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelEndpoint {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
enum HostKeyRejection {
    Unknown {
        endpoint: String,
        fingerprint: String,
    },
    Changed {
        endpoint: String,
        expected: String,
        actual: String,
    },
}

pub struct ClientHandler {
    endpoint: String,
    host_keys: SshHostKeyStore,
    rejection: Arc<Mutex<Option<HostKeyRejection>>>,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let printed = match server_public_key {
            PublicKeyOrCertificate::PublicKey { key, .. } => host_key_fingerprint(key),
            PublicKeyOrCertificate::Certificate(cert) => {
                cert.public_key().fingerprint(HashAlg::Sha256).to_string()
            }
        };
        match self.host_keys.trusted(&self.endpoint) {
            Some(known) if known == printed => Ok(true),
            Some(known) => {
                if let Ok(mut slot) = self.rejection.lock() {
                    *slot = Some(HostKeyRejection::Changed {
                        endpoint: self.endpoint.clone(),
                        expected: known,
                        actual: printed,
                    });
                }
                Ok(false)
            }
            None => {
                if let Ok(mut slot) = self.rejection.lock() {
                    *slot = Some(HostKeyRejection::Unknown {
                        endpoint: self.endpoint.clone(),
                        fingerprint: printed,
                    });
                }
                Ok(false)
            }
        }
    }
}

struct ActiveForward {
    handle: Arc<Handle<ClientHandler>>,
    port: u16,
    last_used: Arc<Mutex<Instant>>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl ActiveForward {
    fn is_alive(&self) -> bool {
        !self.handle.is_closed()
            && self
                .last_used
                .lock()
                .is_ok_and(|ts| ts.elapsed() < IDLE_TTL)
    }

    fn touch(&self) -> TunnelEndpoint {
        if let Ok(mut ts) = self.last_used.lock() {
            *ts = Instant::now();
        }
        TunnelEndpoint {
            host: "127.0.0.1".to_string(),
            port: self.port,
        }
    }

    fn close(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for ActiveForward {
    fn drop(&mut self) {
        self.close();
    }
}

struct ActiveSocks {
    handle: Arc<Handle<ClientHandler>>,
    server: LocalSocksServer,
    last_used: Arc<Mutex<Instant>>,
}

impl ActiveSocks {
    fn is_alive(&self) -> bool {
        !self.handle.is_closed()
            && self.server.is_alive()
            && self
                .last_used
                .lock()
                .is_ok_and(|ts| ts.elapsed() < IDLE_TTL)
    }

    fn touch(&self) -> TunnelEndpoint {
        if let Ok(mut ts) = self.last_used.lock() {
            *ts = Instant::now();
        }
        TunnelEndpoint {
            host: self.server.host.clone(),
            port: self.server.port,
        }
    }
}

/// Gerenciador global de túneis SSH do processo.
pub struct SshTunnelManager {
    host_keys: SshHostKeyStore,
    forwards: Mutex<HashMap<String, ActiveForward>>,
    socks: Mutex<HashMap<String, ActiveSocks>>,
}

static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
static GLOBAL: OnceLock<SshTunnelManager> = OnceLock::new();

fn runtime() -> Result<&'static tokio::runtime::Runtime, DbError> {
    if let Some(rt) = RUNTIME.get() {
        return Ok(rt);
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| DbError::SshConnectFailed(e.to_string()))?;
    let _ = RUNTIME.set(rt);
    RUNTIME
        .get()
        .ok_or_else(|| DbError::SshConnectFailed("SSH runtime unavailable".into()))
}

impl SshTunnelManager {
    pub fn new(host_keys: SshHostKeyStore) -> Self {
        Self {
            host_keys,
            forwards: Mutex::new(HashMap::new()),
            socks: Mutex::new(HashMap::new()),
        }
    }

    pub fn global() -> &'static Self {
        GLOBAL.get_or_init(|| Self::new(SshHostKeyStore::system()))
    }

    pub fn host_keys(&self) -> &SshHostKeyStore {
        &self.host_keys
    }

    /// Garante um túnel port-forward de `config` até `target_host:target_port`.
    pub fn ensure(
        &self,
        config: &SshTunnel,
        target_host: &str,
        target_port: u16,
        passphrase: Option<&str>,
    ) -> Result<TunnelEndpoint, DbError> {
        let key = format!(
            "{}|{}|{target_host}:{target_port}",
            config.endpoint(),
            config.key_path
        );
        if let Ok(mut map) = self.forwards.lock() {
            if let Some(live) = map.get(&key) {
                if live.is_alive() {
                    return Ok(live.touch());
                }
            }
            map.remove(&key);
        }

        let rt = runtime()?;
        let cfg = config.clone();
        let target_h = target_host.to_string();
        let pass = passphrase.map(str::to_string);
        let host_keys = self.host_keys.clone();
        let forward = rt.block_on(async move {
            tokio::time::timeout(
                OPEN_TIMEOUT,
                open_forward(cfg, target_h, target_port, pass, host_keys),
            )
            .await
            .map_err(|_| {
                DbError::SshConnectFailed(format!(
                    "Timed out opening the SSH tunnel to {} after {}s.",
                    config.endpoint(),
                    OPEN_TIMEOUT.as_secs()
                ))
            })?
        })?;

        let endpoint = forward.touch();
        if let Ok(mut map) = self.forwards.lock() {
            map.insert(key, forward);
        }
        Ok(endpoint)
    }

    /// Garante um proxy SOCKS5 local sobre a conexão SSH (para MongoDB).
    pub fn ensure_socks(
        &self,
        config: &SshTunnel,
        passphrase: Option<&str>,
    ) -> Result<TunnelEndpoint, DbError> {
        let key = format!("{}|{}", config.endpoint(), config.key_path);
        if let Ok(mut map) = self.socks.lock() {
            if let Some(live) = map.get(&key) {
                if live.is_alive() {
                    return Ok(live.touch());
                }
            }
            map.remove(&key);
        }

        let rt = runtime()?;
        let cfg = config.clone();
        let pass = passphrase.map(str::to_string);
        let host_keys = self.host_keys.clone();
        let socks = rt.block_on(async move {
            tokio::time::timeout(OPEN_TIMEOUT, open_socks(cfg, pass, host_keys))
                .await
                .map_err(|_| {
                    DbError::SshConnectFailed(format!(
                        "Timed out opening the SSH tunnel to {} after {}s.",
                        config.endpoint(),
                        OPEN_TIMEOUT.as_secs()
                    ))
                })?
        })?;

        let endpoint = socks.touch();
        if let Ok(mut map) = self.socks.lock() {
            map.insert(key, socks);
        }
        Ok(endpoint)
    }

    /// Derruba todos os túneis e proxies ativos.
    pub fn close_all(&self) {
        let forwards: Vec<ActiveForward> = self
            .forwards
            .lock()
            .map(|mut m| m.drain().map(|(_, v)| v).collect())
            .unwrap_or_default();
        let socks: Vec<ActiveSocks> = self
            .socks
            .lock()
            .map(|mut m| m.drain().map(|(_, v)| v).collect())
            .unwrap_or_default();
        if let Ok(rt) = runtime() {
            rt.block_on(async move {
                for mut f in forwards {
                    f.close();
                    let _ = f
                        .handle
                        .disconnect(russh::Disconnect::ByApplication, "", "")
                        .await;
                }
                for mut s in socks {
                    s.server.close();
                    let _ = s
                        .handle
                        .disconnect(russh::Disconnect::ByApplication, "", "")
                        .await;
                }
            });
        }
    }
}

async fn connect_ssh(
    config: &SshTunnel,
    passphrase: Option<&str>,
    host_keys: SshHostKeyStore,
) -> Result<Arc<Handle<ClientHandler>>, DbError> {
    let key = load_private_key(&config.key_path, passphrase)?;
    let rejection = Arc::new(Mutex::new(None));
    let handler = ClientHandler {
        endpoint: config.endpoint(),
        host_keys,
        rejection: Arc::clone(&rejection),
    };
    let ssh_cfg = Config {
        inactivity_timeout: Some(IDLE_TTL),
        keepalive_interval: Some(Duration::from_secs(30)),
        ..Config::default()
    };

    let port = u16::try_from(config.port).unwrap_or(22);
    let mut session = match tokio::time::timeout(
        HANDSHAKE_TIMEOUT,
        client::connect(Arc::new(ssh_cfg), (config.host.as_str(), port), handler),
    )
    .await
    {
        Ok(Ok(handle)) => handle,
        Ok(Err(err)) => {
            if let Some(rej) = rejection.lock().ok().and_then(|mut r| r.take()) {
                return Err(match rej {
                    HostKeyRejection::Changed {
                        endpoint,
                        expected,
                        actual,
                    } => DbError::SshHostKeyChanged {
                        endpoint,
                        expected,
                        actual,
                    },
                    HostKeyRejection::Unknown {
                        endpoint,
                        fingerprint,
                    } => DbError::SshHostKeyUnknown {
                        message: format!(
                            "Unknown host key for {endpoint} ({fingerprint}). \
                             Open this connection in the GSH UI once to review and trust it."
                        ),
                        endpoint,
                        fingerprint,
                    },
                });
            }
            return Err(DbError::SshConnectFailed(format!(
                "Cannot reach SSH server {}:{port}: {err}",
                config.host
            )));
        }
        Err(_) => {
            return Err(DbError::SshConnectFailed(format!(
                "Timed out opening the SSH tunnel to {}.",
                config.endpoint()
            )));
        }
    };

    let hash_alg = if key.algorithm().is_rsa() {
        session
            .best_supported_rsa_hash()
            .await
            .ok()
            .flatten()
            .flatten()
    } else {
        None
    };
    let auth = tokio::time::timeout(
        HANDSHAKE_TIMEOUT,
        session.authenticate_publickey(
            &config.user,
            PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg),
        ),
    )
    .await
    .map_err(|_| {
        DbError::SshConnectFailed(format!(
            "Timed out authenticating SSH tunnel to {}.",
            config.endpoint()
        ))
    })?
    .map_err(|e| DbError::SshAuthFailed(format!("SSH authentication failed: {e}")))?;

    if !matches!(auth, AuthResult::Success) {
        return Err(DbError::SshAuthFailed(format!(
            "SSH authentication failed for {}. Check the user and whether this key is \
             authorized on the server.",
            config.endpoint()
        )));
    }
    Ok(Arc::new(session))
}

async fn open_forward(
    config: SshTunnel,
    target_host: String,
    target_port: u16,
    passphrase: Option<String>,
    host_keys: SshHostKeyStore,
) -> Result<ActiveForward, DbError> {
    let handle = connect_ssh(&config, passphrase.as_deref(), host_keys).await?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|e| {
            DbError::SshConnectFailed(format!("Cannot open local forwarding port: {e}"))
        })?;
    let port = listener
        .local_addr()
        .map_err(|e| DbError::SshConnectFailed(e.to_string()))?
        .port();
    let last_used = Arc::new(Mutex::new(Instant::now()));
    let last_used_task = Arc::clone(&last_used);
    let handle_task = Arc::clone(&handle);
    let (tx, mut rx) = oneshot::channel();

    tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut rx => break,
                accepted = listener.accept() => {
                    let Ok((mut local, _)) = accepted else {
                        continue;
                    };
                    if let Ok(mut ts) = last_used_task.lock() {
                        *ts = Instant::now();
                    }
                    let h = Arc::clone(&handle_task);
                    let th = target_host.clone();
                    tokio::spawn(async move {
                        let Ok(channel) = h
                            .channel_open_direct_tcpip(th, u32::from(target_port), "127.0.0.1", 0)
                            .await
                        else {
                            return;
                        };
                        let mut remote = channel.into_stream();
                        let _ = tokio::io::copy_bidirectional(&mut local, &mut remote).await;
                    });
                }
            }
        }
    });

    Ok(ActiveForward {
        handle,
        port,
        last_used,
        shutdown: Some(tx),
    })
}

async fn open_socks(
    config: SshTunnel,
    passphrase: Option<String>,
    host_keys: SshHostKeyStore,
) -> Result<ActiveSocks, DbError> {
    let handle = connect_ssh(&config, passphrase.as_deref(), host_keys).await?;
    let server = LocalSocksServer::start(Arc::clone(&handle))
        .await
        .map_err(|e| DbError::SshConnectFailed(format!("Cannot open local SOCKS proxy: {e}")))?;
    Ok(ActiveSocks {
        handle,
        server,
        last_used: Arc::new(Mutex::new(Instant::now())),
    })
}
