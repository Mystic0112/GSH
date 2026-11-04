// Spike: mesmo cenário do spike-iced (grid "terminal" a 60 fps + ilha animada) em gpui 0.2.2.
use gpui::{prelude::*, *};
use std::time::{Duration, Instant};

const COLS: usize = 160;
const ROWS: usize = 48;

struct Spike {
    lines: Vec<String>,
    tick: u64,
    started: Instant,
}

impl Spike {
    fn new(cx: &mut Context<Self>) -> Self {
        // loop de 16 ms que pede redraw, igual ao subscription do iced
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(16))
                    .await;
                if this
                    .update(cx, |s, cx| {
                        s.step();
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            lines: vec![String::new(); ROWS],
            tick: 0,
            started: Instant::now(),
        }
    }

    fn step(&mut self) {
        self.tick += 1;
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

impl Render for Spike {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let t = self.started.elapsed().as_secs_f32();
        let phase = (t * std::f32::consts::TAU / 2.0).sin() * 0.5 + 0.5;
        let w = 140.0 + phase * 220.0;

        let mut screen = div()
            .size_full()
            .bg(rgb(0x141418))
            .font_family("monospace")
            .text_size(px(13.0))
            .text_color(rgb(0xd4d4d4))
            .flex()
            .flex_col()
            .pt(px(40.0))
            .pl(px(8.0));
        for line in &self.lines {
            screen = screen.child(div().h(px(16.0)).child(line.clone()));
        }

        div()
            .size_full()
            .relative()
            .child(screen)
            .child(
                div()
                    .absolute()
                    .top(px(8.0))
                    .left_0()
                    .w_full()
                    .flex()
                    .justify_center()
                    .child(
                        div()
                            .w(px(w))
                            .h(px(28.0))
                            .rounded(px(14.0))
                            .bg(rgb(0x000000))
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(rgb(0xffffff))
                            .child(format!("{} agentes", 3 + (phase * 2.0) as u32)),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_, cx| cx.new(Spike::new))
            .unwrap();
        cx.activate(true);
    });
}
