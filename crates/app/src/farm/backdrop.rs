// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! Fundo do cenário no canvas: o layout de farm_world.dart, a pintura HD em imagem
//! e os canteiros, onde os agentes sentam. Fazenda e escritório usam o mesmo layout;
//! só a pintura muda. Coordenadas: as do layout (origem no canto do mundo).
use super::agents::Need;
use super::camera::{snap, Extent};
use super::crops::{self, CropStage};
use super::layout::{layout_farm, Rect, Viewport};
use super::office::sprites::{monitor, monitor_off, monitor_wait, MONITOR_HEIGHT, MONITOR_WIDTH};
use super::office_paint::paint_office;
use super::raster::{Raster, HD};
use super::world::{Kind, Mode, Prop};
use super::world_paint::paint_farm;
use iced::widget::image::Handle;
use iced::Size;

/// Semente do fundo: a mesma do `buildWorld` do Flutter.
const SEED: u32 = 7;

/// Telas das ilhas: apagada, em espera e os três quadros de trabalho por estágio.
pub struct Screens {
    off: Handle,
    wait: Handle,
    working: Vec<Handle>,
}

pub struct Backdrop {
    /// Janela em que o layout foi feito: quando muda, o fundo é refeito.
    pub size: Size,
    /// Modo em que o fundo foi pintado.
    pub mode: Mode,
    /// Janela com o recorte do HUD em que o layout foi feito (`_viewport` do Dart).
    pub view: Viewport,
    /// O mundo para a câmera: tamanho, zoom que o mostra inteiro e centro do conteúdo.
    pub extent: Extent,
    /// Com o que o fundo foi feito: o tamanho do meu mundo. O meu fundo usa o próprio; o de um
    /// colega usa o meu (o lote acompanha o meu mundo e o cenário).
    pub built_for: (i64, i64),
    /// Borda do mundo (`layout.t`) e o miolo jogável (`layout.play`): o lote do colega herda a
    /// borda da minha fazenda e o escritório cola os lotes pela parede.
    pub t: i64,
    pub play: Rect,
    /// Porta de saída do cenário, onde o mensageiro começa quando não há agente.
    pub door: (f32, f32),
    /// Quanto o layout comporta (`capPlots`/`capYard`).
    cap_plots: i64,
    cap_yard: i64,
    width: i32,
    height: i32,
    /// Linha em que a faixa da frente começa, em coordenadas do layout.
    fg_y: i32,
    /// Canteiros: canto superior esquerdo, nas coordenadas do layout.
    plots: Vec<(i32, i32)>,
    bg: Handle,
    fg: Option<Handle>,
    screens: Option<Screens>,
}

/// A janela e o pedaço que o HUD cobre: larga, ele fica ao lado da fazenda; alta, em
/// cima (`_viewport` de farm_view.dart).
pub fn viewport(size: Size) -> Viewport {
    let (w, h) = (f64::from(size.width), f64::from(size.height));
    let wide = w > h * 1.15 && 240.0 < w * 0.4;
    Viewport {
        w,
        h,
        left: if wide { 240.0 } else { 0.0 },
        top: if wide { 8.0 } else { 150.0 },
        right: 12.0,
        bottom: 12.0,
    }
}

impl Backdrop {
    /// Refaz o mundo para `size` e para o que `need` pede (`_rebuild` do Dart).
    pub fn build(size: Size, mode: Mode, need: Need) -> Self {
        Self::paint(
            viewport(size),
            size,
            mode,
            need,
            SEED,
            |z| snap(z, true),
            None,
        )
    }

    /// A fazenda de um colega (`layoutLot` do Dart): a "janela" é o meu mundo inteiro em z = 1,
    /// então o lote sai do tamanho do meu (maior só se não couber), com o miolo no meio e a
    /// mesma borda. `seed` dá a cada colega árvores e móveis diferentes.
    pub fn build_lot(mine: &Backdrop, mode: Mode, need: Need, seed: u32) -> Self {
        let view = Viewport {
            w: mine.width as f64,
            h: mine.height as f64,
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 0.0,
        };
        let mut lot = Self::paint(view, mine.size, mode, need, seed, |_| 1.0, Some(mine.t));
        lot.built_for = (mine.width as i64, mine.height as i64);
        lot
    }

    fn paint(
        view: Viewport,
        size: Size,
        mode: Mode,
        need: Need,
        seed: u32,
        snap: impl Fn(f64) -> f64,
        margin: Option<i64>,
    ) -> Self {
        let mut layout = layout_farm(
            (need.plots as f64 * 1.15).ceil() as i64 + 1,
            (need.yard as f64 * 1.1).ceil() as i64 + 2,
            &view,
            snap,
            margin,
        );
        let (bg, fg, fg_y, screens) = match mode {
            Mode::Farm => {
                let (mut bg, fg, fg_y) = paint_farm(&mut layout, seed);
                // ponytail: cultivos vão no raster porque o canvas desenha imagens por cima das formas
                paint_crops(&mut bg, &layout.plots);
                (bg, Some(fg), fg_y, None)
            }
            Mode::Office => {
                let bg = paint_office(&mut layout, seed);
                (bg, None, layout.h, Some(screens()))
            }
        };
        let (width, height) = (layout.w as i32, layout.h as i32);
        let door = match mode {
            Mode::Farm => (layout.door.dx as f32, (layout.door.dy + 5.0) as f32),
            Mode::Office => (layout.pond_cx() as f32, (layout.play.bottom - 4.0) as f32),
        };
        let plots = layout
            .plots
            .iter()
            .map(|p| (p.left.round() as i32, p.top.round() as i32))
            .collect();
        let center = layout.content.center();
        Self {
            size,
            mode,
            view,
            extent: Extent {
                w: layout.w as f64,
                h: layout.h as f64,
                z: layout.z,
                cx: center.dx,
                cy: center.dy,
                team: None,
            },
            built_for: (layout.w, layout.h),
            t: layout.t,
            play: layout.play,
            door,
            cap_plots: layout.cap_plots as i64,
            cap_yard: layout.cap_yard,
            width,
            height,
            fg_y: fg_y as i32,
            plots,
            bg: Handle::from_rgba(bg.w as u32, bg.h as u32, bg.to_rgba()),
            fg: fg.map(|fg| Handle::from_rgba(fg.w as u32, fg.h as u32, fg.to_rgba())),
            screens,
        }
    }

    /// O layout não comporta `need` (ou sobra muito): refazer o mundo (`outgrown`).
    pub fn outgrown(&self, need: Need) -> bool {
        need.plots > self.cap_plots
            || need.yard > self.cap_yard
            || (self.cap_plots > 8 && (need.plots as f64) < self.cap_plots as f64 * 0.45)
    }

    /// Tamanho do mundo em unidades de mundo (`layout.w` x `layout.h`).
    pub fn world(&self) -> (f64, f64) {
        (f64::from(self.width), f64::from(self.height))
    }

    /// Canteiros que o layout comporta.
    pub fn capacity(&self) -> usize {
        self.plots.len()
    }

    /// Retângulo do fundo (x, y, largura, altura), em unidades de mundo.
    pub fn bg_rect(&self) -> (f32, f32, f32, f32) {
        (0.0, 0.0, self.width as f32, self.height as f32)
    }

    /// Retângulo da faixa da frente: passa por cima dos bichos.
    pub fn fg_rect(&self) -> (f32, f32, f32, f32) {
        (
            0.0,
            self.fg_y as f32,
            self.width as f32,
            (self.height - self.fg_y) as f32,
        )
    }

    pub fn bg(&self) -> Handle {
        self.bg.clone()
    }

    pub fn fg(&self) -> Option<Handle> {
        self.fg.clone()
    }

    /// Canteiros (ou ilhas) como props de cultivo, na ordem do layout.
    pub fn posts(&self) -> Vec<Prop> {
        self.plots
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| Prop {
                kind: Kind::Plot(stage(i)),
                x,
                y,
            })
            .collect()
    }

    /// Canto do canteiro `i`, em unidades de mundo.
    pub fn plot_origin(&self, i: usize) -> Option<(f32, f32)> {
        self.plots.get(i).map(|&(x, y)| (x as f32, y as f32))
    }

    /// Imagem da tela da ilha `i` no quadro `frame`, conforme o estado do dono.
    pub fn screen(&self, i: usize, frame: usize, state: ScreenState) -> Option<Handle> {
        let screens = self.screens.as_ref()?;
        Some(match state {
            ScreenState::Off => screens.off.clone(),
            ScreenState::Waiting => screens.wait.clone(),
            ScreenState::Working => screens
                .working
                .get(stage(i) as usize * 3 + frame % 3)?
                .clone(),
        })
    }
}

/// Como a tela da ilha aparece: o estado do agente que senta ali.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScreenState {
    Off,
    Waiting,
    Working,
}

fn stage(i: usize) -> CropStage {
    match i % 3 {
        0 => CropStage::Seed,
        1 => CropStage::Sprout,
        _ => CropStage::Grow,
    }
}

fn handle(pixels: &[u8]) -> Handle {
    Handle::from_rgba(MONITOR_WIDTH as u32, MONITOR_HEIGHT as u32, pixels.to_vec())
}

/// Todas as telas pré-rasterizadas: não se recria imagem a cada quadro.
fn screens() -> Screens {
    let stages = [
        CropStage::Seed,
        CropStage::Sprout,
        CropStage::Grow,
        CropStage::Ripe,
        CropStage::Wilt,
    ];
    let working = stages
        .iter()
        .flat_map(|&s| (0..3).map(move |f| handle(&monitor(s, f))))
        .collect();
    Screens {
        off: handle(&monitor_off()),
        wait: handle(&monitor_wait(0)),
        working,
    }
}

/// Cultivo de cada canteiro, na mesma posição do `_plot` do Flutter: 3 x 2 plantas,
/// cada pixel do sprite com 0,5 de mundo (1 pixel HD).
fn paint_crops(g: &mut Raster, plots: &[Rect]) {
    let hd = HD as f64;
    for (i, plot) in plots.iter().enumerate() {
        let pixels = crops::rgba(stage(i), 0);
        for row in 0..2 {
            for k in 0..3 {
                let left = plot.left + 6.0 + k as f64 * 9.0 + row as f64 * 4.0 - 2.5;
                let top = plot.top + (if row == 1 { 15.0 } else { 8.0 }) - 9.0;
                for (j, &[r, gr, b, a]) in pixels.as_chunks::<4>().0.iter().enumerate() {
                    if a != 0 {
                        let x = (left + (j % crops::WIDTH) as f64 * 0.5) * hd;
                        let y = (top + (j / crops::WIDTH) as f64 * 0.5) * hd;
                        let argb = u32::from(a) << 24
                            | u32::from(r) << 16
                            | u32::from(gr) << 8
                            | u32::from(b);
                        g.fill(x, y, 1.0, 1.0, argb);
                    }
                }
            }
        }
    }
}
