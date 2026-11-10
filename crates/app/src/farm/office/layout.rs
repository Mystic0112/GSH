//! Sorteio de decoração de `office_world_paint.dart`.
use super::props::{Kind, Prop, KINDS};

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    fn contains_padded(self, x: f64, y: f64, pad: f64) -> bool {
        x > self.left - pad
            && x < self.right + pad
            && y > self.top - pad
            && y < self.bottom + pad + 4.0
    }
}

pub struct Layout<'a> {
    pub width: u32,
    pub height: u32,
    pub play: Rect,
    pub barn: Rect,
    pub field: Rect,
    pub yard: Rect,
    pub content: Rect,
    pub crow: (f64, f64),
    pub sign: (f64, f64),
    pub door: (f64, f64),
    pub pond: (f64, f64, f64, f64),
    pub paths: &'a [&'a [(f64, f64)]],
}

struct Mulberry32(u32);
impl Mulberry32 {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x6d2b79f5);
        let mut t = (self.0 ^ (self.0 >> 15)).wrapping_mul(1 | self.0);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        f64::from(t ^ (t >> 14)) / 4294967296.0
    }
}

fn blocked(l: &Layout<'_>, decor: &[Prop], x: f64, y: f64) -> bool {
    if l.barn.contains_padded(x, y, 8.0)
        || l.field.contains_padded(x, y, 2.0)
        || l.yard.contains_padded(x, y, 6.0)
    {
        return true;
    }
    if (x - l.crow.0).abs() < 12.0 && y > l.crow.1 - 24.0 && y < l.crow.1 + 6.0 {
        return true;
    }
    if (x - l.sign.0).abs() < 8.0 && y > l.sign.1 - 4.0 && y < l.sign.1 + 14.0 {
        return true;
    }
    let px = (x - l.pond.0) / (l.pond.2 + 8.0);
    let py = (y - l.pond.1) / (l.pond.3 + 6.0);
    if px * px + py * py < 1.0 {
        return true;
    }
    decor.iter().any(|d| {
        d.radius > 0.0
            && (x - d.x).abs() < d.radius + 5.0
            && y > d.y - d.radius - 2.0
            && y < d.y + 5.0
    })
}

fn on_path(l: &Layout<'_>, x: f64, y: f64) -> bool {
    l.paths.iter().any(|path| {
        path.windows(2).any(|points| {
            let [a, b] = points else { return false };
            x > a.0.min(b.0) - 9.0
                && x < a.0.max(b.0) + 9.0
                && y > a.1.min(b.1) - 9.0
                && y < a.1.max(b.1) + 9.0
        })
    })
}

/// Retorna os móveis na ordem de aceitação, como `l.decor` no Flutter.
pub fn decor(l: &Layout<'_>, seed: u32) -> Vec<Prop> {
    let mut rnd = Mulberry32(seed);
    let p = l.play;
    let width = p.right - p.left;
    let height = p.bottom - p.top;
    // _street usa três valores por ponto; _building usa dois por ponto.
    let street =
        ((f64::from(l.width) * 2.0 * (f64::from(l.height) * 2.0 - (p.bottom + 12.0) * 2.0)) / 14.0)
            .ceil()
            .max(0.0) as usize;
    let floor = (width * height / 30.0).ceil().max(0.0) as usize;
    for _ in 0..street * 3 + floor * 2 {
        rnd.next();
    }
    let want = (f64::from(l.width) * f64::from(l.height) / 4200.0)
        .round()
        .clamp(6.0, 40.0) as usize;
    let mut decor = Vec::new();
    for _ in 0..900 {
        if decor.len() >= want {
            break;
        }
        let x = p.left + 10.0 + rnd.next() * (width - 20.0);
        let mut y = p.top + 12.0 + rnd.next() * (height - 16.0);
        let mut pr = rnd.next();
        let mut kind = KINDS[0];
        for k in KINDS {
            if pr < k.1 {
                kind = k;
                break;
            }
            pr -= k.1;
        }
        if matches!(kind.0, Kind::Shelf | Kind::Sofa) {
            y = p.top + 13.0;
        }
        if blocked(l, &decor, x, y)
            || on_path(l, x, y)
            || (x - l.door.0).hypot(y - l.door.1) <= 30.0
            || (x > l.content.left - 4.0
                && x < l.content.right + 4.0
                && y > l.content.top - 4.0
                && y < l.content.bottom + 6.0)
            || decor.iter().any(|d| (d.x - x).hypot(d.y - y) < 26.0)
        {
            continue;
        }
        decor.push(Prop {
            kind: kind.0,
            x,
            y,
            radius: kind.2,
            variant: (rnd.next() * 6.0).floor() as u8,
        });
    }
    decor
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dart_mulberry32_seed_7() {
        // Executado com o Dart SDK 3.12.1 e mulberry32 de pix.dart.
        let mut rng = Mulberry32(7);
        let expected = [
            50271532_u32,
            266108690,
            4195786334,
            3002305430,
            2239590375,
            1741702388,
            2002453909,
            1030470827,
            2376515373,
            3134562008,
        ];
        for value in expected {
            assert_eq!(rng.next(), f64::from(value) / 4294967296.0);
        }
    }

    #[test]
    fn dart_decor_fixture() {
        // Referência executada no Dart 3.12.1 com os mesmos limites e semente.
        let outside = Rect {
            left: -1000.0,
            top: -1000.0,
            right: -990.0,
            bottom: -990.0,
        };
        let layout = Layout {
            width: 400,
            height: 300,
            play: Rect {
                left: 20.0,
                top: 20.0,
                right: 380.0,
                bottom: 280.0,
            },
            barn: outside,
            field: outside,
            yard: outside,
            content: outside,
            crow: (-1000.0, -1000.0),
            sign: (-1000.0, -1000.0),
            door: (-1000.0, -1000.0),
            pond: (-1000.0, -1000.0, 1.0, 1.0),
            paths: &[],
        };
        let actual = decor(&layout, 7);
        for (kind, count) in [
            (Kind::Plant, 13),
            (Kind::Shelf, 5),
            (Kind::Pouf, 3),
            (Kind::Lamp, 3),
            (Kind::Bin, 2),
            (Kind::Cooler, 1),
            (Kind::Sofa, 2),
        ] {
            assert_eq!(actual.iter().filter(|p| p.kind == kind).count(), count);
        }
        for (index, kind, x, y, radius, variant) in [
            (0, Kind::Plant, 265.1431201165542, 166.174067187123, 6.0, 1),
            (4, Kind::Shelf, 112.45398192666471, 33.0, 7.0, 3),
            (8, Kind::Sofa, 36.24967157840729, 33.0, 10.0, 4),
        ] {
            let Some(&prop) = actual.get(index) else {
                panic!("missing fixture prop")
            };
            assert_eq!(
                (prop.kind, prop.x, prop.y, prop.radius, prop.variant),
                (kind, x, y, radius, variant)
            );
        }
    }
}
