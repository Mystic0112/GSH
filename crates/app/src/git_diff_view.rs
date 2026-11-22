//! Apresentação do unified diff com limite de linhas e notas por linha para o agente.
use crate::{
    git_panel::{AgentTarget, GitMsg},
    git_presenter,
    git_state::GitState,
    labels, panel_widgets, theme,
};
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, Space,
};
use iced::{Border, Element, Fill, Length, Theme};
use std::collections::BTreeMap;
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

pub fn format_notes_for_agent(
    diff: &gsh_git::FileDiff,
    notes: &BTreeMap<(usize, usize), String>,
) -> Option<String> {
    if notes.is_empty() {
        return None;
    }
    let mut blocks = Vec::new();
    for (&(hunk_idx, line_idx), note) in notes {
        let trimmed = note.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Some(hunk) = diff.hunks.get(hunk_idx) else {
            continue;
        };
        let Some(line) = hunk.lines.get(line_idx) else {
            continue;
        };
        if line.kind == gsh_git::DiffLineKind::Context {
            continue;
        }
        let Some(line_no) = line.new_line.or(line.old_line) else {
            continue;
        };
        let snippet = git_presenter::diff_line(line);
        blocks.push(format!(
            "- `{}:{line_no}` (`{snippet}`): {trimmed}",
            diff.path
        ));
    }
    if blocks.is_empty() {
        return None;
    }
    Some(format!(
        "Revise estas observações no diff de `{}`:\n{}",
        diff.path,
        blocks.join("\n")
    ))
}

pub fn view<'a>(
    diff: &'a gsh_git::FileDiff,
    original: Option<&'a str>,
    state: &'a GitState,
    agents: Vec<AgentTarget>,
) -> Element<'a, GitMsg> {
    let notes = &state.diff_notes;
    let active_note = state.active_note.as_ref();
    let badge = if diff.staged {
        labels::GIT_DIFF_STAGED_BADGE
    } else {
        labels::GIT_DIFF_WORKTREE_BADGE
    };
    let header = row![
        text(labels::ICON_DIFF)
            .font(theme::FONT_ICON)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3),
        text(labels::format_diff_tab_title(&diff.path))
            .font(theme::FONT_TITLE)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text),
        Space::new().width(Fill),
        panel_widgets::ghost(labels::ICON_CLOSE, Some(GitMsg::CloseDiff), false),
    ]
    .spacing(6)
    .align_y(iced::Alignment::Center);
    let mut lines = column![text(badge)
        .font(theme::FONT_BODY_MEDIUM)
        .size(theme::SIZE_BADGE)
        .color(theme::palette().text3)]
    .spacing(1);
    if !notes.is_empty() {
        let selected = agents
            .iter()
            .find(|agent| state.note_agent.as_deref() == Some(agent.id.as_str()))
            .cloned();
        let send = selected
            .as_ref()
            .map(|agent| GitMsg::SendNotesToAgent(agent.id.clone()));
        let unwritten = if agents.is_empty() {
            "Nenhum agente disponível neste workspace"
        } else {
            "Escolher agente"
        };
        lines = lines.push(
            column![
                pick_list(agents, selected, |agent| GitMsg::SelectNoteAgent(agent.id))
                    .placeholder(unwritten)
                    .text_size(theme::SIZE_BADGE),
                button(
                    text(labels::format_git_send_notes_button(notes.len()))
                        .font(theme::FONT_BODY_MEDIUM)
                        .size(theme::SIZE_BADGE)
                )
                .padding([3, 8])
                .on_press_maybe(send)
                .style(theme::ghost_button),
            ]
            .spacing(4),
        );
    }
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
    for (hunk_idx, hunk) in diff.hunks.iter().enumerate() {
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
        for (line_idx, line) in hunk.lines.iter().take(remaining).enumerate() {
            let key = (hunk_idx, line_idx);
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
            let annotate_btn = button(
                text(labels::GIT_DIFF_ADD_NOTE)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_BADGE)
                    .color(theme::palette().text3),
            )
            .padding([1, 6])
            .on_press(GitMsg::StartNote(hunk_idx, line_idx))
            .style(theme::ghost_button);
            let mut line_row = row![gutter, body].align_y(iced::Alignment::Center);
            if line.kind != gsh_git::DiffLineKind::Context {
                line_row = line_row.push(Space::new().width(8)).push(annotate_btn);
            }
            lines = lines.push(container(line_row).style(move |_| container::Style {
                background: background.map(Into::into),
                ..Default::default()
            }));
            if let Some((_, draft)) = active_note.filter(|(active_key, _)| *active_key == key) {
                let input = text_input(labels::GIT_DIFF_NOTE_PLACEHOLDER, draft)
                    .on_input(GitMsg::NoteInput)
                    .on_submit(GitMsg::SaveNote)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_LABEL)
                    .style(|_: &Theme, _| text_input::Style {
                        background: theme::palette().panel2.into(),
                        border: Border {
                            color: theme::palette().border2,
                            width: 1.0,
                            radius: theme::RADIUS_BUTTON.into(),
                        },
                        icon: theme::palette().text3,
                        placeholder: theme::palette().text3,
                        value: theme::palette().text,
                        selection: theme::palette().accent_soft,
                    });
                let save_btn = button(
                    text(labels::GIT_DIFF_NOTE_SAVE)
                        .font(theme::FONT_BODY_MEDIUM)
                        .size(theme::SIZE_BADGE),
                )
                .padding([3, 8])
                .on_press(GitMsg::SaveNote)
                .style(theme::ghost_button);
                let cancel_btn = button(
                    text(labels::DIALOG_CANCEL)
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_BADGE),
                )
                .padding([3, 8])
                .on_press(GitMsg::CancelNote)
                .style(theme::ghost_button);
                lines = lines.push(
                    container(row![input, save_btn, cancel_btn].spacing(4).padding([2, 8]))
                        .width(Fill),
                );
            } else if let Some(saved_note) = notes.get(&key) {
                let edit_btn = button(
                    text(saved_note)
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().accent_text),
                )
                .padding([2, 6])
                .on_press(GitMsg::StartNote(hunk_idx, line_idx))
                .style(theme::ghost_button);
                let remove_btn = button(
                    text(labels::GIT_DIFF_NOTE_REMOVE)
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_BADGE)
                        .color(theme::palette().text3),
                )
                .padding([2, 6])
                .on_press(GitMsg::RemoveNote(hunk_idx, line_idx))
                .style(theme::ghost_button);
                lines = lines.push(
                    container(row![edit_btn, remove_btn].spacing(6).padding([2, 12])).style(|_| {
                        container::Style {
                            background: Some(theme::palette().panel2.into()),
                            ..Default::default()
                        }
                    }),
                );
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_git::{DiffHunk, DiffLine, DiffLineKind, FileDiff, FileDiffKind};

    #[test]
    fn format_notes_for_agent_includes_path_line_snippet_and_note() {
        let diff = FileDiff {
            path: "src/lib.rs".into(),
            staged: false,
            kind: FileDiffKind::Modified,
            hunks: vec![DiffHunk {
                header: "@@ -10,2 +10,3 @@".into(),
                lines: vec![
                    DiffLine {
                        kind: DiffLineKind::Context,
                        old_line: Some(10),
                        new_line: Some(10),
                        text: "fn demo() {".into(),
                    },
                    DiffLine {
                        kind: DiffLineKind::Added,
                        old_line: None,
                        new_line: Some(11),
                        text: "    let x = 42;".into(),
                    },
                ],
            }],
        };
        let mut notes = BTreeMap::new();
        notes.insert((0, 1), "trocar 42 por constante nomeada".to_owned());
        let formatted = format_notes_for_agent(&diff, &notes).unwrap();
        assert!(formatted.contains("src/lib.rs:11"));
        assert!(formatted.contains("+    let x = 42;"));
        assert!(formatted.contains("trocar 42 por constante nomeada"));
    }
}
