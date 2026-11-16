//! A fazenda dos colegas do time ao lado da minha (`farm_team.dart` + `farm_view_team.dart`):
//! um [`Lot`] por colega, com a cena, o fundo pintado e a posição no mundo. O fundo de um
//! colega só é refeito quando a estrutura dele muda (mais canteiros do que cabem, o tamanho do
//! meu mundo ou o cenário), nunca a cada pacote.

use super::agents::{AgentInput, AgentState, Agents};
use super::backdrop::Backdrop;
use super::camera::TeamBounds;
use super::world::Mode;
use gsh_team::{Activity, FarmLoop, Peer};
use std::net::IpAddr;

/// O que a UI sabe de um colega: quem é e os agentes dele já no formato da cena.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInput {
    pub id: String,
    pub name: String,
    pub agents: Vec<AgentInput>,
    pub subagents: Vec<FarmLoop>,
    pub contacts: Vec<gsh_team::TeamAgent>,
    pub address: Option<IpAddr>,
    pub mail_port: Option<u16>,
}

impl PeerInput {
    /// Os loops do colega viram bichos: o principal e os teammates têm canteiro; subagente
    /// (tem dono e não tem nome) não — a cena do Rust ainda não desenha os minis.
    pub fn from_peer(peer: &Peer) -> Self {
        Self {
            id: peer.id.clone(),
            name: peer.name.clone(),
            agents: peer
                .loops
                .iter()
                .filter(|l| !(l.parent_id.is_some() && l.teammate.is_none()))
                .map(|l| AgentInput {
                    pane_id: l.id.clone(),
                    name: l.title.clone(),
                    state: state_of(l),
                })
                .collect(),
            subagents: peer
                .loops
                .iter()
                .filter(|l| l.parent_id.is_some() && l.teammate.is_none())
                .cloned()
                .collect(),
            contacts: peer.agents.clone(),
            address: peer.address,
            mail_port: peer.mail_port,
        }
    }
}

/// `FarmActivity.status`: esperando é alerta, idle/sleeping/done é parado, o resto trabalha.
/// `done` mantém o gorila comemorando (o `Done` local).
fn state_of(l: &FarmLoop) -> AgentState {
    match l.activity {
        Activity::Waiting => AgentState::Waiting,
        Activity::Idle | Activity::Sleeping => AgentState::Idle,
        Activity::Done => AgentState::Done,
        _ => AgentState::Working,
    }
}

/// A atividade que vai no fio para o estado local (o inverso de [`state_of`]): sem fonte de
/// ferramenta aqui, quem trabalha anuncia `thinking`; quem parou, `sleeping` (vai ao curral).
pub fn activity_of(state: AgentState) -> Activity {
    match state {
        AgentState::Working => Activity::Thinking,
        AgentState::Waiting => Activity::Waiting,
        AgentState::Idle => Activity::Sleeping,
        AgentState::Done => Activity::Done,
    }
}

/// A fazenda de um colega no mundo.
pub struct Lot {
    pub id: String,
    pub name: String,
    pub agents: Agents,
    pub subagents: Vec<FarmLoop>,
    /// `None` enquanto o primeiro fundo não foi pintado (o lote não aparece).
    pub backdrop: Option<Backdrop>,
    /// Canto do lote no mundo (a minha fazenda fica em (0, 0)).
    pub at: (f64, f64),
    /// Até onde (x, nas coordenadas do lote) o fundo não é desenhado: no escritório a calçada
    /// da esquerda some e a parede dele vira a divisória com o vizinho.
    pub clip: f64,
    pub contacts: Vec<gsh_team::TeamAgent>,
    pub address: Option<IpAddr>,
    pub mail_port: Option<u16>,
}

impl Lot {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            agents: Agents::default(),
            subagents: Vec::new(),
            backdrop: None,
            at: (0.0, 0.0),
            clip: 0.0,
            contacts: Vec::new(),
            address: None,
            mail_port: None,
        }
    }

    /// Semente do fundo: cada colega com a sua (árvores e móveis diferentes).
    pub fn seed(&self) -> u32 {
        7 + hash_str(&self.id) % 997
    }

    /// Retângulo do lote no mundo (esquerda, topo, direita, base).
    pub fn rect(&self) -> Option<(f64, f64, f64, f64)> {
        let (w, h) = self.backdrop.as_ref()?.world();
        Some((self.at.0, self.at.1, self.at.0 + w, self.at.1 + h))
    }

    /// O fundo atual serve para o mundo `mine` neste cenário e ainda comporta os bichos?
    fn up_to_date(&self, mine: &Backdrop, mode: Mode) -> bool {
        self.backdrop.as_ref().is_some_and(|b| {
            b.mode == mode && b.built_for == mine.built_for && !b.outgrown(self.agents.need())
        })
    }

    /// Repinta o fundo se precisar (`_rebuildPeer`). `true` = mudou.
    /// ponytail: síncrono, como o `relayout` do meu mundo; assíncrono se vários colegas pesarem.
    pub fn rebuild(&mut self, mine: &Backdrop, mode: Mode) -> bool {
        if self.up_to_date(mine, mode) {
            return false;
        }
        let backdrop = Backdrop::build_lot(mine, mode, self.agents.need(), self.seed());
        self.agents.reslot(backdrop.capacity());
        self.backdrop = Some(backdrop);
        true
    }
}

/// FNV-1a estável (o `hashStr` do clodfarm, sobre as unidades UTF-16 como o `codeUnits`).
pub fn hash_str(s: &str) -> u32 {
    s.encode_utf16().fold(2_166_136_261_u32, |x, c| {
        (x ^ u32::from(c)).wrapping_mul(16_777_619)
    })
}

/// Põe os lotes prontos ao lado da minha fazenda (`mine`, em (0, 0)): em fila à direita até 3
/// colegas; com mais, em grade quase quadrada (a minha no canto de cima à esquerda). `joined`
/// (escritório) cola os lotes de uma fileira num prédio só: a parede esquerda do colega cai em
/// cima da minha parede direita e a calçada dele daquele lado é recortada (`clip`); entre
/// fileiras fica a rua. Devolve o retângulo do conjunto.
pub fn arrange(mine: &Backdrop, lots: &mut [Lot], joined: bool) -> (f64, f64, f64, f64) {
    let (mw, mh) = mine.world();
    let ready: Vec<&mut Lot> = lots.iter_mut().filter(|l| l.backdrop.is_some()).collect();
    let n = ready.len() + 1;
    let cols = if n <= 4 {
        n
    } else {
        (n as f64).sqrt().ceil() as usize
    };
    let mut bounds: (f64, f64, f64, f64) = (0.0, 0.0, mw, mh);
    let (mut x, mut y, mut row_h, mut right) = (mw, 0.0_f64, mh, mine.play.right);
    for (i, lot) in ready.into_iter().enumerate().map(|(k, l)| (k + 1, l)) {
        let first = i % cols == 0;
        if first {
            x = 0.0;
            y += row_h;
            row_h = 0.0;
        }
        let Some(b) = &lot.backdrop else { continue };
        let ((w, h), play) = (b.world(), b.play);
        // a parede vertical ocupa 1,5 px de mundo a partir de `play.right + 0,5`
        if joined && !first {
            lot.at = (right + 2.0 - play.left, y);
            lot.clip = play.left - 1.5;
        } else {
            lot.at = (x, y);
            lot.clip = 0.0;
        }
        x = lot.at.0 + w;
        right = lot.at.0 + play.right;
        row_h = row_h.max(h);
        bounds = (
            bounds.0.min(lot.at.0),
            bounds.1.min(lot.at.1),
            bounds.2.max(lot.at.0 + w),
            bounds.3.max(lot.at.1 + h),
        );
    }
    bounds
}

/// Limites da câmera com colegas: o conjunto e o zoom que o mostra inteiro numa janela
/// `view` (largura, altura). `None` = nenhum colega pronto (os limites de sempre).
pub fn team_bounds(
    mine: &Backdrop,
    lots: &mut [Lot],
    joined: bool,
    view: (f64, f64),
) -> Option<TeamBounds> {
    if lots.iter().all(|l| l.backdrop.is_none()) {
        return None;
    }
    let (left, top, right, bottom) = arrange(mine, lots, joined);
    Some(TeamBounds {
        left,
        top,
        right,
        bottom,
        z: (view.0 / (right - left)).min(view.1 / (bottom - top)),
    })
}

/// Reconcilia os lotes com os colegas da rede: cria, atualiza os bichos e tira quem saiu.
/// `true` = algo mudou (precisa redesenhar).
pub fn sync(lots: &mut Vec<Lot>, peers: &[PeerInput]) -> bool {
    let before = lots.len();
    lots.retain(|l| peers.iter().any(|p| p.id == l.id));
    let mut changed = lots.len() != before;
    // a ordem é a dos colegas (por nome), como o `_team` do Dart
    let mut ordered = Vec::with_capacity(peers.len());
    for p in peers {
        let mut lot = match lots.iter().position(|l| l.id == p.id) {
            Some(i) => lots.swap_remove(i),
            None => {
                changed = true;
                Lot::new(&p.id, &p.name)
            }
        };
        if lot.name != p.name {
            lot.name.clone_from(&p.name);
            changed = true;
        }
        if lot.contacts != p.contacts || lot.address != p.address || lot.mail_port != p.mail_port {
            lot.contacts.clone_from(&p.contacts);
            lot.address = p.address;
            lot.mail_port = p.mail_port;
            changed = true;
        }
        changed |= lot.agents.sync(p.agents.clone());
        if lot.subagents != p.subagents {
            lot.subagents.clone_from(&p.subagents);
            changed = true;
        }
        ordered.push(lot);
    }
    *lots = ordered;
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::Size;

    fn loop_(id: &str, parent: Option<&str>, mate: Option<&str>, a: Activity) -> FarmLoop {
        FarmLoop {
            id: id.into(),
            parent_id: parent.map(Into::into),
            teammate: mate.map(Into::into),
            harness: "claude".into(),
            project: "gsh".into(),
            title: format!("t-{id}"),
            agent_type: None,
            activity: a,
            tool: None,
            output_tokens: 0,
            context_tokens: 0,
            input_tokens: 0,
            cache_read_tokens: 0,
            context_window: 0,
            last_seen_ms: 0,
        }
    }

    /// Vetores do `hashStr` do clodfarm (FNV-1a sobre unidades UTF-16) calculados no Dart.
    #[test]
    fn hash_str_igual_ao_do_dart() {
        // dart: hashStr('') = 2166136261, hashStr('a') = 3826002220
        assert_eq!(hash_str(""), 2_166_136_261);
        assert_eq!(hash_str("a"), 3_826_002_220);
        assert_eq!(hash_str("ab12cd34ef56ab12"), hash_str("ab12cd34ef56ab12"));
        assert_ne!(hash_str("x"), hash_str("y"));
    }

    #[test]
    fn peer_vira_bichos_sem_os_subagentes() {
        let peer = Peer {
            id: "m".into(),
            name: "Ana".into(),
            agents: Vec::new(),
            mail_port: None,
            address: None,
            loops: vec![
                loop_("main", None, None, Activity::Editing),
                loop_("sub", Some("main"), None, Activity::Waiting),
                loop_("mate", Some("main"), Some("zé"), Activity::Waiting),
                loop_("dorme", None, None, Activity::Sleeping),
                loop_("fim", None, None, Activity::Done),
            ],
        };
        let p = PeerInput::from_peer(&peer);
        assert_eq!(p.subagents.len(), 1);
        assert_eq!(
            p.subagents.first().map(|agent| agent.id.as_str()),
            Some("sub")
        );
        let got: Vec<_> = p
            .agents
            .iter()
            .map(|a| (a.pane_id.as_str(), a.state))
            .collect();
        assert_eq!(
            got,
            [
                ("main", AgentState::Working),
                ("mate", AgentState::Waiting),
                ("dorme", AgentState::Idle),
                ("fim", AgentState::Done),
            ]
        );
    }

    #[test]
    fn estado_vai_e_volta_pelo_fio() {
        for s in [
            AgentState::Working,
            AgentState::Waiting,
            AgentState::Idle,
            AgentState::Done,
        ] {
            assert_eq!(state_of(&loop_("x", None, None, activity_of(s))), s);
        }
    }

    fn input(id: &str, name: &str, n: usize) -> PeerInput {
        PeerInput {
            id: id.into(),
            name: name.into(),
            agents: (0..n)
                .map(|i| AgentInput {
                    pane_id: format!("{id}{i}"),
                    name: format!("a{i}"),
                    state: AgentState::Working,
                })
                .collect(),
            subagents: Vec::new(),
            contacts: Vec::new(),
            address: None,
            mail_port: None,
        }
    }

    #[test]
    fn sync_cria_atualiza_e_remove() {
        let mut lots = Vec::new();
        assert!(sync(
            &mut lots,
            &[input("a", "Ana", 1), input("b", "Beto", 2)]
        ));
        assert_eq!(lots.len(), 2);
        assert!(!sync(
            &mut lots,
            &[input("a", "Ana", 1), input("b", "Beto", 2)]
        ));
        assert!(sync(&mut lots, &[input("b", "Beto", 2)]));
        assert_eq!(lots.first().map(|l| l.id.as_str()), Some("b"));
        assert!(sync(&mut lots, &[]));
        assert!(lots.is_empty());
    }

    /// Lotes na fileira à direita (até 3 colegas), depois em grade; no escritório colados.
    #[test]
    fn arranjo_em_fila_grade_e_colado() {
        let size = Size::new(1100.0, 700.0);
        let mine = Backdrop::build(size, Mode::Farm, Default::default());
        let mut lots = vec![Lot::new("a", "Ana"), Lot::new("b", "Beto")];
        let none = team_bounds(&mine, &mut lots, false, (1100.0, 700.0));
        assert!(none.is_none(), "sem fundo pintado o lote não aparece");
        for l in &mut lots {
            l.agents.sync(vec![AgentInput {
                pane_id: "p".into(),
                name: "p".into(),
                state: AgentState::Idle,
            }]);
            assert!(l.rebuild(&mine, Mode::Farm));
            assert!(!l.rebuild(&mine, Mode::Farm), "nada mudou: não repinta");
        }
        let (mw, mh) = mine.world();
        let tb = team_bounds(&mine, &mut lots, false, (1100.0, 700.0)).unwrap();
        // em fila: o primeiro lote começa onde a minha fazenda acaba
        assert_eq!(lots[0].at, (mw, 0.0));
        let w0 = lots[0].backdrop.as_ref().unwrap().world().0;
        assert_eq!(lots[1].at, (mw + w0, 0.0));
        assert_eq!((tb.left, tb.top), (0.0, 0.0));
        assert!(tb.right > mw + w0 && tb.bottom >= mh);
        assert!(
            tb.z < mine.extent.z,
            "o conjunto pede mais zoom-out que a minha fazenda"
        );
        // escritório: o lote cola na minha parede e a calçada da esquerda é recortada
        let office = Backdrop::build(size, Mode::Office, Default::default());
        for l in &mut lots {
            l.rebuild(&office, Mode::Office);
        }
        team_bounds(&office, &mut lots, true, (1100.0, 700.0)).unwrap();
        let first = &lots[0];
        let play_left = first.backdrop.as_ref().unwrap().play.left;
        assert_eq!(first.at.0, office.play.right + 2.0 - play_left);
        assert_eq!(first.clip, play_left - 1.5);
    }

    #[test]
    fn grade_com_mais_de_quatro() {
        let size = Size::new(900.0, 600.0);
        let mine = Backdrop::build(size, Mode::Farm, Default::default());
        let mut lots: Vec<Lot> = (0..5).map(|i| Lot::new(&format!("{i}"), "x")).collect();
        for l in &mut lots {
            l.rebuild(&mine, Mode::Farm);
        }
        // n = 6, cols = 3: o 3º colega abre a segunda fileira, na coluna 0
        arrange(&mine, &mut lots, false);
        assert_eq!(lots[2].at.0, 0.0);
        assert!(lots[2].at.1 > 0.0);
        assert_eq!(lots[0].at.1, 0.0);
    }
}
