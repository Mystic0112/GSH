mod camera;
pub mod icons;
mod paint;
mod sprites;
mod world;

use crate::{labels, theme};
use camera::Camera;
use iced::widget::{
    button,
    canvas::{self, Action, Event},
    column, row, text,
};
use iced::{mouse, Element, Fill, Point, Rectangle, Renderer, Theme, Vector};
use world::Mode;

#[derive(Debug, Clone)]
pub enum Message {
    Office,
    Farm,
    Center,
    Pan(Vector),
    Zoom(Point, iced::Size, f32),
}

pub struct FarmPane {
    mode: Mode,
    camera: Camera,
    cache: canvas::Cache,
}
impl Default for FarmPane {
    fn default() -> Self {
        Self {
            mode: Mode::Office,
            camera: Camera::default(),
            cache: canvas::Cache::new(),
        }
    }
}
impl FarmPane {
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Office => self.mode = Mode::Office,
            Message::Farm => self.mode = Mode::Farm,
            Message::Center => self.camera = Camera::default(),
            Message::Pan(delta) => self.camera.offset += delta,
            Message::Zoom(at, view, step) => self.camera.zoom_at(at, view, step),
        }
        self.cache.clear();
    }
    pub fn view(&self) -> Element<'_, Message> {
        let control = |label, msg, active| {
            button(text(label).size(theme::SIZE_LABEL))
                .on_press(msg)
                .style(move |theme, status| {
                    if active {
                        button::primary(theme, status)
                    } else {
                        button::secondary(theme, status)
                    }
                })
        };
        let toolbar = row![
            control(
                labels::FARM_OFFICE,
                Message::Office,
                self.mode == Mode::Office
            ),
            control(labels::FARM_FARM, Message::Farm, self.mode == Mode::Farm),
            control(labels::FARM_CENTER, Message::Center, false)
        ]
        .spacing(6)
        .padding(6);
        column![
            toolbar,
            canvas::Canvas::new(Program(self)).width(Fill).height(Fill)
        ]
        .width(Fill)
        .height(Fill)
        .into()
    }
}
struct Program<'a>(&'a FarmPane);
#[derive(Default)]
struct State {
    last: Option<Point>,
}
impl canvas::Program<Message> for Program<'_> {
    type State = State;
    fn update(
        &self,
        state: &mut State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.last = cursor.position_in(bounds);
                state.last.map(|_| Action::request_redraw().and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.last = None;
                None
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let previous = state.last?;
                let next = Point::new(position.x - bounds.x, position.y - bounds.y);
                state.last = Some(next);
                Some(Action::publish(Message::Pan(next - previous)).and_capture())
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let at = cursor.position_in(bounds)?;
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        *y
                    }
                };
                if y == 0.0 {
                    return None;
                }
                Some(Action::publish(Message::Zoom(at, bounds.size(), y.signum())).and_capture())
            }
            _ => None,
        }
    }
    fn draw(
        &self,
        _state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        vec![self.0.cache.draw(renderer, bounds.size(), |frame| {
            paint::draw(frame, bounds.size(), self.0.camera, self.0.mode)
        })]
    }
}
