//! Moldura do pane central (`pane_view.dart` e `pane_tab_leading.dart` do Flutter).
//!
//! Barra de abas de 40px em `BG` com divisória inferior `BORDER` (aba ativa em
//! `PANEL` com indicador inferior `ACCENT` de 2px, abas
//! inativas em `BG` com texto `TEXT3`, botão `×` por aba, botão `+` que abre
//! novo terminal e controles de split/fechar) e corpo em `PANEL` com borda `BORDER`.

use iced::widget::pane_grid;
use iced::widget::{button, canvas, column, container, row, text, tooltip};
use iced::{Background, Border, Element, Fill, Theme};

use crate::labels;
use crate::pane_model::PaneModel;
use crate::pane_strip::view_tab_strip;
use crate::pane_tool_icon::PaneToolIcon;
use crate::theme;

/// Callbacks de interação da barra de abas do pane central.
pub struct ChromeCallbacks<M, FSelect, FClose> {
    pub on_select_tab: FSelect,
    pub on_close_tab: FClose,
    pub on_new_terminal: M,
    pub noop_msg: M,
    pub on_split_right: M,
    pub on_split_down: M,
    /// Pede para fechar o pane (a confirmação é do chamador).
    pub on_close_pane: M,
    /// `false` com um diálogo aberto: a dica é um overlay e ficaria por cima dele.
    pub show_tooltips: bool,
}

/// Envolve o conteúdo ativo do pane (terminal ou viewer) com a barra de abas.
pub fn view<'a, M, FSelect, FClose, FHover>(
    model: &'a PaneModel,
    active_content: Element<'a, M>,
    callbacks: ChromeCallbacks<M, FSelect, FClose>,
    dirty: &dyn Fn(&std::path::Path) -> bool,
    hovered: Option<usize>,
    on_hover: FHover,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FSelect: Fn(usize) -> M + 'a,
    FClose: Fn(usize) -> M + 'a,
    FHover: Fn(Option<usize>) -> M,
{
    let split_controls = split_controls(
        callbacks.on_split_right.clone(),
        callbacks.on_split_down.clone(),
        callbacks.on_close_pane.clone(),
        callbacks.show_tooltips,
    );
    // as ferramentas ficam dentro da barra (mesmo fundo e borda de baixo), como em pane_view.dart
    let tab_strip = view_tab_strip(model, callbacks, dirty, hovered, on_hover, split_controls);

    let body = container(active_content)
        .width(Fill)
        .height(Fill)
        .padding(1)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel)),
            border: Border {
                color: theme::palette().border,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

    column![tab_strip, body].width(Fill).height(Fill).into()
}

/// Botões do canto direito da barra, na ordem do Flutter: dividir à direita, dividir abaixo, fechar.
fn split_controls<'a, M: Clone + 'a>(
    on_split_right: M,
    on_split_down: M,
    on_close_pane: M,
    show_tooltips: bool,
) -> Element<'a, M> {
    row![
        tool_button(
            PaneToolIcon::SplitRight,
            labels::PANE_SPLIT_RIGHT,
            on_split_right,
            show_tooltips
        ),
        tool_button(
            PaneToolIcon::SplitDown,
            labels::PANE_SPLIT_DOWN,
            on_split_down,
            show_tooltips
        ),
        tool_button(
            PaneToolIcon::Close,
            labels::PANE_CLOSE,
            on_close_pane,
            show_tooltips
        ),
    ]
    .spacing(theme::PANE_TOOL_GAP)
    .padding(iced::Padding {
        right: theme::PANE_TOOL_SIZE,
        ..iced::Padding::ZERO
    })
    .into()
}

/// Botão fantasma 13px com ícone desenhado e tooltip (`btn` de `_PaneTools`).
fn tool_button<'a, M: Clone + 'a>(
    icon: PaneToolIcon,
    tip: &'static str,
    on_press: M,
    show_tooltip: bool,
) -> Element<'a, M> {
    let size = theme::PANE_TOOL_SIZE;
    let button = button(canvas::Canvas::new(icon).width(size).height(size))
        .width(size)
        .height(size)
        .padding(0)
        .on_press(on_press)
        .style(theme::ghost_button);
    if !show_tooltip {
        return button.into();
    }
    tooltip(
        button,
        container(
            text(tip)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text),
        )
        .padding([4, 8])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                color: theme::palette().border2,
                width: 1.0,
                radius: theme::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }),
        tooltip::Position::Bottom,
    )
    .into()
}

/// Divisor entre panes (`PaneDivider`): a linha de 1px é `border` (fundo do grid) e vira `accent`
/// ao passar o mouse ou arrastar.
pub fn grid_style(_: &Theme) -> pane_grid::Style {
    let accent = pane_grid::Line {
        color: theme::palette().accent,
        width: theme::PANE_DIVIDER_LINE,
    };
    pane_grid::Style {
        hovered_region: pane_grid::Highlight {
            background: Background::Color(iced::Color::TRANSPARENT),
            border: Border::default(),
        },
        picked_split: accent,
        hovered_split: accent,
    }
}

/// Estado visual vazio quando todas as abas do pane foram fechadas.
pub fn view_empty_body<'a, M: 'a>() -> Element<'a, M> {
    container(
        text(labels::PANE_EMPTY_STATE)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3),
    )
    .padding([20, 24])
    .width(Fill)
    .height(Fill)
    .into()
}
