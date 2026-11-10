//! Realce léxico por escopos, independente da paleta de interface.
use std::ops::Range;
use std::sync::OnceLock;
use syntect::parsing::{ParseState, ScopeStack, SyntaxSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
    Base,
    Comment,
    Keyword,
    String,
    Number,
    Type,
    Builtin,
    Function,
    Variable,
    Meta,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub range: Range<usize>,
    pub token: Token,
}

fn token(stack: &ScopeStack) -> Token {
    for scope in stack.as_slice().iter().rev() {
        let name = scope.build_string();
        if name.starts_with("comment") {
            return Token::Comment;
        }
        if name.starts_with("string") {
            return Token::String;
        }
        if name.starts_with("constant.numeric") || name.starts_with("constant.language") {
            return Token::Number;
        }
        if name.starts_with("support.function") || name.starts_with("support.type") {
            return Token::Builtin;
        }
        if name.starts_with("entity.name.function") {
            return Token::Function;
        }
        if name.starts_with("entity.name.type") {
            return Token::Type;
        }
        if name.starts_with("keyword") || name.starts_with("storage") {
            return Token::Keyword;
        }
        if name.starts_with("variable") || name.starts_with("entity.name.tag") {
            return Token::Variable;
        }
        if name.starts_with("meta.preprocessor") {
            return Token::Meta;
        }
    }
    Token::Base
}

pub fn highlight(text: &str, extension: &str) -> Vec<Vec<Span>> {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
    let syntax = syntaxes
        .find_syntax_by_extension(extension)
        .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
    let mut parser = ParseState::new(syntax);
    let mut scopes = ScopeStack::new();
    text.split_inclusive('\n')
        .map(|line| {
            let Ok(operations) = parser.parse_line(line, syntaxes) else {
                return Vec::new();
            };
            let mut spans = Vec::new();
            let mut start = 0;
            for (end, operation) in operations {
                if end > start && end <= line.len() {
                    spans.push(Span {
                        range: start..end,
                        token: token(&scopes),
                    });
                }
                if scopes.apply(&operation).is_err() {
                    return Vec::new();
                }
                start = end;
            }
            if start < line.len() {
                spans.push(Span {
                    range: start..line.len(),
                    token: token(&scopes),
                });
            }
            spans
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rust_keyword_and_string_have_distinct_tokens() {
        let line = "let greeting = \"hello\";\n";
        let lines = highlight(line, "rs");
        let spans = &lines[0];
        assert_eq!(
            spans
                .iter()
                .find(|s| &line[s.range.clone()] == "let")
                .unwrap()
                .token,
            Token::Keyword
        );
        assert_eq!(
            spans
                .iter()
                .find(|s| line[s.range.clone()].contains("hello"))
                .unwrap()
                .token,
            Token::String
        );
    }
    #[test]
    fn unknown_language_is_plain_text() {
        assert!(highlight("let x = \"text\";\n", "unknown-extension")[0]
            .iter()
            .all(|span| span.token == Token::Base));
    }
}
