//! Histórico resumido do repositório.
use crate::{git_panel::GitMsg, git_presenter, git_state::GitState, labels, theme};
use iced::widget::{column, scrollable, text};
use iced::{Element, Fill};
pub fn view(state: &GitState) -> Element<'_, GitMsg> {
    let mut list = column![].spacing(10).padding(14);
    if state.history.is_empty() {
        list = list.push(
            text(labels::GIT_HISTORY_EMPTY)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        );
    }
    for commit in &state.history {
        let (title, meta) = git_presenter::commit(commit);
        list = list.push(column![
            text(title)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text),
            text(meta)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3)
        ]);
    }
    scrollable(list).height(Fill).into()
}
