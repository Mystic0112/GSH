//! Relógio e trajetória dos mensageiros (`farm_courier.dart`).
use gsh_team::KnockKind;
use iced::Point;
use std::time::{Duration, Instant};

const SPEED: f32 = 46.0;
const WALK_MAX: f32 = 6.0;
const DELIVER: f32 = 2.0;
const VANISH: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Walk,
    Deliver,
    Vanish,
    Done,
}

#[derive(Debug, Clone, Copy)]
pub struct Pose {
    pub at: Point,
    pub stage: Stage,
    pub direction: i64,
    pub legs: i64,
    pub hop: f32,
    pub fade: f32,
}

#[derive(Debug, Clone)]
pub struct Courier {
    pub from: Point,
    pub to: Point,
    pub path: Vec<Point>,
    pub bubble_at: Point,
    pub kind: KnockKind,
    started: Instant,
}

impl Courier {
    pub fn new(
        from: Point,
        to: Point,
        path: Vec<Point>,
        bubble_at: Point,
        kind: KnockKind,
    ) -> Self {
        Self {
            from,
            to,
            path,
            bubble_at,
            kind,
            started: Instant::now(),
        }
    }

    pub fn duration(&self) -> Duration {
        let walk = (path_length(&self.path) / SPEED).clamp(0.6, WALK_MAX);
        Duration::from_secs_f32(walk + DELIVER + VANISH)
    }

    pub fn finished(&self, now: Instant) -> bool {
        now.duration_since(self.started) >= self.duration()
    }

    pub fn pose(&self, now: Instant) -> Pose {
        let age = now.duration_since(self.started).as_secs_f32();
        let delta = Vector2 {
            x: self.to.x - self.from.x,
            y: self.to.y - self.from.y,
        };
        let dist = path_length(&self.path);
        let walk = (dist / SPEED).clamp(0.6, WALK_MAX);
        let mut direction = if delta.x >= 0.0 { 1 } else { -1 };
        if age < walk {
            let progress = age.max(0.0) / walk;
            let (at, segment) = if self.path.len() > 2 {
                point_along(&self.path, progress * dist)
            } else {
                let bend = 24.0_f32.min(dist * 0.15);
                let control = Point::new(
                    self.from.x + delta.x / 2.0,
                    self.from.y + delta.y / 2.0 - bend,
                );
                let a = (1.0 - progress).powi(2);
                let b = 2.0 * progress * (1.0 - progress);
                let c = progress.powi(2);
                (
                    Point::new(
                        self.from.x * a + control.x * b + self.to.x * c,
                        self.from.y * a + control.y * b + self.to.y * c,
                    ),
                    delta,
                )
            };
            let step = ((age.max(0.0) / 0.11).floor() as usize) % 4;
            if segment.x.abs() > 0.3 {
                direction = if segment.x >= 0.0 { 1 } else { -1 };
            }
            return Pose {
                at,
                stage: Stage::Walk,
                direction,
                legs: match step {
                    0 => 1,
                    2 => 2,
                    _ => 0,
                },
                hop: if step % 2 == 1 { -0.5 } else { 0.0 },
                fade: (age / 0.35).clamp(0.0, 1.0),
            };
        }
        let after = age - walk - DELIVER;
        Pose {
            at: self.to,
            stage: if after < 0.0 {
                Stage::Deliver
            } else if after < VANISH {
                Stage::Vanish
            } else {
                Stage::Done
            },
            direction,
            legs: 0,
            hop: 0.0,
            fade: if after < 0.0 {
                1.0
            } else {
                (1.0 - after / VANISH).clamp(0.0, 1.0)
            },
        }
    }
}

#[derive(Clone, Copy)]
struct Vector2 {
    x: f32,
    y: f32,
}

fn distance(a: Point, b: Point) -> f32 {
    (b.x - a.x).hypot(b.y - a.y)
}

fn path_length(points: &[Point]) -> f32 {
    points.windows(2).fold(0.0, |length, pair| {
        match (pair.first().copied(), pair.get(1).copied()) {
            (Some(a), Some(b)) => length + distance(a, b),
            _ => length,
        }
    })
}

fn point_along(points: &[Point], mut length: f32) -> (Point, Vector2) {
    for pair in points.windows(2) {
        let (Some(a), Some(b)) = (pair.first().copied(), pair.get(1).copied()) else {
            continue;
        };
        let segment = Vector2 {
            x: b.x - a.x,
            y: b.y - a.y,
        };
        let size = distance(a, b);
        if length <= size {
            let ratio = if size == 0.0 {
                0.0
            } else {
                (length / size).min(1.0)
            };
            return (
                Point::new(a.x + segment.x * ratio, a.y + segment.y * ratio),
                segment,
            );
        }
        length -= size;
    }
    (
        points.last().copied().unwrap_or(Point::ORIGIN),
        Vector2 { x: 0.0, y: 0.0 },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courier_walks_delivers_vanishes_and_finishes() {
        let start = Instant::now();
        let mut courier = Courier::new(
            Point::new(0.0, 0.0),
            Point::new(46.0, 0.0),
            vec![Point::new(0.0, 0.0), Point::new(46.0, 0.0)],
            Point::new(46.0, -25.0),
            KnockKind::Message,
        );
        courier.started = start;
        assert_eq!(
            courier.pose(start + Duration::from_millis(100)).stage,
            Stage::Walk
        );
        assert_eq!(
            courier.pose(start + Duration::from_millis(2_900)).stage,
            Stage::Deliver
        );
        assert_eq!(
            courier.pose(start + Duration::from_millis(3_250)).stage,
            Stage::Vanish
        );
        assert_eq!(
            courier.pose(start + Duration::from_millis(3_600)).stage,
            Stage::Done
        );
        assert!(courier.finished(start + Duration::from_millis(3_600)));
    }
}
