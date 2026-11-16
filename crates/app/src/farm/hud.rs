// Ported from clodfarm (github.com/matank001/clodfarm), MIT License, see third_party/clodfarm/LICENSE.
//! HUD da cena (farm_hud.dart): caixas de pixel dentro do canvas, com fundo escuro,
//! borda de tinta de 2px, cantos retos e fonte mono. `hud_box` é o `_HudBox`: o HUD,
//! o seletor Fazenda | Escritório e o aviso de cena vazia usam a mesma caixa.
use super::agents::FarmCounts;
use super::icons::FarmIcon;
use crate::{labels, theme};
use iced::advanced::image::FilterMethod;
use iced::widget::{column, container, image, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Theme};

/// O que o HUD mostra.
pub struct Stats {
    /// Tokens gastos pelos loops à vista.
    pub tokens: u64,
    pub agents: usize,
    pub subagents: usize,
    pub counts: FarmCounts,
}

/// A chama do contador (`iconSprite(FarmIcon.flame)`), pré-rasterizada uma vez.
pub fn flame() -> image::Handle {
    image::Handle::from_rgba(10, 10, FarmIcon::Flame.rgba().to_vec())
}

/// 1234567 → "1,234,567" (`_grouped`).
pub fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// `_HudBox`: fundo escuro, borda de tinta, cantos retos.
pub fn hud_box<'a, M: 'a>(child: impl Into<Element<'a, M>>) -> container::Container<'a, M> {
    container(child)
        .padding(theme::scene::HUD_BOX_PADDING)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::scene::HUD_BG)),
            border: Border {
                color: theme::scene::HUD_INK,
                width: theme::scene::HUD_BORDER,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
}

/// Texto do HUD: mono em negrito, em caixa alta.
fn hud_text<'a>(content: String, size: f32, color: Color, line_height: f32) -> text::Text<'a> {
    text(content.to_uppercase())
        .font(theme::FONT_MONO_BOLD)
        .size(size)
        .line_height(text::LineHeight::Relative(line_height))
        .color(color)
}

fn small<'a>(content: String) -> text::Text<'a> {
    hud_text(
        content,
        theme::scene::HUD_TEXT_SIZE,
        theme::scene::HUD_TEXT,
        theme::scene::HUD_LINE_HEIGHT,
    )
}

/// Quadradinho colorido com borda de tinta e o texto do status.
fn status<'a, M: 'a>(color: Color, label: String) -> Element<'a, M> {
    row![
        container(Space::new())
            .width(theme::scene::HUD_STATUS_BOX)
            .height(theme::scene::HUD_STATUS_BOX)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(color)),
                border: Border {
                    color: theme::scene::HUD_INK,
                    width: theme::scene::HUD_STATUS_BORDER,
                    radius: 0.0.into(),
                },
                ..container::Style::default()
            }),
        small(label)
    ]
    .spacing(theme::scene::HUD_STATUS_GAP)
    .align_y(Alignment::Center)
    .into()
}

/// `_Hud`: tokens gastos, agentes e subagentes, e o placar por status.
pub fn hud<'a, M: 'a>(flame: &image::Handle, stats: &Stats) -> Element<'a, M> {
    let palette = theme::palette();
    let chip = |child: Element<'a, M>| {
        container(hud_box(child)).padding(iced::Padding {
            top: theme::scene::HUD_CHIP_GAP,
            ..iced::Padding::ZERO
        })
    };
    let spent = hud_box(
        row![
            image(flame.clone())
                .width(theme::scene::HUD_FLAME_SIZE)
                .height(theme::scene::HUD_FLAME_SIZE)
                .filter_method(FilterMethod::Nearest),
            hud_text(
                labels::farm_spent_hud(&grouped(stats.tokens)),
                theme::scene::HUD_TOKENS_SIZE,
                theme::scene::HUD_TEXT,
                theme::scene::HUD_LINE_HEIGHT,
            )
        ]
        .spacing(theme::scene::HUD_FLAME_GAP)
        .align_y(Alignment::Center),
    );
    let crew = chip(
        small(format!(
            "{} · {}",
            labels::farm_agents(stats.agents),
            labels::farm_subagents(stats.subagents)
        ))
        .into(),
    );
    let board = chip(
        column![
            status(palette.online, labels::farm_working(stats.counts.working)),
            status(palette.warn, labels::farm_need_you(stats.counts.waiting)),
            status(palette.git_untracked, labels::farm_idle(stats.counts.idle)),
        ]
        .into(),
    );
    column![spent, crew, board].into()
}

/// `_FarmEmpty`: o aviso no meio da cena quando não há agente.
pub fn empty<'a, M: 'a>() -> Element<'a, M> {
    let body = column![
        hud_text(
            labels::FARM_EMPTY_TITLE.to_string(),
            theme::scene::EMPTY_TITLE_SIZE,
            theme::scene::HUD_TEXT,
            theme::scene::EMPTY_LINE_HEIGHT,
        ),
        text(labels::FARM_EMPTY_HINT)
            .font(theme::FONT_MONO)
            .size(theme::scene::EMPTY_HINT_SIZE)
            .line_height(text::LineHeight::Relative(theme::scene::EMPTY_LINE_HEIGHT))
            .color(theme::scene::HUD_MUTED)
            .align_x(text::Alignment::Center),
    ]
    .spacing(theme::scene::EMPTY_GAP)
    .align_x(Alignment::Center);
    container(
        hud_box(container(body).padding(theme::scene::EMPTY_PADDING))
            .max_width(theme::scene::EMPTY_MAX_WIDTH),
    )
    .center(iced::Fill)
    .into()
}

#[cfg(test)]
mod tests {
    use super::grouped;

    #[test]
    fn grouped_separa_milhares() {
        assert_eq!(grouped(0), "0");
        assert_eq!(grouped(999), "999");
        assert_eq!(grouped(1_000), "1,000");
        assert_eq!(grouped(320_244_144), "320,244,144");
    }
}
