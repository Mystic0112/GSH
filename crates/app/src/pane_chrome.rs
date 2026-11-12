//! Moldura do pane central (`pane_view.dart` e `pane_tab_leading.dart` do Flutter).
//!
//! Barra de abas de 40px em `BG` com divisória inferior `BORDER` (aba ativa em
//! `PANEL` com indicador inferior `ACCENT` de 2px, abas
//! inativas em `BG` com texto `TEXT3`, botão `×` por aba, botão `+` que abre
//! novo terminal e controles de split/fechar) e corpo em `PANEL` com borda `BORDER`.

use iced::widget::{button, column, container, row, text};
use iced::{Background, Border, Element, Fill, Theme};

use crate::labels;
use crate::pane_model::PaneModel;
use crate::pane_strip::view_tab_strip;
use crate::theme;

/// Callbacks de interação da barra de abas do pane central.
pub struct ChromeCallbacks<M, FSelect, FClose> {
    pub on_select_tab: FSelect,
    pub on_close_tab: FClose,
    pub on_new_terminal: M,
    pub noop_msg: M,
    /// Divide o pane à direita; `None` quando o pane não pode dividir (ex.: é o lado não focado).
    pub on_split: Option<M>,
    /// Fecha a divisão, juntando as abas do outro pane; `None` quando não há divisão.
    pub on_close_split: Option<M>,
}

/// Envolve o conteúdo ativo do pane (terminal ou viewer) com a barra de abas.
pub fn view<'a, M, FSelect, FClose>(
    model: &'a PaneModel,
    active_content: Element<'a, M>,
    callbacks: ChromeCallbacks<M, FSelect, FClose>,
    dirty: &dyn Fn(&std::path::Path) -> bool,
) -> Element<'a, M>
where
    M: Clone + 'a,
    FSelect: Fn(usize) -> M + 'a,
    FClose: Fn(usize) -> M + 'a,
{
    let split_controls =
        split_controls(callbacks.on_split.clone(), callbacks.on_close_split.clone());
    let tab_strip = row![
        container(view_tab_strip(model, callbacks, dirty)).width(Fill),
        split_controls
    ]
    .align_y(iced::Alignment::Center)
    .width(Fill);

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

/// Botões de divisão no canto direito da barra: dividir, ou fechar a divisão.
fn split_controls<'a, M: Clone + 'a>(
    on_split: Option<M>,
    on_close_split: Option<M>,
) -> Element<'a, M> {
    let mut controls = row![].spacing(theme::PANE_TOOL_GAP).padding(iced::Padding {
        right: theme::PANE_TOOL_SIZE,
        ..iced::Padding::ZERO
    });
    if let Some(msg) = on_split {
        controls = controls.push(
            button(
                text(labels::ICON_SPLIT_H)
                    .font(theme::FONT_ICON)
                    .size(theme::PANE_TOOL_SIZE),
            )
            .padding(0)
            .on_press(msg)
            .style(theme::ghost_button),
        );
    }
    if let Some(msg) = on_close_split {
        controls = controls.push(
            button(
                text(labels::ICON_CLOSE)
                    .font(theme::FONT_ICON)
                    .size(theme::PANE_TOOL_SIZE),
            )
            .padding(0)
            .on_press(msg)
            .style(theme::ghost_button),
        );
    }
    controls.into()
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
