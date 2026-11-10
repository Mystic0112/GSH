//! Rasterização de `_bezel`, `_monitor` e `printerHD` de office_sprites.dart.
use crate::farm::crops::CropStage;
use crate::theme::scene::office as c;

pub const MONITOR_WIDTH: usize = 14;
pub const MONITOR_HEIGHT: usize = 10;
pub const PRINTER_WIDTH: usize = 28;
pub const PRINTER_HEIGHT: usize = 24;

struct Pix<const N: usize, const W: usize>([u8; N]);
impl<const N: usize, const W: usize> Pix<N, W> {
    fn rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let [a, r, g, b] = color.to_be_bytes();
        for yy in y..y + h {
            for xx in x..x + w {
                if xx < W {
                    let i = (yy * W + xx) * 4;
                    if let Some(pixel) = self.0.get_mut(i..i + 4) {
                        pixel.copy_from_slice(&[r, g, b, a]);
                    }
                }
            }
        }
    }
}

type Monitor = Pix<{ MONITOR_WIDTH * MONITOR_HEIGHT * 4 }, MONITOR_WIDTH>;
type Printer = Pix<{ PRINTER_WIDTH * PRINTER_HEIGHT * 4 }, PRINTER_WIDTH>;

fn bezel() -> Monitor {
    let mut p: Monitor = Pix([0; MONITOR_WIDTH * MONITOR_HEIGHT * 4]);
    p.rect(0, 0, 14, 8, c::BEZEL);
    p.rect(6, 8, 2, 1, c::STAND);
    p.rect(4, 9, 6, 1, c::FOOT);
    p
}

/// `frame` deve ser 0, 1 ou 2, como no Dart.
pub fn monitor(stage: CropStage, frame: usize) -> [u8; MONITOR_WIDTH * MONITOR_HEIGHT * 4] {
    let frame = frame % 3;
    let mut p = bezel();
    p.rect(1, 1, 12, 6, c::SCREEN);
    let lines = match stage {
        CropStage::Seed | CropStage::Wilt => 1,
        CropStage::Sprout => 2,
        CropStage::Grow => 4,
        CropStage::Ripe => 5,
    };
    if frame != 1 {
        p.rect(2, 1 + lines, 1, 1, c::CURSOR);
    }
    for i in 0..lines {
        let k = i + frame;
        let indent = if (k * 3).is_multiple_of(4) {
            0
        } else {
            1 + k % 2
        };
        let len = 3 + (k * 5) % 6;
        if let Some(&color) = c::INK.get(k % 5) {
            p.rect(2 + indent, 1 + i, len.min(10 - indent), 1, color);
        }
    }
    if stage == CropStage::Ripe {
        p.rect(10, 4, 3, 1, c::CLAY);
        p.rect(11, 3, 1, 3, c::CLAY);
        p.rect(11, 4, 1, 1, c::CREAM);
    }
    p.0
}

/// `frame` deve ser 0, 1 ou 2, como no Dart.
pub fn printer(frame: usize) -> [u8; PRINTER_WIDTH * PRINTER_HEIGHT * 4] {
    let frame = frame % 3;
    let mut p: Printer = Pix([0; PRINTER_WIDTH * PRINTER_HEIGHT * 4]);
    p.rect(3, 21, 24, 3, c::SHADOW);
    p.rect(1, 8, 26, 14, c::PRINTER_INK);
    p.rect(2, 9, 24, 12, c::BODY);
    p.rect(2, 9, 24, 2, c::BODY_LIGHT);
    p.rect(2, 17, 24, 4, c::BODY_DARK);
    p.rect(4, 3, 20, 6, c::PRINTER_INK);
    p.rect(5, 4, 18, 4, c::TRAY);
    p.rect(7, 1, 14, 3, c::PRINTER_INK);
    p.rect(8, 2, 12, 2, c::WHITE);
    p.rect(18, 11, 6, 3, c::BEZEL);
    p.rect(
        19,
        12,
        1,
        1,
        if frame == 2 { c::LED_OFF } else { c::LED_ON },
    );
    p.rect(21, 12, 1, 1, if frame == 1 { c::INK[2] } else { c::STAND });
    p.rect(4, 15, 12, 2, c::BEZEL);
    if frame < 2 {
        let out = if frame == 0 { 3 } else { 6 };
        p.rect(5, 17, 10, out, c::PAPER_EDGE);
        p.rect(6, 17, 8, out - 1, c::WHITE);
        p.rect(7, 18, 5, 1, c::PAPER_EDGE);
    }
    p.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dart_opaque_pixels() {
        // Contados executando os retângulos Dart sobre uma grade independente.
        for stage in [
            CropStage::Seed,
            CropStage::Sprout,
            CropStage::Grow,
            CropStage::Ripe,
            CropStage::Wilt,
        ] {
            for frame in 0..3 {
                assert_eq!(
                    monitor(stage, frame)
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .filter(|p| p[3] == 255)
                        .count(),
                    120
                );
            }
        }
        for (frame, expected) in [0, 1, 2].into_iter().zip([492, 502, 492]) {
            assert_eq!(
                printer(frame)
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .filter(|p| p[3] == 255)
                    .count(),
                expected
            );
        }
    }
}
