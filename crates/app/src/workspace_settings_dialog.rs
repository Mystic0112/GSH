//! Edição de nome, avatar e cor do workspace.

use std::path::PathBuf;

use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Background, Border, Element, Fill, Length, Theme};

use crate::{labels, pane_close_dialog, theme};

pub const PALETTE: [u32; 8] = [
    0xFF6E56CF, 0xFF2F6FF0, 0xFF1AA5A0, 0xFF3FB868, 0xFFE0A33A, 0xFFE5484D, 0xFFD6409F, 0xFF8E8E96,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub id: Option<String>,
    pub name: String,
    pub path: PathBuf,
    pub color: u32,
    pub image: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    NameChanged(String),
    PickPhoto,
    RemovePhoto,
    SelectColor(u32),
    Cancel,
    Save,
}

impl Draft {
    pub fn valid(&self) -> bool {
        !self.name.trim().is_empty()
    }
}

pub fn view<'a>(draft: &'a Draft) -> Element<'a, Message> {
    let avatar: Element<'a, Message> = match &draft.image {
        Some(path) if path.is_file() => {
            iced::widget::image(iced::widget::image::Handle::from_path(path))
                .width(Length::Fixed(40.0))
                .height(Length::Fixed(40.0))
                .into()
        }
        _ => container(
            text(
                draft
                    .name
                    .trim()
                    .chars()
                    .next()
                    .map_or("?".to_owned(), |c| c.to_uppercase().to_string()),
            )
            .size(22)
            .color(theme::palette().text),
        )
        .width(40)
        .height(40)
        .center_x(40)
        .center_y(40)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color(draft.color))),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into(),
    };
    let swatches = PALETTE
        .into_iter()
        .map(|value| {
            button(
                container(Space::new().width(22).height(22)).style(move |_: &Theme| {
                    container::Style {
                        background: Some(Background::Color(color(value))),
                        border: Border {
                            color: if value == draft.color {
                                theme::palette().text
                            } else {
                                theme::palette().panel
                            },
                            width: if value == draft.color { 2.0 } else { 1.0 },
                            radius: 14.0.into(),
                        },
                        ..Default::default()
                    }
                }),
            )
            .on_press(Message::SelectColor(value))
            .padding(3)
            .style(|_: &Theme, _| iced::widget::button::Style {
                background: None,
                border: Border::default(),
                ..Default::default()
            })
        })
        .fold(row![].spacing(4), |row, item| row.push(item));
    let photo = if draft.image.is_some() {
        row![
            pane_close_dialog::action(
                if draft.image.is_some() {
                    labels::WORKSPACE_CHANGE_PHOTO
                } else {
                    labels::WORKSPACE_ADD_PHOTO
                },
                pane_close_dialog::ButtonKind::Outline,
                Some(Message::PickPhoto)
            ),
            pane_close_dialog::action(
                labels::WORKSPACE_REMOVE_PHOTO,
                pane_close_dialog::ButtonKind::Ghost,
                Some(Message::RemovePhoto)
            ),
        ]
        .spacing(6)
    } else {
        row![pane_close_dialog::action(
            labels::WORKSPACE_ADD_PHOTO,
            pane_close_dialog::ButtonKind::Outline,
            Some(Message::PickPhoto)
        )]
    };
    let body = column![
        row![
            avatar,
            text_input(labels::WORKSPACE_NAME, &draft.name).on_input(Message::NameChanged)
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center),
        photo,
        text(labels::WORKSPACE_COLOR)
            .size(13)
            .color(theme::palette().text2),
        swatches,
        text(labels::WORKSPACE_FOLDER)
            .size(13)
            .color(theme::palette().text2),
        text(draft.path.display().to_string())
            .size(12)
            .color(theme::palette().text3),
    ]
    .spacing(10)
    .width(Fill)
    .into();
    let actions = row![
        pane_close_dialog::action(
            labels::WORKSPACE_CANCEL,
            pane_close_dialog::ButtonKind::Outline,
            Some(Message::Cancel)
        ),
        pane_close_dialog::action(
            labels::WORKSPACE_SAVE,
            pane_close_dialog::ButtonKind::Primary,
            draft.valid().then_some(Message::Save)
        ),
    ]
    .spacing(8);
    pane_close_dialog::form(
        text(labels::WORKSPACE_SETTINGS_TITLE)
            .size(theme::DIALOG_TITLE_SIZE)
            .color(theme::palette().text)
            .into(),
        body,
        actions.into(),
        Some(420.0),
        Message::Cancel,
        Message::Cancel,
    )
}

fn color(argb: u32) -> iced::Color {
    iced::Color::from_rgba8(
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
        ((argb >> 24) as u8) as f32 / 255.0,
    )
}
