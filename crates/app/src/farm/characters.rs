use super::{
    agents::{Agent, Pose},
    camera::Camera,
    clawd::{self, Pose as ClawdPose},
    world::Prop,
};
use crate::{labels, theme};
use iced::advanced::image::{FilterMethod, Image};
use iced::{
    widget::canvas::{self, Text},
    Point, Rectangle, Size,
};

/// Posto de cada agente: um por canteiro ou mesa, repetindo as linhas.
fn position(agent: &Agent, posts: &[Prop]) -> Point {
    let n = posts.len().max(1);
    let Some(post) = posts.get(agent.slot % n) else {
        return Point::ORIGIN;
    };
    Point::new(
        post.x as f32 - 8.0,
        post.y as f32 + 8.0 + (agent.slot / n) as f32 * 10.0,
    )
}
pub fn hit<'a>(
    agents: &'a [Agent],
    posts: &[Prop],
    camera: Camera,
    size: Size,
    point: Point,
) -> Option<&'a Agent> {
    let world = camera.to_world(point, size);
    agents.iter().rev().find(|agent| {
        let p = position(agent, posts);
        Rectangle {
            x: p.x,
            y: p.y,
            width: 10.0,
            height: 12.0,
        }
        .contains(world)
    })
}
/// Pose do Clawd para o estado do agente (`Pose` de agents.rs): dormindo, digitando,
/// alerta ou comemorando. Digitando fica com os braços numa posição só (a animação
/// de digitação vem com o relógio da cena).
fn clawd_pose(pose: Pose) -> ClawdPose {
    let base = ClawdPose {
        look: 0,
        legs: 0,
        blink: false,
        sleep: false,
        arms: 0,
        happy: false,
    };
    match pose {
        Pose::Sleeping => ClawdPose {
            sleep: true,
            ..base
        },
        Pose::Typing => ClawdPose { arms: 2, ..base },
        Pose::Alert => ClawdPose { arms: 1, ..base },
        Pose::Happy => ClawdPose {
            arms: 1,
            happy: true,
            ..base
        },
    }
}

/// O Clawd de cada agente: 16 x 18 de mundo, com os pés em `position`.
pub fn draw(
    frame: &mut canvas::Frame,
    agents: &[Agent],
    posts: &[Prop],
    camera: Camera,
    size: Size,
) {
    for agent in agents {
        let p = camera.to_screen(position(agent, posts), size);
        let z = camera.zoom;
        let pose = agent.input.state.pose();
        frame.draw_image(
            Rectangle {
                x: p.x - 8.0 * z,
                y: p.y - 17.0 * z,
                width: 16.0 * z,
                height: 18.0 * z,
            },
            Image::new(clawd::frame(clawd_pose(pose))).filter_method(FilterMethod::Nearest),
        );
        if pose == Pose::Alert {
            // o balão de alerta fica ao lado do Clawd, para não sumir atrás dele
            frame.fill_rectangle(
                Point::new(p.x + 8.0 * z, p.y - 17.0 * z),
                Size::new(4.0 * z, 6.0 * z),
                theme::palette().warn,
            );
            frame.fill_text(Text {
                content: labels::FARM_ALERT.into(),
                position: Point::new(p.x + 8.0 * z, p.y - 22.0 * z),
                color: theme::scene::MONITOR,
                size: iced::Pixels(5.0 * z),
                ..Text::default()
            });
        }
    }
}
