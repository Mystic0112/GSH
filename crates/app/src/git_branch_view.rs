use iced::widget::{button, column, container, row, text, Space};
use iced::{Element, Fill};

use crate::{git_panel::GitMsg, git_state::GitState, labels, theme};

pub fn view(state: &GitState) -> Element<'_, GitMsg> {
    let current = state
        .status
        .as_ref()
        .map(|status| status.branch.as_str())
        .unwrap_or_default();
    let mut rows = column![].spacing(4).padding(14);
    if !state.can_switch_branch() {
        rows = rows.push(
            text(labels::GIT_BRANCH_DIRTY_NOTICE)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().warn),
        );
    }
    if state.branches.is_empty() {
        rows = rows.push(text(labels::GIT_BRANCHES_EMPTY).color(theme::palette().text3));
    }
    for branch in &state.branches {
        let is_current = branch == current;
        let mut row_content = row![
            text(branch).color(theme::palette().text),
            Space::new().width(Fill)
        ];
        if is_current {
            row_content = row_content
                .push(text(labels::GIT_BRANCH_CURRENT).color(theme::palette().accent_text));
        }
        rows = rows.push(
            button(row_content)
                .width(Fill)
                .padding([7, 10])
                .on_press_maybe(
                    (!is_current && state.can_switch_branch())
                        .then(|| GitMsg::SwitchBranch(branch.clone())),
                )
                .style(theme::ghost_button),
        );
    }
    container(rows).width(Fill).into()
}
