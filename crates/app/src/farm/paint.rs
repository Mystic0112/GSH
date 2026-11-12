// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
use super::{
    camera::Camera,
    crops, sprites,
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

/// Retângulos do canteiro em pixels HD, conforme `_plot` em farm_world_props.dart.
fn plot_pixels() -> Vec<(f32, f32, f32, f32, Color)> {
    let mut pixels = vec![
        (-2.0, 0.0, 64.0, 34.0, c::SOIL_DARK),
        (0.0, -2.0, 60.0, 38.0, c::SOIL_DARK),
        (0.0, 0.0, 60.0, 34.0, c::PLOT_EDGE),
        (0.0, 0.0, 60.0, 32.0, c::SOIL),
    ];
    for y in (3..31).step_by(6) {
        pixels.extend([
            (2.0, y as f32, 56.0, 2.0, c::SOIL3),
            (2.0, (y + 2) as f32, 56.0, 2.0, c::SOIL2),
            (3.0, y as f32, 52.0, 1.0, c::PLOT_RIDGE),
        ]);
    }
    for i in 0..10 {
        pixels.push((
            (3 + (i * 17) % 54) as f32,
            (2 + (i * 11) % 28) as f32,
            1.0,
            1.0,
            if i % 2 == 1 {
                c::SOIL_DARK
            } else {
                c::PLOT_FLECK
            },
        ));
    }
    pixels
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
                for (dx, dy, w, h, color) in plot_pixels() {
                    rect(
                        frame,
                        camera,
                        view,
                        x + dx / 2.0,
                        y + dy / 2.0,
                        w / 2.0,
                        h / 2.0,
                        color,
                    );
                }
                let pixels = crops::rgba(stage, 0);
                for row in 0..2 {
                    for k in 0..3 {
                        let left = x + 6.0 + k as f32 * 9.0 + row as f32 * 4.0 - 2.5;
                        let top = y + if row == 1 { 15.0 } else { 8.0 } - 9.0;
                        for (i, &[r, g, b, a]) in pixels.as_chunks::<4>().0.iter().enumerate() {
                            if a != 0 {
                                rect(
                                    frame,
                                    camera,
                                    view,
                                    left + (i % crops::WIDTH) as f32 * 0.5,
                                    top + (i / crops::WIDTH) as f32 * 0.5,
                                    0.5,
                                    0.5,
                                    Color::from_rgba8(r, g, b, f32::from(a) / 255.0),
                                );
                            }
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plot_background_matches_hd_geometry() {
        let pixels = plot_pixels();
        assert_eq!(pixels.len(), 29);
        assert_eq!(pixels.first(), Some(&(-2.0, 0.0, 64.0, 34.0, c::SOIL_DARK)));
        assert_eq!(pixels.last(), Some(&(48.0, 17.0, 1.0, 1.0, c::SOIL_DARK)));
    }
}
