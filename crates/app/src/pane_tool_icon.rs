//! Ícones desenhados da barra do pane (`_SplitterScreenIcon` de `pane_view.dart`).
//!
//! Dois retângulos de contorno lado a lado (dividir à direita), empilhados
//! (dividir abaixo) ou um × de duas barras redondas (fechar), todos em `TEXT3`,
//! com traço de 10% do lado.

use iced::widget::canvas::{self, LineCap, Path, Stroke};
use iced::{mouse, Point, Rectangle, Renderer, Size, Theme};

use crate::theme;

#[derive(Debug, Clone, Copy)]
pub enum PaneToolIcon {
    SplitRight,
    SplitDown,
    Close,
}

impl<M> canvas::Program<M> for PaneToolIcon {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let side = bounds.width.min(bounds.height);
        let line = side * theme::PANE_TOOL_STROKE_RATIO;
        let stroke = Stroke::default()
            .with_color(theme::palette().text3)
            .with_width(line);
        match self {
            Self::SplitRight => {
                let box_width = (side - line) / 2.0;
                for left in [0.0, box_width + line] {
                    let path = Path::rounded_rectangle(
                        Point::new(left + line / 2.0, line / 2.0),
                        Size::new(box_width - line, side - line),
                        (line / 2.0).into(),
                    );
                    frame.stroke(&path, stroke);
                }
            }
            Self::SplitDown => {
                let box_height = (side - line) / 2.0;
                for top in [0.0, box_height + line] {
                    let path = Path::rounded_rectangle(
                        Point::new(line / 2.0, top + line / 2.0),
                        Size::new(side - line, box_height - line),
                        (line / 2.0).into(),
                    );
                    frame.stroke(&path, stroke);
                }
            }
            Self::Close => {
                // Barras de comprimento `side` rotacionadas 45°; a ponta redonda soma meia espessura.
                let reach = (side / 2.0 - line / 2.0) * std::f32::consts::FRAC_1_SQRT_2;
                let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
                let stroke = stroke.with_line_cap(LineCap::Round);
                for sign in [1.0, -1.0] {
                    let path = Path::line(
                        Point::new(center.x - reach, center.y - sign * reach),
                        Point::new(center.x + reach, center.y + sign * reach),
                    );
                    frame.stroke(&path, stroke);
                }
            }
        }
        vec![frame.into_geometry()]
    }
}
