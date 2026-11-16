mod agents;
mod backdrop;
mod camera;
mod characters;
mod clawd;
pub mod crops;
mod dart_sort;
mod hud;
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
pub use agents::{AgentInput, AgentState, FarmCounts};
use backdrop::{Backdrop, ScreenState};
use camera::Camera;
use iced::advanced::mouse::{click, Click};
use iced::widget::image::Handle;
use iced::widget::{
    button,
    canvas::{self, Action, Event},
    container, row, stack, text,
};
use iced::{
    mouse, window, Background, Element, Fill, Point, Rectangle, Renderer, Size, Theme, Vector,
};
use world::{Mode, Prop};

/// Zoom por pixel de rolagem (`exp(-dy * 0.0018)` do Flutter) e quantos pixels vale uma
/// linha da roda (o Flutter no Linux reporta 53 por clique).
const ZOOM_PER_PIXEL: f64 = 0.0018;
const WHEEL_LINE_PIXELS: f64 = 53.0;

#[derive(Debug, Clone)]
pub enum Message {
    Focus(String),
    Office,
    Farm,
    Center,
    Pan(Vector),
    /// Ponto da janela, tamanho da janela e fator de zoom.
    Zoom(Point, iced::Size, f64),
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
    /// A chama do HUD, rasterizada uma vez.
    flame: Handle,
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
            flame: hud::flame(),
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
    pub fn counts(&self) -> FarmCounts {
        self.agents.counts()
    }
    pub fn sync_agents(&mut self, inputs: Vec<AgentInput>) {
        if self.agents.sync(inputs) {
            // a cena cresce (ou encolhe) com quem está nela
            self.relayout();
            self.cache.clear();
        }
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Focus(_) => {}
            Message::Office => {
                self.mode = Mode::Office;
                self.relayout();
            }
            Message::Farm => {
                self.mode = Mode::Farm;
                self.relayout();
            }
            Message::Center => {
                if let Some(b) = &self.backdrop {
                    self.camera.fit(&b.extent, &b.view);
                }
            }
            Message::Pan(delta) => {
                if let (Some(b), Some(size)) = (&self.backdrop, self.view_size) {
                    self.camera.pan(delta, &b.extent, size);
                }
            }
            Message::Zoom(at, view, factor) => {
                if let Some(b) = &self.backdrop {
                    self.camera.zoom_at(at, factor, &b.extent, view);
                }
            }
            Message::Resized(size) => {
                self.view_size = Some(size);
                self.relayout();
            }
        }
        self.cache.clear();
    }
    /// Refaz o mundo se a janela, o cenário ou o que ele precisa comportar mudaram
    /// (`_rebuild` do Flutter). Câmera enquadrada enquadra de novo; a que o usuário
    /// mexeu fica onde está, sem sair do mundo.
    /// ponytail: síncrono; assíncrono com debounce se a janela ficar lenta.
    fn relayout(&mut self) {
        let Some(size) = self.view_size else {
            return;
        };
        let need = self.agents.need();
        let mode = self.mode;
        if !self
            .backdrop
            .as_ref()
            .is_none_or(|b| b.size != size || b.mode != mode || b.outgrown(need))
        {
            return;
        }
        let first = self.backdrop.is_none();
        let backdrop = Backdrop::build(size, mode, need);
        self.agents.reslot(backdrop.capacity());
        if self.camera.auto || first {
            self.camera.fit(&backdrop.extent, &backdrop.view);
        } else {
            self.camera.z = self.camera.z.max(backdrop.extent.z);
            self.camera.clamp_to(&backdrop.extent, size);
        }
        self.backdrop = Some(backdrop);
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
        // Fazenda | Escritório: pílula no canto da cena (_SceneryToggle em farm_hud.dart).
        let option = |label: &'static str, msg, active| {
            let (fg, bg) = if active {
                (theme::scene::HUD_TEXT, Some(theme::scene::HUD_INK))
            } else {
                (theme::scene::HUD_MUTED, None)
            };
            button(
                text(label.to_uppercase())
                    .font(theme::FONT_MONO_BOLD)
                    .size(theme::scene::TOGGLE_TEXT_SIZE)
                    .line_height(text::LineHeight::Relative(theme::scene::TOGGLE_LINE_HEIGHT)),
            )
            .padding(theme::scene::TOGGLE_PADDING)
            .on_press_maybe((!active).then_some(msg))
            .style(move |_: &Theme, _| button::Style {
                background: bg.map(Background::Color),
                text_color: fg,
                ..button::Style::default()
            })
        };
        let toggle = hud::hud_box(
            row![
                option(labels::FARM_FARM, Message::Farm, self.mode == Mode::Farm),
                option(
                    labels::FARM_OFFICE,
                    Message::Office,
                    self.mode == Mode::Office
                )
            ]
            .spacing(theme::scene::TOGGLE_GAP),
        );
        // Sem fonte de tokens nem de subagentes no Rust ainda: os dois ficam em 0.
        let stats = hud::Stats {
            tokens: 0,
            agents: self.agents.items.len(),
            subagents: 0,
            counts: self.counts(),
        };
        let mut scene = stack![
            canvas::Canvas::new(Program(self)).width(Fill).height(Fill),
            container(hud::hud(&self.flame, &stats)).padding(theme::scene::HUD_MARGIN),
            container(toggle)
                .width(Fill)
                .align_right(Fill)
                .padding(theme::scene::HUD_MARGIN)
        ];
        if self.agents.items.is_empty() {
            scene = scene.push(hud::empty());
        }
        scene.width(Fill).height(Fill).into()
    }
}
struct Program<'a>(&'a FarmPane);
#[derive(Default)]
struct State {
    last: Option<Point>,
    pressed: Option<Point>,
    dragged: bool,
    click: Option<Click>,
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
                // duplo clique recentraliza a câmera (onDoubleTap em farm_view.dart)
                let at = state.last?;
                let click = Click::new(at, mouse::Button::Left, state.click);
                state.click = Some(click);
                if click.kind() == click::Kind::Double {
                    return Some(Action::publish(Message::Center).and_capture());
                }
                Some(Action::request_redraw().and_capture())
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
                // iced: y > 0 = rolar para cima = aproximar (no Flutter, dy < 0)
                let pixels = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => f64::from(*y) * WHEEL_LINE_PIXELS,
                    mouse::ScrollDelta::Pixels { y, .. } => f64::from(*y),
                };
                if pixels == 0.0 {
                    return None;
                }
                let factor = (pixels * ZOOM_PER_PIXEL).exp();
                Some(Action::publish(Message::Zoom(at, bounds.size(), factor)).and_capture())
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
                paint::draw_backdrop(frame, camera, backdrop);
            }
        });
        let mut layers = vec![base];
        // telas acesas no escritório: entre o fundo e os agentes
        if let Some(backdrop) = self.0.backdrop.as_ref().filter(|b| b.mode == Mode::Office) {
            let mut screens = canvas::Frame::new(renderer, size);
            paint::draw_screens(&mut screens, camera, backdrop, &self.0.screen_states());
            layers.push(screens.into_geometry());
        }
        let mut agents = canvas::Frame::new(renderer, size);
        characters::draw(&mut agents, &self.0.agents.items, &posts, camera);
        layers.push(agents.into_geometry());
        // a faixa da frente da fazenda passa por cima dos bichos
        if let Some(backdrop) = self.0.backdrop.as_ref().filter(|b| b.mode == Mode::Farm) {
            let mut front = canvas::Frame::new(renderer, size);
            paint::draw_front(&mut front, camera, backdrop);
            layers.push(front.into_geometry());
        }
        let mut overlay = canvas::Frame::new(renderer, bounds.size());
        if let Some(point) = cursor.position_in(bounds) {
            if let Some(agent) =
                characters::hit(&self.0.agents.items, &self.0.posts(), self.0.camera, point)
            {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(size: Size) -> FarmPane {
        let mut pane = FarmPane::default();
        pane.update(Message::Resized(size));
        pane
    }

    /// O mundo enquadrado cobre a janela inteira (sem margem vazia), no cenário largo e no alto.
    #[test]
    fn mundo_cobre_a_janela_largo_e_alto() {
        for (w, h) in [
            (1100.0, 700.0),
            (1600.0, 900.0),
            (560.0, 800.0),
            (420.0, 900.0),
        ] {
            let size = Size::new(w, h);
            let pane = pane(size);
            let b = pane.backdrop.as_ref().expect("fundo");
            let z = f64::from(pane.camera.scale());
            let (left, top) = (-pane.camera.x * z, -pane.camera.y * z);
            assert!(left <= 0.0 && top <= 0.0, "{w}x{h}");
            assert!(left + b.extent.w * z >= f64::from(w) - 1e-6, "{w}x{h}");
            assert!(top + b.extent.h * z >= f64::from(h) - 1e-6, "{w}x{h}");
        }
    }

    /// Arrastar e afastar o zoom não deixam margem: a câmera pára nas bordas do mundo.
    #[test]
    fn pan_e_zoom_nao_revelam_margem() {
        let size = Size::new(1100.0, 700.0);
        let mut pane = pane(size);
        pane.update(Message::Zoom(Point::new(500.0, 300.0), size, 0.001));
        pane.update(Message::Pan(Vector::new(5000.0, -5000.0)));
        let b = pane.backdrop.as_ref().expect("fundo");
        let z = f64::from(pane.camera.scale());
        let (left, top) = (-pane.camera.x * z, -pane.camera.y * z);
        assert!(left <= 1e-6 && top <= 1e-6);
        assert!(left + b.extent.w * z >= f64::from(size.width) - 1e-6);
        assert!(top + b.extent.h * z >= f64::from(size.height) - 1e-6);
    }

    /// Largo: HUD ao lado (left 240, top 8); alto: em cima (top 150).
    #[test]
    fn viewport_do_hud() {
        let wide = backdrop::viewport(Size::new(1100.0, 700.0));
        assert_eq!(
            (wide.left, wide.top, wide.right, wide.bottom),
            (240.0, 8.0, 12.0, 12.0)
        );
        let tall = backdrop::viewport(Size::new(560.0, 800.0));
        assert_eq!(
            (tall.left, tall.top, tall.right, tall.bottom),
            (0.0, 150.0, 12.0, 12.0)
        );
    }

    /// A cena cresce com os agentes (`outgrown`): 10 agentes pedem mais canteiros que o vazio.
    #[test]
    fn cena_cresce_com_os_agentes() {
        let mut pane = pane(Size::new(1100.0, 700.0));
        let empty = pane.backdrop.as_ref().map(Backdrop::capacity);
        let inputs = (0..10)
            .map(|i| AgentInput {
                pane_id: format!("p{i}"),
                name: format!("p{i}"),
                state: AgentState::Working,
            })
            .collect();
        pane.sync_agents(inputs);
        let grown = pane.backdrop.as_ref().map(Backdrop::capacity);
        assert!(grown > empty, "{empty:?} -> {grown:?}");
        assert!(grown >= Some(10));
    }
}
