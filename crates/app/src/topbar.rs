//! Barra superior do shell (`gsh_topbar.dart` / `WindowTitleBar` do Flutter).
//!
//! Altura 46px, superfície `BG`, linha divisória inferior de 1px `BORDER`,
//! botão de recolher o rail à esquerda + título do workspace ativo, pílula da
//! `AgentIsland` centralizada e botão do painel de arquivos à direita.

use iced::widget::{button, column, container, row, stack, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

use crate::labels;
use crate::theme;

/// Dados de entrada para renderizar a topbar sem acoplamento ao estado global.
#[derive(Debug, Clone, Copy)]
pub struct TopbarProps<'a> {
    pub workspace_title: &'a str,
    pub rail_open: bool,
    pub files_open: bool,
}

/// Renderiza a barra superior do shell com a pílula da ilha centralizada.
pub fn view<'a, M: Clone + 'a>(
    props: TopbarProps<'a>,
    island_pill: Element<'a, M>,
    on_toggle_rail: M,
    on_toggle_files: M,
) -> Element<'a, M> {
    let left = row![
        Space::new().width(12.0),
        icon_toggle_button(labels::ICON_SIDEBAR_LEFT, props.rail_open, on_toggle_rail),
        Space::new().width(8.0),
        text(props.workspace_title)
            .font(theme::FONT_TITLE)
            .size(theme::SIZE_TOPBAR_TITLE)
            .color(theme::palette().text),
    ]
    .align_y(Alignment::Center);

    let right = row![
        icon_toggle_button(
            labels::ICON_SIDEBAR_RIGHT,
            props.files_open,
            on_toggle_files
        ),
        Space::new().width(12.0),
    ]
    .align_y(Alignment::Center);

    let chrome_row = container(
        row![left, Space::new().width(Fill), right]
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::TOPBAR_HEIGHT - 1.0));

    let center_island = container(island_pill)
        .width(Fill)
        .height(Length::Fixed(theme::TOPBAR_HEIGHT - 1.0))
        .center_x(Fill)
        .center_y(Length::Fixed(theme::TOPBAR_HEIGHT - 1.0));

    let bar = stack![chrome_row, center_island];

    let bottom_hairline = container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        });

    container(column![bar, bottom_hairline])
        .width(Fill)
        .height(Length::Fixed(theme::TOPBAR_HEIGHT))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().bg)),
            ..Default::default()
        })
        .into()
}

fn icon_toggle_button<'a, M: Clone + 'a>(
    glyph: &'static str,
    selected: bool,
    on_press: M,
) -> Element<'a, M> {
    button(
        container(
            text(glyph)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_TITLE)
                .color(if selected {
                    theme::palette().accent_text
                } else {
                    theme::palette().text3
                }),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(28.0))
    .height(Length::Fixed(28.0))
    .padding(0)
    .on_press(on_press)
    .style(move |_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let bg = if selected {
            Some(Background::Color(theme::palette().accent_soft))
        } else if hovered {
            Some(Background::Color(theme::palette().panel3))
        } else {
            None
        };
        button::Style {
            background: bg,
            text_color: if selected {
                theme::palette().accent_text
            } else if hovered {
                theme::palette().text2
            } else {
                theme::palette().text3
            },
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}
