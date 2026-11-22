use std::path::{Path, PathBuf};

use iced::widget::{column, container, row, scrollable, text, Space};
use iced::{Background, Color, Element, Fill, Length, Theme};

use crate::file_viewer::{FileViewerMsg, FileViewerState, TextDocument};
use crate::{labels, theme};

struct MarkdownViewer;

impl<'a> iced::widget::markdown::Viewer<'a, FileViewerMsg> for MarkdownViewer {
    fn on_link_click(_: String) -> FileViewerMsg {
        FileViewerMsg::Ignore
    }
}

pub fn view(state: &FileViewerState) -> Element<'_, FileViewerMsg> {
    match state {
        FileViewerState::Loaded(doc) => view_document(doc),
        FileViewerState::Markdown(_, content) => iced::widget::markdown::view_with(
            content.items(),
            iced::widget::markdown::Settings::with_text_size(
                theme::SIZE_BODY,
                iced::widget::markdown::Style::from(Theme::Dark),
            ),
            &MarkdownViewer,
        ),
        FileViewerState::Svg(source) => iced::widget::svg(iced::widget::svg::Handle::from_memory(
            source.as_bytes().to_vec(),
        ))
        .width(Fill)
        .height(Fill)
        .into(),
        FileViewerState::Image(bytes) => {
            iced::widget::image(iced::widget::image::Handle::from_bytes(bytes.clone()))
                .width(Fill)
                .height(Fill)
                .into()
        }
        FileViewerState::Media(path) => view_media(path),
        FileViewerState::LargePreview {
            size_bytes,
            max_bytes,
            document,
        } => column![
            text(labels::format_viewer_large_preview(*size_bytes, *max_bytes))
                .color(theme::palette().warn),
            view_document(document),
        ]
        .into(),
        FileViewerState::TooLarge {
            size_bytes,
            max_bytes,
        } => view_status_notice(
            labels::format_viewer_too_large(*size_bytes, *max_bytes),
            theme::palette().warn,
        ),
        FileViewerState::Binary(kind) => {
            view_status_notice(kind.ui_message().to_string(), theme::palette().text2)
        }
        FileViewerState::Error(err) => view_status_notice(err.ui_message(), theme::palette().error),
    }
}

fn view_media(path: &Path) -> Element<'_, FileViewerMsg> {
    let path = path.to_path_buf();
    container(column![text(path.to_string_lossy().to_string()), media_action(path)].spacing(12))
        .padding(24)
        .width(Fill)
        .height(Fill)
        .into()
}

#[cfg(target_os = "linux")]
fn media_action(path: PathBuf) -> Element<'static, FileViewerMsg> {
    iced::widget::button(text("Abrir no reprodutor padrão do sistema"))
        .on_press(FileViewerMsg::OpenMedia(path))
        .into()
}

#[cfg(not(target_os = "linux"))]
fn media_action(_: PathBuf) -> Element<'static, FileViewerMsg> {
    text("A reprodução pelo sistema está disponível no GSH para Linux.").into()
}

fn view_document<'a, M: 'a>(doc: &'a TextDocument) -> Element<'a, M> {
    let gutter = container(
        text(doc.gutter_text())
            .font(theme::FONT_MONO)
            .size(theme::SIZE_MONO)
            .line_height(theme::LINE_HEIGHT_MONO)
            .color(theme::palette().text2)
            .wrapping(text::Wrapping::None),
    )
    .padding([0, 14]);
    let separator = container(Space::new().width(1.0).height(Fill))
        .width(Length::Fixed(1.0))
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        });
    let code = container(
        text(doc.body_text())
            .font(theme::FONT_MONO)
            .size(theme::SIZE_MONO)
            .line_height(theme::LINE_HEIGHT_MONO)
            .color(theme::palette().text)
            .wrapping(text::Wrapping::None),
    )
    .padding([0, 14]);
    let scrolled = scrollable(row![gutter, separator, code].padding([12, 0]))
        .direction(scrollable::Direction::Both {
            vertical: scrollable::Scrollbar::default(),
            horizontal: scrollable::Scrollbar::default(),
        })
        .width(Fill)
        .height(Fill);
    container(scrolled)
        .width(Fill)
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel)),
            ..Default::default()
        })
        .into()
}

fn view_status_notice<'a, M: 'a>(message: String, color: Color) -> Element<'a, M> {
    container(
        text(message)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_BODY)
            .color(color),
    )
    .padding([20, 24])
    .width(Fill)
    .height(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        ..Default::default()
    })
    .into()
}
