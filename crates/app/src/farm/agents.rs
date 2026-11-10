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
impl Agents {
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
