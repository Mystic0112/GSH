//! Gorila da ilha de agentes (`island_gorilla.dart` e `farm_sprites.dart` do
//! app Flutter): poses, ritmo de animação por pose e desenho pixel-a-pixel em
//! `iced::widget::canvas` usando apenas formas geométricas (sem assets de
//! imagem).

use iced::widget::canvas::{self, Path};
use iced::{Color, Point, Size};

/// Largura do recorte do gorila na ilha (`IslandGorilla.size.width` no Flutter).
pub const GORILLA_WIDTH: f32 = 32.0;

/// Altura do recorte do gorila na ilha (`IslandGorilla.size.height` no Flutter:
/// linhas 11..36 do sprite 32x36).
pub const GORILLA_HEIGHT: f32 = 25.0;

/// Linhas acima da cabeça careca que o sprite 32x36 deixa vazias (`_top = 11.0`).
const SPRITE_TOP_CROP: i32 = 11;
const SPRITE_WIDTH: i32 = 32;
const SPRITE_HEIGHT: i32 = 36;

/// Paleta HD do corpo de argila (`BodyPal.of(clay)` em `farm_sprites.dart`).
struct BodyPal {
    o: u32,
    l: u32,
    h: u32,
    b: u32,
    s: u32,
    d: u32,
    k: u32,
}

const CLAY_PAL: BodyPal = BodyPal {
    o: 0xFF4A1F12,
    l: 0xFFF7C3A3,
    h: 0xFFEC9670,
    b: 0xFFD97757,
    s: 0xFFB95D3F,
    d: 0xFF8A3A21,
    k: 0xFFF3AB98,
};

const EYE_COLOR: u32 = 0xFF231815;
const EYE_SUB_COLOR: u32 = 0xFF4A3A36;
const SHINE_COLOR: u32 = 0xFFFFFFFF;
const HAPPY_MOUTH_COLOR: u32 = 0xFFC8505A;

/// O que o gorila da ilha está fazendo (`GorillaMood` em `island_gorilla.dart`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GorillaMood {
    /// Nenhum agente: dorme (com um "z" parado no alto à direita).
    Sleep,
    /// Ilha recolhida e ninguém trabalhando/esperando: quadro parado.
    Rest,
    /// Ilha aberta e ninguém trabalhando/esperando: respira e pisca.
    Breathe,
    /// Alguém trabalhando: digita.
    Typing,
    /// Alguém espera por você ou pede permissão: acena, com um "!" na cor de aviso.
    Wave,
    /// Um agente acabou de terminar: pula de alegria.
    Cheer,
}

impl GorillaMood {
    /// Intervalo em milissegundos entre quadros da pose (`_schedule` em
    /// `island_gorilla.dart`). `Sleep` e `Rest` ficam parados (`None`).
    pub fn frame_interval_ms(self) -> Option<u64> {
        match self {
            GorillaMood::Sleep | GorillaMood::Rest => None,
            GorillaMood::Breathe => Some(650),
            GorillaMood::Typing => Some(140),
            GorillaMood::Wave => Some(280),
            GorillaMood::Cheer => Some(90),
        }
    }
}

/// Parâmetros de um quadro do gorila calculados a partir de `(mood, tick)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GorillaFrameSpec {
    pub look: i32,
    pub legs: i32,
    pub blink: bool,
    pub sleep: bool,
    pub arms: i32,
    pub happy: bool,
    pub dy: f32,
    pub sleeping_zed: bool,
    pub show_alert: bool,
}

/// Calcula o quadro e o deslocamento vertical `dy` para `(mood, tick)`,
/// espelhando `IslandGorilla.build` em `island_gorilla.dart`.
pub fn frame_spec(mood: GorillaMood, tick: u64) -> GorillaFrameSpec {
    let is_odd = tick % 2 == 1;
    match mood {
        GorillaMood::Sleep => GorillaFrameSpec {
            look: 0,
            legs: 0,
            blink: false,
            sleep: true,
            arms: 0,
            happy: false,
            dy: 0.0,
            sleeping_zed: true,
            show_alert: false,
        },
        GorillaMood::Rest => GorillaFrameSpec {
            look: 0,
            legs: 0,
            blink: false,
            sleep: false,
            arms: 0,
            happy: false,
            dy: 0.0,
            sleeping_zed: false,
            show_alert: false,
        },
        GorillaMood::Breathe => GorillaFrameSpec {
            look: 0,
            legs: 0,
            blink: tick % 7 == 6,
            sleep: false,
            arms: 0,
            happy: false,
            dy: if is_odd { 1.0 } else { 0.0 },
            sleeping_zed: false,
            show_alert: false,
        },
        GorillaMood::Typing => GorillaFrameSpec {
            look: 0,
            legs: 0,
            blink: false,
            sleep: false,
            arms: if is_odd { 2 } else { 3 },
            happy: false,
            dy: 0.0,
            sleeping_zed: false,
            show_alert: false,
        },
        GorillaMood::Wave => GorillaFrameSpec {
            look: 1,
            legs: 0,
            blink: false,
            sleep: false,
            arms: if is_odd { 1 } else { 0 },
            happy: false,
            dy: if tick.is_multiple_of(4) { -1.0 } else { 0.0 },
            sleeping_zed: false,
            show_alert: tick % 4 != 3,
        },
        GorillaMood::Cheer => {
            let phase = ((tick % 100) as f32) * std::f32::consts::PI / 5.0;
            let jump = -(phase.sin().abs() * 5.0).round();
            GorillaFrameSpec {
                look: 0,
                legs: 0,
                blink: false,
                sleep: false,
                arms: if (tick / 2) % 2 == 1 { 1 } else { 0 },
                happy: true,
                dy: jump,
                sleeping_zed: false,
                show_alert: false,
            }
        }
    }
}

/// Grade de pixels ARGB (`Pix` em `pix.dart`), usada para montar o sprite do
/// gorila e o "z" de sono e emitir retângulos contíguos no canvas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pix {
    w: i32,
    h: i32,
    pixels: Vec<u32>,
}

impl Pix {
    pub fn new(w: i32, h: i32) -> Self {
        let count = usize::try_from(w.saturating_mul(h)).unwrap_or(0);
        Self {
            w,
            h,
            pixels: vec![0; count],
        }
    }

    pub fn width(&self) -> i32 {
        self.w
    }

    pub fn height(&self) -> i32 {
        self.h
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        let row = y.checked_mul(self.w)?;
        let idx = row.checked_add(x)?;
        usize::try_from(idx).ok()
    }

    pub fn set(&mut self, x: i32, y: i32, color: u32) {
        if color == 0 {
            return;
        }
        if let Some(idx) = self.index(x, y) {
            if let Some(slot) = self.pixels.get_mut(idx) {
                *slot = color;
            }
        }
    }

    pub fn get(&self, x: i32, y: i32) -> u32 {
        self.index(x, y)
            .and_then(|idx| self.pixels.get(idx).copied())
            .unwrap_or(0)
    }

    pub fn rect(&mut self, x: i32, y: i32, rw: i32, rh: i32, color: u32) {
        for j in 0..rh {
            for i in 0..rw {
                self.set(x + i, y + j, color);
            }
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: u32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let n = dx.max(dy).max(1);
        for i in 0..=n {
            let x = x0 + ((x1 - x0) * i + n / 2) / n;
            let y = y0 + ((y1 - y0) * i + n / 2) / n;
            self.set(x, y, color);
        }
    }

    /// Contorna toda forma preenchida em `color` (os vazios vizinhos
    /// ortogonais de um pixel cheio).
    pub fn outline(&mut self, color: u32) {
        let mut to_fill = Vec::new();
        for y in 0..self.h {
            for x in 0..self.w {
                if self.get(x, y) != 0 {
                    continue;
                }
                if self.get(x - 1, y) != 0
                    || self.get(x + 1, y) != 0
                    || self.get(x, y - 1) != 0
                    || self.get(x, y + 1) != 0
                {
                    to_fill.push((x, y));
                }
            }
        }
        for (x, y) in to_fill {
            self.set(x, y, color);
        }
    }

    /// Quantidade de pixels não transparentes (útil para testes).
    pub fn filled_count(&self) -> usize {
        self.pixels.iter().filter(|&&c| c != 0).count()
    }
}

/// Monta o sprite `clawdHD` (`32 x 36`) exatamente como em `farm_sprites.dart`.
pub fn clawd_hd(spec: GorillaFrameSpec) -> Pix {
    let pal = &CLAY_PAL;
    let mut p = Pix::new(SPRITE_WIDTH, SPRITE_HEIGHT);
    let (x0, x1) = if spec.sleep { (4, 27) } else { (5, 26) };
    let (y0, y1) = if spec.sleep { (18, 33) } else { (13, 30) };

    for y in y0..=y1 {
        for x in x0..=x1 {
            if (x == x0 || x == x1) && (y == y0 || y == y1) {
                continue;
            }
            let mut c = pal.b;
            if y <= y0 + 1 || x <= x0 + 1 {
                c = pal.h;
            }
            if y == y0 && x >= x0 + 2 && x <= x0 + 9 {
                c = pal.l;
            }
            if x == x0 + 1 && y == y0 + 1 {
                c = pal.l;
            }
            if x >= x1 - 1 || y >= y1 - 1 {
                c = pal.s;
            }
            if y == y1 && x > x0 + 1 && x < x1 - 1 {
                c = pal.d;
            }
            p.set(x, y, c);
        }
    }

    let (ay0, ay1) = if spec.sleep {
        (y1 - 5, y1 - 5)
    } else if spec.arms == 1 {
        (15, 15)
    } else if spec.arms == 2 {
        (19, 22)
    } else if spec.arms == 3 {
        (22, 19)
    } else {
        (21, 21)
    };
    let tall = if spec.sleep { 3 } else { 4 };
    for k in 0..tall {
        p.set(x0 - 3, ay0 + k, if k == 0 { pal.l } else { pal.h });
        p.set(x0 - 2, ay0 + k, pal.h);
        p.set(x0 - 1, ay0 + k, pal.b);
        p.set(x1 + 1, ay1 + k, pal.b);
        p.set(x1 + 2, ay1 + k, pal.s);
        p.set(x1 + 3, ay1 + k, if k == tall - 1 { pal.d } else { pal.s });
    }

    if !spec.sleep {
        const LXS: [i32; 4] = [6, 11, 19, 24];
        for (i, &lx) in LXS.iter().enumerate() {
            let is_even = i.is_multiple_of(2);
            let up = spec.legs != 0 && (is_even == (spec.legs == 1));
            let y_max = if up { 32 } else { 33 };
            for y in 31..=y_max {
                p.set(lx, y, if y == 31 { pal.s } else { pal.d });
                p.set(lx + 1, y, pal.d);
            }
        }
    }

    p.outline(pal.o);

    let ey = y0 + 4;
    let sh = spec.look * 2;
    for ex in [10 + sh, 19 + sh] {
        if spec.sleep {
            p.set(ex, ey + 3, EYE_COLOR);
            p.set(ex + 1, ey + 4, EYE_COLOR);
            p.set(ex + 2, ey + 3, EYE_COLOR);
        } else if spec.blink {
            p.rect(ex, ey + 4, 3, 1, EYE_COLOR);
            p.set(ex - 1, ey + 3, pal.s);
            p.set(ex + 3, ey + 3, pal.s);
        } else {
            p.rect(ex, ey, 3, 6, EYE_COLOR);
            p.set(ex, ey, SHINE_COLOR);
            p.set(ex, ey + 1, SHINE_COLOR);
            p.set(ex + 2, ey + 4, EYE_SUB_COLOR);
        }
    }

    p.rect(7 + sh, ey + 7, 3, 1, pal.k);
    p.rect(22 + sh, ey + 7, 3, 1, pal.k);

    if spec.happy && !spec.sleep {
        p.rect(15 + sh, ey + 7, 2, 1, EYE_COLOR);
        p.rect(15 + sh, ey + 8, 2, 1, HAPPY_MOUTH_COLOR);
    }

    p
}

/// Sprite `zed` (`6 x 6`), o "z" de quem dorme (`farm_sprites.dart`).
pub fn zed_sprite() -> Pix {
    let mut p = Pix::new(6, 6);
    p.rect(0, 0, 5, 1, 0xFFF4F1E8);
    p.line(4, 1, 0, 4, 0xFFF4F1E8);
    p.rect(0, 4, 5, 1, 0xFFF4F1E8);
    p.outline(0xFF3C4A6B);
    p
}

fn argb_to_color(argb: u32) -> Color {
    let a = ((argb >> 24) & 0xFF) as f32 / 255.0;
    let r = ((argb >> 16) & 0xFF) as u8;
    let g = ((argb >> 8) & 0xFF) as u8;
    let b = (argb & 0xFF) as u8;
    Color::from_rgba8(r, g, b, a)
}

/// Desenha uma grade `Pix` a partir da linha `y_start` no `canvas::Frame`,
/// agrupando runs horizontais da mesma cor num único `fill_rectangle` (mesmo
/// algoritmo de `Pix.toImage` em `pix.dart`).
fn draw_pix_cropped(frame: &mut canvas::Frame, pix: &Pix, y_start: i32, origin: Point) {
    let h = pix.height();
    let w = pix.width();
    for y in y_start..h {
        let mut x = 0;
        while x < w {
            let c = pix.get(x, y);
            let mut end = x + 1;
            while end < w && pix.get(end, y) == c {
                end += 1;
            }
            if c != 0 {
                let rx = origin.x + (x as f32);
                let ry = origin.y + ((y - y_start) as f32);
                let rw = (end - x) as f32;
                frame.fill_rectangle(Point::new(rx, ry), Size::new(rw, 1.0), argb_to_color(c));
            }
            x = end;
        }
    }
}

/// Desenha o gorila da ilha (`32 x 25`) no `canvas::Frame` na posição `origin`,
/// incluindo o "z" quando `GorillaMood::Sleep` e o selo "!" piscando quando
/// `GorillaMood::Wave` (`_GorillaPainter.paint` em `island_gorilla.dart`).
pub fn draw_gorilla(
    frame: &mut canvas::Frame,
    origin: Point,
    mood: GorillaMood,
    tick: u64,
    warn_color: Color,
    bg_color: Color,
) {
    let spec = frame_spec(mood, tick);
    let body = clawd_hd(spec);
    let draw_origin = Point::new(origin.x, origin.y + spec.dy);
    draw_pix_cropped(frame, &body, SPRITE_TOP_CROP, draw_origin);

    if spec.sleeping_zed {
        let zed = zed_sprite();
        let zed_origin = Point::new(origin.x + GORILLA_WIDTH - 5.0, origin.y - 3.0 + spec.dy);
        draw_pix_cropped(frame, &zed, 0, zed_origin);
    }

    if spec.show_alert {
        let ax = origin.x + GORILLA_WIDTH - 2.0;
        let ay = origin.y + 1.0 + spec.dy;
        let badge = Path::circle(Point::new(ax, ay), 5.0);
        frame.fill(&badge, warn_color);
        frame.fill_rectangle(
            Point::new(ax - 0.75, ay - 3.5),
            Size::new(1.5, 4.5),
            bg_color,
        );
        frame.fill_rectangle(
            Point::new(ax - 0.75, ay + 2.0),
            Size::new(1.5, 1.5),
            bg_color,
        );
    }
}
