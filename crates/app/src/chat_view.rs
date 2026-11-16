//! Visualização do pane de chat (`chat_pane.dart` e `chat_items.dart` do Flutter).
//!
//! Renderiza cabeçalho da conversa, estado vazio, coluna central de 760px com
//! mensagens do usuário, respostas do assistente (com cursor durante stream),
//! chamadas de ferramenta resumidas em linha (expansíveis) e avisos de erro.

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

use crate::chat_composer;
use crate::chat_pane::{ChatItem, ChatMsg, ChatPane, ChatToolEntry};
use crate::labels;
use crate::theme;

struct ChatMarkdownViewer<'a> {
    pane: &'a ChatPane,
}

impl<'a> iced::widget::markdown::Viewer<'a, ChatMsg> for ChatMarkdownViewer<'a> {
    fn on_link_click(url: String) -> ChatMsg {
        ChatMsg::OpenLink(url)
    }

    fn image(
        &self,
        settings: iced::widget::markdown::Settings,
        url: &'a String,
        _title: &'a str,
        alt: &iced::widget::markdown::Text,
    ) -> Element<'a, ChatMsg> {
        if let Some(bytes) = self.pane.remote_image(url) {
            return iced::widget::image(iced::widget::image::Handle::from_bytes(bytes.to_vec()))
                .width(Fill)
                .height(Length::Shrink)
                .into();
        }
        let local = url.strip_prefix("file://").unwrap_or(url);
        if std::path::Path::new(local).is_file() {
            return iced::widget::image(iced::widget::image::Handle::from_path(local))
                .width(Fill)
                .height(Length::Shrink)
                .into();
        }
        container(
            iced::widget::rich_text(alt.spans(settings.style))
                .on_link_click(ChatMarkdownViewer::on_link_click),
        )
        .padding(4)
        .into()
    }
}

/// Largura máxima da coluna central da transcrição (`columnWidth = 760` em `chat_pane.dart`).
pub const TRANSCRIPT_MAX_WIDTH: f32 = 760.0;
/// Largura máxima do balão de mensagem do usuário (`maxWidth: 580` em `chat_items.dart`).
const USER_BUBBLE_MAX_WIDTH: f32 = 580.0;
/// Máximo de linhas exibidas no resultado expandido de uma ferramenta (`_maxResultLines = 14`).
const MAX_TOOL_RESULT_LINES: usize = 14;

/// Renderiza o [`ChatPane`] completo (cabeçalho + transcrição/vazio + composer).
pub fn view(pane: &ChatPane, transcript_width: f32) -> Element<'_, ChatMsg> {
    let header = view_header(pane);

    let center: Element<'_, ChatMsg> = if pane.items().is_empty() {
        view_empty(pane)
    } else {
        view_transcript(pane, transcript_width)
    };

    let composer_wrap = container(
        container(chat_composer::view(pane))
            .max_width(transcript_width)
            .width(Fill),
    )
    .width(Fill)
    .center_x(Fill)
    .padding(Padding {
        top: 0.0,
        right: 24.0,
        bottom: 10.0,
        left: 24.0,
    });

    container(
        column![header, view_history(pane), center, composer_wrap]
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        ..Default::default()
    })
    .into()
}

fn view_header(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let title: Element<'_, ChatMsg> = if pane.rename_open() {
        row![
            iced::widget::text_input(labels::CHAT_RENAME, pane.rename_text())
                .on_input(ChatMsg::RenameChanged)
                .on_submit(ChatMsg::SaveRename)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_BODY)
                .padding([5, 8])
                .style(chat_input_style),
            button(text("✓"))
                .on_press(ChatMsg::SaveRename)
                .style(theme::ghost_button),
            button(text("×"))
                .on_press(ChatMsg::CancelRename)
                .style(theme::ghost_button)
        ]
        .into()
    } else {
        button(
            text(pane.title().unwrap_or_else(|| pane.conversation_title()))
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_RAIL_ITEM)
                .color(theme::palette().text),
        )
        .padding([2, 4])
        .on_press(ChatMsg::BeginRename)
        .style(theme::ghost_button)
        .into()
    };
    let content = row![
        text(labels::ICON_LAPTOP)
            .font(theme::FONT_ICON)
            .size(theme::SIZE_TITLE)
            .color(theme::palette().text2),
        Space::new().width(8.0),
        title,
        Space::new().width(10.0),
        container(
            text(pane.folder_label())
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text2),
        )
        .padding([2, 6])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel3)),
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        }),
        Space::new().width(Fill),
        button(row![
            text(labels::ICON_HISTORY)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TAB),
            Space::new().width(4),
            text(labels::CHAT_RESUME).size(theme::SIZE_SECTION_HEADER)
        ])
        .padding([3, 6])
        .on_press(ChatMsg::ToggleHistory)
        .style(theme::ghost_button),
        button(row![
            text(labels::ICON_PHONE_IPHONE)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TAB),
            Space::new().width(4),
            text(labels::CHAT_REMOTE_CONTROL).size(theme::SIZE_SECTION_HEADER)
        ])
        .padding([3, 6])
        .on_press(ChatMsg::HandOff)
        .style(theme::ghost_button),
    ]
    .height(Length::Fixed(37.0))
    .padding([0, 14])
    .align_y(Alignment::Center);

    let bottom_border = container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        });

    column![content, bottom_border]
        .width(Fill)
        .height(Length::Fixed(38.0))
        .into()
}

fn view_history(pane: &ChatPane) -> Element<'_, ChatMsg> {
    if !pane.history_open() {
        return Space::new().height(0.0).into();
    }
    let sessions = pane.recent_sessions();
    let mut list = column![text(labels::CHAT_RECENT_SESSIONS)
        .font(theme::FONT_BODY_MEDIUM)
        .size(theme::SIZE_LABEL)];
    if sessions.is_empty() {
        list = list.push(text(labels::CHAT_RESUME_NONE).size(theme::SIZE_TAB));
    }
    for session in sessions.iter().take(30) {
        list = list.push(
            button(
                text(format!(
                    "{} · {} · {}",
                    session.title,
                    gsh_core::tabs::basename(&session.project),
                    time_ago(session.last_seen_ms)
                ))
                .size(theme::SIZE_LABEL),
            )
            .on_press(ChatMsg::ResumeSession(session.id.clone()))
            .style(theme::ghost_button),
        );
    }
    container(scrollable(list.spacing(3)).height(Length::Fixed(150.0)))
        .width(Fill)
        .padding([6, 14])
        .into()
}

fn time_ago(at_ms: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let seconds = now.saturating_sub(at_ms) / 1000;
    if seconds < 60 {
        "agora".into()
    } else if seconds < 3600 {
        format!("há {} min", seconds / 60)
    } else if seconds < 86400 {
        format!("há {} h", seconds / 3600)
    } else {
        format!("há {} d", seconds / 86400)
    }
}

fn view_empty(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let subtitle = labels::format_chat_empty_subtitle(pane.folder_label());
    let hints = [
        labels::CHAT_EMPTY_HINT_SEND,
        labels::CHAT_EMPTY_HINT_PASTE,
        labels::CHAT_EMPTY_HINT_MODE,
    ];

    let mut card = column![
        container(
            text(labels::ICON_CHAT)
                .font(theme::FONT_ICON)
                .size(28.0)
                .color(theme::palette().text4),
        )
        .width(Fill)
        .center_x(Fill),
        Space::new().height(14.0),
        container(
            text(labels::CHAT_EMPTY_TITLE)
                .font(theme::FONT_DISPLAY)
                .size(18.0)
                .color(theme::palette().text),
        )
        .width(Fill)
        .center_x(Fill),
        Space::new().height(6.0),
        container(
            text(subtitle)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text3),
        )
        .width(Fill)
        .center_x(Fill),
        Space::new().height(22.0),
    ]
    .max_width(420.0)
    .width(Fill);

    let hint_icons = [
        labels::ICON_KEYBOARD_RETURN,
        labels::ICON_IMAGE,
        labels::ICON_TUNE,
    ];
    for (hint, icon) in hints.into_iter().zip(hint_icons) {
        card = card.push(
            row![
                text(icon)
                    .font(theme::FONT_ICON)
                    .size(11.25)
                    .color(theme::palette().text3),
                Space::new().width(10.0),
                text(hint)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_LABEL + 1.0)
                    .color(theme::palette().text2),
            ]
            .padding(Padding {
                top: 0.0,
                right: 0.0,
                bottom: 10.0,
                left: 0.0,
            })
            .align_y(Alignment::Center),
        );
    }

    card = card.push(Space::new().height(8.0)).push(
        container(
            button(
                row![
                    text(labels::ICON_HISTORY)
                        .font(theme::FONT_ICON)
                        .size(theme::SIZE_LABEL),
                    Space::new().width(6.0),
                    text(labels::CHAT_RESUME_EMPTY)
                        .font(theme::FONT_BODY_MEDIUM)
                        .size(theme::SIZE_LABEL),
                ]
                .align_y(Alignment::Center),
            )
            .padding([4, 9])
            .on_press(ChatMsg::ToggleHistory)
            .style(|_: &Theme, status| {
                let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: Some(Background::Color(if hovered {
                        theme::palette().panel3
                    } else {
                        theme::palette().panel2
                    })),
                    text_color: theme::palette().text,
                    border: Border {
                        color: theme::palette().border2,
                        width: 1.0,
                        radius: theme::RADIUS_BUTTON.into(),
                    },
                    ..Default::default()
                }
            }),
        )
        .width(Fill)
        .center_x(Fill),
    );

    container(card)
        .width(Fill)
        .height(Fill)
        .padding(24)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}

fn view_transcript(pane: &ChatPane, transcript_width: f32) -> Element<'_, ChatMsg> {
    let mut list = column![].max_width(transcript_width).width(Fill);
    let items = pane.items();
    let mut index = 0;
    while index < items.len() {
        if matches!(items.get(index),Some(ChatItem::Tool(tool)) if is_edit_tool(tool)) {
            let start = index;
            while index < items.len()
                && matches!(items.get(index),Some(ChatItem::Tool(tool)) if is_edit_tool(tool))
            {
                index += 1;
            }
            let indices = (start..index).collect::<Vec<_>>();
            list = list.push(view_edits_card(pane, &indices));
            if index < items.len() {
                list = list.push(Space::new().height(14.0));
            }
        } else if let Some(item) = items.get(index) {
            list = list.push(view_item(index, item, pane));
            if let Some(next) = items.get(index.saturating_add(1)) {
                list = list.push(Space::new().height(gap_between(item, next)));
            }
            index += 1;
        } else {
            break;
        }
    }

    let centered = container(list).width(Fill).center_x(Fill).padding([24, 24]);

    scrollable(centered).width(Fill).height(Fill).into()
}

fn is_edit_tool(tool: &ChatToolEntry) -> bool {
    matches!(tool.name.as_str(), "Edit" | "MultiEdit" | "Write")
}

fn edit_hunks(tool: &ChatToolEntry) -> Vec<(String, String)> {
    let value = |key: &str| {
        tool.input
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    match tool.name.as_str() {
        "Edit" => vec![(value("old_string"), value("new_string"))],
        "Write" => vec![(String::new(), value("content"))],
        "MultiEdit" => tool
            .input
            .get("edits")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|edit| {
                Some((
                    edit.get("old_string")?.as_str()?.to_string(),
                    edit.get("new_string")?.as_str()?.to_string(),
                ))
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn source_lines(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        text.lines().count()
    }
}

fn view_edits_card<'a>(pane: &'a ChatPane, indices: &[usize]) -> Element<'a, ChatMsg> {
    let tools = indices
        .iter()
        .filter_map(|i| match pane.items().get(*i) {
            Some(ChatItem::Tool(t)) => Some((*i, t)),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (idx, tool) in &tools {
        let path = tool
            .input
            .get("file_path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(&tool.summary)
            .to_string();
        if let Some((_, ids)) = groups.iter_mut().find(|(p, _)| p == &path) {
            ids.push(*idx);
        } else {
            groups.push((path, vec![*idx]));
        }
    }
    let totals = tools
        .iter()
        .fold((0usize, 0usize), |(added, removed), (_, tool)| {
            let (a, r) = edit_hunks(tool)
                .iter()
                .fold((0, 0), |(a, r), (before, after)| {
                    (a + source_lines(after), r + source_lines(before))
                });
            (added + a, removed + r)
        });
    let all_done = tools.iter().all(|(_, t)| t.done);
    let card_index = indices.first().copied().unwrap_or(0);
    let collapsed = pane.edit_card_collapsed(card_index);
    let mut rows = column![button(
        text(format!(
            "{} {} arquivos   +{}  -{}",
            if all_done {
                "✓"
            } else {
                labels::ICON_TOOL_RUNNING
            },
            labels::CHAT_EDITED_FILES,
            totals.0,
            totals.1
        ))
        .size(theme::SIZE_LABEL)
    )
    .on_press(ChatMsg::ToggleEditCard(card_index))
    .style(theme::ghost_button)]
    .spacing(3);
    if !collapsed {
        for (path, ids) in groups {
            let sums = ids
                .iter()
                .filter_map(|i| match pane.items().get(*i) {
                    Some(ChatItem::Tool(t)) => Some(edit_hunks(t)),
                    _ => None,
                })
                .flatten()
                .fold((0usize, 0usize), |(a, r), (before, after)| {
                    (a + source_lines(&after), r + source_lines(&before))
                });
            let opened = ids
                .iter()
                .any(|i| matches!(pane.items().get(*i),Some(ChatItem::Tool(t)) if t.expanded));
            rows = rows.push(
                button(
                    text(format!(
                        "{} {}   +{}  -{}",
                        if opened { "⌄" } else { "›" },
                        gsh_core::tabs::basename(&path),
                        sums.0,
                        sums.1
                    ))
                    .size(theme::SIZE_TAB),
                )
                .on_press(ChatMsg::ToggleEditTools(ids.clone()))
                .style(theme::ghost_button),
            );
            if opened {
                for idx in ids {
                    if let Some(ChatItem::Tool(tool)) = pane.items().get(idx) {
                        for (before, after) in edit_hunks(tool) {
                            for line in before.lines().take(40) {
                                rows = rows.push(
                                    text(format!("− {line}"))
                                        .font(theme::FONT_MONO)
                                        .size(theme::SIZE_TAB)
                                        .color(theme::palette().error),
                                );
                            }
                            for line in after.lines().take(40) {
                                rows = rows.push(
                                    text(format!("+ {line}"))
                                        .font(theme::FONT_MONO)
                                        .size(theme::SIZE_TAB)
                                        .color(theme::palette().online),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    container(rows)
        .width(Fill)
        .padding(4)
        .style(|_: &Theme| container::Style {
            border: Border {
                color: theme::palette().border2,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn gap_between(a: &ChatItem, b: &ChatItem) -> f32 {
    if matches!(a, ChatItem::User { .. }) || matches!(b, ChatItem::User { .. }) {
        24.0
    } else if matches!(a, ChatItem::Tool(_)) && matches!(b, ChatItem::Tool(_)) {
        4.0
    } else {
        14.0
    }
}

fn view_item<'a>(idx: usize, item: &'a ChatItem, pane: &'a ChatPane) -> Element<'a, ChatMsg> {
    match item {
        ChatItem::User { text: msg } => view_user_bubble(msg),
        ChatItem::Assistant {
            text: msg,
            streaming,
        } => view_assistant_text(msg, *streaming, pane.markdown(idx), pane),
        ChatItem::Tool(tool) => view_tool_line(idx, tool),
        ChatItem::Error { message } => view_error_notice(message),
        ChatItem::Thinking { text: thinking } => container(
            text(thinking.as_str())
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        )
        .width(Fill)
        .into(),
        ChatItem::Permission {
            request_id,
            tool,
            description,
            always_rule,
            input,
            question,
            options,
        } => {
            let mut actions = row![
                button(text(labels::CHAT_ALLOW))
                    .on_press(ChatMsg::PermissionAnswer {
                        request_id: request_id.clone(),
                        allow: true,
                        always: false
                    })
                    .style(theme::ghost_button),
                button(text(labels::CHAT_DENY))
                    .on_press(ChatMsg::PermissionAnswer {
                        request_id: request_id.clone(),
                        allow: false,
                        always: false
                    })
                    .style(theme::ghost_button)
            ]
            .spacing(6);
            if always_rule.is_some() {
                actions = actions.push(
                    button(text(labels::CHAT_ALWAYS))
                        .on_press(ChatMsg::PermissionAnswer {
                            request_id: request_id.clone(),
                            allow: true,
                            always: true,
                        })
                        .style(theme::ghost_button),
                );
            }
            if let Some(question_text) = question {
                actions = actions.push(
                    text(question_text)
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_LABEL),
                );
                for (label, option_description) in options {
                    let mut updated = input.clone();
                    updated.insert(
                        "answers".to_string(),
                        serde_json::json!({(question_text): label}),
                    );
                    actions = actions.push(
                        button(
                            text(format!("{label}  {option_description}")).size(theme::SIZE_LABEL),
                        )
                        .on_press(ChatMsg::AnswerQuestion {
                            request_id: request_id.clone(),
                            input: updated,
                        })
                        .style(theme::ghost_button),
                    );
                }
            }
            container(
                column![
                    text(format!("{tool}: {description}"))
                        .font(theme::FONT_BODY_MEDIUM)
                        .size(theme::SIZE_BODY),
                    actions
                ]
                .spacing(8),
            )
            .padding(10)
            .width(Fill)
            .into()
        }
        ChatItem::Questions {
            request_id,
            input,
            questions,
            answered,
        } => {
            let drafts = input
                .get("draftAnswers")
                .and_then(serde_json::Value::as_object);
            let mut card = column![text(labels::CHAT_QUESTION_TITLE)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_BODY)];
            for q in questions {
                if !q.header.is_empty() {
                    card = card.push(
                        text(q.header.to_uppercase())
                            .font(theme::FONT_BODY_SEMIBOLD)
                            .size(theme::SIZE_LABEL)
                            .color(theme::palette().text2),
                    );
                }
                card = card.push(
                    text(q.question.as_str())
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_BODY),
                );
                if let Some(answer) = answered.as_ref().and_then(|a| a.get(&q.question)) {
                    card = card.push(text(format!("→ {answer}")).color(theme::palette().online));
                } else {
                    for (label, description) in &q.options {
                        let selected = drafts
                            .and_then(|a| a.get(&q.question))
                            .and_then(|v| v.get("selected"))
                            .and_then(serde_json::Value::as_array)
                            .is_some_and(|a| a.iter().any(|v| v.as_str() == Some(label)));
                        card = card.push(
                            button(
                                text(format!(
                                    "{} {}  {}",
                                    if q.multi_select {
                                        if selected {
                                            "☑"
                                        } else {
                                            "☐"
                                        }
                                    } else if selected {
                                        "◉"
                                    } else {
                                        "○"
                                    },
                                    label,
                                    description
                                ))
                                .size(theme::SIZE_LABEL),
                            )
                            .on_press(ChatMsg::QuestionToggle {
                                request_id: request_id.clone(),
                                question: q.question.clone(),
                                option: label.clone(),
                            })
                            .style(theme::ghost_button),
                        );
                    }
                    let free = drafts
                        .and_then(|a| a.get(&q.question))
                        .and_then(|v| v.get("other"))
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("");
                    card = card.push(
                        iced::widget::text_input(labels::CHAT_QUESTION_FREE_TEXT, free)
                            .on_input({
                                let request_id = request_id.clone();
                                let question = q.question.clone();
                                move |value| ChatMsg::QuestionFreeText {
                                    request_id: request_id.clone(),
                                    question: question.clone(),
                                    value,
                                }
                            })
                            .font(theme::FONT_BODY)
                            .size(theme::SIZE_LABEL)
                            .padding([5, 8])
                            .style(chat_input_style),
                    );
                }
            }
            if answered.is_none() {
                card = card.push(
                    button(text(labels::CHAT_QUESTION_SEND))
                        .on_press(ChatMsg::SubmitQuestions {
                            request_id: request_id.clone(),
                        })
                        .style(theme::ghost_button),
                );
            }
            container(card.spacing(6)).padding(12).width(Fill).into()
        }
        ChatItem::Plan {
            request_id,
            plan,
            feedback,
            ..
        } => container(
            column![
                text(labels::CHAT_PLAN_TITLE)
                    .font(theme::FONT_BODY_MEDIUM)
                    .size(theme::SIZE_BODY),
                if let Some(content) = pane.markdown(idx) {
                    iced::widget::markdown::view_with(
                        content.items(),
                        iced::widget::markdown::Settings::with_text_size(
                            theme::SIZE_BODY,
                            iced::widget::markdown::Style::from(iced::Theme::Dark),
                        ),
                        &ChatMarkdownViewer { pane },
                    )
                } else {
                    text(plan.as_str())
                        .font(theme::FONT_BODY)
                        .size(theme::SIZE_BODY)
                        .into()
                },
                iced::widget::text_input(labels::CHAT_PLAN_FEEDBACK, feedback)
                    .on_input({
                        let request_id = request_id.clone();
                        move |value| ChatMsg::PlanFeedback {
                            request_id: request_id.clone(),
                            value,
                        }
                    })
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_BODY)
                    .padding([5, 8])
                    .style(chat_input_style),
                row![
                    button(text(labels::CHAT_PLAN_APPROVE))
                        .on_press(ChatMsg::PlanDecision {
                            request_id: request_id.clone(),
                            approve: true
                        })
                        .style(theme::ghost_button),
                    button(text(labels::CHAT_PLAN_REFINE))
                        .on_press(ChatMsg::PlanDecision {
                            request_id: request_id.clone(),
                            approve: false
                        })
                        .style(theme::ghost_button)
                ]
                .spacing(6)
            ]
            .spacing(8),
        )
        .padding(12)
        .width(Fill)
        .into(),
        ChatItem::ResumePrompt { session_id } => container(
            column![
                text(labels::CHAT_RESUME_LIVE_TITLE),
                row![
                    button(text(labels::CHAT_RESUME_CONFIRM))
                        .on_press(ChatMsg::ConfirmResume {
                            session_id: session_id.clone(),
                            confirm: true
                        })
                        .style(theme::ghost_button),
                    button(text(labels::CHAT_RESUME_CANCEL))
                        .on_press(ChatMsg::ConfirmResume {
                            session_id: session_id.clone(),
                            confirm: false
                        })
                        .style(theme::ghost_button)
                ]
                .spacing(6)
            ]
            .spacing(8),
        )
        .padding(10)
        .width(Fill)
        .into(),
        ChatItem::Info {
            text: info,
            warning: _,
        } => container(
            text(info.as_str())
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        )
        .width(Fill)
        .into(),
        ChatItem::Peer { from, text: peer } => container(
            text(format!("{from}: {peer}"))
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text2),
        )
        .width(Fill)
        .into(),
        ChatItem::Classified { state, detail } => container(
            text(format!("{state:?}: {detail}"))
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        )
        .width(Fill)
        .into(),
    }
}

fn view_user_bubble(msg: &str) -> Element<'_, ChatMsg> {
    let bubble = container(
        text(msg)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_BODY)
            .color(theme::palette().text),
    )
    .max_width(USER_BUBBLE_MAX_WIDTH)
    .padding([7, 12])
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel3)),
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    row![Space::new().width(Fill), bubble].width(Fill).into()
}

fn view_assistant_text<'a>(
    msg: &'a str,
    streaming: bool,
    content: Option<&'a iced::widget::markdown::Content>,
    pane: &'a ChatPane,
) -> Element<'a, ChatMsg> {
    let display = if streaming {
        format!("{msg}{}", labels::CHAT_STREAM_CURSOR)
    } else {
        msg.to_string()
    };

    if let Some(content) = content {
        let rendered = iced::widget::markdown::view_with(
            content.items(),
            iced::widget::markdown::Settings::with_text_size(
                theme::SIZE_BODY,
                iced::widget::markdown::Style::from(iced::Theme::Dark),
            ),
            &ChatMarkdownViewer { pane },
        );
        let mut blocks = column![rendered];
        if streaming {
            blocks = blocks.push(
                text(labels::CHAT_STREAM_CURSOR)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_BODY)
                    .color(theme::palette().accent_text),
            );
        }
        return container(blocks).width(Fill).into();
    }
    container(
        text(display)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_BODY)
            .color(theme::palette().text),
    )
    .width(Fill)
    .into()
}

fn view_tool_line(idx: usize, tool: &ChatToolEntry) -> Element<'_, ChatMsg> {
    let fg = if tool.is_error {
        theme::palette().error
    } else {
        theme::palette().text3
    };
    let chevron = if tool.expanded {
        labels::ICON_CHEVRON_DOWN
    } else {
        labels::ICON_CHEVRON_RIGHT
    };

    let mut line_row = row![].align_y(Alignment::Center);
    if !tool.done {
        line_row = line_row
            .push(
                text(labels::ICON_TOOL_RUNNING)
                    .font(theme::FONT_ICON)
                    .size(theme::SIZE_SECTION_HEADER)
                    .color(theme::palette().text3),
            )
            .push(Space::new().width(6.0));
    }

    line_row = line_row
        .push(
            text(&tool.name)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_BODY)
                .color(theme::palette().text2),
        )
        .push(
            text(labels::CHAT_TOOL_NAME_PAD)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_BODY),
        )
        .push(
            text(&tool.summary)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_BODY)
                .color(fg),
        )
        .push(Space::new().width(6.0))
        .push(
            text(chevron)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(fg),
        );

    let header_btn = button(line_row)
        .padding([2, 6])
        .on_press(ChatMsg::ToggleTool(idx))
        .style(|_: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if hovered {
                    Some(Background::Color(theme::palette().panel2))
                } else {
                    None
                },
                text_color: theme::palette().text2,
                border: Border {
                    radius: theme::RADIUS_BUTTON.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    if !tool.expanded {
        return header_btn.into();
    }

    let detail_header =
        labels::format_chat_tool_detail(&tool.name, &tool.summary, tool.command.as_deref());
    let mut details_col = column![text(detail_header)
        .font(theme::FONT_MONO)
        .size(theme::SIZE_LABEL)
        .color(theme::palette().text2)];

    if matches!(tool.name.as_str(), "Edit" | "MultiEdit" | "Write") {
        if let Some(path) = tool
            .input
            .get("file_path")
            .and_then(serde_json::Value::as_str)
        {
            details_col = details_col.push(
                text(path.to_string())
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_TAB)
                    .color(theme::palette().text3),
            );
        }
        if let Some(old) = tool
            .input
            .get("old_string")
            .and_then(serde_json::Value::as_str)
        {
            details_col = details_col.push(
                text(format!("− {old}"))
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().error),
            );
        }
        if let Some(new) = tool
            .input
            .get("new_string")
            .and_then(serde_json::Value::as_str)
        {
            details_col = details_col.push(
                text(format!("+ {new}"))
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().online),
            );
        }
    }

    let trimmed_res = tool.result.trim();
    if !trimmed_res.is_empty() {
        let truncated: String = trimmed_res
            .lines()
            .take(MAX_TOOL_RESULT_LINES)
            .collect::<Vec<_>>()
            .join("\n");
        details_col = details_col.push(Space::new().height(8.0)).push(
            text(truncated)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_LABEL)
                .color(if tool.is_error {
                    theme::palette().error
                } else {
                    theme::palette().text3
                }),
        );
    }

    let details_box = container(details_col)
        .width(Fill)
        .padding([10, 12])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                color: theme::palette().border,
                width: 1.0,
                radius: theme::RADIUS_CARD.into(),
            },
            ..Default::default()
        });

    column![header_btn, Space::new().height(6.0), details_box]
        .width(Fill)
        .into()
}

fn view_error_notice(message: &str) -> Element<'_, ChatMsg> {
    container(
        text(message)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().error),
    )
    .width(Fill)
    .into()
}

fn chat_input_style(
    _: &Theme,
    _: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    iced::widget::text_input::Style {
        background: Background::Color(theme::palette().panel2),
        border: Border {
            color: theme::palette().border2,
            width: 1.0,
            radius: theme::RADIUS_BUTTON.into(),
        },
        icon: theme::palette().text3,
        placeholder: theme::palette().text4,
        value: theme::palette().text,
        selection: theme::palette().accent_soft,
    }
}
