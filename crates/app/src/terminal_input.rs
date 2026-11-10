//! Mapeamento puro de eventos de teclado do `iced` para bytes de terminal (VT/ANSI)
//! e ações de área de transferência (`Copy`/`Paste`).

use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};

/// Ação resultante de uma tecla pressionada no pane do terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// Envia a sequência de bytes diretamente ao PTY (`TermSession::send`).
    Send(Vec<u8>),
    /// Copia o texto selecionado para a área de transferência (`Ctrl+Shift+C`).
    Copy,
    /// Cola o conteúdo da área de transferência no PTY (`Ctrl+Shift+V`).
    Paste,
}

/// Classifica uma tecla pressionada em [`KeyAction`], tratando primeiro os
/// atalhos de área de transferência (`Ctrl+Shift+C` / `Ctrl+Shift+V`) e em
/// seguida a conversão para bytes VT/ANSI.
pub fn classify_key_press(
    key: &Key,
    modified_key: &Key,
    modifiers: Modifiers,
) -> Option<KeyAction> {
    if modifiers.logo() {
        return None;
    }

    if matches!(key.as_ref(), Key::Named(Named::Copy)) {
        return Some(KeyAction::Copy);
    }
    if matches!(key.as_ref(), Key::Named(Named::Paste)) {
        return Some(KeyAction::Paste);
    }

    if modifiers.control() && modifiers.shift() {
        if let Some(ch) = extract_single_char(key).or_else(|| extract_single_char(modified_key)) {
            if ch.eq_ignore_ascii_case(&'c') {
                return Some(KeyAction::Copy);
            }
            if ch.eq_ignore_ascii_case(&'v') {
                return Some(KeyAction::Paste);
            }
        }
    }

    let bytes = key_bytes_with_modifiers(key, modified_key, modifiers)?;
    Some(KeyAction::Send(bytes))
}

/// Converte `(key, modified_key, ctrl)` na sequência de bytes correspondente
/// para envio ao PTY.
pub fn key_bytes(key: &Key, modified_key: &Key, ctrl: bool) -> Option<Vec<u8>> {
    let modifiers = if ctrl {
        Modifiers::CTRL
    } else {
        Modifiers::NONE
    };
    key_bytes_with_modifiers(key, modified_key, modifiers)
}

/// Converte `(key, modified_key, modifiers)` em bytes VT/ANSI.
pub fn key_bytes_with_modifiers(
    key: &Key,
    modified_key: &Key,
    modifiers: Modifiers,
) -> Option<Vec<u8>> {
    let ctrl = modifiers.control();
    let shift = modifiers.shift();

    if let Key::Named(named) = key.as_ref() {
        return named_key_bytes(named, shift, ctrl).map(<[u8]>::to_vec);
    }

    if ctrl {
        let ch = extract_single_char(key).or_else(|| extract_single_char(modified_key))?;
        let code = ctrl_char_byte(ch)?;
        return Some(vec![code]);
    }

    match modified_key.as_ref() {
        Key::Character(s) if !s.is_empty() => Some(s.as_bytes().to_vec()),
        _ => match key.as_ref() {
            Key::Character(s) if !s.is_empty() => Some(s.as_bytes().to_vec()),
            _ => None,
        },
    }
}

/// Converte uma tecla especial [`Named`] na sequência de escape ANSI/xterm.
pub const fn named_key_bytes(named: Named, shift: bool, ctrl: bool) -> Option<&'static [u8]> {
    match named {
        Named::Enter => Some(b"\r"),
        Named::Backspace => Some(b"\x7f"),
        Named::Tab => {
            if shift {
                Some(b"\x1b[Z")
            } else {
                Some(b"\t")
            }
        }
        Named::Escape => Some(b"\x1b"),
        Named::Space => {
            if ctrl {
                Some(b"\x00")
            } else {
                Some(b" ")
            }
        }
        Named::ArrowUp => Some(b"\x1b[A"),
        Named::ArrowDown => Some(b"\x1b[B"),
        Named::ArrowRight => Some(b"\x1b[C"),
        Named::ArrowLeft => Some(b"\x1b[D"),
        Named::Home => Some(b"\x1b[H"),
        Named::End => Some(b"\x1b[F"),
        Named::PageUp => Some(b"\x1b[5~"),
        Named::PageDown => Some(b"\x1b[6~"),
        Named::Insert => Some(b"\x1b[2~"),
        Named::Delete => Some(b"\x1b[3~"),
        Named::F1 => Some(b"\x1bOP"),
        Named::F2 => Some(b"\x1bOQ"),
        Named::F3 => Some(b"\x1bOR"),
        Named::F4 => Some(b"\x1bOS"),
        Named::F5 => Some(b"\x1b[15~"),
        Named::F6 => Some(b"\x1b[17~"),
        Named::F7 => Some(b"\x1b[18~"),
        Named::F8 => Some(b"\x1b[19~"),
        Named::F9 => Some(b"\x1b[20~"),
        Named::F10 => Some(b"\x1b[21~"),
        Named::F11 => Some(b"\x1b[23~"),
        Named::F12 => Some(b"\x1b[24~"),
        _ => None,
    }
}

/// Converte um caractere combinado com `Ctrl` no byte de controle ASCII (`0x00..=0x1F` / `0x7F`).
pub const fn ctrl_char_byte(ch: char) -> Option<u8> {
    match ch {
        'a'..='z' => Some((ch as u8) - b'a' + 1),
        'A'..='Z' => Some((ch as u8) - b'A' + 1),
        '@' | ' ' => Some(0x00),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '^' => Some(0x1e),
        '_' => Some(0x1f),
        '?' => Some(0x7f),
        _ => None,
    }
}

fn extract_single_char(key: &Key) -> Option<char> {
    if let Key::Character(s) = key.as_ref() {
        let mut chars = s.chars();
        let first = chars.next()?;
        if chars.next().is_none() {
            return Some(first);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn char_key(s: &str) -> Key {
        Key::Character(s.into())
    }

    #[test]
    fn maps_special_named_keys_and_function_keys() {
        assert_eq!(
            key_bytes(&Key::Named(Named::Enter), &Key::Named(Named::Enter), false),
            Some(b"\r".to_vec())
        );
        assert_eq!(
            key_bytes(
                &Key::Named(Named::Backspace),
                &Key::Named(Named::Backspace),
                false
            ),
            Some(b"\x7f".to_vec())
        );
        assert_eq!(
            key_bytes(&Key::Named(Named::Tab), &Key::Named(Named::Tab), false),
            Some(b"\t".to_vec())
        );
        assert_eq!(
            key_bytes_with_modifiers(
                &Key::Named(Named::Tab),
                &Key::Named(Named::Tab),
                Modifiers::SHIFT
            ),
            Some(b"\x1b[Z".to_vec())
        );
        assert_eq!(
            key_bytes(
                &Key::Named(Named::Escape),
                &Key::Named(Named::Escape),
                false
            ),
            Some(b"\x1b".to_vec())
        );
        assert_eq!(
            key_bytes(
                &Key::Named(Named::ArrowUp),
                &Key::Named(Named::ArrowUp),
                false
            ),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_bytes(
                &Key::Named(Named::ArrowDown),
                &Key::Named(Named::ArrowDown),
                false
            ),
            Some(b"\x1b[B".to_vec())
        );
        assert_eq!(
            key_bytes(
                &Key::Named(Named::ArrowRight),
                &Key::Named(Named::ArrowRight),
                false
            ),
            Some(b"\x1b[C".to_vec())
        );
        assert_eq!(
            key_bytes(
                &Key::Named(Named::ArrowLeft),
                &Key::Named(Named::ArrowLeft),
                false
            ),
            Some(b"\x1b[D".to_vec())
        );
        assert_eq!(
            key_bytes(&Key::Named(Named::Home), &Key::Named(Named::Home), false),
            Some(b"\x1b[H".to_vec())
        );
        assert_eq!(
            key_bytes(&Key::Named(Named::End), &Key::Named(Named::End), false),
            Some(b"\x1b[F".to_vec())
        );
        assert_eq!(
            key_bytes(
                &Key::Named(Named::PageUp),
                &Key::Named(Named::PageUp),
                false
            ),
            Some(b"\x1b[5~".to_vec())
        );
        assert_eq!(
            key_bytes(
                &Key::Named(Named::PageDown),
                &Key::Named(Named::PageDown),
                false
            ),
            Some(b"\x1b[6~".to_vec())
        );
        assert_eq!(
            key_bytes(&Key::Named(Named::F1), &Key::Named(Named::F1), false),
            Some(b"\x1bOP".to_vec())
        );
        assert_eq!(
            key_bytes(&Key::Named(Named::F5), &Key::Named(Named::F5), false),
            Some(b"\x1b[15~".to_vec())
        );
        assert_eq!(
            key_bytes(&Key::Named(Named::F12), &Key::Named(Named::F12), false),
            Some(b"\x1b[24~".to_vec())
        );
    }

    #[test]
    fn maps_ctrl_letters_and_clipboard_shortcuts() {
        // Ctrl+C sem Shift envia ETX (0x03)
        assert_eq!(
            classify_key_press(&char_key("c"), &char_key("c"), Modifiers::CTRL),
            Some(KeyAction::Send(vec![0x03]))
        );
        // Ctrl+D envia EOT (0x04), Ctrl+Z envia SUB (0x1A)
        assert_eq!(
            key_bytes(&char_key("d"), &char_key("d"), true),
            Some(vec![0x04])
        );
        assert_eq!(
            key_bytes(&char_key("z"), &char_key("z"), true),
            Some(vec![0x1a])
        );

        // Ctrl+Shift+C copia; Ctrl+Shift+V cola
        let ctrl_shift = Modifiers::CTRL | Modifiers::SHIFT;
        assert_eq!(
            classify_key_press(&char_key("c"), &char_key("C"), ctrl_shift),
            Some(KeyAction::Copy)
        );
        assert_eq!(
            classify_key_press(&char_key("v"), &char_key("V"), ctrl_shift),
            Some(KeyAction::Paste)
        );

        // Caracteres normais e modificados por Shift
        assert_eq!(
            classify_key_press(&char_key("a"), &char_key("A"), Modifiers::SHIFT),
            Some(KeyAction::Send(b"A".to_vec()))
        );
        assert_eq!(
            classify_key_press(&char_key("ç"), &char_key("ç"), Modifiers::NONE),
            Some(KeyAction::Send("ç".as_bytes().to_vec()))
        );
    }
}
