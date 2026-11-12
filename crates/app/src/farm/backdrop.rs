// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! Fundo da fazenda no canvas: o layout de farm_world.dart, a pintura HD em imagem
//! e os canteiros, onde os agentes sentam. Coordenadas do canvas: o layout menos
//! a metade do mundo, então a origem fica no centro da fazenda.
use super::crops::{self, CropStage};
use super::layout::{layout_farm, Rect, Viewport};
use super::raster::{Raster, HD};
use super::world::{Kind, Prop};
use super::world_paint::paint_farm;
use iced::widget::image::Handle;
use iced::Size;

/// Canteiros e curral pedidos ao layout: 3x3 como antes, e um curral mínimo.
const PLOTS: i64 = 9;
const YARD: i64 = 5;
/// Semente do fundo: a mesma do `buildWorld` do Flutter.
const SEED: u32 = 7;

pub struct Backdrop {
    /// Janela em que o layout foi feito: quando muda, o fundo é refeito.
    pub size: Size,
    /// Zoom que mostra a fazenda inteira na janela (`layout.z`).
    pub zoom: f32,
    width: i32,
    height: i32,
    /// Linha em que a faixa da frente começa, em coordenadas do layout.
    fg_y: i32,
    /// Canteiros: canto superior esquerdo, já no sistema do canvas.
    plots: Vec<(i32, i32)>,
    bg: Handle,
    fg: Handle,
}

impl Backdrop {
    pub fn build(size: Size) -> Self {
        let view = Viewport {
            w: f64::from(size.width),
            h: f64::from(size.height),
            ..Viewport::default()
        };
        let mut layout = layout_farm(PLOTS, YARD, &view, |z| z, None);
        let (mut bg, fg, fg_y) = paint_farm(&mut layout, SEED);
        // ponytail: cultivos vão no raster porque o canvas desenha imagens por cima das formas
        paint_crops(&mut bg, &layout.plots);
        let (width, height) = (layout.w as i32, layout.h as i32);
        let (ox, oy) = (width / 2, height / 2);
        let plots = layout
            .plots
            .iter()
            .map(|p| (p.left.round() as i32 - ox, p.top.round() as i32 - oy))
            .collect();
        Self {
            size,
            zoom: layout.z as f32,
            width,
            height,
            fg_y: fg_y as i32,
            plots,
            bg: Handle::from_rgba(bg.w as u32, bg.h as u32, bg.to_rgba()),
            fg: Handle::from_rgba(fg.w as u32, fg.h as u32, fg.to_rgba()),
        }
    }

    /// Retângulo do fundo no canvas (x, y, largura, altura), em unidades de mundo.
    pub fn bg_rect(&self) -> (f32, f32, f32, f32) {
        self.rect(0, 0, self.width, self.height)
    }

    /// Retângulo da faixa da frente no canvas: passa por cima dos bichos.
    pub fn fg_rect(&self) -> (f32, f32, f32, f32) {
        self.rect(0, self.fg_y, self.width, self.height - self.fg_y)
    }

    fn rect(&self, x: i32, y: i32, w: i32, h: i32) -> (f32, f32, f32, f32) {
        let (ox, oy) = (self.width / 2, self.height / 2);
        ((x - ox) as f32, (y - oy) as f32, w as f32, h as f32)
    }

    pub fn bg(&self) -> Handle {
        self.bg.clone()
    }

    pub fn fg(&self) -> Handle {
        self.fg.clone()
    }

    /// Os canteiros como props de cultivo, na ordem do layout.
    pub fn posts(&self) -> Vec<Prop> {
        self.plots
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| Prop {
                kind: Kind::Plot(stage(i)),
                x,
                y,
            })
            .collect()
    }
}

fn stage(i: usize) -> CropStage {
    match i % 3 {
        0 => CropStage::Seed,
        1 => CropStage::Sprout,
        _ => CropStage::Grow,
    }
}

/// Cultivo de cada canteiro, na mesma posição do `_plot` do Flutter: 3 x 2 plantas,
/// cada pixel do sprite com 0,5 de mundo (1 pixel HD).
fn paint_crops(g: &mut Raster, plots: &[Rect]) {
    let hd = HD as f64;
    for (i, plot) in plots.iter().enumerate() {
        let pixels = crops::rgba(stage(i), 0);
        for row in 0..2 {
            for k in 0..3 {
                let left = plot.left + 6.0 + k as f64 * 9.0 + row as f64 * 4.0 - 2.5;
                let top = plot.top + (if row == 1 { 15.0 } else { 8.0 }) - 9.0;
                for (j, &[r, gr, b, a]) in pixels.as_chunks::<4>().0.iter().enumerate() {
                    if a != 0 {
                        let x = (left + (j % crops::WIDTH) as f64 * 0.5) * hd;
                        let y = (top + (j / crops::WIDTH) as f64 * 0.5) * hd;
                        let argb = u32::from(a) << 24
                            | u32::from(r) << 16
                            | u32::from(gr) << 8
                            | u32::from(b);
                        g.fill(x, y, 1.0, 1.0, argb);
                    }
                }
            }
        }
    }
}
