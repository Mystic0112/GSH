//! Visualização do pane de chat (`chat_pane.dart` e `chat_items.dart` do Flutter).
//!
//! Renderiza cabeçalho da conversa, estado vazio, coluna central de 760px com
//! mensagens do usuário, respostas do assistente (com cursor durante stream),
//! chamadas de ferramenta resumidas em linha (expansíveis) e avisos de erro.

use gsh_chat::SessionState;
use iced::widget::{button, column, container, row, scrollable, text, tooltip, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

use crate::chat_composer;
use crate::chat_pane::{ChatItem, ChatMsg, ChatPane, ChatToolEntry};
use crate::chat_prompt_cards::{
    outline_prompt_button, primary_prompt_button, prompt_frame, prompt_outcome,
};
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

    fn code_block(
        &self,
        settings: iced::widget::markdown::Settings,
        _language: Option<&'a str>,
        _code: &'a str,
        lines: &'a [iced::widget::markdown::Text],
    ) -> Element<'a, ChatMsg> {
        let lines = column(lines.iter().map(|line| {
            iced::widget::rich_text(line.spans(settings.style))
                .font(settings.style.code_block_font)
                .size(settings.code_size)
                .into()
        }));
        let code = scrollable(container(lines).padding(settings.code_size)).direction(
            scrollable::Direction::Horizontal(
                scrollable::Scrollbar::default()
                    .width(settings.code_size / 2)
                    .scroller_width(settings.code_size / 2),
            ),
        );

        container(code)
            .width(Fill)
            .padding(settings.code_size / 4)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::palette().bg)),
                border: Border {
                    color: theme::palette().border,
                    width: 1.0,
                    radius: 7.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

fn chat_markdown_settings() -> iced::widget::markdown::Settings {
    let active_theme = match theme::active() {
        theme::ThemeVariant::Light => iced::Theme::Light,
        theme::ThemeVariant::Dark => iced::Theme::Dark,
    };
    let palette = theme::palette();
    let mut settings = iced::widget::markdown::Settings::with_text_size(
        theme::SIZE_BODY,
        iced::widget::markdown::Style::from(active_theme),
    );
    settings.h1_size = 22.0.into();
    settings.h2_size = 18.0.into();
    settings.h3_size = 16.0.into();
    settings.h4_size = 14.5.into();
    settings.h5_size = 13.5.into();
    settings.h6_size = 12.5.into();
    settings.code_size = 12.0.into();
    settings.style.font = theme::FONT_BODY;
    settings.style.inline_code_font = theme::FONT_MONO;
    settings.style.code_block_font = theme::FONT_MONO;
    settings.style.inline_code_color = palette.text;
    settings.style.inline_code_highlight.background = palette.panel3.into();
    settings.style.link_color = palette.accent_text;
    settings
}

/// Largura máxima da coluna central da transcrição (`columnWidth = 760` em `chat_pane.dart`).
pub const TRANSCRIPT_MAX_WIDTH: f32 = 760.0;
/// Largura máxima do balão de mensagem do usuário (`maxWidth: 580` em `chat_items.dart`).
const USER_BUBBLE_MAX_WIDTH: f32 = 580.0;
/// Máximo de linhas exibidas no resultado expandido de uma ferramenta (`_maxResultLines = 14`).
const MAX_TOOL_RESULT_LINES: usize = 14;

/// Renderiza o [`ChatPane`] completo (cabeçalho + transcrição/vazio + composer).
pub fn view(
    pane: &ChatPane,
    transcript_width: f32,
    show_session_state: bool,
) -> Element<'_, ChatMsg> {
    let header = view_header(pane, show_session_state);

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

    let chat = container(
        column![header, view_history(pane), center, composer_wrap]
            .width(Fill)
            .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        ..Default::default()
    });

    if pane.file_drag_over() {
        iced::widget::stack(vec![
            chat.into(),
            container(Space::new())
                .width(Fill)
                .height(Fill)
                .style(|_: &Theme| container::Style {
                    background: Some(Background::Color(theme::palette().accent_soft)),
                    border: Border {
                        color: theme::palette().accent,
                        width: 2.0,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .into(),
        ])
        .width(Fill)
        .height(Fill)
        .into()
    } else {
        chat.into()
    }
}

fn view_header(pane: &ChatPane, show_session_state: bool) -> Element<'_, ChatMsg> {
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
    let session_badge = if show_session_state {
        pane.session_state()
            .map(|(state, detail)| view_session_state_badge(state, detail))
            .unwrap_or_else(|| Space::new().width(0.0).into())
    } else {
        Space::new().width(0.0).into()
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
        session_badge,
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

fn view_session_state_badge(state: SessionState, detail: &str) -> Element<'_, ChatMsg> {
    let color = match state {
        SessionState::Completed => theme::palette().online,
        SessionState::ReviewReady => theme::palette().accent_text,
        SessionState::Blocked => theme::palette().warn,
        SessionState::Failed => theme::palette().error,
    };
    let label = labels::chat_session_state(state);
    let badge: Element<'_, ChatMsg> = container(
        row![
            container(Space::new().width(6.0).height(6.0)).style(move |_: &Theme| {
                container::Style {
                    background: Some(Background::Color(color)),
                    border: Border {
                        radius: 3.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }),
            text(label).size(theme::SIZE_TAB).color(color),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([3, 8])
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(theme::with_alpha(color, 0.12))),
        border: Border {
            radius: 10.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into();
    if detail.is_empty() {
        badge
    } else {
        tooltip(
            badge,
            crate::pane_chrome::tip_bubble(detail),
            tooltip::Position::Bottom,
        )
        .into()
    }
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

    scrollable(centered)
        .id(pane.transcript_scroll_id())
        .on_scroll(|viewport| ChatMsg::TranscriptScrolled(transcript_near_end(viewport)))
        .width(Fill)
        .height(Fill)
        .into()
}

fn transcript_near_end(viewport: scrollable::Viewport) -> bool {
    transcript_remaining_height(
        viewport.content_bounds().height,
        viewport.bounds().height,
        viewport.absolute_offset().y,
    ) <= 80.0
}

fn transcript_remaining_height(content_height: f32, viewport_height: f32, offset_y: f32) -> f32 {
    content_height - viewport_height - offset_y
}

#[cfg(test)]
mod transcript_scroll_tests {
    use super::transcript_remaining_height;

    #[test]
    fn follows_stream_only_when_within_eighty_pixels_of_end() {
        assert!(transcript_remaining_height(1_000.0, 500.0, 420.0) <= 80.0);
        assert!(transcript_remaining_height(1_000.0, 500.0, 500.0) <= 80.0);
        assert!(transcript_remaining_height(400.0, 500.0, 0.0) <= 80.0);
        assert!(transcript_remaining_height(1_000.0, 500.0, 419.0) > 80.0);
    }
}

fn is_edit_tool(tool: &ChatToolEntry) -> bool {
    !tool.is_error && matches!(tool.name.as_str(), "Edit" | "MultiEdit" | "Write")
}

#[cfg(test)]
mod edit_card_tests {
    use super::is_edit_tool;
    use crate::chat_pane::ChatToolEntry;

    fn edit_tool(is_error: bool) -> ChatToolEntry {
        ChatToolEntry {
            id: "edit".to_owned(),
            name: "Edit".to_owned(),
            summary: "arquivo.rs".to_owned(),
            command: None,
            done: true,
            is_error,
            result: String::new(),
            expanded: false,
            input: serde_json::Map::new(),
        }
    }

    #[test]
    fn uses_portuguese_singular_and_plural_file_labels() {
        assert_eq!(crate::labels::chat_edited_files(1), "Editou 1 arquivo");
        assert_eq!(crate::labels::chat_edited_files(2), "Editou 2 arquivos");
    }

    #[test]
    fn excludes_failed_edit_tools_from_grouped_cards() {
        assert!(is_edit_tool(&edit_tool(false)));
        assert!(!is_edit_tool(&edit_tool(true)));
    }
}

#[cfg(test)]
mod message_action_tests {
    #[test]
    fn relative_message_time_matches_chat_telemetry_labels() {
        assert_eq!(crate::labels::chat_relative_time(60_000, 60_000), "agora");
        assert_eq!(
            crate::labels::chat_relative_time(60_000, 361_000),
            "há 5 min"
        );
        assert_eq!(crate::labels::chat_relative_time(0, 7_200_000), "há 2 h");
        assert_eq!(crate::labels::chat_relative_time(0, 172_800_000), "há 2 d");
    }

    #[test]
    fn session_states_use_flutter_portuguese_labels() {
        use gsh_chat::SessionState;

        assert_eq!(
            crate::labels::chat_session_state(SessionState::Completed),
            "Concluída"
        );
        assert_eq!(
            crate::labels::chat_session_state(SessionState::ReviewReady),
            "Pronta para revisão"
        );
        assert_eq!(
            crate::labels::chat_session_state(SessionState::Blocked),
            "Bloqueada"
        );
        assert_eq!(
            crate::labels::chat_session_state(SessionState::Failed),
            "Falhou"
        );
    }
}

#[cfg(test)]
#[path = "chat_view_snapshots.rs"]
mod snapshots;

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
    let heading = row![
        text(if all_done {
            labels::ICON_DIFFERENCE
        } else {
            labels::ICON_TOOL_RUNNING
        })
        .font(theme::FONT_ICON)
        .size(theme::SIZE_ICON)
        .color(theme::palette().text3),
        text(labels::chat_edited_files(groups.len())).size(theme::SIZE_LABEL),
        Space::new().width(Fill),
        text(format!("+{}", totals.0))
            .font(theme::FONT_MONO)
            .size(theme::SIZE_TAB)
            .color(theme::palette().online),
        text(format!("−{}", totals.1))
            .font(theme::FONT_MONO)
            .size(theme::SIZE_TAB)
            .color(theme::palette().error),
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let mut rows = column![button(heading)
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
                    row![
                        text(if opened { "⌄" } else { "›" }).size(theme::SIZE_TAB),
                        text(labels::ICON_CODE)
                            .font(theme::FONT_ICON)
                            .size(theme::SIZE_ICON)
                            .color(theme::palette().text3),
                        text(gsh_core::tabs::basename(&path)).size(theme::SIZE_TAB),
                        Space::new().width(Fill),
                        text(format!("+{}", sums.0))
                            .font(theme::FONT_MONO)
                            .size(theme::SIZE_TAB)
                            .color(theme::palette().online),
                        text(format!("−{}", sums.1))
                            .font(theme::FONT_MONO)
                            .size(theme::SIZE_TAB)
                            .color(theme::palette().error),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
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
    let body = match item {
        ChatItem::User {
            text: msg,
            attachments,
            ..
        } => view_user_bubble(idx, msg, attachments, pane),
        ChatItem::Assistant {
            text: msg,
            streaming,
            ..
        } => view_assistant_text(idx, msg, *streaming, pane.markdown(idx), pane),
        ChatItem::Tool(tool) => view_tool_line(idx, tool),
        ChatItem::Error { message } => view_error_notice(message),
        ChatItem::Thinking {
            text: thinking,
            streaming,
        } => view_thinking(idx, thinking, *streaming, pane),
        ChatItem::Permission {
            request_id,
            tool,
            description,
            always_rule,
            allowed,
            always_allowed,
            input,
            question,
            options,
        } => crate::chat_prompt_cards::view_permission_card(
            crate::chat_prompt_cards::PermissionCard {
                request_id,
                tool,
                description,
                always_rule: always_rule.as_deref(),
                allowed: *allowed,
                always_allowed: *always_allowed,
                input,
                question: question.as_deref(),
                options,
            },
        ),
        ChatItem::Questions {
            request_id,
            input,
            questions,
            answered,
        } => crate::chat_prompt_cards::view_question_card(
            request_id,
            input,
            questions,
            answered.as_ref(),
        ),
        ChatItem::Plan {
            request_id,
            plan,
            feedback,
            refining,
            decision,
            ..
        } => view_plan_card(idx, request_id, plan, feedback, *refining, *decision, pane),
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
    };
    let message = match item {
        ChatItem::User { text, at_ms, .. } => Some((text.as_str(), *at_ms, true, false)),
        ChatItem::Assistant {
            text,
            streaming: false,
            at_ms,
        } => Some((text.as_str(), *at_ms, false, true)),
        _ => None,
    };
    if let Some((text, at_ms, user, assistant)) = message {
        view_message_actions(idx, text, at_ms, user, assistant, pane, body)
    } else {
        body
    }
}

fn view_thinking<'a>(
    index: usize,
    thinking: &'a str,
    streaming: bool,
    pane: &'a ChatPane,
) -> Element<'a, ChatMsg> {
    let collapsed = pane.thinking_collapsed(index);
    let heading = row![
        text(labels::ICON_LIGHTBULB)
            .font(theme::FONT_ICON)
            .size(theme::SIZE_ICON)
            .color(theme::palette().text3),
        text(if streaming {
            labels::CHAT_THINKING_LIVE
        } else {
            labels::CHAT_THINKING
        })
        .size(theme::SIZE_LABEL)
        .color(theme::palette().text3),
        text(if collapsed {
            labels::ICON_CHEVRON_RIGHT
        } else {
            labels::ICON_CHEVRON_DOWN
        })
        .font(theme::FONT_ICON)
        .size(theme::SIZE_TAB)
        .color(theme::palette().text3),
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    let mut content = column![button(heading)
        .on_press(ChatMsg::ToggleThinking(index))
        .style(theme::ghost_button)]
    .spacing(4);
    if !collapsed {
        content = content.push(
            row![
                container(Space::new().width(2.0).height(Fill)).style(|_: &Theme| {
                    container::Style {
                        background: Some(Background::Color(theme::palette().border2)),
                        ..Default::default()
                    }
                }),
                Space::new().width(10.0),
                selectable_message_text(index, pane, thinking.trim(), theme::SIZE_LABEL, true,),
            ]
            .width(Fill),
        );
    }
    content.width(Fill).into()
}

fn selectable_message_text<'a>(
    index: usize,
    pane: &'a ChatPane,
    fallback: &'a str,
    size: f32,
    italic: bool,
) -> Element<'a, ChatMsg> {
    let font = if italic {
        iced::Font {
            style: iced::font::Style::Italic,
            ..theme::FONT_BODY
        }
    } else {
        theme::FONT_BODY
    };
    let Some(content) = pane.message_editor(index) else {
        return text(fallback)
            .font(font)
            .size(size)
            .color(theme::palette().text)
            .into();
    };
    iced::widget::text_editor(content)
        .on_action(move |action| ChatMsg::MessageTextAction { index, action })
        .font(font)
        .size(size)
        .line_height(iced::widget::text::LineHeight::Relative(
            theme::LINE_HEIGHT_DISPLAY,
        ))
        .padding(0)
        .height(Length::Shrink)
        .style(|_: &Theme, _status| iced::widget::text_editor::Style {
            background: Background::Color(iced::Color::TRANSPARENT),
            border: Border::default(),
            placeholder: theme::palette().text4,
            value: theme::palette().text,
            selection: theme::palette().accent_soft,
        })
        .into()
}

fn view_message_actions<'a>(
    index: usize,
    message: &str,
    at_ms: u64,
    user: bool,
    assistant: bool,
    pane: &'a ChatPane,
    body: Element<'a, ChatMsg>,
) -> Element<'a, ChatMsg> {
    let always = assistant && index + 1 == pane.items().len() && !pane.is_busy();
    let visible = always || pane.hovered_message() == Some(index);
    let copied = pane.copied_message() == Some(index);
    let copy = button(
        text(if copied {
            labels::ICON_CHECK
        } else {
            labels::ICON_CONTENT_COPY
        })
        .font(theme::FONT_ICON)
        .size(theme::SIZE_TAB)
        .color(theme::palette().text3),
    )
    .padding(3)
    .on_press(ChatMsg::CopyMessage {
        index,
        text: message.to_owned(),
    })
    .style(theme::ghost_button);
    let copy = tooltip(
        copy,
        text(if copied {
            labels::CHAT_COPIED
        } else {
            labels::CHAT_COPY
        }),
        tooltip::Position::Top,
    );
    let actions = if visible {
        row![
            copy,
            text(labels::chat_relative_time(at_ms, current_time_ms()))
                .size(theme::SIZE_TAB)
                .color(theme::palette().text4),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
    } else {
        row![Space::new().height(18.0)]
    };
    let action_line = if user {
        row![Space::new().width(Fill), actions]
    } else {
        row![actions]
    };
    iced::widget::mouse_area(column![body, action_line.height(18.0)].spacing(4))
        .on_enter(ChatMsg::MessageHovered(Some(index)))
        .on_exit(ChatMsg::MessageHovered(None))
        .into()
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn view_user_bubble<'a>(
    idx: usize,
    msg: &'a str,
    attachments: &'a [gsh_chat::Attachment],
    pane: &'a ChatPane,
) -> Element<'a, ChatMsg> {
    let mut content = column![].spacing(8).align_x(Alignment::End);
    for attachment in attachments {
        if let Some(handle) = crate::chat_attachments::image_handle(attachment) {
            let preview = iced::widget::image(handle)
                .content_fit(iced::ContentFit::Contain)
                .border_radius(10.0);
            content = content.push(
                container(preview)
                    .max_width(360.0)
                    .max_height(320.0)
                    .clip(true)
                    .style(|_: &Theme| container::Style {
                        border: Border {
                            color: theme::palette().border2,
                            width: 1.0,
                            radius: 10.0.into(),
                        },
                        ..Default::default()
                    }),
            );
        }
    }

    let files = attachments
        .iter()
        .enumerate()
        .filter(|(_, attachment)| attachment.kind != gsh_chat::AttachmentKind::Image)
        .fold(row![].spacing(6), |chips, (_, attachment)| {
            let icon = if attachment.kind == gsh_chat::AttachmentKind::Pdf {
                labels::ICON_PICTURE_AS_PDF
            } else {
                labels::ICON_DESCRIPTION
            };
            chips.push(
                container(
                    row![
                        text(icon)
                            .font(theme::FONT_ICON)
                            .size(theme::SIZE_SECTION_HEADER)
                            .color(theme::palette().text3),
                        text(attachment.name.as_str())
                            .size(theme::SIZE_TAB)
                            .color(theme::palette().text),
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                )
                .padding([4, 6])
                .style(|_: &Theme| container::Style {
                    background: Some(Background::Color(theme::palette().panel2)),
                    border: Border {
                        color: theme::palette().border,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                }),
            )
        });

    let has_files = attachments
        .iter()
        .any(|attachment| attachment.kind != gsh_chat::AttachmentKind::Image);
    if !msg.is_empty() || has_files {
        let mut bubble = column![].spacing(6).align_x(Alignment::End);
        if !msg.is_empty() {
            bubble = bubble.push(selectable_message_text(
                idx,
                pane,
                msg,
                theme::SIZE_BODY,
                false,
            ));
        }
        if has_files {
            bubble = bubble.push(files);
        }
        content = content.push(
            container(bubble)
                .max_width(USER_BUBBLE_MAX_WIDTH)
                .padding([7, 12])
                .style(|_: &Theme| container::Style {
                    background: Some(Background::Color(theme::palette().panel3)),
                    border: Border {
                        radius: 12.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
        );
    }

    row![Space::new().width(Fill), content].width(Fill).into()
}

fn view_assistant_text<'a>(
    index: usize,
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
            chat_markdown_settings(),
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
    let content = if streaming {
        text(display)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_BODY)
            .color(theme::palette().text)
            .into()
    } else {
        selectable_message_text(index, pane, msg, theme::SIZE_BODY, false)
    };
    container(content).width(Fill).into()
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

fn view_plan_card<'a>(
    index: usize,
    request_id: &str,
    plan: &str,
    feedback: &str,
    refining: bool,
    decision: Option<crate::chat_pane::PlanDecision>,
    pane: &'a ChatPane,
) -> Element<'a, ChatMsg> {
    let mut body = column![if let Some(content) = pane.markdown(index) {
        iced::widget::markdown::view_with(
            content.items(),
            chat_markdown_settings(),
            &ChatMarkdownViewer { pane },
        )
    } else {
        text(plan.to_owned())
            .font(theme::FONT_BODY)
            .size(theme::SIZE_BODY)
            .into()
    }]
    .spacing(10);

    match decision {
        Some(crate::chat_pane::PlanDecision::Approved) => {
            body = body.push(prompt_outcome(
                labels::CHAT_PLAN_APPROVED.to_owned(),
                theme::palette().online,
            ));
        }
        Some(crate::chat_pane::PlanDecision::Refined) => {
            body = body.push(prompt_outcome(
                labels::chat_plan_refined(feedback),
                theme::palette().warn,
            ));
        }
        None if refining => {
            if let Some(editor) = pane.plan_feedback_editor(request_id) {
                let lines = editor.line_count().clamp(2, 6);
                let feedback_input = iced::widget::text_editor(editor)
                    .on_action({
                        let request_id = request_id.to_owned();
                        move |action| ChatMsg::PlanFeedbackAction {
                            request_id: request_id.clone(),
                            action,
                        }
                    })
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_BODY)
                    .line_height(iced::widget::text::LineHeight::Relative(
                        theme::LINE_HEIGHT_DISPLAY,
                    ))
                    .padding([6, 8])
                    .height(Length::Fixed(
                        lines as f32 * theme::SIZE_BODY * theme::LINE_HEIGHT_DISPLAY + 12.0,
                    ))
                    .style(|_: &Theme, _status| iced::widget::text_editor::Style {
                        background: Background::Color(theme::palette().panel2),
                        border: Border {
                            color: theme::palette().border2,
                            width: 1.0,
                            radius: theme::RADIUS_BUTTON.into(),
                        },
                        placeholder: theme::palette().text4,
                        value: theme::palette().text,
                        selection: theme::palette().accent_soft,
                    });
                body = body.push(feedback_input);
            }
            body = body.push(
                row![
                    primary_prompt_button(
                        labels::CHAT_PLAN_SEND_REFINE.to_owned(),
                        ChatMsg::PlanDecision {
                            request_id: request_id.to_owned(),
                            approve: false
                        },
                    ),
                    outline_prompt_button(
                        "Cancelar".to_owned(),
                        ChatMsg::PlanRefining {
                            request_id: request_id.to_owned(),
                            refining: false
                        },
                    ),
                ]
                .spacing(8),
            );
        }
        None => {
            body = body.push(
                row![
                    primary_prompt_button(
                        labels::CHAT_PLAN_APPROVE.to_owned(),
                        ChatMsg::PlanDecision {
                            request_id: request_id.to_owned(),
                            approve: true
                        },
                    ),
                    outline_prompt_button(
                        labels::CHAT_PLAN_REFINE.to_owned(),
                        ChatMsg::PlanRefining {
                            request_id: request_id.to_owned(),
                            refining: true
                        },
                    ),
                ]
                .spacing(8),
            );
        }
    }

    prompt_frame(
        decision.is_none(),
        labels::ICON_CHECKLIST,
        labels::CHAT_PLAN_TITLE.to_owned(),
        body.into(),
    )
}
