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
    /// Copy when there is a selection, otherwise send Ctrl+C to the process.
    CopyOrInterrupt,
    /// Cola o conteúdo da área de transferência no PTY (`Ctrl+Shift+V`).
    Paste,
}

/// Prepara o texto do clipboard conforme o modo bracketed paste ativo.
pub fn paste_bytes(contents: &str, bracketed: bool) -> Vec<u8> {
    if !bracketed {
        return contents.as_bytes().to_vec();
    }
    let mut bytes = b"\x1b[200~".to_vec();
    bytes.extend_from_slice(contents.as_bytes());
    bytes.extend_from_slice(b"\x1b[201~");
    bytes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseReportOptions {
    pub release: bool,
    pub motion: bool,
    pub modifiers: Modifiers,
    pub utf8: bool,
    pub sgr: bool,
}

/// Codifica um evento de mouse xterm. `button` usa o código base do protocolo
/// (0..2 botões, 64/65 roda, 3 soltura), e coordenadas são 1-based.
pub fn mouse_report_bytes(
    button: u8,
    col: usize,
    row: usize,
    options: MouseReportOptions,
) -> Vec<u8> {
    let MouseReportOptions {
        release,
        motion,
        modifiers,
        utf8,
        sgr,
    } = options;
    let modifier_bits = u8::from(modifiers.shift()) * 4
        + u8::from(modifiers.alt()) * 8
        + u8::from(modifiers.control()) * 16;
    let code = if release {
        3_u8.saturating_add(modifier_bits)
    } else {
        button
            .saturating_add(modifier_bits)
            .saturating_add(if motion { 32 } else { 0 })
    };
    let x = col.saturating_add(1);
    let y = row.saturating_add(1);
    if sgr {
        return format!("\x1b[<{code};{x};{y}{}", if release { 'm' } else { 'M' }).into_bytes();
    }
    let mut bytes = vec![0x1b, b'[', b'M'];
    if utf8 {
        for value in [
            u32::from(code) + 32,
            u32::try_from(x).unwrap_or(u32::MAX) + 32,
            u32::try_from(y).unwrap_or(u32::MAX) + 32,
        ] {
            if let Some(character) = char::from_u32(value) {
                let mut encoded = [0; 4];
                bytes.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
            }
        }
    } else {
        bytes.push(32_u8.saturating_add(code));
        bytes.push(u8::try_from(x).unwrap_or(u8::MAX).saturating_add(32));
        bytes.push(u8::try_from(y).unwrap_or(u8::MAX).saturating_add(32));
    }
    bytes
}

/// Classifica uma tecla pressionada em [`KeyAction`], tratando primeiro os
/// atalhos de área de transferência (`Ctrl+C` / `Ctrl+V`) e em
/// seguida a conversão para bytes VT/ANSI.
pub fn classify_key_press(
    key: &Key,
    modified_key: &Key,
    modifiers: Modifiers,
) -> Option<KeyAction> {
    classify_key_press_with_modes(key, modified_key, modifiers, false)
}

/// Classifica a tecla usando o modo de cursor de aplicação anunciado pela TUI.
pub fn classify_key_press_with_modes(
    key: &Key,
    modified_key: &Key,
    modifiers: Modifiers,
    application_cursor: bool,
) -> Option<KeyAction> {
    if modifiers.logo() {
        let ch = extract_single_char(key).or_else(|| extract_single_char(modified_key));
        return match ch.map(|value| value.to_ascii_lowercase()) {
            Some('v') => Some(KeyAction::Paste),
            Some('c') => Some(KeyAction::CopyOrInterrupt),
            _ => None,
        };
    }

    if matches!(key.as_ref(), Key::Named(Named::Copy)) {
        return Some(KeyAction::Copy);
    }
    if matches!(key.as_ref(), Key::Named(Named::Paste)) {
        return Some(KeyAction::Paste);
    }

    if modifiers.control() && !modifiers.alt() {
        if let Some(ch) = extract_single_char(key).or_else(|| extract_single_char(modified_key)) {
            if ch.eq_ignore_ascii_case(&'v') {
                return Some(KeyAction::Paste);
            }
            if ch.eq_ignore_ascii_case(&'c') && !modifiers.shift() {
                return Some(KeyAction::CopyOrInterrupt);
            }
        }
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

    let bytes = key_bytes_with_terminal_modes(key, modified_key, modifiers, application_cursor)?;
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
    let alt = modifiers.alt();

    if let Key::Named(named) = key.as_ref() {
        return named_key_bytes_with_modes(named, shift, alt, ctrl, false);
    }

    if ctrl && !shift && !alt {
        let ch = extract_single_char(key).or_else(|| extract_single_char(modified_key))?;
        let code = ctrl_char_byte(ch)?;
        return Some(vec![code]);
    }

    let mut bytes = match modified_key.as_ref() {
        Key::Character(s) if !s.is_empty() => Some(s.as_bytes().to_vec()),
        _ => match key.as_ref() {
            Key::Character(s) if !s.is_empty() => Some(s.as_bytes().to_vec()),
            _ => None,
        },
    }?;
    if alt && !ctrl && !shift {
        bytes.insert(0, 0x1b);
    }
    Some(bytes)
}

/// Traduz uma tecla usando os modos ativos do emulador VT.
pub fn key_bytes_with_terminal_modes(
    key: &Key,
    modified_key: &Key,
    modifiers: Modifiers,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    if let Key::Named(named) = key.as_ref() {
        return named_key_bytes_with_modes(
            named,
            modifiers.shift(),
            modifiers.alt(),
            modifiers.control(),
            application_cursor,
        );
    }
    key_bytes_with_modifiers(key, modified_key, modifiers)
}

fn named_key_bytes_with_modes(
    named: Named,
    shift: bool,
    alt: bool,
    ctrl: bool,
    application_cursor: bool,
) -> Option<Vec<u8>> {
    if named == Named::Enter {
        return Some(if shift {
            b"\x1bOM".to_vec()
        } else {
            b"\r".to_vec()
        });
    }
    if named == Named::Backspace {
        return Some(if ctrl {
            b"\x08".to_vec()
        } else if alt {
            b"\x1b\x7f".to_vec()
        } else {
            b"\x7f".to_vec()
        });
    }
    let modifier = 1 + u8::from(shift) + 2 * u8::from(alt) + 4 * u8::from(ctrl);
    let final_byte = match named {
        Named::ArrowUp => Some(b'A'),
        Named::ArrowDown => Some(b'B'),
        Named::ArrowRight => Some(b'C'),
        Named::ArrowLeft => Some(b'D'),
        Named::Home => Some(b'H'),
        Named::End => Some(b'F'),
        _ => None,
    };
    if let Some(final_byte) = final_byte {
        if modifier > 1 {
            return Some(format!("\x1b[1;{modifier}{}", char::from(final_byte)).into_bytes());
        }
        let app = application_cursor
            && matches!(
                named,
                Named::ArrowUp
                    | Named::ArrowDown
                    | Named::ArrowLeft
                    | Named::ArrowRight
                    | Named::Home
                    | Named::End
            );
        return Some(if app {
            vec![0x1b, b'O', final_byte]
        } else {
            vec![0x1b, b'[', final_byte]
        });
    }
    let tilde = match named {
        Named::PageUp => Some(5),
        Named::PageDown => Some(6),
        Named::Insert => Some(2),
        Named::Delete => Some(3),
        Named::F5 => Some(15),
        Named::F6 => Some(17),
        Named::F7 => Some(18),
        Named::F8 => Some(19),
        Named::F9 => Some(20),
        Named::F10 => Some(21),
        Named::F11 => Some(23),
        Named::F12 => Some(24),
        _ => None,
    };
    if let Some(code) = tilde {
        return Some(if modifier > 1 {
            format!("\x1b[{code};{modifier}~").into_bytes()
        } else {
            format!("\x1b[{code}~").into_bytes()
        });
    }
    let ss3 = match named {
        Named::F1 => Some(b'P'),
        Named::F2 => Some(b'Q'),
        Named::F3 => Some(b'R'),
        Named::F4 => Some(b'S'),
        _ => None,
    };
    if let Some(code) = ss3 {
        return Some(if modifier > 1 {
            format!("\x1bO{modifier}{}", char::from(code)).into_bytes()
        } else {
            vec![0x1b, b'O', code]
        });
    }
    named_key_bytes(named, shift, ctrl).map(<[u8]>::to_vec)
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
    fn matches_flutter_keytab_for_modifiers_and_application_cursor() {
        let cases = [
            (
                Named::ArrowLeft,
                Modifiers::NONE,
                false,
                b"\x1b[D".as_slice(),
            ),
            (
                Named::ArrowLeft,
                Modifiers::NONE,
                true,
                b"\x1bOD".as_slice(),
            ),
            (
                Named::ArrowLeft,
                Modifiers::CTRL,
                false,
                b"\x1b[1;5D".as_slice(),
            ),
            (
                Named::ArrowRight,
                Modifiers::ALT,
                false,
                b"\x1b[1;3C".as_slice(),
            ),
            (
                Named::Home,
                Modifiers::SHIFT,
                false,
                b"\x1b[1;2H".as_slice(),
            ),
            (
                Named::PageDown,
                Modifiers::CTRL,
                false,
                b"\x1b[6;5~".as_slice(),
            ),
            (
                Named::Delete,
                Modifiers::SHIFT,
                false,
                b"\x1b[3;2~".as_slice(),
            ),
            (Named::F5, Modifiers::ALT, false, b"\x1b[15;3~".as_slice()),
            (Named::F1, Modifiers::CTRL, false, b"\x1bO5P".as_slice()),
            (Named::Enter, Modifiers::SHIFT, false, b"\x1bOM".as_slice()),
            (Named::Enter, Modifiers::NONE, false, b"\r".as_slice()),
            (Named::Backspace, Modifiers::CTRL, false, b"\x08".as_slice()),
            (
                Named::Backspace,
                Modifiers::ALT,
                false,
                b"\x1b\x7f".as_slice(),
            ),
        ];
        for (named, modifiers, app_cursor, expected) in cases {
            assert_eq!(
                key_bytes_with_terminal_modes(
                    &Key::Named(named),
                    &Key::Named(named),
                    modifiers,
                    app_cursor,
                )
                .as_deref(),
                Some(expected),
                "{named:?} with {modifiers:?}"
            );
        }
        assert_eq!(
            key_bytes_with_modifiers(&char_key("b"), &char_key("b"), Modifiers::ALT),
            Some(b"\x1bb".to_vec())
        );
        assert_eq!(
            key_bytes_with_modifiers(&char_key("c"), &char_key("c"), Modifiers::CTRL),
            Some(vec![0x03])
        );
    }

    #[test]
    fn wraps_clipboard_text_only_when_bracketed_paste_is_enabled() {
        assert_eq!(paste_bytes("line\n", false), b"line\n");
        assert_eq!(paste_bytes("line\n", true), b"\x1b[200~line\n\x1b[201~");
    }

    #[test]
    fn encodes_mouse_click_release_motion_and_wheel_in_sgr_and_legacy_forms() {
        assert_eq!(
            mouse_report_bytes(
                0,
                4,
                2,
                MouseReportOptions {
                    release: false,
                    motion: false,
                    modifiers: Modifiers::NONE,
                    utf8: false,
                    sgr: true
                }
            ),
            b"\x1b[<0;5;3M"
        );
        assert_eq!(
            mouse_report_bytes(
                0,
                4,
                2,
                MouseReportOptions {
                    release: true,
                    motion: false,
                    modifiers: Modifiers::NONE,
                    utf8: false,
                    sgr: true
                }
            ),
            b"\x1b[<3;5;3m"
        );
        assert_eq!(
            mouse_report_bytes(
                0,
                4,
                2,
                MouseReportOptions {
                    release: false,
                    motion: true,
                    modifiers: Modifiers::NONE,
                    utf8: false,
                    sgr: true
                }
            ),
            b"\x1b[<32;5;3M"
        );
        assert_eq!(
            mouse_report_bytes(
                64,
                4,
                2,
                MouseReportOptions {
                    release: false,
                    motion: false,
                    modifiers: Modifiers::NONE,
                    utf8: false,
                    sgr: true
                }
            ),
            b"\x1b[<64;5;3M"
        );
        assert_eq!(
            mouse_report_bytes(
                0,
                4,
                2,
                MouseReportOptions {
                    release: false,
                    motion: false,
                    modifiers: Modifiers::NONE,
                    utf8: false,
                    sgr: false
                }
            ),
            b"\x1b[M %#"
        );
        assert_eq!(
            mouse_report_bytes(
                0,
                223,
                0,
                MouseReportOptions {
                    release: false,
                    motion: false,
                    modifiers: Modifiers::NONE,
                    utf8: true,
                    sgr: false
                }
            ),
            "\x1b[M \u{100}!".as_bytes()
        );
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
        // Ctrl+C copia a seleção ou envia ETX quando não há seleção.
        assert_eq!(
            classify_key_press(&char_key("c"), &char_key("c"), Modifiers::CTRL),
            Some(KeyAction::CopyOrInterrupt)
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
