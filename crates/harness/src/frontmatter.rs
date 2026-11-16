//! Frontmatter (`---` … `---`) do começo de um markdown.

use std::collections::HashMap;

/// `chave: valor` por linha (só o nível de cima), com aspas simples ou duplas
/// tiradas. Sem frontmatter, devolve vazio.
///
/// O Flutter tenta YAML primeiro e só cai neste leitor quando o YAML falha; aqui
/// não há crate YAML, então só existe o leitor por linha (valor em várias linhas,
/// como `description: >`, sai como o texto da primeira linha).
pub fn parse_frontmatter(content: &str) -> HashMap<String, String> {
    let text = content.replace("\r\n", "\n");
    let mut out = HashMap::new();
    if !text.starts_with("---\n") {
        return out;
    }
    let Some(rest) = text.get(4..) else {
        return out;
    };
    let Some(end) = rest.find("\n---") else {
        return out;
    };
    let Some(block) = rest.get(..end) else {
        return out;
    };
    for line in block.split('\n') {
        let Some(colon) = line.find(':') else {
            continue;
        };
        if colon == 0 || line.starts_with(' ') {
            continue;
        }
        let (Some(key), Some(value)) = (line.get(..colon), line.get(colon + 1..)) else {
            continue;
        };
        out.insert(key.trim().to_owned(), unquote(value.trim()).to_owned());
    }
    out
}

fn unquote(value: &str) -> &str {
    let quoted = value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')));
    if quoted {
        value.get(1..value.len() - 1).unwrap_or(value)
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_top_level_keys_and_strips_matching_quotes() {
        let fm = parse_frontmatter(
            "---\nname: levi\ndescription: \"QA: tudo\"\n  nested: x\ntools: 'a'\n---\ncorpo",
        );
        assert_eq!(fm.get("name").map(String::as_str), Some("levi"));
        assert_eq!(fm.get("description").map(String::as_str), Some("QA: tudo"));
        assert_eq!(fm.get("tools").map(String::as_str), Some("a"));
        assert!(!fm.contains_key("nested"));
    }

    #[test]
    fn no_frontmatter_or_unterminated_is_empty() {
        assert!(parse_frontmatter("sem frontmatter").is_empty());
        assert!(parse_frontmatter("---\nname: x\n").is_empty());
        assert!(parse_frontmatter("---\n---\n").is_empty());
    }

    #[test]
    fn crlf_and_mismatched_quotes() {
        let fm = parse_frontmatter("---\r\nname: \"x'\r\n---\r\n");
        assert_eq!(fm.get("name").map(String::as_str), Some("\"x'"));
    }
}
