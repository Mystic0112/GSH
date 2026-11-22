//! Alterações, commit e diff selecionado.
use crate::{
    git_file_row,
    git_panel::{AgentTarget, GitMsg},
    git_pr::{CheckStatus, PrItem},
    git_presenter,
    git_state::GitState,
    labels, theme,
};
use iced::widget::{button, column, pick_list, row, scrollable, text, text_input};
use iced::{Border, Element, Fill, Theme};
pub fn view(state: &GitState, agents: Vec<AgentTarget>) -> Element<'_, GitMsg> {
    let mut list = column![].spacing(3);
    if let Some(status) = &state.status {
        list = list.push(
            text(labels::format_staged_header(status.staged.len()))
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_SECTION_HEADER)
                .color(theme::palette().text3),
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
                .color(theme::palette().text3),
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
                    .color(theme::palette().text3),
            );
        }
    }
    let input = text_input(labels::GIT_COMMIT_PLACEHOLDER, &state.message)
        .on_input_maybe((!state.busy).then_some(GitMsg::Message as fn(String) -> GitMsg))
        .font(theme::FONT_BODY)
        .size(theme::SIZE_LABEL)
        .style(|_, _| text_input::Style {
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
                theme::palette().panel2
            } else {
                theme::palette().accent
            }
            .into(),
        ),
        text_color: if matches!(status, button::Status::Disabled) {
            theme::palette().text4
        } else {
            theme::ON_ACCENT
        },
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let suggest = button(text(if state.suggesting {
        labels::GIT_SUGGESTING_COMMIT
    } else {
        labels::GIT_SUGGEST_COMMIT
    }))
    .width(Fill)
    .on_press_maybe(state.can_suggest_commit().then_some(GitMsg::SuggestCommit))
    .style(theme::ghost_button);
    let mut body = column![input, suggest, commit, pr_view(state, agents.clone())]
        .spacing(8)
        .padding(14);
    if let Some(diff) = &state.diff {
        body = body.push(crate::git_diff_view::view(
            diff,
            state
                .renames
                .get(&diff.path)
                .map(String::as_str)
                .filter(|_| diff.staged),
            state,
            agents,
        ));
    }
    body = body.push(list);
    scrollable(body).height(Fill).into()
}

fn pr_view(state: &GitState, agents: Vec<AgentTarget>) -> Element<'_, GitMsg> {
    let refresh = if state.fetching_pr {
        labels::GIT_FETCHING_PR_FEEDBACK
    } else if state.pr.is_some() {
        "Atualizar PR"
    } else {
        "Consultar PR da branch"
    };
    let mut panel = column![button(text(refresh).size(theme::SIZE_LABEL))
        .on_press_maybe((!state.busy).then_some(GitMsg::RefreshPr))
        .style(theme::ghost_button)]
    .spacing(8);
    let Some(pr) = &state.pr else {
        return panel.into();
    };
    let revision = state.pr_revision;
    panel = panel.push(pr_button(
        format!("#{} {} ↗", pr.number, pr.title),
        Some(GitMsg::OpenPrUrl(revision, None)),
    ));
    let selected = agents
        .iter()
        .find(|agent| state.pr_agent.as_deref() == Some(agent.id.as_str()))
        .cloned();
    let send = |item| {
        selected
            .as_ref()
            .filter(|_| !state.busy)
            .map(|agent| GitMsg::SendPrFeedback(revision, item, agent.id.clone()))
    };
    let placeholder = if agents.is_empty() {
        "Nenhum agente disponível neste workspace"
    } else {
        "Escolher agente"
    };
    panel = panel.push(
        pick_list(agents, selected.clone(), |agent| {
            GitMsg::SelectPrAgent(agent.id)
        })
        .placeholder(placeholder)
        .text_size(theme::SIZE_LABEL),
    );
    if let Some(notice) = state.pr_notice {
        panel = panel.push(
            text(notice)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().accent),
        );
    }
    panel = panel.push(
        text("Checks")
            .font(theme::FONT_TITLE)
            .size(theme::SIZE_LABEL),
    );
    if pr.checks.is_empty() {
        panel = panel.push(text("Nenhum check para este PR.").size(theme::SIZE_LABEL));
    }
    for (index, check) in pr.checks.iter().enumerate() {
        let item = PrItem::Check(index);
        let status = match check.status {
            CheckStatus::Passed => "Passou",
            CheckStatus::Failed => "Falhou",
            CheckStatus::Running => "Em andamento",
            CheckStatus::Other if check.raw_status.is_empty() => "Estado indisponível",
            CheckStatus::Other => check.raw_status.as_str(),
        };
        let mut section =
            column![text(format!("{status} · {}", check.name)).size(theme::SIZE_LABEL)].spacing(4);
        if let Some(log) = &check.log {
            section = match log {
                Ok(log) => section.push(text(log).font(theme::FONT_MONO).size(theme::SIZE_BADGE)),
                Err(error) => section.push(
                    text(error)
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().error),
                ),
            };
        }
        let mut actions = row![].spacing(4);
        if check.url.is_some() {
            actions = actions.push(pr_button(
                "Abrir check ↗",
                Some(GitMsg::OpenPrUrl(revision, Some(item))),
            ));
        }
        if check.status == CheckStatus::Failed {
            let can_load_log = pr.log_request(index).is_some();
            if can_load_log && !matches!(check.log, Some(Ok(_))) {
                actions = actions.push(pr_button(
                    "Carregar log da falha",
                    (!state.busy).then_some(GitMsg::LoadPrLog(revision, index)),
                ));
            }
            let can_send = !can_load_log || check.log.is_some();
            actions = actions.push(pr_button(
                labels::GIT_SEND_PR_FEEDBACK,
                can_send.then(|| send(item)).flatten(),
            ));
        }
        panel = panel.push(section.push(actions.wrap()));
    }
    panel = panel.push(
        text("Comentários de revisão abertos")
            .font(theme::FONT_TITLE)
            .size(theme::SIZE_LABEL),
    );
    if let Some(error) = &pr.review_error {
        panel = panel.push(
            text(error)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().error),
        );
    } else if pr.comments.is_empty() {
        panel = panel.push(text("Nenhum comentário de revisão aberto.").size(theme::SIZE_LABEL));
    }
    for (index, comment) in pr.comments.iter().enumerate() {
        let item = PrItem::Comment(index);
        let mut actions = row![].spacing(4);
        if comment.url.is_some() {
            actions = actions.push(pr_button(
                "Abrir comentário ↗",
                Some(GitMsg::OpenPrUrl(revision, Some(item))),
            ));
        }
        panel = panel.push(
            column![
                text(format!("@{} · {}", comment.author, comment.location))
                    .font(theme::FONT_BODY_MEDIUM)
                    .size(theme::SIZE_LABEL),
                text(&comment.body).size(theme::SIZE_LABEL),
                text(&comment.snippet)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_BADGE),
                actions
                    .push(pr_button(labels::GIT_SEND_PR_FEEDBACK, send(item)))
                    .wrap(),
            ]
            .spacing(4),
        );
    }
    panel.into()
}

fn pr_button(label: impl Into<String>, message: Option<GitMsg>) -> Element<'static, GitMsg> {
    button(text(label.into()).size(theme::SIZE_LABEL))
        .style(theme::ghost_button)
        .on_press_maybe(message)
        .into()
}
