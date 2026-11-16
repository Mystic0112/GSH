//! Busca por conteudo no workspace (find-in-files), sem UI.
//!
//! Porta de `content_searcher_impl.dart` + `content_search.dart` do Flutter: varredura com
//! pilha que pula pastas ocultas e pesadas, mesmos limites, binario = byte nulo nos primeiros
//! 8000 bytes, UTF-8 tolerante e resultados vazios ignorados. Os resultados saem em lotes
//! (um `Vec<FileMatches>` por chamada de `emit`) para a UI nao ser inundada de mensagens.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use fancy_regex::{Regex, RegexBuilder};

/// Pastas ruidosas puladas no walk (alem de qualquer pasta que comece com ".").
const IGNORED_DIRS: [&str; 15] = [
    "node_modules",
    "build",
    ".dart_tool",
    ".next",
    "dist",
    "out",
    "Pods",
    "DerivedData",
    ".gradle",
    ".venv",
    "venv",
    "__pycache__",
    "target",
    "vendor",
    "coverage",
];

pub const MAX_FILES_WITH_MATCHES: usize = 2000;
pub const MAX_TOTAL_MATCHES: usize = 10_000;
pub const MAX_MATCHES_PER_FILE: usize = 200;
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_LINE_CHARS: usize = 1000;
const BINARY_PROBE_BYTES: usize = 8000;
/// `\b` do `RegExp` do Dart: so `[A-Za-z0-9_]` conta como caractere de palavra.
const ASCII_WORD_BOUNDARY: &str =
    "(?:(?<=[A-Za-z0-9_])(?![A-Za-z0-9_])|(?<![A-Za-z0-9_])(?=[A-Za-z0-9_]))";

/// Um lote sai quando junta tantos arquivos ou passado este tempo desde o ultimo.
const BATCH_FILES: usize = 20;
const BATCH_INTERVAL: Duration = Duration::from_millis(50);

/// Parametros de uma busca (`ContentQuery`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentQuery {
    pub root: PathBuf,
    pub term: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
}

/// Intervalo `[start, end)` de um match, em **bytes** da linha original (`LineMatch::text`
/// pode ser a linha truncada; quem desenha limita o intervalo ao tamanho do texto).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchRange {
    pub start: usize,
    pub end: usize,
}

/// Linha com pelo menos um match; `line_number` e base 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineMatch {
    pub line_number: usize,
    pub text: String,
    pub ranges: Vec<MatchRange>,
}

/// Resultados de um arquivo; `relative_path` usa sempre "/".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileMatches {
    pub relative_path: String,
    pub matches: Vec<LineMatch>,
}

impl FileMatches {
    /// Total de matches no arquivo (soma dos intervalos de todas as linhas).
    pub fn match_count(&self) -> usize {
        self.matches.iter().map(|line| line.ranges.len()).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SearchError {
    #[error("invalid search pattern: {0}")]
    InvalidPattern(String),
}

/// Varre `query.root` e entrega os resultados em lotes por `emit`. Termo vazio ou raiz que
/// nao e pasta: termina sem emitir nada. `cancel` ligado interrompe a varredura.
pub fn search(
    query: &ContentQuery,
    cancel: &AtomicBool,
    emit: impl FnMut(Vec<FileMatches>),
) -> Result<(), SearchError> {
    if query.term.trim().is_empty() {
        return Ok(());
    }
    let pattern = build_pattern(query)?;
    if !query.root.is_dir() {
        return Ok(());
    }

    let mut batch = Batch::new(emit);
    let mut files_with_matches = 0usize;
    let mut total_matches = 0usize;
    let mut stack = vec![query.root.clone()];

    'walk: while let Some(dir) = stack.pop() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue; // sem permissao etc.
        };
        for entry in entries.flatten() {
            if cancel.load(Ordering::Relaxed) {
                break 'walk;
            }
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name().to_string_lossy().into_owned();
            if kind.is_dir() {
                if name.starts_with('.') || IGNORED_DIRS.contains(&name.as_str()) {
                    continue;
                }
                stack.push(entry.path());
            } else if kind.is_file() {
                if name == ".DS_Store" {
                    continue;
                }
                let path = entry.path();
                let Some(matches) = scan_file(&path, &pattern) else {
                    continue;
                };
                if matches.is_empty() {
                    continue;
                }
                let file = FileMatches {
                    relative_path: relative_path(&query.root, &path),
                    matches,
                };
                files_with_matches += 1;
                total_matches += file.match_count();
                batch.push(file);
                if files_with_matches >= MAX_FILES_WITH_MATCHES
                    || total_matches >= MAX_TOTAL_MATCHES
                {
                    break 'walk;
                }
            }
        }
    }
    batch.flush();
    Ok(())
}

/// Acumula arquivos e os entrega em lotes.
struct Batch<F: FnMut(Vec<FileMatches>)> {
    emit: F,
    files: Vec<FileMatches>,
    last_flush: Instant,
}

impl<F: FnMut(Vec<FileMatches>)> Batch<F> {
    fn new(emit: F) -> Self {
        Self {
            emit,
            files: Vec::new(),
            last_flush: Instant::now(),
        }
    }

    fn push(&mut self, file: FileMatches) {
        self.files.push(file);
        if self.files.len() >= BATCH_FILES || self.last_flush.elapsed() >= BATCH_INTERVAL {
            self.flush();
        }
    }

    fn flush(&mut self) {
        self.last_flush = Instant::now();
        if !self.files.is_empty() {
            (self.emit)(std::mem::take(&mut self.files));
        }
    }
}

/// Nao-regex e escapado; `whole_word` envolve em `\b(?:...)\b` (word boundary ASCII, como o
/// `RegExp` do Dart). O fancy-regex nao aceita `(?-u:\b)`, entao o `\b` ASCII e escrito com
/// lookaround.
fn build_pattern(query: &ContentQuery) -> Result<Regex, SearchError> {
    let mut source = if query.regex {
        query.term.clone()
    } else {
        fancy_regex::escape(&query.term).into_owned()
    };
    if query.whole_word {
        source = format!("{ASCII_WORD_BOUNDARY}(?:{source}){ASCII_WORD_BOUNDARY}");
    }
    RegexBuilder::new(&source)
        .case_insensitive(!query.case_sensitive)
        .build()
        .map_err(|error| SearchError::InvalidPattern(error.to_string()))
}

/// Linhas com match, ou `None` se o arquivo deve ser pulado (grande, ilegivel ou binario).
fn scan_file(path: &Path, pattern: &Regex) -> Option<Vec<LineMatch>> {
    if fs::metadata(path).ok()?.len() > MAX_FILE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let probe = bytes.get(..BINARY_PROBE_BYTES.min(bytes.len()))?;
    if probe.contains(&0) {
        return None;
    }
    let content = String::from_utf8_lossy(&bytes);

    let mut out = Vec::new();
    for (index, line) in split_lines(&content).into_iter().enumerate() {
        let (ranges, failed) = line_ranges(pattern, line);
        if !ranges.is_empty() {
            out.push(LineMatch {
                line_number: index + 1,
                text: truncate_line(line),
                ranges,
            });
        }
        if failed || out.len() >= MAX_MATCHES_PER_FILE {
            break;
        }
    }
    Some(out)
}

/// Intervalos casados na linha, ignorando matches vazios (ex.: regex `a*`). O `bool` avisa
/// que a execucao falhou (limite de backtracking do fancy-regex): a linha fica com o que ja
/// casou e o arquivo para de ser lido.
fn line_ranges(pattern: &Regex, line: &str) -> (Vec<MatchRange>, bool) {
    let mut ranges = Vec::new();
    for found in pattern.find_iter(line) {
        match found {
            Ok(found) if found.end() > found.start() => ranges.push(MatchRange {
                start: found.start(),
                end: found.end(),
            }),
            Ok(_) => {}
            Err(_) => return (ranges, true),
        }
    }
    (ranges, false)
}

/// Como o `LineSplitter` do Dart: quebra em `\n`, `\r\n` e `\r`; sem linha vazia no fim.
fn split_lines(content: &str) -> Vec<&str> {
    let bytes = content.as_bytes();
    let mut lines = Vec::new();
    let (mut start, mut i) = (0, 0);
    while let Some(&byte) = bytes.get(i) {
        if byte == b'\n' || byte == b'\r' {
            if let Some(line) = content.get(start..i) {
                lines.push(line);
            }
            i += 1;
            if byte == b'\r' && bytes.get(i) == Some(&b'\n') {
                i += 1;
            }
            start = i;
        } else {
            i += 1;
        }
    }
    if let Some(rest) = content.get(start..).filter(|rest| !rest.is_empty()) {
        lines.push(rest);
    }
    lines
}

fn truncate_line(line: &str) -> String {
    match line.char_indices().nth(MAX_LINE_CHARS) {
        Some((end, _)) => line.get(..end).unwrap_or(line).to_owned(),
        None => line.to_owned(),
    }
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    struct Fixture(tempfile::TempDir);

    impl Fixture {
        fn new() -> Self {
            Self(tempfile::tempdir().unwrap())
        }

        fn write(&self, rel: &str, content: impl AsRef<[u8]>) {
            let path = self.0.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }

        fn query(&self, term: &str) -> ContentQuery {
            ContentQuery {
                root: self.0.path().to_path_buf(),
                term: term.to_owned(),
                case_sensitive: false,
                whole_word: false,
                regex: false,
            }
        }
    }

    fn run(query: &ContentQuery) -> Result<Vec<FileMatches>, SearchError> {
        let mut all = Vec::new();
        search(query, &AtomicBool::new(false), |batch| all.extend(batch))?;
        Ok(all)
    }

    #[test]
    fn finds_matches_with_line_range_and_relative_path() {
        let fx = Fixture::new();
        fx.write(
            "app/auth/session.ts",
            "import { Clock } from \"x\";\nclass SessionStore {\n  now() { return this.clock.now(); }\n}\n",
        );
        let results = run(&fx.query("clock")).unwrap();
        assert_eq!(results.len(), 1);
        let file = results.first().unwrap();
        assert_eq!(file.relative_path, "app/auth/session.ts");
        // "Clock" (linha 1) + "clock" (linha 3) = 2 matches, sem diferenciar caixa.
        assert_eq!(file.match_count(), 2);
        let first = file.matches.first().unwrap();
        assert_eq!(first.line_number, 1);
        let range = first.ranges.first().unwrap();
        assert_eq!(first.text.get(range.start..range.end), Some("Clock"));
    }

    #[test]
    fn case_sensitive_respects_case() {
        let fx = Fixture::new();
        fx.write("a.txt", "Clock\nclock\n");
        let mut query = fx.query("clock");
        query.case_sensitive = true;
        let results = run(&query).unwrap();
        let file = results.first().unwrap();
        assert_eq!(file.match_count(), 1);
        assert_eq!(file.matches.first().unwrap().line_number, 2);
    }

    #[test]
    fn whole_word_does_not_match_substring() {
        let fx = Fixture::new();
        fx.write("a.txt", "clock\nclockwork\n");
        let mut query = fx.query("clock");
        query.whole_word = true;
        let results = run(&query).unwrap();
        let file = results.first().unwrap();
        assert_eq!(file.match_count(), 1);
        assert_eq!(file.matches.first().unwrap().line_number, 1);
    }

    #[test]
    fn regex_works_and_invalid_regex_is_an_error() {
        let fx = Fixture::new();
        fx.write("a.txt", "foo123bar\n");
        let mut query = fx.query(r"\d+");
        query.regex = true;
        let results = run(&query).unwrap();
        let line = results.first().unwrap().matches.first().unwrap();
        assert_eq!(line.ranges, vec![MatchRange { start: 3, end: 6 }]);

        let mut invalid = fx.query("[");
        invalid.regex = true;
        assert!(matches!(run(&invalid), Err(SearchError::InvalidPattern(_))));
    }

    #[test]
    fn skips_noisy_dirs_and_binary_files() {
        let fx = Fixture::new();
        fx.write("node_modules/dep/index.js", "needle");
        fx.write("src/app.js", "needle");
        // Binario: byte nulo no comeco.
        fx.write("blob.bin", [0u8, 110, 101, 101, 100, 108, 101]);
        let results = run(&fx.query("needle")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().relative_path, "src/app.js");
    }

    #[test]
    fn empty_term_yields_nothing() {
        let fx = Fixture::new();
        fx.write("a.txt", "anything");
        assert!(run(&fx.query("   ")).unwrap().is_empty());
    }

    #[test]
    fn skips_hidden_dirs_but_not_hidden_files_nor_ds_store() {
        let fx = Fixture::new();
        fx.write(".git/config", "needle");
        fx.write("src/.hidden", "needle");
        fx.write(".DS_Store", "needle");
        fx.write("sub/.DS_Store", "needle");
        let results = run(&fx.query("needle")).unwrap();
        let paths: Vec<_> = results.iter().map(|f| f.relative_path.as_str()).collect();
        assert_eq!(paths, vec!["src/.hidden"]);
    }

    #[test]
    fn ignores_empty_matches() {
        let fx = Fixture::new();
        fx.write("a.txt", "bbb\naab\n");
        let mut query = fx.query("a*");
        query.regex = true;
        let results = run(&query).unwrap();
        let file = results.first().unwrap();
        assert_eq!(file.matches.len(), 1);
        assert_eq!(file.matches.first().unwrap().line_number, 2);
    }

    #[test]
    fn caps_matches_per_file_and_truncates_long_lines() {
        let fx = Fixture::new();
        fx.write("many.txt", "hit\n".repeat(300));
        fx.write("long.txt", format!("hit{}", "x".repeat(2000)));
        let results = run(&fx.query("hit")).unwrap();
        let many = results.iter().find(|f| f.relative_path == "many.txt");
        assert_eq!(many.unwrap().matches.len(), MAX_MATCHES_PER_FILE);
        let long = results.iter().find(|f| f.relative_path == "long.txt");
        let text = &long.unwrap().matches.first().unwrap().text;
        assert_eq!(text.chars().count(), MAX_LINE_CHARS);
    }

    #[test]
    fn skips_files_over_2_mib() {
        let fx = Fixture::new();
        let mut big = b"needle\n".to_vec();
        big.resize(usize::try_from(MAX_FILE_BYTES).unwrap() + 1, b'x');
        fx.write("big.txt", big);
        assert!(run(&fx.query("needle")).unwrap().is_empty());
    }

    #[test]
    fn stops_after_max_files_with_matches() {
        let fx = Fixture::new();
        for i in 0..(MAX_FILES_WITH_MATCHES + 5) {
            fx.write(&format!("f{i}.txt"), "needle");
        }
        let results = run(&fx.query("needle")).unwrap();
        assert_eq!(results.len(), MAX_FILES_WITH_MATCHES);
    }

    #[test]
    fn splits_lines_like_dart() {
        assert_eq!(split_lines("a\nb\r\nc\rd\n"), vec!["a", "b", "c", "d"]);
        assert_eq!(split_lines("a\r\r\nb"), vec!["a", "", "b"]);
        assert_eq!(split_lines("a\n\nb"), vec!["a", "", "b"]);
        assert!(split_lines("").is_empty());
    }

    #[test]
    fn whole_word_boundary_is_ascii_like_dart() {
        let fx = Fixture::new();
        fx.write("a.txt", "cafe\ncafé\n");
        let mut query = fx.query("caf");
        query.whole_word = true;
        let results = run(&query).unwrap();
        // "é" nao e caractere de palavra no RegExp do Dart, entao "caf|é" casa por inteiro.
        assert_eq!(results.first().unwrap().match_count(), 1);
    }

    #[test]
    fn lossy_utf8_still_matches() {
        let fx = Fixture::new();
        fx.write("a.txt", [b'n', b'e', 0xff, b'e', b'd', b'l', b'e', b'\n']);
        fx.write("b.txt", b"needle\xff\n");
        let results = run(&fx.query("needle")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results.first().unwrap().relative_path, "b.txt");
    }

    #[test]
    fn cancel_stops_before_emitting() {
        let fx = Fixture::new();
        fx.write("a.txt", "needle");
        let cancel = AtomicBool::new(true);
        let mut all = Vec::new();
        search(&fx.query("needle"), &cancel, |batch| all.extend(batch)).unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn missing_root_yields_nothing() {
        let fx = Fixture::new();
        let mut query = fx.query("needle");
        query.root = fx.0.path().join("nope");
        assert!(run(&query).unwrap().is_empty());
    }

    #[test]
    fn emits_in_batches() {
        let fx = Fixture::new();
        for i in 0..(BATCH_FILES * 2 + 1) {
            fx.write(&format!("f{i}.txt"), "needle");
        }
        let mut sizes = Vec::new();
        search(&fx.query("needle"), &AtomicBool::new(false), |batch| {
            sizes.push(batch.len());
        })
        .unwrap();
        assert_eq!(sizes.iter().sum::<usize>(), BATCH_FILES * 2 + 1);
        assert!(sizes.iter().all(|&n| n <= BATCH_FILES));
        assert!(sizes.len() >= 3);
    }

    fn regex_query(fx: &Fixture, term: &str) -> ContentQuery {
        let mut query = fx.query(term);
        query.regex = true;
        query
    }

    #[test]
    fn regex_accepts_lookahead_and_lookbehind() {
        let fx = Fixture::new();
        fx.write("a.txt", "foobar\nfoobaz\nxbar\n");
        let results = run(&regex_query(&fx, "foo(?=bar)")).unwrap();
        let file = results.first().unwrap();
        assert_eq!(file.matches.len(), 1);
        let line = file.matches.first().unwrap();
        assert_eq!(line.line_number, 1);
        assert_eq!(line.ranges, vec![MatchRange { start: 0, end: 3 }]);

        let results = run(&regex_query(&fx, "(?<!x)bar")).unwrap();
        let lines: Vec<_> = results[0].matches.iter().map(|m| m.line_number).collect();
        assert_eq!(lines, vec![1]);
    }

    #[test]
    fn regex_accepts_backreference() {
        let fx = Fixture::new();
        fx.write("a.txt", "abab\nabba\nabcd\n");
        let results = run(&regex_query(&fx, r"(ab)\1")).unwrap();
        let file = results.first().unwrap();
        assert_eq!(file.matches.len(), 1);
        assert_eq!(file.matches.first().unwrap().line_number, 1);
        assert_eq!(
            file.matches.first().unwrap().ranges,
            vec![MatchRange { start: 0, end: 4 }]
        );
    }

    #[test]
    fn whole_word_with_a_regex_term_keeps_lookaround_working() {
        let fx = Fixture::new();
        fx.write("a.txt", "foo bar\nfoo baz\nfoobar\n");
        let mut query = regex_query(&fx, "foo(?= bar)");
        query.whole_word = true;
        let results = run(&query).unwrap();
        let lines: Vec<_> = results[0].matches.iter().map(|m| m.line_number).collect();
        assert_eq!(lines, vec![1]);
    }

    #[test]
    fn backtrack_limit_ends_that_file_without_error_or_panic() {
        let fx = Fixture::new();
        // Backreference catastrofica: estoura o limite de backtracking do fancy-regex.
        let evil = format!("{}!", "a".repeat(40));
        fx.write("evil.txt", format!("{evil}\nfind-me\n"));
        fx.write("ok.txt", "find-me\n");
        let results = run(&regex_query(&fx, r"find-me|(a+)+\1$")).unwrap();
        assert!(results.iter().any(|f| f.relative_path == "ok.txt"));
        // O arquivo que estourou o limite para na linha que falhou: a linha seguinte nao entra.
        assert!(results.iter().all(|f| f.relative_path != "evil.txt"));
    }

    #[test]
    fn unbalanced_group_and_bad_backreference_are_invalid_patterns() {
        let fx = Fixture::new();
        fx.write("a.txt", "x");
        for term in ["(", "a)", r"\1", "(?<=a+)b"] {
            assert!(
                matches!(
                    run(&regex_query(&fx, term)),
                    Err(SearchError::InvalidPattern(_))
                ),
                "{term}"
            );
        }
    }
}
