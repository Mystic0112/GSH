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

/// Largura máxima da coluna central da transcrição (`columnWidth = 760` em `chat_pane.dart`).
pub const TRANSCRIPT_MAX_WIDTH: f32 = 760.0;
/// Largura máxima do balão de mensagem do usuário (`maxWidth: 580` em `chat_items.dart`).
const USER_BUBBLE_MAX_WIDTH: f32 = 580.0;
/// Máximo de linhas exibidas no resultado expandido de uma ferramenta (`_maxResultLines = 14`).
const MAX_TOOL_RESULT_LINES: usize = 14;

/// Renderiza o [`ChatPane`] completo (cabeçalho + transcrição/vazio + composer).
pub fn view(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let header = view_header(pane);

    let center: Element<'_, ChatMsg> = if pane.items().is_empty() {
        view_empty(pane)
    } else {
        view_transcript(pane)
    };

    let composer_wrap = container(
        container(chat_composer::view(pane))
            .max_width(TRANSCRIPT_MAX_WIDTH)
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
        column![header, center, composer_wrap]
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
    let content = row![
        text(labels::ICON_CHAT)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_TITLE)
            .color(theme::palette().text2),
        Space::new().width(8.0),
        text(pane.conversation_title())
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::SIZE_RAIL_ITEM)
            .color(theme::palette().text),
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
                .font(theme::FONT_MONO)
                .size(theme::SIZE_DISPLAY)
                .color(theme::palette().text4),
        )
        .width(Fill)
        .center_x(Fill),
        Space::new().height(12.0),
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
        Space::new().height(20.0),
    ]
    .max_width(420.0)
    .width(Fill);

    for hint in hints {
        card = card.push(
            row![
                text(labels::ICON_CHEVRON_RIGHT)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text3),
                Space::new().width(8.0),
                text(hint)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_TITLE)
                    .color(theme::palette().text2),
            ]
            .padding(Padding {
                top: 0.0,
                right: 0.0,
                bottom: 8.0,
                left: 0.0,
            })
            .align_y(Alignment::Center),
        );
    }

    container(card)
        .width(Fill)
        .height(Fill)
        .padding(24)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}

fn view_transcript(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut list = column![].max_width(TRANSCRIPT_MAX_WIDTH).width(Fill);
    let items = pane.items();

    for (idx, item) in items.iter().enumerate() {
        list = list.push(view_item(idx, item));
        if let Some(next) = items.get(idx.saturating_add(1)) {
            list = list.push(Space::new().height(gap_between(item, next)));
        }
    }

    let centered = container(list).width(Fill).center_x(Fill).padding([24, 24]);

    scrollable(centered).width(Fill).height(Fill).into()
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

fn view_item(idx: usize, item: &ChatItem) -> Element<'_, ChatMsg> {
    match item {
        ChatItem::User { text: msg } => view_user_bubble(msg),
        ChatItem::Assistant {
            text: msg,
            streaming,
        } => view_assistant_text(msg, *streaming),
        ChatItem::Tool(tool) => view_tool_line(idx, tool),
        ChatItem::Error { message } => view_error_notice(message),
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

fn view_assistant_text(msg: &str, streaming: bool) -> Element<'_, ChatMsg> {
    let display = if streaming {
        format!("{msg}{}", labels::CHAT_STREAM_CURSOR)
    } else {
        msg.to_string()
    };

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
                    .font(theme::FONT_MONO)
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
