use serde::Deserialize;
use std::{io, ops::Range};

// The file editor opens files up to 2 MiB. Allow some room for unsaved edits.
pub(super) const MAX_DOCUMENT: usize = 4 * 1024 * 1024;

pub(super) fn validate_size(text: &str) -> io::Result<()> {
    if text.len() > MAX_DOCUMENT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "document exceeds LSP size limit",
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

#[derive(Clone, Copy, Debug, Deserialize)]
pub struct TextRange {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Deserialize)]
pub struct Diagnostic {
    pub range: TextRange,
    pub severity: Option<u8>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub range: TextRange,
    pub new_text: String,
}

pub(super) struct Positions {
    positions: Vec<u32>,
    lines: Vec<usize>,
}

impl Positions {
    pub(super) fn new(text: &str) -> Self {
        let mut positions = Vec::new();
        let mut lines = vec![0];
        let mut chars = text.char_indices().peekable();
        while let Some((offset, ch)) = chars.next() {
            positions.push(offset as u32);
            match ch {
                '\r' | '\n' => {
                    if ch == '\r' && chars.peek().is_some_and(|(_, ch)| *ch == '\n') {
                        chars.next();
                    }
                    lines.push(positions.len());
                }
                ch if ch.len_utf16() == 2 => positions.push(u32::MAX),
                _ => {}
            }
        }
        positions.push(text.len() as u32);
        lines.push(positions.len());
        Self { positions, lines }
    }

    pub(super) fn offset(&self, position: Position) -> Option<usize> {
        let line = position.line as usize;
        let start = *self.lines.get(line)?;
        let end = *self.lines.get(line + 1)?;
        let offset = *self
            .positions
            .get(start..end)?
            .get(position.character as usize)?;
        (offset != u32::MAX).then_some(offset as usize)
    }

    pub(super) fn range(&self, range: TextRange) -> Option<Range<usize>> {
        let start = self.offset(range.start)?;
        let end = self.offset(range.end)?;
        (start <= end).then_some(start..end)
    }
}

pub(super) fn end_position(text: &str) -> (usize, usize) {
    let mut lines = 0;
    let mut start = 0;
    let mut tail = text;
    while let Some(end) = tail.find(['\r', '\n']) {
        let line_break = tail.get(end..).unwrap_or_default();
        let skip = end + if line_break.starts_with("\r\n") { 2 } else { 1 };
        start += skip;
        lines += 1;
        tail = text.get(start..).unwrap_or_default();
    }
    (lines, tail.encode_utf16().count())
}

pub fn edit_text(text: &str, edits: Vec<TextEdit>) -> io::Result<String> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid LSP edit range");
    if edits.is_empty() {
        return Ok(text.to_owned());
    }
    let positions = Positions::new(text);
    let mut replacements = edits
        .into_iter()
        .map(|edit| {
            let range = positions.range(edit.range).ok_or_else(invalid)?;
            Ok((range, edit.new_text))
        })
        .collect::<io::Result<Vec<_>>>()?;
    // The protocol permits multiple inserts at an offset in their original order.
    replacements.sort_by_key(|(range, _)| (range.start, range.end));
    let mut result = String::new();
    let mut last = 0;
    for (range, value) in replacements {
        let prefix = text.get(last..range.start).ok_or_else(invalid)?;
        if result.len() + prefix.len() + value.len() > MAX_DOCUMENT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "LSP edit exceeds size limit",
            ));
        }
        result.push_str(prefix);
        result.push_str(&value);
        last = range.end;
    }
    let tail = text.get(last..).ok_or_else(invalid)?;
    if result.len() + tail.len() > MAX_DOCUMENT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "LSP edit exceeds size limit",
        ));
    }
    result.push_str(tail);
    Ok(result)
}
