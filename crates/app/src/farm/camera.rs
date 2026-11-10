use iced::{Point, Size, Vector};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub zoom: f32,
    pub offset: Vector,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            zoom: 2.0,
            offset: Vector::new(0.0, 0.0),
        }
    }
}

impl Camera {
    pub fn origin(self, view: Size) -> Point {
        Point::new(
            (view.width / 2.0 + self.offset.x).round(),
            (view.height / 2.0 + self.offset.y).round(),
        )
    }
    pub fn to_screen(self, world: Point, view: Size) -> Point {
        let origin = self.origin(view);
        Point::new(
            origin.x + world.x * self.zoom,
            origin.y + world.y * self.zoom,
        )
    }
    pub fn to_world(self, screen: Point, view: Size) -> Point {
        let origin = self.origin(view);
        Point::new(
            (screen.x - origin.x) / self.zoom,
            (screen.y - origin.y) / self.zoom,
        )
    }
    pub fn zoom_at(&mut self, at: Point, view: Size, step: f32) {
        let world = self.to_world(at, view);
        self.zoom = (self.zoom + step).clamp(1.0, 8.0);
        let after = self.to_screen(world, view);
        self.offset += at - after;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn camera_round_trip_and_zoom_anchor() {
        let mut camera = Camera::default();
        let view = Size::new(1000.0, 700.0);
        let world = Point::new(32.0, -42.0);
        let screen = camera.to_screen(world, view);
        assert_eq!(camera.to_world(screen, view), world);
        camera.zoom_at(screen, view, 1.0);
        assert_eq!(camera.to_screen(world, view), screen);
        camera.zoom_at(screen, view, 100.0);
        assert_eq!(camera.zoom, 8.0);
    }
}
