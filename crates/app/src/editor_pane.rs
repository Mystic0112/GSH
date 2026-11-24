//! Adaptador de interação do editor iced; o documento pertence a EditorState.
use crate::{editor_state::EditorState, theme};
use iced::widget::text_editor::{Action, Content, Cursor, Position};
use iced::Task;

#[derive(Debug, Clone)]
pub enum EditorMsg {
    Action(Action),
    Save,
    TogglePreview,
    OpenPath(std::path::PathBuf),
    Undo,
    Redo,
    Overwrite,
    #[cfg(target_os = "linux")]
    Format,
    #[cfg(target_os = "linux")]
    Formatted(i64, Result<String, String>),
    #[cfg(target_os = "linux")]
    RevealDiagnostic(usize),
    ApplyCkpLayout,
    Viewport(f32, f32),
    /// Ctrl/Cmd+Enter, só nos editores que o pedem (`with_submit_shortcut`): a aba de query o usa
    /// para executar. Não faz nada no editor de arquivo.
    Submit,
    FindOpen,
    FindClose,
    FindQuery(String),
    FindToggleCase,
    FindToggleWord,
    FindToggleRegex,
    FindNext,
    FindPrevious,
    RevealLine(usize),
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
    pub(crate) ckp_error: Option<String>,
    #[cfg(target_os = "linux")]
    pub(crate) lsp: Option<crate::lsp_runtime::Document>,
    pub(crate) find_open: bool,
    pub(crate) find_query: String,
    pub(crate) find_case: bool,
    pub(crate) find_word: bool,
    pub(crate) find_regex: bool,
    pub(crate) find_matches: Vec<crate::editor_find::MatchRange>,
    pub(crate) find_index: Option<usize>,
    pub(crate) find_invalid: bool,
}
impl EditorPane {
    pub fn new(path: std::path::PathBuf, text: String) -> Self {
        Self::with_encoding(path, text, crate::editor_state::TextEncoding::Utf8)
    }
    pub fn with_encoding(
        path: std::path::PathBuf,
        text: String,
        encoding: crate::editor_state::TextEncoding,
    ) -> Self {
        let highlight_text = text.as_str().into();
        let longest_line = longest_line(&text);
        let mut gutter = crate::editor_gutter::Gutter::default();
        gutter.update(text.split('\n').count());
        Self {
            content: Content::with_text(&text),
            state: EditorState::with_encoding(path, text, encoding),
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
            ckp_error: None,
            #[cfg(target_os = "linux")]
            lsp: None,
            find_open: false,
            find_query: String::new(),
            find_case: false,
            find_word: false,
            find_regex: false,
            find_matches: Vec::new(),
            find_index: None,
            find_invalid: false,
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
    pub(crate) fn overview_marks(&self) -> Vec<crate::editor_overview_ruler::Mark> {
        let text = self.state.buffer.text();
        let mut marks = Vec::new();
        for (index, found) in self.find_matches.iter().enumerate() {
            let line = text
                .get(..found.start)
                .unwrap_or_default()
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            marks.push(crate::editor_overview_ruler::Mark {
                line,
                lane: 0,
                color: if Some(index) == self.find_index {
                    theme::palette().accent_text
                } else {
                    theme::palette().accent
                },
            });
        }
        #[cfg(target_os = "linux")]
        if let Some(document) = &self.lsp {
            for diagnostic in &document.diagnostics {
                let color = match diagnostic.severity {
                    Some(1) => theme::palette().error,
                    Some(2) => theme::palette().warn,
                    _ => theme::palette().text2,
                };
                marks.push(crate::editor_overview_ruler::Mark {
                    line: diagnostic.range.start.line.saturating_add(1) as usize,
                    lane: 2,
                    color,
                });
            }
        }
        marks
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
                                self.record_undo_cursor(before);
                            }
                            self.redo_cursors.clear();
                        } else {
                            self.restore(before);
                        }
                        self.highlight_text = self.state.buffer.text().into();
                        self.longest_line = longest_line(self.state.buffer.text());
                        if self.find_open {
                            let _ = self.recompute_find(false);
                        }
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
            EditorMsg::Submit
            | EditorMsg::ApplyCkpLayout
            | EditorMsg::TogglePreview
            | EditorMsg::OpenPath(_) => {}
            EditorMsg::FindOpen => {
                self.find_open = true;
                let cursor = self.content.cursor();
                if let Some(anchor) = cursor
                    .selection
                    .filter(|anchor| anchor.line == cursor.position.line)
                {
                    let line = self
                        .state
                        .buffer
                        .text()
                        .split('\n')
                        .nth(cursor.position.line)
                        .unwrap_or_default();
                    let start = anchor.column.min(cursor.position.column);
                    let end = anchor.column.max(cursor.position.column);
                    if let Some(selected) = line.get(start..end).filter(|text| !text.is_empty()) {
                        self.find_query = selected.to_owned();
                    }
                }
                return iced::Task::batch([
                    self.recompute_find(true),
                    iced::widget::operation::focus(crate::editor_view::FIND_INPUT_ID),
                    iced::widget::operation::select_all(crate::editor_view::FIND_INPUT_ID),
                ]);
            }
            EditorMsg::FindClose => {
                self.find_open = false;
                self.find_matches.clear();
                self.find_index = None;
                self.find_invalid = false;
            }
            EditorMsg::FindQuery(query) => {
                self.find_query = query;
                return self.recompute_find(true);
            }
            EditorMsg::FindToggleCase => {
                self.find_case = !self.find_case;
                return self.recompute_find(true);
            }
            EditorMsg::FindToggleWord => {
                self.find_word = !self.find_word;
                return self.recompute_find(true);
            }
            EditorMsg::FindToggleRegex => {
                self.find_regex = !self.find_regex;
                return self.recompute_find(true);
            }
            EditorMsg::FindNext => return self.move_find(1),
            EditorMsg::FindPrevious => return self.move_find(-1),
            EditorMsg::RevealLine(line) => return self.reveal_line(line),
            #[cfg(target_os = "linux")]
            EditorMsg::Format | EditorMsg::Formatted(_, _) => {}
            #[cfg(target_os = "linux")]
            EditorMsg::RevealDiagnostic(line) => return self.reveal_line(line),
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
    fn record_undo_cursor(&mut self, cursor: Cursor) {
        self.undo_cursors.push(cursor);
        if self.undo_cursors.len() > 500 {
            self.undo_cursors.remove(0);
        }
    }
    #[cfg(target_os = "linux")]
    pub(crate) fn finish_lsp_format(
        &mut self,
        version: i64,
        result: Result<String, String>,
    ) -> bool {
        let Some(document) = self
            .lsp
            .as_mut()
            .filter(|doc| doc.pending && doc.version == version)
        else {
            return false;
        };
        document.pending = false;
        let text = match result {
            Ok(text) => text,
            Err(error) => {
                document.error = Some(error);
                return false;
            }
        };
        if self.state.notice.is_some() {
            return false;
        }
        let cursor = self.content.cursor();
        self.state.buffer.break_group();
        if !matches!(self.state.replace_text(&text), Ok(true)) {
            return false;
        }
        self.state.buffer.break_group();
        self.record_undo_cursor(cursor);
        self.redo_cursors.clear();
        self.restore(Cursor {
            selection: None,
            ..cursor
        });
        true
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
    fn restore(&mut self, mut cursor: Cursor) {
        self.content = Content::with_text(self.state.buffer.text());
        cursor.position.line = cursor
            .position
            .line
            .min(self.content.line_count().saturating_sub(1));
        cursor.position.column = self.content.line(cursor.position.line).map_or(0, |line| {
            let mut column = cursor.position.column.min(line.text.len());
            while !line.text.is_char_boundary(column) {
                column -= 1;
            }
            column
        });
        self.content.move_to(cursor);
        self.highlight_text = self.state.buffer.text().into();
        self.gutter.update(self.content.line_count());
        self.longest_line = longest_line(self.state.buffer.text());
    }

    fn recompute_find(&mut self, reveal: bool) -> Task<EditorMsg> {
        let result = crate::editor_find::compute(
            self.state.buffer.text(),
            &self.find_query,
            self.find_case,
            self.find_word,
            self.find_regex,
        );
        self.find_invalid = result.invalid_regex;
        self.find_matches = result.matches;
        self.find_index = if self.find_matches.is_empty() {
            None
        } else if reveal {
            let caret = self.content.cursor().position;
            let caret_offset = self
                .state
                .buffer
                .text()
                .split('\n')
                .take(caret.line)
                .map(|line| line.len() + 1)
                .sum::<usize>()
                + caret.column;
            Some(
                self.find_matches
                    .iter()
                    .position(|range| range.start >= caret_offset)
                    .unwrap_or(0),
            )
        } else {
            Some(
                self.find_index
                    .unwrap_or(0)
                    .min(self.find_matches.len().saturating_sub(1)),
            )
        };
        if self.find_open && reveal {
            self.reveal_find()
        } else {
            Task::none()
        }
    }

    fn move_find(&mut self, delta: isize) -> Task<EditorMsg> {
        if self.find_matches.is_empty() {
            return Task::none();
        }
        let current = self.find_index.unwrap_or(0) as isize;
        let count = self.find_matches.len() as isize;
        self.find_index = Some((current + delta).rem_euclid(count) as usize);
        self.reveal_find()
    }

    fn reveal_find(&mut self) -> Task<EditorMsg> {
        let Some(found) = self
            .find_index
            .and_then(|index| self.find_matches.get(index))
            .copied()
        else {
            return Task::none();
        };
        let before = self
            .state
            .buffer
            .text()
            .get(..found.start)
            .unwrap_or_default();
        let line = before.bytes().filter(|byte| *byte == b'\n').count();
        let column = before.rsplit('\n').next().map_or(0, str::len);
        let end_before = self
            .state
            .buffer
            .text()
            .get(..found.end)
            .unwrap_or_default();
        let end_line = end_before.bytes().filter(|byte| *byte == b'\n').count();
        let end_column = end_before.rsplit('\n').next().map_or(0, str::len);
        self.content.move_to(Cursor {
            position: Position { line, column },
            selection: Some(Position {
                line: end_line,
                column: end_column,
            }),
        });
        let y =
            crate::editor_scroll::reveal_offset(line, theme::SIZE_MONO * theme::LINE_HEIGHT_MONO);
        iced::widget::operation::scroll_to(
            self.scroll_id.clone(),
            iced::widget::scrollable::AbsoluteOffset {
                x: Some(0.0),
                y: Some(y),
            },
        )
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
