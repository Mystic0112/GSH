//! Geometria pura do terminal: cálculo de `cols`/`rows` pelas métricas da
//! fonte, conversão de coordenadas em pixels para `(row, col)`, intervalos de
//! seleção linear e conversão de eventos de rolagem.

use gsh_term::GridPos;
use iced::mouse::ScrollDelta;
use iced::Point;

use crate::theme::{TERM_CELL_HEIGHT, TERM_CELL_WIDTH};

/// Colunas iniciais padrão ao abrir uma sessão antes do primeiro layout.
pub const DEFAULT_COLS: u16 = 80;
/// Linhas iniciais padrão ao abrir uma sessão antes do primeiro layout.
pub const DEFAULT_ROWS: u16 = 24;
/// Limite mínimo de colunas aceito pelo emulador VT.
pub const MIN_COLS: u16 = 2;
/// Limite mínimo de linhas aceito pelo emulador VT.
pub const MIN_ROWS: u16 = 1;
/// Limite máximo seguro de colunas para evitar alocações excessivas.
pub const MAX_COLS: u16 = 500;
/// Limite máximo seguro de linhas.
pub const MAX_ROWS: u16 = 300;
/// Quantidade de linhas roladas por "dente" da roda do mouse.
pub const SCROLL_WHEEL_LINES: f32 = 3.0;
/// Distância mínima em pixels para distinguir arraste de clique simples.
pub const DRAG_THRESHOLD_PX: f32 = 3.0;

/// Métricas geométricas da célula do terminal no canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellMetrics {
    pub cell_width: f32,
    pub cell_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
}

impl Default for CellMetrics {
    fn default() -> Self {
        Self {
            cell_width: TERM_CELL_WIDTH,
            cell_height: TERM_CELL_HEIGHT,
            padding_x: 0.0,
            padding_y: 0.0,
        }
    }
}

/// Calcula `(cols, rows)` que cabem na área `(width_px, height_px)` descontando
/// o padding e aplicando os limites mínimos/máximos do emulador.
pub fn compute_grid_size(width_px: f32, height_px: f32, metrics: CellMetrics) -> (u16, u16) {
    let cw = metrics.cell_width.max(1.0);
    let ch = metrics.cell_height.max(1.0);
    let inner_w = (width_px - metrics.padding_x).max(0.0);
    let inner_h = (height_px - metrics.padding_y * 2.0).max(0.0);

    let raw_cols = (inner_w / cw).floor();
    let raw_rows = (inner_h / ch).floor();

    let cols = if raw_cols.is_finite() && raw_cols >= f32::from(MIN_COLS) {
        (raw_cols as u32).min(u32::from(MAX_COLS)) as u16
    } else {
        MIN_COLS
    };

    let rows = if raw_rows.is_finite() && raw_rows >= f32::from(MIN_ROWS) {
        (raw_rows as u32).min(u32::from(MAX_ROWS)) as u16
    } else {
        MIN_ROWS
    };

    (cols, rows)
}

/// Indica se o terminal deve ser redimensionado de `current` para `next`.
pub const fn should_resize(current: (u16, u16), next: (u16, u16)) -> bool {
    next.0 >= MIN_COLS && next.1 >= MIN_ROWS && (current.0 != next.0 || current.1 != next.1)
}

/// Converte uma posição `(x, y)` local ao canvas na célula [`GridPos`] `(row, col)`
/// correspondente, limitada a `0..rows` e `0..cols`.
pub fn pixel_to_grid(x: f32, y: f32, metrics: CellMetrics, cols: u16, rows: u16) -> GridPos {
    if cols == 0 || rows == 0 {
        return GridPos::new(0, 0);
    }

    let cw = metrics.cell_width.max(1.0);
    let ch = metrics.cell_height.max(1.0);
    let rel_x = (x - metrics.padding_x).max(0.0);
    let rel_y = (y - metrics.padding_y).max(0.0);

    let max_col = usize::from(cols).saturating_sub(1);
    let max_row = usize::from(rows).saturating_sub(1);

    let col = ((rel_x / cw).floor() as usize).min(max_col);
    let row = ((rel_y / ch).floor() as usize).min(max_row);

    GridPos::new(row, col)
}

/// Indica se o ponteiro se moveu além do limiar de arraste (`DRAG_THRESHOLD_PX`).
pub fn exceeds_drag_threshold(origin: Point, current: Point) -> bool {
    let dx = current.x - origin.x;
    let dy = current.y - origin.y;
    (dx * dx + dy * dy) >= DRAG_THRESHOLD_PX * DRAG_THRESHOLD_PX
}

/// Intervalo de seleção linear no grid visível entre `anchor` (início) e `focus` (fim).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRange {
    pub anchor: GridPos,
    pub focus: GridPos,
}

impl SelectionRange {
    pub const fn new(anchor: GridPos, focus: GridPos) -> Self {
        Self { anchor, focus }
    }

    /// Retorna as extremidades `(start, end)` ordenadas lexicograficamente por `(row, col)`.
    pub fn ordered(self) -> (GridPos, GridPos) {
        if self.anchor <= self.focus {
            (self.anchor, self.focus)
        } else {
            (self.focus, self.anchor)
        }
    }

    /// Verifica se a célula `(row, col)` pertence à seleção linear.
    pub fn contains(self, row: usize, col: usize) -> bool {
        let (start, end) = self.ordered();
        if row < start.row || row > end.row {
            return false;
        }
        if start.row == end.row {
            return col >= start.col && col <= end.col;
        }
        if row == start.row {
            return col >= start.col;
        }
        if row == end.row {
            return col <= end.col;
        }
        true
    }

    /// Retorna o intervalo inclusivo `(start_col, end_col)` selecionado na linha `row`.
    pub fn row_span(self, row: usize, cols: u16) -> Option<(usize, usize)> {
        let max_col = usize::from(cols).checked_sub(1)?;
        let (start, end) = self.ordered();
        if row < start.row || row > end.row {
            return None;
        }
        let c0 = if row == start.row {
            start.col.min(max_col)
        } else {
            0
        };
        let c1 = if row == end.row {
            end.col.min(max_col)
        } else {
            max_col
        };
        if c0 <= c1 {
            Some((c0, c1))
        } else {
            None
        }
    }
}

/// Passo de rolagem do histórico de scrollback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollStep {
    /// Rola para cima (em direção ao histórico mais antigo).
    Up(usize),
    /// Rola para baixo (em direção ao presente).
    Down(usize),
}

/// Converte um [`ScrollDelta`] do mouse em [`ScrollStep`].
pub fn scroll_delta_to_step(delta: ScrollDelta, cell_height: f32) -> Option<ScrollStep> {
    let lines_f = match delta {
        ScrollDelta::Lines { y, .. } => y * SCROLL_WHEEL_LINES,
        ScrollDelta::Pixels { y, .. } => {
            let ch = cell_height.max(1.0);
            y / ch
        }
    };

    if !lines_f.is_finite() || lines_f.abs() < 0.25 {
        return None;
    }

    let count = (lines_f.abs().round() as usize).max(1);
    if lines_f > 0.0 {
        Some(ScrollStep::Up(count))
    } else {
        Some(ScrollStep::Down(count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_cols_and_rows_and_guards_resize() {
        let metrics = CellMetrics {
            cell_width: 8.0,
            cell_height: 16.0,
            padding_x: 8.0,
            padding_y: 8.0,
        };
        // (80 * 8 + 8, 24 * 16 + 16) = (648, 400): only left padding applies.
        assert_eq!(compute_grid_size(648.0, 400.0, metrics), (80, 24));
        // Tamanhos pequenos nunca caem abaixo de (MIN_COLS, MIN_ROWS)
        assert_eq!(compute_grid_size(2.0, 2.0, metrics), (MIN_COLS, MIN_ROWS));

        assert!(!should_resize((80, 24), (80, 24)));
        assert!(should_resize((80, 24), (100, 30)));
        assert!(!should_resize((80, 24), (1, 24)));
    }

    #[test]
    fn maps_pixels_to_clamped_grid_positions() {
        let metrics = CellMetrics {
            cell_width: 8.0,
            cell_height: 16.0,
            padding_x: 10.0,
            padding_y: 10.0,
        };
        assert_eq!(pixel_to_grid(0.0, 0.0, metrics, 80, 24), GridPos::new(0, 0));
        assert_eq!(
            pixel_to_grid(10.0 + 8.0 * 5.2, 10.0 + 16.0 * 3.7, metrics, 80, 24),
            GridPos::new(3, 5)
        );
        assert_eq!(
            pixel_to_grid(9999.0, 9999.0, metrics, 80, 24),
            GridPos::new(23, 79)
        );
    }

    #[test]
    fn selection_geometry_handles_forward_reverse_and_multiline() {
        // Mesma linha invertida (direita -> esquerda)
        let single = SelectionRange::new(GridPos::new(2, 10), GridPos::new(2, 4));
        assert!(single.contains(2, 4));
        assert!(single.contains(2, 7));
        assert!(single.contains(2, 10));
        assert!(!single.contains(2, 3));
        assert!(!single.contains(2, 11));
        assert_eq!(single.row_span(2, 80), Some((4, 10)));
        assert_eq!(single.row_span(1, 80), None);

        // Multi-linha de baixo para cima
        let multi = SelectionRange::new(GridPos::new(4, 3), GridPos::new(2, 75));
        assert_eq!(multi.row_span(2, 80), Some((75, 79)));
        assert_eq!(multi.row_span(3, 80), Some((0, 79)));
        assert_eq!(multi.row_span(4, 80), Some((0, 3)));
        assert!(multi.contains(3, 40));
    }

    #[test]
    fn converts_scroll_wheel_deltas() {
        assert_eq!(
            scroll_delta_to_step(ScrollDelta::Lines { x: 0.0, y: 1.0 }, 17.0),
            Some(ScrollStep::Up(3))
        );
        assert_eq!(
            scroll_delta_to_step(ScrollDelta::Lines { x: 0.0, y: -2.0 }, 17.0),
            Some(ScrollStep::Down(6))
        );
        assert_eq!(
            scroll_delta_to_step(ScrollDelta::Pixels { x: 0.0, y: -34.0 }, 17.0),
            Some(ScrollStep::Down(2))
        );
    }
}
