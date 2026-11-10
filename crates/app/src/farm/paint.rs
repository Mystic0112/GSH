use super::{
    camera::Camera,
    sprites,
    world::{self, Kind, Mode},
};
use crate::theme::scene as c;
use iced::widget::canvas::{self, Path};
use iced::{Color, Point, Size};

#[allow(clippy::too_many_arguments)]
fn rect(
    frame: &mut canvas::Frame,
    camera: Camera,
    view: Size,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
) {
    let p = camera.to_screen(Point::new(x, y), view);
    frame.fill(
        &Path::rectangle(p, Size::new(w * camera.zoom, h * camera.zoom)),
        color,
    );
}
fn sprite(frame: &mut canvas::Frame, cam: Camera, view: Size, x: f32, y: f32, pixels: &[&str]) {
    for (dy, row) in pixels.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            let color = match ch {
                'b' => c::MONITOR,
                's' => c::SCREEN,
                'r' => c::REFLECTION,
                'a' => c::STANDBY,
                'm' => c::MONITOR_STAND,
                'n' => c::METAL,
                'g' => c::LEAF_DARK,
                'l' => c::LEAF,
                'h' => c::LEAF_LIGHT,
                'd' => c::SOIL_DARK,
                'o' | 'f' => c::CROP,
                't' => c::CROP_LIGHT,
                _ => continue,
            };
            rect(
                frame,
                cam,
                view,
                x + dx as f32,
                y + dy as f32,
                1.0,
                1.0,
                color,
            );
        }
    }
}
pub fn draw(frame: &mut canvas::Frame, view: Size, camera: Camera, mode: Mode) {
    frame.fill_rectangle(
        Point::ORIGIN,
        view,
        if mode == Mode::Farm {
            c::GRASS
        } else {
            c::FLOOR
        },
    );
    if mode == Mode::Farm {
        for y in -12i32..12 {
            for x in -19..19 {
                if (x * 17 + y * 31).rem_euclid(7) == 0 {
                    rect(
                        frame,
                        camera,
                        view,
                        (x * 10) as f32,
                        (y * 10) as f32,
                        2.0,
                        1.0,
                        c::GRASS_DARK,
                    );
                }
            }
        }
    }
    for prop in world::props(mode) {
        let (x, y) = (prop.x as f32, prop.y as f32);
        match prop.kind {
            Kind::Plot(stage) => {
                rect(frame, camera, view, x, y, 32.0, 18.0, c::SOIL_DARK);
                rect(frame, camera, view, x + 2.0, y + 2.0, 28.0, 14.0, c::SOIL);
                for dx in [2.0, 10.0, 18.0] {
                    let crop = match stage {
                        0 => sprites::SEED,
                        1 => sprites::SPROUT,
                        _ => sprites::GROW,
                    };
                    sprite(frame, camera, view, x + dx, y - 2.0, crop);
                }
            }
            Kind::Desk => {
                rect(frame, camera, view, x - 8.0, y - 7.0, 48.0, 34.0, c::CARPET);
                rect(frame, camera, view, x, y, 32.0, 16.0, c::WALL);
                rect(frame, camera, view, x + 1.0, y + 1.0, 30.0, 14.0, c::DESK);
                for dx in [2.0, 17.0] {
                    sprite(frame, camera, view, x + dx, y + 2.0, sprites::MONITOR);
                }
                for dx in [4.0, 22.0] {
                    for dy in [-5.0, 19.0] {
                        rect(frame, camera, view, x + dx, y + dy, 7.0, 5.0, c::CHAIR);
                    }
                }
            }
            Kind::Plant => {
                rect(frame, camera, view, x, y, 8.0, 7.0, c::WOOD);
                rect(
                    frame,
                    camera,
                    view,
                    x - 3.0,
                    y - 9.0,
                    14.0,
                    10.0,
                    c::LEAF_DARK,
                );
                rect(frame, camera, view, x, y - 12.0, 8.0, 8.0, c::LEAF);
            }
            Kind::Wall => {
                rect(frame, camera, view, x, y, 330.0, 4.0, c::WALL);
                rect(frame, camera, view, x, y, 4.0, 200.0, c::WALL);
                rect(frame, camera, view, x + 326.0, y, 4.0, 200.0, c::WALL);
            }
            Kind::Path => rect(frame, camera, view, x, y, 140.0, 8.0, c::WOOD),
            Kind::Barn => {
                rect(frame, camera, view, x, y, 46.0, 44.0, c::WALL);
                rect(frame, camera, view, x + 2.0, y + 8.0, 42.0, 34.0, c::BARN);
                rect(frame, camera, view, x + 17.0, y + 24.0, 12.0, 18.0, c::DESK);
            }
            Kind::Pond => {
                rect(frame, camera, view, x, y, 48.0, 20.0, c::SOIL);
                rect(frame, camera, view, x + 3.0, y + 2.0, 42.0, 16.0, c::WATER);
            }
        }
    }
}
