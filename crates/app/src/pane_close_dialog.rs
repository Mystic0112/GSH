//! Diálogo "Fechar painel?" (`_confirmClosePane` de `pane_view.dart` e
//! `showConfirmDialog` de `confirm_dialog.dart`, com `danger: true`).
//!
//! Scrim sobre a janela inteira (clicar fora cancela, como a barreira do
//! `showDialog`) e um cartão com título, aviso e os botões Cancelar / Fechar.

use iced::widget::{button, column, container, mouse_area, row, text, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Theme};

use crate::labels;
use crate::theme;

pub fn view<'a, M: Clone + 'a>(
    tab_count: usize,
    on_cancel: M,
    on_confirm: M,
    on_noop: M,
) -> Element<'a, M> {
    dialog(
        labels::PANE_CLOSE_TITLE.to_owned(),
        message_body(labels::pane_close_message(tab_count)),
        labels::PANE_CLOSE_CONFIRM,
        Emphasis::Danger,
        on_cancel,
        Some(on_confirm),
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
        title.to_owned(),
        message_body(message),
        labels::DIALOG_OK,
        Emphasis::Primary,
        on_close.clone(),
        Some(on_close),
        on_noop,
    )
}

/// `showConfirmDialog(danger: true)`: título, mensagem e Cancelar / `confirm_label` (destrutivo).
pub fn confirm<'a, M: Clone + 'a>(
    title: String,
    message: String,
    confirm_label: &'static str,
    on_cancel: M,
    on_confirm: M,
    on_noop: M,
) -> Element<'a, M> {
    dialog(
        title,
        message_body(message),
        confirm_label,
        Emphasis::Danger,
        on_cancel,
        Some(on_confirm),
        on_noop,
    )
}

/// Diálogo com um corpo livre (`AlertDialog` com `content`): título, corpo e os botões
/// Cancelar / `confirm_label`. `on_confirm` vazio deixa a confirmação desabilitada.
pub fn custom<'a, M: Clone + 'a>(
    title: String,
    body: Element<'a, M>,
    confirm_label: &'static str,
    on_cancel: M,
    on_confirm: Option<M>,
    on_noop: M,
) -> Element<'a, M> {
    dialog(
        title,
        body,
        confirm_label,
        Emphasis::Primary,
        on_cancel,
        on_confirm,
        on_noop,
    )
}

/// Mensagem do aviso, quebrando na largura do conteúdo.
fn message_body<'a, M: 'a>(message: String) -> Element<'a, M> {
    container(
        text(message)
            .font(theme::FONT_BODY)
            .size(theme::DIALOG_MESSAGE_SIZE)
            .color(theme::palette().text2),
    )
    .max_width(theme::DIALOG_CONTENT_WIDTH)
    .into()
}

/// Botão de confirmação: `DestructiveButton` ou `PrimaryButton`.
#[derive(Clone, Copy)]
enum Emphasis {
    Danger,
    Primary,
}

fn dialog<'a, M: Clone + 'a>(
    title: String,
    body: Element<'a, M>,
    confirm_label: &'static str,
    emphasis: Emphasis,
    on_cancel: M,
    on_confirm: Option<M>,
    on_noop: M,
) -> Element<'a, M> {
    let title = text(title)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text);
    let actions = row![
        dialog_button(labels::DIALOG_CANCEL, Some(on_cancel.clone()), None),
        dialog_button(confirm_label, on_confirm, Some(emphasis)),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center);

    // largura do conteúdo (a mensagem quebra em 380), ações à direita, como no AlertDialog
    let header = column![title, body].spacing(theme::DIALOG_TITLE_GAP);
    overlay(
        column![header, actions]
            .spacing(theme::DIALOG_GAP)
            .align_x(Alignment::End)
            .into(),
        on_cancel,
        on_noop,
    )
}

/// Estilo de um botão de ação do diálogo (`DestructiveButton`, `PrimaryButton`, `OutlineButton`
/// e `GhostButton` do shadcn).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Danger,
    Primary,
    Outline,
    Ghost,
}

/// Botão de ação para os diálogos com corpo e ações próprios ([`form`]).
pub fn action<'a, M: Clone + 'a>(
    label: &'static str,
    kind: ButtonKind,
    on_press: Option<M>,
) -> Element<'a, M> {
    match kind {
        ButtonKind::Danger => dialog_button(label, on_press, Some(Emphasis::Danger)),
        ButtonKind::Primary => dialog_button(label, on_press, Some(Emphasis::Primary)),
        ButtonKind::Outline => dialog_button(label, on_press, None),
        ButtonKind::Ghost => button(
            text(label)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::DIALOG_BUTTON_SIZE)
                .color(theme::palette().text),
        )
        .padding([8, 16])
        .on_press_maybe(on_press)
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| Background::Color(theme::palette().panel3)),
            text_color: theme::palette().text,
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into(),
    }
}

/// `AlertDialog` com título, corpo e linha de ações livres.
///
/// `actions_width` é a largura do conteúdo quando a linha de ações precisa ocupá-la inteira (o
/// `Spacer` do Flutter, com um botão na extrema esquerda); `None` deixa as ações encolhidas e à
/// direita. O iced não deriva a largura do card de um filho `Fill`, por isso ela é explícita.
pub fn form<'a, M: Clone + 'a>(
    title: Element<'a, M>,
    body: Element<'a, M>,
    actions: Element<'a, M>,
    actions_width: Option<f32>,
    on_cancel: M,
    on_noop: M,
) -> Element<'a, M> {
    let header = column![title, body].spacing(theme::DIALOG_TITLE_GAP);
    let actions = match actions_width {
        Some(width) => container(actions).width(Length::Fixed(width)),
        None => container(actions),
    };
    overlay(
        column![header, actions]
            .spacing(theme::DIALOG_GAP)
            .align_x(Alignment::End)
            .into(),
        on_cancel,
        on_noop,
    )
}

/// Scrim sobre a janela e o card centrado com `content`.
fn overlay<'a, M: Clone + 'a>(content: Element<'a, M>, on_cancel: M, on_noop: M) -> Element<'a, M> {
    let card = container(content)
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
    on_press: Option<M>,
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
        // `OutlineButton` desabilitado: texto esmaecido
        None if on_press.is_none() => theme::palette().text4,
        None => theme::palette().text,
    };
    button(content(foreground))
        .padding([8, 16])
        .on_press_maybe(on_press)
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
                // `PrimaryButton` desabilitado: `mutedForeground` (text2) no lugar do accent
                Some(Emphasis::Primary) if matches!(status, button::Status::Disabled) => {
                    (Some(Background::Color(palette.text2)), palette.text2)
                }
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
