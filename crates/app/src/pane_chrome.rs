//! Moldura do pane central (`pane_view.dart` e `pane_tab_leading.dart` do Flutter).
//!
//! Barra de abas de 40px em `BG` com divisória inferior `BORDER` (aba ativa em
//! `PANEL` com indicador inferior `ACCENT` de 2px e hover `PANEL3`, abas
//! inativas em `BG` com texto `TEXT3`, botão `×` por aba, botão `+` que abre
//! novo terminal e controles de split/fechar) e corpo em `PANEL` com borda `BORDER`.

use iced::widget::{column, container, text};
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
    pub on_new_chat: M,
    pub on_farm: M,
    pub noop_msg: M,
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
    let tab_strip = view_tab_strip(model, callbacks, dirty);

    let body = container(active_content)
        .width(Fill)
        .height(Fill)
        .padding(1)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::PANEL)),
            border: Border {
                color: theme::BORDER,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

    column![tab_strip, body].width(Fill).height(Fill).into()
}

/// Estado visual vazio quando todas as abas do pane foram fechadas.
pub fn view_empty_body<'a, M: 'a>() -> Element<'a, M> {
    container(
        text(labels::PANE_EMPTY_STATE)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::TEXT3),
    )
    .padding([20, 24])
    .width(Fill)
    .height(Fill)
    .into()
}
