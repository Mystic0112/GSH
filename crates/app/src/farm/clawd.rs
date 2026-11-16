// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! O Clawd, o bicho de cada agente (farm_sprites.dart: `BodyPal` e `clawdHD`). Um
//! corpo de 32 x 36 em HD, desenhado a 16 x 18 de mundo. Os 288 quadros possíveis
//! são rasterizados uma vez, como o `_frames` do Flutter.
#![allow(dead_code)] // ponytail: chapéus e os minis dos subagentes ficam para depois

use super::pix::Pix;
use super::raster::{mix, shade};
use crate::theme::scene::clawd as c;
use iced::widget::image::Handle;
use std::sync::OnceLock;

/// Paleta HD de um corpo: tinta, brilho, luz, base, sombra, escuro, bochecha.
pub struct BodyPal {
    pub o: u32,
    pub l: u32,
    pub h: u32,
    pub b: u32,
    pub s: u32,
    pub d: u32,
    pub k: u32,
}

impl BodyPal {
    /// Corpo de argila (o padrão) ou um corpo tingido (o visual do squad).
    pub fn of(body: u32) -> Self {
        if body == c::CLAY {
            return Self {
                o: c::CLAY_O,
                l: c::CLAY_L,
                h: c::CLAY_H,
                b: c::CLAY,
                s: c::CLAY_S,
                d: c::CLAY_D,
                k: c::CLAY_K,
            };
        }
        Self {
            o: mix(shade(body, -0.5), c::INK_BASE, 0.25),
            l: shade(body, 0.2),
            h: shade(body, 0.09),
            b: body,
            s: shade(body, -0.11),
            d: shade(body, -0.28),
            k: mix(body, c::BLUSH_BASE, 0.45),
        }
    }
}

/// Uma pose do Clawd: `look` (-1, 0, 1), `legs` (0 parado, 1/2 passos), `arms`
/// (0 baixo, 1 pra cima, 2/3 digitando), e os olhos fechados, dormindo ou feliz.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pose {
    pub look: i64,
    pub legs: i64,
    pub blink: bool,
    pub sleep: bool,
    pub arms: i64,
    pub happy: bool,
}

/// Clawd em HD, 32 x 36 (o corpo começa na linha 12, embaixo do chapéu).
pub fn clawd_hd(pal: &BodyPal, pose: Pose) -> Pix {
    let Pose {
        look,
        legs,
        blink,
        sleep,
        arms,
        happy,
    } = pose;
    let mut p = Pix::new(32, 36);
    let (x0, x1) = if sleep { (4, 27) } else { (5, 26) };
    let (y0, y1) = if sleep { (18, 33) } else { (13, 30) };
    for y in y0..=y1 {
        for x in x0..=x1 {
            if (x == x0 || x == x1) && (y == y0 || y == y1) {
                continue;
            }
            let mut col = pal.b;
            if y <= y0 + 1 || x <= x0 + 1 {
                col = pal.h;
            }
            if y == y0 && x >= x0 + 2 && x <= x0 + 9 {
                col = pal.l;
            }
            if x == x0 + 1 && y == y0 + 1 {
                col = pal.l;
            }
            if x >= x1 - 1 || y >= y1 - 1 {
                col = pal.s;
            }
            if y == y1 && x > x0 + 1 && x < x1 - 1 {
                col = pal.d;
            }
            p.set(x, y, col);
        }
    }
    let ay = if sleep {
        [y1 - 5, y1 - 5]
    } else {
        match arms {
            1 => [15, 15],
            2 => [19, 22],
            3 => [22, 19],
            _ => [21, 21],
        }
    };
    let tall = if sleep { 3 } else { 4 };
    for k in 0..tall {
        let (a, b) = (ay[0] + k, ay[1] + k);
        p.set(x0 - 3, a, if k == 0 { pal.l } else { pal.h });
        p.set(x0 - 2, a, pal.h);
        p.set(x0 - 1, a, pal.b);
        p.set(x1 + 1, b, pal.b);
        p.set(x1 + 2, b, pal.s);
        p.set(x1 + 3, b, if k == tall - 1 { pal.d } else { pal.s });
    }
    if !sleep {
        let lxs = [6, 11, 19, 24];
        for (i, &lx) in lxs.iter().enumerate() {
            let up = legs != 0 && (i.is_multiple_of(2) == (legs == 1));
            let bottom = if up { 32 } else { 33 };
            for y in 31..=bottom {
                p.set(lx, y, if y == 31 { pal.s } else { pal.d });
                p.set(lx + 1, y, pal.d);
            }
        }
    }
    p.outline(pal.o);
    let ey = y0 + 4;
    let sh = look * 2;
    for ex in [10 + sh, 19 + sh] {
        if sleep {
            p.set(ex, ey + 3, c::EYE);
            p.set(ex + 1, ey + 4, c::EYE);
            p.set(ex + 2, ey + 3, c::EYE);
        } else if blink {
            p.rect(ex, ey + 4, 3, 1, c::EYE);
            p.set(ex - 1, ey + 3, pal.s);
            p.set(ex + 3, ey + 3, pal.s);
        } else {
            p.rect(ex, ey, 3, 6, c::EYE);
            p.set(ex, ey, c::SHINE);
            p.set(ex, ey + 1, c::SHINE);
            p.set(ex + 2, ey + 4, c::EYE_SHADOW);
        }
    }
    p.rect(7 + sh, ey + 7, 3, 1, pal.k);
    p.rect(22 + sh, ey + 7, 3, 1, pal.k);
    if happy && !sleep {
        p.rect(15 + sh, ey + 7, 2, 1, c::EYE);
        p.rect(15 + sh, ey + 8, 2, 1, c::MOUTH_HAPPY);
    }
    p
}

/// Índice do quadro no cache (a mesma conta de `FarmSkin.frame`, 288 combinações).
fn index(pose: Pose) -> usize {
    let code = (pose.look + 1)
        + 3 * pose.legs
        + 9 * i64::from(pose.blink)
        + 18 * i64::from(pose.sleep)
        + 36 * pose.arms
        + 144 * i64::from(pose.happy);
    usize::try_from(code).unwrap_or(0)
}

/// Todas as poses de argila, rasterizadas uma vez.
fn frames() -> &'static [Handle] {
    static FRAMES: OnceLock<Vec<Handle>> = OnceLock::new();
    FRAMES.get_or_init(|| {
        let pal = BodyPal::of(c::CLAY);
        let mut table = vec![Handle::from_rgba(1, 1, vec![0; 4]); 288];
        for look in -1..=1 {
            for legs in 0..=2 {
                for blink in [false, true] {
                    for sleep in [false, true] {
                        for arms in 0..=3 {
                            for happy in [false, true] {
                                let pose = Pose {
                                    look,
                                    legs,
                                    blink,
                                    sleep,
                                    arms,
                                    happy,
                                };
                                let rgba = clawd_hd(&pal, pose).to_rgba();
                                if let Some(slot) = table.get_mut(index(pose)) {
                                    *slot = Handle::from_rgba(32, 36, rgba);
                                }
                            }
                        }
                    }
                }
            }
        }
        table
    })
}

/// O quadro do Clawd na pose dada (cache de argila).
pub fn frame(pose: Pose) -> Handle {
    frames()
        .get(index(pose))
        .cloned()
        .unwrap_or_else(|| Handle::from_rgba(1, 1, vec![0; 4]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fnv(p: &[u32]) -> u32 {
        p.iter()
            .fold(2_166_136_261u32, |x, &c| (x ^ c).wrapping_mul(16_777_619))
    }

    fn stats(p: &Pix) -> (u32, usize) {
        (fnv(&p.p), p.p.iter().filter(|&&c| c != 0).count())
    }

    fn pose(look: i64, legs: i64, blink: bool, sleep: bool, arms: i64, happy: bool) -> Pose {
        Pose {
            look,
            legs,
            blink,
            sleep,
            arms,
            happy,
        }
    }

    /// Golden do harness Dart (golden/clawd.dart): mesma grade, mesmos pixels.
    #[test]
    fn clawd_igual_ao_dart() {
        let clay = BodyPal::of(c::CLAY);
        let none = pose(0, 0, false, false, 0, false);
        assert_eq!(stats(&clawd_hd(&clay, none)), (1_991_268_406, 542));
        assert_eq!(
            stats(&clawd_hd(&clay, pose(-1, 0, false, false, 0, false))),
            (3_663_177_060, 542)
        );
        assert_eq!(
            stats(&clawd_hd(&clay, pose(0, 1, false, false, 0, false))),
            (2_190_282_982, 534)
        );
        assert_eq!(
            stats(&clawd_hd(&clay, pose(0, 2, false, false, 0, false))),
            (1_716_235_798, 534)
        );
        assert_eq!(
            stats(&clawd_hd(&clay, pose(0, 0, true, false, 0, false))),
            (3_223_877_792, 542)
        );
        assert_eq!(
            stats(&clawd_hd(&clay, pose(0, 0, false, true, 0, false))),
            (1_236_422_634, 482)
        );
        assert_eq!(
            stats(&clawd_hd(&clay, pose(0, 0, false, false, 2, false))),
            (606_238_502, 542)
        );
        assert_eq!(
            stats(&clawd_hd(&clay, pose(0, 0, false, false, 3, false))),
            (3_944_433_126, 542)
        );
        assert_eq!(
            stats(&clawd_hd(&clay, pose(0, 0, false, false, 1, true))),
            (745_089_828, 542)
        );
        let teal = BodyPal::of(0xff2b_b3a3);
        assert_eq!(
            stats(&clawd_hd(&teal, pose(1, 1, false, false, 0, false))),
            (2_275_087_629, 534)
        );
    }

    #[test]
    fn cache_cobre_as_288_poses() {
        assert_eq!(frames().len(), 288);
        assert_ne!(
            index(pose(-1, 0, false, false, 0, false)),
            index(pose(1, 0, false, false, 0, false))
        );
    }
}
