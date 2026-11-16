// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! Câmera da cena (farm_camera.dart): (x, y) em pixels de mundo no canto da janela,
//! z = px lógicos por pixel de mundo. Desenha num zoom "encaixado" em pixels inteiros
//! (nítido e sem tremer ao arrastar) e nunca sai do mundo: `clamp_to` segura o pan e
//! `min_z` o zoom, então não sobra margem vazia em volta.
use super::layout::Viewport;
use iced::{Point, Size, Vector};

/// `devicePixelRatio` do Dart. O canvas do iced trabalha em px lógicos e não expõe o
/// fator de escala da janela; com 1 o zoom encaixa em px lógicos inteiros, que também
/// são px físicos inteiros nas escalas inteiras (1x, 2x).
const DPR: f64 = 1.0;

/// O mundo que a câmera enquadra (o `FarmLayout` do Dart): tamanho, zoom que o mostra
/// inteiro e o centro do conteúdo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub w: f64,
    pub h: f64,
    pub z: f64,
    pub cx: f64,
    pub cy: f64,
    /// Com colegas do time na cena: o mundo todo e o zoom que o mostra inteiro
    /// (`team`/`teamZ` do FarmCamera). `None` = só a minha fazenda.
    pub team: Option<TeamBounds>,
}

/// O conjunto das fazendas (a minha e as dos colegas), em pixels de mundo.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TeamBounds {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub z: f64,
}

impl Extent {
    /// Limites da câmera: o conjunto com colegas, senão a minha fazenda (`_bounds`).
    fn bounds(&self) -> (f64, f64, f64, f64) {
        self.team.map_or((0.0, 0.0, self.w, self.h), |t| {
            (t.left, t.top, t.right, t.bottom)
        })
    }

    /// Zoom mínimo: o que enquadra a minha fazenda ou, com colegas, o conjunto (`minZ`).
    pub fn min_z(&self) -> f64 {
        self.team.map_or(self.z, |t| self.z.min(t.z))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// Enquadrada pelo `fit`: ao redimensionar a janela, enquadra de novo.
    pub auto: bool,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 3.0,
            auto: true,
        }
    }
}

/// `snap` do Dart: arredonda o zoom para px de tela inteiros (para baixo com `down`).
pub fn snap(z: f64, down: bool) -> f64 {
    let zr = z * DPR;
    if zr < 2.0 {
        return z;
    }
    let step = if zr >= 4.0 && DPR >= 1.5 { 2.0 } else { 1.0 };
    let n = if down {
        (zr / step + 1e-6).floor()
    } else {
        (zr / step).round()
    };
    step.max(n * step) / DPR
}

/// Zoom desenhado e o deslocamento arredondado em pixels de tela (`rt` do Dart).
struct Rt {
    z: f64,
    zr: f64,
    ox: f64,
    oy: f64,
}

impl Camera {
    fn rt(self) -> Rt {
        let z = snap(self.z, false);
        let zr = z * DPR;
        Rt {
            z,
            zr,
            ox: (-self.x * zr).round(),
            oy: (-self.y * zr).round(),
        }
    }

    /// Zoom em que o mundo é desenhado (px lógicos por px de mundo).
    pub fn scale(self) -> f32 {
        self.rt().z as f32
    }

    pub fn to_world(self, screen: Point) -> Point {
        let r = self.rt();
        Point::new(
            ((f64::from(screen.x) * DPR - r.ox) / r.zr) as f32,
            ((f64::from(screen.y) * DPR - r.oy) / r.zr) as f32,
        )
    }

    pub fn to_screen(self, world: Point) -> Point {
        let r = self.rt();
        Point::new(
            ((r.ox + f64::from(world.x) * r.zr) / DPR) as f32,
            ((r.oy + f64::from(world.y) * r.zr) / DPR) as f32,
        )
    }

    /// Enquadra o conteúdo na área que o HUD deixa livre.
    pub fn fit(&mut self, e: &Extent, v: &Viewport) {
        self.z = e.z;
        self.auto = true;
        self.x = e.cx - (v.left + (v.w - v.left - v.right) / 2.0) / self.z;
        self.y = e.cy - (v.top + (v.h - v.top - v.bottom) / 2.0) / self.z;
        self.clamp_to(e, Size::new(v.w as f32, v.h as f32));
    }

    /// Segura a janela dentro do mundo; mundo menor que a janela fica centrado.
    pub fn clamp_to(&mut self, e: &Extent, view: Size) {
        let dz = snap(self.z, false);
        let (vw, vh) = (f64::from(view.width) / dz, f64::from(view.height) / dz);
        let (left, top, right, bottom) = e.bounds();
        self.x = if right - left <= vw {
            left + (right - left - vw) / 2.0
        } else {
            self.x.min(right - vw).max(left)
        };
        self.y = if bottom - top <= vh {
            top + (bottom - top - vh) / 2.0
        } else {
            self.y.min(bottom - vh).max(top)
        };
    }

    /// Zoom em torno de `at` (px da janela); nunca abaixo do que enquadra o mundo.
    pub fn zoom_at(&mut self, at: Point, factor: f64, e: &Extent, view: Size) {
        let (sx, sy) = (f64::from(at.x), f64::from(at.y));
        let z0 = snap(self.z, false);
        let (wx, wy) = (self.x + sx / z0, self.y + sy / z0);
        self.z = (self.z * factor).min(8.0_f64.max(e.z * 3.0)).max(e.min_z());
        let z1 = snap(self.z, false);
        self.x = wx - sx / z1;
        self.y = wy - sy / z1;
        self.auto = false;
        self.clamp_to(e, view);
    }

    pub fn pan(&mut self, delta: Vector, e: &Extent, view: Size) {
        self.x -= f64::from(delta.x) / self.z;
        self.y -= f64::from(delta.y) / self.z;
        self.auto = false;
        self.clamp_to(e, view);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXTENT: Extent = Extent {
        w: 400.0,
        h: 300.0,
        z: 2.0,
        cx: 200.0,
        cy: 150.0,
        team: None,
    };

    fn viewport(w: f64, h: f64) -> Viewport {
        Viewport {
            w,
            h,
            right: 12.0,
            bottom: 12.0,
            ..Viewport::default()
        }
    }

    #[test]
    fn snap_encaixa_em_pixels_inteiros() {
        assert_eq!(snap(1.7, false), 1.7);
        assert_eq!(snap(3.4, false), 3.0);
        assert_eq!(snap(3.6, false), 4.0);
        assert_eq!(snap(3.9999995, true), 4.0);
        assert_eq!(snap(3.9, true), 3.0);
    }

    #[test]
    fn round_trip_e_zoom_ancorado() {
        let mut cam = Camera::default();
        let view = Size::new(800.0, 600.0);
        cam.fit(&EXTENT, &viewport(800.0, 600.0));
        let world = Point::new(120.0, 80.0);
        let screen = cam.to_screen(world);
        assert_eq!(cam.to_world(screen), world);
        cam.zoom_at(screen, 1.5, &EXTENT, view);
        assert_eq!(cam.to_screen(world), screen);
        assert!(!cam.auto);
    }

    #[test]
    fn nunca_sai_do_mundo() {
        let view = Size::new(800.0, 600.0);
        let mut cam = Camera::default();
        cam.fit(&EXTENT, &viewport(800.0, 600.0));
        // zoom para fora não passa do que enquadra o mundo
        cam.zoom_at(Point::new(400.0, 300.0), 0.01, &EXTENT, view);
        assert_eq!(cam.z, EXTENT.z);
        // com zoom para dentro, o pan pára nas bordas do mundo
        cam.zoom_at(Point::new(400.0, 300.0), 100.0, &EXTENT, view);
        assert_eq!(cam.z, 8.0_f64.max(EXTENT.z * 3.0));
        cam.pan(Vector::new(1.0e6, 1.0e6), &EXTENT, view);
        assert_eq!((cam.x, cam.y), (0.0, 0.0));
        cam.pan(Vector::new(-1.0e6, -1.0e6), &EXTENT, view);
        let dz = snap(cam.z, false);
        assert_eq!(
            (cam.x, cam.y),
            (
                EXTENT.w - f64::from(view.width) / dz,
                EXTENT.h - f64::from(view.height) / dz
            )
        );
    }

    /// Com colegas, o zoom mínimo e os limites passam a ser os do conjunto (`team`/`teamZ`).
    #[test]
    fn com_colegas_os_limites_sao_os_do_conjunto() {
        let team = Extent {
            team: Some(TeamBounds {
                left: 0.0,
                top: 0.0,
                right: 800.0,
                bottom: 300.0,
                z: 1.0,
            }),
            ..EXTENT
        };
        let view = Size::new(800.0, 600.0);
        let mut cam = Camera::default();
        cam.fit(&team, &viewport(800.0, 600.0));
        assert_eq!(team.min_z(), 1.0);
        // dá para afastar até o conjunto caber, o que a minha fazenda sozinha não deixa
        cam.zoom_at(Point::new(400.0, 300.0), 0.01, &team, view);
        assert_eq!(cam.z, 1.0);
        cam.zoom_at(Point::new(400.0, 300.0), 4.0, &team, view);
        cam.pan(Vector::new(-1.0e6, 0.0), &team, view);
        let dz = snap(cam.z, false);
        assert_eq!(cam.x, 800.0 - f64::from(view.width) / dz);
    }

    #[test]
    fn fit_deixa_o_hud_a_esquerda() {
        let mut cam = Camera::default();
        let v = Viewport {
            left: 240.0,
            top: 8.0,
            ..viewport(1000.0, 600.0)
        };
        cam.fit(&EXTENT, &v);
        let centro = cam.to_screen(Point::new(200.0, 150.0));
        // centro da área livre: 240 + (1000 - 240 - 12) / 2 = 614, mas o mundo
        // (800 px a 2x) cabe na janela e o clamp o centra na janela inteira
        assert_eq!(centro.x, 500.0);
    }
}
