//! Paletas de syntax (`SyntaxColors` e `SyntaxPalette` de `syntax_colors.dart`).

use iced::Color;

/// Cores semânticas do realce de código, cada paleta com o próprio fundo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Syntax {
    pub background: Color,
    pub base: Color,
    pub keyword: Color,
    pub string: Color,
    pub klass: Color,
    pub function: Color,
    pub variable: Color,
    pub meta: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::from_rgb8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

/// Uma paleta escolhível: nome gravado no `config.toml`, rótulo (nome próprio, não se traduz)
/// e as cores.
pub struct Palette {
    pub name: &'static str,
    pub label: &'static str,
    pub colors: Syntax,
}

pub const GITHUB_LIGHT: Syntax = Syntax {
    background: rgb(0xFFFFFF),
    base: rgb(0x24292F),
    keyword: rgb(0xCF222E),
    string: rgb(0x0A3069),
    klass: rgb(0x953800),
    function: rgb(0x8250DF),
    variable: rgb(0x953800),
    meta: rgb(0x116329),
};

pub const GITHUB_DARK: Syntax = Syntax {
    background: rgb(0x0D1117),
    base: rgb(0xC9D1D9),
    keyword: rgb(0xFF7B72),
    string: rgb(0xA5D6FF),
    klass: rgb(0xFFA657),
    function: rgb(0xD2A8FF),
    variable: rgb(0xFFA657),
    meta: rgb(0x7EE787),
};

/// Na ordem do seletor.
pub const PALETTES: [Palette; 8] = [
    Palette {
        name: "githubLight",
        label: "GitHub Light",
        colors: GITHUB_LIGHT,
    },
    Palette {
        name: "githubDark",
        label: "GitHub Dark",
        colors: GITHUB_DARK,
    },
    Palette {
        name: "oneLight",
        label: "One Light",
        colors: Syntax {
            background: rgb(0xFAFAFA),
            base: rgb(0x383A42),
            keyword: rgb(0xA626A4),
            string: rgb(0x50A14F),
            klass: rgb(0xC18401),
            function: rgb(0x4078F2),
            variable: rgb(0xE45649),
            meta: rgb(0x0184BC),
        },
    },
    Palette {
        name: "oneDark",
        label: "One Dark",
        colors: Syntax {
            background: rgb(0x282C34),
            base: rgb(0xABB2BF),
            keyword: rgb(0xC678DD),
            string: rgb(0x98C379),
            klass: rgb(0xE5C07B),
            function: rgb(0x61AFEF),
            variable: rgb(0xE06C75),
            meta: rgb(0x56B6C2),
        },
    },
    Palette {
        name: "draculaLight",
        label: "Dracula Light",
        colors: Syntax {
            background: rgb(0xF6F2FF),
            base: rgb(0x2A2A37),
            keyword: rgb(0xC2268E),
            string: rgb(0x6B7A1F),
            klass: rgb(0x0E7490),
            function: rgb(0x1F8A4C),
            variable: rgb(0xB45309),
            meta: rgb(0xC2268E),
        },
    },
    Palette {
        name: "draculaDark",
        label: "Dracula",
        colors: Syntax {
            background: rgb(0x282A36),
            base: rgb(0xF8F8F2),
            keyword: rgb(0xFF79C6),
            string: rgb(0xF1FA8C),
            klass: rgb(0x8BE9FD),
            function: rgb(0x50FA7B),
            variable: rgb(0xFFB86C),
            meta: rgb(0xFF79C6),
        },
    },
    Palette {
        name: "flexokiLight",
        label: "Flexoki Light",
        colors: Syntax {
            background: rgb(0xFFFCF0),
            base: rgb(0x100F0F),
            keyword: rgb(0xAF3029),
            string: rgb(0x66800B),
            klass: rgb(0xAD8301),
            function: rgb(0x205EA6),
            variable: rgb(0xA02F6F),
            meta: rgb(0x5E409D),
        },
    },
    Palette {
        name: "flexokiDark",
        label: "Flexoki Dark",
        colors: Syntax {
            background: rgb(0x1C1B1A),
            base: rgb(0xCECDC3),
            keyword: rgb(0xD14D41),
            string: rgb(0x879A39),
            klass: rgb(0xD0A215),
            function: rgb(0x4385BE),
            variable: rgb(0xCE5D97),
            meta: rgb(0x8B7EC8),
        },
    },
];

/// Nome salvo → paleta; desconhecido ou `None` = seguir o tema.
pub fn by_name(name: Option<&str>) -> Option<&'static Palette> {
    let name = name?;
    PALETTES.iter().find(|p| p.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_resolve_and_unknown_follows_the_theme() {
        assert_eq!(by_name(Some("oneDark")).map(|p| p.label), Some("One Dark"));
        assert!(by_name(Some("neon")).is_none());
        assert!(by_name(None).is_none());
        assert_eq!(PALETTES.len(), 8);
    }
}
