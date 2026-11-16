use super::lots::hash_str;
use super::{
    agents::{Agent, Pose},
    camera::Camera,
    clawd::{self, Pose as ClawdPose},
    world::Prop,
};
use crate::{labels, theme};
use gsh_team::{Activity, FarmLoop};
use iced::advanced::image::{FilterMethod, Image};
use iced::{
    widget::canvas::{self, Text},
    Point, Rectangle, Size,
};

/// Posto de cada agente: um por canteiro ou mesa, repetindo as linhas.
pub(super) fn position(agent: &Agent, posts: &[Prop]) -> Point {
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
    point: Point,
) -> Option<&'a Agent> {
    let world = camera.to_world(point);
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
pub fn draw(frame: &mut canvas::Frame, agents: &[Agent], posts: &[Prop], camera: Camera) {
    for agent in agents {
        let rect = sprite_rect(agent, posts, camera);
        let z = camera.scale();
        let pose = agent.input.state.pose();
        frame.draw_image(
            rect,
            Image::new(clawd::frame(clawd_pose(pose))).filter_method(FilterMethod::Nearest),
        );
        let p = Point::new(rect.x + 8.0 * z, rect.y + 17.0 * z);
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

/// Retângulo em tela passado ao painter para desenhar o sprite de um agente.
pub fn sprite_rect(agent: &Agent, posts: &[Prop], camera: Camera) -> Rectangle {
    let p = camera.to_screen(position(agent, posts));
    let z = camera.scale();
    Rectangle {
        x: p.x - 8.0 * z,
        y: p.y - 17.0 * z,
        width: 16.0 * z,
        height: 18.0 * z,
    }
}

/// Minis dos subagentes remotos, posicionados ao redor do canteiro do pai como `subSpots`.
pub fn draw_minis(
    frame: &mut canvas::Frame,
    subs: &[FarmLoop],
    agents: &[Agent],
    posts: &[Prop],
    camera: Camera,
) {
    const SPOTS: [(f32, f32); 8] = [
        (5.0, -2.0),
        (25.0, -2.0),
        (5.0, 25.0),
        (25.0, 25.0),
        (15.0, -2.0),
        (15.0, 25.0),
        (37.0, 5.0),
        (37.0, 17.0),
    ];
    const COLORS: [u32; 8] = [
        0xff3b82f6, 0xff10b981, 0xfff59e0b, 0xfff43f5e, 0xff8b5cf6, 0xff06b6d4, 0xfff97316,
        0xffec4899,
    ];
    let z = camera.scale();
    static EPOCH: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    let time = EPOCH
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_secs_f32();
    for parent in agents {
        let mut children: Vec<&FarmLoop> = subs
            .iter()
            .filter(|sub| sub.parent_id.as_deref() == Some(&parent.input.pane_id))
            .collect();
        children.sort_by_key(|sub| match sub.activity {
            Activity::Waiting => 1,
            Activity::Idle | Activity::Sleeping => 2,
            Activity::Done => 3,
            _ => 0,
        });
        let Some(post) = posts.get(parent.slot % posts.len().max(1)) else {
            continue;
        };
        for (i, sub) in children.into_iter().take(SPOTS.len()).enumerate() {
            let Some((dx, dy)) = SPOTS.get(i).copied() else {
                continue;
            };
            let color = COLORS
                .get(
                    (hash_str(sub.agent_type.as_deref().unwrap_or(&sub.id)) as usize)
                        % COLORS.len(),
                )
                .copied()
                .unwrap_or(0xff3b82f6);
            let seed = hash_str(&sub.id) % 31;
            let phase = ((time / 0.11).floor() as u64 % 4) as usize;
            let legs = if sub.activity == Activity::Thinking
                || matches!(
                    sub.activity,
                    Activity::Reading
                        | Activity::Editing
                        | Activity::Running
                        | Activity::Searching
                        | Activity::Delegating
                        | Activity::Tooling
                ) {
                match phase {
                    0 => 1,
                    2 => 2,
                    _ => 0,
                }
            } else {
                0
            };
            let blink = matches!(sub.activity, Activity::Idle | Activity::Sleeping)
                || (time + seed as f32 * 0.07) % 3.2 > 3.05;
            let arms = i64::from(
                matches!(
                    sub.activity,
                    Activity::Thinking
                        | Activity::Reading
                        | Activity::Editing
                        | Activity::Running
                        | Activity::Searching
                        | Activity::Delegating
                        | Activity::Tooling
                ) && (time * 6.0 + seed as f32).floor() as i64 % 2 != 0,
            );
            let color = if sub.activity == Activity::Done {
                0xff9aa3b2
            } else {
                color
            };
            let image = iced::advanced::image::Image::new(super::clawd::mini_frame(
                color, legs, arms, blink,
            ))
            .filter_method(iced::advanced::image::FilterMethod::Nearest);
            let feet = camera.to_screen(Point::new(post.x as f32 + dx, post.y as f32 + dy));
            frame.draw_image(
                Rectangle {
                    x: feet.x - 5.0 * z,
                    y: feet.y - 8.0 * z,
                    width: 10.0 * z,
                    height: 8.0 * z,
                },
                image,
            );
        }
    }
}
