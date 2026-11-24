//! Adaptador do realce léxico para o widget iced.
use iced::advanced::text::highlighter::{Format, Highlighter};
use iced::{Font, Theme};
use std::ops::Range;
use std::sync::Arc;
const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;

#[derive(Clone, PartialEq)]
pub struct Settings {
    extension: String,
    text: Arc<str>,
    search: Vec<Range<usize>>,
    current: Option<Range<usize>>,
}
impl Settings {
    pub fn new(extension: String, text: Arc<str>) -> Self {
        Self {
            extension,
            text,
            search: Vec::new(),
            current: None,
        }
    }
    pub fn new_with_search(
        extension: String,
        text: Arc<str>,
        search: &[crate::editor_find::MatchRange],
        current: Option<usize>,
    ) -> Self {
        let current_range = current
            .and_then(|index| search.get(index))
            .map(|range| range.start..range.end);
        Self {
            extension,
            text,
            search: search.iter().map(|range| range.start..range.end).collect(),
            current: current_range,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorHighlight {
    Syntax(gsh_editor::highlight::Token),
    Search,
    CurrentSearch,
}
pub struct SyntaxHighlighter {
    lines: Vec<Vec<gsh_editor::highlight::Span>>,
    search_lines: Vec<Vec<(Range<usize>, bool)>>,
    line: usize,
}
impl Highlighter for SyntaxHighlighter {
    type Settings = Settings;
    type Highlight = EditorHighlight;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Self::Highlight)>;
    fn new(settings: &Settings) -> Self {
        Self {
            lines: highlight(settings),
            search_lines: search_lines(settings),
            line: 0,
        }
    }
    fn update(&mut self, settings: &Settings) {
        self.lines = highlight(settings);
        self.search_lines = search_lines(settings);
        self.line = 0;
    }
    fn change_line(&mut self, line: usize) {
        self.line = line;
    }
    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let line_index = self.line;
        let spans = self.lines.get(line_index).into_iter().flatten();
        self.line += 1;
        let matches = self
            .search_lines
            .get(line_index)
            .cloned()
            .unwrap_or_default();
        let mut output = Vec::new();
        for span in spans {
            let end = span.range.end.min(line.len());
            if span.range.start >= end {
                continue;
            }
            let mut pieces = std::iter::once(span.range.start..end).collect::<Vec<_>>();
            for (match_range, _) in &matches {
                pieces = pieces
                    .into_iter()
                    .flat_map(|piece| {
                        if match_range.start >= piece.end || match_range.end <= piece.start {
                            return vec![piece];
                        }
                        let mut outside = Vec::new();
                        if piece.start < match_range.start {
                            outside.push(piece.start..match_range.start);
                        }
                        if match_range.end < piece.end {
                            outside.push(match_range.end..piece.end);
                        }
                        outside
                    })
                    .collect();
            }
            output.extend(
                pieces
                    .into_iter()
                    .map(|range| (range, EditorHighlight::Syntax(span.token))),
            );
        }
        output.extend(matches.iter().filter_map(|(range, current)| {
            let end = range.end.min(line.len());
            (range.start < end).then_some((
                range.start..end,
                if *current {
                    EditorHighlight::CurrentSearch
                } else {
                    EditorHighlight::Search
                },
            ))
        }));
        output.sort_by_key(|(range, _)| range.start);
        output.into_iter()
    }
    fn current_line(&self) -> usize {
        self.line
    }
}
fn search_lines(settings: &Settings) -> Vec<Vec<(Range<usize>, bool)>> {
    let mut lines = Vec::new();
    let mut offset = 0_usize;
    for line in settings.text.split('\n') {
        let end = offset.saturating_add(line.len());
        let mut found = Vec::new();
        for range in &settings.search {
            let start = range.start.max(offset);
            let finish = range.end.min(end);
            if start < finish {
                let current = settings
                    .current
                    .as_ref()
                    .is_some_and(|active| active.start < finish && active.end > start);
                found.push((start - offset..finish - offset, current));
            }
        }
        lines.push(found);
        offset = end.saturating_add(1);
    }
    lines
}
fn highlight(settings: &Settings) -> Vec<Vec<gsh_editor::highlight::Span>> {
    if settings.text.len() > MAX_HIGHLIGHT_BYTES {
        Vec::new()
    } else {
        gsh_editor::highlight::highlight(&settings.text, &settings.extension)
    }
}
pub fn format(color: &EditorHighlight, _: &Theme) -> Format<Font> {
    Format {
        color: Some(match color {
            EditorHighlight::Syntax(token) => crate::theme::syntax_color(*token),
            EditorHighlight::Search => crate::theme::palette().accent,
            EditorHighlight::CurrentSearch => crate::theme::palette().warn,
        }),
        font: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rehighlight_keeps_multiline_context() {
        let text = "let x = \"first\nsecond\";\n";
        let expected = gsh_editor::highlight::highlight(text, "rs");
        let mut highlighter = SyntaxHighlighter::new(&Settings::new("rs".into(), text.into()));
        highlighter.change_line(1);
        let actual: Vec<_> = highlighter.highlight_line("second\";").collect();
        assert_eq!(
            actual.first().unwrap().1,
            EditorHighlight::Syntax(expected[1][0].token)
        );
    }

    #[test]
    fn highlights_each_search_match_and_marks_the_current_one() {
        let text: Arc<str> = "needle\nneedle".into();
        let ranges = [
            crate::editor_find::MatchRange { start: 0, end: 6 },
            crate::editor_find::MatchRange { start: 7, end: 13 },
        ];
        let settings = Settings::new_with_search("rs".into(), text, &ranges, Some(1));
        let mut highlighter = SyntaxHighlighter::new(&settings);
        let first = highlighter.highlight_line("needle").collect::<Vec<_>>();
        let second = highlighter.highlight_line("needle").collect::<Vec<_>>();
        assert!(first
            .iter()
            .any(|(_, kind)| *kind == EditorHighlight::Search));
        assert!(second
            .iter()
            .any(|(_, kind)| *kind == EditorHighlight::CurrentSearch));
    }
}
