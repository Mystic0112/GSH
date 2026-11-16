//! Estado da aba Busca (`_ContentSearchPanelState` de `content_search_panel.dart`), sem UI.
//!
//! O campo, os toggles e os resultados vivem aqui; a thread de varredura fica em `main.rs`.
//! Cada busca tem uma geração: evento de uma busca antiga (cancelada ou substituída) é descartado.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use gsh_search::{ContentQuery, FileMatches, SearchError};

/// Espera depois da última tecla antes de buscar (toggle busca na hora).
pub const DEBOUNCE_MS: u64 = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    CaseSensitive,
    WholeWord,
    Regex,
}

/// Uma busca pronta para rodar fora da thread da UI.
#[derive(Debug)]
pub struct Launch {
    pub generation: u64,
    pub query: ContentQuery,
    pub cancel: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
pub struct SearchState {
    pub term: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub results: Vec<FileMatches>,
    pub collapsed: HashSet<String>,
    pub searching: bool,
    pub invalid_regex: bool,
    /// Fase do spinner, em ms.
    pub spin_ms: u64,
    generation: u64,
    debounce: u64,
    cancel: Option<Arc<AtomicBool>>,
}

impl SearchState {
    /// Texto digitado: devolve o token do debounce; só o último token dispara a busca.
    pub fn type_term(&mut self, term: String) -> u64 {
        self.term = term;
        self.debounce = self.debounce.wrapping_add(1);
        self.debounce
    }

    pub fn debounce_due(&self, token: u64) -> bool {
        token == self.debounce
    }

    pub fn toggle(&mut self, toggle: Toggle) {
        let flag = match toggle {
            Toggle::CaseSensitive => &mut self.case_sensitive,
            Toggle::WholeWord => &mut self.whole_word,
            Toggle::Regex => &mut self.regex,
        };
        *flag = !*flag;
    }

    pub fn toggle_group(&mut self, relative_path: String) {
        if !self.collapsed.remove(&relative_path) {
            self.collapsed.insert(relative_path);
        }
    }

    pub fn has_query(&self) -> bool {
        !self.term.trim().is_empty()
    }

    /// Soma dos matches de todos os arquivos recebidos (o "N" da pílula).
    pub fn total_matches(&self) -> usize {
        self.results.iter().map(FileMatches::match_count).sum()
    }

    /// `_runSearch`: cancela a anterior, limpa e, havendo termo e pasta, prepara a nova.
    pub fn begin(&mut self, root: Option<PathBuf>) -> Option<Launch> {
        self.cancel_running();
        self.results.clear();
        self.collapsed.clear();
        self.invalid_regex = false;
        self.searching = false;
        if !self.has_query() {
            return None;
        }
        let root = root?;
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel = Some(Arc::clone(&cancel));
        self.searching = true;
        self.spin_ms = 0;
        Some(Launch {
            generation: self.generation,
            query: ContentQuery {
                root,
                term: self.term.clone(),
                case_sensitive: self.case_sensitive,
                whole_word: self.whole_word,
                regex: self.regex,
            },
            cancel,
        })
    }

    pub fn apply_batch(&mut self, generation: u64, batch: Vec<FileMatches>) {
        if generation == self.generation {
            self.results.extend(batch);
        }
    }

    /// Fim da varredura; regex inválida só marca erro quando o toggle de regex está ligado.
    pub fn finish(&mut self, generation: u64, outcome: Result<(), SearchError>) {
        if generation != self.generation {
            return;
        }
        if outcome.is_err() {
            self.invalid_regex = self.regex;
        }
        self.searching = false;
        self.cancel = None;
    }

    /// Sai da aba: o Flutter desmonta o painel, então campo, toggles e resultados somem.
    pub fn reset(&mut self) {
        self.cancel_running();
        let (generation, debounce) = (self.generation, self.debounce);
        *self = Self::default();
        self.generation = generation.wrapping_add(1);
        self.debounce = debounce.wrapping_add(1);
    }

    fn cancel_running(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.generation = self.generation.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_search::{LineMatch, MatchRange};

    fn file(path: &str, hits: usize) -> FileMatches {
        FileMatches {
            relative_path: path.to_owned(),
            matches: vec![LineMatch {
                line_number: 1,
                text: "x".repeat(hits),
                ranges: (0..hits)
                    .map(|i| MatchRange {
                        start: i,
                        end: i + 1,
                    })
                    .collect(),
            }],
        }
    }

    fn typed(term: &str) -> SearchState {
        let mut state = SearchState::default();
        state.type_term(term.to_owned());
        state
    }

    #[test]
    fn only_the_last_keystroke_fires_the_debounce() {
        let mut state = SearchState::default();
        let first = state.type_term("a".into());
        let second = state.type_term("ab".into());
        assert!(!state.debounce_due(first));
        assert!(state.debounce_due(second));
    }

    #[test]
    fn blank_term_or_missing_root_launches_nothing_and_clears() {
        let mut state = typed("  ");
        state.results.push(file("a.rs", 1));
        state.collapsed.insert("a.rs".into());
        state.invalid_regex = true;
        assert!(state.begin(Some(PathBuf::from("/x"))).is_none());
        assert!(state.results.is_empty() && state.collapsed.is_empty());
        assert!(!state.searching && !state.invalid_regex);

        let mut state = typed("needle");
        assert!(state.begin(None).is_none());
        assert!(!state.searching);
    }

    #[test]
    fn begin_cancels_the_previous_search_and_drops_its_events() {
        let mut state = typed("needle");
        let first = state.begin(Some(PathBuf::from("/x"))).unwrap();
        assert!(state.searching);
        let second = state.begin(Some(PathBuf::from("/x"))).unwrap();
        assert!(first.cancel.load(Ordering::Relaxed));
        assert!(!second.cancel.load(Ordering::Relaxed));
        assert_ne!(first.generation, second.generation);

        state.apply_batch(first.generation, vec![file("old.rs", 1)]);
        state.finish(first.generation, Ok(()));
        assert!(state.results.is_empty());
        assert!(state.searching);

        state.apply_batch(second.generation, vec![file("new.rs", 2)]);
        state.apply_batch(second.generation, vec![file("new2.rs", 1)]);
        assert_eq!(state.results.len(), 2);
        assert_eq!(state.total_matches(), 3);
        state.finish(second.generation, Ok(()));
        assert!(!state.searching);
    }

    #[test]
    fn query_carries_term_and_toggles() {
        let mut state = typed("a.b");
        state.toggle(Toggle::CaseSensitive);
        state.toggle(Toggle::Regex);
        state.toggle(Toggle::WholeWord);
        state.toggle(Toggle::WholeWord);
        let launch = state.begin(Some(PathBuf::from("/root"))).unwrap();
        assert_eq!(launch.query.term, "a.b");
        assert!(launch.query.case_sensitive && launch.query.regex && !launch.query.whole_word);
        assert_eq!(launch.query.root, PathBuf::from("/root"));
    }

    #[test]
    fn invalid_pattern_only_flags_when_regex_toggle_is_on() {
        let error = || Err(SearchError::InvalidPattern("bad".into()));
        let mut state = typed("[");
        state.toggle(Toggle::Regex);
        let launch = state.begin(Some(PathBuf::from("/x"))).unwrap();
        state.finish(launch.generation, error());
        assert!(state.invalid_regex && !state.searching);

        let mut state = typed("[");
        let launch = state.begin(Some(PathBuf::from("/x"))).unwrap();
        state.finish(launch.generation, error());
        assert!(!state.invalid_regex && !state.searching);
    }

    #[test]
    fn groups_collapse_and_expand() {
        let mut state = SearchState::default();
        state.toggle_group("a.rs".into());
        assert!(state.collapsed.contains("a.rs"));
        state.toggle_group("a.rs".into());
        assert!(state.collapsed.is_empty());
    }

    #[test]
    fn reset_cancels_and_forgets_everything_but_stays_stale_proof() {
        let mut state = typed("needle");
        state.toggle(Toggle::Regex);
        let launch = state.begin(Some(PathBuf::from("/x"))).unwrap();
        let token = state.type_term("needle2".into());
        state.reset();
        assert!(launch.cancel.load(Ordering::Relaxed));
        assert!(state.term.is_empty() && !state.regex && !state.searching);
        assert!(!state.debounce_due(token));
        state.apply_batch(launch.generation, vec![file("a.rs", 1)]);
        assert!(state.results.is_empty());
    }
}
