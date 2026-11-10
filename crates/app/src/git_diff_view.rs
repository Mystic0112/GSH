//! Apresentação do unified diff com limite de linhas.
use crate::{git_panel::GitMsg, git_presenter, labels, panel_widgets, theme};
use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Element, Fill, Length};
const MAX_DIFF_LINES: usize = 2000;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffSize {
    Complete,
    Truncated,
}
fn diff_size(diff: &gsh_git::FileDiff) -> DiffSize {
    if diff
        .hunks
        .iter()
        .map(|h| h.lines.len().saturating_add(1))
        .sum::<usize>()
        > MAX_DIFF_LINES
    {
        DiffSize::Truncated
    } else {
        DiffSize::Complete
    }
}
pub fn view<'a>(diff: &'a gsh_git::FileDiff, original: Option<&'a str>) -> Element<'a, GitMsg> {
    let badge = if diff.staged {
        labels::GIT_DIFF_STAGED_BADGE
    } else {
        labels::GIT_DIFF_WORKTREE_BADGE
    };
    let header = row![
        text(labels::ICON_DIFF)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3),
        text(labels::format_diff_tab_title(&diff.path))
            .font(theme::FONT_TITLE)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text),
        Space::new().width(Fill),
        panel_widgets::ghost(labels::ICON_CLOSE, Some(GitMsg::CloseDiff), false)
    ]
    .align_y(iced::Alignment::Center);
    let mut lines = column![text(badge)
        .font(theme::FONT_BODY_MEDIUM)
        .size(theme::SIZE_BADGE)
        .color(theme::palette().text3)]
    .spacing(1);
    if let Some(original) = original {
        lines = lines.push(
            text(format!("{}{original}", labels::GIT_RENAMED_FROM))
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        );
    }
    if diff.kind == gsh_git::FileDiffKind::Binary {
        lines = lines.push(
            text(labels::GIT_DIFF_BINARY)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        );
    }
    if diff.kind == gsh_git::FileDiffKind::Unchanged {
        lines = lines.push(
            text(labels::GIT_DIFF_UNCHANGED)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        );
    }
    let mut remaining = MAX_DIFF_LINES;
    for hunk in &diff.hunks {
        if remaining == 0 {
            break;
        }
        remaining = remaining.saturating_sub(1);
        lines = lines.push(
            text(&hunk.header)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        );
        for line in hunk.lines.iter().take(remaining) {
            let (color, background) = git_presenter::diff_style(line.kind);
            let old = line.old_line.map(|n| n.to_string()).unwrap_or_default();
            let new = line.new_line.map(|n| n.to_string()).unwrap_or_default();
            let gutter = text(format!("{old:>4} {new:>4} "))
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text4);
            let body = text(git_presenter::diff_line(line))
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(color);
            lines = lines.push(
                container(row![gutter, body]).style(move |_| container::Style {
                    background: background.map(Into::into),
                    ..Default::default()
                }),
            );
        }
        remaining = remaining.saturating_sub(hunk.lines.len());
    }
    if diff_size(diff) == DiffSize::Truncated {
        lines = lines.push(
            text(labels::GIT_DIFF_TRUNCATED)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().warn),
        );
    }
    column![
        header,
        scrollable(lines.width(Length::Shrink))
            .horizontal()
            .width(Fill)
    ]
    .spacing(4)
    .into()
}
