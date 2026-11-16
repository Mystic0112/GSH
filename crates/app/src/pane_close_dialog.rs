//! Diálogo "Fechar painel?" (`_confirmClosePane` de `pane_view.dart` e
//! `showConfirmDialog` de `confirm_dialog.dart`, com `danger: true`).
//!
//! Scrim sobre a janela inteira (clicar fora cancela, como a barreira do
//! `showDialog`) e um cartão com título, aviso e os botões Cancelar / Fechar.

use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Theme};

use crate::labels;
use crate::theme;

pub fn view<'a, M: Clone + 'a>(
    tab_count: usize,
    on_cancel: M,
    on_confirm: M,
    on_noop: M,
) -> Element<'a, M> {
    dialog(
        labels::PANE_CLOSE_TITLE,
        labels::pane_close_message(tab_count),
        labels::PANE_CLOSE_CONFIRM,
        Emphasis::Danger,
        on_cancel,
        on_confirm,
        on_noop,
    )
}

/// `showConfirmDialog` sem `danger` (confirmar em `PrimaryButton`): o aviso de
/// erro da Galeria, com "OK".
pub fn notice<'a, M: Clone + 'a>(
    title: &'static str,
    message: String,
    on_close: M,
    on_noop: M,
) -> Element<'a, M> {
    dialog(
        title,
        message,
        labels::DIALOG_OK,
        Emphasis::Primary,
        on_close.clone(),
        on_close,
        on_noop,
    )
}

/// Botão de confirmação: `DestructiveButton` ou `PrimaryButton`.
#[derive(Clone, Copy)]
enum Emphasis {
    Danger,
    Primary,
}

fn dialog<'a, M: Clone + 'a>(
    title: &'static str,
    message: String,
    confirm_label: &'static str,
    emphasis: Emphasis,
    on_cancel: M,
    on_confirm: M,
    on_noop: M,
) -> Element<'a, M> {
    let title = text(title)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text);
    let message = text(message)
        .font(theme::FONT_BODY)
        .size(theme::DIALOG_MESSAGE_SIZE)
        .color(theme::palette().text2);
    let actions = row![
        dialog_button(labels::DIALOG_CANCEL, on_cancel.clone(), None),
        dialog_button(confirm_label, on_confirm, Some(emphasis)),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center);

    // largura do conteúdo (a mensagem quebra em 380), ações à direita, como no AlertDialog
    let header = column![
        title,
        container(message).max_width(theme::DIALOG_CONTENT_WIDTH)
    ]
    .spacing(theme::DIALOG_TITLE_GAP);
    let card = container(
        column![header, actions]
            .spacing(theme::DIALOG_GAP)
            .align_x(Alignment::End),
    )
    .padding(theme::DIALOG_PADDING)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        border: Border {
            color: theme::palette().panel2,
            width: 1.0,
            radius: theme::DIALOG_RADIUS.into(),
        },
        ..Default::default()
    });

    let scrim = mouse_area(
        container(Space::new().width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::palette().scrim)),
                ..Default::default()
            }),
    )
    .on_press(on_cancel);

    iced::widget::stack![
        scrim,
        container(mouse_area(card).on_press(on_noop))
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
    ]
    .into()
}

/// Botão do diálogo: confirmação (`emphasis`) ou `OutlineButton` (contorno).
fn dialog_button<'a, M: Clone + 'a>(
    label: &'static str,
    on_press: M,
    emphasis: Option<Emphasis>,
) -> Element<'a, M> {
    let content = move |color: Color| {
        text(label)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::DIALOG_BUTTON_SIZE)
            .color(color)
    };
    let foreground = match emphasis {
        Some(Emphasis::Danger) => theme::on_color(theme::palette().error),
        Some(Emphasis::Primary) => theme::ON_ACCENT,
        None => theme::palette().text,
    };
    button(content(foreground))
        .padding([8, 16])
        .on_press(on_press)
        .style(move |_: &Theme, status| {
            let palette = theme::palette();
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let fill = |color: Color| {
                let color = if hovered {
                    Color { a: 0.9, ..color }
                } else {
                    color
                };
                (Some(Background::Color(color)), color)
            };
            let (background, border_color) = match emphasis {
                Some(Emphasis::Danger) => fill(palette.error),
                Some(Emphasis::Primary) => fill(palette.accent),
                None => (
                    hovered.then_some(Background::Color(palette.panel3)),
                    palette.border,
                ),
            };
            button::Style {
                background,
                text_color: foreground,
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: theme::RADIUS_BUTTON.into(),
                },
                ..Default::default()
            }
        })
        .into()
}
