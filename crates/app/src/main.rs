// Shell da UI do GSH no Linux (iced). Layout de três colunas com a ilha de
// agentes no topo, igual ao GSH atual: workspaces à esquerda, panes no centro,
// arquivos à direita.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod gorilla;
pub mod island;
pub mod labels;

use iced::widget::{column, container, mouse_area, row, stack, text, Space};
use iced::{Background, Border, Color, Element, Fill, Padding, Theme};

use crate::island::{AgentIsland, IslandMsg};

const BG: Color = Color::from_rgb8(0x14, 0x14, 0x18);
const PANEL: Color = Color::from_rgb8(0x1c, 0x1c, 0x22);
const MUTED: Color = Color::from_rgb8(0x8a, 0x8a, 0x94);
const TEXT: Color = Color::from_rgb8(0xd4, 0xd4, 0xd4);

#[derive(Debug, Clone)]
enum Msg {
    Island(IslandMsg),
}

#[derive(Default)]
struct Shell {
    island: AgentIsland,
}

impl Shell {
    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Island(island_msg) => self.island.update(island_msg),
        }
    }

    fn view(&self) -> Element<'_, Msg> {
        let island_pill = self.island.view_pill().map(Msg::Island);

        let top = container(island_pill)
            .width(Fill)
            .center_x(Fill)
            .padding([4, 0]);

        let workspaces = panel(
            column![
                text(labels::WORKSPACES_HEADER).size(12).color(MUTED),
                text(labels::WORKSPACE_GSH).size(14).color(TEXT),
                text(labels::WORKSPACE_EXAMPLE).size(14).color(TEXT),
            ]
            .spacing(10)
            .padding(12),
            220.0,
        );

        let panes = container(text(labels::PANES_PLACEHOLDER).size(13).color(MUTED))
            .padding(16)
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| panel_style());

        let files = panel(
            column![
                text(labels::FILES_HEADER).size(12).color(MUTED),
                text(labels::FILES_PLACEHOLDER).size(13).color(MUTED),
            ]
            .spacing(10)
            .padding(12),
            240.0,
        );

        let body = row![workspaces, panes, files].spacing(8).height(Fill);

        let base = container(column![top, body].spacing(8).padding(8))
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(BG)),
                ..Default::default()
            });

        if self.island.is_expanded() {
            let panel_overlay = container(self.island.view_panel().map(Msg::Island))
                .width(Fill)
                .center_x(Fill)
                .padding(Padding {
                    top: 52.0,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                });

            let backdrop = mouse_area(
                container(Space::new().width(Fill).height(Fill))
                    .width(Fill)
                    .height(Fill),
            )
            .on_press(Msg::Island(IslandMsg::ClosePanel));

            stack![base, backdrop, panel_overlay].into()
        } else {
            base.into()
        }
    }
}

fn panel<'a>(content: impl Into<Element<'a, Msg>>, width: f32) -> Element<'a, Msg> {
    container(content)
        .width(width)
        .height(Fill)
        .style(|_: &Theme| panel_style())
        .into()
}

fn panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(PANEL)),
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn main() -> iced::Result {
    iced::application(Shell::default, Shell::update, Shell::view)
        .title(labels::APP_TITLE)
        .run()
}
