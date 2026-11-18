//! Painéis auxiliares do composer: sessões recentes e uso da conta.

use iced::widget::{button, column, container, text};
use iced::{Background, Border, Element, Fill, Length, Theme};

use crate::chat_pane::{ChatMsg, ChatPane};
use crate::{labels, theme};

pub(super) fn view_peer_picker(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut list = column![text(labels::CHAT_PEER_PICKER).size(theme::SIZE_LABEL)];
    let sessions = pane.peer_sessions();
    if sessions.is_empty() {
        list = list.push(text(labels::CHAT_NO_PEER_SESSIONS).size(theme::SIZE_TAB));
    }
    for session in sessions {
        list = list.push(
            button(
                text(format!(
                    "{} · {} · {}",
                    session.title,
                    gsh_core::tabs::basename(&session.project),
                    time_ago(session.last_seen_ms)
                ))
                .size(theme::SIZE_TAB),
            )
            .on_press(ChatMsg::PickPeerSession(session.id))
            .style(theme::ghost_button),
        );
    }
    list = list.push(
        button(text(labels::CHAT_LIST_ALL_SESSIONS).size(theme::SIZE_TAB))
            .on_press(ChatMsg::PickPeerSession(String::new()))
            .style(theme::ghost_button),
    );
    container(list.spacing(3)).padding(8).width(Fill).into()
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
    } else {
        format!("há {} h", seconds / 3600)
    }
}

pub(super) fn view_usage_details(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let (tokens, window) = pane.context();
    let context = if window > 0 {
        format!(
            "{} · {}%",
            labels::format_chat_context(tokens, window),
            (tokens.saturating_mul(100) / window).min(100)
        )
    } else {
        labels::CHAT_CONTEXT_UNAVAILABLE.to_string()
    };
    let mut rows = column![
        text(
            pane.usage()
                .and_then(|u| u.plan.as_deref())
                .unwrap_or(labels::CHAT_USAGE)
        )
        .font(theme::FONT_BODY_MEDIUM)
        .size(theme::SIZE_LABEL),
        text(context)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_TAB)
            .color(theme::palette().text2)
    ];
    if pane.usage().is_none() {
        rows = rows.push(
            text(labels::CHAT_USAGE_UNAVAILABLE)
                .size(theme::SIZE_TAB)
                .color(theme::palette().text3),
        );
    }
    for limit in pane.usage().into_iter().flat_map(|u| &u.limits) {
        let label = match limit.kind {
            gsh_chat::LimitKind::Session => labels::CHAT_USAGE_SESSION,
            gsh_chat::LimitKind::Weekly => labels::CHAT_USAGE_WEEKLY,
            gsh_chat::LimitKind::WeeklyModel => {
                limit.model.as_deref().unwrap_or(labels::CHAT_USAGE_WEEKLY)
            }
            gsh_chat::LimitKind::Other => continue,
        };
        let reset = limit
            .resets_at_ms
            .map(|ms| format!(" · {}", reset_relative(ms)))
            .unwrap_or_default();
        rows = rows.push(
            text(format!("{label}: {}%{}", limit.percent, reset))
                .font(theme::FONT_BODY)
                .size(theme::SIZE_TAB)
                .color(if limit.warning || limit.percent >= 80 {
                    theme::palette().warn
                } else {
                    theme::palette().text2
                }),
        );
    }
    container(rows.spacing(4))
        .padding(12)
        .width(Length::Fixed(300.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                color: theme::palette().border2,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn reset_relative(at_ms: u64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let at = crate::clock::local_time(at_ms);
    let now = crate::clock::local_time(now_ms);
    let time = format!("{:02}:{:02}", at.hour, at.minute);
    if at.year == now.year && at.month == now.month && at.day == now.day {
        format!("{}{time}", labels::CHAT_USAGE_RESET_TODAY)
    } else {
        format!(
            "{}{:02}/{:02} às {time}",
            labels::CHAT_USAGE_RESET_ON,
            at.day,
            at.month
        )
    }
}
