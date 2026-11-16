mod agents;
mod backdrop;
mod camera;
mod characters;
mod clawd;
pub mod crops;
mod dart_sort;
pub mod icons;
mod layout;
pub mod office;
mod office_paint;
mod paint;
mod pix;
mod raster;
mod world;
mod world_paint;
mod world_props;

use crate::{labels, theme};
use agents::Agents;
pub use agents::{AgentInput, AgentState};
use backdrop::{Backdrop, ScreenState};
use camera::Camera;
use iced::widget::{
    button,
    canvas::{self, Action, Event},
    column, row, text,
};
use iced::{mouse, window, Element, Fill, Point, Rectangle, Renderer, Size, Theme, Vector};
use world::{Mode, Prop};

#[derive(Debug, Clone)]
pub enum Message {
    Focus(String),
    Office,
    Farm,
    Center,
    Pan(Vector),
    Zoom(Point, iced::Size, f32),
    Resized(Size),
}

pub struct FarmPane {
    agents: Agents,
    mode: Mode,
    camera: Camera,
    cache: canvas::Cache,
    /// Última janela conhecida: o fundo é refeito quando ela muda.
    view_size: Option<Size>,
    backdrop: Option<Backdrop>,
}
impl Default for FarmPane {
    fn default() -> Self {
        Self {
            agents: Agents::default(),
            mode: Mode::Office,
            camera: Camera::default(),
            cache: canvas::Cache::new(),
            view_size: None,
            backdrop: None,
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
            Message::Office => {
                self.mode = Mode::Office;
                self.refit_backdrop();
                self.camera = self.fit_camera();
            }
            Message::Farm => {
                self.mode = Mode::Farm;
                self.refit_backdrop();
                self.camera = self.fit_camera();
            }
            Message::Center => self.camera = self.fit_camera(),
            Message::Pan(delta) => self.camera.offset += delta,
            Message::Zoom(at, view, step) => self.camera.zoom_at(at, view, step),
            Message::Resized(size) => {
                self.view_size = Some(size);
                self.refit_backdrop();
                self.camera = self.fit_camera();
            }
        }
        self.cache.clear();
    }
    /// Refaz o fundo para a janela atual, se ela mudou desde o último layout.
    /// ponytail: refaz a cada mudança de tamanho, síncrono; assíncrono se a janela ficar lenta.
    fn refit_backdrop(&mut self) {
        let Some(size) = self.view_size else {
            return;
        };
        let mode = self.mode;
        if self
            .backdrop
            .as_ref()
            .is_none_or(|b| b.size != size || b.mode != mode)
        {
            self.backdrop = Some(Backdrop::build(size, mode));
        }
    }
    /// Câmera da fazenda: o zoom que mostra o layout inteiro; sem fundo, a padrão.
    fn fit_camera(&self) -> Camera {
        self.backdrop
            .as_ref()
            .map_or_else(Camera::default, |b| Camera {
                zoom: b.zoom,
                offset: Vector::ZERO,
            })
    }
    /// Postos dos agentes: canteiros na fazenda, ilhas no escritório (mesmo layout).
    fn posts(&self) -> Vec<Prop> {
        self.backdrop
            .as_ref()
            .map_or_else(Vec::new, Backdrop::posts)
    }
    /// Estado da tela de cada ilha: o agente que senta nela (slot = índice da ilha).
    fn screen_states(&self) -> Vec<ScreenState> {
        let count = self.posts().len();
        (0..count)
            .map(|i| {
                match self
                    .agents
                    .items
                    .iter()
                    .find(|a| a.slot == i)
                    .map(|a| a.input.state)
                {
                    Some(AgentState::Working) => ScreenState::Working,
                    Some(AgentState::Waiting) => ScreenState::Waiting,
                    _ => ScreenState::Off,
                }
            })
            .collect()
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
        // a janela avisa o tamanho a cada quadro; se mudou, o fundo é refeito
        if matches!(event, Event::Window(window::Event::RedrawRequested(_)))
            && self.0.view_size != Some(bounds.size())
        {
            return Some(Action::publish(Message::Resized(bounds.size())));
        }
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
                                &self.0.posts(),
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
        let (size, camera) = (bounds.size(), self.0.camera);
        let posts = self.0.posts();
        // o canvas desenha as imagens por cima das formas de um mesmo geometry: cada camada vai no seu
        let base = self.0.cache.draw(renderer, size, |frame| {
            if let Some(backdrop) = &self.0.backdrop {
                paint::draw_backdrop(frame, size, camera, backdrop);
            }
        });
        let mut layers = vec![base];
        // telas acesas no escritório: entre o fundo e os agentes
        if let Some(backdrop) = self.0.backdrop.as_ref().filter(|b| b.mode == Mode::Office) {
            let mut screens = canvas::Frame::new(renderer, size);
            paint::draw_screens(
                &mut screens,
                size,
                camera,
                backdrop,
                &self.0.screen_states(),
            );
            layers.push(screens.into_geometry());
        }
        let mut agents = canvas::Frame::new(renderer, size);
        characters::draw(&mut agents, &self.0.agents.items, &posts, camera, size);
        layers.push(agents.into_geometry());
        // a faixa da frente da fazenda passa por cima dos bichos
        if let Some(backdrop) = self.0.backdrop.as_ref().filter(|b| b.mode == Mode::Farm) {
            let mut front = canvas::Frame::new(renderer, size);
            paint::draw_front(&mut front, size, camera, backdrop);
            layers.push(front.into_geometry());
        }
        let mut overlay = canvas::Frame::new(renderer, bounds.size());
        if let Some(point) = cursor.position_in(bounds) {
            if let Some(agent) = characters::hit(
                &self.0.agents.items,
                &self.0.posts(),
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
        layers.push(overlay.into_geometry());
        layers
    }
}
