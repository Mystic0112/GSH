//! Modais de confiança TOFU para host key SSH e desbloqueio de passphrase de chave privada
//! (`ssh_prompts.dart`).

use std::fmt;

use iced::widget::{column, container, row, text, text_input, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Theme};
use zeroize::Zeroize;

use crate::labels;
use crate::pane_close_dialog::{self, ButtonKind};
use crate::theme;

/// Estado do modal de host key desconhecida (`showSshHostKeyDialog`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyPrompt {
    pub endpoint: String,
    pub fingerprint: String,
}

impl HostKeyPrompt {
    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        on_cancel: M,
        on_trust: M,
        on_noop: M,
    ) -> Element<'a, M> {
        let palette = theme::palette();
        let title = row![
            text(labels::ICON_GPP_MAYBE)
                .font(theme::FONT_ICON)
                .size(theme::DB_DIALOG_TITLE_ICON)
                .color(palette.warn),
            Space::new().width(8),
            text(labels::SSH_UNKNOWN_HOST_TITLE)
                .font(theme::FONT_TITLE)
                .size(theme::DIALOG_TITLE_SIZE)
                .color(palette.text),
        ]
        .align_y(Alignment::Center)
        .into();

        let fingerprint_box = container(
            text(&self.fingerprint)
                .font(theme::FONT_MONO)
                .size(theme::SSH_PROMPT_MONO_SIZE)
                .color(palette.text),
        )
        .width(Fill)
        .padding(theme::SSH_PROMPT_BOX_PADDING)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(palette.bg)),
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: theme::SSH_PROMPT_BOX_RADIUS.into(),
            },
            ..Default::default()
        });

        let command_box = container(
            text(labels::SSH_COMMAND_PREVIEW)
                .font(theme::FONT_MONO)
                .size(theme::SSH_PROMPT_MONO_SIZE - 0.5)
                .color(palette.text3),
        )
        .width(Fill)
        .padding([6, 10])
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(palette.bg)),
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        });

        let body = container(
            column![
                text(labels::ssh_never_connected(&self.endpoint))
                    .font(theme::FONT_BODY)
                    .size(theme::DIALOG_MESSAGE_SIZE)
                    .color(palette.text2),
                fingerprint_box,
                text(labels::SSH_TRUST_HINT)
                    .font(theme::FONT_BODY)
                    .size(theme::SSH_PROMPT_HINT_SIZE)
                    .color(palette.text3),
                command_box,
            ]
            .spacing(10)
            .width(Fill),
        )
        .width(Length::Fixed(theme::SSH_PROMPT_HOST_KEY_MAX_WIDTH))
        .into();

        let actions = row![
            pane_close_dialog::action(
                labels::DIALOG_CANCEL,
                ButtonKind::Outline,
                Some(on_cancel.clone())
            ),
            pane_close_dialog::action(labels::SSH_TRUST, ButtonKind::Primary, Some(on_trust)),
        ]
        .spacing(theme::DIALOG_ACTIONS_GAP)
        .align_y(Alignment::Center)
        .into();

        pane_close_dialog::form(title, body, actions, None, on_cancel, on_noop)
    }
}

/// Estado do modal de passphrase de chave privada (`showSshPassphraseDialog`).
#[derive(Clone, PartialEq, Eq)]
pub struct PassphrasePrompt {
    pub connection_name: String,
    pub key_path: String,
    pub passphrase: String,
}

impl Drop for PassphrasePrompt {
    fn drop(&mut self) {
        self.passphrase.zeroize();
    }
}

impl fmt::Debug for PassphrasePrompt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PassphrasePrompt")
            .field("connection_name", &self.connection_name)
            .field("key_path", &self.key_path)
            .field("passphrase", &"[REDACTED]")
            .finish()
    }
}

impl PassphrasePrompt {
    pub fn new(connection_name: String, key_path: String) -> Self {
        Self {
            connection_name,
            key_path,
            passphrase: String::new(),
        }
    }

    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        on_input: impl Fn(String) -> M + 'a,
        on_cancel: M,
        on_unlock: M,
        on_noop: M,
    ) -> Element<'a, M> {
        let palette = theme::palette();
        let title = text(labels::SSH_KEY_PASSPHRASE_TITLE)
            .font(theme::FONT_TITLE)
            .size(theme::DIALOG_TITLE_SIZE)
            .color(palette.text)
            .into();

        let mut field = text_input(labels::DB_DIALOG_PASSWORD, &self.passphrase)
            .secure(true)
            .on_input(on_input)
            .font(theme::FONT_BODY)
            .size(theme::DB_DIALOG_INPUT_SIZE)
            .padding(theme::DB_DIALOG_INPUT_PADDING)
            .style(move |_, _| text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: Border {
                    color: palette.border,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                icon: palette.text3,
                placeholder: palette.text4,
                value: palette.text,
                selection: theme::TERM_SELECTION_BG,
            });
        if !self.passphrase.is_empty() {
            field = field.on_submit(on_unlock.clone());
        }

        let body = container(
            column![
                text(labels::ssh_unlock_message(
                    &self.key_path,
                    &self.connection_name
                ))
                .font(theme::FONT_BODY)
                .size(theme::DIALOG_MESSAGE_SIZE)
                .color(palette.text2),
                field,
                text(labels::SSH_KEPT_IN_MEMORY_HINT)
                    .font(theme::FONT_BODY)
                    .size(theme::SSH_PROMPT_HINT_SIZE)
                    .color(palette.text3),
            ]
            .spacing(10)
            .width(Fill),
        )
        .width(Length::Fixed(theme::SSH_PROMPT_PASSPHRASE_MAX_WIDTH))
        .into();

        let unlock_msg = (!self.passphrase.is_empty()).then_some(on_unlock);
        let actions = row![
            pane_close_dialog::action(
                labels::DIALOG_CANCEL,
                ButtonKind::Outline,
                Some(on_cancel.clone())
            ),
            pane_close_dialog::action(labels::SSH_UNLOCK, ButtonKind::Primary, unlock_msg),
        ]
        .spacing(theme::DIALOG_ACTIONS_GAP)
        .align_y(Alignment::Center)
        .into();

        pane_close_dialog::form(title, body, actions, None, on_cancel, on_noop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_redacts_passphrase() {
        let mut p = PassphrasePrompt::new("prod".into(), "~/.ssh/id_ed25519".into());
        p.passphrase = "top-secret-pass".into();
        let dbg = format!("{p:?}");
        assert!(!dbg.contains("top-secret-pass"), "{dbg}");
        assert!(dbg.contains("[REDACTED]"), "{dbg}");
    }

    #[test]
    fn views_build_without_panicking() {
        let hk = HostKeyPrompt {
            endpoint: "u@bastion:22".into(),
            fingerprint: "SHA256:abc".into(),
        };
        let _ = hk.view((), (), ());

        let pp = PassphrasePrompt::new("prod".into(), "~/.ssh/id_ed25519".into());
        let _ = pp.view(|_| (), (), (), ());
    }
}
