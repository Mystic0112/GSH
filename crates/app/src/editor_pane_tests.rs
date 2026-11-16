use super::*;
#[cfg(test)]
mod history_tests {
    use super::*;
    use iced::widget::text_editor::{Edit, Motion};

    #[test]
    fn editing_and_history_preserve_line_endings_and_final_newline() {
        for text in ["a\nb\n", "a\r\nb\r\n", "a\nb"] {
            let mut pane = EditorPane::new("test.rs".into(), text.into());
            assert_eq!(pane.content.text(), text);
            let _ = pane.update(EditorMsg::Action(Action::Move(Motion::DocumentEnd)));
            let _ = pane.update(EditorMsg::Action(Action::Edit(Edit::Insert('é'))));
            assert_eq!(pane.state.buffer.text(), format!("{text}é"));
            let _ = pane.update(EditorMsg::Undo);
            assert_eq!(pane.state.buffer.text(), text);
            let _ = pane.update(EditorMsg::Redo);
            assert_eq!(pane.state.buffer.text(), format!("{text}é"));
        }
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;
    use iced::widget::text_editor::{Edit, Motion};
    fn action(pane: &mut EditorPane, action: Action) {
        let _ = pane.update(EditorMsg::Action(action));
    }
    #[test]
    fn production_edits_cover_line_boundaries_multibyte_and_tab() {
        let mut pane = EditorPane::new("file".into(), "á\nb".into());
        action(&mut pane, Action::Move(Motion::Down));
        action(&mut pane, Action::Move(Motion::Home));
        action(&mut pane, Action::Edit(Edit::Backspace));
        assert_eq!(pane.state.buffer.text(), "áb");
        let _ = pane.update(EditorMsg::Undo);
        action(&mut pane, Action::Move(Motion::DocumentStart));
        action(&mut pane, Action::Move(Motion::End));
        action(&mut pane, Action::Edit(Edit::Delete));
        assert_eq!(pane.state.buffer.text(), "áb");
        action(&mut pane, Action::Edit(Edit::Insert('\t')));
        action(&mut pane, Action::Edit(Edit::Insert('中')));
        assert_eq!(pane.state.buffer.text(), "á\t中b");
        let _ = pane.update(EditorMsg::Undo);
        assert_eq!(pane.state.buffer.text(), "á\tb");
    }
}

#[cfg(test)]
mod reveal_tests {
    use super::*;

    #[test]
    fn reveal_line_puts_the_cursor_at_the_start_of_the_line_without_selecting() {
        let mut pane = EditorPane::new("f.rs".into(), "one\ntwo\n  three\n".into());
        let _ = pane.reveal_line(3);
        let cursor = pane.content.cursor();
        assert_eq!((cursor.position.line, cursor.position.column), (2, 0));
        assert!(cursor.selection.is_none());
        assert_eq!(pane.state.buffer.text(), "one\ntwo\n  three\n");
        assert!(!pane.state.buffer.is_dirty());
    }

    #[test]
    fn reveal_line_ignores_lines_outside_the_file() {
        let mut pane = EditorPane::new("f.rs".into(), "one\ntwo".into());
        let _ = pane.reveal_line(2);
        let _ = pane.reveal_line(0);
        let _ = pane.reveal_line(99);
        assert_eq!(pane.content.cursor().position.line, 1);
    }
}
