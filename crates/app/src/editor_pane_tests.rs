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
