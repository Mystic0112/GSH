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

#[cfg(test)]
mod find_tests {
    use super::*;
    use iced::widget::text_editor::Motion;

    #[test]
    fn find_navigates_with_wrap_and_closes_cleanly() {
        let mut pane = EditorPane::new("f.rs".into(), "needle\nother needle".into());
        let _ = pane.update(EditorMsg::FindOpen);
        let _ = pane.update(EditorMsg::FindQuery("needle".into()));
        assert_eq!(pane.find_matches.len(), 2);
        assert_eq!(pane.find_index, Some(0));
        let _ = pane.update(EditorMsg::FindNext);
        assert_eq!(pane.find_index, Some(1));
        let _ = pane.update(EditorMsg::FindNext);
        assert_eq!(pane.find_index, Some(0));
        let _ = pane.update(EditorMsg::FindPrevious);
        assert_eq!(pane.find_index, Some(1));
        let _ = pane.update(EditorMsg::FindClose);
        assert!(!pane.find_open);
        assert!(pane.find_matches.is_empty());
        assert_eq!(pane.state.buffer.text(), "needle\nother needle");
    }

    #[test]
    fn find_uses_a_single_line_selection_as_its_initial_query() {
        let mut pane = EditorPane::new("f.rs".into(), "needle other".into());
        let _ = pane.update(EditorMsg::Action(Action::Move(Motion::DocumentStart)));
        let _ = pane.update(EditorMsg::Action(Action::Select(Motion::WordRight)));
        let _ = pane.update(EditorMsg::FindOpen);
        assert_eq!(pane.find_query, "needle");
        assert_eq!(pane.find_matches.len(), 1);
    }
}

#[cfg(target_os = "linux")]
#[test]
fn pending_formatter_cannot_overwrite_edit_undo_and_format_is_one_undo_step() {
    use crate::lsp_runtime::Pool;
    use iced::widget::text_editor::Edit;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("file.lsptest");
    std::fs::write(&path, "a\nb\n").unwrap();
    let mut config = gsh_config::schema::Config::default();
    config
        .lsp_formatters
        .insert("lsptest".into(), "formatter %FILE%".into());
    let mut pool = Pool::default();
    assert!(pool.configure(&config));
    let mut pane = EditorPane::new(path.clone(), "a\nb\n".into());
    let mut doc = pool.open(&path, Some(dir.path())).unwrap();
    let stale = doc.version;
    doc.pending = true;
    let _ = pane.update(EditorMsg::Action(Action::Edit(Edit::Insert('x'))));
    pool.change(&mut doc, pane.highlight_text.clone());
    let _ = pane.update(EditorMsg::Undo);
    pool.change(&mut doc, pane.highlight_text.clone());
    let version = doc.version;
    doc.pending = true;
    pane.lsp = Some(doc);
    assert!(!pane.finish_lsp_format(stale, Ok("stale".into())));
    assert!(pane.finish_lsp_format(version, Ok("A\nB\n".into())));
    assert_eq!(pane.content.text(), "A\nB\n");
    let _ = pane.update(EditorMsg::Undo);
    assert_eq!(pane.content.text(), "a\nb\n");
    let _ = pane.update(EditorMsg::Redo);
    assert_eq!(pane.state.buffer.text(), "A\nB\n");
    assert_eq!(std::fs::read_to_string(path).unwrap(), "a\nb\n");
}
