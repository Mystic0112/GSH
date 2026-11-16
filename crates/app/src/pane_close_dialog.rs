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
    let title = text(labels::PANE_CLOSE_TITLE)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text);
    let message = text(labels::pane_close_message(tab_count))
        .font(theme::FONT_BODY)
        .size(theme::DIALOG_MESSAGE_SIZE)
        .color(theme::palette().text2);
    let actions = row![
        dialog_button(labels::DIALOG_CANCEL, on_cancel.clone(), false),
        dialog_button(labels::PANE_CLOSE_CONFIRM, on_confirm, true),
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

/// Botão do diálogo: `DestructiveButton` (fundo de erro) ou `OutlineButton` (contorno).
fn dialog_button<'a, M: Clone + 'a>(
    label: &'static str,
    on_press: M,
    danger: bool,
) -> Element<'a, M> {
    let content = move |color: Color| {
        text(label)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::DIALOG_BUTTON_SIZE)
            .color(color)
    };
    let foreground = if danger {
        theme::on_color(theme::palette().error)
    } else {
        theme::palette().text
    };
    button(content(foreground))
        .padding([8, 16])
        .on_press(on_press)
        .style(move |_: &Theme, status| {
            let palette = theme::palette();
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let (background, border_color) = if danger {
                let error = if hovered {
                    Color {
                        a: 0.9,
                        ..palette.error
                    }
                } else {
                    palette.error
                };
                (Some(Background::Color(error)), error)
            } else {
                (
                    hovered.then_some(Background::Color(palette.panel3)),
                    palette.border,
                )
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
