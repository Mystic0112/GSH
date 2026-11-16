//! Adaptador de interação do editor iced; o documento pertence a EditorState.
use crate::{editor_state::EditorState, theme};
use iced::widget::text_editor::{Action, Content, Cursor, Position};
use iced::Task;

#[derive(Debug, Clone)]
pub enum EditorMsg {
    Action(Action),
    Save,
    Undo,
    Redo,
    Overwrite,
    Viewport(f32, f32),
    /// Ctrl/Cmd+Enter, só nos editores que o pedem (`with_submit_shortcut`): a aba de query o usa
    /// para executar. Não faz nada no editor de arquivo.
    Submit,
}

#[derive(Debug)]
pub struct EditorPane {
    pub state: EditorState,
    pub(crate) content: Content,
    undo_cursors: Vec<Cursor>,
    redo_cursors: Vec<Cursor>,
    pub(crate) scroll_id: iced::widget::Id,
    pub(crate) scroll_y: f32,
    pub(crate) viewport_height: std::cell::Cell<f32>,
    pub(crate) highlight_text: std::sync::Arc<str>,
    pub(crate) gutter: crate::editor_gutter::Gutter,
    pub(crate) longest_line: usize,
    pub(crate) config: gsh_config::schema::Editor,
    pub(crate) submit_shortcut: bool,
}
impl EditorPane {
    pub fn new(path: std::path::PathBuf, text: String) -> Self {
        let highlight_text = text.as_str().into();
        let longest_line = longest_line(&text);
        let mut gutter = crate::editor_gutter::Gutter::default();
        gutter.update(text.split('\n').count());
        Self {
            content: Content::with_text(&text),
            state: EditorState::new(path, text),
            undo_cursors: Vec::new(),
            redo_cursors: Vec::new(),
            scroll_id: iced::widget::Id::unique(),
            scroll_y: 0.0,
            viewport_height: std::cell::Cell::new(0.0),
            highlight_text,
            gutter,
            longest_line,
            config: gsh_config::schema::Editor::default(),
            submit_shortcut: false,
        }
    }
    /// Ctrl/Cmd+Enter vira `EditorMsg::Submit` em vez de uma quebra de linha.
    pub fn with_submit_shortcut(mut self) -> Self {
        self.submit_shortcut = true;
        self
    }
    pub fn with_config(mut self, config: gsh_config::schema::Editor) -> Self {
        self.set_config(config);
        self
    }
    pub fn set_config(&mut self, config: gsh_config::schema::Editor) {
        self.config = config;
    }
    pub fn update(&mut self, msg: EditorMsg) -> Task<EditorMsg> {
        match msg {
            EditorMsg::Viewport(y, height) => {
                self.scroll_y = y;
                self.viewport_height.set(height);
            }
            EditorMsg::Action(Action::Scroll { lines }) => {
                return iced::widget::operation::scroll_by(
                    self.scroll_id.clone(),
                    iced::widget::scrollable::AbsoluteOffset {
                        x: 0.0,
                        y: lines as f32 * theme::SIZE_MONO * theme::LINE_HEIGHT_MONO,
                    },
                );
            }
            EditorMsg::Action(action) => {
                let before = self.content.cursor();
                let edit = action.is_edit();
                if !edit {
                    self.state.buffer.break_group();
                }
                let follow = matches!(
                    action,
                    Action::Move(_) | Action::Select(_) | Action::Edit(_)
                );
                self.content.perform(action);
                self.gutter.update(self.content.line_count());
                if edit {
                    let text = self.content.text();
                    if text != self.state.buffer.text() {
                        let previous_step = self.state.buffer.history_marker();
                        if matches!(self.state.replace_text(&text), Ok(true)) {
                            if previous_step != self.state.buffer.history_marker() {
                                self.undo_cursors.push(before);
                                if self.undo_cursors.len() > 500 {
                                    self.undo_cursors.remove(0);
                                }
                            }
                            self.redo_cursors.clear();
                        } else {
                            self.restore(before);
                        }
                        self.highlight_text = self.state.buffer.text().into();
                        self.longest_line = longest_line(self.state.buffer.text());
                    }
                }
                if follow && self.viewport_height.get() > 0.0 {
                    let line_height = theme::SIZE_MONO * theme::LINE_HEIGHT_MONO;
                    let y = self.content.cursor().position.line as f32 * line_height
                        + theme::EDITOR_PAD;
                    let delta = crate::editor_scroll::follow_delta(
                        y,
                        line_height,
                        self.scroll_y,
                        self.viewport_height.get(),
                    );
                    if delta != 0.0 {
                        return iced::widget::operation::scroll_by(
                            self.scroll_id.clone(),
                            iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: delta },
                        );
                    }
                }
            }
            EditorMsg::Submit => {}
            EditorMsg::Save => self.state.save(),
            EditorMsg::Overwrite => self.state.overwrite_confirmed(),
            EditorMsg::Undo => {
                if let Some(cursor) = self.undo_cursors.pop() {
                    self.redo_cursors.push(self.content.cursor());
                    self.state.undo();
                    self.restore(cursor);
                }
            }
            EditorMsg::Redo => {
                if let Some(cursor) = self.redo_cursors.pop() {
                    self.undo_cursors.push(self.content.cursor());
                    self.state.redo();
                    self.restore(cursor);
                }
            }
        }
        Task::none()
    }
    /// Resultado de busca: cursor no início da linha (base 1) e rolagem até ela, como o
    /// `_reveal(select: false)` do Flutter. Linha fora do arquivo: não faz nada.
    pub fn reveal_line(&mut self, line: usize) -> Task<EditorMsg> {
        let Some(index) = line
            .checked_sub(1)
            .filter(|index| *index < self.content.line_count())
        else {
            return Task::none();
        };
        self.content.move_to(Cursor {
            position: Position {
                line: index,
                column: 0,
            },
            selection: None,
        });
        let y =
            crate::editor_scroll::reveal_offset(index, theme::SIZE_MONO * theme::LINE_HEIGHT_MONO);
        iced::widget::operation::scroll_to(
            self.scroll_id.clone(),
            iced::widget::scrollable::AbsoluteOffset {
                x: Some(0.0),
                y: Some(y),
            },
        )
    }
    fn restore(&mut self, cursor: Cursor) {
        self.content = Content::with_text(self.state.buffer.text());
        self.content.move_to(cursor);
        self.highlight_text = self.state.buffer.text().into();
        self.gutter.update(self.content.line_count());
        self.longest_line = longest_line(self.state.buffer.text());
    }
}

fn longest_line(text: &str) -> usize {
    text.lines()
        .map(|line| {
            line.chars()
                .map(|ch| {
                    if ch == '\t' {
                        4
                    } else if ch.is_ascii() {
                        1
                    } else {
                        2
                    }
                })
                .sum()
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "editor_pane_tests.rs"]
mod tests;
