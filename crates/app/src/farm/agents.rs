use gsh_core::status::TurnStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Working,
    Waiting,
    Done,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    Sleeping,
    Typing,
    Alert,
    Happy,
}
impl AgentState {
    pub fn from_status(status: TurnStatus, finished: bool) -> Self {
        match status {
            TurnStatus::Working => Self::Working,
            TurnStatus::Waiting => Self::Waiting,
            TurnStatus::Idle if finished => Self::Done,
            TurnStatus::Idle => Self::Idle,
        }
    }
    pub fn pose(self) -> Pose {
        match self {
            Self::Idle => Pose::Sleeping,
            Self::Working => Pose::Typing,
            Self::Waiting => Pose::Alert,
            Self::Done => Pose::Happy,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentInput {
    pub pane_id: String,
    pub name: String,
    pub state: AgentState,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agent {
    pub input: AgentInput,
    pub slot: usize,
}
#[derive(Default)]
pub struct Agents {
    pub items: Vec<Agent>,
}
/// Contadores da topbar (`FarmStrip` do Flutter): `Done` conta como parado.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FarmCounts {
    pub working: usize,
    pub waiting: usize,
    pub idle: usize,
}
/// Quanto a cena precisa caber: canteiros (um por agente) e vagas no curral (quem dorme).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Need {
    pub plots: i64,
    pub yard: i64,
}
impl Agents {
    pub fn need(&self) -> Need {
        Need {
            plots: self.items.len() as i64,
            yard: self
                .items
                .iter()
                .filter(|a| a.input.state == AgentState::Idle)
                .count() as i64,
        }
    }
    /// Quem ficou além do que o layout comporta pega o primeiro canteiro livre dentro dele.
    pub fn reslot(&mut self, capacity: usize) {
        for i in 0..self.items.len() {
            if self.items.get(i).is_none_or(|a| a.slot < capacity) {
                continue;
            }
            if let Some(free) = (0..capacity).find(|s| !self.items.iter().any(|a| a.slot == *s)) {
                if let Some(agent) = self.items.get_mut(i) {
                    agent.slot = free;
                }
            }
        }
    }
    pub fn counts(&self) -> FarmCounts {
        let mut counts = FarmCounts::default();
        for agent in &self.items {
            match agent.input.state {
                AgentState::Working => counts.working += 1,
                AgentState::Waiting => counts.waiting += 1,
                AgentState::Idle | AgentState::Done => counts.idle += 1,
            }
        }
        counts
    }
    pub fn sync(&mut self, inputs: Vec<AgentInput>) -> bool {
        let old = self.items.clone();
        self.items
            .retain(|a| inputs.iter().any(|i| i.pane_id == a.input.pane_id));
        for input in inputs {
            if let Some(agent) = self
                .items
                .iter_mut()
                .find(|a| a.input.pane_id == input.pane_id)
            {
                agent.input = input;
            } else {
                let slot = (0..)
                    .find(|slot| !self.items.iter().any(|a| a.slot == *slot))
                    .unwrap_or(0);
                self.items.push(Agent { input, slot });
            }
        }
        old != self.items
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(id: &str) -> AgentInput {
        AgentInput {
            pane_id: id.into(),
            name: id.into(),
            state: AgentState::Idle,
        }
    }
    #[test]
    fn poses_and_status() {
        assert_eq!(
            AgentState::from_status(TurnStatus::Idle, false).pose(),
            Pose::Sleeping
        );
        assert_eq!(
            AgentState::from_status(TurnStatus::Working, false).pose(),
            Pose::Typing
        );
        assert_eq!(
            AgentState::from_status(TurnStatus::Waiting, false).pose(),
            Pose::Alert
        );
        assert_eq!(
            AgentState::from_status(TurnStatus::Idle, true).pose(),
            Pose::Happy
        );
    }
    #[test]
    fn counts_group_done_with_idle() {
        let mut agents = Agents::default();
        let mut inputs = vec![input("a"), input("b"), input("c"), input("d")];
        inputs[1].state = AgentState::Working;
        inputs[2].state = AgentState::Waiting;
        inputs[3].state = AgentState::Done;
        agents.sync(inputs);
        assert_eq!(
            agents.counts(),
            FarmCounts {
                working: 1,
                waiting: 1,
                idle: 2
            }
        );
    }
    #[test]
    fn need_counts_agents_and_sleepers_and_reslot_fits_capacity() {
        let mut agents = Agents::default();
        let mut inputs: Vec<AgentInput> = (0..8).map(|i| input(&format!("p{i}"))).collect();
        inputs[0].state = AgentState::Working;
        agents.sync(inputs);
        assert_eq!(agents.need(), Need { plots: 8, yard: 7 });
        agents.reslot(6);
        let mut slots: Vec<usize> = agents.items.iter().map(|a| a.slot).collect();
        slots.sort_unstable();
        // o layout encolheu para 6 canteiros: quem estava além pega vaga livre, o resto fica
        assert_eq!(slots.iter().filter(|s| **s < 6).count(), 6);
        assert_eq!(slots.iter().filter(|s| **s >= 6).count(), 2);
    }
    #[test]
    fn slots_stay_and_removed_slot_is_reused() {
        let mut agents = Agents::default();
        agents.sync(vec![input("a"), input("b")]);
        agents.sync(vec![input("b"), input("a")]);
        assert_eq!(
            agents.items.iter().map(|a| a.slot).collect::<Vec<_>>(),
            vec![0, 1]
        );
        agents.sync(vec![input("b"), input("c")]);
        assert_eq!(
            agents.items.iter().map(|a| a.slot).collect::<Vec<_>>(),
            vec![1, 0]
        );
    }
}
