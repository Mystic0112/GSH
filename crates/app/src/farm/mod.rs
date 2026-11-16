mod agents;
mod backdrop;
mod camera;
mod characters;
mod clawd;
mod courier;
pub mod crops;
mod dart_sort;
mod hud;
pub mod icons;
mod layout;
pub mod lots;
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
use gsh_team::Knock;
use gsh_team::KnockKind;
use iced::advanced::mouse::{click, Click};
use iced::widget::image::Handle;
use iced::widget::{
    button,
    canvas::{self, Action, Event},
    column, container, row, stack, text,
};
use iced::{
    mouse, window, Background, Border, Element, Fill, Point, Rectangle, Renderer, Size, Theme,
    Vector,
};
use std::collections::HashMap;
use world::{Mode, Prop};

/// Zoom por pixel de rolagem (`exp(-dy * 0.0018)` do Flutter) e quantos pixels vale uma
/// linha da roda (o Flutter no Linux reporta 53 por clique).
const ZOOM_PER_PIXEL: f64 = 0.0018;
const WHEEL_LINE_PIXELS: f64 = 53.0;

#[derive(Debug, Clone)]
pub struct CourierFlight {
    pub from: Point,
    pub to: Point,
    pub path: Vec<Point>,
    pub bubble_at: Point,
    pub kind: KnockKind,
}

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
    AcceptKnock(String),
    RefuseKnock(String),
}

pub struct FarmPane {
    agents: Agents,
    peers: Vec<lots::Lot>,
    couriers: Vec<courier::Courier>,
    team_knocks: Vec<Knock>,
    door_targets: HashMap<String, String>,
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
            peers: Vec::new(),
            couriers: Vec::new(),
            team_knocks: Vec::new(),
            door_targets: HashMap::new(),
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
    pub fn sync_peers(&mut self, inputs: &[lots::PeerInput]) {
        if !lots::sync(&mut self.peers, inputs) {
            return;
        }
        if let Some(mine) = self.backdrop.as_mut() {
            for peer in &mut self.peers {
                peer.rebuild(mine, self.mode);
            }
            mine.extent.team = lots::team_bounds(
                mine,
                &mut self.peers,
                self.mode == Mode::Office,
                (mine.view.w, mine.view.h),
            );
            if self.camera.auto {
                self.camera.fit(&mine.extent, &mine.view);
            }
        }
        self.cache.clear();
    }
    pub fn sync_knocks(&mut self, knocks: Vec<Knock>, targets: HashMap<String, String>) {
        self.team_knocks = knocks;
        self.door_targets = targets;
    }
    pub fn add_courier(&mut self, flight: CourierFlight) {
        let now = std::time::Instant::now();
        self.couriers.retain(|courier| !courier.finished(now));
        self.couriers.push(courier::Courier::new(
            flight.from,
            flight.to,
            flight.path,
            flight.bubble_at,
            flight.kind,
        ));
        self.cache.clear();
    }
    pub fn courier_endpoints(
        &self,
        peer_id: &str,
        session_id: Option<&str>,
        local_tab: &str,
        outgoing: bool,
    ) -> Option<(Point, Point, Vec<Point>, Point)> {
        let mine = self.backdrop.as_ref()?;
        let peer = self.peers.iter().find(|lot| lot.id == peer_id)?;
        let peer_backdrop = peer.backdrop.as_ref()?;
        let agent_point = |items: &[agents::Agent], posts: &[world::Prop], id: &str| {
            items.iter().find(|a| a.input.pane_id == id).map(|a| {
                let p = characters::position(a, posts);
                Point::new(p.x - 11.0, p.y + 1.0)
            })
        };
        let peer_tab = session_id
            .and_then(|sid| {
                peer.contacts
                    .iter()
                    .find(|agent| agent.session_id.as_deref() == Some(sid))
            })
            .map_or(session_id.unwrap_or_default(), |agent| {
                agent.tab_id.as_str()
            });
        let remote_agent = agent_point(&peer.agents.items, &peer_backdrop.posts(), peer_tab)
            .map(|point| Point::new(point.x + peer.at.0 as f32, point.y + peer.at.1 as f32));
        let remote_at = remote_agent.unwrap_or_else(|| {
            let (x, y) = peer_backdrop.door;
            Point::new(x + peer.at.0 as f32, y + peer.at.1 as f32)
        });
        let local_agent = agent_point(&self.agents.items, &self.posts(), local_tab);
        let local_at = local_agent.unwrap_or_else(|| Point::new(mine.door.0, mine.door.1));
        let remote_bubble = remote_agent
            .map_or(Point::new(remote_at.x, remote_at.y - 25.0), |point| {
                Point::new(point.x + 18.0, point.y - 26.0)
            });
        let local_bubble = local_agent.map_or(Point::new(local_at.x, local_at.y - 25.0), |point| {
            Point::new(point.x + 18.0, point.y - 26.0)
        });
        let local_inside = Point::new(mine.door.0, mine.play.bottom as f32 - 4.0);
        let local_outside = Point::new(mine.door.0, mine.play.bottom as f32 + 8.0);
        let peer_inside = Point::new(
            peer_backdrop.door.0 + peer.at.0 as f32,
            peer.at.1 as f32 + peer_backdrop.play.bottom as f32 - 4.0,
        );
        let peer_outside = Point::new(
            peer_backdrop.door.0 + peer.at.0 as f32,
            peer.at.1 as f32 + peer_backdrop.play.bottom as f32 + 8.0,
        );
        Some(if outgoing {
            (
                local_at,
                remote_at,
                vec![
                    local_at,
                    local_inside,
                    local_outside,
                    peer_outside,
                    peer_inside,
                    remote_at,
                ],
                remote_bubble,
            )
        } else {
            (
                remote_at,
                local_at,
                vec![
                    remote_at,
                    peer_inside,
                    peer_outside,
                    local_outside,
                    local_inside,
                    local_at,
                ],
                local_bubble,
            )
        })
    }
    pub fn update(&mut self, message: Message) {
        match message {
            Message::Focus(_) => {}
            Message::AcceptKnock(id) | Message::RefuseKnock(id) => {
                self.team_knocks.retain(|knock| knock.id != id);
            }
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
        let mut backdrop = backdrop;
        for peer in &mut self.peers {
            peer.rebuild(&backdrop, mode);
        }
        backdrop.extent.team = lots::team_bounds(
            &backdrop,
            &mut self.peers,
            mode == Mode::Office,
            (f64::from(size.width), f64::from(size.height)),
        );
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
        let visible_knocks = visible_door_cards(&self.team_knocks);
        if !visible_knocks.is_empty() {
            let cards = visible_knocks
                .iter()
                .fold(column![].spacing(8), |cards, knock| {
                    cards.push(team_door_card(knock, &self.door_targets))
                });
            scene = scene.push(
                container(row![iced::widget::Space::new().width(Fill), cards])
                    .width(Fill)
                    .height(Fill)
                    .align_y(iced::alignment::Vertical::Bottom)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 16.0,
                        bottom: 16.0,
                        left: 0.0,
                    }),
            );
        }
        if self.agents.items.is_empty() && self.peers.is_empty() {
            scene = scene.push(hud::empty());
        }
        scene.width(Fill).height(Fill).into()
    }
}

struct Program<'a>(&'a FarmPane);

fn team_door_card<'a>(
    knock: &'a Knock,
    targets: &'a HashMap<String, String>,
) -> Element<'a, Message> {
    let palette = theme::palette();
    let target = targets
        .get(&knock.to_tab_id)
        .map_or(knock.to_tab_id.as_str(), String::as_str);
    let head = if knock.kind == KnockKind::File {
        labels::farm_team_door_file(
            &knock.from_name,
            &knock.from_agent,
            knock.file_name.as_deref().unwrap_or_default(),
            target,
        )
    } else {
        labels::farm_team_door_message(&knock.from_name, &knock.from_agent, target)
    };
    let mut body = column![
        text(labels::FARM_TEAM_DOOR_TITLE)
            .font(theme::FONT_BODY_MEDIUM)
            .size(12)
            .color(palette.warn),
        iced::widget::Space::new().height(4),
        text(head)
            .font(theme::FONT_BODY)
            .size(13.5)
            .color(palette.text),
    ]
    .spacing(4);
    let preview = knock.text.trim();
    if !preview.is_empty() {
        body = body.push(iced::widget::Space::new().height(2));
        body = body.push(
            text(preview)
                .font(theme::FONT_BODY_MEDIUM)
                .size(12)
                .wrapping(iced::widget::text::Wrapping::Word)
                .height(iced::Length::Fixed(56.0))
                .color(palette.text2),
        );
    }
    let actions = row![
        iced::widget::Space::new().width(Fill),
        door_button(
            labels::FARM_TEAM_DOOR_REFUSE,
            false,
            Message::RefuseKnock(knock.id.clone())
        ),
        door_button(
            labels::FARM_TEAM_DOOR_OPEN,
            true,
            Message::AcceptKnock(knock.id.clone())
        ),
    ]
    .spacing(8);
    body = body.push(iced::widget::Space::new().height(10));
    body = body.push(actions);
    container(body)
        .width(Fill)
        .max_width(380)
        .padding(12)
        .style(move |_| container::Style {
            background: Some(palette.panel2.into()),
            border: Border {
                color: palette.warn,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

fn visible_door_cards(knocks: &[Knock]) -> &[Knock] {
    let end = knocks.len().min(3);
    knocks.get(..end).unwrap_or_default()
}

fn door_button(
    label: &'static str,
    primary: bool,
    message: Message,
) -> iced::widget::Button<'static, Message> {
    let palette = theme::palette();
    button(text(label).font(theme::FONT_BODY_MEDIUM).size(13))
        .padding([6, 10])
        .on_press(message)
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if primary {
                    Some(
                        if hovered {
                            palette.panel3
                        } else {
                            palette.accent
                        }
                        .into(),
                    )
                } else if hovered {
                    Some(palette.panel3.into())
                } else {
                    None
                },
                text_color: if primary { palette.bg } else { palette.text },
                border: Border {
                    color: if primary {
                        palette.accent
                    } else {
                        palette.border2
                    },
                    width: if primary { 0.0 } else { 1.0 },
                    radius: 8.0.into(),
                },
                ..button::Style::default()
            }
        })
}

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
        for peer in &self.0.peers {
            let Some(backdrop) = peer.backdrop.as_ref() else {
                continue;
            };
            let peer_camera = Camera {
                x: camera.x - peer.at.0,
                y: camera.y - peer.at.1,
                ..camera
            };
            let mut peer_layer = canvas::Frame::new(renderer, size);
            let clip_left = if backdrop.mode == Mode::Office && peer.clip > 0.0 {
                peer_camera
                    .to_screen(Point::new(peer.clip as f32, 0.0))
                    .x
                    .max(0.0)
            } else {
                0.0
            };
            let clip = Rectangle {
                x: clip_left,
                y: 0.0,
                width: (size.width - clip_left).max(0.0),
                height: size.height,
            };
            peer_layer.with_clip(clip, |layer| {
                paint::draw_backdrop(layer, peer_camera, backdrop);
                if backdrop.mode == Mode::Farm {
                    let label = labels::farm_peer_banner(&peer.name);
                    let center = peer_camera.to_screen(Point::new(backdrop.extent.cx as f32, 8.0));
                    let width = label.chars().count() as f32 * 6.0 + 12.0;
                    layer.fill_rectangle(
                        Point::new(center.x - width / 2.0, center.y),
                        Size::new(width, 13.0),
                        theme::scene::HUD_BG,
                    );
                    layer.fill_text(canvas::Text {
                        content: label,
                        position: Point::new(center.x - width / 2.0 + 5.0, center.y + 2.0),
                        color: theme::scene::HUD_TEXT,
                        size: iced::Pixels(8.0),
                        ..canvas::Text::default()
                    });
                }
                if backdrop.mode == Mode::Office {
                    let states: Vec<_> = (0..peer.backdrop.as_ref().map_or(0, Backdrop::capacity))
                        .map(|i| {
                            peer.agents.items.iter().find(|a| a.slot == i).map_or(
                                ScreenState::Off,
                                |a| match a.input.state {
                                    AgentState::Working => ScreenState::Working,
                                    AgentState::Waiting => ScreenState::Waiting,
                                    _ => ScreenState::Off,
                                },
                            )
                        })
                        .collect();
                    paint::draw_screens(layer, peer_camera, backdrop, &states);
                }
                characters::draw(layer, &peer.agents.items, &backdrop.posts(), peer_camera);
                characters::draw_minis(
                    layer,
                    &peer.subagents,
                    &peer.agents.items,
                    &backdrop.posts(),
                    peer_camera,
                );
                if backdrop.mode == Mode::Farm {
                    paint::draw_front(layer, peer_camera, backdrop);
                }
            });
            layers.push(peer_layer.into_geometry());
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
        let now = std::time::Instant::now();
        for courier in &self.0.couriers {
            let pose = courier.pose(now);
            if pose.stage == courier::Stage::Done {
                continue;
            }
            let point = self.0.camera.to_screen(pose.at);
            let z = self.0.camera.scale();
            let pose_data = clawd::Pose {
                look: pose.direction,
                legs: pose.legs,
                blink: false,
                sleep: false,
                arms: if pose.stage == courier::Stage::Walk {
                    1
                } else {
                    0
                },
                happy: false,
            };
            let sprite = clawd::courier_frame(pose_data);
            overlay.draw_image(
                Rectangle {
                    x: point.x - 8.0 * z,
                    y: point.y - 17.0 * z + pose.hop * z,
                    width: 16.0 * z,
                    height: 18.0 * z,
                },
                iced::advanced::image::Image::new(sprite)
                    .filter_method(iced::advanced::image::FilterMethod::Nearest)
                    .opacity(pose.fade),
            );
            if pose.stage == courier::Stage::Walk {
                let icon = match courier.kind {
                    KnockKind::Message => icons::FarmIcon::Envelope,
                    KnockKind::File => icons::FarmIcon::Parcel,
                };
                overlay.draw_image(
                    Rectangle {
                        x: point.x - 2.5 * z,
                        y: point.y - 20.0 * z + pose.hop * z,
                        width: 5.0 * z,
                        height: 5.0 * z,
                    },
                    iced::advanced::image::Image::new(Handle::from_rgba(
                        10,
                        10,
                        icon.rgba().to_vec(),
                    ))
                    .filter_method(iced::advanced::image::FilterMethod::Nearest),
                );
            }
            if pose.stage == courier::Stage::Deliver {
                let anchor = self.0.camera.to_screen(courier.bubble_at);
                let unit = if z >= 3.0 { 2.0 } else { 1.0 };
                let width = 16.0 * unit;
                let height = 16.0 * unit;
                let left = (anchor.x - width / 2.0).round();
                let top = anchor.y - height - 3.0 * unit;
                let center = anchor.x.round();
                let ink = iced::Color::from_rgb8(0x1f, 0x2a, 0x44);
                let fill = iced::Color::from_rgb8(0xff, 0xf8, 0xe8);
                overlay.fill_rectangle(
                    Point::new(left + unit, top),
                    Size::new(width - 2.0 * unit, height),
                    ink,
                );
                overlay.fill_rectangle(
                    Point::new(left, top + unit),
                    Size::new(width, height - 2.0 * unit),
                    ink,
                );
                overlay.fill_rectangle(
                    Point::new(center - 2.0 * unit, top + height - unit),
                    Size::new(4.0 * unit, 2.0 * unit),
                    ink,
                );
                overlay.fill_rectangle(
                    Point::new(center - unit, top + height + unit),
                    Size::new(2.0 * unit, unit),
                    ink,
                );
                overlay.fill_rectangle(
                    Point::new(left + unit, top + unit),
                    Size::new(width - 2.0 * unit, height - 2.0 * unit),
                    fill,
                );
                overlay.fill_rectangle(
                    Point::new(center - unit, top + height - unit),
                    Size::new(2.0 * unit, unit),
                    fill,
                );
                let icon = match courier.kind {
                    KnockKind::Message => icons::FarmIcon::Envelope,
                    KnockKind::File => icons::FarmIcon::Parcel,
                };
                overlay.draw_image(
                    Rectangle {
                        x: left + 3.0 * unit,
                        y: top + 3.0 * unit,
                        width: 10.0 * unit,
                        height: 10.0 * unit,
                    },
                    iced::advanced::image::Image::new(Handle::from_rgba(
                        10,
                        10,
                        icon.rgba().to_vec(),
                    ))
                    .filter_method(iced::advanced::image::FilterMethod::Nearest),
                );
            }
            if pose.stage == courier::Stage::Vanish {
                let spread = 1.0 - pose.fade;
                for i in 0..4 {
                    let angle = i as f32 * std::f32::consts::FRAC_PI_2 + 0.6;
                    let at = Point::new(
                        pose.at.x - 1.75 + angle.cos() * (3.0 + spread * 9.0),
                        pose.at.y - 11.75 + angle.sin() * (3.0 + spread * 7.0),
                    );
                    let at = self.0.camera.to_screen(at);
                    overlay.draw_image(
                        Rectangle {
                            x: at.x,
                            y: at.y,
                            width: 3.5 * z,
                            height: 3.5 * z,
                        },
                        iced::advanced::image::Image::new(clawd::sparkle(i))
                            .filter_method(iced::advanced::image::FilterMethod::Nearest)
                            .opacity(pose.fade),
                    );
                }
            }
        }
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

    #[test]
    fn colega_remoto_com_agente_working_tem_sprite_e_rotulo_no_lote() {
        for mode in [Mode::Farm, Mode::Office] {
            let size = Size::new(1100.0, 700.0);
            let mut farm = pane(size);
            farm.mode = mode;
            farm.relayout();
            farm.sync_peers(&[lots::PeerInput {
                id: "beta-id".into(),
                name: "beta".into(),
                agents: vec![AgentInput {
                    pane_id: "session-beta".into(),
                    name: "Terminal beta".into(),
                    state: AgentState::Working,
                }],
                subagents: Vec::new(),
                contacts: Vec::new(),
                address: None,
                mail_port: None,
            }]);

            let lot = farm.peers.first().expect("lote beta");
            let backdrop = lot.backdrop.as_ref().expect("fundo do lote beta");
            assert_eq!(lot.name, "beta");
            assert_eq!(lot.agents.items.len(), 1);
            assert_eq!(
                lot.agents.items.first().map(|a| a.input.state),
                Some(AgentState::Working)
            );
            assert_eq!(labels::farm_peer_banner(&lot.name), "TIME · beta");

            let camera = Camera {
                x: farm.camera.x - lot.at.0,
                y: farm.camera.y - lot.at.1,
                ..farm.camera
            };
            let rect = characters::sprite_rect(
                lot.agents.items.first().expect("agente beta"),
                &backdrop.posts(),
                camera,
            );
            assert!(rect.width > 0.0 && rect.height > 0.0);
            assert!(rect.x.is_finite() && rect.y.is_finite());
        }
    }

    #[test]
    fn envio_loopback_vira_knock_e_cartao_de_porta() {
        use gsh_team::{Mail, TeamConfig};
        use std::net::{IpAddr, Ipv4Addr};
        use std::thread;
        use std::time::Duration;

        let config = TeamConfig {
            enabled: true,
            team: "loopback-team".into(),
            secret: "shared-secret".into(),
            ..TeamConfig::default()
        };
        let receiver = Mail::start(&config)
            .expect("mail starts")
            .expect("mail enabled");
        let knock = Knock {
            id: "cli-send-loopback".into(),
            kind: KnockKind::Message,
            from_machine: "alpha-machine".into(),
            from_name: "alpha".into(),
            from_agent: "alpha-agent".into(),
            from_session_id: Some("alpha-session".into()),
            to_tab_id: "beta-agent-tab".into(),
            text: "hello from gsh send".into(),
            file_name: None,
            file_bytes: None,
        };
        Mail::send(
            &config,
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            receiver.port(),
            &knock,
        )
        .expect("CLI send transport succeeds");
        let received = (0..100)
            .find_map(|_| {
                let next = receiver.try_recv();
                if next.is_none() {
                    thread::sleep(Duration::from_millis(10));
                }
                next
            })
            .expect("mail produces pending knock");
        assert_eq!(received, knock);

        let mut farm = FarmPane::default();
        farm.sync_knocks(
            vec![received],
            HashMap::from([("beta-agent-tab".into(), "beta agent".into())]),
        );
        let cards = visible_door_cards(&farm.team_knocks);
        assert_eq!(cards.len(), 1);
        assert_eq!(
            cards.first().map(|card| card.id.as_str()),
            Some("cli-send-loopback")
        );
        assert!(
            labels::farm_team_door_message("alpha", "alpha-agent", "beta agent")
                .contains("beta agent")
        );
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
