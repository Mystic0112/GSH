// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! O escritório (office_world_paint.dart e office_world_props.dart): o mesmo layout
//! da fazenda, pintado como um prédio visto de cima. O campo vira a ala das salas,
//! o curral o lounge, o celeiro a copa e o lago uma mesa de reunião. Os monitores
//! que acendem com o trabalho não estão aqui: são do painter, em `paint.rs`.
#![allow(dead_code)] // ponytail: ligado ao cenário do escritório na integração

use super::dart_sort::sort_by;
use super::layout::{Decor, Layout, Rect};
use super::raster::mulberry32;
use super::raster::{Raster, HD};
use super::world_paint::{clear, dist};
use crate::theme::scene::{field as t, office as o, office_world as w};
use std::cmp::Ordering;

/// Sombra do escritório: rgba(40, 36, 30, .18).
const SHADOW: u32 = o::SHADOW;
/// Parede da face de cima e a linha de rodapé do prédio.
const FACE_H: f64 = 9.0;

/// Elemento `i` de uma paleta cíclica (`list[i % len]` do Dart), sem indexar.
fn cycle(list: &[u32], i: usize) -> u32 {
    list.get(i % list.len().max(1)).copied().unwrap_or(0)
}

/// Escurece ou clareia (`shade` do pix.dart), sempre opaca.
fn shade(c: u32, amt: f64) -> u32 {
    let f = |v: u32| (f64::from(v) + amt * 255.0).round().clamp(0.0, 255.0) as u32;
    0xff00_0000 | (f((c >> 16) & 0xff) << 16) | (f((c >> 8) & 0xff) << 8) | f(c & 0xff)
}

/// Pinta o miolo do escritório e devolve os obstáculos (`_officeProps`). Grava os
/// móveis em `l.decor`, que o bloqueio de caminhada consulta.
pub fn office_props(g: &mut Raster, l: &mut Layout, rnd: &mut dyn FnMut() -> f64) {
    let p = l.play;
    building(g, p, rnd);
    entrance(g, l.pond_cx(), p.bottom);
    for pts in l.paths.clone() {
        runner(g, &pts);
    }
    meeting(g, l);
    // móveis soltos no salão, longe do miolo, do corredor e da porta da copa
    const KINDS: [(&str, f64, f64); 7] = [
        ("plant", 0.36, 6.0),
        ("shelf", 0.18, 7.0),
        ("pouf", 0.14, 5.0),
        ("lamp", 0.12, 3.0),
        ("bin", 0.08, 3.0),
        ("cooler", 0.06, 4.0),
        ("sofa", 0.06, 10.0),
    ];
    l.decor.clear();
    let want = (((l.w * l.h) as f64 / 4200.0).round() as i64).clamp(6, 40);
    let mut tries = 0;
    while (l.decor.len() as i64) < want && tries < 900 {
        tries += 1;
        let x = p.left + 10.0 + rnd() * (p.width() - 20.0);
        let mut y = p.top + 12.0 + rnd() * (p.height() - 16.0);
        let mut pr = rnd();
        let mut kind = KINDS[0];
        for k in KINDS {
            if pr < k.1 {
                kind = k;
                break;
            }
            pr -= k.1;
        }
        // estante e sofá encostam na parede de cima
        if kind.0 == "shelf" || kind.0 == "sofa" {
            y = p.top + 13.0;
        }
        if !clear(l, x, y, 9.0) || l.decor.iter().any(|o| dist(o.x, o.y, x, y) < 26.0) {
            continue;
        }
        let c_idx = (rnd() * 6.0).floor() as usize;
        l.decor.push(Decor {
            kind: kind.0,
            x,
            y,
            r: kind.2,
            c: c_idx,
        });
    }
    let mut sorted = l.decor.clone();
    sort_by(&mut sorted, &|a, b| {
        a.y.partial_cmp(&b.y).unwrap_or(Ordering::Equal)
    });
    for d in &sorted {
        office_decor(g, d);
    }
    lounge(g, l);
    wing(g, l);
    copa(g, l.barn);
    // na placa da fazenda, um vaso grande ao lado da porta da ala
    let sign = Decor {
        kind: "plant",
        x: l.sign.dx,
        y: l.sign.dy + 13.0,
        r: 6.0,
        c: 1,
    };
    office_decor(g, &sign);
}

/// Parede horizontal: o topo escuro e, embaixo, a face clara de `face` px com rodapé.
fn wall_h(g: &mut Raster, x0: f64, x1: f64, y: f64, face: f64) {
    let hd = HD as f64;
    let (ax, width, ay) = ((x0) * hd, (x1 - x0) * hd, y * hd);
    if face > 0.0 {
        g.fill(ax, ay, width, face * hd, w::FACE);
        g.fill(ax, ay + face * hd - 2.0, width, 1.0, w::FACE2);
        g.fill(ax, ay + face * hd - 1.0, width, 1.0, w::BASE);
        g.fill(ax, ay + face * hd, width, 2.0, SHADOW);
    }
    g.fill(ax, ay - 3.0, width, 3.0, w::WL0);
    g.fill(ax, ay - 3.0, width, 1.0, w::WL1);
}

/// Parede vertical (só o topo escuro aparece de cima).
fn wall_v(g: &mut Raster, x: f64, y0: f64, y1: f64) {
    let hd = HD as f64;
    g.fill(
        x * hd - 1.0,
        y0 * hd - 3.0,
        3.0,
        (y1 - y0) * hd + 3.0,
        w::WL0,
    );
    g.fill(
        x * hd - 1.0,
        y0 * hd - 3.0,
        1.0,
        (y1 - y0) * hd + 3.0,
        w::WL1,
    );
    g.fill(x * hd + 2.0, y0 * hd, 2.0, (y1 - y0) * hd, SHADOW);
}

/// O prédio: piso de lajotas claras, a parede de cima com janelas e as paredes da volta.
fn building(g: &mut Raster, p: Rect, rnd: &mut dyn FnMut() -> f64) {
    let hd = HD as f64;
    let tile = 12.0;
    let mut ty = p.top;
    while ty < p.bottom {
        let mut tx = p.left;
        while tx < p.right {
            let k = ((tx - p.left) / tile).trunc() + ((ty - p.top) / tile).trunc();
            let tw = tile.min(p.right - tx);
            let th = tile.min(p.bottom - ty);
            let c = if (k as i64) % 2 == 0 { w::FL0 } else { w::FL1 };
            g.fill(tx * hd, ty * hd, tw * hd, th * hd, c);
            g.fill(tx * hd, ty * hd, tw * hd, 1.0, w::FL_LINE);
            g.fill(tx * hd, ty * hd, 1.0, th * hd, w::FL_LINE);
            tx += tile;
        }
        ty += tile;
    }
    let specks = p.width() * p.height() / 30.0;
    let mut i = 0;
    while (i as f64) < specks {
        let x = ((p.left + rnd() * p.width()) * hd).round();
        let y = ((p.top + rnd() * p.height()) * hd).round();
        g.fill(x, y, 1.0, 1.0, w::FL_LINE);
        i += 1;
    }
    // parede de cima com face alta, janelas e um relógio
    wall_h(g, p.left - 2.0, p.right + 2.0, p.top - FACE_H, FACE_H);
    let mut wx = p.left + 18.0;
    while wx < p.right - 30.0 {
        window(g, wx, p.top - FACE_H + 1.5);
        wx += 52.0;
    }
    clock(g, p.right - 14.0, p.top - FACE_H / 2.0);
    wall_v(g, p.left - 1.0, p.top - FACE_H, p.bottom + 1.0);
    wall_v(g, p.right + 1.0, p.top - FACE_H, p.bottom + 1.0);
    g.fill(
        (p.left - 2.0) * hd,
        p.bottom * hd,
        (p.width() + 4.0) * hd,
        4.0,
        w::WL0,
    );
    g.fill(
        (p.left - 2.0) * hd,
        p.bottom * hd,
        (p.width() + 4.0) * hd,
        1.0,
        w::WL1,
    );
}

/// Corredor: uma passadeira cinza-azulada sobre o piso.
fn runner(g: &mut Raster, pts: &[super::layout::Offset]) {
    let hd = HD as f64;
    for (pad, c) in [(7.0, 0xff9a_a8b8), (6.0, 0xffb4_c0cd), (4.0, 0xffbe_cad6)] {
        for seg in pts.windows(2) {
            if let [a, b] = seg {
                g.fill(
                    a.dx.min(b.dx) * hd - pad,
                    a.dy.min(b.dy) * hd - pad,
                    (a.dx - b.dx).abs() * hd + pad * 2.0,
                    (a.dy - b.dy).abs() * hd + pad * 2.0,
                    c,
                );
            }
        }
    }
}

/// No lugar do lago: tapete azul com a mesa de reunião e cadeiras.
fn meeting(g: &mut Raster, l: &Layout) {
    let hd = HD as f64;
    let (cx, cy, rx, ry) = (l.pond_cx(), l.pond_cy(), l.pond_rx(), l.pond_ry());
    let rug = Rect::from_ltrb(cx - rx - 6.0, cy - ry - 4.0, cx + rx + 6.0, cy + ry + 6.0);
    rug_fill(g, rug, 0xff5d_7fae, 0xff4a_6894, 0xff7f_9dc6);
    let tl = cx - rx + 5.0;
    let tr = cx + rx - 5.0;
    // cadeiras em cima e embaixo da mesa
    let mut x = tl + 4.0;
    while x <= tr - 3.0 {
        chair(g, x, cy - ry + 1.0, true);
        chair(g, x, cy + ry + 2.0, false);
        x += 9.0;
    }
    let t = Rect::from_ltrb(tl, cy - ry + 3.0, tr, cy + ry - 1.0);
    g.fill(
        t.left * hd + 2.0,
        t.bottom * hd,
        t.width() * hd,
        3.0,
        SHADOW,
    );
    g.fill(
        t.left * hd + 2.0,
        t.top * hd,
        t.width() * hd - 4.0,
        t.height() * hd,
        w::WD_EDGE,
    );
    g.fill(
        t.left * hd,
        t.top * hd + 2.0,
        t.width() * hd,
        t.height() * hd - 4.0,
        w::WD_EDGE,
    );
    g.fill(
        t.left * hd + 2.0,
        t.top * hd + 1.0,
        t.width() * hd - 4.0,
        t.height() * hd - 4.0,
        w::WD0,
    );
    g.fill(
        t.left * hd + 1.0,
        t.top * hd + 3.0,
        t.width() * hd - 2.0,
        t.height() * hd - 7.0,
        w::WD0,
    );
    g.fill(
        t.left * hd + 2.0,
        t.bottom * hd - 4.0,
        t.width() * hd - 4.0,
        3.0,
        w::WD2,
    );
    g.fill(
        t.left * hd + 4.0,
        t.top * hd + 2.0,
        t.width() * hd - 8.0,
        1.0,
        0xffdc_b27c,
    );
    // papéis e um notebook em cima
    let mx = cx * hd;
    g.fill(mx - 14.0, cy * hd - 6.0, 7.0, 5.0, 0xfff7_f5ee);
    g.fill(mx - 13.0, cy * hd - 5.0, 4.0, 1.0, 0xffb9_bec6);
    g.fill(mx + 6.0, cy * hd - 7.0, 9.0, 6.0, 0xff2f_3542);
    g.fill(mx + 7.0, cy * hd - 6.0, 7.0, 3.0, 0xff6f_a8dc);
}

/// Tapete retangular com borda e um filete interno.
fn rug_fill(g: &mut Raster, r: Rect, inner: u32, edge: u32, line: u32) {
    let hd = HD as f64;
    let (x, y, width, height) = (r.left * hd, r.top * hd, r.width() * hd, r.height() * hd);
    g.fill(x + 2.0, y + height, width, 2.0, SHADOW);
    g.fill(x, y, width, height, edge);
    g.fill(x + 1.0, y + 1.0, width - 2.0, height - 2.0, inner);
    g.fill(x + 4.0, y + 4.0, width - 8.0, 1.0, line);
    g.fill(x + 4.0, y + height - 5.0, width - 8.0, 1.0, line);
    g.fill(x + 4.0, y + 4.0, 1.0, height - 8.0, line);
    g.fill(x + width - 5.0, y + 4.0, 1.0, height - 8.0, line);
}

/// No lugar do curral: o lounge de piso de madeira, tapete verde e um pufe em cada vaga.
fn lounge(g: &mut Raster, l: &Layout) {
    let hd = HD as f64;
    let y = l.yard;
    let mut py = y.top;
    let mut i: i64 = 0;
    while py < y.bottom {
        let h = 3.0_f64.min(y.bottom - py);
        let c = if i % 2 == 1 { w::WD0 } else { w::WD1 };
        g.fill(y.left * hd, py * hd, y.width() * hd, h * hd, c);
        g.fill(y.left * hd, py * hd, y.width() * hd, 1.0, w::WD2);
        let mut px = y.left + ((i * 13) % 23) as f64;
        while px < y.right {
            g.fill(px * hd, py * hd, 1.0, h * hd, w::WD2);
            px += 23.0;
        }
        py += 3.0;
        i += 1;
    }
    rug_fill(
        g,
        Rect::from_ltrb(y.left + 3.0, y.top + 6.0, y.right - 3.0, y.bottom - 2.0),
        0xff6a_a06d,
        0xff4d_7f53,
        0xff8c_c08a,
    );
    const COLORS: [u32; 5] = [
        0xffe0_533f,
        0xfff2_b43a,
        0xff4f_86d9,
        0xff6f_bf5a,
        0xffe5_7fb0,
    ];
    let cap = l.yard_cols * ((y.height() - 10.0) / 17.0).round() as i64;
    for i in 0..cap {
        let s = l.yard_spot(i);
        let c = cycle(&COLORS, i as usize);
        pouf(g, s.dx * hd, (s.dy - 2.0) * hd, c);
    }
    // paredes com a porta no meio de cima e um vaso no canto de fora
    let gx0 = y.center().dx - 8.0;
    let gx1 = y.center().dx + 8.0;
    wall_h(g, y.left - 2.0, gx0, y.top, 2.0);
    wall_h(g, gx1, y.right + 2.0, y.top, 2.0);
    wall_v(g, y.left - 2.0, y.top, y.bottom);
    wall_v(g, y.right + 2.0, y.top, y.bottom);
    wall_h(g, y.left - 2.0, y.right + 2.0, y.bottom, 0.0);
    plant(g, (y.right + 7.0) * hd, (y.bottom - 1.0) * hd, 0, true);
}

/// No lugar do cercado: a ala das salas de projeto, de carpete, com a porta à
/// esquerda e uma ilha de mesas por canteiro.
fn wing(g: &mut Raster, l: &Layout) {
    let hd = HD as f64;
    let f = l.field;
    g.fill(
        f.left * hd,
        f.top * hd,
        f.width() * hd,
        f.height() * hd,
        w::CP0,
    );
    let n = f.width() * f.height() / 6.0;
    let mut i: i64 = 0;
    while (i as f64) < n {
        let x = f.left + ((i * 7919) % 1000) as f64 / 1000.0 * f.width();
        let y = f.top + ((i * 104_729) % 997) as f64 / 997.0 * f.height();
        let c = if i % 2 == 1 { w::CP1 } else { w::CP2 };
        g.fill((x * hd).round(), (y * hd).round(), 1.0, 1.0, c);
        i += 1;
    }
    for (idx, plot) in l.plots.iter().enumerate() {
        island(g, *plot, idx as i64);
    }
    // estante no canto de cima do corredor da impressora, vasos nos cantos
    if l.gate_at - f.top > 32.0 {
        shelf(g, (f.left + 9.0) * hd, (f.top + 14.0) * hd, 2);
    }
    plant(g, (f.right - 7.0) * hd, (f.top + 11.0) * hd, 3, false);
    plant(g, (f.right - 7.0) * hd, (f.bottom - 2.0) * hd, 4, true);
    let g0 = l.gate_at - 8.0;
    let g1 = l.gate_at + 8.0;
    wall_h(g, f.left - 1.0, f.right + 1.0, f.top, 3.0);
    wall_v(g, f.left, f.top, g0);
    wall_v(g, f.left, g1, f.bottom);
    wall_v(g, f.right, f.top, f.bottom);
    wall_h(g, f.left - 1.0, f.right + 1.0, f.bottom, 0.0);
    // batentes da porta
    for y in [g0, g1] {
        g.fill(f.left * hd - 2.0, y * hd - 1.0, 5.0, 2.0, 0xffe9_edf2);
    }
}

/// Uma ilha: tapete da sala, cadeiras onde sentam os minis e duas fileiras de mesas.
fn island(g: &mut Raster, p: Rect, i: i64) {
    let hd = HD as f64;
    rug_fill(
        g,
        Rect::from_ltrb(p.left - 13.0, p.top - 12.0, p.right + 11.0, p.bottom + 10.0),
        w::RUG0,
        w::RUG_EDGE,
        0xffcd_d5de,
    );
    for k in 0..3 {
        let kf = f64::from(k);
        chair(g, p.left + 5.0 + kf * 10.0, p.top - 5.0, true);
        chair(g, p.left + 5.0 + kf * 10.0, p.bottom + 5.0, false);
    }
    let x = p.left * hd;
    let width = p.width() * hd;
    g.fill(x + 2.0, p.bottom * hd, width, 3.0, SHADOW);
    for row in 0..2_i64 {
        let y = (p.top + row as f64 * 8.0) * hd;
        g.fill(x - 1.0, y - 1.0, width + 2.0, 18.0, 0xff8f_97a3);
        g.fill(x, y, width, 12.0, 0xfff7_f7f4);
        g.fill(x, y, width, 1.0, 0xffff_ffff);
        g.fill(x, y + 12.0, width, 4.0, 0xffdd_e1e6);
        g.fill(x, y + 15.0, width, 1.0, 0xffb5_bcc6);
        for k in 0..3 {
            let dx = x + f64::from(k) * 20.0;
            if k > 0 {
                g.fill(dx, y, 1.0, 16.0, 0xffc9_ced6);
            }
            // teclado e mouse
            g.fill(dx + 5.0, y + 9.0, 9.0, 2.0, 0xffc3_c9d2);
            g.fill(dx + 6.0, y + 9.0, 7.0, 1.0, 0xffdf_e3e8);
            g.fill(dx + 16.0, y + 9.0, 2.0, 2.0, 0xff9a_a3b2);
        }
        // uma caneca ou papéis em mesas sorteadas pela posição
        let pick = (i * 3 + row * 5) % 4;
        let px = x + (pick.clamp(0, 2) as f64) * 20.0 + 15.0;
        if pick == 0 {
            g.fill(px, y + 2.0, 3.0, 3.0, 0xffd9_4b3c);
            g.fill(px, y + 2.0, 3.0, 1.0, 0xff6b_2018);
        } else if pick == 3 {
            g.fill(px - 1.0, y + 2.0, 5.0, 4.0, 0xfff7_f5ee);
            g.fill(px - 1.0, y + 2.0, 5.0, 4.0, 0xffdf_e3e8);
            g.fill(px, y + 3.0, 3.0, 1.0, 0xff9a_a3b2);
        }
    }
}

/// No lugar do celeiro: a copa, com piso xadrez, bancada, geladeira e uma mesinha.
fn copa(g: &mut Raster, b: Rect) {
    let hd = HD as f64;
    const FACE: f64 = 8.0;
    g.ellipse(
        (b.center().dx + 3.0) * hd,
        (b.bottom + 1.0) * hd,
        b.width() * hd / 2.0 + 8.0,
        6.0,
        SHADOW,
    );
    let mut ty = b.top;
    while ty < b.bottom {
        let mut tx = b.left;
        while tx < b.right {
            let k = ((tx - b.left) / 4.0).trunc() + ((ty - b.top) / 4.0).trunc();
            let c = if (k as i64) % 2 == 0 {
                0xffee_f3f8
            } else {
                0xffc9_d7e6
            };
            g.fill(tx * hd, ty * hd, 8.0, 8.0, c);
            tx += 4.0;
        }
        ty += 4.0;
    }
    wall_h(g, b.left - 1.0, b.right + 1.0, b.top, FACE);
    clock(g, b.left + 9.0, b.top + 4.0);
    // bancada com pia e cafeteira
    let cy = (b.top + FACE) * hd;
    let cx0 = (b.left + 1.0) * hd;
    let cw = (b.width() - 14.0) * hd;
    g.fill(cx0, cy, cw, 14.0, w::WD_EDGE);
    g.fill(cx0, cy, cw, 6.0, 0xffe9_e4da);
    g.fill(cx0, cy + 6.0, cw, 7.0, w::WD1);
    g.fill(cx0, cy + 6.0, cw, 1.0, w::WD2);
    let mut x = cx0 + 10.0;
    while x < cx0 + cw {
        g.fill(x, cy + 7.0, 1.0, 6.0, w::WD2);
        g.fill(x - 3.0, cy + 9.0, 2.0, 1.0, 0xff6e_4a2a);
        x += 12.0;
    }
    g.fill(cx0 + 14.0, cy + 1.0, 12.0, 4.0, 0xffa5_adb8);
    g.fill(cx0 + 15.0, cy + 2.0, 10.0, 2.0, 0xff7f_8894);
    g.fill(cx0 + 19.0, cy - 2.0, 2.0, 3.0, 0xff8f_97a3);
    g.fill(cx0 + 36.0, cy - 6.0, 9.0, 10.0, 0xff2b_2f3a);
    g.fill(cx0 + 37.0, cy - 5.0, 7.0, 3.0, 0xff4a_505d);
    g.fill(cx0 + 42.0, cy - 1.0, 2.0, 1.0, 0xffe0_533f);
    g.fill(cx0 + 39.0, cy + 1.0, 3.0, 3.0, 0xfff7_f5ee);
    g.fill(cx0 + 52.0, cy - 3.0, 8.0, 6.0, 0xffe9_e4da);
    g.fill(cx0 + 53.0, cy - 2.0, 6.0, 3.0, 0xff3a_404c);
    // geladeira no canto direito
    let fx = (b.right - 12.0) * hd;
    let fy = (b.top + 2.0) * hd;
    g.fill(fx + 2.0, fy + 38.0, 22.0, 3.0, SHADOW);
    g.fill(fx, fy, 22.0, 38.0, 0xff7d_8592);
    g.fill(fx + 1.0, fy + 1.0, 20.0, 36.0, 0xfff2_f4f6);
    g.fill(fx + 1.0, fy + 1.0, 20.0, 3.0, 0xffff_ffff);
    g.fill(fx + 1.0, fy + 14.0, 20.0, 1.0, 0xffb9_bec6);
    g.fill(fx + 3.0, fy + 7.0, 1.0, 5.0, 0xff9a_a3b2);
    g.fill(fx + 3.0, fy + 18.0, 1.0, 8.0, 0xff9a_a3b2);
    g.fill(fx + 18.0, fy + 1.0, 3.0, 36.0, 0xffd8_dde3);
    // mesinha redonda com dois banquinhos
    let tx = (b.left + 14.0) * hd;
    let ty = (b.top + 31.0) * hd;
    g.ellipse(tx - 12.0, ty, 4.0, 3.0, w::WD_EDGE);
    g.ellipse(tx - 12.0, ty - 1.0, 3.0, 2.0, w::WD0);
    g.ellipse(tx + 12.0, ty, 4.0, 3.0, w::WD_EDGE);
    g.ellipse(tx + 12.0, ty - 1.0, 3.0, 2.0, w::WD0);
    g.ellipse(tx + 1.0, ty + 3.0, 9.0, 4.0, SHADOW);
    g.ellipse(tx, ty, 9.0, 6.0, 0xff8f_97a3);
    g.ellipse(tx, ty - 1.0, 8.0, 5.0, 0xfff7_f7f4);
    g.fill(tx - 3.0, ty - 3.0, 3.0, 3.0, 0xffd9_4b3c);
    plant(g, (b.right - 5.0) * hd, (b.bottom - 2.0) * hd, 2, false);
    let d0 = b.center().dx - 9.0;
    let d1 = b.center().dx + 9.0;
    wall_v(g, b.left - 1.0, b.top, b.bottom);
    wall_v(g, b.right + 1.0, b.top, b.bottom);
    wall_h(g, b.left - 1.0, d0, b.bottom, 0.0);
    wall_h(g, d1, b.right + 1.0, b.bottom, 0.0);
    // capacho na porta
    g.fill(
        d0 * hd + 4.0,
        b.bottom * hd - 5.0,
        (d1 - d0) * hd - 8.0,
        4.0,
        0xff8a_6a4a,
    );
}

/// Móvel do decor sorteado, ou a planta da placa (`_officeDecor`).
fn office_decor(g: &mut Raster, d: &Decor) {
    let hd = HD as f64;
    let x = (d.x * hd).round();
    let y = (d.y * hd).round();
    match d.kind {
        "plant" => plant(g, x, y, d.c, d.c.is_multiple_of(2)),
        "shelf" => shelf(g, x, y, d.c as i64),
        "pouf" => pouf(g, x, y - 4.0, cycle(&w::POUF, d.c)),
        "lamp" => lamp(g, x, y),
        "bin" => {
            g.ellipse(x + 1.0, y, 5.0, 2.0, SHADOW);
            g.fill(x - 4.0, y - 9.0, 8.0, 9.0, 0xff5f_6773);
            g.fill(x - 3.0, y - 9.0, 6.0, 8.0, 0xff8a_929e);
            g.fill(x - 3.0, y - 9.0, 2.0, 8.0, 0xffa3_abb6);
            g.fill(x - 4.0, y - 10.0, 8.0, 2.0, 0xff5f_6773);
            g.fill(x - 3.0, y - 10.0, 6.0, 1.0, 0xff3a_404c);
        }
        "cooler" => {
            g.ellipse(x + 1.0, y, 7.0, 2.0, SHADOW);
            g.fill(x - 5.0, y - 12.0, 10.0, 12.0, 0xff7d_8592);
            g.fill(x - 4.0, y - 12.0, 8.0, 11.0, 0xfff2_f4f6);
            g.fill(x - 2.0, y - 8.0, 1.0, 2.0, 0xff4f_86d9);
            g.fill(x + 1.0, y - 8.0, 1.0, 2.0, 0xffd9_4b3c);
            g.ellipse(x, y - 17.0, 5.0, 6.0, 0xff4f_86b8);
            g.ellipse(x, y - 17.0, 4.0, 5.0, 0xff8f_cbef);
            g.fill(x - 2.0, y - 21.0, 1.0, 5.0, 0xffd8_f0fb);
        }
        "sofa" => sofa(g, x, y, cycle(&w::POUF, d.c + 3)),
        _ => {}
    }
}

/// Vaso: folhas redondas (`white` ou `c` par) ou folhas compridas para cima.
fn plant(g: &mut Raster, x: f64, y: f64, c: usize, white: bool) {
    let pot = if white { 0xfff2_f1ec } else { 0xffc8_693e };
    let pot_s = if white { 0xffcf_d0cb } else { 0xff9c_4c2a };
    g.ellipse(x + 1.0, y, 8.0, 2.0, SHADOW);
    g.fill(x - 6.0, y - 10.0, 12.0, 10.0, 0xff5c_4636);
    g.fill(x - 5.0, y - 9.0, 10.0, 9.0, pot);
    g.fill(x + 2.0, y - 9.0, 3.0, 9.0, pot_s);
    g.fill(x - 6.0, y - 11.0, 12.0, 3.0, 0xff5c_4636);
    g.fill(
        x - 5.0,
        y - 11.0,
        10.0,
        2.0,
        if white { 0xffff_ffff } else { 0xffdd_8458 },
    );
    if c.is_multiple_of(2) {
        let blobs = [
            (-6.0, -16.0, 6.0),
            (6.0, -16.0, 6.0),
            (0.0, -21.0, 7.0),
            (0.0, -14.0, 6.0),
        ];
        for (dx, dy, r) in blobs {
            g.circle(x + dx, y + dy, r + 1.0, t::T0);
        }
        for (dx, dy, r) in blobs {
            g.circle(x + dx, y + dy, r, t::T2);
            g.circle(x + dx - 1.0, y + dy - 2.0, r * 0.55, t::T3);
        }
        g.fill(x - 3.0, y - 25.0, 2.0, 1.0, t::T4);
        g.fill(x + 4.0, y - 19.0, 2.0, 1.0, t::T4);
    } else {
        // espada-de-são-jorge: folhas em pé, contornadas
        for (dx, top) in [(-5.0, 22.0), (-2.0, 28.0), (1.0, 25.0), (4.0, 20.0)] {
            g.fill(x + dx - 1.0, y - 9.0 - top, 4.0, top, t::T0);
            g.fill(x + dx, y - 8.0 - top, 2.0, top - 1.0, t::T3);
            g.fill(x + dx, y - 8.0 - top, 1.0, top - 1.0, t::T4);
            g.fill(x + dx + 1.0, y - 18.0, 1.0, 6.0, 0xffb9_d46a);
        }
    }
}

/// Estante branca com livros coloridos (3 prateleiras).
fn shelf(g: &mut Raster, x: f64, y: f64, c: i64) {
    const WIDTH: f64 = 22.0;
    const HEIGHT: f64 = 26.0;
    let x0 = x - 11.0;
    let y0 = y - HEIGHT;
    g.fill(x0 + 2.0, y, WIDTH, 2.0, SHADOW);
    g.fill(x0, y0, WIDTH, HEIGHT, 0xff7d_8592);
    g.fill(x0 + 1.0, y0 + 1.0, WIDTH - 2.0, HEIGHT - 2.0, 0xfff2_f0ea);
    g.fill(x0 + 1.0, y0 + 1.0, WIDTH - 2.0, 2.0, 0xffff_ffff);
    for row in 0..3 {
        let ry = y0 + 4.0 + f64::from(row) * 7.0;
        g.fill(x0 + 2.0, ry, WIDTH - 4.0, 6.0, 0xff9a_8d7a);
        let mut bx = x0 + 2.0;
        let mut k = c + i64::from(row) * 3;
        while bx < x0 + WIDTH - 3.0 {
            let bw = 2.0 + (k % 2) as f64;
            let bh = 4.0 + ((k * 7) % 3) as f64;
            if (k * 5) % 7 != 0 {
                let book = cycle(&w::BOOKS, k as usize);
                g.fill(bx, ry + 6.0 - bh, bw, bh, book);
                g.fill(bx, ry + 6.0 - bh, 1.0, bh, 0x33ff_ffff);
            }
            bx += bw;
            k += 1;
        }
        g.fill(x0 + 1.0, ry + 6.0, WIDTH - 2.0, 1.0, 0xffd8_d4cb);
    }
}

/// Pufe redondo, visto de cima, centrado em (x, y) na grade HD.
fn pouf(g: &mut Raster, x: f64, y: f64, c: u32) {
    g.ellipse(x + 2.0, y + 6.0, 10.0, 3.0, SHADOW);
    g.ellipse(x, y, 11.0, 8.0, shade(c, -0.32));
    g.ellipse(x, y, 10.0, 7.0, c);
    g.ellipse(x - 2.0, y - 2.0, 6.0, 3.0, shade(c, 0.1));
    g.fill(x - 4.0, y - 4.0, 3.0, 1.0, shade(c, 0.22));
}

/// Luminária de chão: base, haste e cúpula com luz quente.
fn lamp(g: &mut Raster, x: f64, y: f64) {
    g.ellipse(x + 1.0, y, 6.0, 2.0, SHADOW);
    g.ellipse(x, y - 1.0, 4.0, 1.0, 0xff3a_404c);
    g.fill(x - 1.0, y - 26.0, 2.0, 25.0, 0xff4b_5261);
    g.ellipse(x, y - 30.0, 12.0, 7.0, w::LAMP_GLOW);
    g.fill(x - 6.0, y - 34.0, 12.0, 8.0, 0xff8d_7f66);
    g.fill(x - 5.0, y - 34.0, 10.0, 7.0, 0xfff3_ead8);
    g.fill(x - 5.0, y - 34.0, 10.0, 2.0, 0xfffffaf0);
    g.fill(x - 4.0, y - 27.0, 8.0, 1.0, 0xffff_e29a);
}

/// Sofá de dois lugares encostado na parede.
fn sofa(g: &mut Raster, x: f64, y: f64, c: u32) {
    let d = shade(c, -0.3);
    let l = shade(c, 0.12);
    g.fill(x - 15.0, y, 32.0, 3.0, SHADOW);
    g.fill(x - 16.0, y - 18.0, 32.0, 18.0, d);
    g.fill(x - 15.0, y - 17.0, 30.0, 7.0, c);
    g.fill(x - 15.0, y - 17.0, 30.0, 2.0, l);
    g.fill(x - 15.0, y - 9.0, 4.0, 8.0, c);
    g.fill(x + 11.0, y - 9.0, 4.0, 8.0, c);
    g.fill(x - 11.0, y - 10.0, 11.0, 8.0, l);
    g.fill(x, y - 10.0, 11.0, 8.0, l);
    g.fill(x - 1.0, y - 10.0, 1.0, 8.0, d);
}

/// Cadeira de escritório azul vista de cima, centrada em (x, y) no mundo; `up` = encosto em cima.
fn chair(g: &mut Raster, x: f64, y: f64, up: bool) {
    let hd = HD as f64;
    let cx = (x * hd).round();
    let cy = (y * hd).round();
    g.fill(cx - 4.0, cy + 4.0, 10.0, 2.0, SHADOW);
    g.fill(cx - 5.0, cy - 4.0, 10.0, 9.0, 0xff26_3f73);
    g.fill(cx - 4.0, cy - 3.0, 8.0, 7.0, 0xff50_79c4);
    g.fill(cx - 3.0, cy - 2.0, 4.0, 2.0, 0xff6d_93d8);
    let by = if up { cy - 6.0 } else { cy + 3.0 };
    g.fill(cx - 5.0, by, 10.0, 4.0, 0xff26_3f73);
    g.fill(cx - 4.0, by + 1.0, 8.0, 2.0, 0xff3b_5fa8);
}

/// Janela na face da parede de cima (x, y = canto, no mundo).
fn window(g: &mut Raster, x: f64, y: f64) {
    let hd = HD as f64;
    let (wx, wy) = (x * hd, y * hd);
    g.fill(wx, wy, 36.0, 12.0, 0xff7d_8592);
    g.fill(wx + 1.0, wy + 1.0, 34.0, 10.0, 0xfff2_f4f6);
    g.fill(wx + 2.0, wy + 2.0, 15.0, 8.0, 0xff9f_d0ee);
    g.fill(wx + 19.0, wy + 2.0, 15.0, 8.0, 0xff9f_d0ee);
    g.fill(wx + 4.0, wy + 3.0, 2.0, 1.0, 0xffe6_f5fd);
    g.fill(wx + 3.0, wy + 4.0, 2.0, 1.0, 0xffe6_f5fd);
    g.fill(wx + 21.0, wy + 3.0, 2.0, 1.0, 0xffe6_f5fd);
    g.fill(wx + 2.0, wy + 7.0, 15.0, 3.0, 0xff7f_bde3);
    g.fill(wx + 19.0, wy + 7.0, 15.0, 3.0, 0xff7f_bde3);
}

/// Relógio de parede (x, y = centro, no mundo).
fn clock(g: &mut Raster, x: f64, y: f64) {
    let hd = HD as f64;
    let (cx, cy) = (x * hd, y * hd);
    g.circle(cx, cy, 5.0, 0xff3a_404c);
    g.circle(cx, cy, 4.0, 0xffff_ffff);
    g.fill(cx, cy - 3.0, 1.0, 3.0, 0xff3a_404c);
    g.fill(cx, cy, 3.0, 1.0, 0xffd9_4b3c);
}

/// O entorno urbano: calçada de placas em volta do prédio, floreiras na frente da
/// parede de cima, e embaixo meio-fio, rua com faixa tracejada e postes.
pub fn street(g: &mut Raster, l: &Layout, rnd: &mut dyn FnMut() -> f64) {
    let hd = HD as f64;
    let width = l.w as f64 * hd;
    let height = l.h as f64 * hd;
    let p = l.play;
    g.fill(0.0, 0.0, width, height, w::WALK0);
    let mut y = 0.0;
    while y < height {
        g.fill(0.0, y, width, 1.0, w::WALK1);
        y += 20.0;
    }
    let mut x = 0.0;
    while x < width {
        g.fill(x, 0.0, 1.0, height, w::WALK1);
        x += 20.0;
    }
    // rua: do meio-fio até o fim do mundo embaixo
    let curb = (p.bottom + 12.0) * hd;
    g.fill(0.0, curb, width, height - curb, w::ASPH0);
    g.fill(0.0, curb, width, 2.0, 0xffe6_e7ea);
    g.fill(0.0, curb + 2.0, width, 2.0, 0xff8d_9097);
    let specks = width * (height - curb) / 14.0;
    let mut i = 0;
    while (i as f64) < specks {
        let x = (rnd() * width).round();
        let y = (curb + 4.0 + rnd() * (height - curb - 4.0)).round();
        let c = if rnd() < 0.5 { w::ASPH1 } else { w::ASPH2 };
        g.fill(x, y, 1.0, 1.0, c);
        i += 1;
    }
    let mid = ((curb + 4.0 + height) / 2.0).round();
    if height - curb > 16.0 {
        let mut x = 4.0;
        while x < width {
            g.fill(x, mid - 1.0, 12.0, 2.0, 0xffe8_d27a);
            x += 24.0;
        }
    }
    // sombra do prédio na calçada (direita e embaixo)
    g.fill(
        (p.right + 2.0) * hd,
        (p.top - 7.0) * hd,
        4.0,
        (p.height() + 9.0) * hd,
        SHADOW,
    );
    g.fill(
        (p.left - 1.0) * hd,
        (p.bottom + 2.0) * hd - 2.0,
        (p.width() + 3.0) * hd,
        4.0,
        SHADOW,
    );
    // postes na calçada de baixo, floreiras na de cima
    let mut x = p.left + 24.0;
    while x < p.right - 8.0 {
        street_lamp(g, x * hd, (p.bottom + 11.0) * hd);
        x += 56.0;
    }
    let top = (p.top - 10.0) * hd;
    if top > 14.0 {
        let mut x = p.left + 10.0;
        while x < p.right - 20.0 {
            planter(g, x * hd, top - 2.0);
            x += 44.0;
        }
    }
}

/// Poste de rua: base, haste e luminária com um halo quente no chão.
fn street_lamp(g: &mut Raster, x: f64, y: f64) {
    g.ellipse(x + 2.0, y, 5.0, 2.0, SHADOW);
    g.fill(x - 2.0, y - 3.0, 4.0, 3.0, 0xff3a_404c);
    g.fill(x - 1.0, y - 30.0, 2.0, 28.0, 0xff4b_5261);
    g.fill(x - 1.0, y - 31.0, 9.0, 2.0, 0xff4b_5261);
    g.fill(x + 5.0, y - 30.0, 6.0, 3.0, 0xff3a_404c);
    g.fill(x + 6.0, y - 28.0, 4.0, 1.0, 0xffff_e29a);
}

/// Floreira de calçada: caixa de concreto com arbusto.
fn planter(g: &mut Raster, x: f64, y: f64) {
    g.fill(x + 2.0, y, 24.0, 2.0, SHADOW);
    g.fill(x, y - 7.0, 24.0, 7.0, 0xff8d_9097);
    g.fill(x + 1.0, y - 6.0, 22.0, 5.0, 0xffa9_acb2);
    g.fill(x + 1.0, y - 7.0, 22.0, 1.0, 0xffd5_d7db);
    for (dx, r) in [(5.0, 5.0), (12.0, 6.0), (19.0, 5.0)] {
        g.circle(x + dx, y - 9.0, r + 1.0, t::T0);
    }
    for (dx, r) in [(5.0, 5.0), (12.0, 6.0), (19.0, 5.0)] {
        g.circle(x + dx, y - 9.0, r, t::T2);
        g.circle(x + dx - 1.0, y - 11.0, r * 0.5, t::T3);
    }
}

/// Porta de vidro na parede de baixo (a entrada pela calçada).
fn entrance(g: &mut Raster, x: f64, y: f64) {
    let hd = HD as f64;
    let (dx, dy) = (x * hd, y * hd);
    g.fill(dx - 18.0, dy - 1.0, 36.0, 6.0, 0xff7d_8592);
    g.fill(dx - 16.0, dy, 15.0, 4.0, 0xff9f_d0ee);
    g.fill(dx + 1.0, dy, 15.0, 4.0, 0xff9f_d0ee);
    g.fill(dx - 14.0, dy + 1.0, 3.0, 1.0, 0xffe6_f5fd);
    g.fill(dx + 3.0, dy + 1.0, 3.0, 1.0, 0xffe6_f5fd);
    g.fill(dx - 12.0, dy + 6.0, 24.0, 4.0, 0xff8a_6a4a);
}

/// O fundo do escritório (`_paintWorld`, ramo do escritório): a rua e o prédio. Não
/// tem faixa da frente nem pedras.
pub fn paint_office(l: &mut Layout, seed: u32) -> Raster {
    let mut rnd = mulberry32(seed);
    let mut g = Raster::new(l.w * HD, l.h * HD);
    street(&mut g, l, &mut rnd);
    office_props(&mut g, l, &mut rnd);
    g
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::farm::layout::{layout_farm, Viewport};

    fn fnv(p: &[u32]) -> u32 {
        p.iter()
            .fold(2_166_136_261u32, |x, &c| (x ^ c).wrapping_mul(16_777_619))
    }

    /// Golden do harness Dart (golden/office.dart): fundo e móveis, nas três configurações.
    #[test]
    fn escritorio_igual_ao_dart() {
        let mut a = layout_farm(
            6,
            5,
            &Viewport {
                w: 800.0,
                h: 600.0,
                ..Viewport::default()
            },
            |z| z,
            None,
        );
        let g = paint_office(&mut a, 7);
        assert_eq!((a.w, a.h, fnv(&g.p)), (400, 300, 877542553));
        assert_eq!(a.decor.len(), 25);
        let decor: Vec<(&str, f64, f64, f64, usize)> = a
            .decor
            .iter()
            .map(|d| (d.kind, d.x, d.y, d.r, d.c))
            .collect();
        assert_eq!(
            decor,
            vec![
                ("pouf", 165.37420595996082, 75.18826460838318, 5.0, 4),
                ("plant", 210.69153966009617, 61.11101675033569, 6.0, 3),
                ("pouf", 267.5669867377728, 72.93123015761375, 5.0, 3),
                ("shelf", 82.47848123684525, 61.0, 7.0, 1),
                ("lamp", 132.55316185206175, 228.4174633473158, 3.0, 3),
                ("lamp", 98.75186923705041, 246.84634298086166, 3.0, 4),
                ("pouf", 57.509357146918774, 104.71461801230907, 5.0, 1),
                ("plant", 342.9208857472986, 65.83593307435513, 6.0, 3),
                ("plant", 345.51764088124037, 200.97352288663387, 6.0, 0),
                ("shelf", 111.71381365507841, 61.0, 7.0, 2),
                ("plant", 59.37670621462166, 251.99992409348488, 6.0, 5),
                ("bin", 199.85615496523678, 238.2246399074793, 3.0, 4),
                ("sofa", 310.6997052654624, 61.0, 10.0, 2),
                ("cooler", 321.15989693813026, 234.76226399838924, 4.0, 0),
                ("plant", 340.2136800084263, 125.13032904267311, 6.0, 3),
                ("plant", 58.99339060112834, 160.64457195997238, 6.0, 0),
                ("shelf", 242.87468009628356, 61.0, 7.0, 2),
                ("plant", 274.7535846605897, 234.8944281488657, 6.0, 0),
                ("pouf", 243.17911010980606, 229.10813480615616, 5.0, 5),
                ("pouf", 168.74783296510577, 236.71671058237553, 5.0, 4),
                ("shelf", 142.58771930634975, 61.0, 7.0, 3),
                ("plant", 63.537912998348475, 225.85693953931332, 6.0, 5),
                ("shelf", 52.48444328457117, 61.0, 7.0, 0),
                ("pouf", 337.2696310468018, 174.9845211803913, 5.0, 4),
                ("pouf", 347.36092472448945, 238.58998560905457, 5.0, 0)
            ]
        );

        let mut b = layout_farm(
            12,
            9,
            &Viewport {
                w: 1000.0,
                h: 700.0,
                left: 60.0,
                bottom: 30.0,
                ..Viewport::default()
            },
            |z| (z * 8.0).floor() / 8.0,
            None,
        );
        let g = paint_office(&mut b, 7);
        assert_eq!(
            (b.w, b.h, fnv(&g.p), b.decor.len()),
            (400, 260, 542310129, 0)
        );

        let mut c = layout_farm(
            20,
            14,
            &Viewport {
                w: 1280.0,
                h: 800.0,
                top: 40.0,
                right: 300.0,
                ..Viewport::default()
            },
            f64::round,
            Some(30),
        );
        let g = paint_office(&mut c, 7);
        assert_eq!(
            (c.w, c.h, fnv(&g.p), c.decor.len()),
            (454, 300, 3003508576, 0)
        );
    }
}
