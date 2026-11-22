//! Servidor do socket de status: `~/.gsh/status[-debug].sock` no Unix; TCP no
//! loopback com token no Windows (lá não há socket Unix).
//!
//! Cada conexão traz **uma linha JSON** e é despachada na primeira linha, não no
//! fim da conexão: o `gsh hook` (status) fecha logo após escrever, mas a CLI
//! (`"type":"cmd"`) espera a resposta com a conexão aberta. Status não tem
//! resposta; comando tem uma linha de resposta e a conexão fecha.
//!
//! Comandos da CLI são entregues ao app para decisão; pedidos de permissão
//! podem manter a conexão aberta por até 24 h, como o timeout do hook Claude.
//! Se o app não estiver ouvindo, a CLI deixa o terminal perguntar.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::net::{Ipv4Addr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, SyncSender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use gsh_core::status::{LineError, StatusUpdate};
use gsh_core::wire::{ENV_STATUS_PORT, ENV_STATUS_TOKEN};
use serde_json::Value;
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::net::UnixListener;

mod connections;
use connections::{accept_loop, Listener};

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("cannot create {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("cannot clear stale socket {path}: {source}")]
    StaleSocket {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    /// Algo que não é socket ocupa o caminho: não apagamos arquivo alheio.
    #[error("{path} exists and is not a socket")]
    PathOccupied { path: PathBuf },
    #[error("cannot bind {target}: {source}")]
    Bind {
        target: String,
        #[source]
        source: io::Error,
    },
    #[error("cannot restrict permissions of {path}: {source}")]
    Permissions {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("no secure random source for the token: {0}")]
    Token(#[source] getrandom::Error),
    #[error("cannot start server thread: {0}")]
    Thread(#[source] io::Error),
}

/// Transporte do servidor. [`Transport::native`] é o de produção; o TCP existe
/// em todas as plataformas para poder ser testado fora do Windows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    #[cfg(unix)]
    Unix,
    Tcp,
}

impl Transport {
    /// Socket Unix onde existe; TCP + token no Windows.
    pub const fn native() -> Transport {
        #[cfg(unix)]
        {
            Transport::Unix
        }
        #[cfg(not(unix))]
        {
            Transport::Tcp
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// `~/.gsh`: onde moram o socket e o arquivo de endpoint.
    pub gsh_dir: PathBuf,
    /// Sufixo `-debug` nos nomes. Um GSH de dev e um instalado rodam lado a lado
    /// sobre o mesmo `~/.gsh`; sem o sufixo um apagaria o socket do outro.
    pub debug: bool,
    pub transport: Transport,
}

impl ServerConfig {
    /// Flavor pelo perfil de compilação (`debug_assertions` = o `kDebugMode` do
    /// Flutter) e transporte nativo.
    pub fn new(gsh_dir: impl Into<PathBuf>) -> ServerConfig {
        ServerConfig {
            gsh_dir: gsh_dir.into(),
            debug: cfg!(debug_assertions),
            transport: Transport::native(),
        }
    }
}

/// `~/.gsh` da conta atual (`HOME`, senão `USERPROFILE`, senão o diretório
/// temporário, como o app original).
pub fn gsh_dir_from_env() -> PathBuf {
    ["HOME", "USERPROFILE"]
        .iter()
        .find_map(|k| std::env::var_os(k).filter(|v| !v.is_empty()))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join(".gsh")
}

/// Onde o servidor está ouvindo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Endpoint {
    #[cfg(unix)]
    Unix(PathBuf),
    /// Loopback `127.0.0.1:<port>`; o token fica no servidor.
    Tcp { port: u16 },
}

/// O que o servidor entrega ao app.
#[derive(Debug)]
pub enum Delivery {
    Status(StatusUpdate),
    Command(PendingCommand),
    /// Linha que não é JSON (o app original registrava um aviso com o trecho).
    Malformed(LineError),
}

#[derive(Debug, Clone)]
pub struct PendingCommand {
    pub request: Value,
    pub reply: SyncSender<Value>,
}

/// Servidor de status em execução. Derrubar (`stop` ou `Drop`) fecha o listener
/// e as conexões pendentes e apaga o socket (ou o arquivo de endpoint).
#[derive(Debug)]
pub struct StatusServer {
    endpoint: Endpoint,
    token: Option<String>,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
    /// Socket (Unix) ou arquivo de endpoint (TCP) a apagar ao sair.
    cleanup: PathBuf,
    endpoint_file_error: Option<io::Error>,
}

struct Shared {
    stop: Arc<AtomicBool>,
    token: Option<String>,
    // ponytail: canal sem limite; o consumidor é a UI, que drena por tick.
    // Se o app parar de ler, troque por `sync_channel` e descarte o mais antigo.
    tx: Sender<Delivery>,
}

impl Shared {
    fn stopping(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }
}

impl StatusServer {
    /// Sobe o servidor. Um socket Unix de execução anterior (app que caiu sem
    /// limpar) é apagado e refeito, como o app original; qualquer coisa que não
    /// seja socket no caminho é erro. Quem chama decide se segue sem status: o
    /// app original seguia, com o `hookEnv` vazio.
    pub fn start(config: &ServerConfig) -> Result<(StatusServer, Receiver<Delivery>), ServerError> {
        fs::create_dir_all(&config.gsh_dir).map_err(|source| ServerError::CreateDir {
            path: config.gsh_dir.clone(),
            source,
        })?;
        let stem = if config.debug {
            "status-debug"
        } else {
            "status"
        };

        let (listener, endpoint, token, cleanup, endpoint_file_error) = match config.transport {
            #[cfg(unix)]
            Transport::Unix => {
                let path = config.gsh_dir.join(format!("{stem}.sock"));
                let listener = bind_private_unix(&path)?;
                (
                    Listener::Unix(listener),
                    Endpoint::Unix(path.clone()),
                    None,
                    path,
                    None,
                )
            }
            Transport::Tcp => {
                let token = random_token()?;
                let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|source| {
                    ServerError::Bind {
                        target: "127.0.0.1:0".to_string(),
                        source,
                    }
                })?;
                let port = listener
                    .local_addr()
                    .map_err(|source| ServerError::Bind {
                        target: "127.0.0.1:0".to_string(),
                        source,
                    })?
                    .port();
                // Anuncia `{port, tok}` para quem não herda o env de uma aba (CLI
                // rodada de fora, segunda instância). Falhar aqui não derruba o
                // servidor: as abas recebem o env direto.
                let path = config.gsh_dir.join(format!("{stem}.json"));
                let err = write_endpoint_file(&path, port, &token).err();
                (
                    Listener::Tcp(listener),
                    Endpoint::Tcp { port },
                    Some(token),
                    path,
                    err,
                )
            }
        };

        listener
            .set_nonblocking()
            .map_err(|source| ServerError::Bind {
                target: "nonblocking accept".to_string(),
                source,
            })?;
        let (tx, rx) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let shared = Arc::new(Shared {
            stop: Arc::clone(&stop),
            token: token.clone(),
            tx,
        });
        let accept = thread::Builder::new()
            .name("gsh-status-accept".into())
            .spawn(move || accept_loop(listener, &shared));
        let accept = match accept {
            Ok(handle) => handle,
            Err(source) => {
                let _ = fs::remove_file(&cleanup);
                return Err(ServerError::Thread(source));
            }
        };
        Ok((
            StatusServer {
                endpoint,
                token,
                stop,
                accept: Some(accept),
                cleanup,
                endpoint_file_error,
            },
            rx,
        ))
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Env que o PTY de cada aba precisa para o `gsh hook` e a CLI acharem o
    /// app: `GSH_STATUS_SOCK` no Unix; `GSH_STATUS_PORT` e `GSH_STATUS_TOKEN` no
    /// TCP. Quem injeta no PTY é o terminal, não este crate.
    pub fn hook_env(&self) -> Vec<(&'static str, String)> {
        match &self.endpoint {
            #[cfg(unix)]
            Endpoint::Unix(path) => {
                vec![(
                    gsh_core::wire::ENV_STATUS_SOCK,
                    path.to_string_lossy().into_owned(),
                )]
            }
            Endpoint::Tcp { port } => {
                let mut env = vec![(ENV_STATUS_PORT, port.to_string())];
                if let Some(tok) = &self.token {
                    env.push((ENV_STATUS_TOKEN, tok.clone()));
                }
                env
            }
        }
    }

    /// Por que o arquivo de endpoint (TCP) não foi gravado, se não foi.
    pub fn endpoint_file_error(&self) -> Option<&io::Error> {
        self.endpoint_file_error.as_ref()
    }

    /// Derruba o servidor e apaga o socket. Equivale a deixar cair.
    pub fn stop(self) {
        drop(self);
    }

    fn shutdown(&mut self) {
        if let Some(handle) = self.accept.take() {
            self.stop.store(true, Ordering::Release);
            let _ = handle.join();
        }
        // Melhor esforço: socket/endpoint órfão é refeito no próximo `start`.
        let _ = fs::remove_file(&self.cleanup);
    }
}

impl Drop for StatusServer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(unix)]
pub fn bind_private_unix(path: &Path) -> Result<UnixListener, ServerError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_socket() => {
            fs::remove_file(path).map_err(|source| ServerError::StaleSocket {
                path: path.to_path_buf(),
                source,
            })?;
        }
        Ok(_) => {
            return Err(ServerError::PathOccupied {
                path: path.to_path_buf(),
            })
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(ServerError::StaleSocket {
                path: path.to_path_buf(),
                source,
            })
        }
    }
    let listener = UnixListener::bind(path).map_err(|source| ServerError::Bind {
        target: path.display().to_string(),
        source,
    })?;
    // Só o dono conecta (conectar a um UDS exige escrita no arquivo). Há uma
    // janela entre o bind e o chmod; o `~/.gsh` é da conta.
    if let Err(source) = fs::set_permissions(path, fs::Permissions::from_mode(0o600)) {
        let _ = fs::remove_file(path);
        return Err(ServerError::Permissions {
            path: path.to_path_buf(),
            source,
        });
    }
    Ok(listener)
}

fn random_token() -> Result<String, ServerError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(ServerError::Token)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

fn write_endpoint_file(path: &Path, port: u16, token: &str) -> io::Result<()> {
    let body = serde_json::json!({"port": port, "tok": token}).to_string();
    let mut opts = OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    opts.mode(0o600);
    opts.open(path)?.write_all(body.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_tem_32_hex_e_muda() {
        let a = random_token().unwrap();
        let b = random_token().unwrap();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
