//! Buffer de texto independente da interface.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

use std::io::Write;
use std::ops::Range;
use std::path::Path;

pub mod highlight;
#[cfg(target_os = "linux")]
pub mod lsp;

const HISTORY_LIMIT: usize = 500;

#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error("invalid position")]
    InvalidPosition,
    #[error("save failed: {0}")]
    Save(#[from] std::io::Error),
    #[error("file is read-only")]
    ReadOnly,
    #[error("file changed on disk")]
    ModifiedOnDisk,
}

#[derive(Debug, Clone)]
struct Edit {
    start: usize,
    old: String,
    new: String,
    before: u64,
    after: u64,
    cursor_before: usize,
    anchor_before: usize,
    cursor_after: usize,
}

#[derive(Debug, Clone)]
pub struct Buffer {
    text: String,
    cursor: usize,
    anchor: usize,
    revision: u64,
    saved_revision: u64,
    next_revision: u64,
    undo: Vec<Edit>,
    redo: Vec<Edit>,
    group_open: bool,
}
impl Buffer {
    pub fn new(text: String) -> Self {
        Self {
            text,
            cursor: 0,
            anchor: 0,
            revision: 0,
            saved_revision: 0,
            next_revision: 1,
            undo: Vec::new(),
            redo: Vec::new(),
            group_open: false,
        }
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    pub fn cursor(&self) -> usize {
        self.cursor
    }
    pub fn selection(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }
    pub fn history_marker(&self) -> Option<u64> {
        self.undo.last().map(|edit| edit.before)
    }
    pub fn break_group(&mut self) {
        self.group_open = false;
    }
    pub fn select(&mut self, anchor: usize, cursor: usize) -> Result<(), EditorError> {
        if !self.text.is_char_boundary(anchor) || !self.text.is_char_boundary(cursor) {
            return Err(EditorError::InvalidPosition);
        }
        self.anchor = anchor;
        self.cursor = cursor;
        Ok(())
    }
    pub fn insert(&mut self, value: &str) -> bool {
        let range = self.selection();
        let Some(old) = self.text.get(range.clone()) else {
            return false;
        };
        if old == value {
            return false;
        }
        let Some(after) = self.next_revision.checked_add(1) else {
            return false;
        };
        let single_character = value.chars().count() == 1 && value.chars().all(|c| !c.is_control());
        let can_group = self.group_open
            && single_character
            && old.is_empty()
            && self.revision != self.saved_revision
            && self
                .undo
                .last()
                .is_some_and(|last| last.old.is_empty() && last.cursor_after == range.start);
        if can_group {
            self.text.replace_range(range.clone(), value);
            if let Some(last) = self.undo.last_mut() {
                last.new.push_str(value);
                last.cursor_after = range.start + value.len();
                last.after = self.next_revision;
            }
            self.cursor = range.start + value.len();
            self.anchor = self.cursor;
            self.revision = self.next_revision;
            self.next_revision = after;
            self.redo.clear();
            return true;
        }
        let edit = Edit {
            start: range.start,
            old: old.to_owned(),
            new: value.to_owned(),
            before: self.revision,
            after: self.next_revision,
            cursor_before: self.cursor,
            anchor_before: self.anchor,
            cursor_after: range.start + value.len(),
        };
        self.next_revision = after;
        self.text.replace_range(range, value);
        self.cursor = edit.cursor_after;
        self.anchor = self.cursor;
        self.revision = edit.after;
        self.undo.push(edit);
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.group_open = single_character && edit_is_insertion(value, &self.undo);
        true
    }
    pub fn undo(&mut self) {
        self.group_open = false;
        if let Some(edit) = self.undo.pop() {
            let end = edit.start + edit.new.len();
            self.text.replace_range(edit.start..end, &edit.old);
            self.cursor = edit.cursor_before;
            self.anchor = edit.anchor_before;
            self.revision = edit.before;
            self.redo.push(edit);
        }
    }
    pub fn redo(&mut self) {
        self.group_open = false;
        if let Some(edit) = self.redo.pop() {
            let end = edit.start + edit.old.len();
            self.text.replace_range(edit.start..end, &edit.new);
            self.cursor = edit.cursor_after;
            self.anchor = self.cursor;
            self.revision = edit.after;
            self.undo.push(edit);
        }
    }
    pub fn save(&mut self, path: &Path) -> Result<(), EditorError> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let metadata = std::fs::metadata(path)?;
        if metadata.permissions().readonly() {
            return Err(EditorError::ReadOnly);
        }
        let mut temp = tempfile::NamedTempFile::new_in(parent)?;
        temp.as_file().set_permissions(metadata.permissions())?;
        temp.write_all(self.text.as_bytes())?;
        temp.as_file().sync_all()?;
        temp.persist(path).map_err(|e| e.error)?;
        std::fs::File::open(parent)?.sync_all()?;
        self.saved_revision = self.revision;
        self.group_open = false;
        Ok(())
    }
}

fn edit_is_insertion(value: &str, undo: &[Edit]) -> bool {
    !value.is_empty() && undo.last().is_some_and(|edit| edit.old.is_empty())
}

#[cfg(test)]
mod tests;
