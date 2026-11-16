//! Arquivo `.dbq` (`dbq_document.dart`): frontmatter em comentários SQL e o corpo SQL puro.
//!
//! ```sql
//! -- db: dev-local
//! -- limit: 100
//! SELECT * FROM orders;
//! ```
//!
//! O frontmatter persiste a conexão escolhida e o limite; o editor mostra só o SQL. O parse é
//! conservador: consome apenas o bloco inicial de linhas `-- <chave-conhecida>: <valor>`.

/// Documento `.dbq`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DbqDocument {
    /// Conexão (`-- db: <nome>`); `None` pede o seletor.
    pub db: Option<String>,
    /// Limite (`-- limit: <N>`); `None` = padrão da execução.
    pub limit: Option<i64>,
    /// Corpo SQL, sem o frontmatter.
    pub sql: String,
}

impl DbqDocument {
    /// Lê o conteúdo de um arquivo.
    pub fn parse(content: &str) -> Self {
        let lines: Vec<&str> = content.split('\n').collect();
        let mut doc = Self::default();
        let mut body_start = 0;
        for line in &lines {
            let Some((key, value)) = key_line(line) else {
                break;
            };
            match key {
                "db" => doc.db = (!value.is_empty()).then(|| value.to_string()),
                "limit" => doc.limit = value.parse().ok(),
                _ => break,
            }
            body_start += 1;
        }
        // uma linha em branco separando o frontmatter do corpo é cosmética
        if body_start > 0 && lines.get(body_start).is_some_and(|line| line.is_empty()) {
            body_start += 1;
        }
        doc.sql = lines.get(body_start..).unwrap_or_default().join("\n");
        doc
    }

    /// Conteúdo do arquivo: frontmatter, linha em branco e SQL.
    pub fn serialize(&self) -> String {
        let mut head = String::new();
        if let Some(db) = &self.db {
            head.push_str(&format!("-- db: {db}\n"));
        }
        if let Some(limit) = self.limit {
            head.push_str(&format!("-- limit: {limit}\n"));
        }
        if head.is_empty() {
            return self.sql.clone();
        }
        format!("{head}\n{}", self.sql)
    }
}

/// `^--\s*([a-z][a-z0-9-]*)\s*:\s*(.*?)\s*$` restrito às chaves conhecidas.
fn key_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("--")?.trim_start();
    let key_len = rest
        .char_indices()
        .find(|(index, c)| {
            !(c.is_ascii_lowercase() || (*index > 0 && (c.is_ascii_digit() || *c == '-')))
        })
        .map_or(rest.len(), |(index, _)| index);
    let (key, after) = rest.split_at(key_len);
    if key.is_empty() || !matches!(key, "db" | "limit") {
        return None;
    }
    let value = after.trim_start().strip_prefix(':')?.trim();
    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_and_body() {
        let doc = DbqDocument::parse("-- db: dev-local\n-- limit: 50\n\nSELECT 1;\n");
        assert_eq!(doc.db.as_deref(), Some("dev-local"));
        assert_eq!(doc.limit, Some(50));
        assert_eq!(doc.sql, "SELECT 1;\n");
    }

    #[test]
    fn a_plain_comment_ends_the_frontmatter() {
        let doc = DbqDocument::parse("-- db: a\n-- anything\nSELECT 1;");
        assert_eq!(doc.db.as_deref(), Some("a"));
        assert_eq!(doc.sql, "-- anything\nSELECT 1;");
    }

    #[test]
    fn unknown_keys_are_body() {
        let doc = DbqDocument::parse("-- note: x\nSELECT 1;");
        assert_eq!(doc.db, None);
        assert_eq!(doc.sql, "-- note: x\nSELECT 1;");
    }

    #[test]
    fn empty_value_and_bad_limit_are_none() {
        let doc = DbqDocument::parse("-- db:\n-- limit: many\nx");
        assert_eq!(doc.db, None);
        assert_eq!(doc.limit, None);
        assert_eq!(doc.sql, "x");
    }

    #[test]
    fn serialization_round_trips() {
        let doc = DbqDocument {
            db: Some("pg".into()),
            limit: Some(10),
            sql: "select 1;\n".into(),
        };
        assert_eq!(doc.serialize(), "-- db: pg\n-- limit: 10\n\nselect 1;\n");
        assert_eq!(DbqDocument::parse(&doc.serialize()), doc);
        let bare = DbqDocument {
            sql: "select 2".into(),
            ..DbqDocument::default()
        };
        assert_eq!(bare.serialize(), "select 2");
    }
}
