//! Apresentação compartilhada de anexos no composer e na transcrição do chat.

use gsh_chat::{Attachment, AttachmentKind};
use iced::widget::{button, container, row, text};
use iced::{Alignment, Background, Border, Element, Length, Theme};

use crate::chat_pane::ChatMsg;
use crate::{labels, theme};

pub fn image_handle(attachment: &Attachment) -> Option<iced::widget::image::Handle> {
    if attachment.kind != AttachmentKind::Image {
        return None;
    }

    attachment
        .bytes
        .as_ref()
        .map(|bytes| iced::widget::image::Handle::from_bytes(bytes.clone()))
        .or_else(|| {
            attachment
                .path
                .as_ref()
                .map(iced::widget::image::Handle::from_path)
        })
}

pub fn view_pending_attachment(
    attachment: &gsh_chat::Attachment,
    index: usize,
) -> Element<'_, ChatMsg> {
    if let Some(handle) = image_handle(attachment) {
        let preview = iced::widget::image(handle)
            .width(Length::Fixed(64.0))
            .height(Length::Fixed(64.0))
            .content_fit(iced::ContentFit::Cover)
            .border_radius(8.0);
        return row![
            container(preview).style(|_: &Theme| container::Style {
                border: Border {
                    color: theme::palette().border2,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }),
            button(text(labels::ICON_CLOSE).font(theme::FONT_ICON).size(12))
                .padding(3)
                .on_press(ChatMsg::RemoveAttachment(index))
                .style(theme::ghost_button)
        ]
        .spacing(3)
        .align_y(Alignment::Start)
        .into();
    }

    let icon = if attachment.kind == gsh_chat::AttachmentKind::Pdf {
        labels::ICON_PICTURE_AS_PDF
    } else {
        labels::ICON_DESCRIPTION
    };
    container(
        row![
            text(icon)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_SECTION_HEADER)
                .color(theme::palette().text3),
            text(&attachment.name)
                .size(theme::SIZE_TAB)
                .color(theme::palette().text)
                .width(Length::Fixed(200.0)),
            button(text(labels::ICON_CLOSE).font(theme::FONT_ICON).size(12))
                .padding(2)
                .on_press(ChatMsg::RemoveAttachment(index))
                .style(theme::ghost_button),
        ]
        .spacing(6)
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
    })
    .into()
}
