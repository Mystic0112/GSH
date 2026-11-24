use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Background, Color, Element, Fill, Length, Theme};

use crate::file_viewer::{FileViewerMsg, FileViewerState, TextDocument};
use crate::{labels, theme};

struct MarkdownViewer;

impl<'a> iced::widget::markdown::Viewer<'a, FileViewerMsg> for MarkdownViewer {
    fn on_link_click(uri: iced::widget::markdown::Uri) -> FileViewerMsg {
        FileViewerMsg::OpenLink(uri.to_string())
    }
}

pub fn view<'a>(state: &'a FileViewerState, root: Option<&'a Path>) -> Element<'a, FileViewerMsg> {
    let (path, body, can_toggle) = match state {
        FileViewerState::Loaded(doc) => (doc.path(), view_document(doc), false),
        FileViewerState::Markdown(document, content) => (
            document.path(),
            markdown_body(document.source(), content),
            true,
        ),
        FileViewerState::Svg(path, source) => (path.as_path(), svg_body(source), true),
        FileViewerState::Image(path, bytes) => (
            path.as_path(),
            iced::widget::image(iced::widget::image::Handle::from_bytes(bytes.clone()))
                .width(Fill)
                .height(Fill)
                .into(),
            false,
        ),
        FileViewerState::Media(path) => (path.as_path(), view_media(path), false),
        FileViewerState::LargePreview {
            size_bytes,
            max_bytes,
            document,
        } => (
            document.path(),
            column![
                text(labels::format_viewer_large_preview(*size_bytes, *max_bytes))
                    .color(theme::palette().warn),
                view_document(document),
            ]
            .into(),
            false,
        ),
        FileViewerState::TooLarge {
            path,
            size_bytes,
            max_bytes,
        } => (
            path.as_path(),
            view_status_notice(
                labels::format_viewer_too_large(*size_bytes, *max_bytes),
                theme::palette().warn,
            ),
            false,
        ),
        FileViewerState::Binary(path, kind) => (
            path.as_path(),
            view_status_notice(kind.ui_message().to_string(), theme::palette().text2),
            false,
        ),
        FileViewerState::Error(path, err) => (
            path.as_path(),
            view_status_notice(err.ui_message(), theme::palette().error),
            false,
        ),
    };
    let path = path.to_path_buf();
    column![
        container(body).height(Fill).width(Fill),
        footer(
            &path,
            root,
            false,
            can_toggle,
            false,
            can_toggle.then_some(FileViewerMsg::ToggleSource(path.clone())),
            FileViewerMsg::OpenPath,
        ),
    ]
    .height(Fill)
    .width(Fill)
    .into()
}

pub fn source_preview<'a>(
    path: &Path,
    root: Option<&Path>,
    source: &str,
    markdown: Option<&'a iced::widget::markdown::Content>,
    dirty: bool,
) -> Element<'a, FileViewerMsg> {
    let body = if path
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
        svg_body(source)
    } else {
        markdown.map_or_else(|| text("").into(), |content| markdown_body(source, content))
    };
    let path = path.to_path_buf();
    column![
        container(body).height(Fill).width(Fill),
        footer(
            &path,
            root,
            dirty,
            true,
            false,
            Some(FileViewerMsg::ToggleSource(path.clone())),
            FileViewerMsg::OpenPath,
        ),
    ]
    .height(Fill)
    .width(Fill)
    .into()
}

fn markdown_body<'a>(
    source: &str,
    content: &'a iced::widget::markdown::Content,
) -> Element<'a, FileViewerMsg> {
    let frontmatter = crate::file_viewer_assets::split_frontmatter(source);
    let palette = theme::palette();
    let app_theme = match theme::active() {
        theme::ThemeVariant::Dark => Theme::Dark,
        theme::ThemeVariant::Light => Theme::Light,
    };
    let mut style = iced::widget::markdown::Style::from(app_theme);
    style.font = theme::FONT_BODY;
    style.inline_code_color = palette.text;
    style.inline_code_highlight = iced::widget::markdown::Highlight {
        background: Background::Color(palette.panel3),
        border: iced::Border {
            radius: 4.0.into(),
            ..Default::default()
        },
    };
    style.link_color = palette.accent_text;
    let mut settings = iced::widget::markdown::Settings::with_text_size(theme::SIZE_BODY, style);
    settings.h1_size = 22.0.into();
    settings.h2_size = 18.0.into();
    settings.h3_size = 16.0.into();
    settings.h4_size = 14.5.into();
    settings.h5_size = 13.5.into();
    settings.h6_size = 12.5.into();
    settings.style.code_block_font = theme::FONT_MONO;
    let mut column = column![];
    if let Some(frontmatter) = frontmatter {
        column = column.push(frontmatter_table(frontmatter.fields));
    }
    column = column.push(iced::widget::markdown::view_with(
        content.items(),
        settings,
        &MarkdownViewer,
    ));
    scrollable(
        container(column)
            .padding(iced::Padding {
                top: 20.0,
                right: 16.0,
                bottom: 20.0,
                left: 24.0,
            })
            .width(Fill),
    )
    .width(Fill)
    .height(Fill)
    .into()
}

fn svg_body(source: &str) -> Element<'static, FileViewerMsg> {
    iced::widget::svg(iced::widget::svg::Handle::from_memory(
        source.as_bytes().to_vec(),
    ))
    .width(Fill)
    .height(Fill)
    .into()
}

pub fn footer<'a, Message: Clone + 'static>(
    path: &Path,
    root: Option<&Path>,
    dirty: bool,
    has_toggle: bool,
    source_active: bool,
    toggle: Option<Message>,
    open_path: impl Fn(PathBuf) -> Message + Copy + 'a,
) -> Element<'a, Message> {
    let display = root
        .and_then(|root| path.strip_prefix(root).ok())
        .unwrap_or(path);
    let mut actual = if root.is_some_and(|root| path.starts_with(root)) {
        root.unwrap_or(path).to_path_buf()
    } else if path.is_absolute() {
        Path::new(std::path::MAIN_SEPARATOR_STR).to_path_buf()
    } else {
        PathBuf::new()
    };
    let components = display.components().collect::<Vec<_>>();
    let mut crumbs = row![].spacing(2);
    for (index, component) in components.iter().enumerate() {
        let name = component.as_os_str().to_string_lossy().into_owned();
        if name.is_empty() || name == "/" {
            continue;
        }
        actual.push(component.as_os_str());
        if index > 0 {
            crumbs = crumbs.push(text("›").size(12).color(theme::palette().text4));
        }
        if index + 1 == components.len() {
            crumbs = crumbs
                .push(crate::file_icons::view_sized::<Message>(
                    &name, false, false, 13.0,
                ))
                .push(Space::new().width(5))
                .push(text(name).size(12).color(theme::palette().text2));
        } else {
            crumbs = crumbs.push(
                button(text(name).size(12).color(theme::palette().text4))
                    .padding(0)
                    .on_press(open_path(actual.clone())),
            );
        }
    }
    let mut actions = row![].spacing(8).align_y(iced::Alignment::Center);
    if dirty {
        actions = actions.push(
            container(Space::new().width(7).height(7))
                .width(7)
                .height(7)
                .style(|_: &Theme| container::Style {
                    background: Some(theme::palette().accent.into()),
                    border: iced::Border::default().rounded(4),
                    ..Default::default()
                }),
        );
    }
    if has_toggle {
        let palette = theme::palette();
        let segment = |label: &'static str, active: bool| {
            button(
                text(label)
                    .size(12)
                    .color(if active { palette.text } else { palette.text3 }),
            )
            .padding([4, 10])
            .on_press_maybe(toggle.clone())
            .style(move |_, status| button::Style {
                background: (active || status == button::Status::Hovered)
                    .then_some(palette.panel.into()),
                border: iced::Border {
                    radius: 5.0.into(),
                    ..Default::default()
                },
                text_color: if active { palette.text } else { palette.text3 },
                ..Default::default()
            })
        };
        actions = actions.push(
            container(row![
                segment(labels::FILE_VIEWER_PREVIEW, !source_active),
                segment(labels::FILE_VIEWER_SOURCE, source_active),
            ])
            .padding(2)
            .style(move |_: &Theme| container::Style {
                background: Some(palette.panel2.into()),
                border: iced::Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
    }
    container(
        row![
            scrollable(crumbs)
                .direction(scrollable::Direction::Horizontal(
                    scrollable::Scrollbar::default(),
                ))
                .width(Fill),
            actions
        ]
        .align_y(iced::Alignment::Center)
        .spacing(8),
    )
    .padding([0, 8])
    .height(34)
    .width(Fill)
    .style(|_: &Theme| container::Style {
        background: Some(theme::palette().bg.into()),
        border: iced::Border {
            color: theme::palette().border,
            width: 1.0,
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn frontmatter_table(fields: Vec<(String, String)>) -> Element<'static, FileViewerMsg> {
    let palette = theme::palette();
    let mut rows = column![].spacing(0);
    for (key, value) in fields {
        let key_cell = container(
            text(key)
                .font(theme::FONT_BODY_BOLD)
                .size(theme::SIZE_BODY)
                .color(palette.text),
        )
        .padding([6, 10])
        .width(Length::Shrink)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(palette.panel3)),
            ..Default::default()
        });
        let value_cell = container(
            text(value)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_BODY)
                .color(palette.text),
        )
        .padding([6, 10])
        .width(Fill);
        rows = rows.push(
            container(row![key_cell, value_cell].spacing(1).width(Fill))
                .padding(1)
                .width(Fill)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(palette.border2)),
                    ..Default::default()
                }),
        );
    }
    container(rows)
        .padding(iced::Padding {
            top: 0.0,
            right: 0.0,
            bottom: 12.0,
            left: 0.0,
        })
        .width(Fill)
        .into()
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
