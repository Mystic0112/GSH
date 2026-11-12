// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
use super::{
    backdrop::Backdrop,
    camera::Camera,
    sprites,
    world::{self, Kind},
};
use crate::theme::scene as c;
use iced::advanced::image::{FilterMethod, Image};
use iced::widget::canvas::{self, Path};
use iced::{Color, Point, Rectangle, Size};

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

/// Desenha o cenário do escritório: piso, mesas, parede e plantas.
pub fn draw_office(frame: &mut canvas::Frame, view: Size, camera: Camera) {
    frame.fill_rectangle(Point::ORIGIN, view, c::FLOOR);
    for prop in world::office_props() {
        let (x, y) = (prop.x as f32, prop.y as f32);
        match prop.kind {
            Kind::Plot(_) => {}
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
        }
    }
}

/// Imagem em coordenadas de mundo, com o zoom da câmera e pixel nítido (sem suavizar).
fn image(
    frame: &mut canvas::Frame,
    view: Size,
    camera: Camera,
    rect_world: (f32, f32, f32, f32),
    handle: iced::widget::image::Handle,
) {
    let (x, y, w, h) = rect_world;
    let p = camera.to_screen(Point::new(x, y), view);
    frame.draw_image(
        Rectangle {
            x: p.x,
            y: p.y,
            width: w * camera.zoom,
            height: h * camera.zoom,
        },
        Image::new(handle).filter_method(FilterMethod::Nearest),
    );
}

/// Fundo da fazenda: a pintura HD inteira, com os cultivos já compostos nela.
pub fn draw_backdrop(frame: &mut canvas::Frame, view: Size, camera: Camera, backdrop: &Backdrop) {
    image(frame, view, camera, backdrop.bg_rect(), backdrop.bg());
}

/// Faixa da frente: as árvores que passam por cima dos bichos.
pub fn draw_front(frame: &mut canvas::Frame, view: Size, camera: Camera, backdrop: &Backdrop) {
    image(frame, view, camera, backdrop.fg_rect(), backdrop.fg());
}
