//! Barra superior do shell (`gsh_topbar.dart` / `WindowTitleBar` do Flutter).
//!
//! Altura 46px, superfície `BG`, linha divisória inferior de 1px `BORDER`,
//! botão de recolher o rail à esquerda + título do workspace ativo, pílula da
//! `AgentIsland` centralizada e botão do painel de arquivos à direita.

use iced::widget::{button, column, container, responsive, row, text, tooltip, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

use crate::labels;
use crate::theme;

/// Dados de entrada para renderizar a topbar sem acoplamento ao estado global.
#[derive(Debug, Clone, Copy)]
pub struct TopbarProps<'a> {
    pub workspace_title: &'a str,
    pub rail_open: bool,
    pub files_open: bool,
    pub counts: crate::farm::FarmCounts,
}

/// Ações dos botões da topbar (mensagens do shell).
pub struct TopbarActions<M> {
    pub on_toggle_rail: M,
    pub on_toggle_files: M,
    pub on_chat: M,
    pub on_farm: M,
    /// Abre a aba Harness do workspace; `None` esconde o botão (sem pasta local ou recurso desligado).
    pub on_harness: Option<M>,
    pub on_agent_graph: M,
}

/// Renderiza a barra superior do shell com a pílula da ilha centralizada.
pub fn view<'a, M: Clone + 'a>(
    props: TopbarProps<'a>,
    island_pill: impl Fn() -> Element<'a, M> + 'a,
    actions: TopbarActions<M>,
) -> Element<'a, M> {
    let left = row![
        Space::new().width(12.0),
        icon_toggle_button(
            labels::ICON_SIDEBAR_LEFT,
            !props.rail_open,
            actions.on_toggle_rail
        ),
        Space::new().width(8.0),
        text(props.workspace_title)
            .font(theme::FONT_TITLE)
            .size(theme::SIZE_TOPBAR_TITLE)
            .color(theme::palette().text),
    ]
    .align_y(Alignment::Center);

    let mut right = row![
        farm_strip(props.counts, actions.on_farm.clone()),
        icon_toggle_button(labels::ICON_CHAT, false, actions.on_chat),
        icon_toggle_button(labels::ICON_FARM, false, actions.on_farm),
    ];
    if let Some(on_harness) = actions.on_harness {
        right = right.push(tooltip(
            icon_toggle_button(labels::ICON_TUNE, false, on_harness),
            crate::pane_chrome::tip_bubble(labels::TOPBAR_OPEN_HARNESS),
            tooltip::Position::Bottom,
        ));
    }
    right = right.push(tooltip(
        icon_toggle_button(labels::ICON_HUB_OUTLINED, false, actions.on_agent_graph),
        crate::pane_chrome::tip_bubble(labels::TOPBAR_VISUALIZE_AGENTS),
        tooltip::Position::Bottom,
    ));
    // ponytail: o grafo de agentes entra com o painel dele
    let right = right
        .push(icon_toggle_button(
            labels::ICON_SIDEBAR_RIGHT,
            !props.files_open,
            actions.on_toggle_files,
        ))
        .push(Space::new().width(12.0))
        .spacing(4)
        .align_y(Alignment::Center);

    let center_island = responsive(move |size| {
        if island_fits(size.width) {
            container(island_pill())
                .center_x(Fill)
                .center_y(Fill)
                .into()
        } else {
            Space::new().into()
        }
    });

    let bar = container(
        row![left, center_island, right]
            .width(Fill)
            .height(Fill)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::TOPBAR_HEIGHT - 1.0));

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

fn island_fits(width: f32) -> bool {
    width >= theme::TOPBAR_CENTER_MIN_WIDTH
}

fn icon_toggle_button<'a, M: Clone + 'a>(
    glyph: &'static str,
    selected: bool,
    on_press: M,
) -> Element<'a, M> {
    button(
        container(
            text(glyph)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TOPBAR_ICON)
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
    .width(Length::Fixed(theme::TOPBAR_ICON_BUTTON_SIZE))
    .height(Length::Fixed(theme::TOPBAR_ICON_BUTTON_SIZE))
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

fn counter<'a, M: 'a>(count: usize, color: iced::Color) -> Element<'a, M> {
    row![
        counter_dot(color),
        text(count.to_string())
            .font(theme::FONT_MONO)
            .size(theme::SIZE_LABEL)
            .color(if count == 0 {
                theme::palette().text4
            } else {
                theme::palette().text2
            })
    ]
    .spacing(theme::FARM_STRIP_DOT_GAP)
    .align_y(Alignment::Center)
    .into()
}

/// Contadores sempre visíveis, mesmo zerados: o Flutter só os esconde sem loops, mas
/// aqui os loops vêm só dos terminais observados por hook, e esconder deixaria a barra vazia.
fn farm_strip<'a, M: Clone + 'a>(counts: crate::farm::FarmCounts, on_open: M) -> Element<'a, M> {
    let waiting = counts.waiting;
    let counters = button(
        row![
            counter(counts.working, theme::palette().online),
            counter(waiting, theme::palette().warn),
            counter(counts.idle, theme::palette().git_untracked),
        ]
        .spacing(theme::FARM_STRIP_COUNTER_GAP)
        .align_y(Alignment::Center),
    )
    .height(theme::TOPBAR_ICON_BUTTON_SIZE)
    .padding([0.0, theme::FARM_STRIP_BUTTON_PADDING])
    .on_press(on_open.clone())
    .style(|_: &Theme, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(Background::Color(theme::palette().panel3)),
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    });
    let summary = labels::format_farm_summary(counts.working, waiting, counts.idle);
    let tooltip = tooltip(
        counters,
        text(summary).size(theme::SIZE_LABEL),
        tooltip::Position::Bottom,
    );
    let mut strip = row![tooltip].spacing(4).align_y(Alignment::Center);
    if waiting > 0 {
        let pill = button(
            row![
                counter_dot(theme::palette().warn),
                text(labels::farm_need_you(waiting))
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().warn),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .height(theme::FARM_STRIP_PILL_HEIGHT)
        .padding([0.0, theme::FARM_STRIP_PILL_PADDING])
        .on_press(on_open)
        .style(|_: &Theme, status| {
            let alpha = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                0.24
            } else {
                0.14
            };
            button::Style {
                background: Some(Background::Color(iced::Color {
                    a: alpha,
                    ..theme::palette().warn
                })),
                border: Border {
                    radius: theme::FARM_STRIP_PILL_RADIUS.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });
        strip = strip.push(pill);
    }
    // Flutter: 8px depois do strip; o `spacing(4)` da barra já cobre metade.
    container(strip)
        .padding(iced::Padding {
            right: theme::FARM_STRIP_TRAILING_GAP,
            ..iced::Padding::ZERO
        })
        .into()
}

fn counter_dot<'a, M: 'a>(color: iced::Color) -> Element<'a, M> {
    container(Space::new())
        .width(theme::FARM_STRIP_DOT_SIZE)
        .height(theme::FARM_STRIP_DOT_SIZE)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: (theme::FARM_STRIP_DOT_SIZE / 2.0).into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn island_requires_160_pixels_of_free_width() {
        assert!(!super::island_fits(159.99));
        assert!(super::island_fits(160.0));
        assert!(super::island_fits(161.0));
    }
}
