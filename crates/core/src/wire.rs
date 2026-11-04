//! Protocolo do socket do app: uma requisição por conexão, uma linha JSON em
//! cada direção. Só montagem e leitura de valores; quem abre o socket é a CLI.
//!
//! O mesmo socket carrega dois tipos de mensagem: comandos (`"type":"cmd"`,
//! com resposta) e status de hook (sem `type`, sem resposta).

use serde_json::{json, Map, Value};

/// Env injetada pelo app na PTY da aba: id da aba (legado, ainda lido pelo hook).
pub const ENV_PANE_ID: &str = "GSH_PANE_ID";
/// Env injetada pelo app na PTY da aba: id da aba (nome atual).
pub const ENV_TAB_ID: &str = "GSH_TAB_ID";
/// Caminho do socket Unix do app.
pub const ENV_STATUS_SOCK: &str = "GSH_STATUS_SOCK";
/// Porta TCP no loopback (Windows).
pub const ENV_STATUS_PORT: &str = "GSH_STATUS_PORT";
/// Token do TCP (Windows); no socket Unix a permissão do arquivo já protege.
pub const ENV_STATUS_TOKEN: &str = "GSH_STATUS_TOKEN";

/// Sentinela que o app troca pelo id da aba em foco (`--focused`).
pub const FOCUSED: &str = "@focused";

/// Requisição de comando: `{"type":"cmd","cmd":…,"tabId":…?,"args":…?}`.
/// `tab_id` vazio é tratado como ausente.
pub fn cmd_request(cmd: &str, tab_id: Option<&str>, args: Option<Value>) -> Value {
    let mut req = Map::new();
    req.insert("type".into(), json!("cmd"));
    req.insert("cmd".into(), json!(cmd));
    if let Some(id) = tab_id.filter(|id| !id.is_empty()) {
        req.insert("tabId".into(), json!(id));
    }
    if let Some(args) = args {
        req.insert("args".into(), args);
    }
    Value::Object(req)
}

/// Anexa o token (`tok`) a uma requisição ou payload. Não-objeto fica intacto.
pub fn with_token(mut msg: Value, token: &str) -> Value {
    if let Value::Object(map) = &mut msg {
        map.insert("tok".into(), json!(token));
    }
    msg
}

/// Pedido de permissão do hook: o app segura a resposta até alguém decidir.
pub fn permission_ask_request(pane_id: &str, event: &Value) -> Value {
    let field = |key: &str| event.get(key).cloned().unwrap_or(Value::Null);
    cmd_request(
        "permission-ask",
        Some(pane_id),
        Some(json!({
            "tool_name": field("tool_name"),
            "tool_input": field("tool_input"),
            "permission_suggestions": field("permission_suggestions"),
        })),
    )
}

/// Resposta do app: sempre um objeto JSON. Ausente ou malformada vira
/// `{"ok": false, "error": …}` para o chamador tratar como erro de aplicação.
#[derive(Debug, Clone, PartialEq)]
pub struct Response(Value);

impl Response {
    pub fn parse(raw: &str) -> Response {
        let line = raw.trim();
        if line.is_empty() {
            return Self::failure("no response from app");
        }
        match serde_json::from_str::<Value>(line) {
            Ok(v) if v.is_object() => Response(v),
            _ => Self::failure("malformed response"),
        }
    }

    fn failure(error: &str) -> Response {
        Response(json!({"ok": false, "error": error}))
    }

    /// `ok == true` (qualquer outra coisa, inclusive ausente, é falha).
    pub fn is_ok(&self) -> bool {
        self.0.get("ok") == Some(&Value::Bool(true))
    }

    /// Aviso do app (hoje: `--workspace` mirando outra máquina). A CLI o joga
    /// no stderr para não sujar o JSON do stdout.
    pub fn warning(&self) -> Option<&str> {
        self.0.get("warning").and_then(Value::as_str)
    }

    /// Mensagem de erro, com o mesmo fallback da CLI original (`failed`).
    pub fn error_text(&self) -> String {
        match self.0.get("error") {
            Some(Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
            None => "failed".to_string(),
        }
    }

    /// `data` da resposta, ou `{}` quando ausente.
    pub fn data(&self) -> Value {
        self.0.get("data").cloned().unwrap_or_else(|| json!({}))
    }

    /// `data` como lista (vazia quando ausente ou de outro tipo).
    pub fn data_list(&self) -> Vec<Value> {
        match self.0.get("data") {
            Some(Value::Array(items)) => items.clone(),
            _ => Vec::new(),
        }
    }

    /// Saída do hook montada pelo app em `data.output` (resposta do
    /// `permission-ask`). `None` = deixar o terminal perguntar.
    pub fn hook_output(&self) -> Option<&Value> {
        if !self.is_ok() {
            return None;
        }
        self.0.get("data")?.get("output").filter(|v| !v.is_null())
    }
}

/// Sockets a tentar, em ordem: o do ambiente e os caminhos bem conhecidos em
/// `~/.gsh`. O socket do próprio flavor vem primeiro: um GSH de dev e um
/// instalado rodam lado a lado, e a CLI de dev não deve ser atendida pelo app
/// instalado, que pode ser bem mais velho.
pub fn candidate_sockets(
    env_sock: Option<String>,
    home: Option<String>,
    debug_first: bool,
) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(path) = env_sock {
        out.push(path);
    }
    if let Some(home) = home {
        let release = format!("{home}/.gsh/status.sock");
        let debug = format!("{home}/.gsh/status-debug.sock");
        if debug_first {
            out.push(debug);
            out.push(release);
        } else {
            out.push(release);
            out.push(debug);
        }
    }
    out
}

/// Conteúdo de `~/.gsh/status[-debug].json` (Windows): `{port, tok}`.
pub fn parse_endpoint_file(raw: &str) -> Option<(u16, Option<String>)> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let port = u16::try_from(v.get("port")?.as_u64()?).ok()?;
    let tok = v.get("tok").and_then(Value::as_str).map(str::to_string);
    Some((port, tok))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_carrega_type_cmd_e_omite_o_que_falta() {
        let req = cmd_request("list-panes", None, None);
        assert_eq!(req.to_string(), r#"{"cmd":"list-panes","type":"cmd"}"#);
        let req = cmd_request("close-tab", Some(""), Some(json!({})));
        assert_eq!(
            req.to_string(),
            r#"{"args":{},"cmd":"close-tab","type":"cmd"}"#
        );
        let req = cmd_request("msg", Some("t7"), Some(json!({"to": "a", "text": "b"})));
        assert_eq!(
            req.to_string(),
            r#"{"args":{"text":"b","to":"a"},"cmd":"msg","tabId":"t7","type":"cmd"}"#
        );
    }

    #[test]
    fn token_entra_como_tok() {
        let req = with_token(cmd_request("x", None, None), "abc");
        assert_eq!(req["tok"], "abc");
        assert_eq!(with_token(json!(1), "abc"), json!(1));
    }

    #[test]
    fn resposta_ausente_ou_malformada_vira_erro_de_aplicacao() {
        for (raw, msg) in [
            ("", "no response from app"),
            ("  \n", "no response from app"),
            ("nao e json", "malformed response"),
            ("[1,2]", "malformed response"),
            ("\"x\"", "malformed response"),
        ] {
            let r = Response::parse(raw);
            assert!(!r.is_ok(), "{raw}");
            assert_eq!(r.error_text(), msg, "{raw}");
        }
    }

    #[test]
    fn resposta_ok_expoe_data_e_aviso() {
        let r = Response::parse(r#"{"ok":true,"data":{"tabId":"t1"},"warning":"w"}"#);
        assert!(r.is_ok());
        assert_eq!(r.warning(), Some("w"));
        assert_eq!(r.data()["tabId"], "t1");
        assert!(r.data_list().is_empty());
        assert_eq!(Response::parse(r#"{"ok":true}"#).data(), json!({}));
        // `ok` só vale se for o booleano true.
        assert!(!Response::parse(r#"{"ok":"true"}"#).is_ok());
    }

    #[test]
    fn error_text_cai_em_failed_e_aceita_nao_string() {
        assert_eq!(Response::parse(r#"{"ok":false}"#).error_text(), "failed");
        assert_eq!(
            Response::parse(r#"{"ok":false,"error":{"k":1}}"#).error_text(),
            r#"{"k":1}"#
        );
        assert_eq!(
            Response::parse(r#"{"ok":false,"error":"boom"}"#).error_text(),
            "boom"
        );
    }

    #[test]
    fn hook_output_so_com_ok_e_output_nao_nulo() {
        let ok = Response::parse(r#"{"ok":true,"data":{"output":{"a":1}}}"#);
        assert_eq!(ok.hook_output(), Some(&json!({"a": 1})));
        for raw in [
            r#"{"ok":true,"data":{"output":null}}"#,
            r#"{"ok":true,"data":{}}"#,
            r#"{"ok":true}"#,
            r#"{"ok":false,"data":{"output":{"a":1}}}"#,
        ] {
            assert_eq!(Response::parse(raw).hook_output(), None, "{raw}");
        }
    }

    #[test]
    fn permission_ask_leva_os_tres_campos() {
        let ev = json!({"tool_name": "Bash", "tool_input": {"command": "ls"}});
        let req = permission_ask_request("t3", &ev);
        assert_eq!(req["cmd"], "permission-ask");
        assert_eq!(req["tabId"], "t3");
        assert_eq!(req["args"]["tool_name"], "Bash");
        assert_eq!(req["args"]["tool_input"]["command"], "ls");
        assert_eq!(req["args"]["permission_suggestions"], Value::Null);
    }

    #[test]
    fn candidatos_de_socket_env_primeiro_depois_flavor_proprio() {
        let h = Some("/h".to_string());
        assert_eq!(
            candidate_sockets(Some("/x.sock".into()), h.clone(), false),
            [
                "/x.sock",
                "/h/.gsh/status.sock",
                "/h/.gsh/status-debug.sock"
            ]
        );
        assert_eq!(
            candidate_sockets(None, h, true),
            ["/h/.gsh/status-debug.sock", "/h/.gsh/status.sock"]
        );
        assert!(candidate_sockets(None, None, false).is_empty());
    }

    #[test]
    fn arquivo_de_endpoint_do_windows() {
        assert_eq!(
            parse_endpoint_file(r#"{"port":4000,"tok":"t"}"#),
            Some((4000, Some("t".into())))
        );
        assert_eq!(parse_endpoint_file(r#"{"port":4000}"#), Some((4000, None)));
        assert_eq!(parse_endpoint_file(r#"{"port":70000}"#), None);
        assert_eq!(parse_endpoint_file(r#"{"tok":"t"}"#), None);
        assert_eq!(parse_endpoint_file("lixo"), None);
    }
}
