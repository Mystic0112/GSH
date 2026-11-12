// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! Pintura do fundo do mundo (farm_world_paint.dart): grama com ruído, manchas,
//! árvores e pedras. Funções puras sobre `Raster`; o layout (bloqueios, clareira,
//! trilhas) entra por parâmetro até o `farm_world.dart` ser portado.
#![allow(dead_code)] // ponytail: o fundo ainda não está ligado ao cenário (falta layout e props)

use super::raster::{mulberry32, Raster, HD};
use crate::theme::scene::field as f;

type Tree = (f64, f64, f64);

const NOISE_N: i64 = 64;
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
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
/// Copas: (dx, dy, k) em fração do raio; cada bola tem raio `r * k`.
const TREE_BLOBS: [(f64, f64, f64); 6] = [
    (0.0, 0.0, 1.0),
    (-0.5, -0.15, 0.62),
    (0.5, -0.1, 0.6),
    (0.0, -0.52, 0.62),
    (-0.3, 0.3, 0.55),
    (0.35, 0.32, 0.5),
];

/// Ruído de valor (`_valueNoise`): grade 64x64 de aleatórios, interpolação suave.
struct ValueNoise {
    grid: Vec<f64>,
}

impl ValueNoise {
    fn new(seed: u32) -> Self {
        let mut rnd = mulberry32(seed);
        let grid = (0..NOISE_N * NOISE_N).map(|_| rnd()).collect();
        Self { grid }
    }

    fn at(&self, x: i64, y: i64) -> f64 {
        let idx = y.rem_euclid(NOISE_N) * NOISE_N + x.rem_euclid(NOISE_N);
        self.grid.get(idx as usize).copied().unwrap_or(0.0)
    }

    fn sample(&self, x: f64, y: f64) -> f64 {
        let (xf0, yf0) = (x.floor(), y.floor());
        let (xf, yf) = (x - xf0, y - yf0);
        let (xi, yi) = (xf0 as i64, yf0 as i64);
        let u = xf * xf * (3.0 - 2.0 * xf);
        let v = yf * yf * (3.0 - 2.0 * yf);
        let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
        lerp(
            lerp(self.at(xi, yi), self.at(xi + 1, yi), u),
            lerp(self.at(xi, yi + 1), self.at(xi + 1, yi + 1), u),
            v,
        )
    }
}

fn bayer(x: i64, y: i64) -> f64 {
    BAYER
        .get((y & 3) as usize)
        .and_then(|row| row.get((x & 3) as usize))
        .map_or(0.0, |&v| f64::from(v))
}

/// Grama: duas oitavas de ruído, dithering ordenado entre cinco tons e uma
/// clareira gasta. `clearing` = (cx, cy, rx, ry), em `_paintWorld`.
pub fn grass_field(g: &mut Raster, w: i64, h: i64, seed: u32, clearing: (f64, f64, f64, f64)) {
    let (cx, cy, rx, ry) = clearing;
    let n1 = ValueNoise::new(seed + 1);
    let n2 = ValueNoise::new(seed + 2);
    let tones = [f::G0, f::G1, f::G2, f::G3, f::G4, f::WORN];
    for y in 0..h {
        for x in 0..w {
            let (xf, yf) = (x as f64, y as f64);
            let ex = (xf - cx) / rx;
            let ey = (yf - cy) / ry;
            let e = ex * ex + ey * ey;
            let mut t = n1.sample(xf / 22.0, yf / 22.0) * 0.62
                + n2.sample(xf / 7.0, yf / 7.0) * 0.38
                + (1.0 - e.min(1.4)) * 0.3;
            t += (bayer(x, y) / 16.0 - 0.5) * 0.14;
            let k = ((t - 0.18) * 5.2).floor().clamp(0.0, 5.0) as usize;
            if let Some(&tone) = tones.get(k) {
                let (px, py) = ((x * HD) as f64, (y * HD) as f64);
                g.fill(px, py, HD as f64, HD as f64, tone);
            }
        }
    }
}

/// Tufos, manchas e flores espalhados sobre a grama (uma por 40 pixels).
/// `blocked` diz onde não cabe flor (o layout fornece).
pub fn grass_specks(
    g: &mut Raster,
    w: i64,
    h: i64,
    rnd: &mut dyn FnMut() -> f64,
    blocked: &dyn Fn(f64, f64) -> bool,
) {
    let hd = HD as f64;
    let count = ((w * h) as f64 / 40.0).ceil() as i64;
    for _ in 0..count {
        let x = (rnd() * w as f64 * hd).floor();
        let y = (rnd() * h as f64 * hd).floor();
        let r = rnd();
        if r < 0.55 {
            let cc = if rnd() < 0.5 { f::G0 } else { f::G1 };
            g.fill(x, y, 1.0, 3.0, cc);
            g.fill(x + 2.0, y + 1.0, 1.0, 2.0, cc);
            g.fill(x + 1.0, y + 2.0, 1.0, 1.0, cc);
        } else if r < 0.85 {
            let cc = if rnd() < 0.5 { f::G4 } else { f::GRASS_TUFT };
            g.fill(x, y, 1.0, 2.0, cc);
        } else if !blocked(x / hd, y / hd) {
            let idx = (rnd() * f::FLOWER_C.len() as f64).floor() as usize;
            let cc = f::FLOWER_C.get(idx).copied().unwrap_or(0);
            g.fill(x - 1.0, y, 3.0, 1.0, cc);
            g.fill(x, y - 1.0, 1.0, 3.0, cc);
            g.fill(x, y, 1.0, 1.0, f::FLOWER_CENTER);
        }
    }
}

/// Posições das árvores da borda, sem desenhar: (trás, frente). Mesma ordem de
/// sorteio do `_paintWorld`: a faixa da frente é sorteada antes de qualquer desenho.
pub fn place_trees(w: i64, h: i64, t: i64, rnd: &mut dyn FnMut() -> f64) -> (Vec<Tree>, Vec<Tree>) {
    let (w, h, t) = (w as f64, h as f64, t as f64);
    let mut trees: Vec<Tree> = Vec::new();
    let mut x = -6.0;
    while x < w + 12.0 {
        trees.push((x, 2.0 + rnd() * (t - 16.0), 10.0 + rnd() * 5.0));
        if rnd() < 0.6 {
            trees.push((x + 6.0, -6.0 + rnd() * 8.0, 11.0 + rnd() * 4.0));
        }
        x += 12.0 + rnd() * 9.0;
    }
    let mut y = t - 4.0;
    while y < h - t + 6.0 {
        trees.push((2.0 + rnd() * (t - 14.0), y, 9.0 + rnd() * 5.0));
        trees.push((w - 2.0 - rnd() * (t - 14.0), y, 9.0 + rnd() * 5.0));
        if rnd() < 0.5 {
            trees.push((-4.0, y + 6.0, 11.0));
        }
        if rnd() < 0.5 {
            trees.push((w + 4.0, y + 6.0, 11.0));
        }
        y += 12.0 + rnd() * 8.0;
    }
    let mut front: Vec<Tree> = Vec::new();
    let mut x = -6.0;
    while x < w + 12.0 {
        front.push((x, h - t + 12.0 + rnd() * 10.0, 11.0 + rnd() * 5.0));
        x += 12.0 + rnd() * 9.0;
    }
    (trees, front)
}

/// Uma árvore (`_tree`): sombra, tronco, copa em bolas de três tons e folhas.
pub fn tree(g: &mut Raster, x: i64, y: i64, r: i64, rnd: &mut dyn FnMut() -> f64) {
    let (xf, yf, rf) = (x as f64, y as f64, r as f64);
    g.ellipse(
        xf + 4.0,
        yf + rf - 2.0,
        rf + 3.0,
        (rf / 3.0).round().max(5.0),
        f::SHADOW32,
    );
    g.fill(xf - 4.0, yf + rf - 15.0, 8.0, 15.0, f::TRUNK);
    g.fill(xf - 4.0, yf + rf - 15.0, 2.0, 15.0, f::BARK_LIGHT);
    g.fill(xf + 2.0, yf + rf - 15.0, 2.0, 15.0, f::BARK_DARK);
    g.fill(xf - 6.0, yf + rf - 2.0, 3.0, 2.0, f::TRUNK);
    g.fill(xf + 3.0, yf + rf - 2.0, 3.0, 2.0, f::BARK_DARK);
    let blobs: Vec<Tree> = TREE_BLOBS
        .iter()
        .map(|&(dx, dy, k)| (xf + dx * rf, yf + dy * rf, (rf * k).max(3.0)))
        .collect();
    for &(bx, by, br) in &blobs {
        g.circle(bx, by, br + 2.0, f::T0);
    }
    for &(bx, by, br) in &blobs {
        g.circle(bx, by, br, f::T1);
    }
    for &(bx, by, br) in &blobs {
        g.circle(bx - br * 0.18, by - br * 0.22, br * 0.78, f::T2);
    }
    for &(bx, by, br) in &blobs {
        if by < yf + rf * 0.2 {
            g.circle(bx - br * 0.32, by - br * 0.38, br * 0.46, f::T3);
        }
    }
    for &(bx, by, br) in blobs.iter().skip(1).take(3) {
        g.circle(bx - br * 0.4, by - br * 0.46, br * 0.2, f::T4);
    }
    let mut i = 0;
    while (i as f64) < rf * 2.2 {
        let a = rnd() * std::f64::consts::PI * 2.0;
        let d = rnd() * rf * 0.95;
        let lx = (xf + a.cos() * d).round();
        let ly = (yf + a.sin() * d * 0.9).round();
        g.fill(lx, ly, 2.0, 1.0, if ly < yf { f::T4 } else { f::T1 });
        if ly < yf - rf * 0.3 {
            g.fill(lx, ly - 1.0, 1.0, 1.0, f::T5);
        }
        i += 1;
    }
}

/// Uma pedra (`_rock`): sombra, corpo em anéis e musgo; `left, top, width, height`
/// em pixels de mundo. Em pedra com `left + top` ímpar, a faísca do Claude é gravada.
/// Obs.: `rx`/`ry` usam a medida de mundo (sem `HD`), como no Dart.
pub fn rock(g: &mut Raster, left: f64, top: f64, width: f64, height: f64) {
    let hd = HD as f64;
    let cx = (left * hd).round();
    let rx = width;
    let ry = (height * 0.9).round();
    let cy = (top + height) * hd - ry;
    let cy = cy.round();
    g.ellipse(cx + 3.0, cy + ry, rx + 2.0, 4.0, f::SHADOW38);
    g.ellipse(cx, cy, rx + 1.0, ry + 1.0, f::R0);
    g.ellipse(cx, cy, rx, ry, f::R1);
    g.ellipse(
        cx - (rx * 0.15).round(),
        cy - (ry * 0.2).round(),
        (rx * 0.72).round(),
        (ry * 0.62).round(),
        f::R2,
    );
    g.ellipse(
        cx - (rx * 0.35).round(),
        cy - (ry * 0.45).round(),
        (rx * 0.32).round(),
        (ry * 0.22).round(),
        f::R3,
    );
    g.fill(
        cx - (rx * 0.5).round(),
        cy - (ry * 0.55).round(),
        2.0,
        1.0,
        f::R4,
    );
    g.ellipse(
        cx + (rx * 0.3).round(),
        cy - ry + 1.0,
        (rx * 0.32).round(),
        1.0,
        f::ROCK_MOSS,
    );
    g.fill(
        cx + (rx * 0.2).round(),
        cy - ry,
        3.0,
        1.0,
        f::ROCK_MOSS_LIGHT,
    );
    if ((left + top).round() as i64).rem_euclid(2) == 1 {
        for (j, row) in SPARK.iter().enumerate() {
            for (i, ch) in row.chars().enumerate() {
                if ch != '.' && (i + j) % 2 == 0 {
                    g.fill(
                        cx - 4.0 + i as f64,
                        cy - 3.0 + j as f64,
                        1.0,
                        1.0,
                        f::ROCK_SPARK,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hash FNV-1a sobre as palavras ARGB: a mesma conta do harness Dart.
    fn fnv(p: &[u32]) -> u32 {
        p.iter()
            .fold(2_166_136_261u32, |x, &c| (x ^ c).wrapping_mul(16_777_619))
    }

    fn painted(g: &Raster) -> (u32, usize) {
        (fnv(&g.p), g.p.iter().filter(|&&c| c != 0).count())
    }

    #[test]
    fn mulberry32_igual_ao_dart() {
        let mut rnd = mulberry32(7);
        let got = [rnd(), rnd(), rnd(), rnd()];
        assert_eq!(
            got,
            [
                0.011704753153026104,
                0.06195825757458806,
                0.97690763277933,
                0.6990287057124078
            ]
        );
    }

    #[test]
    fn ruido_de_valor_igual_ao_dart() {
        let n = ValueNoise::new(8);
        assert_eq!(n.sample(0.5, 0.25), 0.439126705063245);
        assert_eq!(n.sample(3.7, 12.2), 0.661867334754288);
        assert_eq!(n.sample(-1.3, 4.9), 0.17838531591536833);
    }

    #[test]
    fn circulo_e_elipse_com_alpha_igual_ao_dart() {
        let mut c = Raster::new(30, 30);
        c.circle(15.0, 15.0, 7.0, 0xff11_2233);
        assert_eq!(painted(&c), (3_558_517_428, 177));

        let mut e = Raster::new(30, 30);
        e.fill(0.0, 0.0, 30.0, 30.0, 0xff0a_0b0c);
        e.ellipse(15.0, 15.0, 9.0, 4.0, 0x80ab_cdef);
        assert_eq!(painted(&e), (301_987_486, 900));
    }

    #[test]
    fn arvore_igual_ao_dart() {
        let mut g = Raster::new(80, 80);
        tree(&mut g, 40, 50, 16, &mut mulberry32(5));
        assert_eq!(painted(&g), (4_069_967_211, 1258));
    }

    #[test]
    fn pedra_com_faisca_igual_ao_dart() {
        let mut g = Raster::new(80, 60);
        rock(&mut g, 11.5, 21.0, 13.0, 9.0);
        assert_eq!(painted(&g), (1_722_857_735, 399));
    }

    #[test]
    fn grama_e_manchas_iguais_ao_dart() {
        let mut g = Raster::new(60, 40);
        grass_field(&mut g, 30, 20, 7, (20.0, 12.0, 40.0, 30.0));
        grass_specks(&mut g, 30, 20, &mut mulberry32(9), &|_, _| false);
        assert_eq!(painted(&g), (923_576_175, 2400));
    }

    #[test]
    fn posicao_das_arvores_igual_ao_dart() {
        let (trees, front) = place_trees(120, 80, 14, &mut mulberry32(11));
        let expect_trees: Vec<Tree> = vec![
            (-6.0, 0.9768259027041495, 12.649732041172683),
            (11.311418723780662, 0.2984466077759862, 12.690111487172544),
            (17.311418723780662, -1.755955906584859, 13.430946335196495),
            (27.36434273980558, 1.039915060158819, 14.217603360302746),
            (33.36434273980558, -0.6383392866700888, 13.726114348508418),
            (40.81166976923123, 1.642970853485167, 11.380531808827072),
            (46.81166976923123, -2.984293779358268, 13.330036328174174),
            (53.95900279493071, 0.8058360819704831, 12.882693752180785),
            (59.95900279493071, 0.7540029771625996, 14.221539250575006),
            (72.87824339861982, 1.3647782765328884, 13.115537585690618),
            (78.87824339861982, -5.55013170465827, 13.96364171244204),
            (93.67727205483243, 1.4063263116404414, 10.732666963012889),
            (108.8021989481058, 0.8931080941110849, 12.883359203115106),
            (122.09116422291845, 0.3005857737734914, 14.101435180054978),
            (128.09116422291845, -5.796668045222759, 11.306881264783442),
            (2.0, 10.0, 11.630552019691095),
            (118.0, 10.0, 11.510042660869658),
            (124.0, 16.0, 11.0),
            (2.0, 27.152788531035185, 11.222691839560866),
            (118.0, 27.152788531035185, 13.92186619550921),
            (-4.0, 33.152788531035185, 11.0),
            (124.0, 33.152788531035185, 11.0),
            (2.0, 41.97609137929976, 12.667029527714476),
            (118.0, 41.97609137929976, 12.797306944616139),
            (-4.0, 47.97609137929976, 11.0),
            (2.0, 56.75868901796639, 9.800866248318925),
            (118.0, 56.75868901796639, 12.151133917272091),
            (124.0, 62.75868901796639, 11.0),
            (2.0, 71.71210115589201, 13.686520461691543),
            (118.0, 71.71210115589201, 12.149472050135955),
            (-4.0, 77.71210115589201, 11.0),
        ];
        let expect_front: Vec<Tree> = vec![
            (-6.0, 82.34858965221792, 12.539286004845053),
            (10.630558751290664, 78.08593933889642, 15.403546679532155),
            (25.344922320451587, 82.0814150753431, 11.617461067158729),
            (38.06277427403256, 85.45100487256423, 13.722585753072053),
            (54.689554853131995, 79.03362665744498, 13.79839766328223),
            (72.28791190311313, 86.84408250916749, 15.8068419797346),
            (89.67162313987501, 84.03743408573791, 13.86593550699763),
            (106.05489904247224, 78.94432590762153, 14.258705043699592),
            (123.91219625947997, 87.95421951171011, 14.552690573269501),
        ];
        assert_eq!(trees, expect_trees);
        assert_eq!(front, expect_front);
    }
}
