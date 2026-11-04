// Shell da UI do GSH no Linux (iced). Layout de três colunas com a ilha no topo,
// igual ao GSH atual: workspaces à esquerda, panes no centro, arquivos à direita.
// Os dados ainda são fixos: o modelo de domínio (card F1) entra depois.
use iced::widget::{column, container, row, text};
use iced::{Background, Border, Color, Element, Fill, Theme};

const BG: Color = Color::from_rgb8(0x14, 0x14, 0x18);
const PANEL: Color = Color::from_rgb8(0x1c, 0x1c, 0x22);
const MUTED: Color = Color::from_rgb8(0x8a, 0x8a, 0x94);
const TEXT: Color = Color::from_rgb8(0xd4, 0xd4, 0xd4);

#[derive(Debug, Clone)]
enum Msg {}

struct Shell;

impl Shell {
    fn update(&mut self, msg: Msg) {
        match msg {}
    }

    fn view(&self) -> Element<'_, Msg> {
        let island = container(text("3 agentes").size(13).color(TEXT))
            .padding([6, 16])
            .style(|_: &Theme| pill());

        let top = container(island).width(Fill).center_x(Fill).padding([8, 0]);

        let workspaces = panel(
            column![
                text("Workspaces").size(12).color(MUTED),
                text("GSH").size(14).color(TEXT),
                text("Projeto de exemplo").size(14).color(TEXT),
            ]
            .spacing(10)
            .padding(12),
            220.0,
        );

        let panes = container(
            text("Terminal (spike de PTY ainda não ligado)")
                .size(13)
                .color(MUTED),
        )
        .padding(16)
        .width(Fill)
        .height(Fill)
        .style(|_: &Theme| panel_style());

        let files = panel(
            column![
                text("Arquivos").size(12).color(MUTED),
                text("(árvore entra depois)").size(13).color(MUTED),
            ]
            .spacing(10)
            .padding(12),
            240.0,
        );

        let body = row![workspaces, panes, files].spacing(8).height(Fill);

        container(column![top, body].spacing(8).padding(8))
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(BG)),
                ..Default::default()
            })
            .into()
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

fn pill() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::BLACK)),
        border: Border {
            radius: 14.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn main() -> iced::Result {
    iced::application(|| Shell, Shell::update, Shell::view)
        .title("GSH")
        .run()
}
