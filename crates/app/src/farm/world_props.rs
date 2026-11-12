// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! Props do fundo (farm_world_props.dart): celeiro, canteiro, cercas, curral, lago,
//! placa, caminho, feno e decoração. Desenho em `Raster`, na ordem do Dart.
//! As cores ARGB inline são a paleta de pixel do próprio Flutter (sem nome lá);
//! as com nome estão em `theme::scene::field`.
#![allow(dead_code)] // ponytail: o fundo ainda não está ligado ao cenário (falta o desenho da imagem)

use super::layout::{Decor, Layout, Offset, Rect};
use super::raster::{Raster, HD};
use super::world_paint::SPARK;
use crate::theme::scene::field as f;

const FLOWER_STEM: u32 = 0xff3f7a2a;
const FLOWER_CENTER: u32 = 0xfff5c542;

fn hay(g: &mut Raster, x: f64, y: f64) {
    g.fill(x, y, 18.0, 14.0, 0xff7a5a1a);
    g.fill(x + 1.0, y + 1.0, 16.0, 12.0, 0xffe0b64e);
    g.fill(x + 1.0, y + 1.0, 16.0, 3.0, 0xfff0cf7a);
    g.fill(x + 1.0, y + 10.0, 16.0, 3.0, 0xffc7973a);
    g.fill(x + 5.0, y + 1.0, 2.0, 12.0, 0xffa8782a);
    g.fill(x + 12.0, y + 1.0, 2.0, 12.0, 0xffa8782a);
    for i in 0..6 {
        let fi = f64::from(i);
        g.fill(
            x + 2.0 + fi * 3.0,
            y - 1.0 + f64::from(i % 2),
            1.0,
            2.0,
            0xfff0cf7a,
        );
    }
}

/// O celeiro: paredes de tábua, telhado de empena, sótão, portão com X e feno.
pub fn barn(g: &mut Raster, b: Rect) {
    let hd = HD as f64;
    let x = b.left * hd;
    let y = b.top * hd;
    let w = b.width() * hd;
    let hh = b.height() * hd;
    const ROOF_H: f64 = 32.0;
    g.ellipse(
        x + w / 2.0 + 6.0,
        y + hh + 1.0,
        w / 2.0 + 12.0,
        8.0,
        f::SHADOW36,
    );
    // paredes: tábuas vermelhas, cantoneira branca, base de pedra
    let wy = y + ROOF_H - 2.0;
    let wh = hh - ROOF_H + 2.0;
    g.fill(x, wy, w, wh, 0xff3a1410);
    let mut i = 0;
    let mut px = x + 2.0;
    while px < x + w - 2.0 {
        let c = if i % 2 == 1 { 0xffa8432f } else { 0xffb34a33 };
        g.fill(px, wy, 6.0, wh - 2.0, c);
        g.fill(px, wy, 1.0, wh - 2.0, 0xff8e3624);
        px += 6.0;
        i += 1;
    }
    g.fill(x + 2.0, wy, w - 4.0, 2.0, 0xffc95c42);
    for k in 0..18 {
        let kf = f64::from(k);
        g.fill(
            x + 4.0 + ((kf * 37.0) % (w - 10.0)),
            wy + 6.0 + ((kf * 23.0) % (wh - 12.0)),
            2.0,
            1.0,
            0xff933826,
        );
    }
    g.fill(x + 2.0, wy, 4.0, wh - 2.0, 0xffefe4cf);
    g.fill(x + w - 6.0, wy, 4.0, wh - 2.0, 0xffd6c8ad);
    g.fill(x, y + hh - 5.0, w, 5.0, 0xff6f6a66);
    let mut px = x + 2.0;
    while px < x + w - 2.0 {
        g.fill(px, y + hh - 5.0, 5.0, 3.0, 0xff8d8783);
        px += 7.0;
    }
    // telhado: empena escalonada de telhas com acabamento branco
    for k in 0..32 {
        let kf = f64::from(k);
        let inset = (((ROOF_H - kf) * 0.95).round() - 6.0).max(0.0);
        let rx = x - 4.0 + inset;
        let rw = w + 8.0 - inset * 2.0;
        g.fill(rx, y + kf, rw, 1.0, 0xff2d1511);
        let body = if k % 4 == 3 { 0xff4a2019 } else { 0xff6a3024 };
        g.fill(rx + 2.0, y + kf, (rw - 4.0).max(0.0), 1.0, body);
        if k % 4 != 3 {
            let mut sx = rx + 4.0 + f64::from((k >> 2) % 2) * 4.0;
            while sx < rx + rw - 4.0 {
                g.fill(sx, y + kf, 1.0, 1.0, 0xff4f231b);
                sx += 8.0;
            }
        }
        g.fill(rx + 2.0, y + kf, 2.0, 1.0, 0xfff3ead8);
        g.fill(rx + rw - 4.0, y + kf, 2.0, 1.0, 0xffd6c8ad);
    }
    g.fill(x - 4.0, y + ROOF_H - 2.0, w + 8.0, 2.0, 0xfff3ead8);
    // janela do sótão com feno
    let lx = x + w / 2.0 - 10.0;
    let ly = y + 10.0;
    g.fill(lx, ly, 20.0, 16.0, 0xfff3ead8);
    g.fill(lx + 2.0, ly + 2.0, 16.0, 12.0, 0xff2d1a10);
    g.fill(lx + 2.0, ly + 9.0, 16.0, 5.0, 0xffe0b64e);
    for k in 0..5 {
        let kf = f64::from(k);
        g.fill(
            lx + 3.0 + kf * 3.0,
            ly + 7.0 + f64::from(k % 2),
            1.0,
            3.0,
            0xfff0cf7a,
        );
    }
    g.fill(lx + 9.0, ly + 2.0, 2.0, 12.0, 0xfff3ead8);
    // o portão grande com o X branco
    const DW: f64 = 34.0;
    let dh = wh - 9.0;
    let dx = x + w / 2.0 - DW / 2.0;
    let dy = wy + 6.0;
    g.fill(dx - 2.0, dy - 2.0, DW + 4.0, dh + 2.0, 0xfff3ead8);
    g.fill(dx, dy, DW, dh, 0xff7a2c1f);
    let mut px = dx + 3.0;
    while px < dx + DW {
        g.fill(px, dy, 1.0, dh, 0xff6a2519);
        px += 4.0;
    }
    let mut k = 0;
    while f64::from(k) < dh {
        let kf = f64::from(k);
        let t = ((kf / dh) * (DW / 2.0 - 2.0)).round();
        g.fill(dx + t, dy + kf, 2.0, 1.0, 0xfff3ead8);
        g.fill(dx + DW / 2.0 - 2.0 - t, dy + kf, 2.0, 1.0, 0xfff3ead8);
        g.fill(dx + DW / 2.0 + t, dy + kf, 2.0, 1.0, 0xfff3ead8);
        g.fill(dx + DW - 2.0 - t, dy + kf, 2.0, 1.0, 0xfff3ead8);
        k += 1;
    }
    g.fill(dx + DW / 2.0 - 1.0, dy, 2.0, dh, 0xfff3ead8);
    g.fill(dx - 1.0, dy + 4.0, 3.0, 2.0, 0xff2d1a10);
    g.fill(dx - 1.0, dy + dh - 7.0, 3.0, 2.0, 0xff2d1a10);
    g.fill(dx + DW - 2.0, dy + 4.0, 3.0, 2.0, 0xff2d1a10);
    g.fill(dx + DW - 2.0, dy + dh - 7.0, 3.0, 2.0, 0xff2d1a10);
    // lampião e o catavento: a faísca do Claude na cumeeira
    g.fill(x + w / 2.0 + 22.0, wy + 8.0, 4.0, 6.0, 0xff2d1a10);
    g.fill(x + w / 2.0 + 23.0, wy + 9.0, 2.0, 4.0, 0xffffd27a);
    let vx = x + w / 2.0;
    let vy = y - 12.0;
    g.fill(vx - 1.0, vy + 6.0, 2.0, 8.0, 0xff2d1511);
    for (j, row) in SPARK.iter().enumerate() {
        for (k, ch) in row.chars().enumerate() {
            if ch != '.' {
                let c = if ch == 'w' { 0xffffd59e } else { 0xffd97757 };
                g.fill(vx - 5.0 + k as f64, vy - 4.0 + j as f64, 1.0, 1.0, c);
            }
        }
    }
    hay(g, x + w + 2.0, y + hh - 14.0);
    hay(g, x - 20.0, y + hh - 14.0);
    hay(g, x + w + 6.0, y + hh - 26.0);
}

/// Canteiro de terra: borda escura, sulcos e pedrinhas.
pub fn plot(g: &mut Raster, p: Rect) {
    let hd = HD as f64;
    let x = p.left * hd;
    let y = p.top * hd;
    let w = p.width() * hd;
    let hh = p.height() * hd;
    g.fill(x - 2.0, y, w + 4.0, hh + 2.0, f::SOIL_DEEP);
    g.fill(x, y - 2.0, w, hh + 6.0, f::SOIL_DEEP);
    g.fill(x, y, w, hh + 2.0, 0xff4f3420);
    g.fill(x, y, w, hh, f::SOIL);
    let mut yy = y + 3.0;
    while yy < y + hh - 1.0 {
        g.fill(x + 2.0, yy, w - 4.0, 2.0, f::SOIL3);
        g.fill(x + 2.0, yy + 2.0, w - 4.0, 2.0, f::SOIL2);
        g.fill(x + 3.0, yy, w - 8.0, 1.0, 0xff8e6a46);
        yy += 6.0;
    }
    for i in 0..10 {
        let fi = f64::from(i);
        let c = if i % 2 == 1 { f::SOIL_DEEP } else { 0xff9a7650 };
        g.fill(
            x + 3.0 + (fi * 17.0) % (w - 6.0),
            y + 2.0 + (fi * 11.0) % (hh - 4.0),
            1.0,
            1.0,
            c,
        );
    }
}

/// Caminho de terra: três tons em faixas e grãos por cima.
pub fn path(g: &mut Raster, pts: &[Offset], rnd: &mut dyn FnMut() -> f64) {
    let hd = HD as f64;
    let seg = |g: &mut Raster, a: Offset, b: Offset, pad: f64, c: u32| {
        g.fill(
            a.dx.min(b.dx) * hd - pad,
            a.dy.min(b.dy) * hd - pad,
            (a.dx - b.dx).abs() * hd + pad * 2.0,
            (a.dy - b.dy).abs() * hd + pad * 2.0,
            c,
        );
    };
    for (pad, c) in [(8.0, 0xff7f6a3d), (7.0, 0xffa88a58), (5.0, 0xffb69866)] {
        for w in pts.windows(2) {
            if let [a, b] = w {
                seg(g, *a, *b, pad, c);
            }
        }
    }
    for w in pts.windows(2) {
        let [a, b] = w else { continue };
        let n = (((a.dx - b.dx).abs() + (a.dy - b.dy).abs()) * 0.8).round() as i64;
        for _ in 0..n {
            let t = rnd();
            let px = ((a.dx + (b.dx - a.dx) * t) * hd + (rnd() - 0.5) * 11.0).round();
            let py = ((a.dy + (b.dy - a.dy) * t) * hd + (rnd() - 0.5) * 11.0).round();
            let c = if rnd() < 0.5 { 0xffcdb07c } else { 0xff8b7045 };
            g.fill(px, py, 2.0, 1.0, c);
        }
    }
}

fn post(g: &mut Raster, x: f64, y: f64) {
    g.fill(x - 2.0, y - 11.0, 5.0, 13.0, 0xff3a2616);
    g.fill(x - 1.0, y - 10.0, 3.0, 11.0, 0xff8a6038);
    g.fill(x - 1.0, y - 10.0, 3.0, 2.0, 0xffb08250);
    g.fill(x - 2.0, y + 2.0, 6.0, 2.0, f::SHADOW30);
}

fn hrail(g: &mut Raster, xa: f64, xb: f64, y: f64) {
    g.fill(xa, y - 8.0, xb - xa, 2.0, 0xff3a2616);
    g.fill(xa, y - 7.0, xb - xa, 1.0, 0xffa37446);
    g.fill(xa, y - 4.0, xb - xa, 2.0, 0xff3a2616);
    g.fill(xa, y - 3.0, xb - xa, 1.0, 0xff8a6038);
}

fn vrail(g: &mut Raster, x: f64, ya: f64, yb: f64) {
    g.fill(x - 1.0, ya, 3.0, yb - ya, 0xff3a2616);
    g.fill(x, ya, 1.0, yb - ya, 0xff8a6038);
}

/// A cerca do cercado, com portão no lado esquerdo na altura `gate_at`.
pub fn fence(g: &mut Raster, fr: Rect, gate_at: f64) {
    let hd = HD as f64;
    let x0 = fr.left * hd;
    let y0 = fr.top * hd;
    let x1 = fr.right * hd;
    let y1 = fr.bottom * hd;
    let gw = 16.0 * hd;
    let (gl0, gl1) = (gate_at * hd - gw / 2.0, gate_at * hd + gw / 2.0);
    hrail(g, x0, x1, y0);
    vrail(g, x0, y0, gl0);
    vrail(g, x0, gl1, y1);
    vrail(g, x1, y0, y1);
    let mut x = x0;
    while x <= x1 {
        post(g, x.min(x1), y0);
        x += 20.0;
    }
    let mut y = y0 + 20.0;
    while y < y1 {
        if y < gl0 - 2.0 || y > gl1 + 2.0 {
            post(g, x0, y);
        }
        post(g, x1, y);
        y += 20.0;
    }
    post(g, x0, gl0);
    post(g, x0, gl1);
    hrail(g, x0, x1, y1);
    let mut x = x0;
    while x <= x1 {
        post(g, x.min(x1), y1);
        x += 20.0;
    }
}

/// O curral: palha no chão, cerca de madeira com portão e estacas.
pub fn yard(g: &mut Raster, yr: Rect, rnd: &mut dyn FnMut() -> f64) {
    let hd = HD as f64;
    let x = yr.left * hd;
    let y = yr.top * hd;
    let w = yr.width() * hd;
    let hh = yr.height() * hd;
    g.fill(x - 2.0, y - 2.0, w + 4.0, hh + 4.0, f::YARD_SHADE);
    g.fill(x, y, w, hh, 0xffb99a55);
    const STRAW: [u32; 4] = [0xffd8bb6c, 0xffa4843f, 0xffc9a95c, 0xffe6cc84];
    let count = ((w * hh) / 18.0).ceil() as i64;
    for i in 0..count {
        let px = (x + rnd() * (w - 3.0)).round();
        let py = (y + rnd() * (hh - 1.0)).round();
        let c = STRAW.get((i % 4) as usize).copied().unwrap_or(0);
        g.fill(px, py, 3.0, 1.0, c);
    }
    let cx = yr.center().dx * hd;
    let (gate0, gate1) = ((cx - 16.0).round(), (cx + 16.0).round());
    rail(g, x - 4.0, gate0, y);
    rail(g, gate1, x + w + 4.0, y);
    for px in [x - 4.0, x + w + 3.0] {
        g.fill(px - 1.0, y, 3.0, hh, 0xff3a2616);
        g.fill(px, y, 1.0, hh, 0xff8a6038);
    }
    rail(g, x - 4.0, x + w + 4.0, y + hh);
    let mut px = x - 4.0;
    while px <= x + w + 4.0 {
        if px < gate0 - 3.0 || px > gate1 + 3.0 {
            yard_post(g, px, y);
        }
        yard_post(g, px, y + hh);
        px += 18.0;
    }
    yard_post(g, gate0, y);
    yard_post(g, gate1, y);
    let mut py = y + 18.0;
    while py < y + hh {
        yard_post(g, x - 4.0, py);
        yard_post(g, x + w + 3.0, py);
        py += 18.0;
    }
}

fn yard_post(g: &mut Raster, px: f64, py: f64) {
    g.fill(px - 2.0, py - 9.0, 5.0, 12.0, 0xff3a2616);
    g.fill(px - 1.0, py - 8.0, 3.0, 10.0, 0xff8a6038);
    g.fill(px - 1.0, py - 8.0, 3.0, 2.0, 0xffb08250);
}

fn rail(g: &mut Raster, xa: f64, xb: f64, py: f64) {
    g.fill(xa, py - 6.0, xb - xa, 2.0, 0xff3a2616);
    g.fill(xa, py - 5.0, xb - xa, 1.0, 0xffa37446);
    g.fill(xa, py - 2.0, xb - xa, 2.0, 0xff3a2616);
}

/// O lago com bordas de pedra, juncos e uma flor de nenúfar.
pub fn pond(g: &mut Raster, l: &Layout, rnd: &mut dyn FnMut() -> f64) {
    let hd = HD as f64;
    let cx = l.pond_cx() * hd;
    let cy = l.pond_cy() * hd;
    let rx = l.pond_rx() * hd;
    let ry = l.pond_ry() * hd;
    g.ellipse(cx, cy + 2.0, rx + 7.0, ry + 6.0, 0xff8e7a4c);
    g.ellipse(cx, cy + 1.0, rx + 5.0, ry + 4.0, 0xffc8b27a);
    g.ellipse(cx, cy + 1.0, rx + 2.0, ry + 2.0, 0xff3d5a22);
    g.ellipse(cx, cy, rx, ry, f::W0);
    g.ellipse(cx, cy + 2.0, rx - 1.0, ry - 1.0, f::W1);
    g.ellipse(cx - 6.0, cy - 3.0, rx - 14.0, ry - 8.0, f::W2);
    for _ in 0..10 {
        let x = (cx - rx * 0.6 + rnd() * rx * 1.2).round();
        let y = (cy - ry * 0.5 + rnd() * ry).round();
        let w = 4.0 + (rnd() * 4.0).round();
        g.fill(x, y, w, 1.0, f::W3);
    }
    for (dx, dy, s) in [(-0.45, 0.2, 1.0), (0.35, -0.15, 0.8), (0.15, 0.45, 0.7)] {
        let px = cx + dx * rx;
        let py = cy + dy * ry;
        let r = 5.0 * s + 2.0;
        g.ellipse(px, py, r + 1.0, r * 0.6 + 1.0, 0xff1f4d1d);
        g.ellipse(px, py, r, r * 0.6, 0xff4f9a3a);
        g.fill(px, py - 1.0, r.ceil(), 2.0, f::W1);
        g.fill(px - r * 0.5, py - 2.0, 3.0, 1.0, 0xff7cc35a);
    }
    g.fill(
        cx - rx * 0.45 - 1.0,
        cy + ry * 0.2 - 4.0,
        3.0,
        3.0,
        0xfff59cc0,
    );
    g.fill(cx - rx * 0.45, cy + ry * 0.2 - 3.0, 1.0, 1.0, 0xfffff4c2);
    for s in [-1.0, 1.0] {
        for i in 0..4 {
            let fi = f64::from(i);
            let rxp = (cx + s * (rx - 6.0 + fi * 4.0)).round();
            let top = cy - 10.0 - fi * 3.0 - f64::from(i % 2) * 3.0;
            let c = if i % 2 == 1 { 0xff2f5a1f } else { 0xff3f7a2a };
            g.fill(rxp, top, 1.0, cy + 4.0 - top, c);
            g.fill(rxp - 1.0, top - 1.0, 3.0, 5.0, 0xff6b4a2b);
            g.fill(rxp, top - 3.0, 1.0, 2.0, 0xff3f7a2a);
        }
    }
}

/// A placa de seta apontando para o cercado.
pub fn sign(g: &mut Raster, s: Offset) {
    let hd = HD as f64;
    let x = s.dx * hd;
    let y = s.dy * hd;
    g.fill(x - 1.0, y + 6.0, 3.0, 18.0, 0xff3a2616);
    g.fill(x, y + 6.0, 1.0, 18.0, 0xff8a6038);
    g.fill(x - 3.0, y + 23.0, 8.0, 2.0, f::SHADOW35);
    g.fill(x - 11.0, y - 2.0, 22.0, 11.0, 0xff3a2616);
    g.fill(x - 10.0, y - 1.0, 20.0, 9.0, 0xffc9964a);
    g.fill(x - 10.0, y - 1.0, 20.0, 2.0, 0xffdcae62);
    // uma seta: para o cercado
    g.fill(x - 7.0, y + 3.0, 9.0, 2.0, 0xff3a2616);
    g.fill(x + 2.0, y + 1.0, 1.0, 6.0, 0xff3a2616);
    g.fill(x + 3.0, y + 2.0, 1.0, 4.0, 0xff3a2616);
    g.fill(x + 4.0, y + 3.0, 1.0, 2.0, 0xff3a2616);
}

/// Uma decoração do campo: flores, arbusto, abóbora, tronco ou cogumelos.
pub fn decor(g: &mut Raster, d: &Decor, rnd: &mut dyn FnMut() -> f64) {
    let hd = HD as f64;
    let x = (d.x * hd).round();
    let y = (d.y * hd).round();
    match d.kind {
        "flowers" => {
            let n = 3 + (rnd() * 4.0).floor() as i64;
            for i in 0..n {
                let fx = x + ((rnd() - 0.5) * 16.0).round();
                let fy = y + ((rnd() - 0.5) * 8.0).round();
                let c = f::FLOWER_C
                    .get((d.c + i as usize) % f::FLOWER_C.len())
                    .copied()
                    .unwrap_or(0);
                g.fill(fx, fy, 1.0, 3.0, FLOWER_STEM);
                g.fill(fx - 1.0, fy - 2.0, 3.0, 1.0, c);
                g.fill(fx, fy - 3.0, 1.0, 3.0, c);
                g.fill(fx, fy - 2.0, 1.0, 1.0, FLOWER_CENTER);
            }
        }
        "bush" => {
            const BLOBS: [(f64, f64, f64); 4] = [
                (-5.0, -4.0, 6.0),
                (5.0, -4.0, 6.0),
                (0.0, -8.0, 6.0),
                (0.0, -3.0, 7.0),
            ];
            g.ellipse(x + 2.0, y + 1.0, 12.0, 4.0, f::SHADOW30);
            for &(dx, dy, r) in &BLOBS {
                g.circle(x + dx, y + dy, r + 1.0, f::T0);
            }
            for &(dx, dy, r) in &BLOBS {
                g.circle(x + dx, y + dy, r, f::T2);
                g.circle(x + dx - 1.0, y + dy - 2.0, r * 0.55, f::T3);
            }
            if d.c % 2 == 1 {
                for (dx, dy) in [(-6.0, -5.0), (3.0, -9.0), (6.0, -3.0), (-1.0, -2.0)] {
                    g.fill(x + dx, y + dy, 2.0, 2.0, 0xffd6334a);
                    g.fill(x + dx, y + dy, 1.0, 1.0, 0xffff8a9a);
                }
            }
        }
        "pumpkin" => {
            for (dx, s) in [(-5.0, 0.8), (4.0, 1.0)] {
                let px = x + dx;
                let r = 5.0 * s;
                g.ellipse(px + 1.0, y + 1.0, r + 2.0, 2.0, f::SHADOW30);
                g.ellipse(px, y - r * 0.8, r + 2.0, r * 0.8 + 1.0, 0xff6b2f10);
                g.ellipse(px, y - r * 0.8, r + 1.0, r * 0.8, 0xffe07b24);
                g.fill(px - 1.0, y - r * 1.6, 2.0, r * 1.6 - 1.0, 0xffc4631a);
                g.fill(px - r * 0.6, y - r * 1.2, 2.0, 3.0, 0xfff5a04a);
                g.fill(px - 1.0, y - r * 1.6 - 3.0, 2.0, 3.0, 0xff4f6b25);
            }
        }
        "log" => {
            g.ellipse(x + 1.0, y + 1.0, 13.0, 3.0, f::SHADOW30);
            g.fill(x - 12.0, y - 8.0, 22.0, 9.0, 0xff3a2616);
            g.fill(x - 11.0, y - 7.0, 20.0, 7.0, 0xff7a5433);
            g.fill(x - 11.0, y - 7.0, 20.0, 2.0, 0xff9a6d45);
            g.ellipse(x + 10.0, y - 4.0, 3.0, 4.0, 0xff3a2616);
            g.ellipse(x + 10.0, y - 4.0, 2.0, 3.0, 0xffd8b27a);
            g.fill(x + 10.0, y - 4.0, 1.0, 1.0, 0xff9a6d45);
        }
        "mushrooms" => {
            for (dx, s) in [(-3.0, 1.0), (3.0, 0.7), (0.0, 0.55)] {
                let px = x + dx;
                let r = 3.0 * s + 1.0;
                g.fill(px - 1.0, y - 4.0 * s, 2.0, 4.0 * s, 0xfff4ecd8);
                g.ellipse(px, y - 4.0 * s - 1.0, r + 1.0, r * 0.7 + 1.0, 0xff5c130e);
                g.ellipse(px, y - 4.0 * s - 1.0, r, r * 0.7, 0xffd6334a);
                g.fill(px - 1.0, y - 4.0 * s - 2.0, 1.0, 1.0, 0xffffffff);
                g.fill(px + 1.0, y - 4.0 * s - 1.0, 1.0, 1.0, 0xffffffff);
            }
        }
        _ => {}
    }
}
