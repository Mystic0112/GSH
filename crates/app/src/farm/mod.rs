mod agents;
mod camera;
mod characters;
pub mod crops;
pub mod icons;
mod layout;
pub mod office;
mod paint;
mod raster;
mod sprites;
mod world;
mod world_paint;

use crate::{labels, theme};
use agents::Agents;
pub use agents::{AgentInput, AgentState};
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
    Focus(String),
    Office,
    Farm,
    Center,
    Pan(Vector),
    Zoom(Point, iced::Size, f32),
}

pub struct FarmPane {
    agents: Agents,
    mode: Mode,
    camera: Camera,
    cache: canvas::Cache,
}
impl Default for FarmPane {
    fn default() -> Self {
        Self {
            agents: Agents::default(),
            mode: Mode::Office,
            camera: Camera::default(),
            cache: canvas::Cache::new(),
        }
    }
}
impl FarmPane {
    #[cfg(test)]
    pub fn agent_states(&self) -> Vec<AgentState> {
        self.agents
            .items
            .iter()
            .map(|agent| agent.input.state)
            .collect()
    }
    pub fn sync_agents(&mut self, inputs: Vec<AgentInput>) {
        if self.agents.sync(inputs) {
            self.cache.clear();
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Focus(_) => {}
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
        let count = |state| {
            self.agents
                .items
                .iter()
                .filter(|a| a.input.state == state)
                .count()
        };
        let stat = |label, state, color| {
            text(format!("{} {}", label, count(state)))
                .size(theme::SIZE_LABEL)
                .color(color)
        };
        let hud = row![
            stat(
                labels::FARM_WORKING,
                AgentState::Working,
                theme::palette().ok
            ),
            stat(
                labels::FARM_WAITING,
                AgentState::Waiting,
                theme::palette().warn
            ),
            stat(labels::FARM_IDLE, AgentState::Idle, theme::palette().online),
            stat(labels::FARM_DONE, AgentState::Done, theme::palette().error),
        ]
        .spacing(16)
        .padding(6);
        column![
            toolbar,
            hud,
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
    pressed: Option<Point>,
    dragged: bool,
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
                state.pressed = state.last;
                state.dragged = false;
                state.last.map(|_| Action::request_redraw().and_capture())
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.last = None;
                let pressed = state.pressed.take();
                if !state.dragged {
                    if let (Some(start), Some(at)) = (pressed, cursor.position_in(bounds)) {
                        if (at.x - start.x).hypot(at.y - start.y) < 4.0 {
                            if let Some(agent) = characters::hit(
                                &self.0.agents.items,
                                self.0.mode,
                                self.0.camera,
                                bounds.size(),
                                at,
                            ) {
                                return Some(
                                    Action::publish(Message::Focus(agent.input.pane_id.clone()))
                                        .and_capture(),
                                );
                            }
                        }
                    }
                }
                None
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let Some(previous) = state.last else {
                    return Some(Action::request_redraw());
                };
                let next = Point::new(position.x - bounds.x, position.y - bounds.y);
                state.last = Some(next);
                if state
                    .pressed
                    .is_some_and(|p| (next.x - p.x).hypot(next.y - p.y) >= 4.0)
                {
                    state.dragged = true;
                }
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
        cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let scene = self.0.cache.draw(renderer, bounds.size(), |frame| {
            paint::draw(frame, bounds.size(), self.0.camera, self.0.mode);
            characters::draw(
                frame,
                &self.0.agents.items,
                self.0.mode,
                self.0.camera,
                bounds.size(),
            );
        });
        let mut overlay = canvas::Frame::new(renderer, bounds.size());
        if let Some(point) = cursor.position_in(bounds) {
            if let Some(agent) = characters::hit(
                &self.0.agents.items,
                self.0.mode,
                self.0.camera,
                bounds.size(),
                point,
            ) {
                let width = agent.input.name.chars().count() as f32 * 8.0 + 10.0;
                let pos = Point::new(
                    (point.x + 10.0).min((bounds.width - width).max(0.0)),
                    (point.y - 22.0).max(0.0),
                );
                overlay.fill_rectangle(pos, iced::Size::new(width, 20.0), theme::palette().panel);
                overlay.fill_text(canvas::Text {
                    content: agent.input.name.clone(),
                    position: Point::new(pos.x + 5.0, pos.y + 2.0),
                    color: theme::palette().text,
                    size: iced::Pixels(13.0),
                    ..canvas::Text::default()
                });
            }
        }
        vec![scene, overlay.into_geometry()]
    }
}
