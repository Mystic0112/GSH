// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! Grade de pixels de um sprite (a classe `Pix` de pix.dart): formas preenchidas,
//! contorno automático e colagem de outra grade. Cor 0 é transparente.
#![allow(dead_code)] // ponytail: usado pelos sprites do Clawd; hats e mini entram depois

pub struct Pix {
    pub w: i64,
    pub h: i64,
    pub p: Vec<u32>,
}

impl Pix {
    pub fn new(w: i64, h: i64) -> Self {
        let len = usize::try_from(w * h).unwrap_or(0);
        Self {
            w,
            h,
            p: vec![0; len],
        }
    }

    /// Escreve o pixel se estiver dentro da grade e `c` não for transparente.
    pub fn set(&mut self, x: i64, y: i64, c: u32) {
        if c != 0 && x >= 0 && y >= 0 && x < self.w && y < self.h {
            if let Some(px) = self.p.get_mut((y * self.w + x) as usize) {
                *px = c;
            }
        }
    }

    /// Cor do pixel, 0 fora da grade.
    pub fn get(&self, x: i64, y: i64) -> u32 {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return 0;
        }
        self.p.get((y * self.w + x) as usize).copied().unwrap_or(0)
    }

    pub fn rect(&mut self, x: i64, y: i64, w: i64, h: i64, c: u32) {
        for j in 0..h {
            for i in 0..w {
                self.set(x + i, y + j, c);
            }
        }
    }

    /// Contorna toda forma preenchida em `c`: os vazios vizinhos (4 direções) de um cheio.
    pub fn outline(&mut self, c: u32) {
        let mut add = Vec::new();
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
                    add.push((x, y));
                }
            }
        }
        for (x, y) in add {
            self.set(x, y, c);
        }
    }

    /// Cola outra grade por cima (pixels transparentes não cobrem).
    pub fn stamp(&mut self, o: &Pix, dx: i64, dy: i64) {
        for y in 0..o.h {
            for x in 0..o.w {
                self.set(x + dx, y + dy, o.get(x, y));
            }
        }
    }

    /// Bytes RGBA para `image::Handle::from_rgba`.
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
}
