//! Tipos próprios de snapshot do terminal para consumo pela UI.
//!
//! Nenhum tipo de `alacritty_terminal` ou `vte` atravessa esta fronteira.

use alacritty_terminal::term::cell::{Cell as AlacrittyCell, Flags};
use alacritty_terminal::vte::ansi::{Color as VteColor, CursorShape, NamedColor as VteNamedColor};

/// Cor RGB de 24 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// Cores ANSI nomeadas da paleta do terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NamedColor {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    DimBlack,
    DimRed,
    DimGreen,
    DimYellow,
    DimBlue,
    DimMagenta,
    DimCyan,
    DimWhite,
    Cursor,
    BrightForeground,
    DimForeground,
}

impl NamedColor {
    /// Resolve a cor nomeada para o RGB da paleta dark padrão do GSH
    /// (`gshTerminalThemeDark` no Flutter).
    pub const fn to_rgb(self) -> Rgb {
        match self {
            Self::Black | Self::DimBlack => Rgb::new(0x26, 0x26, 0x2A),
            Self::Red | Self::DimRed => Rgb::new(0xE5, 0x48, 0x4D),
            Self::Green | Self::DimGreen => Rgb::new(0x3F, 0xB8, 0x68),
            Self::Yellow | Self::DimYellow => Rgb::new(0xE0, 0xA3, 0x3A),
            Self::Blue | Self::DimBlue | Self::Cursor => Rgb::new(0x2F, 0x6F, 0xF0),
            Self::Magenta | Self::DimMagenta => Rgb::new(0xC7, 0x92, 0xEA),
            Self::Cyan | Self::DimCyan => Rgb::new(0x1A, 0xA5, 0xA0),
            Self::White | Self::DimWhite | Self::DimForeground => Rgb::new(0xC9, 0xC9, 0xCF),
            Self::BrightBlack => Rgb::new(0x6A, 0x6A, 0x73),
            Self::BrightRed => Rgb::new(0xFF, 0x6B, 0x6F),
            Self::BrightGreen => Rgb::new(0x82, 0xE0, 0xA5),
            Self::BrightYellow => Rgb::new(0xFF, 0xCB, 0x6B),
            Self::BrightBlue => Rgb::new(0x82, 0xAA, 0xFF),
            Self::BrightMagenta => Rgb::new(0xD6, 0xA0, 0xFF),
            Self::BrightCyan => Rgb::new(0x89, 0xDD, 0xFF),
            Self::BrightWhite | Self::BrightForeground => Rgb::new(0xEC, 0xEC, 0xEF),
        }
    }
}

/// Cor de frente ou de fundo de uma célula no snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TermColor {
    /// Cor padrão de frente ou de fundo do tema do terminal.
    #[default]
    Default,
    /// Cor ANSI nomeada.
    Named(NamedColor),
    /// Índice na tabela de 256 cores (`0..=255`).
    Indexed(u8),
    /// Cor RGB direta (truecolor).
    Rgb(Rgb),
}

/// Alias semântico para [`TermColor`].
pub type CellColor = TermColor;

impl TermColor {
    /// Constrói uma cor RGB direta.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Rgb(Rgb::new(r, g, b))
    }

    /// Indica se é a cor padrão (não modificada por sequência SGR).
    pub const fn is_default(self) -> bool {
        matches!(self, Self::Default)
    }

    /// Resolve a cor como cor de frente na paleta dark padrão do GSH.
    pub const fn resolve_fg(self) -> Rgb {
        match self {
            Self::Default => Rgb::new(0xEC, 0xEC, 0xEF),
            Self::Named(named) => named.to_rgb(),
            Self::Indexed(idx) => indexed_to_rgb(idx),
            Self::Rgb(rgb) => rgb,
        }
    }

    /// Resolve a cor como cor de fundo na paleta dark padrão do GSH.
    /// Retorna `None` quando a cor é [`TermColor::Default`] (fundo do painel).
    pub const fn resolve_bg(self) -> Option<Rgb> {
        match self {
            Self::Default => None,
            Self::Named(named) => Some(named.to_rgb()),
            Self::Indexed(idx) => Some(indexed_to_rgb(idx)),
            Self::Rgb(rgb) => Some(rgb),
        }
    }
}

/// Converte um índice da paleta de 256 cores (`0..=255`) para [`Rgb`],
/// seguindo a mesma tabela de `PaletteBuilder` do GSH em Flutter.
pub const fn indexed_to_rgb(idx: u8) -> Rgb {
    match idx {
        0 => NamedColor::Black.to_rgb(),
        1 => NamedColor::Red.to_rgb(),
        2 => NamedColor::Green.to_rgb(),
        3 => NamedColor::Yellow.to_rgb(),
        4 => NamedColor::Blue.to_rgb(),
        5 => NamedColor::Magenta.to_rgb(),
        6 => NamedColor::Cyan.to_rgb(),
        7 => NamedColor::White.to_rgb(),
        8 => NamedColor::BrightBlack.to_rgb(),
        9 => NamedColor::BrightRed.to_rgb(),
        10 => NamedColor::BrightGreen.to_rgb(),
        11 => NamedColor::BrightYellow.to_rgb(),
        12 => NamedColor::BrightBlue.to_rgb(),
        13 => NamedColor::BrightMagenta.to_rgb(),
        14 => NamedColor::BrightCyan.to_rgb(),
        15 => NamedColor::BrightWhite.to_rgb(),
        16..=231 => {
            let n = idx - 16;
            let r_idx = n / 36;
            let g_idx = (n % 36) / 6;
            let b_idx = n % 6;
            let r = if r_idx == 0 { 0 } else { 55 + r_idx * 40 };
            let g = if g_idx == 0 { 0 } else { 55 + g_idx * 40 };
            let b = if b_idx == 0 { 0 } else { 55 + b_idx * 40 };
            Rgb::new(r, g, b)
        }
        232..=255 => {
            let level = 8 + (idx - 232) * 10;
            Rgb::new(level, level, level)
        }
    }
}

/// Célula visível no snapshot do terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    /// Caractere exibido na célula (`' '` quando vazia).
    pub ch: char,
    /// Cor de frente (texto).
    pub fg: TermColor,
    /// Cor de fundo.
    pub bg: TermColor,
    /// Estilo negrito (SGR 1).
    pub bold: bool,
    /// Estilo sublinhado (SGR 4 e variantes).
    pub underline: bool,
    /// Vídeo reverso / invertido (SGR 7).
    pub inverse: bool,
}

/// Alias semântico para [`Cell`].
pub type SnapshotCell = Cell;

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: TermColor::Default,
            bg: TermColor::Default,
            bold: false,
            underline: false,
            inverse: false,
        }
    }
}

impl Cell {
    /// Caractere da célula.
    pub const fn char(&self) -> char {
        self.ch
    }

    /// Cor de frente efetiva em RGB, respeitando a flag `inverse`.
    pub const fn effective_fg(&self) -> Rgb {
        if self.inverse {
            match self.bg.resolve_bg() {
                Some(rgb) => rgb,
                None => Rgb::new(0x18, 0x18, 0x1B),
            }
        } else {
            self.fg.resolve_fg()
        }
    }

    /// Cor de fundo efetiva em RGB, respeitando a flag `inverse`.
    /// Retorna `None` quando o fundo é o padrão transparente do terminal.
    pub const fn effective_bg(&self) -> Option<Rgb> {
        if self.inverse {
            Some(self.fg.resolve_fg())
        } else {
            self.bg.resolve_bg()
        }
    }
}

/// Posição `(row, col)` 0-based no grid visível.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct GridPos {
    pub row: usize,
    pub col: usize,
}

impl GridPos {
    pub const fn new(row: usize, col: usize) -> Self {
        Self { row, col }
    }
}

/// Estado do cursor no snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cursor {
    /// Linha no grid visível (`0..rows`).
    pub row: usize,
    /// Coluna no grid visível (`0..cols`).
    pub col: usize,
    /// Indica se o cursor está visível no viewport atual.
    pub visible: bool,
}

impl Cursor {
    /// Posição `(row, col)` do cursor.
    pub const fn position(&self) -> GridPos {
        GridPos {
            row: self.row,
            col: self.col,
        }
    }
}

/// Fotografia imutável do estado visível do terminal para renderização na UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Largura do grid em colunas.
    pub cols: u16,
    /// Altura do grid visível em linhas.
    pub rows: u16,
    /// Grid visível: `rows` linhas, cada uma com `cols` células.
    pub lines: Vec<Vec<Cell>>,
    /// Estado do cursor (posição e visibilidade).
    pub cursor: Cursor,
    /// Deslocamento atual do scrollback (`0` = no fim/ao vivo, `> 0` = linhas roladas para cima).
    pub display_offset: usize,
    /// Número total de linhas de histórico (scrollback) disponíveis acima da tela ativa.
    pub history_size: usize,
}

/// Alias semântico para [`Snapshot`].
pub type TermSnapshot = Snapshot;

impl Snapshot {
    /// Retorna a célula em `(row, col)` se estiver dentro dos limites do viewport.
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.lines.get(row).and_then(|line| line.get(col))
    }

    /// Retorna o texto da linha `row` sem espaços em branco à direita.
    pub fn line_text(&self, row: usize) -> Option<String> {
        let line = self.lines.get(row)?;
        let raw: String = line.iter().map(|c| c.ch).collect();
        Some(raw.trim_end_matches(' ').to_owned())
    }

    /// Concatena todas as linhas visíveis (sem espaços à direita por linha) separadas por `\n`.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for (idx, line) in self.lines.iter().enumerate() {
            if idx > 0 {
                out.push('\n');
            }
            let raw: String = line.iter().map(|c| c.ch).collect();
            out.push_str(raw.trim_end_matches(' '));
        }
        out
    }
}

pub(crate) fn convert_cell(cell: &AlacrittyCell) -> Cell {
    Cell {
        ch: cell.c,
        fg: convert_color(cell.fg),
        bg: convert_color(cell.bg),
        bold: cell.flags.contains(Flags::BOLD),
        underline: cell.flags.intersects(Flags::ALL_UNDERLINES),
        inverse: cell.flags.contains(Flags::INVERSE),
    }
}

pub(crate) fn convert_color(color: VteColor) -> TermColor {
    match color {
        VteColor::Named(VteNamedColor::Foreground | VteNamedColor::Background) => {
            TermColor::Default
        }
        VteColor::Named(named) => TermColor::Named(convert_named(named)),
        VteColor::Indexed(idx) => TermColor::Indexed(idx),
        VteColor::Spec(rgb) => TermColor::Rgb(Rgb::new(rgb.r, rgb.g, rgb.b)),
    }
}

const fn convert_named(named: VteNamedColor) -> NamedColor {
    match named {
        VteNamedColor::Black | VteNamedColor::Background => NamedColor::Black,
        VteNamedColor::Red => NamedColor::Red,
        VteNamedColor::Green => NamedColor::Green,
        VteNamedColor::Yellow => NamedColor::Yellow,
        VteNamedColor::Blue => NamedColor::Blue,
        VteNamedColor::Magenta => NamedColor::Magenta,
        VteNamedColor::Cyan => NamedColor::Cyan,
        VteNamedColor::White | VteNamedColor::Foreground => NamedColor::White,
        VteNamedColor::BrightBlack => NamedColor::BrightBlack,
        VteNamedColor::BrightRed => NamedColor::BrightRed,
        VteNamedColor::BrightGreen => NamedColor::BrightGreen,
        VteNamedColor::BrightYellow => NamedColor::BrightYellow,
        VteNamedColor::BrightBlue => NamedColor::BrightBlue,
        VteNamedColor::BrightMagenta => NamedColor::BrightMagenta,
        VteNamedColor::BrightCyan => NamedColor::BrightCyan,
        VteNamedColor::BrightWhite => NamedColor::BrightWhite,
        VteNamedColor::DimBlack => NamedColor::DimBlack,
        VteNamedColor::DimRed => NamedColor::DimRed,
        VteNamedColor::DimGreen => NamedColor::DimGreen,
        VteNamedColor::DimYellow => NamedColor::DimYellow,
        VteNamedColor::DimBlue => NamedColor::DimBlue,
        VteNamedColor::DimMagenta => NamedColor::DimMagenta,
        VteNamedColor::DimCyan => NamedColor::DimCyan,
        VteNamedColor::DimWhite => NamedColor::DimWhite,
        VteNamedColor::Cursor => NamedColor::Cursor,
        VteNamedColor::BrightForeground => NamedColor::BrightForeground,
        VteNamedColor::DimForeground => NamedColor::DimForeground,
    }
}

pub(crate) const fn is_cursor_shape_visible(shape: CursorShape) -> bool {
    !matches!(shape, CursorShape::Hidden)
}
