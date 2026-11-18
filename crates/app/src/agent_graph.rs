//! Canvas de agentes. As coordenadas persistidas são as coordenadas da cena Flutter.

use iced::widget::canvas::{Action, Event};
use iced::widget::{button, canvas, column, container, row, text, text_input, Space};
use iced::{mouse, Alignment, Background, Border, Element, Fill, Point, Rectangle, Theme, Vector};

use crate::{labels, theme};

pub const NODE_WIDTH: f32 = 220.0;
pub const NODE_HEIGHT: f32 = 88.0;
pub const PORT_RADIUS: f32 = 7.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub status: String,
    pub color: iced::Color,
    pub maestro: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Create,
    Move(String, f32, f32),
    Connect(String, String),
    Select(String),
    RemoveLink(String, String),
    Context(String),
    Focus(String),
    ToggleMaestro(String),
    CloseAgent(String),
    BeginRename(String),
    RenameInput(String),
    RenameCommit,
    DismissContext,
}

pub type Edge = (String, String, bool, usize, Option<String>);

#[derive(Debug, Clone, Copy)]
enum Drag {
    Node(usize, Point),
    Link(usize),
    Pan(Point),
}

#[derive(Debug)]
struct State {
    drag: Option<Drag>,
    scale: f32,
    offset: Vector,
    selected: Option<usize>,
    hovered: Option<(String, String, Point)>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            drag: None,
            scale: 1.0,
            offset: Vector::ZERO,
            selected: None,
            hovered: None,
        }
    }
}

pub fn view<'a>(
    nodes: Vec<Node>,
    positions: Vec<(String, f32, f32)>,
    links: Vec<Edge>,
    context: Option<String>,
    rename: Option<(String, String)>,
) -> Element<'a, Message> {
    if nodes.is_empty() {
        let content = column![
            text(labels::ICON_HUB_OUTLINED)
                .font(theme::FONT_ICON)
                .size(36)
                .color(theme::palette().text3),
            Space::new().height(12),
            text(labels::AGENT_GRAPH_EMPTY_TITLE)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            text(labels::AGENT_GRAPH_EMPTY_HINT)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3)
                .width(Fill)
                .center(),
            Space::new().height(16),
            create_button(),
        ]
        .width(Fill)
        .align_x(Alignment::Center);
        return container(
            container(content.width(360.0))
                .width(360.0)
                .center_x(Fill)
                .center_y(Fill),
        )
        .width(Fill)
        .height(Fill)
        .style(|_: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(theme::palette().bg)),
            ..Default::default()
        })
        .into();
    }
    let surface = canvas(Program {
        nodes: nodes.clone(),
        positions,
        links,
    })
    .width(Fill)
    .height(Fill);
    let add = button(text(labels::AGENT_GRAPH_NEW_AGENT).size(theme::SIZE_LABEL))
        .padding([7, 12])
        .on_press(Message::Create)
        .style(|_: &Theme, status| button::Style {
            background: Some(Background::Color(
                if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    theme::palette().panel3
                } else {
                    theme::palette().panel
                },
            )),
            text_color: theme::palette().text,
            border: Border {
                color: theme::palette().border,
                width: 1.0,
                radius: theme::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        });
    let mut layers = iced::widget::stack![
        surface,
        container(row![add].padding(12))
            .align_right(12)
            .align_top(12)
    ];
    if let Some(id) = context {
        let node = nodes.iter().find(|node| node.id == id);
        let maestro = node.is_some_and(|node| node.maestro);
        let actions = column![
            text(node.map_or_else(|| id.clone(), |node| node.label.clone()))
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text),
            menu_action(labels::AGENT_GRAPH_FOCUS, Message::Focus(id.clone())),
            menu_action(labels::AGENT_GRAPH_RENAME, Message::BeginRename(id.clone())),
            menu_action(
                if maestro {
                    labels::AGENT_GRAPH_REMOVE_MAESTRO
                } else {
                    labels::AGENT_GRAPH_MAKE_MAESTRO
                },
                Message::ToggleMaestro(id.clone())
            ),
            menu_action(
                if id.starts_with("ext:") {
                    labels::AGENT_GRAPH_REMOVE_FROM_CANVAS
                } else {
                    labels::AGENT_GRAPH_CLOSE_TAB
                },
                Message::CloseAgent(id)
            ),
            menu_action(labels::DIALOG_CANCEL, Message::DismissContext),
        ]
        .spacing(5);
        layers = layers.push(
            container(actions)
                .padding(10)
                .style(|_: &Theme| iced::widget::container::Style {
                    background: Some(Background::Color(theme::palette().panel)),
                    border: Border {
                        color: theme::palette().border,
                        width: 1.0,
                        radius: theme::RADIUS_BUTTON.into(),
                    },
                    ..Default::default()
                })
                .align_right(12)
                .align_top(52),
        );
    }
    if let Some((_, name)) = rename {
        let form = column![
            text(labels::AGENT_GRAPH_RENAME_TITLE)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text),
            text_input(labels::AGENT_GRAPH_NAME_PLACEHOLDER, &name)
                .on_input(Message::RenameInput)
                .style(crate::panel_widgets::input_style),
            row![
                menu_action(labels::DIALOG_CANCEL, Message::DismissContext),
                menu_action(labels::AGENT_GRAPH_RENAME, Message::RenameCommit)
            ]
            .spacing(6)
        ]
        .spacing(6);
        layers = layers.push(
            container(form)
                .padding(10)
                .style(|_: &Theme| iced::widget::container::Style {
                    background: Some(Background::Color(theme::palette().panel)),
                    border: Border {
                        color: theme::palette().border,
                        width: 1.0,
                        radius: theme::RADIUS_BUTTON.into(),
                    },
                    ..Default::default()
                })
                .align_right(12)
                .align_top(52),
        );
    }
    layers.into()
}

fn menu_action<'a>(label: &'static str, message: Message) -> Element<'a, Message> {
    button(text(label).size(theme::SIZE_LABEL))
        .width(Fill)
        .on_press(message)
        .into()
}

fn create_button<'a>() -> Element<'a, Message> {
    button(
        row![
            text(labels::ICON_PLUS).font(theme::FONT_ICON),
            text(labels::AGENT_GRAPH_NEW_AGENT)
        ]
        .spacing(8),
    )
    .padding([7, 12])
    .on_press(Message::Create)
    .style(|_: &Theme, status| button::Style {
        background: Some(Background::Color(
            if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                theme::palette().panel3
            } else {
                theme::palette().panel
            },
        )),
        text_color: theme::palette().text,
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: theme::RADIUS_BUTTON.into(),
        },
        ..Default::default()
    })
    .into()
}

struct Program {
    nodes: Vec<Node>,
    positions: Vec<(String, f32, f32)>,
    links: Vec<Edge>,
}

impl Program {
    fn at(&self, id: &str) -> Point {
        self.positions
            .iter()
            .find(|(key, _, _)| key == id)
            .map_or(Point::new(80.0, 80.0), |(_, x, y)| Point::new(*x, *y))
    }
    fn node_at(&self, point: Point, offset: Vector, scale: f32) -> Option<(usize, bool)> {
        self.nodes.iter().enumerate().find_map(|(index, node)| {
            let at = self.at(&node.id);
            let x = at.x * scale + offset.x;
            let y = at.y * scale + offset.y;
            let port = point.x >= x + NODE_WIDTH * scale - PORT_RADIUS * scale
                && point.x <= x + (NODE_WIDTH + PORT_RADIUS) * scale
                && point.y >= y + (NODE_HEIGHT / 2.0 - PORT_RADIUS) * scale
                && point.y <= y + (NODE_HEIGHT / 2.0 + PORT_RADIUS) * scale;
            let card = point.x >= x
                && point.x <= x + NODE_WIDTH * scale
                && point.y >= y
                && point.y <= y + NODE_HEIGHT * scale;
            (port || card).then_some((index, port))
        })
    }
}

fn link_at(
    program: &Program,
    point: Point,
    offset: Vector,
    scale: f32,
) -> Option<(String, String)> {
    program.links.iter().find_map(|(a, b, _, _, _)| {
        let from = program.at(a);
        let to = program.at(b);
        let forward = to.x >= from.x;
        let start = from + Vector::new(if forward { NODE_WIDTH } else { 0.0 }, NODE_HEIGHT / 2.0);
        let end = to + Vector::new(if forward { 0.0 } else { NODE_WIDTH }, NODE_HEIGHT / 2.0);
        let dx = ((end.x - start.x).abs() / 2.0).max(48.0) * if forward { 1.0 } else { -1.0 };
        let near = (0..=32).any(|step| {
            let scene = cubic(
                start,
                start + Vector::new(dx, 0.0),
                end - Vector::new(dx, 0.0),
                end,
                step as f32 / 32.0,
            );
            let screen = Point::new(scene.x * scale + offset.x, scene.y * scale + offset.y);
            let distance = point - screen;
            (distance.x * distance.x + distance.y * distance.y).sqrt() < 8.0
        });
        near.then(|| (a.clone(), b.clone()))
    })
}

impl canvas::Program<Message> for Program {
    type State = State;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<Action<Message>> {
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                let point = cursor.position_in(bounds)?;
                if let Some((index, _)) = self.node_at(point, state.offset, state.scale.max(0.05)) {
                    let node = self.nodes.get(index)?;
                    return Some(Action::publish(Message::Context(node.id.clone())).and_capture());
                }
                if let Some((a, b)) = link_at(self, point, state.offset, state.scale.max(0.05)) {
                    return Some(Action::publish(Message::RemoveLink(a, b)).and_capture());
                }
                Some(Action::publish(Message::Create).and_capture())
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let point = cursor.position_in(bounds)?;
                if let Some((index, port)) =
                    self.node_at(point, state.offset, state.scale.max(0.05))
                {
                    state.selected = Some(index);
                    state.drag = Some(if port {
                        Drag::Link(index)
                    } else {
                        Drag::Node(index, point)
                    });
                    if !port {
                        let node = self.nodes.get(index)?;
                        return Some(
                            Action::publish(Message::Select(node.id.clone())).and_capture(),
                        );
                    }
                } else {
                    state.drag = Some(Drag::Pan(point));
                }
                Some(Action::request_redraw().and_capture())
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                let point = Point::new(position.x - bounds.x, position.y - bounds.y);
                match state.drag {
                    Some(Drag::Node(index, previous)) => {
                        let node = self.nodes.get(index)?;
                        let at = self.at(&node.id);
                        let next = Point::new(
                            (at.x + (point.x - previous.x) / state.scale.max(0.05))
                                .clamp(0.0, 3773.0),
                            (at.y + (point.y - previous.y) / state.scale.max(0.05))
                                .clamp(0.0, 2912.0),
                        );
                        state.drag = Some(Drag::Node(index, point));
                        Some(
                            Action::publish(Message::Move(node.id.clone(), next.x, next.y))
                                .and_capture(),
                        )
                    }
                    Some(Drag::Pan(previous)) => {
                        state.offset += point - previous;
                        state.drag = Some(Drag::Pan(point));
                        Some(Action::request_redraw().and_capture())
                    }
                    Some(Drag::Link(_)) => Some(Action::request_redraw()),
                    None => {
                        state.hovered = link_at(self, point, state.offset, state.scale)
                            .map(|(a, b)| (a, b, point));
                        Some(Action::request_redraw())
                    }
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let Some(Drag::Link(from)) = state.drag.take() else {
                    state.drag = None;
                    return None;
                };
                let point = cursor.position_in(bounds)?;
                let target = self
                    .node_at(point, state.offset, state.scale.max(0.05))
                    .map(|(to, _)| to)?;
                if from == target {
                    return None;
                }
                let from = self.nodes.get(from)?;
                let target = self.nodes.get(target)?;
                Some(
                    Action::publish(Message::Connect(from.id.clone(), target.id.clone()))
                        .and_capture(),
                )
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let point = cursor.position_in(bounds)?;
                let amount = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y * 0.08,
                    mouse::ScrollDelta::Pixels { y, .. } => *y * 0.001,
                };
                let previous = state.scale.max(0.05);
                let next = (previous * (1.0 + amount).clamp(0.5, 1.5)).clamp(0.3, 2.5);
                let relative = point - state.offset;
                let scene = Point::new(relative.x / previous, relative.y / previous);
                state.offset = Vector::new(point.x - scene.x * next, point.y - scene.y * next);
                state.scale = next;
                Some(Action::request_redraw().and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &iced::Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let geometry = canvas::Cache::new().draw(renderer, bounds.size(), |frame| {
            frame.with_save(|frame| {
                frame.translate(state.offset);
                frame.scale(state.scale.max(0.05));
                for (a, b, defined, count, _) in &self.links {
                    let Some(first) = self.nodes.iter().find(|node| node.id == *a) else {
                        continue;
                    };
                    let Some(second) = self.nodes.iter().find(|node| node.id == *b) else {
                        continue;
                    };
                    let p0 = self.at(a);
                    let p3 = self.at(b);
                    let forward = p3.x >= p0.x;
                    let start =
                        p0 + Vector::new(if forward { NODE_WIDTH } else { 0.0 }, NODE_HEIGHT / 2.0);
                    let end =
                        p3 + Vector::new(if forward { 0.0 } else { NODE_WIDTH }, NODE_HEIGHT / 2.0);
                    let dx = ((end.x - start.x).abs() / 2.0).max(48.0)
                        * if forward { 1.0 } else { -1.0 };
                    let path = canvas::Path::new(|builder| {
                        builder.move_to(start);
                        builder.bezier_curve_to(
                            start + Vector::new(dx, 0.0),
                            end - Vector::new(dx, 0.0),
                            end,
                        );
                    });
                    let color = if first.maestro || second.maestro {
                        first.color
                    } else {
                        theme::palette().text3
                    };
                    if *defined {
                        frame.stroke(
                            &path,
                            canvas::Stroke::default()
                                .with_color(color)
                                .with_width(2.0 + (*count).min(10) as f32 * 0.35),
                        );
                    } else {
                        for segment in 0..16 {
                            if segment % 2 == 1 {
                                continue;
                            }
                            let from = cubic(
                                start,
                                start + Vector::new(dx, 0.0),
                                end - Vector::new(dx, 0.0),
                                end,
                                segment as f32 / 16.0,
                            );
                            let to = cubic(
                                start,
                                start + Vector::new(dx, 0.0),
                                end - Vector::new(dx, 0.0),
                                end,
                                (segment + 1) as f32 / 16.0,
                            );
                            let dash = canvas::Path::line(from, to);
                            frame.stroke(
                                &dash,
                                canvas::Stroke::default()
                                    .with_color(color)
                                    .with_width(1.5 + (*count).min(10) as f32 * 0.35),
                            );
                        }
                    }
                }
                for (index, node) in self.nodes.iter().enumerate() {
                    let at = self.at(&node.id);
                    let card = canvas::Path::rounded_rectangle(
                        at,
                        iced::Size::new(NODE_WIDTH, NODE_HEIGHT),
                        8.0.into(),
                    );
                    frame.fill(&card, theme::palette().panel2);
                    frame.stroke(
                        &card,
                        canvas::Stroke::default().with_color(node.color).with_width(
                            if state.selected == Some(index) {
                                2.0
                            } else {
                                1.0
                            },
                        ),
                    );
                    frame.fill_text(canvas::Text {
                        content: node.label.clone(),
                        position: Point::new(at.x + 14.0, at.y + 22.0),
                        max_width: 178.0,
                        color: theme::palette().text,
                        size: 13.0.into(),
                        font: theme::FONT_BODY_SEMIBOLD,
                        ..Default::default()
                    });
                    frame.fill_text(canvas::Text {
                        content: node.detail.clone(),
                        position: Point::new(at.x + 14.0, at.y + 48.0),
                        max_width: 190.0,
                        color: theme::palette().text3,
                        size: 11.0.into(),
                        font: theme::FONT_MONO,
                        ..Default::default()
                    });
                    frame.fill_text(canvas::Text {
                        content: node.status.clone(),
                        position: Point::new(at.x + 14.0, at.y + 74.0),
                        max_width: 190.0,
                        color: node.color,
                        size: 11.0.into(),
                        font: theme::FONT_BODY,
                        ..Default::default()
                    });
                    let port = canvas::Path::circle(
                        Point::new(at.x + NODE_WIDTH, at.y + NODE_HEIGHT / 2.0),
                        PORT_RADIUS,
                    );
                    frame.fill(&port, theme::palette().panel);
                    frame.stroke(
                        &port,
                        canvas::Stroke::default()
                            .with_color(node.color)
                            .with_width(2.0),
                    );
                }
                if let Some((a, b, cursor)) = &state.hovered {
                    if let Some((_, _, defined, count, preview)) = self.links.iter().find(|edge| {
                        (&edge.0 == a && &edge.1 == b) || (&edge.0 == b && &edge.1 == a)
                    }) {
                        let scale = state.scale.max(0.05);
                        let origin = Point::new(
                            (cursor.x - state.offset.x) / scale + 12.0,
                            (cursor.y - state.offset.y) / scale + 12.0,
                        );
                        let mut rows: Vec<(String, iced::Color, usize)> = Vec::new();
                        if *defined {
                            rows.push((
                                labels::AGENT_GRAPH_LINKED.to_owned(),
                                theme::palette().text,
                                1,
                            ));
                        }
                        if *count > 0 {
                            rows.push((
                                labels::agent_graph_message_count(*count),
                                theme::palette().text2,
                                1,
                            ));
                        }
                        if let Some(preview) = preview {
                            rows.push((
                                wrap_tooltip_text(&labels::agent_graph_last_message(preview), 40),
                                theme::palette().text3,
                                3,
                            ));
                        }
                        if *defined {
                            rows.push((
                                labels::AGENT_GRAPH_REMOVE_LINK_HINT.to_owned(),
                                theme::palette().text4,
                                1,
                            ));
                        }
                        if !rows.is_empty() {
                            let width = 280.0 / scale;
                            let height = (16.0
                                + rows.iter().map(|row| row.2).sum::<usize>() as f32 * 14.0)
                                / scale;
                            let tooltip = canvas::Path::rounded_rectangle(
                                origin,
                                iced::Size::new(width, height),
                                6.0.into(),
                            );
                            frame.fill(&tooltip, theme::palette().panel3);
                            frame.stroke(
                                &tooltip,
                                canvas::Stroke::default()
                                    .with_color(theme::palette().border)
                                    .with_width(1.0 / scale),
                            );
                            let mut line = 0;
                            for (content, color, lines) in rows {
                                frame.fill_text(canvas::Text {
                                    content,
                                    position: Point::new(
                                        origin.x + 10.0 / scale,
                                        origin.y + (16.0 + line as f32 * 14.0) / scale,
                                    ),
                                    max_width: width - 20.0 / scale,
                                    color,
                                    size: (11.0 / scale).into(),
                                    font: theme::FONT_BODY,
                                    ..Default::default()
                                });
                                line += lines;
                            }
                        }
                    }
                }
            });
        });
        vec![geometry]
    }

    fn mouse_interaction(
        &self,
        _: &Self::State,
        _: Rectangle,
        _: mouse::Cursor,
    ) -> mouse::Interaction {
        mouse::Interaction::default()
    }
}

fn wrap_tooltip_text(text: &str, width: usize) -> String {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let needed =
            current.chars().count() + usize::from(!current.is_empty()) + word.chars().count();
        if needed > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.len() > 3 {
        lines.truncate(3);
        if let Some(last) = lines.last_mut() {
            last.push('…');
        }
    }
    lines.join("\n")
}

fn cubic(p0: Point, p1: Point, p2: Point, p3: Point, t: f32) -> Point {
    let u = 1.0 - t;
    Point::new(
        p0.x * u.powi(3)
            + 3.0 * p1.x * u.powi(2) * t
            + 3.0 * p2.x * u * t.powi(2)
            + p3.x * t.powi(3),
        p0.y * u.powi(3)
            + 3.0 * p1.y * u.powi(2) * t
            + 3.0 * p2.y * u * t.powi(2)
            + p3.y * t.powi(3),
    )
}
