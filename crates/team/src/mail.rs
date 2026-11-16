//! Correio por TCP direto na rede local (`LanTeamMail`): cada GSH escuta numa porta livre
//! (anunciada no pacote da fazenda) e quem manda abre uma conexao, escreve uma linha JSON e
//! le uma linha de resposta.
//!
//! Linha: `{"e": <corpo JSON cifrado com a senha do time, base64>}`. O corpo leva a hora e o
//! id da batida: hora fora de +-5 min ou id repetido nao entram (reenvio de pacote capturado).

use crate::config::TeamConfig;
use crate::crypto::{self, derive_key, TeamKey};
use crate::error::TeamError;
use crate::knock::{decode_knock, encode_knock, Knock};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use std::collections::{HashSet, VecDeque};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Teto da linha (o arquivo vai em base64 dentro dela).
const MAX_BYTES: u64 = 25 * 1024 * 1024;
const SKEW_MS: u64 = 5 * 60 * 1000;
const SEEN_CAP: usize = 2000;
const IO_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
/// Conexoes simultaneas aceitas (cada uma pode trazer ate 25 MiB): mais que isso e recusado.
const MAX_CONNECTIONS: usize = 16;

/// Por que `send` nao entregou; o texto sai na CLI do agente que mandou (ingles, estavel).
#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error("team mail is off (set the team password)")]
    MailOff,
    #[error("file too large")]
    TooLarge,
    #[error("refused: {error}")]
    Refused { error: String },
    #[error("could not reach {address}:{port} ({detail})")]
    Unreachable {
        address: IpAddr,
        port: u16,
        detail: String,
    },
    #[error(transparent)]
    Seal(#[from] TeamError),
}

/// Batidas ja vistas (reenvio de pacote capturado): as `SEEN_CAP` mais recentes.
#[derive(Default)]
struct Seen {
    order: VecDeque<String>,
    set: HashSet<String>,
}

impl Seen {
    /// `false` = ja tinha visto.
    fn add(&mut self, id: &str) -> bool {
        if !self.set.insert(id.to_owned()) {
            return false;
        }
        self.order.push_back(id.to_owned());
        if self.order.len() > SEEN_CAP {
            if let Some(old) = self.order.pop_front() {
                self.set.remove(&old);
            }
        }
        true
    }
}

fn reply(error: Option<&str>) -> String {
    match error {
        None => json!({"ok": true}),
        Some(e) => json!({"ok": false, "error": e}),
    }
    .to_string()
}

/// Uma linha recebida -> batida aceita (ou o motivo da recusa, que volta para quem mandou).
fn accept_line(
    line: &str,
    key: &TeamKey,
    now_ms: i64,
    seen: &mut Seen,
) -> Result<Knock, &'static str> {
    let Ok(Value::Object(o)) = serde_json::from_str::<Value>(line) else {
        return Err("bad request");
    };
    let Some(Value::String(e)) = o.get("e") else {
        return Err("bad request");
    };
    let sealed = STANDARD.decode(e).map_err(|_| "bad request")?;
    let plain = crypto::open(key, &sealed).ok_or("wrong team password")?;
    let body = String::from_utf8(plain).map_err(|_| "bad request")?;
    let knock = decode_knock(&body, now_ms, SKEW_MS).ok_or("stale or invalid knock")?;
    if !seen.add(&knock.id) {
        return Err("duplicate knock");
    }
    Ok(knock)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

/// Le uma linha de no maximo `MAX_BYTES`; `None` = estourou, deu erro ou timeout.
fn read_line(stream: &TcpStream) -> Option<String> {
    let mut buf = Vec::new();
    BufReader::new(stream.take(MAX_BYTES + 1))
        .read_until(b'\n', &mut buf)
        .ok()?;
    if buf.last() == Some(&b'\n') {
        buf.pop();
    }
    if buf.len() as u64 > MAX_BYTES {
        return None;
    }
    String::from_utf8(buf).ok()
}

pub struct Mail {
    port: u16,
    knocks: Receiver<Knock>,
    stop: Arc<AtomicBool>,
}

impl Mail {
    /// Liga o correio numa porta livre. `None` = sem senha do time (correio desligado).
    pub fn start(config: &TeamConfig) -> Result<Option<Self>, TeamError> {
        if !config.mail_active() {
            return Ok(None);
        }
        let listener =
            TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).map_err(TeamError::io("bind"))?;
        let port = listener
            .local_addr()
            .map_err(TeamError::io("local addr"))?
            .port();
        let (tx, knocks) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker = Server {
            team: config.team.trim().to_owned(),
            secret: config.secret.clone(),
            tx,
            stop: Arc::clone(&stop),
        };
        thread::Builder::new()
            .name("gsh-team-mail".into())
            .spawn(move || worker.run(&listener))
            .map_err(TeamError::io("spawn"))?;
        Ok(Some(Self { port, knocks, stop }))
    }

    /// Porta TCP em que este GSH recebe (vai no `mp` do pacote da fazenda).
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Proxima batida (assinatura e reenvio ja conferidos).
    pub fn try_recv(&self) -> Option<Knock> {
        self.knocks.try_recv().ok()
    }

    /// Entrega `knock` em `address:port`. Bloqueia (ate ~35 s): chame de uma thread.
    pub fn send(
        config: &TeamConfig,
        address: IpAddr,
        port: u16,
        knock: &Knock,
    ) -> Result<(), SendError> {
        if !config.mail_active() {
            return Err(SendError::MailOff);
        }
        let key = derive_key(&config.team, &config.secret);
        let body = encode_knock(knock, &config.team, now_ms());
        let line = json!({"e": STANDARD.encode(crypto::seal(&key, body.as_bytes())?)}).to_string();
        if line.len() as u64 > MAX_BYTES {
            return Err(SendError::TooLarge);
        }
        let unreachable = |e: std::io::Error| SendError::Unreachable {
            address,
            port,
            detail: e.to_string(),
        };
        let mut stream =
            TcpStream::connect_timeout(&SocketAddr::new(address, port), CONNECT_TIMEOUT)
                .map_err(unreachable)?;
        stream
            .set_read_timeout(Some(IO_TIMEOUT))
            .map_err(unreachable)?;
        stream
            .set_write_timeout(Some(IO_TIMEOUT))
            .map_err(unreachable)?;
        stream
            .write_all(line.as_bytes())
            .and_then(|()| stream.write_all(b"\n"))
            .and_then(|()| stream.flush())
            .map_err(unreachable)?;
        let answer = read_line(&stream).unwrap_or_default();
        match serde_json::from_str::<Value>(&answer) {
            Ok(Value::Object(r)) if r.get("ok") == Some(&Value::Bool(true)) => Ok(()),
            Ok(Value::Object(r)) => Err(SendError::Refused {
                error: match r.get("error") {
                    Some(Value::String(s)) => s.clone(),
                    other => other.map_or_else(|| "null".into(), Value::to_string),
                },
            }),
            _ => Err(SendError::Refused { error: answer }),
        }
    }
}

impl Drop for Mail {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // acorda o accept() bloqueado para a thread ver a flag e sair
        let _ = TcpStream::connect_timeout(
            &SocketAddr::new(Ipv4Addr::LOCALHOST.into(), self.port),
            CONNECT_TIMEOUT,
        );
    }
}

struct Server {
    team: String,
    secret: String,
    tx: Sender<Knock>,
    stop: Arc<AtomicBool>,
}

impl Server {
    fn run(self, listener: &TcpListener) {
        let key = derive_key(&self.team, &self.secret);
        let seen = Arc::new(Mutex::new(Seen::default()));
        let active = Arc::new(AtomicUsize::new(0));
        for conn in listener.incoming() {
            if self.stop.load(Ordering::Relaxed) {
                break;
            }
            let Ok(stream) = conn else { continue };
            if active.fetch_add(1, Ordering::AcqRel) >= MAX_CONNECTIONS {
                active.fetch_sub(1, Ordering::AcqRel);
                continue; // sobrecarga: derruba a conexao
            }
            let (key, seen, tx, active) = (
                key.clone(),
                Arc::clone(&seen),
                self.tx.clone(),
                Arc::clone(&active),
            );
            let spawned = thread::Builder::new().name("gsh-team-conn".into()).spawn({
                let active = Arc::clone(&active);
                move || {
                    serve(stream, &key, &seen, &tx);
                    active.fetch_sub(1, Ordering::AcqRel);
                }
            });
            if spawned.is_err() {
                active.fetch_sub(1, Ordering::AcqRel);
            }
        }
    }
}

fn serve(mut stream: TcpStream, key: &TeamKey, seen: &Mutex<Seen>, tx: &Sender<Knock>) {
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let answer = match read_line(&stream) {
        None => reply(Some("bad request")),
        Some(line) => {
            let mut seen = seen.lock().unwrap_or_else(PoisonError::into_inner);
            match accept_line(&line, key, now_ms(), &mut seen) {
                Ok(knock) => {
                    // ninguem le mais = o app fechou; a resposta ainda vai
                    let _ = tx.send(knock);
                    reply(None)
                }
                Err(why) => reply(Some(why)),
            }
        }
    };
    let _ = stream.write_all(answer.as_bytes());
    let _ = stream.write_all(b"\n");
    let _ = stream.flush();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::knock::KnockKind;

    fn cfg(secret: &str) -> TeamConfig {
        TeamConfig {
            enabled: true,
            team: "gorila".into(),
            secret: secret.into(),
            ..TeamConfig::default()
        }
    }

    fn knock(id: &str) -> Knock {
        Knock {
            id: id.into(),
            kind: KnockKind::Message,
            from_machine: "m1".into(),
            from_name: "Ana".into(),
            from_agent: "claude".into(),
            from_session_id: None,
            to_tab_id: "t1".into(),
            text: "oi".into(),
            file_name: None,
            file_bytes: None,
        }
    }

    fn wait_for(mail: &Mail) -> Option<Knock> {
        for _ in 0..100 {
            if let Some(k) = mail.try_recv() {
                return Some(k);
            }
            thread::sleep(Duration::from_millis(50));
        }
        None
    }

    fn local() -> IpAddr {
        Ipv4Addr::LOCALHOST.into()
    }

    #[test]
    fn sem_senha_o_correio_nao_liga() {
        assert!(Mail::start(&cfg("")).unwrap().is_none());
    }

    #[test]
    fn entrega_recusa_senha_errada_e_reenvio() {
        let mail = Mail::start(&cfg("segredo")).unwrap().unwrap();
        let port = mail.port();
        let mut k = knock("k1");
        k.file_bytes = Some(vec![0, 1, 255]);
        k.kind = KnockKind::File;
        Mail::send(&cfg("segredo"), local(), port, &k).unwrap();
        assert_eq!(wait_for(&mail), Some(k.clone()));
        // mesmo id de novo: reenvio
        let dup = Mail::send(&cfg("segredo"), local(), port, &k).unwrap_err();
        assert_eq!(dup.to_string(), "refused: duplicate knock");
        // outra senha
        let wrong = Mail::send(&cfg("outra"), local(), port, &knock("k2")).unwrap_err();
        assert_eq!(wrong.to_string(), "refused: wrong team password");
        assert!(mail.try_recv().is_none());
        // quem manda sem senha nem tenta
        let off = Mail::send(&cfg(""), local(), port, &knock("k3")).unwrap_err();
        assert!(matches!(off, SendError::MailOff));
    }

    #[test]
    fn linha_ruim_e_batida_velha_sao_recusadas_sem_panic() {
        let key = derive_key("gorila", "segredo");
        let mut seen = Seen::default();
        let now = 1_790_000_000_000;
        let e = |body: &str| {
            let sealed = crypto::seal(&key, body.as_bytes()).unwrap();
            json!({"e": STANDARD.encode(sealed)}).to_string()
        };
        for line in [
            "",
            "x",
            "[]",
            "{}",
            r#"{"e":5}"#,
            r#"{"e":"@@@"}"#,
            r#"{"e":"AAAA"}"#,
        ] {
            assert!(accept_line(line, &key, now, &mut seen).is_err(), "{line}");
        }
        let old = encode_knock(&knock("old"), "gorila", now - 301_000);
        assert_eq!(
            accept_line(&e(&old), &key, now, &mut seen).unwrap_err(),
            "stale or invalid knock"
        );
        let good = encode_knock(&knock("ok"), "gorila", now);
        assert!(accept_line(&e(&good), &key, now, &mut seen).is_ok());
        assert_eq!(
            accept_line(&e(&good), &key, now, &mut seen).unwrap_err(),
            "duplicate knock"
        );
    }

    #[test]
    fn lista_de_vistos_e_limitada() {
        let mut seen = Seen::default();
        for i in 0..SEEN_CAP + 10 {
            assert!(seen.add(&i.to_string()));
        }
        assert_eq!(seen.set.len(), SEEN_CAP);
        assert!(seen.add("0"), "o mais antigo ja saiu da lista");
        assert!(!seen.add(&(SEEN_CAP + 5).to_string()));
    }

    #[test]
    fn porta_fechada_vira_erro_de_alcance() {
        let port = {
            let l = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
            l.local_addr().unwrap().port()
        };
        let err = Mail::send(&cfg("segredo"), local(), port, &knock("k")).unwrap_err();
        assert!(
            err.to_string().starts_with("could not reach 127.0.0.1:"),
            "{err}"
        );
    }
}
