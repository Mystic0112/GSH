//! Divisão do buffer em statements (`sql_statements.dart`): a semântica de "Run" dos clients
//! (DataGrip/DBeaver): o statement sob o cursor ou os que a seleção toca.
//!
//! Os offsets são em bytes do buffer original.

/// Um statement: texto executável (sem o `;` final) e o intervalo `[start, end)` no buffer
/// (o `;` pertence ao intervalo).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SqlStatement {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Divide `source` em statements por `;`, ignorando `;` dentro de strings (`'…'` com `''`,
/// `"…"`, `` `…` ``) e de comentários (`--` e `/* */`). Blocos só de comentário são descartados.
pub fn split(source: &str) -> Vec<SqlStatement> {
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut statement_start = 0;
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b'\'' | b'"' | b'`' => at = skip_quoted(bytes, at, byte),
            b'-' if bytes.get(at + 1) == Some(&b'-') => at = line_end(bytes, at),
            b'/' if bytes.get(at + 1) == Some(&b'*') => at = block_end(bytes, at),
            b';' => {
                push(&mut out, source, statement_start, at + 1);
                statement_start = at + 1;
                at += 1;
            }
            _ => at += 1,
        }
    }
    push(&mut out, source, statement_start, source.len());
    out
}

/// Statement sob o `offset` do cursor: o primeiro cujo intervalo ainda não acabou no offset
/// (cursor entre statements pega o anterior). `None` só com lista vazia.
pub fn statement_at(statements: &[SqlStatement], offset: usize) -> Option<&SqlStatement> {
    statements
        .iter()
        .find(|statement| offset <= statement.end)
        .or_else(|| statements.last())
}

/// Statements que a seleção `[start, end)` toca, expandidos ao statement inteiro. Seleção só em
/// espaço cai no [`statement_at`] do início.
pub fn statements_in_range(
    statements: &[SqlStatement],
    start: usize,
    end: usize,
) -> Vec<&SqlStatement> {
    let hit: Vec<&SqlStatement> = statements
        .iter()
        .filter(|statement| statement.start < end && statement.end > start)
        .collect();
    if hit.is_empty() {
        statement_at(statements, start).into_iter().collect()
    } else {
        hit
    }
}

fn skip_quoted(bytes: &[u8], open: usize, quote: u8) -> usize {
    let mut at = open + 1;
    while let Some(&byte) = bytes.get(at) {
        if byte == quote {
            if quote == b'\'' && bytes.get(at + 1) == Some(&b'\'') {
                at += 2;
                continue;
            }
            return at + 1;
        }
        at += 1;
    }
    bytes.len()
}

fn line_end(bytes: &[u8], from: usize) -> usize {
    bytes
        .get(from..)
        .and_then(|rest| rest.iter().position(|&b| b == b'\n'))
        .map_or(bytes.len(), |nl| from + nl + 1)
}

fn block_end(bytes: &[u8], from: usize) -> usize {
    bytes
        .get(from + 2..)
        .and_then(|rest| rest.windows(2).position(|pair| pair == b"*/"))
        .map_or(bytes.len(), |close| from + 2 + close + 2)
}

fn push(out: &mut Vec<SqlStatement>, source: &str, start: usize, end: usize) {
    let raw = source.get(start..end).unwrap_or_default().trim();
    let text = raw.strip_suffix(';').map_or(raw, str::trim);
    if text.is_empty() || only_comments(text) {
        return;
    }
    out.push(SqlStatement {
        text: text.to_string(),
        start,
        end,
    });
}

/// Só comentários e espaço (nada executável).
fn only_comments(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' => at += 1,
            b'-' if bytes.get(at + 1) == Some(&b'-') => at = line_end(bytes, at),
            b'/' if bytes.get(at + 1) == Some(&b'*') => at = block_end(bytes, at),
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(source: &str) -> Vec<String> {
        split(source).into_iter().map(|s| s.text).collect()
    }

    #[test]
    fn semicolons_inside_quotes_and_comments_do_not_split() {
        let source = "select 'a;b'; -- x; y\nselect \"c;d\", `e;f`; /* g; */ select 1";
        assert_eq!(
            texts(source),
            [
                "select 'a;b'",
                "-- x; y\nselect \"c;d\", `e;f`",
                "/* g; */ select 1"
            ]
        );
    }

    #[test]
    fn doubled_quote_is_an_escape() {
        assert_eq!(
            texts("select 'it''s;ok'; select 2;"),
            ["select 'it''s;ok'", "select 2"]
        );
    }

    #[test]
    fn comment_only_chunks_are_dropped() {
        assert!(split("-- nada\n/* nada */;").is_empty());
        assert!(split("  \n ").is_empty());
    }

    #[test]
    fn cursor_picks_the_statement_it_is_in_or_after() {
        let statements = split("select 1;\n\nselect 2;");
        assert_eq!(statement_at(&statements, 3).unwrap().text, "select 1");
        // depois do `;` do primeiro e antes do segundo: ainda é o primeiro
        assert_eq!(statement_at(&statements, 9).unwrap().text, "select 1");
        assert_eq!(statement_at(&statements, 12).unwrap().text, "select 2");
        assert_eq!(statement_at(&statements, 999).unwrap().text, "select 2");
        assert!(statement_at(&[], 0).is_none());
    }

    #[test]
    fn selection_expands_to_whole_statements() {
        let statements = split("select 1;\nselect 2;\nselect 3;");
        let hit = statements_in_range(&statements, 12, 22);
        let hit: Vec<_> = hit.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(hit, ["select 2", "select 3"]);
        // seleção só em espaço: cai no statement do início
        let blank = statements_in_range(&statements, 9, 10);
        assert_eq!(blank.len(), 1);
    }
}
