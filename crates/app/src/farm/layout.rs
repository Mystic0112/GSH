// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! Layout da fazenda (farm_world.dart): cercado com canteiros, celeiro com curral,
//! lago e caminhos. Geometria pura; `layout_farm` escolhe o arranjo que mostra a
//! fazenda maior na janela.
#![allow(dead_code)] // ponytail: ligado ao cenário numa rodada seguinte

/// Canteiro (w x h) e o passo entre eles (cx x cy). O cy é maior que o do
/// clodfarm (40) para caber a etiqueta com o título embaixo de cada canteiro.
pub const PLOT_W: f64 = 30.0;
pub const PLOT_H: f64 = 16.0;
pub const PLOT_CX: f64 = 60.0;
pub const PLOT_CY: f64 = 46.0;
const YARD_CX: f64 = 19.0;
const YARD_CY: f64 = 17.0;
const CROW_LANE: f64 = 22.0;
const GAP: f64 = 16.0;

/// `ui.Offset`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Offset {
    pub dx: f64,
    pub dy: f64,
}

/// `ui.Rect`: guarda os quatro lados, como o Dart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Rect {
    pub fn from_ltwh(left: f64, top: f64, width: f64, height: f64) -> Self {
        Self::from_ltrb(left, top, left + width, top + height)
    }

    pub fn from_ltrb(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> f64 {
        self.right - self.left
    }

    pub fn height(&self) -> f64 {
        self.bottom - self.top
    }

    pub fn center(&self) -> Offset {
        Offset {
            dx: self.left + self.width() / 2.0,
            dy: self.top + self.height() / 2.0,
        }
    }
}

/// `FarmDecor`: flor, arbusto, cogumelo, abóbora ou tronco na terra livre.
#[derive(Clone, Debug, PartialEq)]
pub struct Decor {
    pub kind: &'static str,
    pub x: f64,
    pub y: f64,
    pub r: f64,
    pub c: usize,
}

/// `FarmViewport`: a janela onde a fazenda é desenhada (px lógicos) e o quanto o
/// HUD cobre.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Viewport {
    pub w: f64,
    pub h: f64,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl Viewport {
    /// O zoom "clássico" do clodfarm: px por pixel de mundo.
    pub fn s(&self) -> i64 {
        ((self.w / 330.0).min(self.h / 205.0))
            .round()
            .clamp(2.0, 6.0) as i64
    }
}

/// `FarmLayout` (sem `nav`: a grade de caminhar vem do mundo).
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub w: i64,
    pub h: i64,
    pub t: i64,
    pub play: Rect,
    pub barn: Rect,
    pub yard: Rect,
    pub field: Rect,
    pub content: Rect,
    pub yard_cols: i64,
    pub columns: i64,
    /// Lago: `pond.left/top` é o centro e `width/height` os raios.
    pub pond: Rect,
    pub plots: Vec<Rect>,
    /// Portão do cercado: lado esquerdo, na altura `gate_at`.
    pub gate_at: f64,
    pub sign: Offset,
    pub crow: Offset,
    pub door: Offset,
    pub paths: Vec<Vec<Offset>>,
    /// Zoom que mostra a fazenda inteira na janela (px lógicos por px de mundo).
    pub z: f64,
    pub cap_plots: usize,
    pub cap_yard: i64,
    /// Preenchidos pelo mundo (pedras e decoração), depois do layout.
    pub rocks: Vec<Rect>,
    pub decor: Vec<Decor>,
}

impl Layout {
    pub fn pond_cx(&self) -> f64 {
        self.pond.left
    }

    pub fn pond_cy(&self) -> f64 {
        self.pond.top
    }

    pub fn pond_rx(&self) -> f64 {
        self.pond.width()
    }

    pub fn pond_ry(&self) -> f64 {
        self.pond.height()
    }

    /// Onde o n-ésimo que dorme senta no curral.
    pub fn yard_spot(&self, i: i64) -> Offset {
        Offset {
            dx: self.yard.left + 13.0 + (i % self.yard_cols) as f64 * YARD_CX,
            dy: self.yard.top + 19.0 + (i / self.yard_cols) as f64 * YARD_CY,
        }
    }

    /// Lugar livre (fora de celeiro, cercado, lago, pedras): ninguém anda ali.
    pub fn blocked(&self, x: f64, y: f64) -> bool {
        let in_r = |r: &Rect, pad: f64| {
            x > r.left - pad && x < r.right + pad && y > r.top - pad && y < r.bottom + pad + 4.0
        };
        if in_r(&self.barn, 8.0) || in_r(&self.field, 2.0) || in_r(&self.yard, 6.0) {
            return true;
        }
        if (x - self.crow.dx).abs() < 12.0 && y > self.crow.dy - 24.0 && y < self.crow.dy + 6.0 {
            return true;
        }
        if (x - self.sign.dx).abs() < 8.0 && y > self.sign.dy - 4.0 && y < self.sign.dy + 14.0 {
            return true;
        }
        let px = (x - self.pond_cx()) / (self.pond_rx() + 8.0);
        let py = (y - self.pond_cy()) / (self.pond_ry() + 6.0);
        if px * px + py * py < 1.0 {
            return true;
        }
        if self.decor.iter().any(|d| {
            d.r > 0.0 && (x - d.x).abs() < d.r + 5.0 && y > d.y - d.r - 2.0 && y < d.y + 5.0
        }) {
            return true;
        }
        self.rocks.iter().any(|r| {
            (x - r.left).abs() < r.width() / 2.0 + 6.0 && y > r.top - 4.0 && y < r.bottom + 4.0
        })
    }

    /// Se o ponto cai perto de algum trecho de caminho (caixa de cada segmento).
    pub fn on_path(&self, x: f64, y: f64, pad: f64) -> bool {
        self.paths.iter().any(|pts| {
            pts.windows(2)
                .any(|seg| matches!(seg, [a, b] if near_segment(*a, *b, x, y, pad)))
        })
    }
}

fn near_segment(a: Offset, b: Offset, x: f64, y: f64, pad: f64) -> bool {
    x > a.dx.min(b.dx) - pad
        && x < a.dx.max(b.dx) + pad
        && y > a.dy.min(b.dy) - pad
        && y < a.dy.max(b.dy) + pad
}

/// `clampD` do pix.dart.
fn clamp_d(v: f64, a: f64, b: f64) -> f64 {
    a.max(b.min(v))
}

/// O layout real, crescido para caber: um cercado com um canteiro por loop
/// principal, o celeiro com o curral de quem dorme, o lago e os caminhos.
/// Testa larguras de cercado e curral e fica com a que mostra a fazenda maior na
/// janela. `snap` arredonda o zoom para pixels inteiros. `margin` fixa a borda.
pub fn layout_farm(
    plots: i64,
    yard: i64,
    view: &Viewport,
    snap: impl Fn(f64) -> f64,
    margin: Option<i64>,
) -> Layout {
    let n_p = plots.max(6);
    let n_y = yard.max(5);
    let av_w = (view.w - view.left - view.right).max(120.0);
    let av_h = (view.h - view.top - view.bottom).max(120.0);
    // Melhor candidato até agora; começa em -inf, então o primeiro sempre entra
    // (é o `best == null` do Dart).
    let mut best = Candidate::default();
    let (mut fh, mut lw, mut yw, mut yh) = (0.0, 0.0, 0.0, 0.0);
    for fc in 1..=n_p.min(30) {
        let fr = (n_p as f64 / fc as f64).ceil() as i64;
        if fc > 1 && (n_p as f64 / (fc - 1) as f64).ceil() as i64 == fr {
            continue;
        }
        for yc in [5, 7, 9, 12, 16, 22, 30] {
            if yc > 5 && yc > n_y + 4 {
                break;
            }
            let fw = fc as f64 * PLOT_CX - (PLOT_CX - PLOT_W) + 35.0 + CROW_LANE;
            let fhh = fr as f64 * PLOT_CY - (PLOT_CY - PLOT_H) + 30.0;
            let yww = yc as f64 * YARD_CX + 8.0;
            let yhh = (n_y as f64 / yc as f64).ceil() * YARD_CY + 10.0;
            let lww = yww.max(96.0);
            let lh = 8.0 + 44.0 + 16.0 + yhh + 40.0;
            let cw = lww + GAP + fw;
            let ch = lh.max(fhh);
            let z = (av_w / cw).min(av_h / ch);
            if z > best.z + 1e-6 {
                best = Candidate {
                    z,
                    fc,
                    fr,
                    yc,
                    cw,
                    ch,
                    fw,
                };
                fh = fhh;
                lw = lww;
                yw = yww;
                yh = yhh;
            }
        }
    }
    let b = best;
    let z = snap(f64::min(view.s() as f64, b.z));
    let t = margin.unwrap_or_else(|| {
        ((view.w.min(view.h) / view.s() as f64 * 0.14).round() as i64).clamp(20, 40)
    });
    let tf = t as f64;
    let x0 = ((t + 4) as f64).max((view.left / z + (av_w / z - b.cw) / 2.0).round());
    let y0 = ((t + 6) as f64).max((view.top / z + (av_h / z - b.ch) / 2.0).round());
    let w = ((view.w / z).ceil() as i64).max((x0 + b.cw + tf + 4.0).ceil() as i64);
    let h = ((view.h / z).ceil() as i64).max((y0 + b.ch + tf + 4.0).ceil() as i64);
    let play = Rect::from_ltrb(tf + 2.0, tf + 8.0, w as f64 - tf - 2.0, h as f64 - tf - 4.0);
    let barn = Rect::from_ltwh(x0 + ((lw - 46.0) / 2.0).round(), y0 + 8.0, 46.0, 44.0);
    let yard_r = Rect::from_ltwh(x0 + ((lw - yw) / 2.0).round(), barn.bottom + 16.0, yw, yh);
    let pond = Rect::from_ltwh(
        x0 + (lw / 2.0).round(),
        yard_r.bottom + 21.0,
        30.0_f64.min((lw / 2.0).round() - 6.0),
        11.0,
    );
    let field = Rect::from_ltwh(x0 + lw + GAP, y0 + ((b.ch - fh) / 2.0).round(), b.fw, fh);
    let fx = field.left + 19.0 + CROW_LANE;
    let fy = field.top + 15.0;
    let plot_rects: Vec<Rect> = (0..b.fc * b.fr)
        .map(|i| {
            Rect::from_ltwh(
                fx + (i % b.fc) as f64 * PLOT_CX,
                fy + (i / b.fc) as f64 * PLOT_CY,
                PLOT_W,
                PLOT_H,
            )
        })
        .collect();
    let crow = Offset {
        dx: field.left + 13.0,
        dy: field.bottom - 9.0,
    };
    let door = Offset {
        dx: barn.center().dx,
        dy: barn.bottom + 2.0,
    };
    // caminhos de terra: da porta do celeiro ao curral, e dali ao portão
    let py = door.dy + 7.0;
    let gy = clamp_d(
        py,
        field.top + 12.0,
        (field.top + 12.0).max(field.bottom - 38.0),
    );
    let mx = field.left - 8.0;
    let mut paths = vec![vec![
        Offset {
            dx: door.dx,
            dy: door.dy - 1.0,
        },
        Offset {
            dx: door.dx,
            dy: yard_r.top + 2.0,
        },
    ]];
    if gy == py {
        paths.push(vec![
            Offset {
                dx: door.dx,
                dy: py,
            },
            Offset {
                dx: field.left + 4.0,
                dy: py,
            },
        ]);
    } else {
        paths.push(vec![
            Offset {
                dx: door.dx,
                dy: py,
            },
            Offset { dx: mx, dy: py },
            Offset { dx: mx, dy: gy },
            Offset {
                dx: field.left + 4.0,
                dy: gy,
            },
        ]);
    }
    Layout {
        w,
        h,
        t,
        play,
        barn,
        yard: yard_r,
        yard_cols: b.yc,
        columns: b.fc,
        pond,
        field,
        content: Rect::from_ltwh(x0, y0, b.cw, b.ch),
        gate_at: gy,
        sign: Offset {
            dx: field.left - 7.0,
            dy: gy - 9.0,
        },
        crow,
        door,
        paths,
        z,
        cap_plots: plot_rects.len(),
        plots: plot_rects,
        cap_yard: b.yc * (n_y as f64 / b.yc as f64).ceil() as i64,
        rocks: Vec::new(),
        decor: Vec::new(),
    }
}

#[derive(Clone, Copy)]
struct Candidate {
    z: f64,
    fc: i64,
    fr: i64,
    yc: i64,
    cw: f64,
    ch: f64,
    fw: f64,
}

impl Default for Candidate {
    fn default() -> Self {
        Self {
            z: f64::NEG_INFINITY,
            fc: 0,
            fr: 0,
            yc: 0,
            cw: 0.0,
            ch: 0.0,
            fw: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    /// Saída do harness Dart (golden/layout.dart): mesmas linhas, mesma ordem.
    const GOLDEN: &str = r#"A w 400 h 300 t 40 z 2.0 cols 2 yc 5 gate 142.0
A play 42.0 48.0 358.0 256.0
A barn 96.0 89.0 46.0 44.0
A yard 67.0 149.0 103.0 27.0
A pond 119.0 197.0 30.0 11.0
A field 186.0 81.0 147.0 138.0
A content 67.0 81.0 266.0 138.0
A sign 179.0 133.0 crow 199.0 210.0 door 119.0 135.0
A caps 6 5 plots 6
A P 227.0 96.0 30.0 16.0
A P 287.0 96.0 30.0 16.0
A P 227.0 142.0 30.0 16.0
A P 287.0 142.0 30.0 16.0
A P 227.0 188.0 30.0 16.0
A P 287.0 188.0 30.0 16.0
A path 119.0,134.0 119.0,151.0
A path 119.0,142.0 190.0,142.0
A spot 0 80.0 168.0
A spot 1 99.0 168.0
A spot 2 118.0 168.0
A spot 3 137.0 168.0
A blk00000
A blk00000
A blk00011
A blk00011
A blk00011
A blk00011
A onPath 30.0 false false false
A onPath 119.0 false true false
A onPath 150.0 false true false
A onPath 190.0 false true false
B w 400 h 260 t 33 z 2.875 cols 3 yc 5 gate 100.0
B play 35.0 41.0 365.0 223.0
B barn 66.0 47.0 46.0 44.0
B yard 37.0 107.0 103.0 44.0
B pond 89.0 172.0 30.0 11.0
B field 156.0 39.0 207.0 184.0
B content 37.0 39.0 326.0 184.0
B sign 149.0 91.0 crow 169.0 214.0 door 89.0 93.0
B caps 12 10 plots 12
B P 197.0 54.0 30.0 16.0
B P 257.0 54.0 30.0 16.0
B P 317.0 54.0 30.0 16.0
B P 197.0 100.0 30.0 16.0
B P 257.0 100.0 30.0 16.0
B P 317.0 100.0 30.0 16.0
B P 197.0 146.0 30.0 16.0
B P 257.0 146.0 30.0 16.0
B P 317.0 146.0 30.0 16.0
B P 197.0 192.0 30.0 16.0
B P 257.0 192.0 30.0 16.0
B P 317.0 192.0 30.0 16.0
B path 89.0,92.0 89.0,109.0
B path 89.0,100.0 160.0,100.0
B spot 0 50.0 126.0
B spot 1 69.0 126.0
B spot 2 88.0 126.0
B spot 3 107.0 126.0
B blk00000
B blk00001
B blk00111
B blk00001
B blk00111
B blk00111
B onPath 30.0 false false false
B onPath 119.0 false false false
B onPath 150.0 false false false
B onPath 190.0 false false false
C w 454 h 300 t 30 z 3.0 cols 4 yc 5 gate 97.0
C play 32.0 38.0 422.0 266.0
C barn 63.0 44.0 46.0 44.0
C yard 34.0 104.0 103.0 61.0
C pond 86.0 186.0 30.0 11.0
C field 153.0 36.0 267.0 230.0
C content 34.0 36.0 386.0 230.0
C sign 146.0 88.0 crow 166.0 257.0 door 86.0 90.0
C caps 20 15 plots 20
C P 194.0 51.0 30.0 16.0
C P 254.0 51.0 30.0 16.0
C P 314.0 51.0 30.0 16.0
C P 374.0 51.0 30.0 16.0
C P 194.0 97.0 30.0 16.0
C P 254.0 97.0 30.0 16.0
C P 314.0 97.0 30.0 16.0
C P 374.0 97.0 30.0 16.0
C P 194.0 143.0 30.0 16.0
C P 254.0 143.0 30.0 16.0
C P 314.0 143.0 30.0 16.0
C P 374.0 143.0 30.0 16.0
C P 194.0 189.0 30.0 16.0
C P 254.0 189.0 30.0 16.0
C P 314.0 189.0 30.0 16.0
C P 374.0 189.0 30.0 16.0
C P 194.0 235.0 30.0 16.0
C P 254.0 235.0 30.0 16.0
C P 314.0 235.0 30.0 16.0
C P 374.0 235.0 30.0 16.0
C path 86.0,89.0 86.0,106.0
C path 86.0,97.0 157.0,97.0
C spot 0 47.0 123.0
C spot 1 66.0 123.0
C spot 2 85.0 123.0
C spot 3 104.0 123.0
C blk00000
C blk00011
C blk00111
C blk00011
C blk00111
C blk00111
C onPath 30.0 false false false
C onPath 119.0 false false false
C onPath 150.0 false false false
C onPath 190.0 false false false
"#;

    fn configs() -> [(&'static str, Layout); 3] {
        let a = layout_farm(
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
        let b = layout_farm(
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
        let c = layout_farm(
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
        [("A", a), ("B", b), ("C", c)]
    }

    fn show(k: &str, l: &Layout) -> String {
        let mut out = String::new();
        let mut line = |s: String| {
            out.push_str(&s);
            out.push('\n');
        };
        line(format!(
            "{k} w {} h {} t {} z {:?} cols {} yc {} gate {:?}",
            l.w, l.h, l.t, l.z, l.columns, l.yard_cols, l.gate_at
        ));
        line(format!(
            "{k} play {:?} {:?} {:?} {:?}",
            l.play.left, l.play.top, l.play.right, l.play.bottom
        ));
        line(format!(
            "{k} barn {:?} {:?} {:?} {:?}",
            l.barn.left,
            l.barn.top,
            l.barn.width(),
            l.barn.height()
        ));
        line(format!(
            "{k} yard {:?} {:?} {:?} {:?}",
            l.yard.left,
            l.yard.top,
            l.yard.width(),
            l.yard.height()
        ));
        line(format!(
            "{k} pond {:?} {:?} {:?} {:?}",
            l.pond.left,
            l.pond.top,
            l.pond.width(),
            l.pond.height()
        ));
        line(format!(
            "{k} field {:?} {:?} {:?} {:?}",
            l.field.left,
            l.field.top,
            l.field.width(),
            l.field.height()
        ));
        line(format!(
            "{k} content {:?} {:?} {:?} {:?}",
            l.content.left,
            l.content.top,
            l.content.width(),
            l.content.height()
        ));
        line(format!(
            "{k} sign {:?} {:?} crow {:?} {:?} door {:?} {:?}",
            l.sign.dx, l.sign.dy, l.crow.dx, l.crow.dy, l.door.dx, l.door.dy
        ));
        line(format!(
            "{k} caps {} {} plots {}",
            l.cap_plots,
            l.cap_yard,
            l.plots.len()
        ));
        for p in &l.plots {
            line(format!(
                "{k} P {:?} {:?} {:?} {:?}",
                p.left,
                p.top,
                p.width(),
                p.height()
            ));
        }
        for pts in &l.paths {
            let joined: Vec<String> = pts
                .iter()
                .map(|o| format!("{:?},{:?}", o.dx, o.dy))
                .collect();
            line(format!("{k} path {}", joined.join(" ")));
        }
        for i in 0..4 {
            let s = l.yard_spot(i);
            line(format!("{k} spot {i} {:?} {:?}", s.dx, s.dy));
        }
        for x in [5.0, 40.0, 90.0, 130.0, 200.0, 300.0] {
            let row: String = [5.0, 30.0, 60.0, 100.0, 150.0]
                .iter()
                .map(|&y| if l.blocked(x, y) { '1' } else { '0' })
                .collect();
            line(format!("{k} blk{row}"));
        }
        for x in [30.0, 119.0, 150.0, 190.0] {
            let mut s = format!("{k} onPath {x:?}");
            write!(
                s,
                " {} {} {}",
                l.on_path(x, 60.0, 5.0),
                l.on_path(x, 140.0, 5.0),
                l.on_path(x, 120.0, 2.0)
            )
            .unwrap();
            line(s);
        }
        out
    }

    #[test]
    fn layout_igual_ao_dart() {
        let got: String = configs().iter().map(|(k, l)| show(k, l)).collect();
        assert_eq!(got, GOLDEN);
    }

    #[test]
    fn viewport_s_clamp_do_clodfarm() {
        let v = Viewport {
            w: 800.0,
            h: 600.0,
            ..Viewport::default()
        };
        assert_eq!(v.s(), 2);
        let big = Viewport {
            w: 4000.0,
            h: 3000.0,
            ..Viewport::default()
        };
        assert_eq!(big.s(), 6);
    }
}
