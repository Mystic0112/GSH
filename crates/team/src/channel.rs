//! Canal da fazenda: multicast UDP `239.255.71.83:47183`. Cada GSH manda o proprio pacote
//! para o grupo e escuta o mesmo grupo (`LanTeamFarmChannel`). Uma thread dona do socket faz
//! tudo (derivar a chave, enviar, receber, decodificar): a UI so enfileira e le com `try_recv`.

use crate::config::TeamConfig;
use crate::crypto::{derive_key, TeamKey};
use crate::error::TeamError;
use crate::packet::{decode_packet, encode_packet, Packet};
use socket2::{Domain, Protocol, Socket, Type};
use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub const GROUP: Ipv4Addr = Ipv4Addr::new(239, 255, 71, 83);
pub const PORT: u16 = 47183;

/// De quanto em quanto a thread olha a fila de envio e a flag de parada.
const TICK: Duration = Duration::from_millis(100);
/// Maior datagrama UDP IPv4 com payload.
const MAX_DATAGRAM: usize = 65_507;

pub struct Channel {
    machine_id: String,
    publish: Sender<Packet>,
    packets: Receiver<Packet>,
    stop: Arc<AtomicBool>,
}

impl Channel {
    /// Abre o canal do grupo e porta de sempre.
    pub fn open(config: &TeamConfig) -> Result<Self, TeamError> {
        Self::open_at(config, GROUP, PORT)
    }

    /// Com grupo/porta explicitos (o teste de loopback usa uma porta propria).
    pub fn open_at(config: &TeamConfig, group: Ipv4Addr, port: u16) -> Result<Self, TeamError> {
        let socket = bind(group, port)?;
        let machine_id = random_id()?;
        let (publish, publish_rx) = channel();
        let (packets_tx, packets) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let worker = Worker {
            socket,
            group: SocketAddr::V4(SocketAddrV4::new(group, port)),
            team: config.team.trim().to_owned(),
            secret: config.secret.clone(),
            machine_id: machine_id.clone(),
            publish: publish_rx,
            packets: packets_tx,
            stop: Arc::clone(&stop),
        };
        thread::Builder::new()
            .name("gsh-team-udp".into())
            .spawn(move || worker.run())
            .map_err(TeamError::io("spawn"))?;
        Ok(Self {
            machine_id,
            publish,
            packets,
            stop,
        })
    }

    /// Identifica esta instancia (descarta o eco do proprio pacote).
    pub fn machine_id(&self) -> &str {
        &self.machine_id
    }

    /// Enfileira o pacote; a thread cifra e envia.
    pub fn publish(&self, packet: Packet) {
        // canal fechado = thread morreu (socket com erro); nao ha o que fazer aqui
        let _ = self.publish.send(packet);
    }

    /// Proximo pacote de outro GSH do mesmo time (o eco ja vem filtrado).
    pub fn try_recv(&self) -> Option<Packet> {
        self.packets.try_recv().ok()
    }
}

impl Drop for Channel {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// `RawDatagramSocket.bind(anyIPv4, port, reuseAddress, reusePort)` + `joinMulticast` +
/// `multicastLoopback`: dois GSHs na mesma maquina dividem a porta.
fn bind(group: Ipv4Addr, port: u16) -> Result<UdpSocket, TeamError> {
    let s = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))
        .map_err(TeamError::io("socket"))?;
    s.set_reuse_address(true)
        .map_err(TeamError::io("reuse address"))?;
    #[cfg(all(
        unix,
        not(any(target_os = "solaris", target_os = "illumos", target_os = "cygwin"))
    ))]
    s.set_reuse_port(true)
        .map_err(TeamError::io("reuse port"))?;
    s.bind(&SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port)).into())
        .map_err(TeamError::io("bind"))?;
    s.set_multicast_loop_v4(true)
        .map_err(TeamError::io("multicast loop"))?;
    s.join_multicast_v4(&group, &Ipv4Addr::UNSPECIFIED)
        .map_err(TeamError::io("join multicast"))?;
    let socket: UdpSocket = s.into();
    socket
        .set_read_timeout(Some(TICK))
        .map_err(TeamError::io("read timeout"))?;
    Ok(socket)
}

fn random_id() -> Result<String, TeamError> {
    let mut b = [0_u8; 8];
    getrandom::fill(&mut b).map_err(|e| TeamError::Random {
        detail: e.to_string(),
    })?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

struct Worker {
    socket: UdpSocket,
    group: SocketAddr,
    team: String,
    secret: String,
    machine_id: String,
    publish: Receiver<Packet>,
    packets: Sender<Packet>,
    stop: Arc<AtomicBool>,
}

impl Worker {
    fn run(self) {
        // PBKDF2 aqui, longe da UI; os datagramas esperam no buffer do socket
        let key: Option<TeamKey> =
            (!self.secret.is_empty()).then(|| derive_key(&self.team, &self.secret));
        let mut buf = vec![0_u8; 65_536];
        while !self.stop.load(Ordering::Relaxed) {
            while let Ok(p) = self.publish.try_recv() {
                self.send(&p, key.as_ref());
            }
            match self.socket.recv_from(&mut buf) {
                Ok((n, from)) => {
                    let Some(data) = buf.get(..n) else { continue };
                    let Some(p) = decode_packet(data, key.as_ref(), Some(from.ip())) else {
                        continue;
                    };
                    if p.machine_id == self.machine_id || p.team != self.team {
                        continue;
                    }
                    if self.packets.send(p).is_err() {
                        break; // ninguem mais le
                    }
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {}
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => thread::sleep(TICK), // erro de rede passageiro: nao gira em falso
            }
        }
    }

    fn send(&self, p: &Packet, key: Option<&TeamKey>) {
        let Ok(bytes) = encode_packet(p, key) else {
            return;
        };
        if bytes.len() <= MAX_DATAGRAM {
            // UDP: se a rede cair, o proximo ciclo manda de novo
            let _ = self.socket.send_to(&bytes, self.group);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{Activity, FarmLoop};
    use std::time::Instant;

    fn cfg(team: &str, secret: &str) -> TeamConfig {
        TeamConfig {
            enabled: true,
            team: team.into(),
            secret: secret.into(),
            ..TeamConfig::default()
        }
    }

    fn packet(id: &str, team: &str, name: &str) -> Packet {
        Packet {
            machine_id: id.into(),
            name: name.into(),
            team: team.into(),
            loops: vec![FarmLoop {
                id: "s1".into(),
                parent_id: None,
                teammate: None,
                harness: "claude".into(),
                project: "gsh".into(),
                title: "t".into(),
                agent_type: None,
                activity: Activity::Editing,
                tool: None,
                output_tokens: 0,
                context_tokens: 0,
                input_tokens: 0,
                cache_read_tokens: 0,
                context_window: 0,
                last_seen_ms: 1,
            }],
            agents: Vec::new(),
            mail_port: None,
            address: None,
        }
    }

    /// Porta propria do teste: nao colide com um GSH aberto na maquina (47183).
    fn test_port(offset: u16) -> u16 {
        40_000 + (std::process::id() % 20_000) as u16 + offset
    }

    /// Reenvia ate `ch` receber (o join no grupo e a thread nao sao instantaneos).
    fn recv_within(from: &Channel, to: &Channel, name: &str) -> Option<Packet> {
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            from.publish(packet(from.machine_id(), "gorila", name));
            thread::sleep(Duration::from_millis(150));
            if let Some(p) = to.try_recv() {
                return Some(p);
            }
        }
        None
    }

    /// Dois canais na mesma maquina (mesma porta, SO_REUSEPORT) se enxergam, cifrados.
    #[test]
    fn dois_canais_na_mesma_maquina_se_veem_cifrados() {
        let port = test_port(0);
        let a = Channel::open_at(&cfg("gorila", "segredo"), GROUP, port).unwrap();
        let b = Channel::open_at(&cfg("gorila", "segredo"), GROUP, port).unwrap();
        assert_ne!(a.machine_id(), b.machine_id());
        let got = recv_within(&a, &b, "Ana").expect("b nao recebeu o pacote de a");
        assert_eq!(got.machine_id, a.machine_id());
        assert_eq!(got.name, "Ana");
        assert_eq!(got.loops.len(), 1);
        assert!(
            got.address.is_some(),
            "o IP de origem e preenchido por quem recebe"
        );
        // a nao ve o proprio eco
        assert!(a.try_recv().is_none());
        let back = recv_within(&b, &a, "Beto").expect("a nao recebeu o pacote de b");
        assert_eq!(back.name, "Beto");
    }

    /// Outra senha, outro time ou pacote aberto: o canal nao entrega nada.
    #[test]
    fn senha_errada_time_errado_e_modo_aberto_nao_entregam() {
        let port = test_port(1);
        let a = Channel::open_at(&cfg("gorila", "segredo"), GROUP, port).unwrap();
        let wrong_pw = Channel::open_at(&cfg("gorila", "outra"), GROUP, port).unwrap();
        let wrong_team = Channel::open_at(&cfg("outros", "segredo"), GROUP, port).unwrap();
        let open_mode = Channel::open_at(&cfg("gorila", ""), GROUP, port).unwrap();
        let good = Channel::open_at(&cfg("gorila", "segredo"), GROUP, port).unwrap();
        // quando o bom recebeu, os outros ja tiveram a mesma chance
        recv_within(&a, &good, "Ana").expect("o canal certo nao recebeu");
        for ch in [&wrong_pw, &wrong_team, &open_mode] {
            assert!(ch.try_recv().is_none());
        }
    }
}
