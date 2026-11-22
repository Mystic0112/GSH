//! Composição do painel de controle de versão.
use crate::{git_state::GitState, labels, panel_widgets, theme};
use iced::widget::{column, container, text};
use iced::{Element, Fill};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTarget {
    pub id: String,
    pub label: String,
}

impl fmt::Display for AgentTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.label, self.id)
    }
}
#[derive(Debug, Clone)]
pub enum GitMsg {
    Refresh,
    Changes,
    History,
    Branches,
    Message(String),
    Commit,
    SuggestCommit,
    Stage(String),
    Unstage(String),
    Diff(String, bool),
    SwitchBranch(String),
    CloseDiff,
    StartNote(usize, usize),
    NoteInput(String),
    SaveNote,
    CancelNote,
    RemoveNote(usize, usize),
    SelectNoteAgent(String),
    SendNotesToAgent(String),
    RefreshPr,
    LoadPrLog(u64, usize),
    OpenPrUrl(u64, Option<crate::git_pr::PrItem>),
    SelectPrAgent(String),
    SendPrFeedback(u64, crate::git_pr::PrItem, String),
}

pub fn view(state: &GitState, agents: Vec<AgentTarget>) -> Element<'_, GitMsg> {
    let mut content =
        column![crate::git_header::view(state), panel_widgets::hairline_h()].height(Fill);
    if let Some(error) = &state.error {
        content = content.push(
            container(
                text(labels::format_git_error(error))
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().error),
            )
            .padding([4, 14]),
        );
    }
    let body = if state.repo.is_none() {
        container(
            text(labels::GIT_NO_REPO)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        )
        .padding(14)
        .into()
    } else if state.branches_open {
        crate::git_branch_view::view(state)
    } else if state.history_open {
        crate::git_history_view::view(state)
    } else {
        crate::git_changes_view::view(state, agents)
    };
    content = content.push(body);
    content.into()
}
