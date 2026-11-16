//! `CodeThemePicker`: paleta de código de um modo (claro ou escuro), dropdown mais uma
//! prévia de um diff curto pintado como o app vai pintar.

use super::palettes::{self, Syntax};
use super::prefs::Pref;
use super::widgets::{dropdown, Choice};
use super::Message;
use crate::{labels, pane_chrome, theme};
use iced::widget::{column, container, rich_text, row, span, text, tooltip, Space};
use iced::{Color, Element, Fill, Font, Length};

#[derive(Clone, Copy, PartialEq)]
enum Tok {
    Plain,
    Keyword,
    Fn,
    Param,
    Type,
    Str,
    Meta,
    Punct,
}

/// (número, sinal, pedaços); o `bool` marca o trecho que mudou.
type Line = (&'static str, char, &'static [(Tok, &'static str, bool)]);

const LINES: [Line; 4] = [
    (
        "1",
        ' ',
        &[
            (Tok::Keyword, "function ", false),
            (Tok::Fn, "greet", false),
            (Tok::Punct, "(", false),
            (Tok::Param, "name", false),
            (Tok::Punct, ": ", false),
            (Tok::Type, "string", false),
            (Tok::Punct, ") {", false),
        ],
    ),
    (
        "2",
        '-',
        &[
            (Tok::Keyword, "  return ", false),
            (Tok::Str, "\"Hello, \"", true),
            (Tok::Punct, " + ", true),
            (Tok::Plain, "name", false),
            (Tok::Punct, ";", false),
        ],
    ),
    (
        "2",
        '+',
        &[
            (Tok::Keyword, "  return ", false),
            (Tok::Str, "`Hello, ", true),
            (Tok::Meta, "${", true),
            (Tok::Plain, "name", false),
            (Tok::Meta, "}", true),
            (Tok::Str, "!`", true),
            (Tok::Punct, ";", false),
        ],
    ),
    ("3", ' ', &[(Tok::Punct, "}", false)]),
];

fn follow_theme() -> Choice {
    Choice {
        value: None,
        label: labels::SETTINGS_CC_FOLLOW_THEME.to_owned(),
    }
}

/// O seletor de um modo: `picked` é o nome gravado (`None` = a paleta do tema ativo).
pub fn picker<'a>(dark: bool, picked: Option<&str>) -> Element<'a, Message> {
    let palette = if dark {
        theme::GshPalette::DARK
    } else {
        theme::GshPalette::LIGHT
    };
    let chosen = palettes::by_name(picked);
    let mut options = vec![follow_theme()];
    options.extend(palettes::PALETTES.iter().map(|p| Choice {
        value: Some(p.name.to_owned()),
        label: p.label.to_owned(),
    }));
    let selected = chosen.map_or_else(follow_theme, |p| Choice {
        value: Some(p.name.to_owned()),
        label: p.label.to_owned(),
    });
    let own = if dark {
        palettes::GITHUB_DARK
    } else {
        palettes::GITHUB_LIGHT
    };
    let syntax = Syntax {
        background: own.background,
        ..chosen.map_or(own, |p| p.colors)
    };
    let menu = dropdown(
        options,
        selected,
        Fill,
        theme::SETTINGS_PICKER_PADDING,
        theme::SETTINGS_PICKER_TEXT,
        theme::SETTINGS_PICKER_ICON,
        move |choice| {
            Message::Pref(Pref::CodeTheme {
                dark,
                name: choice.value,
            })
        },
    );
    let hint = if dark {
        labels::SETTINGS_CC_CODE_THEME_DARK_HINT
    } else {
        labels::SETTINGS_CC_CODE_THEME_LIGHT_HINT
    };
    column![
        tooltip(
            menu,
            pane_chrome::tip_bubble(hint),
            tooltip::Position::Bottom
        ),
        Space::new().height(theme::SETTINGS_PICKER_PREVIEW_GAP),
        preview(syntax, palette),
    ]
    .width(Fill)
    .into()
}

fn color_of(tok: Tok, syntax: &Syntax) -> Color {
    match tok {
        Tok::Plain | Tok::Punct => syntax.base,
        Tok::Keyword => syntax.keyword,
        Tok::Fn => syntax.function,
        Tok::Param => syntax.variable,
        Tok::Type => syntax.klass,
        Tok::Str => syntax.string,
        Tok::Meta => syntax.meta,
    }
}

/// Diff de quatro linhas pintado com `syntax`; `ui` dá as cores de diff (erro/ok), da borda e
/// dos números de linha do mesmo variant.
fn preview<'a>(syntax: Syntax, ui: theme::GshPalette) -> Element<'a, Message> {
    let lines = LINES.iter().fold(column![], |col, line| {
        col.push(preview_line(line, &syntax, &ui))
    });
    container(lines)
        .padding([theme::CODE_PREVIEW_PADDING, 0])
        .width(Fill)
        .clip(true)
        .style(move |_| container::Style {
            background: Some(syntax.background.into()),
            border: iced::Border {
                color: ui.border2,
                width: 1.0,
                radius: theme::CODE_PREVIEW_RADIUS.into(),
            },
            ..Default::default()
        })
        .into()
}

fn preview_line<'a>(
    (number, sign, toks): &Line,
    syntax: &Syntax,
    ui: &theme::GshPalette,
) -> Element<'a, Message> {
    let diff = match sign {
        '-' => Some(ui.error),
        '+' => Some(ui.ok),
        _ => None,
    };
    let muted = diff.map_or(ui.text4, |_| syntax.base);
    let cell = |content: String, width: f32, align: iced::alignment::Horizontal| {
        container(
            text(content)
                .font(theme::FONT_MONO)
                .size(theme::CODE_PREVIEW_TEXT)
                .line_height(iced::widget::text::LineHeight::Relative(1.0))
                .color(muted),
        )
        .width(width)
        .align_x(align)
    };
    let spans: Vec<_> = toks
        .iter()
        .map(|(tok, content, changed)| {
            let italic = matches!(tok, Tok::Fn | Tok::Param);
            let mut piece = span::<(), Font>(*content)
                .color(color_of(*tok, syntax))
                .font(if italic {
                    theme::FONT_MONO_ITALIC
                } else {
                    theme::FONT_MONO
                });
            if let (true, Some(diff)) = (*changed, diff) {
                piece = piece.background(Color {
                    a: theme::CODE_PREVIEW_CHANGED_ALPHA,
                    ..diff
                });
            }
            piece
        })
        .collect();
    let code = rich_text(spans)
        .font(theme::FONT_MONO)
        .size(theme::CODE_PREVIEW_TEXT)
        .line_height(iced::widget::text::LineHeight::Relative(1.0))
        .wrapping(iced::widget::text::Wrapping::None);
    container(row![
        cell(
            (*number).to_owned(),
            theme::CODE_PREVIEW_GUTTER,
            iced::alignment::Horizontal::Right
        ),
        cell(
            sign.to_string(),
            theme::CODE_PREVIEW_SIGN,
            iced::alignment::Horizontal::Center
        ),
        container(code).width(Fill).clip(true),
    ])
    .width(Fill)
    .height(Length::Fixed(theme::CODE_PREVIEW_LINE_HEIGHT))
    .center_y(Length::Fixed(theme::CODE_PREVIEW_LINE_HEIGHT))
    .style(move |_| container::Style {
        background: diff.map(|c| {
            Color {
                a: theme::CODE_PREVIEW_LINE_ALPHA,
                ..c
            }
            .into()
        }),
        ..Default::default()
    })
    .into()
}
