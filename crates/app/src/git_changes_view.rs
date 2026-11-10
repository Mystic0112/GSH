//! Alterações, commit e diff selecionado.
use crate::{git_file_row, git_panel::GitMsg, git_presenter, git_state::GitState, labels, theme};
use iced::widget::{button, column, scrollable, text, text_input};
use iced::{Border, Element, Fill, Theme};
pub fn view(state: &GitState) -> Element<'_, GitMsg> {
    let mut list = column![].spacing(3);
    if let Some(status) = &state.status {
        list = list.push(
            text(labels::format_staged_header(status.staged.len()))
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_SECTION_HEADER)
                .color(theme::TEXT3),
        );
        for (path, status) in git_presenter::files(&status.staged) {
            list = list.push(git_file_row::view(
                path,
                status,
                true,
                state.busy,
                state
                    .diff
                    .as_ref()
                    .is_some_and(|d| d.path == path && d.staged),
            ));
        }
        list = list.push(
            text(labels::format_changes_header(status.unstaged.len()))
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_SECTION_HEADER)
                .color(theme::TEXT3),
        );
        for (path, status) in git_presenter::files(&status.unstaged) {
            list = list.push(git_file_row::view(
                path,
                status,
                false,
                state.busy,
                state
                    .diff
                    .as_ref()
                    .is_some_and(|d| d.path == path && !d.staged),
            ));
        }
        if status.staged.is_empty() && status.unstaged.is_empty() {
            list = list.push(
                text(labels::GIT_NO_CHANGES)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_LABEL)
                    .color(theme::TEXT3),
            );
        }
    }
    let input = text_input(labels::GIT_COMMIT_PLACEHOLDER, &state.message)
        .on_input_maybe((!state.busy).then_some(GitMsg::Message as fn(String) -> GitMsg))
        .font(theme::FONT_BODY)
        .size(theme::SIZE_LABEL)
        .style(|_, _| text_input::Style {
            background: theme::PANEL2.into(),
            border: Border {
                color: theme::BORDER2,
                width: 1.0,
                radius: theme::RADIUS_BUTTON.into(),
            },
            icon: theme::TEXT3,
            placeholder: theme::TEXT3,
            value: theme::TEXT,
            selection: theme::ACCENT_SOFT,
        });
    let commit = button(
        text(if state.committing {
            labels::GIT_COMMITTING_BUTTON
        } else {
            labels::GIT_COMMIT_BUTTON
        })
        .font(theme::FONT_BODY_MEDIUM)
        .size(theme::SIZE_LABEL),
    )
    .width(Fill)
    .on_press_maybe(state.can_commit().then_some(GitMsg::Commit))
    .style(|_: &Theme, status| button::Style {
        background: Some(
            if matches!(status, button::Status::Disabled) {
                theme::PANEL2
            } else {
                theme::ACCENT
            }
            .into(),
        ),
        text_color: if matches!(status, button::Status::Disabled) {
            theme::TEXT4
        } else {
            theme::ON_ACCENT
        },
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let mut body = column![input, commit].spacing(8).padding(14);
    if let Some(diff) = &state.diff {
        body = body.push(crate::git_diff_view::view(
            diff,
            state
                .renames
                .get(&diff.path)
                .map(String::as_str)
                .filter(|_| diff.staged),
        ));
    }
    body = body.push(list);
    scrollable(body).height(Fill).into()
}
