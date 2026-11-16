//! Superfícies e ações compartilhadas pelos painéis laterais.
use crate::theme;
use iced::widget::{button, container, row, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

pub fn ghost<'a, M: Clone + 'a>(
    glyph: &'static str,
    message: Option<M>,
    active: bool,
) -> Element<'a, M> {
    button(
        container(text(glyph).font(theme::FONT_ICON).size(theme::SIZE_ICON))
            .center_x(Fill)
            .center_y(Fill),
    )
    .width(28)
    .height(28)
    .padding(0)
    .on_press_maybe(message)
    .style(move |_: &Theme, status| button::Style {
        background: if active {
            Some(Background::Color(theme::palette().accent_soft))
        } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            Some(Background::Color(theme::palette().panel3))
        } else {
            None
        },
        text_color: if matches!(status, button::Status::Disabled) && !active {
            theme::palette().text4
        } else {
            theme::palette().text2
        },
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}
pub fn hairline_h<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(Fill).height(1))
        .width(Fill)
        .height(1)
        .style(|_| container::Style {
            background: Some(theme::palette().border.into()),
            ..Default::default()
        })
        .into()
}
pub fn frame<'a, M: 'a>(content: Element<'a, M>) -> Element<'a, M> {
    let line = container(Space::new().width(1).height(Fill))
        .width(1)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(theme::palette().border.into()),
            ..Default::default()
        });
    row![
        line,
        container(content)
            .width(Length::Fixed(theme::FILES_PANEL_WIDTH - 1.0))
            .height(Fill)
            .style(|_| container::Style {
                background: Some(theme::palette().bg.into()),
                ..Default::default()
            })
    ]
    .width(theme::FILES_PANEL_WIDTH)
    .height(Fill)
    .into()
}

/// Cabeçalho de seção do painel direito (`_PanelHeader`): título em caixa alta
/// `text3` e as ações à direita.
pub fn section_header<'a, M: 'a>(
    title: &'static str,
    actions: Vec<Element<'a, M>>,
) -> Element<'a, M> {
    container(
        row![
            text(title)
                .font(theme::FONT_BODY)
                .size(theme::FILE_PANEL_HEADER_TITLE_SIZE)
                .color(theme::palette().text3),
            Space::new().width(Fill),
        ]
        .extend(actions)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding(Padding {
        top: theme::FILE_PANEL_HEADER_TOP,
        right: theme::FILE_PANEL_HEADER_RIGHT,
        bottom: theme::FILE_PANEL_HEADER_BOTTOM,
        left: theme::FILE_PANEL_HEADER_LEFT,
    })
    .into()
}

/// Mensagem centralizada de corpo vazio (`noFolderMessage`).
pub fn empty_label<'a, M: 'a>(message: &'static str) -> Element<'a, M> {
    container(
        text(message)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3),
    )
    .center_x(Fill)
    .center_y(Fill)
    .into()
}
