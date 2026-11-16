//! Paleta de cores (`gsh` escuro e claro), escala tipográfica e cores do terminal.
//!
//! Fonte única de verdade para tokens visuais do `gsh-app`, espelhando
//! `docs/design-system.md`, `app_colors.dart`, `theme_registry.dart` (`gshTheme`),
//! `app_typography.dart` e `terminal_theme.dart` do GSH Desktop (Flutter).

use gsh_term::{indexed_to_rgb, Cell, CellColor, NamedColor, Rgb};
use iced::font::{Style as FontStyle, Weight};
use iced::{Color, Font, Padding};
use std::sync::atomic::{AtomicBool, Ordering};

/// Estrutura completa de tokens de superfície, borda, texto, marca e semântica
/// de uma variante do tema `gsh` (espelha `AppColors` em `app_colors.dart`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GshPalette {
    pub bg: Color,
    pub panel: Color,
    pub panel2: Color,
    pub panel3: Color,
    pub border: Color,
    pub border2: Color,
    pub text: Color,
    pub text2: Color,
    pub text3: Color,
    pub text4: Color,
    pub accent: Color,
    pub accent_soft: Color,
    pub accent_text: Color,
    pub online: Color,
    pub ok: Color,
    pub error: Color,
    pub warn: Color,
    pub edited: Color,
    pub edited_bg: Color,
    pub git_staged: Color,
    pub git_untracked: Color,
    pub git_deleted: Color,
    pub git_conflict: Color,
    pub scrim: Color,
    pub shadow: Color,
}

const fn rgba8(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    )
}

impl GshPalette {
    /// Variante escura do tema padrão `gsh` (`docs/design-system.md` / `gshTheme.dark`).
    pub const DARK: Self = Self {
        bg: Color::from_rgb8(0x16, 0x18, 0x19),
        panel: Color::from_rgb8(0x1E, 0x21, 0x22),
        panel2: Color::from_rgb8(0x26, 0x2A, 0x2B),
        panel3: Color::from_rgb8(0x2F, 0x34, 0x35),
        border: Color::from_rgb8(0x2A, 0x2E, 0x2F),
        border2: Color::from_rgb8(0x3A, 0x40, 0x42),
        text: Color::from_rgb8(0xF1, 0xF5, 0xF9),
        text2: Color::from_rgb8(0xA3, 0xAC, 0xAE),
        text3: Color::from_rgb8(0x7A, 0x85, 0x88),
        text4: Color::from_rgb8(0x4F, 0x58, 0x5A),
        accent: Color::from_rgb8(0xC9, 0xD3, 0xD6),
        accent_soft: rgba8(0x7A, 0x85, 0x88, 0x33),
        accent_text: Color::from_rgb8(0xDC, 0xE3, 0xE5),
        online: Color::from_rgb8(0x3F, 0xB8, 0x68),
        ok: Color::from_rgb8(0x3F, 0xB8, 0x68),
        error: Color::from_rgb8(0xE5, 0x48, 0x4D),
        warn: Color::from_rgb8(0xE0, 0xA3, 0x3A),
        edited: Color::from_rgb8(0xC9, 0x8A, 0x2B),
        edited_bg: Color::from_rgb8(0x2A, 0x21, 0x0F),
        git_staged: Color::from_rgb8(0x3F, 0xB8, 0x68),
        git_untracked: Color::from_rgb8(0x4F, 0x9D, 0xF0),
        git_deleted: Color::from_rgb8(0xE5, 0x48, 0x4D),
        git_conflict: Color::from_rgb8(0xF0, 0x88, 0x3E),
        scrim: rgba8(0x00, 0x00, 0x00, 0x99),
        shadow: rgba8(0x00, 0x00, 0x00, 0x55),
    };

    /// Variante clara do tema `gsh` (`docs/design-system.md` / `gshTheme.light`).
    pub const LIGHT: Self = Self {
        bg: Color::from_rgb8(0xE3, 0xE7, 0xE8),
        panel: Color::from_rgb8(0xED, 0xF0, 0xF1),
        panel2: Color::from_rgb8(0xF8, 0xFA, 0xFA),
        panel3: Color::from_rgb8(0xDA, 0xDF, 0xE0),
        border: Color::from_rgb8(0xD3, 0xD8, 0xDA),
        border2: Color::from_rgb8(0xBE, 0xC5, 0xC8),
        text: Color::from_rgb8(0x1A, 0x1A, 0x1A),
        text2: Color::from_rgb8(0x4A, 0x54, 0x56),
        text3: Color::from_rgb8(0x6B, 0x75, 0x78),
        text4: Color::from_rgb8(0xA3, 0xAC, 0xAE),
        accent: Color::from_rgb8(0x43, 0x4B, 0x4D),
        accent_soft: rgba8(0x43, 0x4B, 0x4D, 0x1F),
        accent_text: Color::from_rgb8(0x2E, 0x35, 0x36),
        online: Color::from_rgb8(0x2E, 0x9E, 0x54),
        ok: Color::from_rgb8(0x2E, 0x9E, 0x54),
        error: Color::from_rgb8(0xD3, 0x2F, 0x2F),
        warn: Color::from_rgb8(0xB7, 0x79, 0x1F),
        edited: Color::from_rgb8(0xB7, 0x79, 0x1F),
        edited_bg: Color::from_rgb8(0xFB, 0xF1, 0xDC),
        git_staged: Color::from_rgb8(0x2E, 0x9E, 0x54),
        git_untracked: Color::from_rgb8(0x2F, 0x6F, 0xF0),
        git_deleted: Color::from_rgb8(0xD3, 0x2F, 0x2F),
        git_conflict: Color::from_rgb8(0xD9, 0x73, 0x0D),
        scrim: rgba8(0x00, 0x00, 0x00, 0x66),
        shadow: rgba8(0x00, 0x00, 0x00, 0x33),
    };
}

/// Brilho efetivo do tema `gsh` (escuro ou claro).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeVariant {
    Dark,
    Light,
}

impl ThemeVariant {
    /// Paleta de tokens do brilho.
    pub const fn palette(self) -> GshPalette {
        match self {
            Self::Dark => GshPalette::DARK,
            Self::Light => GshPalette::LIGHT,
        }
    }

    /// Resolve o modo gravado no config. `system` segue o escuro: o app não
    /// detecta o brilho do sistema operacional.
    pub const fn from_mode(mode: gsh_config::schema::AppThemeMode) -> Self {
        match mode {
            gsh_config::schema::AppThemeMode::Light => Self::Light,
            gsh_config::schema::AppThemeMode::Dark | gsh_config::schema::AppThemeMode::System => {
                Self::Dark
            }
        }
    }
}

static ACTIVE_LIGHT: AtomicBool = AtomicBool::new(false);

/// Troca a variante ativa; vale para o próximo `view`, sem reiniciar.
pub fn set_active(variant: ThemeVariant) {
    ACTIVE_LIGHT.store(variant == ThemeVariant::Light, Ordering::Relaxed);
}

/// Variante ativa no momento (escuro por padrão).
pub fn active() -> ThemeVariant {
    if ACTIVE_LIGHT.load(Ordering::Relaxed) {
        ThemeVariant::Light
    } else {
        ThemeVariant::Dark
    }
}

/// Paleta ativa. Leia cores daqui em `view`, nunca em constante, para que a
/// troca de tema apareça ao vivo.
pub fn palette() -> GshPalette {
    active().palette()
}

/// Razão de contraste WCAG 2.x entre duas cores opacas (de 1,0 a 21,0).
pub fn contrast_ratio(a: Color, b: Color) -> f32 {
    fn linear(channel: f32) -> f32 {
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }
    fn luminance(c: Color) -> f32 {
        0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
    }
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// Cor de texto legível sobre `background` (`onColor` do Flutter): branco ou preto, o de maior contraste.
pub fn on_color(background: Color) -> Color {
    if contrast_ratio(background, Color::WHITE) >= contrast_ratio(background, Color::BLACK) {
        Color::WHITE
    } else {
        Color::BLACK
    }
}

// Tokens principais do tema `gsh` escuro (valores de compilação; o tema ativo vem de `palette()`)
pub const BG: Color = GshPalette::DARK.bg;
pub const PANEL: Color = GshPalette::DARK.panel;
pub const PANEL2: Color = GshPalette::DARK.panel2;
pub const PANEL3: Color = GshPalette::DARK.panel3;
pub const BORDER: Color = GshPalette::DARK.border;
pub const BORDER2: Color = GshPalette::DARK.border2;
pub const TEXT: Color = GshPalette::DARK.text;
pub const TEXT2: Color = GshPalette::DARK.text2;
pub const TEXT3: Color = GshPalette::DARK.text3;
pub const TEXT4: Color = GshPalette::DARK.text4;
pub const ACCENT: Color = GshPalette::DARK.accent;
pub const ACCENT_SOFT: Color = GshPalette::DARK.accent_soft;
pub const ACCENT_TEXT: Color = GshPalette::DARK.accent_text;
pub const ONLINE: Color = GshPalette::DARK.online;
pub const OK: Color = GshPalette::DARK.ok;
pub const ERROR: Color = GshPalette::DARK.error;
pub const WARN: Color = GshPalette::DARK.warn;
pub const EDITED: Color = GshPalette::DARK.edited;
pub const EDITED_BG: Color = GshPalette::DARK.edited_bg;
pub const GIT_STAGED: Color = GshPalette::DARK.git_staged;
pub const GIT_UNTRACKED: Color = GshPalette::DARK.git_untracked;
pub const GIT_DELETED: Color = GshPalette::DARK.git_deleted;
pub const GIT_CONFLICT: Color = GshPalette::DARK.git_conflict;
pub const SCRIM: Color = GshPalette::DARK.scrim;
pub const SHADOW: Color = GshPalette::DARK.shadow;

/// Borda translúcida de aviso usada nos cards de permissão (`WARN` com 45% de opacidade).
pub const WARN_BORDER: Color = Color { a: 0.45, ..WARN };
/// Fundo translúcido para linhas adicionadas (`+`) no diff (`GIT_STAGED` com 14% de opacidade).
pub const GIT_ADDED_BG: Color = Color {
    a: 0.14,
    ..GIT_STAGED
};
/// Fundo translúcido para linhas removidas (`-`) no diff (`GIT_DELETED` com 14% de opacidade).
pub const GIT_REMOVED_BG: Color = Color {
    a: 0.14,
    ..GIT_DELETED
};
/// Cor de hover para botão primário acentuado.
pub const ACCENT_HOVER: Color = ACCENT_TEXT;
/// Cor de texto sobre botão primário `ACCENT` claro (`#C9D3D6`).
pub const ON_ACCENT: Color = BG;

// Aliases de compatibilidade com módulos anteriores
pub const SHELL_BG: Color = BG;
pub const SHELL_PANEL: Color = PANEL;
pub const SHELL_MUTED: Color = TEXT3;
pub const SHELL_TEXT: Color = TEXT;

// Paleta do terminal (`terminal_theme.dart` + overrides de `gshTheme` em `theme_registry.dart`)
// Em `theme_registry.dart` (`_variant`), o fundo do terminal é unificado com `ui.panel` (`#1E2122`),
// cursor usa `ACCENT` (`#C9D3D6`) e seleção usa `#7A858855`.
pub const TERM_RAW_BG: Color = Color::from_rgb8(0x18, 0x18, 0x1B);
pub const TERM_BG: Color = PANEL;
pub const TERM_FG: Color = Color::from_rgb8(0xEC, 0xEC, 0xEF);
pub const TERM_CURSOR: Color = ACCENT;
pub const TERM_CURSOR_TEXT: Color = BG;
pub const TERM_SELECTION_BG: Color = rgba8(0x7A, 0x85, 0x88, 0x55);
/// Paleta GitHub Dark de syntax_colors.dart.
pub const SYNTAX_BASE: Color = Color::from_rgb8(0xc9, 0xd1, 0xd9);
pub const SYNTAX_COMMENT: Color = Color::from_rgb8(0x8b, 0x94, 0x9e);
pub const SYNTAX_KEYWORD: Color = Color::from_rgb8(0xff, 0x7b, 0x72);
pub const SYNTAX_STRING: Color = Color::from_rgb8(0xa5, 0xd6, 0xff);
pub const SYNTAX_NUMBER: Color = Color::from_rgb8(0x79, 0xc0, 0xff);
pub const SYNTAX_TYPE: Color = Color::from_rgb8(0xff, 0xa6, 0x57);
pub const SYNTAX_BUILTIN: Color = Color::from_rgb8(0x79, 0xc0, 0xff);
pub const SYNTAX_FUNCTION: Color = Color::from_rgb8(0xd2, 0xa8, 0xff);
pub const SYNTAX_VARIABLE: Color = Color::from_rgb8(0xff, 0xa6, 0x57);
pub const SYNTAX_META: Color = Color::from_rgb8(0x7e, 0xe7, 0x87);
pub const EDITOR_PAD: f32 = 14.0;
pub const GUTTER_PAD: f32 = 14.0;
pub const GUTTER_ICON: f32 = 16.0;
pub const EDITOR_VIEWPORT_FALLBACK: f32 = 600.0;
pub const EDITOR_EXTRA_HEIGHT: f32 = 30.0;
pub const EDITOR_MIN_HEIGHT: f32 = 100.0;
pub const EDITOR_PAGE_MAX_LINES: usize = 200;
/// Largura conservadora por coluna para permitir rolagem de linhas sem quebra.
pub const EDITOR_COLUMN_WIDTH: f32 = SIZE_MONO;
pub const EDITOR_GUTTER: Color = Color {
    a: 0.4,
    ..SYNTAX_BASE
};
pub fn syntax_color(token: gsh_editor::highlight::Token) -> Color {
    use gsh_editor::highlight::Token;
    match token {
        Token::Base => SYNTAX_BASE,
        Token::Comment => SYNTAX_COMMENT,
        Token::Keyword => SYNTAX_KEYWORD,
        Token::String => SYNTAX_STRING,
        Token::Number => SYNTAX_NUMBER,
        Token::Type => SYNTAX_TYPE,
        Token::Builtin => SYNTAX_BUILTIN,
        Token::Function => SYNTAX_FUNCTION,
        Token::Variable => SYNTAX_VARIABLE,
        Token::Meta => SYNTAX_META,
    }
}
pub const TERM_SCROLLBAR: Color = Color { a: 0.45, ..TEXT3 };

// Famílias tipográficas (`app_typography.dart`)
// - Hanken Grotesk (corpo/UI) e Montserrat (títulos/abas): instâncias estáticas
//   por peso (Regular 400, Medium 500, SemiBold 600, Bold 700) geradas via
//   fontTools a partir dos `-VF.ttf` e embutidas no binário via `include_bytes!`.
// - JetBrainsMono NF: instalada no sistema (`~/.local/share/fonts/JetBrainsMono`).
pub const FONT_FAMILY_MONO: &str = "JetBrainsMono NF";
pub const FONT_FAMILY_BODY: &str = "Hanken Grotesk";
pub const FONT_FAMILY_TITLE: &str = "Montserrat";

// Aliases mantidos para referência direta às especificações tipográficas
pub const FONT_FAMILY_BODY_SPEC: &str = FONT_FAMILY_BODY;
pub const FONT_FAMILY_TITLE_SPEC: &str = FONT_FAMILY_TITLE;

pub const HANKEN_REGULAR_TTF: &[u8] = include_bytes!("../assets/fonts/HankenGrotesk-Regular.ttf");
pub const HANKEN_MEDIUM_TTF: &[u8] = include_bytes!("../assets/fonts/HankenGrotesk-Medium.ttf");
pub const HANKEN_SEMIBOLD_TTF: &[u8] = include_bytes!("../assets/fonts/HankenGrotesk-SemiBold.ttf");
pub const HANKEN_BOLD_TTF: &[u8] = include_bytes!("../assets/fonts/HankenGrotesk-Bold.ttf");

pub const MONTSERRAT_REGULAR_TTF: &[u8] = include_bytes!("../assets/fonts/Montserrat-Regular.ttf");
pub const MONTSERRAT_MEDIUM_TTF: &[u8] = include_bytes!("../assets/fonts/Montserrat-Medium.ttf");
pub const MONTSERRAT_SEMIBOLD_TTF: &[u8] =
    include_bytes!("../assets/fonts/Montserrat-SemiBold.ttf");
pub const MONTSERRAT_BOLD_TTF: &[u8] = include_bytes!("../assets/fonts/Montserrat-Bold.ttf");

/// Todas as faces estáticas embutidas para registro no `iced::application`.
pub const MATERIAL_SYMBOLS_TTF: &[u8] =
    include_bytes!("../assets/fonts/MaterialSymbolsOutlined.ttf");

pub const EMBEDDED_FONTS: [&[u8]; 9] = [
    HANKEN_REGULAR_TTF,
    HANKEN_MEDIUM_TTF,
    HANKEN_SEMIBOLD_TTF,
    HANKEN_BOLD_TTF,
    MONTSERRAT_REGULAR_TTF,
    MONTSERRAT_MEDIUM_TTF,
    MONTSERRAT_SEMIBOLD_TTF,
    MONTSERRAT_BOLD_TTF,
    MATERIAL_SYMBOLS_TTF,
];

pub const FONT_MONO: Font = Font::with_name(FONT_FAMILY_MONO);
/// Ícones (Material Symbols Outlined, subset com os glifos usados).
pub const FONT_ICON: Font = Font::with_name("Material Symbols Outlined");
pub const FONT_MONO_MEDIUM: Font = Font {
    weight: Weight::Medium,
    ..Font::with_name(FONT_FAMILY_MONO)
};
pub const FONT_MONO_BOLD: Font = Font {
    weight: Weight::Bold,
    ..Font::with_name(FONT_FAMILY_MONO)
};
pub const FONT_MONO_ITALIC: Font = Font {
    style: FontStyle::Italic,
    ..Font::with_name(FONT_FAMILY_MONO)
};

pub const FONT_BODY: Font = Font::with_name(FONT_FAMILY_BODY);
pub const FONT_BODY_MEDIUM: Font = Font {
    weight: Weight::Medium,
    ..Font::with_name(FONT_FAMILY_BODY)
};
pub const FONT_BODY_SEMIBOLD: Font = Font {
    weight: Weight::Semibold,
    ..Font::with_name(FONT_FAMILY_BODY)
};
pub const FONT_BODY_BOLD: Font = Font {
    weight: Weight::Bold,
    ..Font::with_name(FONT_FAMILY_BODY)
};

pub const FONT_TITLE_REGULAR: Font = Font::with_name(FONT_FAMILY_TITLE);
pub const FONT_TAB: Font = Font {
    weight: Weight::Medium,
    ..Font::with_name(FONT_FAMILY_TITLE)
};
pub const FONT_DISPLAY: Font = Font {
    weight: Weight::Semibold,
    ..Font::with_name(FONT_FAMILY_TITLE)
};
pub const FONT_TITLE: Font = Font {
    weight: Weight::Semibold,
    ..Font::with_name(FONT_FAMILY_TITLE)
};
pub const FONT_TITLE_BOLD: Font = Font {
    weight: Weight::Bold,
    ..Font::with_name(FONT_FAMILY_TITLE)
};

// Escala tipográfica (`app_typography.dart`)
// Medidas de SettingsNav, SettingsPanelBody e _SettingsHeader no Flutter.
pub const SETTINGS_NAV_WIDTH: f32 = 220.0;
pub const SETTINGS_NAV_PADDING: u16 = 12;
pub const SETTINGS_NAV_ITEM_HORIZONTAL: u16 = 9;
pub const SETTINGS_NAV_ITEM_VERTICAL: u16 = 7;
pub const SETTINGS_NAV_ITEM_GAP: f32 = 12.0;
pub const SETTINGS_NAV_ITEM_SPACING: f32 = 1.0;
pub const SETTINGS_NAV_ICON_SIZE: f32 = 17.0;
pub const SETTINGS_NAV_TEXT_SIZE: f32 = 14.0;
pub const SETTINGS_NAV_RADIUS: f32 = 7.0;
pub const SETTINGS_PANEL_HORIZONTAL: u16 = 28;
pub const SETTINGS_PANEL_TOP: u16 = 26;
pub const SETTINGS_PANEL_BOTTOM: u16 = 48;
pub const SETTINGS_PANEL_MAX_WIDTH: f32 = 1010.0;
pub const SETTINGS_SECTION_TITLE_SIZE: f32 = 15.0;
pub const SETTINGS_SECTION_TITLE_GAP: f32 = 14.0;
pub const SETTINGS_SECTION_GAP: f32 = 36.0;
pub const SETTINGS_ROW_VERTICAL: u16 = 12;
pub const SETTINGS_ROW_GAP: f32 = 40.0;
pub const SETTINGS_HEADER_BUTTON: f32 = 30.0;
pub const SETTINGS_HEADER_ICON: f32 = 18.0;
pub const SETTINGS_HEADER_RADIUS: f32 = 6.0;
pub const SETTINGS_HEADER_GAP: f32 = 8.0;

pub const SIZE_DISPLAY: f32 = 22.0;
pub const SIZE_TOPBAR_TITLE: f32 = 14.0;
pub const SIZE_ICON: f32 = 16.0;
pub const SIZE_TOPBAR_ICON: f32 = 17.0;
pub const SIZE_TITLE: f32 = 13.0;
pub const SIZE_TAB: f32 = 12.5;
pub const SIZE_BODY: f32 = 14.5;
pub const SIZE_RAIL_ITEM: f32 = 13.5;
pub const SIZE_MONO: f32 = 13.0;
pub const SIZE_LABEL: f32 = 12.0;
pub const SIZE_SECTION_HEADER: f32 = 11.0;
pub const SIZE_BADGE: f32 = 9.5;

pub const LINE_HEIGHT_DISPLAY: f32 = 1.25;
pub const LINE_HEIGHT_BODY: f32 = 1.6;
pub const LINE_HEIGHT_LABEL: f32 = 1.3;
pub const LINE_HEIGHT_MONO: f32 = 1.55;

// Métricas padrão da célula do terminal
pub const TERM_FONT: Font = FONT_MONO;
pub const TERM_FONT_BOLD: Font = FONT_MONO_BOLD;
pub const TERM_FONT_ITALIC: Font = FONT_MONO_ITALIC;
pub const TERM_FONT_SIZE: f32 = SIZE_MONO;
pub const TERM_CELL_WIDTH: f32 = 8.0;
pub const TERM_CELL_HEIGHT: f32 = 17.0;
pub const TERM_PADDING_X: f32 = 8.0;
pub const TERM_PADDING_Y: f32 = 8.0;

// Métricas do chrome (`gsh_topbar.dart`, `projects_rail.dart`, `pane_view.dart`, `file_tree_panel.dart`)
pub const TOPBAR_HEIGHT: f32 = 46.0;
/// Largura dos 3 botões de janela (28px cada + 2 vãos de 2px), para a divisória inferior da barra seguir até a borda.
pub const WINDOW_CONTROLS_WIDTH: f32 = 88.0;
/// Largura livre mínima do centro da barra (`LayoutBuilder` no Flutter).
pub const TOPBAR_CENTER_MIN_WIDTH: f32 = 160.0;
/// Ícone e espaço da pílula `_GitBadge` do Flutter.
pub const GIT_BADGE_ICON_SIZE: f32 = 9.0;
pub const GIT_BADGE_ICON_GAP: f32 = 3.0;
pub const TAB_STRIP_HEIGHT: f32 = 40.0;
pub const TAB_WIDTH: f32 = 188.0;
pub const PANE_TAB_ICON_SIZE: f32 = 13.0;
pub const PANE_FILE_TAB_ICON_SIZE: f32 = 15.0;
pub const PANE_TAB_LEFT_PADDING: f32 = 11.0;
pub const PANE_TAB_RIGHT_PADDING: f32 = 7.0;
pub const PANE_TAB_ICON_GAP: f32 = 7.0;
pub const PANE_TAB_CLOSE_GAP: f32 = 10.0;
pub const PANE_TAB_CLOSE_ICON_SIZE: f32 = 12.0;
pub const PANE_TAB_CLOSE_PADDING: f32 = 3.0;
pub const PANE_TAB_DIRTY_DOT_SIZE: f32 = 8.0;
pub const PANE_TAB_INDICATOR_HEIGHT: f32 = 2.0;
pub const PANE_TAB_ADD_WIDTH: f32 = 32.0;
pub const PANE_TAB_ADD_ICON_SIZE: f32 = 14.0;
pub const PANE_TOOL_SIZE: f32 = 13.0;
pub const PANE_TOOL_GAP: f32 = 12.0;
/// Espessura do traço dos ícones da barra do pane, como fração do lado.
pub const PANE_TOOL_STROKE_RATIO: f32 = 0.10;
/// Divisor entre panes (`PaneDivider`): linha de 1px dentro de uma área de arraste de 12px.
pub const PANE_DIVIDER_LINE: f32 = 1.0;
pub const PANE_DIVIDER_HANDLE: f32 = 12.0;
/// Diálogo de confirmação (`AlertDialog` do shadcn no Flutter).
pub const DIALOG_CONTENT_WIDTH: f32 = 380.0;
pub const DIALOG_PADDING: f32 = 24.0;
pub const DIALOG_GAP: f32 = 16.0;
pub const DIALOG_TITLE_GAP: f32 = 8.0;
pub const DIALOG_ACTIONS_GAP: f32 = 8.0;
pub const DIALOG_TITLE_SIZE: f32 = 15.0;
pub const DIALOG_MESSAGE_SIZE: f32 = 13.5;
pub const DIALOG_BUTTON_SIZE: f32 = 14.0;
pub const DIALOG_RADIUS: f32 = 12.0;
pub const FILE_TREE_ROW_HEIGHT: f32 = 26.0;
pub const FILE_TREE_ROW_INDENT: f32 = 14.0;
pub const FILE_TREE_ROW_PADDING: f32 = 6.0;
pub const FILE_TREE_SCROLL_PADDING_VERTICAL: f32 = 8.0;
pub const FILE_TREE_CHEVRON_WIDTH: f32 = 14.0;
pub const FILE_TREE_CHEVRON_SIZE: f32 = 15.0;
pub const FILE_TREE_CHEVRON_GAP: f32 = 2.0;
pub const FILE_TREE_ICON_SIZE: f32 = 16.0;
pub const FILE_TREE_ICON_GAP: f32 = 7.0;
pub const FILE_TREE_NAME_SIZE: f32 = 13.0;
pub const FILE_TREE_ROW_RADIUS: f32 = 5.0;
pub const FILE_PANEL_HEADER_TITLE_SIZE: f32 = 10.0;
pub const FILE_PANEL_HEADER_ACTION_SIZE: f32 = 14.0;
pub const FILE_PANEL_HEADER_ACTION_PADDING: f32 = 3.0;
pub const FILE_PANEL_HEADER_LEFT: f32 = 12.0;
pub const FILE_PANEL_HEADER_TOP: f32 = 10.0;
pub const FILE_PANEL_HEADER_RIGHT: f32 = 6.0;
pub const FILE_PANEL_HEADER_BOTTOM: f32 = 4.0;
// Aba Busca (`content_search_panel.dart`)
pub const SEARCH_HEADER_HEIGHT: f32 = 34.0;
pub const SEARCH_HEADER_LEFT: f32 = 12.0;
pub const SEARCH_HEADER_RIGHT: f32 = 8.0;
pub const SEARCH_HEADER_GAP: f32 = 8.0;
pub const SEARCH_PILL_TEXT_SIZE: f32 = 10.5;
pub const SEARCH_PILL_PADDING: [f32; 2] = [1.0, 7.0];
pub const SEARCH_PILL_RADIUS: f32 = 10.0;
pub const SEARCH_SPINNER_SIZE: f32 = 12.0;
pub const SEARCH_FIELD_PADDING: Padding = Padding {
    top: 2.0,
    right: 10.0,
    bottom: 10.0,
    left: 10.0,
};
pub const SEARCH_INPUT_SIZE: f32 = 13.0;
pub const SEARCH_INPUT_PADDING: [f32; 2] = [6.0, 8.0];
pub const SEARCH_INPUT_RADIUS: f32 = 6.0;
pub const SEARCH_FIELD_GAP: f32 = 6.0;
pub const SEARCH_TOGGLE_SIZE: f32 = 26.0;
pub const SEARCH_TOGGLE_TEXT_SIZE: f32 = 11.5;
pub const SEARCH_TOGGLE_ACTIVE_ALPHA: f32 = 0.18;
pub const SEARCH_MESSAGE_PADDING: Padding = Padding {
    top: 4.0,
    right: 12.0,
    bottom: 12.0,
    left: 12.0,
};
pub const SEARCH_LIST_BOTTOM: f32 = 8.0;
pub const SEARCH_GROUP_PADDING: [f32; 2] = [4.0, 8.0];
pub const SEARCH_GROUP_ICON_SIZE: f32 = 15.0;
pub const SEARCH_GROUP_CHEVRON_GAP: f32 = 2.0;
pub const SEARCH_GROUP_GAP: f32 = 6.0;
pub const SEARCH_GROUP_NAME_SIZE: f32 = 12.5;
pub const SEARCH_GROUP_DIR_SIZE: f32 = 11.0;
pub const SEARCH_COUNT_TEXT_SIZE: f32 = 10.0;
pub const SEARCH_COUNT_PADDING: [f32; 2] = [1.0, 6.0];
pub const SEARCH_COUNT_RADIUS: f32 = 9.0;
pub const SEARCH_MATCH_PADDING: Padding = Padding {
    top: 2.0,
    right: 8.0,
    bottom: 2.0,
    left: 14.0,
};
pub const SEARCH_MATCH_NUMBER_WIDTH: f32 = 34.0;
pub const SEARCH_MATCH_GAP: f32 = 10.0;
pub const SEARCH_MATCH_TEXT_SIZE: f32 = 11.5;
pub const SEARCH_MATCH_HIGHLIGHT_ALPHA: f32 = 0.28;
pub const SEARCH_MATCH_RADIUS: f32 = 7.0;
/// Rolagem do reveal: a linha fica 80px abaixo do topo (`code_editor.dart:_reveal`).
pub const EDITOR_REVEAL_MARGIN: f32 = 80.0;
// Barra de abas do painel direito e aba Galeria (`file_tree_panel.dart`, `gallery_panel.dart`)
pub const RIGHT_TAB_BAR_LEFT: f32 = 14.0;
pub const RIGHT_TAB_BAR_RIGHT: f32 = 8.0;
pub const RIGHT_TAB_BUTTON_SIZE: f32 = 28.0;
pub const GALLERY_PADDING_SIDE: f32 = 10.0;
pub const GALLERY_PADDING_TOP: f32 = 4.0;
pub const GALLERY_PADDING_BOTTOM: f32 = 12.0;
pub const GALLERY_INTRO_PADDING_SIDE: f32 = 4.0;
pub const GALLERY_INTRO_PADDING_TOP: f32 = 4.0;
pub const GALLERY_INTRO_PADDING_BOTTOM: f32 = 10.0;
pub const GALLERY_CARD_GAP: f32 = 8.0;
pub const GALLERY_CARD_PADDING: f32 = 10.0;
pub const GALLERY_CARD_RADIUS: f32 = 8.0;
pub const GALLERY_ICON_SIZE: f32 = 28.0;
pub const GALLERY_ICON_GAP: f32 = 10.0;
pub const GALLERY_PATH_SIZE: f32 = 10.0;
pub const GALLERY_DESCRIPTION_SIZE: f32 = 11.0;
pub const GALLERY_DESCRIPTION_GAP: f32 = 3.0;
pub const RAIL_WIDTH: f32 = 252.0;
pub const FILES_PANEL_WIDTH: f32 = 260.0;

// Filtro por agente do painel Arquivos (`agent_file_marks.dart` e `showAppMenu`/shadcn `MenuPopup`).
pub const AGENT_DOT_SIZE: f32 = 6.0;
pub const AGENT_MAX_DOTS: usize = 3;
pub const AGENT_FILTER_ICON_SIZE: f32 = 14.0;
pub const AGENT_FILTER_PADDING: f32 = 3.0;
pub const AGENT_FILTER_RADIUS: f32 = 7.0;
pub const AGENT_FILTER_DOT_GAP: f32 = 3.0;
pub const AGENT_MARK_DOT_GAP: f32 = 2.0;
pub const AGENT_MARK_MORE_GAP: f32 = 3.0;
pub const AGENT_MARK_MORE_SIZE: f32 = 10.0;
pub const AGENT_MARK_WARN_SIZE: f32 = 12.0;
pub const AGENT_MARK_WARN_GAP: f32 = 3.0;
pub const AGENT_MENU_MIN_WIDTH: f32 = 240.0;
pub const AGENT_MENU_MAX_WIDTH: f32 = 320.0;
pub const AGENT_MENU_PADDING: f32 = 4.0;
pub const AGENT_MENU_RADIUS: f32 = 6.0;
pub const AGENT_MENU_GAP: f32 = 8.0;
pub const AGENT_MENU_ITEM_LEFT: f32 = 8.0;
pub const AGENT_MENU_ITEM_VERTICAL: f32 = 6.0;
pub const AGENT_MENU_ITEM_RIGHT: f32 = 6.0;
pub const AGENT_MENU_ICON_SIZE: f32 = 15.0;
pub const AGENT_MENU_TAB_ICON_SIZE: f32 = 13.0;
pub const AGENT_MENU_DOT_GAP: f32 = 6.0;
pub const AGENT_MENU_LEADING_WIDTH: f32 = 24.0;
pub const AGENT_MENU_LABEL_MAX: f32 = 160.0;
pub const AGENT_MENU_TEXT_SIZE: f32 = 13.0;
pub const AGENT_MENU_CHECK_SIZE: f32 = 14.0;
pub const AGENT_MENU_DIVIDER_VERTICAL: f32 = 4.0;
pub const AGENT_MENU_OFFSET: f32 = 4.0;
/// Margem do popover do shadcn até a borda da janela, quando ele precisa deslizar para caber.
pub const AGENT_MENU_EDGE_MARGIN: f32 = 8.0;
/// Altura do botão robô no cabeçalho (ícone 14px com linha de 1.3 + 2x3 de padding).
pub const AGENT_FILTER_BUTTON_HEIGHT: f32 = 24.0;
pub const AGENT_LIST_PADDING_VERTICAL: f32 = 8.0;
pub const AGENT_LIST_PADDING_HORIZONTAL: f32 = 6.0;
pub const AGENT_LIST_ROW_HEIGHT: f32 = 26.0;
pub const AGENT_LIST_ROW_PADDING: f32 = 6.0;
pub const AGENT_LIST_ROW_RADIUS: f32 = 5.0;
pub const AGENT_LIST_ICON_SIZE: f32 = 16.0;
pub const AGENT_LIST_ICON_GAP: f32 = 7.0;
pub const AGENT_LIST_NAME_SIZE: f32 = 13.0;
pub const AGENT_LIST_TIME_SIZE: f32 = 11.0;

/// Cor do agente da aba `tab_id` (hash + paleta do núcleo), igual em qualquer tema.
pub fn agent_color(tab_id: &str) -> Color {
    let c = gsh_core::agent_color::agent_color(tab_id);
    Color::from_rgb8(c.r, c.g, c.b)
}
pub const FOOTER_HEIGHT: f32 = 34.0;
pub const RADIUS_CARD: f32 = 7.0;
pub const RADIUS_BUTTON: f32 = 5.0;
pub const RADIUS_BADGE: f32 = 4.0;
pub const WORKSPACE_AVATAR_SIZE: f32 = 30.0;
pub const WORKSPACE_AVATAR_INITIAL_SCALE: f32 = 0.43;
pub const TOPBAR_ICON_BUTTON_SIZE: f32 = 28.0;
pub const FARM_STRIP_DOT_SIZE: f32 = 7.0;
pub const FARM_STRIP_COUNTER_GAP: f32 = 10.0;
pub const FARM_STRIP_DOT_GAP: f32 = 5.0;
pub const FARM_STRIP_BUTTON_PADDING: f32 = 7.0;
pub const FARM_STRIP_TRAILING_GAP: f32 = 4.0;
pub const FARM_STRIP_PILL_HEIGHT: f32 = 22.0;
pub const FARM_STRIP_PILL_RADIUS: f32 = 11.0;
pub const FARM_STRIP_PILL_PADDING: f32 = 9.0;

/// Converte um [`Rgb`] do `gsh-term` em [`iced::Color`].
pub const fn rgb_to_color(rgb: Rgb) -> Color {
    Color::from_rgb8(rgb.r, rgb.g, rgb.b)
}

/// Resolve uma [`NamedColor`] ANSI para [`iced::Color`] na paleta dark do GSH.
pub const fn named_to_color(named: NamedColor) -> Color {
    rgb_to_color(named.to_rgb())
}

/// Resolve um índice da paleta de 256 cores (`0..=255`) para [`iced::Color`].
pub const fn indexed_to_color(idx: u8) -> Color {
    rgb_to_color(indexed_to_rgb(idx))
}

/// Resolve um [`CellColor`] como cor de texto (foreground).
pub const fn cell_fg_color(color: CellColor) -> Color {
    match color {
        CellColor::Default => TERM_FG,
        CellColor::Named(named) => named_to_color(named),
        CellColor::Indexed(idx) => indexed_to_color(idx),
        CellColor::Rgb(rgb) => rgb_to_color(rgb),
    }
}

/// Resolve um [`CellColor`] como cor de fundo (background).
/// Retorna `None` quando a célula usa o fundo padrão do terminal.
pub const fn cell_bg_color(color: CellColor) -> Option<Color> {
    match color {
        CellColor::Default => None,
        CellColor::Named(named) => Some(named_to_color(named)),
        CellColor::Indexed(idx) => Some(indexed_to_color(idx)),
        CellColor::Rgb(rgb) => Some(rgb_to_color(rgb)),
    }
}

/// Retorna o par `(fg, bg)` efetivo de uma [`Cell`], aplicando `inverse` (SGR 7).
pub const fn effective_cell_colors(cell: &Cell) -> (Color, Option<Color>) {
    if cell.inverse {
        let fg = match cell_bg_color(cell.bg) {
            Some(color) => color,
            None => TERM_BG,
        };
        let bg = Some(cell_fg_color(cell.fg));
        (fg, bg)
    } else {
        (cell_fg_color(cell.fg), cell_bg_color(cell.bg))
    }
}

/// Seleciona a variante de [`Font`] para a célula conforme a flag `bold`.
pub const fn cell_font(bold: bool) -> Font {
    if bold {
        TERM_FONT_BOLD
    } else {
        TERM_FONT
    }
}

/// Serializa testes que mexem na variante global (os testes rodam em paralelo).
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gsh_dark_and_light_palettes_match_design_system_tokens() {
        assert_eq!(BG, Color::from_rgb8(0x16, 0x18, 0x19));
        assert_eq!(PANEL, Color::from_rgb8(0x1E, 0x21, 0x22));
        assert_eq!(PANEL2, Color::from_rgb8(0x26, 0x2A, 0x2B));
        assert_eq!(PANEL3, Color::from_rgb8(0x2F, 0x34, 0x35));
        assert_eq!(BORDER, Color::from_rgb8(0x2A, 0x2E, 0x2F));
        assert_eq!(BORDER2, Color::from_rgb8(0x3A, 0x40, 0x42));
        assert_eq!(TEXT, Color::from_rgb8(0xF1, 0xF5, 0xF9));
        assert_eq!(TEXT2, Color::from_rgb8(0xA3, 0xAC, 0xAE));
        assert_eq!(TEXT3, Color::from_rgb8(0x7A, 0x85, 0x88));
        assert_eq!(TEXT4, Color::from_rgb8(0x4F, 0x58, 0x5A));
        assert_eq!(ACCENT, Color::from_rgb8(0xC9, 0xD3, 0xD6));
        assert_eq!(ACCENT_TEXT, Color::from_rgb8(0xDC, 0xE3, 0xE5));
        assert_eq!(ONLINE, Color::from_rgb8(0x3F, 0xB8, 0x68));
        assert_eq!(OK, ONLINE);
        assert_eq!(WARN, Color::from_rgb8(0xE0, 0xA3, 0x3A));
        assert_eq!(ERROR, Color::from_rgb8(0xE5, 0x48, 0x4D));
        assert_eq!(GIT_STAGED, Color::from_rgb8(0x3F, 0xB8, 0x68));
        assert_eq!(GIT_UNTRACKED, Color::from_rgb8(0x4F, 0x9D, 0xF0));
        assert_eq!(GIT_DELETED, Color::from_rgb8(0xE5, 0x48, 0x4D));
        assert_eq!(GIT_CONFLICT, Color::from_rgb8(0xF0, 0x88, 0x3E));

        assert_eq!(GshPalette::LIGHT.bg, Color::from_rgb8(0xE3, 0xE7, 0xE8));
        assert_eq!(GshPalette::LIGHT.panel, Color::from_rgb8(0xED, 0xF0, 0xF1));
        assert_eq!(GshPalette::LIGHT.accent, Color::from_rgb8(0x43, 0x4B, 0x4D));

        assert_eq!(TERM_BG, PANEL);
        assert_eq!(TERM_FG, Color::from_rgb8(0xEC, 0xEC, 0xEF));
        assert_eq!(TERM_CURSOR, ACCENT);
        assert_eq!(TERM_CURSOR_TEXT, BG);
        assert!((TERM_SELECTION_BG.a - (0x55 as f32 / 255.0)).abs() < 1e-5);
    }

    #[test]
    fn light_palette_matches_design_system_samples() {
        let light = ThemeVariant::Light.palette();
        assert_eq!(light.panel2, Color::from_rgb8(0xF8, 0xFA, 0xFA));
        assert_eq!(light.text3, Color::from_rgb8(0x6B, 0x75, 0x78));
        assert_eq!(light.error, Color::from_rgb8(0xD3, 0x2F, 0x2F));
        assert_eq!(light.border2, Color::from_rgb8(0xBE, 0xC5, 0xC8));
        assert_eq!(light.accent_text, Color::from_rgb8(0x2E, 0x35, 0x36));
        assert!((light.accent_soft.a - (0x1F as f32 / 255.0)).abs() < 1e-5);
    }

    #[test]
    fn switching_variant_changes_active_background() {
        let _guard = test_guard();
        set_active(ThemeVariant::Dark);
        assert_eq!(palette().bg, GshPalette::DARK.bg);
        set_active(ThemeVariant::Light);
        assert_eq!(active(), ThemeVariant::Light);
        assert_eq!(palette().bg, Color::from_rgb8(0xE3, 0xE7, 0xE8));
        assert_ne!(palette().bg, GshPalette::DARK.bg);
        set_active(ThemeVariant::Dark);
        assert_eq!(palette().bg, GshPalette::DARK.bg);
    }

    #[test]
    fn config_modes_resolve_to_variants() {
        use gsh_config::schema::AppThemeMode;
        assert_eq!(
            ThemeVariant::from_mode(AppThemeMode::Light),
            ThemeVariant::Light
        );
        assert_eq!(
            ThemeVariant::from_mode(AppThemeMode::Dark),
            ThemeVariant::Dark
        );
        assert_eq!(
            ThemeVariant::from_mode(AppThemeMode::System),
            ThemeVariant::Dark
        );
    }

    #[test]
    fn text3_keeps_minimum_contrast_on_panel_and_panel2() {
        for variant in [ThemeVariant::Dark, ThemeVariant::Light] {
            let p = variant.palette();
            assert!(contrast_ratio(p.text3, p.panel) >= 3.8, "{variant:?} panel");
            assert!(
                contrast_ratio(p.text3, p.panel2) >= 3.8,
                "{variant:?} panel2"
            );
        }
    }

    #[test]
    fn typography_scale_follows_hierarchy() {
        assert_eq!(SIZE_DISPLAY, 22.0);
        assert_eq!(SIZE_BODY, 14.5);
        assert_eq!(SIZE_TOPBAR_TITLE, 14.0);
        assert_eq!(SIZE_RAIL_ITEM, 13.5);
        assert_eq!(SIZE_TITLE, 13.0);
        assert_eq!(SIZE_MONO, 13.0);
        assert_eq!(SIZE_TAB, 12.5);
        assert_eq!(SIZE_LABEL, 12.0);
        assert_eq!(SIZE_SECTION_HEADER, 11.0);
        assert_eq!(SIZE_BADGE, 9.5);
        assert_eq!(TERM_FONT_SIZE, SIZE_MONO);
    }

    #[test]
    fn named_fonts_and_static_weights_are_loaded_without_system_fallback() {
        use iced::font::Family;

        assert_eq!(FONT_BODY.family, Family::Name(FONT_FAMILY_BODY));
        assert_eq!(FONT_BODY.weight, Weight::Normal);
        assert_eq!(FONT_BODY_MEDIUM.family, Family::Name(FONT_FAMILY_BODY));
        assert_eq!(FONT_BODY_MEDIUM.weight, Weight::Medium);
        assert_eq!(FONT_BODY_SEMIBOLD.family, Family::Name(FONT_FAMILY_BODY));
        assert_eq!(FONT_BODY_SEMIBOLD.weight, Weight::Semibold);
        assert_eq!(FONT_BODY_BOLD.family, Family::Name(FONT_FAMILY_BODY));
        assert_eq!(FONT_BODY_BOLD.weight, Weight::Bold);

        assert_eq!(FONT_TITLE_REGULAR.family, Family::Name(FONT_FAMILY_TITLE));
        assert_eq!(FONT_TITLE_REGULAR.weight, Weight::Normal);
        assert_eq!(FONT_TAB.family, Family::Name(FONT_FAMILY_TITLE));
        assert_eq!(FONT_TAB.weight, Weight::Medium);
        assert_eq!(FONT_TITLE.family, Family::Name(FONT_FAMILY_TITLE));
        assert_eq!(FONT_TITLE.weight, Weight::Semibold);
        assert_eq!(FONT_DISPLAY.family, Family::Name(FONT_FAMILY_TITLE));
        assert_eq!(FONT_DISPLAY.weight, Weight::Semibold);
        assert_eq!(FONT_TITLE_BOLD.family, Family::Name(FONT_FAMILY_TITLE));
        assert_eq!(FONT_TITLE_BOLD.weight, Weight::Bold);

        assert_eq!(FONT_MONO.family, Family::Name(FONT_FAMILY_MONO));
        assert_ne!(FONT_BODY, Font::DEFAULT);
        assert_ne!(FONT_TITLE, Font::DEFAULT);
        assert_ne!(FONT_TAB, Font::DEFAULT);

        assert_eq!(EMBEDDED_FONTS.len(), 9);
        for bytes in EMBEDDED_FONTS {
            // Assinatura TrueType sfnt (0x00010000)
            assert_eq!(bytes.get(..4), Some(&[0x00, 0x01, 0x00, 0x00][..]));
        }
    }

    #[test]
    fn resolves_default_named_indexed_and_rgb_cell_colors() {
        assert_eq!(cell_fg_color(CellColor::Default), TERM_FG);
        assert_eq!(cell_bg_color(CellColor::Default), None);

        let red = cell_fg_color(CellColor::Named(NamedColor::Red));
        assert_eq!(red, Color::from_rgb8(0xE5, 0x48, 0x4D));
        assert_eq!(cell_bg_color(CellColor::Named(NamedColor::Red)), Some(red));

        // Índice ANSI 2 (Green) e cubo 6x6x6 (196 = vermelho vivo) e grayscale (255)
        assert_eq!(
            cell_fg_color(CellColor::Indexed(2)),
            named_to_color(NamedColor::Green)
        );
        assert_eq!(
            cell_fg_color(CellColor::Indexed(196)),
            Color::from_rgb8(255, 0, 0)
        );
        assert_eq!(
            cell_fg_color(CellColor::Indexed(255)),
            Color::from_rgb8(238, 238, 238)
        );

        let custom = CellColor::Rgb(Rgb::new(0x12, 0x34, 0x56));
        assert_eq!(cell_fg_color(custom), Color::from_rgb8(0x12, 0x34, 0x56));
        assert_eq!(
            cell_bg_color(custom),
            Some(Color::from_rgb8(0x12, 0x34, 0x56))
        );
    }

    #[test]
    fn inverse_cell_swaps_foreground_and_background() {
        let normal = Cell {
            ch: 'A',
            fg: CellColor::Named(NamedColor::Green),
            bg: CellColor::Default,
            bold: true,
            underline: false,
            inverse: false,
        };
        let (fg, bg) = effective_cell_colors(&normal);
        assert_eq!(fg, named_to_color(NamedColor::Green));
        assert_eq!(bg, None);

        let inverted = Cell {
            inverse: true,
            ..normal
        };
        let (inv_fg, inv_bg) = effective_cell_colors(&inverted);
        assert_eq!(inv_fg, TERM_BG);
        assert_eq!(inv_bg, Some(named_to_color(NamedColor::Green)));
        assert_eq!(cell_font(true), TERM_FONT_BOLD);
        assert_eq!(cell_font(false), TERM_FONT);
    }
}

// Paleta da cena: farm_world.dart, farm_sprites_crops.dart e office_world_paint.dart.
pub mod scene {
    // ARGB de office_sprites.dart (clay em farm_sprites.dart).
    pub mod office {
        pub const BEZEL: u32 = 0xff2a2f3a;
        pub const STAND: u32 = 0xff5a606c;
        pub const FOOT: u32 = 0xff7c828e;
        pub const SCREEN: u32 = 0xff1e2b3d;
        pub const CURSOR: u32 = 0xffe8edf5;
        pub const INK: [u32; 5] = [0xff7fb4ff, 0xff6fcf8b, 0xfff2b43a, 0xffc99bf0, 0xff9fd0ee];
        pub const CLAY: u32 = 0xffd97757;
        pub const CREAM: u32 = 0xfffff1cf;
        pub const SHADOW: u32 = 0x2e28241e;
        pub const PRINTER_INK: u32 = 0xff3a404c;
        pub const BODY: u32 = 0xffdfe2e6;
        pub const BODY_LIGHT: u32 = 0xfff4f5f7;
        pub const BODY_DARK: u32 = 0xffc3c8cf;
        pub const TRAY: u32 = 0xfff2f4f6;
        pub const WHITE: u32 = 0xffffffff;
        pub const LED_OFF: u32 = 0xff4a6a52;
        pub const LED_ON: u32 = 0xff7cfc9a;
        pub const PAPER_EDGE: u32 = 0xff9aa3b2;
    }

    use iced::Color;
    // ARGB de farm_sprites_crops.dart; Pix converte para RGBA na rasterização.
    pub const CROP_GREEN_DARK: u32 = 0xff3f8f35;
    pub const CROP_GREEN: u32 = 0xff6fcf5b;
    pub const CROP_GREEN_LIGHT: u32 = 0xffa6e27a;
    pub const CROP_SEED_SOIL: u32 = 0xff4a3220;
    pub const CROP_WILT_STEM: u32 = 0xff7a6a3a;
    pub const CROP_WILT_TIP: u32 = 0xff5c4b27;
    pub const CROP_WILT_LEAF: u32 = 0xff8a7a45;
    pub const CROP_FRUIT: u32 = 0xffd97757;
    pub const CROP_FRUIT_LIGHT: u32 = 0xfff2a07a;
    pub const CROP_SPARK_CENTER: u32 = 0xfffff1cf;
    pub const CROP_SPARK_CROSS: u32 = 0xffe8845f;
    pub const GRASS: Color = Color::from_rgb8(0x64, 0x84, 0x36);
    pub const GRASS_DARK: Color = Color::from_rgb8(0x57, 0x75, 0x30);
    pub const SOIL: Color = Color::from_rgb8(0x6b, 0x4a, 0x2e);
    pub const SOIL_DARK: Color = Color::from_rgb8(0x3f, 0x29, 0x15);
    pub const SOIL2: Color = Color::from_rgb8(0x57, 0x39, 0x1f);
    pub const SOIL3: Color = Color::from_rgb8(0x7d, 0x5a, 0x3a);
    pub const PLOT_EDGE: Color = Color::from_rgb8(0x4f, 0x34, 0x20);
    pub const PLOT_RIDGE: Color = Color::from_rgb8(0x8e, 0x6a, 0x46);
    pub const PLOT_FLECK: Color = Color::from_rgb8(0x9a, 0x76, 0x50);
    pub const FLOOR: Color = Color::from_rgb8(0xe5, 0xdd, 0xcd);
    pub const CARPET: Color = Color::from_rgb8(0xbf, 0xc9, 0xd4);
    pub const WALL: Color = Color::from_rgb8(0x3a, 0x40, 0x4c);
    pub const DESK: Color = Color::from_rgb8(0xf7, 0xf7, 0xf4);
    pub const CHAIR: Color = Color::from_rgb8(0x50, 0x79, 0xc4);
    pub const WOOD: Color = Color::from_rgb8(0xc9, 0x9c, 0x66);
    pub const WATER: Color = Color::from_rgb8(0x3f, 0x7f, 0xb8);
    pub const BARN: Color = Color::from_rgb8(0x9b, 0x45, 0x32);
    pub const LEAF_DARK: Color = Color::from_rgb8(0x3f, 0x8f, 0x35);
    pub const LEAF: Color = Color::from_rgb8(0x6f, 0xcf, 0x5b);
    pub const LEAF_LIGHT: Color = Color::from_rgb8(0xa6, 0xe2, 0x7a);
    pub const CROP: Color = Color::from_rgb8(0xd9, 0x77, 0x57);
    pub const CROP_LIGHT: Color = Color::from_rgb8(0xf2, 0xa0, 0x7a);
    pub const MONITOR: Color = Color::from_rgb8(0x2a, 0x2f, 0x3a);
    pub const SCREEN: Color = Color::from_rgb8(0x1b, 0x1f, 0x28);
    pub const REFLECTION: Color = Color::from_rgb8(0x32, 0x38, 0x46);
    pub const STANDBY: Color = Color::from_rgb8(0xb0, 0x7a, 0x2a);
    pub const MONITOR_STAND: Color = Color::from_rgb8(0x5a, 0x60, 0x6c);
    pub const METAL: Color = Color::from_rgb8(0x7c, 0x82, 0x8e);

    // HUD da cena em farm_view.dart (_hudBg, _hudInk, _hudText, _hudMuted) e _SceneryToggle em farm_hud.dart.
    pub const HUD_BG: Color = Color::from_rgba8(0x18, 0x24, 0x14, 230.0 / 255.0);
    pub const HUD_INK: Color = Color::from_rgb8(0x0b, 0x12, 0x08);
    pub const HUD_TEXT: Color = Color::from_rgb8(0xff, 0xf8, 0xe8);
    pub const HUD_MUTED: Color = Color::from_rgb8(0xc9, 0xd6, 0xa8);
    pub const HUD_MARGIN: f32 = 10.0;
    pub const HUD_BORDER: f32 = 2.0;
    pub const HUD_BOX_PADDING: [f32; 2] = [5.0, 8.0];
    pub const TOGGLE_PADDING: [f32; 2] = [3.0, 8.0];
    pub const TOGGLE_GAP: f32 = 2.0;
    pub const TOGGLE_TEXT_SIZE: f32 = 11.0;
    pub const TOGGLE_LINE_HEIGHT: f32 = 1.2;
    // _Hud em farm_hud.dart: contador de tokens (chama 20x20, texto 16), chips (11) e status.
    pub const HUD_TEXT_SIZE: f32 = 11.0;
    pub const HUD_LINE_HEIGHT: f32 = 1.2;
    pub const HUD_TOKENS_SIZE: f32 = 16.0;
    pub const HUD_FLAME_SIZE: f32 = 20.0;
    pub const HUD_FLAME_GAP: f32 = 8.0;
    pub const HUD_CHIP_GAP: f32 = 4.0;
    pub const HUD_STATUS_BOX: f32 = 8.0;
    pub const HUD_STATUS_GAP: f32 = 6.0;
    pub const HUD_STATUS_BORDER: f32 = 1.0;
    // _FarmEmpty em farm_hud.dart.
    pub const EMPTY_MAX_WIDTH: f32 = 380.0;
    pub const EMPTY_PADDING: f32 = 8.0;
    pub const EMPTY_GAP: f32 = 8.0;
    pub const EMPTY_TITLE_SIZE: f32 = 14.0;
    pub const EMPTY_HINT_SIZE: f32 = 11.0;
    pub const EMPTY_LINE_HEIGHT: f32 = 1.4;

    // ARGB do campo e das árvores de farm_world.dart e farm_world_paint.dart (usado por farm/world_paint.rs).
    pub mod field {
        pub const G0: u32 = 0xff4a6524;
        pub const G1: u32 = 0xff577530;
        pub const G2: u32 = 0xff648436;
        pub const G3: u32 = 0xff71923c;
        pub const G4: u32 = 0xff80a246;
        pub const WORN: u32 = 0xff8ea456;
        pub const GRASS_TUFT: u32 = 0xff93b552;
        pub const T0: u32 = 0xff1c3519;
        pub const T1: u32 = 0xff28501f;
        pub const T2: u32 = 0xff346a27;
        pub const T3: u32 = 0xff468a31;
        pub const T4: u32 = 0xff63a93f;
        pub const T5: u32 = 0xff8cc65a;
        pub const TRUNK: u32 = 0xff4b3421;
        pub const BARK_LIGHT: u32 = 0xff5e4330;
        pub const BARK_DARK: u32 = 0xff35241a;
        pub const R0: u32 = 0xff34343c;
        pub const R1: u32 = 0xff55555e;
        pub const R2: u32 = 0xff6f6f78;
        pub const R3: u32 = 0xff8d8d96;
        pub const R4: u32 = 0xffaeaeb5;
        pub const ROCK_MOSS: u32 = 0xff5f8a34;
        pub const ROCK_MOSS_LIGHT: u32 = 0xff86b04a;
        pub const ROCK_SPARK: u32 = 0xff45454e;
        // rgba(20, 32, 10, .32) e rgba(20, 32, 10, .38): alpha arredondado para 0..255.
        pub const SHADOW32: u32 = 0x5214_200a;
        pub const SHADOW38: u32 = 0x6114_200a;
        pub const FLOWER_C: [u32; 6] = [
            0xfff4f1e8, 0xfff5c542, 0xfff59cc0, 0xffb9d9ff, 0xffff8f6b, 0xffc99bf0,
        ];
        pub const FLOWER_CENTER: u32 = 0xfff5e27a;
        // solo, água e sombras de farm_world.dart (_soil*, _w*, _shadow*).
        pub const SOIL: u32 = 0xff6b4a2e;
        pub const SOIL2: u32 = 0xff57391f;
        pub const SOIL3: u32 = 0xff7d5a3a;
        pub const SOIL_DEEP: u32 = 0xff3f2915;
        pub const W0: u32 = 0xff2a5a8a;
        pub const W1: u32 = 0xff3f7fb8;
        pub const W2: u32 = 0xff5b9ed0;
        pub const W3: u32 = 0xff9fd0ee;
        pub const SHADOW30: u32 = 0x4d14_200a;
        pub const SHADOW35: u32 = 0x5914_200a;
        pub const SHADOW36: u32 = 0x5c14_200a;
        // rgba(90, 70, 30, .3): o cercado do curral (_yard).
        pub const YARD_SHADE: u32 = 0x4d5a_461e;
    }

    // Paleta do escritório: office_world_paint.dart (_fl*, _cp*, _wl*, _wd*, _walk*, _asph*).
    pub mod office_world {
        pub const FL0: u32 = 0xffebe4d6;
        pub const FL1: u32 = 0xffe5ddcd;
        pub const FL_LINE: u32 = 0xffd8cfbd;
        pub const CP0: u32 = 0xffd5dae0;
        pub const CP1: u32 = 0xffcbd1d9;
        pub const CP2: u32 = 0xffdfe3e8;
        pub const RUG0: u32 = 0xffbfc9d4;
        pub const RUG_EDGE: u32 = 0xffa9b5c3;
        pub const WL0: u32 = 0xff3a404c;
        pub const WL1: u32 = 0xff5d6575;
        pub const FACE: u32 = 0xffd3cbba;
        pub const FACE2: u32 = 0xffc6bda9;
        pub const BASE: u32 = 0xffa99f89;
        pub const WD0: u32 = 0xffc99c66;
        pub const WD1: u32 = 0xffb88a55;
        pub const WD2: u32 = 0xffa47646;
        pub const WD_EDGE: u32 = 0xff6e4a2a;
        pub const WALK0: u32 = 0xffc9cbcf;
        pub const WALK1: u32 = 0xffbcbfc4;
        pub const ASPH0: u32 = 0xff4a4d55;
        pub const ASPH1: u32 = 0xff53565e;
        pub const ASPH2: u32 = 0xff42454c;
        // rgba(255, 220, 140, .18): o halo da luminária.
        pub const LAMP_GLOW: u32 = 0x2eff_dc8c;
        pub const BOOKS: [u32; 7] = [
            0xffd94b3c, 0xff4f86d9, 0xfff2b43a, 0xff6fbf5a, 0xff8a63c9, 0xffe57fb0, 0xff3fa7a0,
        ];
        pub const POUF: [u32; 6] = [
            0xffe0533f, 0xfff2b43a, 0xff6fbf5a, 0xff4f86d9, 0xffe57fb0, 0xff3fa7a0,
        ];
    }

    // Clawd (farm_sprites.dart): paleta do corpo de argila e os traços do rosto.
    pub mod clawd {
        pub const CLAY_O: u32 = 0xff4a1f12;
        pub const CLAY_L: u32 = 0xfff7c3a3;
        pub const CLAY_H: u32 = 0xffec9670;
        pub const CLAY: u32 = 0xffd97757;
        pub const CLAY_S: u32 = 0xffb95d3f;
        pub const CLAY_D: u32 = 0xff8a3a21;
        pub const CLAY_K: u32 = 0xfff3ab98;
        // base da paleta de corpos que não são argila (BodyPal.of)
        pub const INK_BASE: u32 = 0xff1a1216;
        pub const BLUSH_BASE: u32 = 0xffff9f9f;
        pub const EYE: u32 = 0xff23_1815;
        pub const SHINE: u32 = 0xffff_ffff;
        pub const EYE_SHADOW: u32 = 0xff4a_3a36;
        pub const MOUTH_HAPPY: u32 = 0xffc8_505a;
    }
}

/// Cores fixas dos agentes na cena.
pub const FARM_AGENTS: [Color; 8] = [
    Color::from_rgb8(0x2b, 0xb3, 0xa3),
    Color::from_rgb8(0x3b, 0x7d, 0xd8),
    Color::from_rgb8(0xf0, 0x8c, 0xb0),
    Color::from_rgb8(0xf0, 0x8a, 0x2c),
    Color::from_rgb8(0x8a, 0x5c, 0xc7),
    Color::from_rgb8(0x5f, 0xbf, 0x4a),
    Color::from_rgb8(0xf5, 0xc5, 0x42),
    Color::from_rgb8(0xd6, 0x3a, 0x3a),
];

/// Botão discreto: fundo transparente, hover em `PANEL3` (estilo dos ícones da topbar e das abas).
pub fn ghost_button(
    _: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    use iced::widget::button::{Status, Style};
    let palette = palette();
    let background = match status {
        Status::Hovered | Status::Pressed => Some(iced::Background::Color(palette.panel3)),
        _ => None,
    };
    Style {
        background,
        text_color: palette.text2,
        border: iced::Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub const SETTINGS_NAV_SEARCH_RADIUS: f32 = 8.0;
pub const SETTINGS_NAV_SEARCH_HORIZONTAL: u16 = 10;
pub const SETTINGS_NAV_SEARCH_ICON_GAP: f32 = 8.0;
pub const SETTINGS_NAV_SEARCH_VERTICAL: u16 = 7;
pub const SETTINGS_NAV_SEARCH_BOTTOM: f32 = 6.0;
pub const SETTINGS_NAV_SEGMENT_PADDING: u16 = 2;
pub const SETTINGS_NAV_SEGMENT_HORIZONTAL: u16 = 11;
pub const SETTINGS_NAV_SEGMENT_VERTICAL: u16 = 5;
pub const SETTINGS_NAV_SEGMENT_RADIUS: f32 = 6.0;
pub const SETTINGS_NAV_SEGMENT_TEXT: f32 = 13.5;
pub const SETTINGS_NAV_SEGMENT_BOTTOM: f32 = 8.0;
pub const SETTINGS_NAV_GROUP_TOP: f32 = 14.0;
pub const SETTINGS_NAV_GROUP_BOTTOM: f32 = 8.0;
pub const SETTINGS_NAV_GROUP_SIZE: f32 = 12.5;
pub const SETTINGS_NAV_FOOTER_HORIZONTAL: u16 = 20;
pub const SETTINGS_NAV_FOOTER_VERTICAL: u16 = 10;

// Medidas de SettingsSegmented e SettingsDropdown do Flutter.
pub const SETTINGS_SEGMENT_PADDING: u16 = 2;
pub const SETTINGS_SEGMENT_HORIZONTAL: u16 = 11;
pub const SETTINGS_SEGMENT_VERTICAL: u16 = 5;
pub const SETTINGS_SEGMENT_RADIUS: f32 = 6.0;
pub const SETTINGS_SEGMENT_OUTER_RADIUS: f32 = 8.0;
pub const SETTINGS_SEGMENT_TEXT: f32 = 13.5;
pub const SETTINGS_DROPDOWN_RADIUS: f32 = 7.0;
pub const SETTINGS_SWITCH_WIDTH: f32 = 36.0;
pub const SETTINGS_SWITCH_HEIGHT: f32 = 20.0;
pub const SETTINGS_SWITCH_KNOB: f32 = 14.0;
pub const SETTINGS_SWITCH_PADDING: u16 = 2;

// Medidas da aba Claude das Configurações (`settings_widgets.dart`, `claude_code_panel.dart`,
// `code_theme_picker.dart`, `account_panel.dart`).
pub const SETTINGS_SECTION_DESC_GAP: f32 = 4.0;
pub const SETTINGS_DESC_SIZE: f32 = 13.5;
pub const SETTINGS_DESC_LINE_HEIGHT: f32 = 1.45;
pub const SETTINGS_ROW_TITLE_GAP: f32 = 4.0;
pub const SETTINGS_ROW_LEADING_GAP: f32 = 12.0;
pub const SETTINGS_ROW_LEADING_ICON: f32 = 18.0;
pub const SETTINGS_NOTE_VERTICAL: u16 = 12;
pub const SETTINGS_DROPDOWN_WIDTH: f32 = 140.0;
pub const SETTINGS_DROPDOWN_PADDING: [u16; 2] = [7, 10];
pub const SETTINGS_DROPDOWN_TEXT: f32 = 13.5;
pub const SETTINGS_DROPDOWN_ICON: f32 = 16.0;
pub const SETTINGS_TEXT_FIELD_WIDTH: f32 = 224.0;
pub const SETTINGS_TEXT_FIELD_RADIUS: f32 = 7.0;
pub const SETTINGS_TEXT_FIELD_PADDING: [u16; 2] = [8, 12];
pub const SETTINGS_TEXT_FIELD_FILL_ALPHA: f32 = 0.3;
pub const SETTINGS_TEXTAREA_RADIUS: f32 = 8.0;
pub const SETTINGS_TEXTAREA_MIN_LINES: usize = 4;
pub const SETTINGS_TEXTAREA_MAX_LINES: usize = 12;
pub const SETTINGS_TEXTAREA_GAP: f32 = 12.0;
pub const SETTINGS_TEXTAREA_ERROR_GAP: f32 = 6.0;
pub const SETTINGS_CHIP_PADDING: [f32; 2] = [1.0, 6.0];
pub const SETTINGS_CHIP_RADIUS: f32 = 5.0;
pub const SETTINGS_CHIP_TEXT: f32 = 11.5;
pub const SETTINGS_VALUE_TEXT: f32 = 13.5;
pub const SETTINGS_VALUE_MONO_TEXT: f32 = 12.5;
pub const SETTINGS_VALUE_MONO_PADDING: [f32; 2] = [3.0, 8.0];
pub const SETTINGS_VALUE_MONO_RADIUS: f32 = 5.0;
// `OutlineButton`/`PrimaryButton` com `ButtonSize.small` (0,75 do normal).
pub const SETTINGS_BUTTON_PADDING: [u16; 2] = [6, 12];
pub const SETTINGS_BUTTON_TEXT: f32 = 10.5;
pub const SETTINGS_BUTTON_ICON: f32 = 14.0;
pub const SETTINGS_BUTTON_GAP: f32 = 8.0;
pub const SETTINGS_BUTTON_RADIUS: f32 = 6.0;
pub const SETTINGS_PICKER_GAP: f32 = 16.0;
pub const SETTINGS_PICKER_BOTTOM: u16 = 14;
pub const SETTINGS_PICKER_PREVIEW_GAP: f32 = 12.0;
pub const SETTINGS_PICKER_PADDING: [u16; 2] = [5, 10];
pub const SETTINGS_PICKER_TEXT: f32 = 14.0;
pub const SETTINGS_PICKER_ICON: f32 = 18.0;
pub const CODE_PREVIEW_LINE_HEIGHT: f32 = 19.0;
pub const CODE_PREVIEW_GUTTER: f32 = 40.0;
pub const CODE_PREVIEW_SIGN: f32 = 22.0;
pub const CODE_PREVIEW_TEXT: f32 = 13.0;
pub const CODE_PREVIEW_PADDING: u16 = 9;
pub const CODE_PREVIEW_RADIUS: f32 = 7.0;
pub const CODE_PREVIEW_LINE_ALPHA: f32 = 0.16;
pub const CODE_PREVIEW_CHANGED_ALPHA: f32 = 0.28;
pub const SETTINGS_TOAST_PADDING: u16 = 12;
pub const SETTINGS_TOAST_MARGIN: u16 = 16;
pub const SETTINGS_TOAST_MAX_WIDTH: f32 = 360.0;
pub const SETTINGS_TOAST_RADIUS: f32 = 8.0;
pub const SETTINGS_NAV_GROUP_NEXT_TOP: f32 = 22.0;
pub const SETTINGS_FONT_FIELD_WIDTH: f32 = 218.0;
pub const SETTINGS_FONT_DIALOG_SIZE: f32 = 460.0;
pub const SETTINGS_CATALOG_TITLE_SIZE: f32 = 22.0;
pub const SETTINGS_CATALOG_SEARCH_WIDTH: f32 = 255.0;
pub const SETTINGS_USAGE_BAR_HEIGHT: f32 = 8.0;
pub const SETTINGS_USAGE_BAR_RADIUS: f32 = 4.0;
pub const SETTINGS_USAGE_PCT_WIDTH: f32 = 100.0;
pub const SETTINGS_MEMORY_TITLE_WIDTH: f32 = 220.0;
pub const SETTINGS_MEMORY_DATE_WIDTH: f32 = 160.0;
pub const SETTINGS_DS_SEARCH_WIDTH: f32 = 450.0;
pub const SETTINGS_DEV_MCP_LIST_WIDTH: f32 = 190.0;
pub const SETTINGS_CONN_FIELD_WIDTH: f32 = 320.0;
pub const SETTINGS_PLUGIN_CARD_HEIGHT: f32 = 112.0;
pub const SETTINGS_PLUGIN_CARD_WIDTH: f32 = 418.0;
pub const SETTINGS_PLACEHOLDER_ICON: f32 = 64.0;

// Medidas da aba Harness (`harness_view.dart`)
pub const HARNESS_PADDING_SIDE: f32 = 24.0;
pub const HARNESS_PADDING_TOP: f32 = 20.0;
pub const HARNESS_PADDING_BOTTOM: f32 = 32.0;
pub const HARNESS_HEADER_GAP: f32 = 18.0;
pub const HARNESS_SUBTITLE_GAP: f32 = 4.0;
pub const HARNESS_REFRESH_PADDING: f32 = 6.0;
pub const HARNESS_REFRESH_ICON: f32 = 18.0;
pub const HARNESS_REFRESH_SPINNER: f32 = 16.0;
pub const HARNESS_LOADING_SPINNER: f32 = 20.0;
pub const HARNESS_GRID_GAP: f32 = 16.0;
pub const HARNESS_TWO_COLUMNS_MIN_WIDTH: f32 = 980.0;
pub const HARNESS_CARD_RADIUS: f32 = 8.0;
pub const HARNESS_COST_PADDING: f32 = 14.0;
pub const HARNESS_COST_VALUE_SIZE: f32 = 20.0;
pub const HARNESS_COST_TITLE_GAP: f32 = 10.0;
pub const HARNESS_COST_ISSUE_ICON: f32 = 16.0;
pub const HARNESS_COST_ISSUE_GAP: f32 = 4.0;
pub const HARNESS_COST_PARTS_GAP: f32 = 4.0;
pub const HARNESS_COST_HINT_GAP: f32 = 6.0;
pub const HARNESS_COST_TIP_GAP: f32 = 2.0;
pub const HARNESS_SECTION_PADDING: Padding = Padding {
    top: 12.0,
    right: 14.0,
    bottom: 10.0,
    left: 14.0,
};
pub const HARNESS_SECTION_ICON: f32 = 16.0;
pub const HARNESS_SECTION_ICON_GAP: f32 = 8.0;
pub const HARNESS_SECTION_COUNT_GAP: f32 = 6.0;
pub const HARNESS_SECTION_PER_TURN_GAP: f32 = 8.0;
pub const HARNESS_SECTION_PER_TURN_SIZE: f32 = 11.5;
pub const HARNESS_SECTION_DESC_GAP: f32 = 2.0;
pub const HARNESS_SECTION_LIST_GAP: f32 = 8.0;
pub const HARNESS_SECTION_EMPTY_PADDING: f32 = 8.0;
pub const HARNESS_VISIBLE_ENTRIES: usize = 6;
pub const HARNESS_ROW_PADDING: [f32; 2] = [5.0, 6.0];
pub const HARNESS_ROW_RADIUS: f32 = 6.0;
pub const HARNESS_INVALID_ICON: f32 = 14.0;
pub const HARNESS_INVALID_GAP: f32 = 6.0;
pub const HARNESS_ENTRY_NAME_SIZE: f32 = 13.0;
pub const HARNESS_ENTRY_DETAIL_SIZE: f32 = 11.5;
pub const HARNESS_ENTRY_GAP: f32 = 8.0;
pub const HARNESS_ENTRY_ISSUE_ICON: f32 = 15.0;
pub const HARNESS_ENTRY_ISSUE_GAP: f32 = 6.0;
pub const HARNESS_ENTRY_LOCK_ICON: f32 = 13.0;
pub const HARNESS_ENTRY_LOCK_GAP: f32 = 6.0;
pub const HARNESS_CHIP_SIZE: f32 = 11.0;
pub const HARNESS_CHIP_PADDING: [f32; 2] = [2.0, 6.0];
pub const HARNESS_CHIP_RADIUS: f32 = 5.0;
// `GhostButton(size: ButtonSize.small)` do shadcn: 0,75 do padding (16x8), da fonte (14) e raio 12*0,5.
pub const HARNESS_GHOST_PADDING: [f32; 2] = [6.0, 12.0];
pub const HARNESS_GHOST_TEXT: f32 = 10.5;
pub const HARNESS_GHOST_ICON: f32 = 14.0;
pub const HARNESS_GHOST_GAP: f32 = 8.0;
pub const HARNESS_GHOST_RADIUS: f32 = 6.0;
pub const HARNESS_GHOST_HOVER_ALPHA: f32 = 0.8;
// `AlertDialog` do nome do item novo (`_RealmNameDialog`).
pub const HARNESS_NAME_DIALOG_WIDTH: f32 = 360.0;
pub const HARNESS_NAME_ERROR_GAP: f32 = 8.0;
