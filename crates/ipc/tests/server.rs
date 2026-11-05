//! Servidor de status contra clientes crus (sem o binário `gsh`): protocolo de
//! fio, token, socket órfão, permissão e limpeza. O cliente real (`gsh hook`) é
//! exercitado em `crates/cli/tests/status_server.rs`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use gsh_core::status::{AgentHarness, HookEvent, StatusUpdate, TurnStatus};
use gsh_core::wire;
use gsh_ipc::{Delivery, Endpoint, ServerConfig, ServerError, StatusServer, Transport};

const WAIT: Duration = Duration::from_secs(5);
const QUIET: Duration = Duration::from_millis(300);

fn unique_dir(prefix: &str) -> PathBuf {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    // Caminho curto: `sun_path` do Unix tem ~104 bytes.
    let dir = std::env::temp_dir().join(format!(
        "{prefix}{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Nome do arquivo de socket/endpoint que o servidor usa neste perfil.
fn file_name(debug: bool, ext: &str) -> String {
    format!("{}.{ext}", if debug { "status-debug" } else { "status" })
}

fn config(home: &Path, transport: Transport) -> ServerConfig {
    let mut config = ServerConfig::new(home.join(".gsh"));
    config.transport = transport;
    config
}

/// Servidor num `~/.gsh` temporário que some ao final.
struct Fixture {
    home: PathBuf,
    server: StatusServer,
    rx: Receiver<Delivery>,
}

impl Fixture {
    fn start(transport: Transport) -> Fixture {
        let home = unique_dir("gi");
        let (server, rx) = StatusServer::start(&config(&home, transport)).unwrap();
        Fixture { home, server, rx }
    }

    fn status(&self) -> StatusUpdate {
        match self.rx.recv_timeout(WAIT).expect("nada entregue") {
            Delivery::Status(u) => u,
            other => panic!("esperava Status, veio {other:?}"),
        }
    }

    fn assert_quiet(&self) {
        match self.rx.recv_timeout(QUIET) {
            Err(RecvTimeoutError::Timeout) => {}
            other => panic!("não devia entregar nada: {other:?}"),
        }
    }

    /// Token que o servidor exige (`None` no Unix).
    fn token(&self) -> Option<String> {
        self.server
            .hook_env()
            .into_iter()
            .find(|(k, _)| *k == wire::ENV_STATUS_TOKEN)
            .map(|(_, v)| v)
    }

    /// Acrescenta o token do servidor, se ele exigir um.
    fn authed(&self, msg: serde_json::Value) -> String {
        match self.token() {
            Some(tok) => wire::with_token(msg, &tok).to_string(),
            None => msg.to_string(),
        }
    }

    /// Manda `line` e espera o servidor fechar a conexão (= já processou).
    /// Devolve o que ele respondeu.
    fn send(&self, line: &str) -> String {
        send_to(&self.server, line)
    }
}

fn send_to(server: &StatusServer, line: &str) -> String {
    let mut conn = Conn::connect(server);
    conn.write_all(line.as_bytes()).unwrap();
    let mut reply = String::new();
    conn.read_to_string(&mut reply).unwrap();
    reply
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

enum Conn {
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
    Tcp(TcpStream),
}

impl Conn {
    fn connect(server: &StatusServer) -> Conn {
        match server.endpoint() {
            #[cfg(unix)]
            Endpoint::Unix(path) => {
                Conn::Unix(std::os::unix::net::UnixStream::connect(path).unwrap())
            }
            Endpoint::Tcp { port } => Conn::Tcp(TcpStream::connect(("127.0.0.1", *port)).unwrap()),
        }
    }
}

impl Conn {
    /// Fecha só a escrita: o servidor vê o fim da linha pelo EOF.
    fn close_write(&self) {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.shutdown(std::net::Shutdown::Write).unwrap(),
            Conn::Tcp(s) => s.shutdown(std::net::Shutdown::Write).unwrap(),
        }
    }
}

impl Read for Conn {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.read(buf),
            Conn::Tcp(s) => s.read(buf),
        }
    }
}

impl Write for Conn {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.write(buf),
            Conn::Tcp(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Conn::Unix(s) => s.flush(),
            Conn::Tcp(s) => s.flush(),
        }
    }
}

const FULL: &str = r#"{"ev":"PostToolUse","files":["/r/a.rs"],"hn":"codex","paneId":"t3","sid":"s1","st":"working","tid":"u1","tx":"/t.jsonl"}"#;

fn full_update() -> StatusUpdate {
    StatusUpdate {
        pane_id: "t3".into(),
        status: TurnStatus::Working,
        event: Some(HookEvent::PostToolUse),
        session_id: Some("s1".into()),
        transcript_path: Some("/t.jsonl".into()),
        harness: Some(AgentHarness::Codex),
        files: vec!["/r/a.rs".into()],
    }
}

fn transports() -> Vec<Transport> {
    let mut all = vec![Transport::Tcp];
    #[cfg(unix)]
    all.push(Transport::Unix);
    all
}

#[test]
fn status_chega_tipado_em_todos_os_transportes() {
    for t in transports() {
        let fx = Fixture::start(t);
        let msg: serde_json::Value = serde_json::from_str(FULL).unwrap();
        assert_eq!(
            fx.send(&format!("{}\n", fx.authed(msg))),
            "",
            "status não tem resposta"
        );
        assert_eq!(fx.status(), full_update(), "{t:?}");
        fx.assert_quiet();
    }
}

#[test]
fn linha_sem_quebra_final_vale() {
    for t in transports() {
        let fx = Fixture::start(t);
        let msg: serde_json::Value = serde_json::from_str(FULL).unwrap();
        // O hook escreve a linha e fecha, sem `\n`.
        let mut conn = Conn::connect(&fx.server);
        conn.write_all(fx.authed(msg).as_bytes()).unwrap();
        conn.close_write();
        assert_eq!(fx.status().pane_id, "t3", "{t:?}");
    }
}

#[test]
fn ignorados_somem_e_malformada_vira_aviso_com_trecho() {
    let fx = Fixture::start(Transport::native());
    // (no TCP a linha malformada não chega a ter token; ela é erro antes disso)
    for line in ["\n", "   \n", "[1,2]\n", "7\n"] {
        assert_eq!(fx.send(line), "");
    }
    for line in [
        serde_json::json!({"st": "idle"}),
        serde_json::json!({"paneId": "t"}),
    ] {
        assert_eq!(fx.send(&format!("{}\n", fx.authed(line))), "");
    }
    fx.assert_quiet();

    let long = format!("{{{}\n", "x".repeat(300));
    assert_eq!(fx.send(&long), "");
    match fx.rx.recv_timeout(WAIT).unwrap() {
        Delivery::Malformed(e) => {
            assert_eq!(e.sample.chars().count(), 201);
            assert!(e.sample.starts_with("{xxx") && e.sample.ends_with('…'));
        }
        other => panic!("esperava Malformed: {other:?}"),
    }
    // O servidor segue de pé depois do lixo.
    fx.send(&format!(
        "{}\n",
        fx.authed(serde_json::json!({"paneId": "ok", "st": "idle"}))
    ));
    assert_eq!(fx.status().pane_id, "ok");
}

#[test]
fn comando_valido_recebe_commands_unavailable_byte_a_byte() {
    let fx = Fixture::start(Transport::native());
    let cmd = fx.authed(wire::cmd_request("list-panes", None, None));
    assert_eq!(
        fx.send(&format!("{cmd}\n")),
        "{\"ok\":false,\"error\":\"commands unavailable\"}\n"
    );
    // O cliente de comando espera a resposta com a conexão aberta: ela chega
    // sem ele fechar a escrita.
    let mut conn = Conn::connect(&fx.server);
    conn.write_all(format!("{cmd}\n").as_bytes()).unwrap();
    let mut got = [0u8; 64];
    let n = conn.read(&mut got).unwrap();
    assert!(got.get(..n).unwrap().ends_with(b"\n"));
    fx.assert_quiet();
}

#[cfg(unix)]
#[test]
fn socket_orfao_de_execucao_anterior_e_refeito() {
    let home = unique_dir("gi");
    let config = config(&home, Transport::Unix);
    let dir = home.join(".gsh");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file_name(config.debug, "sock"));
    // App que caiu sem limpar: o arquivo do socket fica e recusa conexão.
    drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
    assert!(path.exists());
    assert!(std::os::unix::net::UnixStream::connect(&path).is_err());

    let (server, rx) = StatusServer::start(&config).unwrap();
    let mut c = std::os::unix::net::UnixStream::connect(&path).unwrap();
    c.write_all(b"{\"paneId\":\"t\",\"st\":\"idle\"}\n")
        .unwrap();
    assert!(matches!(
        rx.recv_timeout(WAIT).unwrap(),
        Delivery::Status(_)
    ));
    drop(server);
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn arquivo_comum_no_caminho_nao_e_apagado() {
    let home = unique_dir("gi");
    let config = config(&home, Transport::Unix);
    let dir = home.join(".gsh");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(file_name(config.debug, "sock"));
    std::fs::write(&path, "meu").unwrap();
    let err = StatusServer::start(&config).unwrap_err();
    assert!(matches!(err, ServerError::PathOccupied { .. }), "{err}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "meu");
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn socket_unix_e_so_do_dono_e_o_caminho_bate_com_o_cliente() {
    use std::os::unix::fs::PermissionsExt;
    for debug in [false, true] {
        let home = unique_dir("gi");
        let mut config = config(&home, Transport::Unix);
        config.debug = debug;
        let (server, _rx) = StatusServer::start(&config).unwrap();
        let Endpoint::Unix(path) = server.endpoint().clone() else {
            panic!("esperava Unix");
        };
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        // A CLI procura o do seu flavor primeiro: tem que ser este.
        let candidates =
            wire::candidate_sockets(None, Some(home.to_string_lossy().into_owned()), debug);
        assert_eq!(candidates.first().map(String::as_str), path.to_str());
        assert_eq!(
            server.hook_env(),
            vec![(wire::ENV_STATUS_SOCK, path.to_string_lossy().into_owned())]
        );
        drop(server);
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn parar_apaga_o_arquivo_e_fecha_o_canal() {
    for t in transports() {
        let home = unique_dir("gi");
        let config = config(&home, t);
        let ext = if t == Transport::Tcp { "json" } else { "sock" };
        let file = home.join(".gsh").join(file_name(config.debug, ext));
        let (server, rx) = StatusServer::start(&config).unwrap();
        assert!(file.exists(), "{t:?}");
        server.stop();
        assert!(!file.exists(), "{t:?}: {} sobrou", file.display());
        assert!(matches!(
            rx.recv_timeout(WAIT),
            Err(RecvTimeoutError::Disconnected)
        ));
        let _ = std::fs::remove_dir_all(&home);
    }
}

#[test]
fn tcp_exige_token_e_anuncia_o_endpoint() {
    let fx = Fixture::start(Transport::Tcp);
    let tok = fx.token().unwrap();
    assert_eq!(tok.len(), 32);
    let Endpoint::Tcp { port } = *fx.server.endpoint() else {
        panic!("esperava Tcp");
    };
    assert_eq!(
        fx.server.hook_env(),
        vec![
            (wire::ENV_STATUS_PORT, port.to_string()),
            (wire::ENV_STATUS_TOKEN, tok.clone())
        ]
    );
    // Só no loopback.
    let peer = TcpStream::connect(("127.0.0.1", port)).unwrap();
    assert!(peer.peer_addr().unwrap().ip().is_loopback());

    // Arquivo de endpoint: o que a CLI de fora lê no Windows.
    assert!(fx.server.endpoint_file_error().is_none());
    let file = fx
        .home
        .join(".gsh")
        .join(file_name(cfg!(debug_assertions), "json"));
    let raw = std::fs::read_to_string(&file).unwrap();
    assert_eq!(
        wire::parse_endpoint_file(&raw),
        Some((port, Some(tok.clone())))
    );

    // Sem token, token errado, token que não é string: status some.
    for bad in [
        r#"{"paneId":"x","st":"idle"}"#.to_string(),
        r#"{"paneId":"x","st":"idle","tok":"errado"}"#.to_string(),
        r#"{"paneId":"x","st":"idle","tok":5}"#.to_string(),
        format!(r#"{{"paneId":"x","st":"idle","tok":"{tok}0"}}"#),
    ] {
        assert_eq!(fx.send(&format!("{bad}\n")), "", "{bad}");
    }
    fx.assert_quiet();

    // Comando com token ruim: resposta do app original, byte a byte.
    assert_eq!(
        fx.send("{\"type\":\"cmd\",\"cmd\":\"list-panes\"}\n"),
        "{\"ok\":false,\"error\":\"invalid token\"}\n"
    );
    // Com o token certo chega na camada de comandos.
    let cmd = wire::with_token(wire::cmd_request("list-panes", None, None), &tok);
    assert_eq!(
        fx.send(&format!("{cmd}\n")),
        "{\"ok\":false,\"error\":\"commands unavailable\"}\n"
    );

    // E o status com o token certo entra.
    fx.send(&format!(
        "{}\n",
        fx.authed(serde_json::json!({"paneId": "ok", "st": "waiting"}))
    ));
    let u = fx.status();
    assert_eq!((u.pane_id.as_str(), u.status), ("ok", TurnStatus::Waiting));
    fx.assert_quiet();
}

#[test]
fn muitas_conexoes_simultaneas_chegam_todas() {
    for t in transports() {
        let fx = Fixture::start(t);
        let n = 40;
        let lines: Vec<String> = (0..n)
            .map(|i| {
                let msg = serde_json::json!({"paneId": format!("p{i}"), "st": "working"});
                format!("{}\n", fx.authed(msg))
            })
            .collect();
        std::thread::scope(|s| {
            for line in &lines {
                let server = &fx.server;
                s.spawn(move || send_to(server, line));
            }
        });
        let mut got: Vec<String> = (0..n).map(|_| fx.status().pane_id).collect();
        got.sort();
        let mut want: Vec<String> = (0..n).map(|i| format!("p{i}")).collect();
        want.sort();
        assert_eq!(got, want, "{t:?}");
    }
}

#[test]
fn flavors_dividem_o_gsh_dir_sem_se_apagar() {
    // Dev e instalado rodam lado a lado sobre o mesmo `~/.gsh`.
    let home = unique_dir("gi");
    let mk = |debug: bool| {
        let mut c = config(&home, Transport::native());
        c.debug = debug;
        c
    };
    let (a, _ra) = StatusServer::start(&mk(false)).unwrap();
    let (b, rb) = StatusServer::start(&mk(true)).unwrap();
    assert_ne!(a.endpoint(), b.endpoint());
    drop(a);
    // Derrubar um não derruba o do outro.
    let mut c = Conn::connect(&b);
    let tok = b
        .hook_env()
        .into_iter()
        .find(|(k, _)| *k == wire::ENV_STATUS_TOKEN)
        .map(|(_, v)| v);
    let msg = serde_json::json!({"paneId": "dev", "st": "idle"});
    let line = match tok {
        Some(tok) => wire::with_token(msg, &tok),
        None => msg,
    };
    c.write_all(format!("{line}\n").as_bytes()).unwrap();
    assert!(matches!(
        rb.recv_timeout(WAIT).unwrap(),
        Delivery::Status(_)
    ));
    drop(c);
    drop(b);
    let _ = std::fs::remove_dir_all(&home);
}

#[cfg(unix)]
#[test]
fn parar_nao_trava_se_o_socket_foi_apagado_por_fora() {
    // Outra instância do mesmo flavor apaga e refaz o arquivo: o nosso `stop`
    // não pode depender do caminho para terminar.
    let home = unique_dir("gi");
    let (server, _rx) = StatusServer::start(&config(&home, Transport::Unix)).unwrap();
    let Endpoint::Unix(path) = server.endpoint().clone() else {
        panic!("esperava Unix");
    };
    std::fs::remove_file(&path).unwrap();
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        server.stop();
        let _ = done_tx.send(());
    });
    done_rx.recv_timeout(WAIT).expect("stop travou");
    let _ = std::fs::remove_dir_all(&home);
}
