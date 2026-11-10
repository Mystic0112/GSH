//! Estado de uma aba de edição, independente da apresentação.
use gsh_editor::{Buffer, EditorError};
use std::path::PathBuf;

#[derive(Debug)]
pub struct EditorState {
    pub path: PathBuf,
    pub buffer: Buffer,
    pub notice: Option<EditorNotice>,
    close_pending: bool,
    fingerprint: Option<(std::time::SystemTime, u64)>,
}

#[derive(Debug)]
pub enum EditorNotice {
    ConfirmClose,
    ModifiedOnDisk,
    SaveError(EditorError),
}
impl EditorNotice {
    pub fn color(&self) -> iced::Color {
        match self {
            Self::SaveError(_) => crate::theme::ERROR,
            Self::ConfirmClose | Self::ModifiedOnDisk => crate::theme::WARN,
        }
    }
}

impl EditorState {
    pub fn new(path: PathBuf, text: String) -> Self {
        let fingerprint = file_fingerprint(&path).ok();
        Self {
            path,
            buffer: Buffer::new(text),
            notice: None,
            close_pending: false,
            fingerprint,
        }
    }

    /// Aplica apenas a diferença, nos limites de caracteres UTF-8.
    pub fn replace_text(&mut self, text: &str) -> Result<bool, EditorError> {
        let old = self.buffer.text();
        if old == text {
            return Ok(false);
        }
        let start = old
            .chars()
            .zip(text.chars())
            .take_while(|(a, b)| a == b)
            .map(|(ch, _)| ch.len_utf8())
            .sum::<usize>();
        let old_tail = old.get(start..).unwrap_or_default();
        let new_tail = text.get(start..).unwrap_or_default();
        let suffix = old_tail
            .chars()
            .rev()
            .zip(new_tail.chars().rev())
            .take_while(|(a, b)| a == b)
            .map(|(ch, _)| ch.len_utf8())
            .sum::<usize>();
        let end_old = old.len().saturating_sub(suffix);
        let end_new = text.len().saturating_sub(suffix);
        let replacement = text
            .get(start..end_new)
            .ok_or(EditorError::InvalidPosition)?;
        self.buffer.select(start, end_old)?;
        let changed = self.buffer.insert(replacement);
        self.close_pending = false;
        self.notice = None;
        Ok(changed)
    }
    pub fn save(&mut self) {
        self.save_inner(false);
    }
    pub fn overwrite_confirmed(&mut self) {
        if matches!(self.notice, Some(EditorNotice::ModifiedOnDisk)) {
            self.save_inner(true);
        }
    }
    fn save_inner(&mut self, overwrite: bool) {
        let result = self
            .path
            .canonicalize()
            .map_err(EditorError::Save)
            .and_then(|path| {
                let current = file_fingerprint(&path).map_err(EditorError::Save)?;
                if !overwrite && self.fingerprint != Some(current) {
                    return Err(EditorError::ModifiedOnDisk);
                }
                self.buffer.save(&path)?;
                self.fingerprint = Some(file_fingerprint(&path).map_err(EditorError::Save)?);
                Ok(())
            });
        if matches!(result, Err(EditorError::ModifiedOnDisk)) {
            self.notice = Some(EditorNotice::ModifiedOnDisk);
            return;
        }
        self.notice = result.err().map(EditorNotice::SaveError);
        self.close_pending = false;
    }
    pub fn request_close(&mut self) -> bool {
        if !self.buffer.is_dirty() || self.close_pending {
            return true;
        }
        self.close_pending = true;
        self.notice = Some(EditorNotice::ConfirmClose);
        false
    }
    pub fn cancel_close(&mut self) {
        self.close_pending = false;
        if matches!(self.notice, Some(EditorNotice::ConfirmClose)) {
            self.notice = None;
        }
    }
    pub fn undo(&mut self) {
        self.buffer.undo();
        self.close_pending = false;
        self.notice = None;
    }
    pub fn redo(&mut self) {
        self.buffer.redo();
        self.close_pending = false;
        self.notice = None;
    }
}

fn file_fingerprint(path: &std::path::Path) -> std::io::Result<(std::time::SystemTime, u64)> {
    let metadata = std::fs::metadata(path)?;
    Ok((metadata.modified()?, metadata.len()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edit_undo_redo_save_and_confirm_close() {
        let path = std::env::temp_dir().join(format!("gsh-editor-ui-{}", std::process::id()));
        std::fs::write(&path, "á\n").unwrap();
        let mut state = EditorState::new(path.clone(), "á\n".into());
        state.replace_text("áβ\n").unwrap();
        assert!(state.buffer.is_dirty());
        assert!(!state.request_close());
        state.undo();
        assert_eq!(state.buffer.text(), "á\n");
        state.redo();
        assert!(!state.request_close());
        assert!(state.request_close());
        state.save();
        assert!(state.notice.is_none());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "áβ\n");
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn save_error_does_not_clear_dirty() {
        let mut state = EditorState::new(PathBuf::from("/nonexistent-gsh-dir/file"), "x".into());
        state.replace_text("y").unwrap();
        state.save();
        assert!(matches!(state.notice, Some(EditorNotice::SaveError(_))));
        assert!(state.buffer.is_dirty());
    }
    #[test]
    fn external_change_requires_explicit_overwrite() {
        let path = std::env::temp_dir().join(format!("gsh-conflict-{}", std::process::id()));
        std::fs::write(&path, "original").unwrap();
        let mut state = EditorState::new(path.clone(), "original".into());
        assert!(state.replace_text("local").unwrap());
        std::fs::write(&path, "external change").unwrap();
        state.save();
        assert!(matches!(state.notice, Some(EditorNotice::ModifiedOnDisk)));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "external change");
        state.overwrite_confirmed();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "local");
        let _ = std::fs::remove_file(path);
    }
}
