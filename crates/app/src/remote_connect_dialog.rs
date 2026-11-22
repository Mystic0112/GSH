//! Formulário para abrir uma sessão de terminal em um host SSH.

use iced::widget::{column, text, text_input};
use iced::{Element, Fill};

use crate::{pane_close_dialog, theme};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Draft {
    pub target: String,
    pub socket: String,
    pub cwd: String,
    pub error: Option<String>,
    pub connecting: bool,
}

impl Draft {
    pub fn valid(&self) -> bool {
        !self.target.trim().is_empty()
            && !self.target.trim().starts_with('-')
            && self.socket.trim().starts_with('/')
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    TargetChanged(String),
    SocketChanged(String),
    CwdChanged(String),
    Connect,
    Cancel,
}

pub fn view<'a>(draft: &'a Draft) -> Element<'a, Message> {
    let body = column![
        text("Host SSH (alias do ~/.ssh/config ou usuário@host)")
            .size(13)
            .color(theme::palette().text2),
        text_input("usuario@host", &draft.target).on_input(Message::TargetChanged),
        text("Socket absoluto do gsh-server no host")
            .size(13)
            .color(theme::palette().text2),
        text_input("/home/usuario/.gsh/run/gsh-server.sock", &draft.socket)
            .on_input(Message::SocketChanged),
        text("Diretório inicial remoto (opcional)")
            .size(13)
            .color(theme::palette().text2),
        text_input("/home/usuario/projeto", &draft.cwd).on_input(Message::CwdChanged),
        draft
            .error
            .as_deref()
            .map(|error| text(error.to_owned()).size(13).color(theme::palette().error))
            .unwrap_or_else(|| {
                text("Inicie gsh-server no host e copie o caminho que ele imprimir. OpenSSH exige host key em ~/.ssh/known_hosts.")
                    .size(12)
                    .color(theme::palette().text2)
            }),
    ]
    .spacing(8)
    .width(Fill)
    .into();
    pane_close_dialog::form(
        text("Conectar terminal remoto")
            .font(theme::FONT_TITLE)
            .size(theme::DIALOG_TITLE_SIZE)
            .color(theme::palette().text)
            .into(),
        body,
        iced::widget::row![
            pane_close_dialog::action(
                "Cancelar",
                pane_close_dialog::ButtonKind::Outline,
                Some(Message::Cancel),
            ),
            pane_close_dialog::action(
                if draft.connecting {
                    "Conectando…"
                } else {
                    "Conectar"
                },
                pane_close_dialog::ButtonKind::Primary,
                (draft.valid() && !draft.connecting).then_some(Message::Connect),
            ),
        ]
        .spacing(8)
        .into(),
        Some(420.0),
        Message::Cancel,
        Message::Cancel,
    )
}

#[cfg(test)]
mod tests {
    use super::Draft;

    #[test]
    fn requires_safe_ssh_target_and_absolute_socket() {
        let mut draft = Draft::default();
        assert!(!draft.valid());
        draft.target = "-oProxyCommand=bad".into();
        assert!(!draft.valid());
        draft.target = "gsh-home".into();
        assert!(!draft.valid());
        draft.socket = "/home/helio/.gsh/run/gsh-server.sock".into();
        assert!(draft.valid());
    }
}
