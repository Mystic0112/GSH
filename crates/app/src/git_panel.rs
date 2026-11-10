//! Composição do painel de controle de versão.
use crate::{git_state::GitState, labels, panel_widgets, theme};
use iced::widget::{column, container, text};
use iced::{Element, Fill};
#[derive(Debug, Clone)]
pub enum GitMsg {
    Files,
    Refresh,
    Changes,
    History,
    Message(String),
    Commit,
    Stage(String),
    Unstage(String),
    Diff(String, bool),
    CloseDiff,
}

pub fn view(state: &GitState) -> Element<'_, GitMsg> {
    let mut content =
        column![crate::git_header::view(state), panel_widgets::hairline_h()].height(Fill);
    if let Some(error) = &state.error {
        content = content.push(
            container(
                text(labels::format_git_error(error))
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_LABEL)
                    .color(theme::ERROR),
            )
            .padding([4, 14]),
        );
    }
    let body = if state.repo.is_none() {
        container(
            text(labels::GIT_NO_REPO)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::TEXT3),
        )
        .padding(14)
        .into()
    } else if state.history_open {
        crate::git_history_view::view(state)
    } else {
        crate::git_changes_view::view(state)
    };
    content = content.push(body);
    panel_widgets::frame(content.into())
}
