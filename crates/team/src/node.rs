//! O time visto do app: abre/fecha o canal conforme a config, anuncia os loops locais a cada
//! 2 s (so enquanto ativo) e guarda os colegas que estao na rede (`FarmViewModel` do Dart).

use crate::channel::Channel;
use crate::config::TeamConfig;
use crate::error::TeamError;
use crate::network::allowed_network;
use crate::packet::{FarmLoop, Packet};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Colega sem pacote ha mais que isso sai da tela (fechou o GSH, saiu da rede).
pub const PEER_TIMEOUT: Duration = Duration::from_secs(10);
/// O Dart anuncia a cada varredura da fazenda.
pub const PUBLISH_EVERY: Duration = Duration::from_secs(2);

/// Um colega do time na rede agora (`FarmPeer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Peer {
    pub id: String,
    pub name: String,
    pub loops: Vec<FarmLoop>,
    pub agents: Vec<crate::packet::TeamAgent>,
    pub mail_port: Option<u16>,
    pub address: Option<std::net::IpAddr>,
}

#[derive(Default)]
pub struct Node {
    config: TeamConfig,
    channel: Option<Channel>,
    peers: HashMap<String, (Packet, Instant)>,
    last_publish: Option<Instant>,
}

/// O que sai daqui: so o nome da pasta (nunca o caminho) e, sem `show_titles`, o titulo vira o
/// nome da pasta tambem (`_outgoing` do Dart).
pub fn outgoing(machine_id: &str, config: &TeamConfig, loops: &[FarmLoop]) -> Packet {
    let base = |p: &str| {
        std::path::Path::new(p)
            .file_name()
            .and_then(|n| n.to_str())
            .map_or_else(|| p.to_owned(), str::to_owned)
    };
    Packet {
        machine_id: machine_id.to_owned(),
        name: config.name.trim().to_owned(),
        team: config.team.trim().to_owned(),
        loops: loops
            .iter()
            .map(|l| FarmLoop {
                project: base(&l.project),
                title: if config.show_titles {
                    l.title.clone()
                } else {
                    base(&l.project)
                },
                ..l.clone()
            })
            .collect(),
        // correio fica de fora enquanto o app nao entrega batida ao agente
        agents: Vec::new(),
        mail_port: None,
        address: None,
    }
}

impl Node {
    pub fn machine_id(&self) -> Option<&str> {
        self.channel.as_ref().map(Channel::machine_id)
    }
    pub fn config(&self) -> &TeamConfig {
        &self.config
    }

    /// Aplica as preferencias. Devolve `Err` se a rede nao abriu (a config fica aplicada e o
    /// canal fechado). `true` = a config mudou (a UI deve redesenhar).
    pub fn configure(&mut self, config: TeamConfig) -> Result<bool, TeamError> {
        if config == self.config && (self.channel.is_some() || !config.active()) {
            return Ok(false);
        }
        self.config = config;
        self.channel = None;
        self.last_publish = None;
        if !self.config.active() || !allowed_network(&self.config.network) {
            self.peers.clear();
            return Ok(true);
        }
        self.channel = Some(Channel::open(&self.config)?);
        Ok(true)
    }

    /// Um ciclo: le o que chegou, anuncia `local` se ja passou `PUBLISH_EVERY` e tira quem
    /// sumiu. `true` = a lista de colegas mudou.
    pub fn poll(&mut self, now: Instant, local: &[FarmLoop]) -> bool {
        let Some(channel) = &self.channel else {
            return false;
        };
        let packet = outgoing(channel.machine_id(), &self.config, local);
        self.poll_packet(now, packet)
    }

    /// Variante para o app que também anuncia abas alcançáveis pelo correio TCP.
    pub fn poll_packet(&mut self, now: Instant, mut packet: Packet) -> bool {
        let Some(channel) = &self.channel else {
            return false;
        };
        let mut changed = false;
        while let Some(p) = channel.try_recv() {
            let same = self
                .peers
                .get(&p.machine_id)
                .is_some_and(|(old, _)| *old == p);
            changed |= !same;
            self.peers.insert(p.machine_id.clone(), (p, now));
        }
        if self
            .last_publish
            .is_none_or(|t| now.duration_since(t) >= PUBLISH_EVERY)
        {
            self.last_publish = Some(now);
            packet.machine_id = channel.machine_id().to_owned();
            packet.name.clone_from(&self.config.name.trim().to_owned());
            packet.team = self.config.team.trim().to_owned();
            packet.loops = outgoing(channel.machine_id(), &self.config, &packet.loops).loops;
            packet.address = None;
            channel.publish(packet);
        }
        let before = self.peers.len();
        self.peers
            .retain(|_, (_, at)| now.duration_since(*at) <= PEER_TIMEOUT);
        changed || self.peers.len() != before
    }

    /// Colegas por nome (sem diferenca de maiuscula).
    pub fn peers(&self) -> Vec<Peer> {
        let mut out: Vec<Peer> = self
            .peers
            .values()
            .map(|(p, _)| Peer {
                id: p.machine_id.clone(),
                name: p.name.clone(),
                loops: p.loops.clone(),
                agents: p.agents.clone(),
                mail_port: p.mail_port.and_then(|port| u16::try_from(port).ok()),
                address: p.address,
            })
            .collect();
        out.sort_by_key(|p| p.name.to_lowercase());
        out
    }

    /// O canal esta aberto (so por teste e para a UI saber se vale mandar pacote).
    pub fn is_open(&self) -> bool {
        self.channel.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::{Activity, Packet, TeamAgent};
    use std::thread;

    fn lp(project: &str, title: &str) -> FarmLoop {
        FarmLoop {
            id: "s1".into(),
            parent_id: None,
            teammate: None,
            harness: "claude".into(),
            project: project.into(),
            title: title.into(),
            agent_type: None,
            activity: Activity::Thinking,
            tool: None,
            output_tokens: 0,
            context_tokens: 0,
            input_tokens: 0,
            cache_read_tokens: 0,
            context_window: 0,
            last_seen_ms: 0,
        }
    }

    fn cfg(show_titles: bool) -> TeamConfig {
        TeamConfig {
            enabled: true,
            team: " gorila ".into(),
            name: " Ana ".into(),
            show_titles,
            ..TeamConfig::default()
        }
    }

    /// Privacidade: caminho nunca sai (so o nome da pasta) e sem `show_titles` o titulo some.
    #[test]
    fn saida_so_leva_o_nome_da_pasta() {
        let l = [lp("/home/ana/projetos/segredo", "prompt: senha=123")];
        let hidden = outgoing("m1", &cfg(false), &l);
        assert_eq!(
            (hidden.name.as_str(), hidden.team.as_str()),
            ("Ana", "gorila")
        );
        let first = hidden.loops.first().unwrap();
        assert_eq!(first.project, "segredo");
        assert_eq!(first.title, "segredo");
        let shown = outgoing("m1", &cfg(true), &l);
        assert_eq!(shown.loops.first().unwrap().title, "prompt: senha=123");
        assert_eq!(shown.loops.first().unwrap().project, "segredo");
        assert!(shown.agents.is_empty() && shown.mail_port.is_none());
    }

    #[test]
    fn desligado_ou_sem_time_nao_abre_canal() {
        let mut n = Node::default();
        assert!(!n.configure(TeamConfig::default()).unwrap());
        assert!(!n.is_open());
        n.configure(TeamConfig {
            enabled: true,
            ..TeamConfig::default()
        })
        .unwrap();
        assert!(!n.is_open(), "sem nome de time nao ativa");
        assert!(!n.poll(Instant::now(), &[]));
    }

    #[test]
    fn rede_do_trabalho_fora_da_faixa_fecha_o_canal() {
        let mut n = Node::default();
        n.configure(TeamConfig {
            network: "203.0.113.0/24".into(), // TEST-NET-3: nenhuma interface daqui
            ..cfg(false)
        })
        .unwrap();
        assert!(!n.is_open());
    }

    /// Dois nos na mesma maquina: cada um anuncia e enxerga o outro; sair da rede expira.
    #[test]
    fn dois_nos_se_veem_e_o_que_some_expira() {
        // a porta do canal e a de sempre (47183); so roda se nao houver outro GSH usando
        // o grupo com outro time: o filtro por time/senha ja isola, entao e seguro.
        let team = format!("loopback-{}", std::process::id());
        let mk = |name: &str| TeamConfig {
            enabled: true,
            team: team.clone(),
            name: name.into(),
            secret: "segredo".into(),
            ..TeamConfig::default()
        };
        let (mut a, mut b) = (Node::default(), Node::default());
        a.configure(mk("Ana")).unwrap();
        b.configure(mk("Beto")).unwrap();
        let mine = [lp("/x/gsh", "t")];
        let packet = Packet {
            machine_id: String::new(),
            name: "Beto".into(),
            team: team.clone(),
            loops: Vec::new(),
            agents: vec![TeamAgent {
                tab_id: "t4".into(),
                label: "Terminal · t4".into(),
                harness: "claude".into(),
                project: "gsh".into(),
                session_id: Some("session-4".into()),
            }],
            mail_port: Some(40123),
            address: None,
        };
        let start = Instant::now();
        let mut now = start;
        let mut seen = false;
        for _ in 0..80 {
            a.poll(now, &mine);
            b.poll_packet(now, packet.clone());
            if a.peers().iter().any(|p| p.name == "Beto")
                && b.peers().iter().any(|p| p.name == "Ana")
            {
                seen = true;
                break;
            }
            thread::sleep(Duration::from_millis(150));
            now += PUBLISH_EVERY; // o relogio do no anda a passo de publicacao
        }
        assert!(seen, "os dois nos nao se enxergaram");
        let beto = a.peers().into_iter().find(|p| p.name == "Beto").unwrap();
        assert_eq!(beto.agents.first().map(|a| a.tab_id.as_str()), Some("t4"));
        assert_eq!(beto.mail_port, Some(40123));
        let ana = b.peers().into_iter().find(|p| p.name == "Ana").unwrap();
        assert_eq!(ana.loops.first().map(|l| l.project.as_str()), Some("gsh"));
        // b para de ouvir: depois de PEER_TIMEOUT sem pacote, a some da lista de b
        drop(a);
        let later = now + PEER_TIMEOUT + Duration::from_secs(1);
        assert!(b.poll(later, &[]));
        assert!(b.peers().is_empty());
    }
}
