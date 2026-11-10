//! Rasterização de `farm_sprites_crops.dart` e `Pix` do Flutter.
use crate::theme::scene as c;

pub const WIDTH: usize = 10;
pub const HEIGHT: usize = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CropStage {
    Seed,
    Sprout,
    Grow,
    Ripe,
    Wilt,
}

struct Pix([u8; WIDTH * HEIGHT * 4]);
impl Pix {
    fn set(&mut self, x: i32, y: i32, argb: u32) {
        if (0..WIDTH as i32).contains(&x) && (0..HEIGHT as i32).contains(&y) {
            let index = (y as usize * WIDTH + x as usize) * 4;
            let [a, r, g, b] = argb.to_be_bytes();
            if let Some(pixel) = self.0.get_mut(index..index + 4) {
                pixel.copy_from_slice(&[r, g, b, a]);
            }
        }
    }
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u32) {
        for j in 0..h {
            for i in 0..w {
                self.set(x + i, y + j, color);
            }
        }
    }
    fn line(&mut self, x: i32, y: i32, x1: i32, y1: i32, color: u32) {
        let n = (x1 - x).abs().max((y1 - y).abs()).max(1);
        for i in 0..=n {
            let t = i as f32 / n as f32;
            self.set(
                (x as f32 + (x1 - x) as f32 * t).round() as i32,
                (y as f32 + (y1 - y) as f32 * t).round() as i32,
                color,
            );
        }
    }
}

/// Uma imagem RGBA 10×18; o Dart usa os quadros `sw` -1, 0 e 1.
pub fn rgba(stage: CropStage, sw: i32) -> [u8; WIDTH * HEIGHT * 4] {
    let mut p = Pix([0; WIDTH * HEIGHT * 4]);
    match stage {
        CropStage::Seed => {
            p.rect(3, 15, 4, 1, c::CROP_SEED_SOIL);
            p.set(4, 14, c::CROP_GREEN);
            p.set(5, 13, c::CROP_GREEN);
            p.set(3, 13, c::CROP_GREEN_DARK);
            p.set(6, 14, c::CROP_GREEN_LIGHT);
        }
        CropStage::Wilt => {
            p.line(4, 16, 4, 10, c::CROP_WILT_STEM);
            p.line(4, 10, 7, 11, c::CROP_WILT_STEM);
            p.set(7, 12, c::CROP_WILT_TIP);
            p.line(3, 13, 1, 14, c::CROP_WILT_LEAF);
            p.set(8, 12, c::CROP_WILT_TIP);
        }
        _ => {
            let top = if stage == CropStage::Sprout { 10 } else { 6 };
            for y in top..=16 {
                p.set(4 + if y < top + 3 { sw } else { 0 }, y, c::CROP_GREEN_DARK);
            }
            p.rect(1 + sw, top + 3, 3, 1, c::CROP_GREEN);
            p.set(1 + sw, top + 2, c::CROP_GREEN_LIGHT);
            p.rect(5 + sw, top + 5, 3, 1, c::CROP_GREEN);
            p.set(7 + sw, top + 4, c::CROP_GREEN_LIGHT);
            if stage != CropStage::Sprout {
                p.rect(1, top + 8, 3, 1, c::CROP_GREEN);
                p.rect(5, top + 9, 3, 1, c::CROP_GREEN_DARK);
            }
            if stage == CropStage::Grow {
                p.rect(3 + sw, top - 2, 3, 2, c::CROP_FRUIT);
                p.set(4 + sw, top - 2, c::CROP_FRUIT_LIGHT);
            }
            if stage == CropStage::Ripe {
                const SPARK: [&str; 9] = [
                    "....o....",
                    ".o..o..o.",
                    "..o.o.o..",
                    "...ooo...",
                    "oooowoooo",
                    "...ooo...",
                    "..o.o.o..",
                    ".o..o..o.",
                    "....o....",
                ];
                for (y, row) in SPARK.iter().enumerate() {
                    for (x, ch) in row.bytes().enumerate() {
                        let color = match ch {
                            b'w' => c::CROP_SPARK_CENTER,
                            b'o' if x == 4 || y == 4 => c::CROP_SPARK_CROSS,
                            b'o' => c::CROP_FRUIT,
                            _ => continue,
                        };
                        // floor(x + sw - 0.5), como em Pix.set.
                        p.set(x as i32 + sw - 1, y as i32 - 1, color);
                    }
                }
            }
        }
    }
    p.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dart_dimensions_and_pixel_counts_for_all_frames() {
        // Contagens independentes: execução dos comandos Dart sobre uma grade de coordenadas.
        for (stage, counts) in [
            (CropStage::Seed, [8, 8, 8]),
            (CropStage::Sprout, [14, 15, 14]),
            (CropStage::Grow, [30, 31, 30]),
            (CropStage::Ripe, [48, 52, 52]),
            (CropStage::Wilt, [15, 15, 15]),
        ] {
            for (sw, count) in (-1..=1).zip(counts) {
                let pixels = rgba(stage, sw);
                assert_eq!(pixels.len(), 10 * 18 * 4);
                assert_eq!(
                    pixels
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .filter(|p| p[3] != 0)
                        .count(),
                    count,
                    "{stage:?} {sw}"
                );
            }
        }
    }
}
