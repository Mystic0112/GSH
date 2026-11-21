//! Diálogo de aceite e acompanhamento de transferências LocalSend.

use crate::pane_close_dialog::{self, ButtonKind};
use crate::settings::{LocalSendIncoming, LocalSendMessage, LocalSendState, Message, Settings};
use crate::{labels, theme};
use iced::widget::{column, progress_bar, row, text};
use iced::{Alignment, Element};

pub fn view(state: &LocalSendState) -> Option<Element<'static, Message>> {
    if let Some(incoming) = &state.incoming {
        return Some(receive_dialog(incoming, &state.receive_dir));
    }
    if let Some(transfer) = &state.transfer {
        let title = text(if transfer.sending {
            labels::LOCALSEND_SEND_FILE
        } else {
            labels::LOCALSEND_PROGRESS
        })
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text)
        .into();
        let body = column![
            text(format!("{} · {}", transfer.peer_alias, transfer.file_name))
                .size(theme::DIALOG_MESSAGE_SIZE)
                .color(theme::palette().text2),
            progress_bar(
                0.0..=transfer.total_bytes.max(1) as f32,
                transfer.bytes.min(transfer.total_bytes.max(1)) as f32,
            ),
        ]
        .spacing(10)
        .into();
        let (actions, cancel) = if transfer.sending {
            (
                row![pane_close_dialog::action(
                    labels::LOCALSEND_CANCEL,
                    ButtonKind::Outline,
                    Some(Message::LocalSend(LocalSendMessage::CancelSend)),
                )]
                .align_y(Alignment::Center)
                .into(),
                Message::LocalSend(LocalSendMessage::CancelSend),
            )
        } else {
            (
                row![iced::widget::Space::new().width(1)].into(),
                Message::LocalSend(LocalSendMessage::Noop),
            )
        };
        return Some(pane_close_dialog::form(
            title,
            body,
            actions,
            Some(theme::DIALOG_CONTENT_WIDTH),
            cancel,
            Message::LocalSend(LocalSendMessage::Noop),
        ));
    }
    state
        .received_dir
        .as_ref()
        .map(|path| received_dialog(path))
}

fn receive_dialog(
    incoming: &LocalSendIncoming,
    receive_dir: &std::path::Path,
) -> Element<'static, Message> {
    let title = text(labels::LOCALSEND_RECEIVE_TITLE)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text)
        .into();
    let model = incoming
        .sender_model
        .as_deref()
        .map(|value| format!(" · {value}"))
        .unwrap_or_default();
    let mut files = column![].spacing(4);
    for file in &incoming.files {
        files = files.push(
            row![
                text(file.name.clone())
                    .size(theme::DIALOG_MESSAGE_SIZE)
                    .color(theme::palette().text),
                text(format_bytes(file.size))
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text3),
            ]
            .spacing(8),
        );
    }
    let body = column![
        text(format!(
            "{}{}{} · {}",
            incoming.sender_alias,
            model,
            labels::LOCALSEND_RECEIVE_FROM,
            incoming.sender_ip
        ))
        .size(theme::DIALOG_MESSAGE_SIZE)
        .color(theme::palette().text2),
        files,
        text(format!(
            "{} · {}",
            labels::LOCALSEND_RECEIVE_TO,
            receive_dir.display()
        ))
        .size(theme::SIZE_LABEL)
        .color(theme::palette().text3),
        text(format!(
            "{} · {}",
            labels::LOCALSEND_TOTAL,
            format_bytes(incoming.total_bytes)
        ))
        .size(theme::SIZE_LABEL)
        .color(theme::palette().text3),
    ]
    .spacing(theme::DIALOG_TITLE_GAP)
    .into();
    let actions = row![
        pane_close_dialog::action(
            labels::LOCALSEND_REJECT,
            ButtonKind::Outline,
            Some(Message::LocalSend(LocalSendMessage::RejectReceive)),
        ),
        pane_close_dialog::action(
            labels::LOCALSEND_ACCEPT,
            ButtonKind::Primary,
            Some(Message::LocalSend(LocalSendMessage::AcceptReceive)),
        ),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center)
    .into();
    pane_close_dialog::form(
        title,
        body,
        actions,
        Some(theme::DIALOG_CONTENT_WIDTH),
        Message::LocalSend(LocalSendMessage::RejectReceive),
        Message::LocalSend(LocalSendMessage::Noop),
    )
}

fn received_dialog(receive_dir: &std::path::Path) -> Element<'static, Message> {
    let title = text(labels::LOCALSEND_RECEIVED)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text)
        .into();
    let body = text(format!(
        "{}\n{}",
        labels::LOCALSEND_RECEIVE_TO,
        receive_dir.display()
    ))
    .size(theme::DIALOG_MESSAGE_SIZE)
    .color(theme::palette().text2)
    .into();
    let actions = row![
        pane_close_dialog::action(
            labels::LOCALSEND_SHOW,
            ButtonKind::Outline,
            Some(Message::LocalSend(LocalSendMessage::ShowReceived)),
        ),
        pane_close_dialog::action(
            labels::LOCALSEND_OPEN_FOLDER,
            ButtonKind::Primary,
            Some(Message::LocalSend(LocalSendMessage::OpenReceivedFolder)),
        ),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center)
    .into();
    pane_close_dialog::form(
        title,
        body,
        actions,
        Some(theme::DIALOG_CONTENT_WIDTH),
        Message::LocalSend(LocalSendMessage::Noop),
        Message::LocalSend(LocalSendMessage::Noop),
    )
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1_000_000 {
        format!("{:.0} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{:.1} MB", bytes as f64 / 1_000_000.0)
    }
}

pub fn show_settings_dialog(settings: &Settings) -> Option<Element<'static, Message>> {
    view(&settings.localsend)
}
