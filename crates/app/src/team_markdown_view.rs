use crate::{labels, pane_close_dialog, theme};
use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

use super::TeamMarkdownSpec;

pub(crate) fn review_modal<'a, M: Clone + 'a>(
    result: &'a Result<TeamMarkdownSpec, String>,
    on_cancel: M,
    on_confirm: M,
    on_noop: M,
) -> Element<'a, M> {
    let heading = text(labels::AGENT_GRAPH_IMPORT_REVIEW_TITLE)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text);

    match result {
        Err(err) => {
            let body = container(
                text(err)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().error),
            )
            .width(Length::Fixed(500.0));
            let actions = row![
                Space::new().width(Fill),
                pane_close_dialog::action(
                    labels::DIALOG_CANCEL,
                    pane_close_dialog::ButtonKind::Outline,
                    Some(on_cancel.clone()),
                ),
            ];
            pane_close_dialog::form(
                heading.into(),
                body.into(),
                actions.into(),
                Some(500.0),
                on_cancel,
                on_noop,
            )
        }
        Ok(spec) => {
            let mut rows = column![
                text(labels::AGENT_GRAPH_IMPORT_SECURITY_NOTICE)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_BADGE)
                    .color(theme::palette().warn),
                text(format!("Time: {}", spec.name))
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text),
            ]
            .spacing(6);

            if let Some(maestro) = &spec.maestro {
                rows = rows.push(
                    text(format!("Maestro: {maestro}"))
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().accent_text),
                );
            }

            for agent in &spec.agents {
                let harness_label = if agent.harness.is_empty() {
                    "shell"
                } else {
                    agent.harness.as_str()
                };
                let mut detail = format!(
                    "{} · harness: {} · pasta: {}",
                    agent.name, harness_label, agent.cwd
                );
                if !agent.links.is_empty() {
                    detail.push_str(&format!(" · conexões: {}", agent.links.join(", ")));
                }
                let mut card_col = column![text(detail)
                    .font(theme::FONT_BODY_MEDIUM)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text)]
                .spacing(2);
                if !agent.role.is_empty() {
                    card_col = card_col.push(
                        text(format!("Papel (dado externo): {}", agent.role))
                            .font(theme::FONT_BODY)
                            .size(theme::SIZE_BADGE)
                            .color(theme::palette().text2),
                    );
                }
                if !agent.task.is_empty() {
                    card_col = card_col.push(
                        text(format!("Tarefa (dado externo): {}", agent.task))
                            .font(theme::FONT_BODY)
                            .size(theme::SIZE_BADGE)
                            .color(theme::palette().text3),
                    );
                }
                rows = rows.push(
                    container(card_col)
                        .padding(8)
                        .width(Fill)
                        .style(|_: &Theme| container::Style {
                            background: Some(Background::Color(theme::palette().panel2)),
                            border: Border {
                                color: theme::palette().border,
                                width: 1.0,
                                radius: theme::RADIUS_BUTTON.into(),
                            },
                            ..Default::default()
                        }),
                );
            }

            if !spec.notes.trim().is_empty() {
                rows = rows.push(
                    text(format!("Notas do arquivo: {}", spec.notes.trim()))
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_BADGE)
                        .color(theme::palette().text3),
                );
            }

            let body = scrollable(rows)
                .height(Length::Fixed(300.0))
                .width(Length::Fixed(520.0));
            let actions = row![
                Space::new().width(Fill),
                pane_close_dialog::action(
                    labels::DIALOG_CANCEL,
                    pane_close_dialog::ButtonKind::Outline,
                    Some(on_cancel.clone()),
                ),
                pane_close_dialog::action(
                    labels::AGENT_GRAPH_IMPORT_CONFIRM,
                    pane_close_dialog::ButtonKind::Primary,
                    Some(on_confirm),
                ),
            ]
            .spacing(theme::DIALOG_ACTIONS_GAP)
            .align_y(Alignment::Center);

            pane_close_dialog::form(
                heading.into(),
                body.into(),
                actions.into(),
                Some(500.0),
                on_cancel,
                on_noop,
            )
        }
    }
}
