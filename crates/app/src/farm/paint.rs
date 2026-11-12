// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
use super::{
    backdrop::{Backdrop, ScreenState},
    camera::Camera,
};
use iced::advanced::image::{FilterMethod, Image};
use iced::widget::canvas;
use iced::{Point, Rectangle, Size};

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

/// Fundo do cenário: a pintura HD inteira, com os cultivos já compostos nela.
pub fn draw_backdrop(frame: &mut canvas::Frame, view: Size, camera: Camera, backdrop: &Backdrop) {
    image(frame, view, camera, backdrop.bg_rect(), backdrop.bg());
}

/// Faixa da frente da fazenda: as árvores que passam por cima dos bichos.
pub fn draw_front(frame: &mut canvas::Frame, view: Size, camera: Camera, backdrop: &Backdrop) {
    if let Some(fg) = backdrop.fg() {
        image(frame, view, camera, backdrop.fg_rect(), fg);
    }
}

/// Telas das ilhas do escritório: seis por ilha, 7 x 5 de mundo cada (`_island`).
/// `states[i]` é o estado do dono da ilha `i`; a tela acende só com ele trabalhando.
pub fn draw_screens(
    frame: &mut canvas::Frame,
    view: Size,
    camera: Camera,
    backdrop: &Backdrop,
    states: &[ScreenState],
) {
    for i in 0..states.len() {
        let Some((px, py)) = backdrop.plot_origin(i) else {
            continue;
        };
        let state = states.get(i).copied().unwrap_or(ScreenState::Off);
        for row in 0..2 {
            for k in 0..3 {
                let Some(handle) = backdrop.screen(i, k + row * 2, state) else {
                    continue;
                };
                let x = px + 1.5 + k as f32 * 10.0;
                let y = py - 1.5 + row as f32 * 8.0;
                image(frame, view, camera, (x, y, 7.0, 5.0), handle);
            }
        }
    }
}
