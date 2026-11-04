// Spike: grid de "terminal" a 60 fps + ilha animada no topo, em iced 0.14.
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Program};
use iced::widget::{canvas as canvas_widget, column};
use iced::{Color, Element, Fill, Font, Point, Rectangle, Renderer, Size, Subscription, Theme};
use std::time::{Duration, Instant};

const COLS: usize = 160;
const ROWS: usize = 48;
const CELL_H: f32 = 16.0;

struct Spike {
    lines: Vec<String>,
    tick: u64,
    started: Instant,
}

#[derive(Debug, Clone)]
enum Msg {
    Tick,
}

impl Spike {
    fn boot() -> Self {
        Self {
            lines: vec![String::new(); ROWS],
            tick: 0,
            started: Instant::now(),
        }
    }

    fn update(&mut self, msg: Msg) {
        match msg {
            Msg::Tick => {
                self.tick += 1;
                // simula saída de terminal: uma linha nova por tick, rolando
                let row = (self.tick as usize) % ROWS;
                let text: String = (0..COLS)
                    .map(|i| {
                        let c = (self.tick as usize + i * 7) % 62;
                        match c {
                            0..=9 => (b'0' + c as u8) as char,
                            10..=35 => (b'a' + (c - 10) as u8) as char,
                            _ => (b'A' + (c - 36) as u8) as char,
                        }
                    })
                    .collect();
                self.lines[row] = text;
            }
        }
    }

    fn view(&self) -> Element<'_, Msg> {
        column![canvas_widget(Screen { spike: self }).width(Fill).height(Fill)].into()
    }

    fn subscription(&self) -> Subscription<Msg> {
        iced::time::every(Duration::from_millis(16)).map(|_| Msg::Tick)
    }
}

struct Screen<'a> {
    spike: &'a Spike,
}

impl<'a> Program<Msg> for Screen<'a> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // fundo do "terminal"
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb8(0x14, 0x14, 0x18),
        );

        // uma linha de texto por row (agrupado, como um terminal real faz)
        for (r, line) in self.spike.lines.iter().enumerate() {
            frame.fill_text(canvas::Text {
                content: line.clone(),
                position: Point::new(8.0, 40.0 + r as f32 * CELL_H),
                color: Color::from_rgb8(0xd4, 0xd4, 0xd4),
                size: 13.0.into(),
                font: Font::MONOSPACE,
                ..canvas::Text::default()
            });
        }

        // ilha animada no topo: largura oscila, bolinha gira
        let t = self.spike.started.elapsed().as_secs_f32();
        let phase = (t * std::f32::consts::TAU / 2.0).sin() * 0.5 + 0.5;
        let w = 140.0 + phase * 220.0;
        let x = (bounds.width - w) / 2.0;
        frame.fill(
            &Path::rounded_rectangle(Point::new(x, 8.0), Size::new(w, 28.0), 14.0.into()),
            Color::BLACK,
        );
        let ang = t * 4.0;
        let cx = x + 18.0;
        let cy = 22.0;
        let dot = Path::circle(Point::new(cx + ang.cos() * 4.0, cy + ang.sin() * 4.0), 3.0);
        frame.fill(&dot, Color::from_rgb8(0x4a, 0xde, 0x80));
        frame.fill_text(canvas::Text {
            content: format!("{} agentes", 3 + (phase * 2.0) as u32),
            position: Point::new(x + 32.0, 14.0),
            color: Color::WHITE,
            size: 13.0.into(),
            ..canvas::Text::default()
        });

        vec![frame.into_geometry()]
    }
}

fn main() -> iced::Result {
    iced::application(Spike::boot, Spike::update, Spike::view)
        .subscription(Spike::subscription)
        .title("GSH spike iced")
        .run()
}
