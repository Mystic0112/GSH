// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! Primitivas de pixel do mundo (pix.dart): RNG mulberry32 e a tela `Raster` em ARGB.
#![allow(dead_code)] // ponytail: o fundo ainda não está ligado ao cenário (falta layout e props)

/// Escala do fundo HD (`hd` em farm_sprites.dart).
pub const HD: i64 = 2;

/// RNG semeado do clodfarm (mulberry32), em uint32: mesma semente, mesmo mundo.
pub fn mulberry32(seed: u32) -> impl FnMut() -> f64 {
    let mut s = seed;
    move || {
        s = s.wrapping_add(0x6d2b_79f5);
        let mut t = (s ^ (s >> 15)).wrapping_mul(1 | s);
        t = (t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t))) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }
}

/// Mistura `src` com alpha `a` sobre `dst` (ARGB), como `_blend` do pix.dart.
fn blend(dst: u32, src: u32, a: u32) -> u32 {
    let m = |shift: u32| (((dst >> shift) & 0xff) * (255 - a) + ((src >> shift) & 0xff) * a) / 255;
    let oa = a + (dst >> 24) * (255 - a) / 255;
    (oa << 24) | (m(16) << 16) | (m(8) << 8) | m(0)
}

/// A tela do mundo (o `<canvas>` do fundo no clodfarm): `fill` com alpha.
pub struct Raster {
    pub w: i64,
    pub h: i64,
    pub p: Vec<u32>,
}

impl Raster {
    pub fn new(w: i64, h: i64) -> Self {
        let len = usize::try_from(w * h).unwrap_or(0);
        Self {
            w,
            h,
            p: vec![0; len],
        }
    }

    pub fn fill(&mut self, x: f64, y: f64, rw: f64, rh: f64, c: u32) {
        let x0 = (x.round() as i64).max(0);
        let y0 = (y.round() as i64).max(0);
        let x1 = ((x + rw).round() as i64).min(self.w);
        let y1 = ((y + rh).round() as i64).min(self.h);
        let a = c >> 24;
        if a == 0 {
            return;
        }
        for yy in y0..y1 {
            for xx in x0..x1 {
                if let Some(px) = self.p.get_mut((yy * self.w + xx) as usize) {
                    *px = if a == 0xff { c } else { blend(*px, c, a) };
                }
            }
        }
    }

    /// Bytes RGBA (não pré-multiplicados) para `image::Handle::from_rgba`.
    pub fn to_rgba(&self) -> Vec<u8> {
        self.p
            .iter()
            .flat_map(|&c| {
                [
                    ((c >> 16) & 0xff) as u8,
                    ((c >> 8) & 0xff) as u8,
                    (c & 0xff) as u8,
                    (c >> 24) as u8,
                ]
            })
            .collect()
    }

    /// O `pxCircle` do clodfarm: um disco em fileiras de 1px.
    pub fn circle(&mut self, cx: f64, cy: f64, r: f64, c: u32) {
        let mut y = -r;
        while y <= r {
            let ww = (r * r - y * y + r * 0.8).max(0.0).sqrt().floor();
            self.fill((cx - ww).round(), (cy + y).round(), ww * 2.0 + 1.0, 1.0, c);
            y += 1.0;
        }
    }

    pub fn ellipse(&mut self, cx: f64, cy: f64, rx: f64, ry: f64, c: u32) {
        let mut y = -ry;
        while y <= ry {
            let ww = (rx * (1.0 - y * y / (ry * ry + 0.6)).max(0.0).sqrt()).floor();
            self.fill((cx - ww).round(), (cy + y).round(), ww * 2.0 + 1.0, 1.0, c);
            y += 1.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_sai_na_ordem_do_image_handle() {
        let mut r = Raster::new(2, 1);
        r.fill(0.0, 0.0, 1.0, 1.0, 0xff_11_22_33);
        assert_eq!(r.to_rgba(), vec![0x11, 0x22, 0x33, 0xff, 0, 0, 0, 0]);
    }
}
