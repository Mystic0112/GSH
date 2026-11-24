//! Correspondência da busca no arquivo (Ctrl+F), usada pelo editor iced.

use fancy_regex::RegexBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchRange {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindResult {
    pub matches: Vec<MatchRange>,
    pub invalid_regex: bool,
}

pub fn compute(
    text: &str,
    query: &str,
    case_sensitive: bool,
    whole_word: bool,
    regex: bool,
) -> FindResult {
    if query.is_empty() {
        return FindResult {
            matches: Vec::new(),
            invalid_regex: false,
        };
    }
    let source = if regex {
        query.to_owned()
    } else {
        fancy_regex::escape(query).into_owned()
    };
    let source = if whole_word {
        format!(r"\b(?:{source})\b")
    } else {
        source
    };
    let Ok(pattern) = RegexBuilder::new(&source)
        .case_insensitive(!case_sensitive)
        .multi_line(true)
        .build()
    else {
        return FindResult {
            matches: Vec::new(),
            invalid_regex: true,
        };
    };
    let matches = pattern
        .find_iter(text)
        .filter_map(Result::ok)
        .filter(|found| found.end() > found.start())
        .map(|found| MatchRange {
            start: found.start(),
            end: found.end(),
        })
        .collect();
    FindResult {
        matches,
        invalid_regex: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn literal_case_word_and_regex_matches_use_utf8_byte_offsets() {
        let text = "ação ACao açãozinha";
        assert_eq!(
            compute(text, "ação", false, false, false).matches,
            vec![
                MatchRange { start: 0, end: 6 },
                MatchRange { start: 12, end: 18 }
            ]
        );
        assert!(compute(text, "Ação", true, false, false).matches.is_empty());
        assert_eq!(
            compute("foo food", "foo", false, true, false).matches.len(),
            1
        );
        assert_eq!(
            compute("a1 a2", r"a\d", false, false, true).matches.len(),
            2
        );
    }

    #[test]
    fn invalid_and_zero_width_regexes_are_safe() {
        assert!(compute("abc", "[", false, false, true).invalid_regex);
        assert!(compute("abc", "(?=a)", false, false, true)
            .matches
            .is_empty());
    }
}
