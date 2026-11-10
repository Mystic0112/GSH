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
}
impl Settings {
    pub fn new(extension: String, text: Arc<str>) -> Self {
        Self { extension, text }
    }
}
pub struct SyntaxHighlighter {
    lines: Vec<Vec<gsh_editor::highlight::Span>>,
    line: usize,
}
impl Highlighter for SyntaxHighlighter {
    type Settings = Settings;
    type Highlight = gsh_editor::highlight::Token;
    type Iterator<'a> = std::vec::IntoIter<(Range<usize>, Self::Highlight)>;
    fn new(settings: &Settings) -> Self {
        Self {
            lines: highlight(settings),
            line: 0,
        }
    }
    fn update(&mut self, settings: &Settings) {
        self.lines = highlight(settings);
        self.line = 0;
    }
    fn change_line(&mut self, line: usize) {
        self.line = line;
    }
    fn highlight_line(&mut self, line: &str) -> Self::Iterator<'_> {
        let spans = self.lines.get(self.line).into_iter().flatten();
        self.line += 1;
        spans
            .filter_map(|span| {
                let end = span.range.end.min(line.len());
                (span.range.start < end).then_some((span.range.start..end, span.token))
            })
            .collect::<Vec<_>>()
            .into_iter()
    }
    fn current_line(&self) -> usize {
        self.line
    }
}
fn highlight(settings: &Settings) -> Vec<Vec<gsh_editor::highlight::Span>> {
    if settings.text.len() > MAX_HIGHLIGHT_BYTES {
        Vec::new()
    } else {
        gsh_editor::highlight::highlight(&settings.text, &settings.extension)
    }
}
pub fn format(color: &gsh_editor::highlight::Token, _: &Theme) -> Format<Font> {
    Format {
        color: Some(crate::theme::syntax_color(*color)),
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
        assert_eq!(actual.first().unwrap().1, expected[1][0].token);
    }
}
