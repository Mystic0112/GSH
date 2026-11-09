//! Servidor do socket de status: `~/.gsh/status[-debug].sock` no Unix; TCP no
//! loopback com token no Windows (lá não há socket Unix).
//!
//! Cada conexão traz **uma linha JSON** e é despachada na primeira linha, não no
//! fim da conexão: o `gsh hook` (status) fecha logo após escrever, mas a CLI
//! (`"type":"cmd"`) espera a resposta com a conexão aberta. Status não tem
//! resposta; comando tem uma linha de resposta e a conexão fecha.
//!
//! Comandos da CLI são outra camada: por ora todo comando válido recebe
//! `commands unavailable`, o mesmo que o app original responde sem handler
//! (o `gsh hook` trata como "deixa o terminal perguntar").

use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use gsh_core::status::{decode_line, Incoming, LineError, StatusUpdate};
use gsh_core::wire::{ENV_STATUS_PORT, ENV_STATUS_TOKEN};
use thiserror::Error;

#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt, PermissionsExt};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

/// Teto de uma linha. Folga larga: o maior payload é o `write` da CLI (texto
/// em base64). Acima disso a linha vira lixo e a conexão cai.
const MAX_LINE_BYTES: u64 = 16 * 1024 * 1024;

/// Quanto esperar a linha de um cliente. O hook escreve na hora; sem teto, um
/// cliente mudo seguraria uma thread para sempre.
const READ_TIMEOUT: Duration = Duration::from_secs(10);

/// Intervalo do `accept` sem conexão pendente (ver `accept_loop`).
const ACCEPT_POLL: Duration = Duration::from_millis(25);

const REPLY_INVALID_TOKEN: &[u8] = b"{\"ok\":false,\"error\":\"invalid token\"}\n";
const REPLY_COMMANDS_UNAVAILABLE: &[u8] = b"{\"ok\":false,\"error\":\"commands unavailable\"}\n";

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
    /// Linha que não é JSON (o app original registrava um aviso com o trecho).
    Malformed(LineError),
}

/// Servidor de status em execução. Derrubar (`stop` ou `Drop`) fecha o listener
/// e apaga o socket (ou o arquivo de endpoint).
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
    token: Option<String>,
    // ponytail: canal sem limite; o consumidor é a UI, que drena por tick.
    // Se o app parar de ler, troque por `sync_channel` e descarte o mais antigo.
    tx: Sender<Delivery>,
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
                let listener = bind_unix(&path)?;
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
            token: token.clone(),
            tx,
        });
        let accept = {
            let stop = Arc::clone(&stop);
            thread::Builder::new()
                .name("gsh-status-accept".into())
                .spawn(move || accept_loop(&listener, &stop, &shared))
        };
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
fn bind_unix(path: &Path) -> Result<UnixListener, ServerError> {
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

enum Listener {
    #[cfg(unix)]
    Unix(UnixListener),
    Tcp(TcpListener),
}

impl Listener {
    fn set_nonblocking(&self) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Listener::Unix(l) => l.set_nonblocking(true),
            Listener::Tcp(l) => l.set_nonblocking(true),
        }
    }

    fn accept(&self) -> io::Result<Stream> {
        match self {
            #[cfg(unix)]
            Listener::Unix(l) => l.accept().map(|(s, _)| Stream::Unix(s)),
            Listener::Tcp(l) => l.accept().map(|(s, _)| Stream::Tcp(s)),
        }
    }
}

enum Stream {
    #[cfg(unix)]
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl Stream {
    /// Conexão aceita de um listener não bloqueante pode herdar o modo (macOS,
    /// Windows); a thread da conexão lê de forma bloqueante, com timeout.
    fn set_blocking(&self) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.set_nonblocking(false),
            Stream::Tcp(s) => s.set_nonblocking(false),
        }
    }

    fn set_read_timeout(&self, dur: Duration) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.set_read_timeout(Some(dur)),
            Stream::Tcp(s) => s.set_read_timeout(Some(dur)),
        }
    }
}

impl Read for Stream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.read(buf),
            Stream::Tcp(s) => s.read(buf),
        }
    }
}

impl Write for Stream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.write(buf),
            Stream::Tcp(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            #[cfg(unix)]
            Stream::Unix(s) => s.flush(),
            Stream::Tcp(s) => s.flush(),
        }
    }
}

/// `accept` não bloqueante com respiro: o shutdown só precisa ligar a flag, sem
/// depender de o caminho do socket ainda apontar para nós (outro processo pode
/// ter apagado e refeito o arquivo, e um "acorda o accept" iria para ele).
// ponytail: ~40 despertares/s ocioso e até 25 ms de latência no accept. Se
// incomodar, troque por poll/epoll (mio) com um waker.
fn accept_loop(listener: &Listener, stop: &AtomicBool, shared: &Arc<Shared>) {
    while !stop.load(Ordering::Acquire) {
        match listener.accept() {
            Ok(stream) => {
                if stream.set_blocking().is_err() {
                    continue;
                }
                let shared = Arc::clone(shared);
                // Falha ao criar a thread derruba só esta conexão.
                let _ = thread::Builder::new()
                    .name("gsh-status-conn".into())
                    .spawn(move || handle(stream, &shared));
            }
            // Sem conexão pendente, ou erro de accept (fds esgotados, conexão
            // abortada): não derruba o servidor.
            Err(_) => thread::sleep(ACCEPT_POLL),
        }
    }
}

fn handle(mut stream: Stream, shared: &Shared) {
    // O cliente do hook escreve e fecha na hora. No macOS o setsockopt do
    // timeout devolve EINVAL num socket Unix cujo par já fechou, mas a leitura
    // abaixo ainda devolve os dados e o EOF, sem precisar do timeout. Só
    // desistimos de outro erro.
    match stream.set_read_timeout(READ_TIMEOUT) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::InvalidInput => {}
        Err(_) => return,
    }
    let Ok(line) = read_line_capped(&mut stream, MAX_LINE_BYTES) else {
        return;
    };
    let reply: Option<&[u8]> = match decode_line(&line, shared.token.as_deref()) {
        Ok(Incoming::Status(update)) => {
            let _ = shared.tx.send(Delivery::Status(update));
            None
        }
        Ok(Incoming::Command) => Some(REPLY_COMMANDS_UNAVAILABLE),
        Ok(Incoming::BadToken { command: true }) => Some(REPLY_INVALID_TOKEN),
        Ok(Incoming::BadToken { command: false } | Incoming::Ignored) => None,
        Err(err) => {
            let _ = shared.tx.send(Delivery::Malformed(err));
            None
        }
    };
    if let Some(reply) = reply {
        let _ = stream.write_all(reply).and_then(|()| stream.flush());
    }
}

/// Lê até a primeira quebra de linha, ou o fim da conexão, ou `cap` bytes. A
/// linha sem `\n` final conta (o cliente do hook fecha logo após escrever).
fn read_line_capped<R: Read>(reader: R, cap: u64) -> io::Result<String> {
    let mut buf = Vec::new();
    BufReader::new(reader.take(cap)).read_until(b'\n', &mut buf)?;
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_so_a_primeira_linha() {
        let got = read_line_capped(&b"um\ndois\n"[..], 100).unwrap();
        assert_eq!(got, "um\n");
    }

    #[test]
    fn linha_sem_quebra_final_vale() {
        assert_eq!(
            read_line_capped(&b"{\"a\":1}"[..], 100).unwrap(),
            "{\"a\":1}"
        );
        assert_eq!(read_line_capped(&b""[..], 100).unwrap(), "");
    }

    #[test]
    fn teto_corta_linha_gigante() {
        let got = read_line_capped(&b"abcdefghij\n"[..], 4).unwrap();
        assert_eq!(got, "abcd");
    }

    #[test]
    fn token_tem_32_hex_e_muda() {
        let a = random_token().unwrap();
        let b = random_token().unwrap();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }
}
