use super::{
    agents::{Agent, Pose},
    camera::Camera,
    world::{self, Kind, Mode},
};
use crate::{labels, theme};
use iced::{
    widget::canvas::{self, Text},
    Point, Rectangle, Size,
};

fn position(agent: &Agent, mode: Mode) -> Point {
    let posts: Vec<_> = world::props(mode)
        .into_iter()
        .filter(|p| matches!(p.kind, Kind::Desk | Kind::Plot(_)))
        .collect();
    let Some(post) = posts.get(agent.slot % 9) else {
        return Point::ORIGIN;
    };
    Point::new(
        post.x as f32 - 8.0,
        post.y as f32 + 8.0 + (agent.slot / 9) as f32 * 10.0,
    )
}
pub fn hit(
    agents: &[Agent],
    mode: Mode,
    camera: Camera,
    size: Size,
    point: Point,
) -> Option<&Agent> {
    let world = camera.to_world(point, size);
    agents.iter().rev().find(|agent| {
        let p = position(agent, mode);
        Rectangle {
            x: p.x,
            y: p.y,
            width: 10.0,
            height: 12.0,
        }
        .contains(world)
    })
}
// Sprite simplificado; o clawdHD completo de farm_sprites.dart fica para a fase de fidelidade.
pub fn draw(frame: &mut canvas::Frame, agents: &[Agent], mode: Mode, camera: Camera, size: Size) {
    for agent in agents {
        let p = camera.to_screen(position(agent, mode), size);
        let z = camera.zoom;
        let color = theme::FARM_AGENTS
            .get(agent.slot % theme::FARM_AGENTS.len())
            .copied()
            .unwrap_or(theme::palette().online);
        let mut pixel = |x: f32, y: f32, w: f32, h: f32, color| {
            frame.fill_rectangle(
                Point::new(p.x + x * z, p.y + y * z),
                Size::new(w * z, h * z),
                color,
            )
        };
        pixel(1.0, 2.0, 8.0, 8.0, color);
        pixel(2.0, 10.0, 2.0, 2.0, theme::scene::MONITOR);
        pixel(6.0, 10.0, 2.0, 2.0, theme::scene::MONITOR);
        let pose = agent.input.state.pose();
        if pose == Pose::Sleeping {
            pixel(2.0, 5.0, 2.0, 1.0, theme::scene::MONITOR);
            pixel(6.0, 5.0, 2.0, 1.0, theme::scene::MONITOR);
        } else {
            pixel(3.0, 4.0, 1.0, 2.0, theme::scene::MONITOR);
            pixel(6.0, 4.0, 1.0, 2.0, theme::scene::MONITOR);
        }
        if pose == Pose::Typing {
            pixel(8.0, 7.0, 2.0, 3.0, color);
        }
        if pose == Pose::Happy {
            pixel(3.0, 7.0, 4.0, 1.0, theme::scene::MONITOR);
        }
        if pose == Pose::Alert {
            pixel(7.0, -5.0, 4.0, 6.0, theme::palette().warn);
            frame.fill_text(Text {
                content: labels::FARM_ALERT.into(),
                position: Point::new(p.x + 8.0 * z, p.y - 5.0 * z),
                color: theme::scene::MONITOR,
                size: iced::Pixels(5.0 * z),
                ..Text::default()
            });
        }
    }
}
