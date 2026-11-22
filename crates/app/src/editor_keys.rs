use crate::editor_pane::EditorMsg;
use iced::keyboard;
use iced::widget::text_editor::{self, Binding, KeyPress, Status};

pub(crate) fn bind_key(
    key: KeyPress,
    page_lines: usize,
    tab_size: u8,
    submit_shortcut: bool,
) -> Option<Binding<EditorMsg>> {
    if matches!(key.status, Status::Focused { .. }) {
        if submit_shortcut
            && key.modifiers.command()
            && matches!(key.key, keyboard::Key::Named(keyboard::key::Named::Enter))
        {
            return Some(Binding::Custom(EditorMsg::Submit));
        }
        if key.modifiers.command() {
            if let keyboard::Key::Character(c) = &key.key {
                match c.to_lowercase().as_str() {
                    "z" if key.modifiers.shift() => return Some(Binding::Custom(EditorMsg::Redo)),
                    "z" => return Some(Binding::Custom(EditorMsg::Undo)),
                    "y" => return Some(Binding::Custom(EditorMsg::Redo)),
                    _ => {}
                }
            }
        }
        if matches!(key.key, keyboard::Key::Named(keyboard::key::Named::Tab)) {
            return Some(if tab_size == 2 {
                Binding::Sequence(vec![Binding::Insert(' '); 2])
            } else {
                Binding::Insert('\t')
            });
        }
        let motion = match &key.key {
            keyboard::Key::Named(keyboard::key::Named::PageUp) => Some(text_editor::Motion::Up),
            keyboard::Key::Named(keyboard::key::Named::PageDown) => Some(text_editor::Motion::Down),
            _ => None,
        };
        if let Some(motion) = motion {
            let binding = if key.modifiers.shift() {
                Binding::Select(motion)
            } else {
                Binding::Move(motion)
            };
            return Some(Binding::Sequence(vec![binding; page_lines]));
        }
    }
    Binding::from_key_press(key)
}
