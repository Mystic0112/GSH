//! Formatação da saída de `gsh list-tabs`. A saída é lida por humano E por
//! agente, então largura de coluna e espaçamento importam.

use serde_json::Value;

/// Preenche à direita com espaços até `n` caracteres (nunca trunca).
pub fn pad(s: &str, n: usize) -> String {
    let len = s.chars().count();
    let mut out = String::from(s);
    out.extend(std::iter::repeat_n(' ', n.saturating_sub(len)));
    out
}

/// Último segmento do caminho, ignorando separadores finais. No Windows aceita
/// `\` e `/`.
pub fn basename(path: &str) -> String {
    let is_sep = |c: char| c == '/' || (cfg!(windows) && c == '\\');
    path.split(is_sep)
        .rfind(|p| !p.is_empty())
        .unwrap_or(path)
        .to_string()
}

/// Representação textual de um valor JSON como o Dart faria em `toString()`
/// (string sem aspas; número/bool crus; nulo vazio).
fn display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// Campo do objeto como texto, `""` quando ausente ou nulo.
fn field(v: &Value, key: &str) -> String {
    v.get(key).map(display).unwrap_or_default()
}

/// Uma linha por aba: marcador de trabalho, id, tipo, workspace, nome e harness.
/// Entradas que não são objeto são ignoradas.
pub fn format_tab_list(data: &[Value]) -> Vec<String> {
    data.iter()
        .filter(|e| e.is_object())
        .map(format_tab_line)
        .collect()
}

fn format_tab_line(e: &Value) -> String {
    let flag = if e.get("working") == Some(&Value::Bool(true)) {
        "●"
    } else {
        " "
    };
    // Rótulo manual (nome estável) vence o título dinâmico; `⚲` sinaliza que
    // está travado.
    let label = e.get("label").and_then(Value::as_str).unwrap_or("");
    let name = if label.is_empty() {
        field(e, "title")
    } else {
        format!("⚲ {label}")
    };
    // Workspace: basename do path (o `workspaceId` é um UUID opaco). Sem
    // `workspacePath` (app antigo) mostra o id.
    let ws_path = field(e, "workspacePath");
    let ws = if ws_path.is_empty() {
        field(e, "workspaceId")
    } else {
        basename(&ws_path)
    };
    let harness = field(e, "harness");
    let harness = if harness.is_empty() {
        String::new()
    } else {
        format!("  [{harness}]")
    };
    format!(
        "{flag} {} {} {} {name}{harness}",
        pad(&field(e, "id"), 6),
        pad(&field(e, "kind"), 9),
        pad(&ws, 14)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn pad_preenche_e_nunca_trunca() {
        assert_eq!(pad("ab", 5), "ab   ");
        assert_eq!(pad("abcdef", 3), "abcdef");
        assert_eq!(pad("", 2), "  ");
        assert_eq!(pad("áé", 4), "áé  ", "conta caracteres, não bytes");
    }

    #[test]
    fn basename_ignora_barras_finais() {
        assert_eq!(basename("/a/b/c"), "c");
        assert_eq!(basename("/a/b/c/"), "c");
        assert_eq!(basename("solo"), "solo");
        assert_eq!(basename("/"), "/");
        assert_eq!(basename(""), "");
    }

    #[test]
    fn linha_completa_com_rotulo_harness_e_trabalhando() {
        let tabs = [json!({
            "id": "t1", "kind": "terminal", "title": "zsh", "label": "Worker",
            "workspacePath": "/x/remote_pi", "workspaceId": "uuid",
            "working": true, "harness": "claudeCode"
        })];
        assert_eq!(
            format_tab_list(&tabs),
            ["● t1     terminal  remote_pi      ⚲ Worker  [claudeCode]"]
        );
    }

    #[test]
    fn linha_minima_usa_titulo_e_workspace_id() {
        let tabs = [json!({"id": "t12", "kind": "viewer", "title": "a.md", "workspaceId": "w1"})];
        assert_eq!(
            format_tab_list(&tabs),
            ["  t12    viewer    w1             a.md"]
        );
    }

    #[test]
    fn entradas_que_nao_sao_objeto_somem() {
        assert!(format_tab_list(&[json!(1), json!("x"), Value::Null]).is_empty());
    }
}
