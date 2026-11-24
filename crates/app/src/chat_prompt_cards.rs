//! Cartões de permissão e perguntas do chat, portados de `chat_prompt_cards.dart`.

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Element, Fill, Theme};

use crate::chat_pane::ChatMsg;
use crate::{labels, theme};

fn chat_input_style(_: &Theme, _: text_input::Status) -> text_input::Style {
    text_input::Style {
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
pub(crate) fn prompt_frame<'a>(
    pending: bool,
    icon: &'static str,
    title: String,
    body: Element<'a, ChatMsg>,
) -> Element<'a, ChatMsg> {
    container(
        column![
            row![
                text(icon)
                    .font(theme::FONT_ICON)
                    .size(15)
                    .color(if pending {
                        theme::palette().warn
                    } else {
                        theme::palette().text3
                    }),
                Space::new().width(8.0),
                text(title)
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(14)
                    .color(theme::palette().text)
            ]
            .align_y(Alignment::Center),
            Space::new().height(10.0),
            body
        ]
        .width(Fill),
    )
    .width(Fill)
    .padding(iced::Padding {
        top: 12.0,
        right: 14.0,
        bottom: 14.0,
        left: 14.0,
    })
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel2)),
        border: Border {
            color: if pending {
                theme::with_alpha(theme::palette().warn, 0.55)
            } else {
                theme::palette().border2
            },
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    })
    .into()
}

pub(crate) fn prompt_outcome(text_value: String, color: iced::Color) -> Element<'static, ChatMsg> {
    text(text_value)
        .font(theme::FONT_BODY)
        .size(13)
        .color(color)
        .into()
}

pub(crate) fn primary_prompt_button(label: String, message: ChatMsg) -> Element<'static, ChatMsg> {
    button(
        text(label)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::SIZE_LABEL),
    )
    .padding([5, 10])
    .on_press(message)
    .style(|_: &Theme, _status| iced::widget::button::Style {
        background: Some(Background::Color(theme::palette().accent)),
        text_color: theme::on_color(theme::palette().accent),
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

pub(crate) fn outline_prompt_button(label: String, message: ChatMsg) -> Element<'static, ChatMsg> {
    button(text(label).font(theme::FONT_BODY).size(theme::SIZE_LABEL))
        .padding([5, 10])
        .on_press(message)
        .style(|_: &Theme, status| {
            let hovered = matches!(
                status,
                iced::widget::button::Status::Hovered | iced::widget::button::Status::Pressed
            );
            iced::widget::button::Style {
                background: hovered.then_some(Background::Color(theme::palette().panel3)),
                text_color: theme::palette().text,
                border: Border {
                    color: theme::palette().border2,
                    width: 1.0,
                    radius: theme::RADIUS_BUTTON.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

pub(crate) struct PermissionCard<'a> {
    pub(crate) request_id: &'a str,
    pub(crate) tool: &'a str,
    pub(crate) description: &'a str,
    pub(crate) always_rule: Option<&'a str>,
    pub(crate) allowed: Option<bool>,
    pub(crate) always_allowed: bool,
    pub(crate) input: &'a serde_json::Map<String, serde_json::Value>,
    pub(crate) question: Option<&'a str>,
    pub(crate) options: &'a [(String, String)],
}

pub(crate) fn view_permission_card(card: PermissionCard<'_>) -> Element<'static, ChatMsg> {
    let PermissionCard {
        request_id,
        tool,
        description,
        always_rule,
        allowed,
        always_allowed,
        input,
        question,
        options,
    } = card;
    let mut body = column![container(
        text(description.to_owned())
            .font(theme::FONT_MONO)
            .size(12)
            .color(theme::palette().text2),
    )
    .width(Fill)
    .padding([6, 10])
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })]
    .spacing(8);

    if let Some(diff) = permission_edit_diff(tool, input) {
        body = body.push(diff);
    }

    if let Some(question_text) = question {
        body = body.push(
            text(question_text.to_owned())
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL),
        );
        for (label, option_description) in options {
            let mut updated = input.clone();
            updated.insert(
                "answers".to_string(),
                serde_json::json!({(question_text): label}),
            );
            body = body.push(outline_prompt_button(
                format!("{label}  {option_description}"),
                ChatMsg::AnswerQuestion {
                    request_id: request_id.to_owned(),
                    input: updated,
                },
            ));
        }
    }

    if let Some(allowed) = allowed {
        let result = if !allowed {
            labels::CHAT_DENIED.to_owned()
        } else if always_allowed {
            labels::chat_allowed_always(always_rule.unwrap_or(""))
        } else {
            labels::CHAT_ALLOWED.to_owned()
        };
        body = body.push(prompt_outcome(
            result,
            if allowed {
                theme::palette().online
            } else {
                theme::palette().error
            },
        ));
    } else {
        let mut actions = row![primary_prompt_button(
            labels::CHAT_ALLOW.to_owned(),
            ChatMsg::PermissionAnswer {
                request_id: request_id.to_owned(),
                allow: true,
                always: false,
            },
        )]
        .spacing(8);
        if let Some(rule) = always_rule {
            actions = actions.push(outline_prompt_button(
                labels::chat_allow_always(rule),
                ChatMsg::PermissionAnswer {
                    request_id: request_id.to_owned(),
                    allow: true,
                    always: true,
                },
            ));
        }
        actions = actions.push(outline_prompt_button(
            labels::CHAT_DENY.to_owned(),
            ChatMsg::PermissionAnswer {
                request_id: request_id.to_owned(),
                allow: false,
                always: false,
            },
        ));
        body = body.push(actions);
    }

    prompt_frame(
        allowed.is_none(),
        labels::ICON_SHIELD,
        format!(
            "{}{}{}",
            labels::CHAT_PROMPT_ASK_PREFIX,
            tool,
            labels::CHAT_PROMPT_ASK_SUFFIX
        ),
        body.into(),
    )
}

fn permission_edit_diff(
    tool: &str,
    input: &serde_json::Map<String, serde_json::Value>,
) -> Option<Element<'static, ChatMsg>> {
    fn string(value: Option<&serde_json::Value>) -> &str {
        value.and_then(serde_json::Value::as_str).unwrap_or("")
    }
    let edits: Vec<(String, String)> = match tool {
        "Edit" => vec![(
            string(input.get("old_string")).to_owned(),
            string(input.get("new_string")).to_owned(),
        )],
        "MultiEdit" => input
            .get("edits")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|edit| {
                let edit = edit.as_object()?;
                Some((
                    string(edit.get("old_string")).to_owned(),
                    string(edit.get("new_string")).to_owned(),
                ))
            })
            .collect(),
        "Write" => vec![(String::new(), string(input.get("content")).to_owned())],
        _ => Vec::new(),
    };
    if edits.is_empty() {
        return None;
    }

    let mut rows = column![].spacing(0);
    let mut count = 0;
    for (index, (before, after)) in edits.iter().enumerate() {
        if index > 0 {
            rows = rows.push(text("⋯").color(theme::palette().text4));
        }
        for line in before
            .lines()
            .map(|line| (format!("- {line}"), theme::palette().error))
            .chain(
                after
                    .lines()
                    .map(|line| (format!("+ {line}"), theme::palette().online)),
            )
        {
            if count >= 40 {
                break;
            }
            rows = rows.push(
                container(
                    text(line.0)
                        .font(theme::FONT_MONO)
                        .size(11.5)
                        .color(theme::palette().text2),
                )
                .width(Fill)
                .padding([1, 8])
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(theme::with_alpha(line.1, 0.12))),
                    ..Default::default()
                }),
            );
            count += 1;
        }
    }
    Some(
        container(rows)
            .width(Fill)
            .padding([4, 0])
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::palette().panel)),
                border: Border {
                    color: theme::palette().border,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                ..Default::default()
            })
            .into(),
    )
}

pub(crate) fn view_question_card<'a>(
    request_id: &'a str,
    input: &'a serde_json::Map<String, serde_json::Value>,
    questions: &'a [crate::chat_pane::ChatQuestion],
    answered: Option<&'a std::collections::BTreeMap<String, String>>,
) -> Element<'a, ChatMsg> {
    let drafts = input
        .get("draftAnswers")
        .and_then(serde_json::Value::as_object);
    let ready = questions.iter().all(|question| {
        let draft = drafts.and_then(|all| all.get(&question.question));
        let selected = draft
            .and_then(|answer| answer.get("selected"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|values| !values.is_empty());
        let other = draft
            .and_then(|answer| answer.get("other"))
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| !value.trim().is_empty());
        selected || other
    });
    let mut body = column![].spacing(6);
    for question in questions {
        if !question.header.is_empty() {
            body = body.push(
                text(question.header.to_uppercase())
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(10.5)
                    .color(theme::palette().text3),
            );
        }
        body = body.push(
            text(question.question.clone())
                .font(theme::FONT_BODY)
                .size(theme::SIZE_BODY)
                .color(theme::palette().text),
        );
        if let Some(answer) = answered.and_then(|values| values.get(&question.question)) {
            body = body.push(
                text(format!("→ {answer}"))
                    .font(theme::FONT_BODY)
                    .color(theme::palette().online),
            );
        } else {
            for (label, description) in &question.options {
                let selected = drafts
                    .and_then(|all| all.get(&question.question))
                    .and_then(|answer| answer.get("selected"))
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(label)));
                let icon = if question.multi_select {
                    if selected {
                        labels::ICON_CHECK_BOX
                    } else {
                        labels::ICON_CHECK_BOX_OUTLINE_BLANK
                    }
                } else if selected {
                    labels::ICON_RADIO_BUTTON_CHECKED
                } else {
                    labels::ICON_RADIO_BUTTON_UNCHECKED
                };
                let mut option = column![text(label.clone())
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_BODY)
                    .color(theme::palette().text)];
                if !description.is_empty() {
                    option = option.push(
                        text(description.clone())
                            .font(theme::FONT_BODY)
                            .size(theme::SIZE_LABEL)
                            .color(theme::palette().text3),
                    );
                }
                body = body.push(
                    button(
                        row![
                            text(icon)
                                .font(theme::FONT_ICON)
                                .size(16)
                                .color(if selected {
                                    theme::palette().accent_text
                                } else {
                                    theme::palette().text3
                                }),
                            Space::new().width(8.0),
                            option
                        ]
                        .align_y(Alignment::Center),
                    )
                    .width(Fill)
                    .padding([4, 8])
                    .on_press(ChatMsg::QuestionToggle {
                        request_id: request_id.to_owned(),
                        question: question.question.clone(),
                        option: label.clone(),
                    })
                    .style(move |_: &Theme, status| {
                        let background = if selected {
                            Some(Background::Color(theme::palette().accent_soft))
                        } else if matches!(status, iced::widget::button::Status::Hovered) {
                            Some(Background::Color(theme::palette().panel3))
                        } else {
                            None
                        };
                        iced::widget::button::Style {
                            background,
                            text_color: theme::palette().text,
                            border: Border {
                                radius: 6.0.into(),
                                ..Default::default()
                            },
                            ..Default::default()
                        }
                    }),
                );
            }
            let free = drafts
                .and_then(|all| all.get(&question.question))
                .and_then(|answer| answer.get("other"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            body = body.push(
                iced::widget::text_input(labels::CHAT_QUESTION_FREE_TEXT, free)
                    .on_input({
                        let request_id = request_id.to_owned();
                        let question = question.question.clone();
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
        body = body.push(Space::new().height(4.0));
    }
    if answered.is_none() {
        let submit = button(
            text(labels::CHAT_QUESTION_SEND)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL),
        )
        .padding([5, 10])
        .on_press_maybe(ready.then(|| ChatMsg::SubmitQuestions {
            request_id: request_id.to_owned(),
        }))
        .style(|_: &Theme, _status| iced::widget::button::Style {
            background: Some(Background::Color(theme::palette().accent)),
            text_color: theme::on_color(theme::palette().accent),
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        });
        body = body.push(submit);
    }

    prompt_frame(
        answered.is_none(),
        labels::ICON_HELP_OUTLINE,
        labels::CHAT_QUESTION_TITLE.to_owned(),
        body.into(),
    )
}
