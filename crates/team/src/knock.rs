//! Batida do correio (versao 1 de lan_team_mail.dart): o corpo JSON que, cifrado, vai na
//! linha `{"e": base64}`. Hora fora da janela ou campo errado = descartada.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnockKind {
    Message,
    File,
}

impl KnockKind {
    fn name(self) -> &'static str {
        match self {
            Self::Message => "message",
            Self::File => "file",
        }
    }
}

/// Algo batendo na porta de um agente: mensagem ou arquivo de um colega (`TeamKnock`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Knock {
    /// Unico por batida (tambem e o nonce contra reenvio).
    pub id: String,
    pub kind: KnockKind,
    pub from_machine: String,
    pub from_name: String,
    /// Nome da aba do agente que mandou (para responder).
    pub from_agent: String,
    pub from_session_id: Option<String>,
    pub to_tab_id: String,
    pub text: String,
    pub file_name: Option<String>,
    pub file_bytes: Option<Vec<u8>>,
}

pub fn encode_knock(k: &Knock, team: &str, at_ms: i64) -> String {
    let mut o = Map::new();
    o.insert("v".into(), json!(1));
    o.insert("team".into(), json!(team.trim()));
    o.insert("ts".into(), json!(at_ms));
    o.insert("id".into(), json!(k.id));
    o.insert("kind".into(), json!(k.kind.name()));
    o.insert("fm".into(), json!(k.from_machine));
    o.insert("fn".into(), json!(k.from_name));
    o.insert("fa".into(), json!(k.from_agent));
    if let Some(s) = &k.from_session_id {
        o.insert("fs".into(), json!(s));
    }
    o.insert("to".into(), json!(k.to_tab_id));
    o.insert("text".into(), json!(k.text));
    if let Some(f) = &k.file_name {
        o.insert("file".into(), json!(f));
    }
    if let Some(b) = &k.file_bytes {
        o.insert("data".into(), json!(STANDARD.encode(b)));
    }
    serde_json::to_string(&Value::Object(o)).unwrap_or_default()
}

fn req_str(o: &Map<String, Value>, k: &str) -> Option<String> {
    o.get(k).and_then(Value::as_str).map(str::to_owned)
}

/// String opcional: ausente/null = `Some(None)`; tipo errado = `None` (derruba a batida).
fn opt_str(o: &Map<String, Value>, k: &str) -> Option<Option<String>> {
    match o.get(k) {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(s)) => Some(Some(s.clone())),
        Some(_) => None,
    }
}

/// `None` = versao desconhecida, hora fora da janela (`skew_ms`) ou campo faltando.
pub fn decode_knock(body: &str, now_ms: i64, skew_ms: u64) -> Option<Knock> {
    let Value::Object(o) = serde_json::from_str::<Value>(body).ok()? else {
        return None;
    };
    if o.get("v").and_then(Value::as_f64) != Some(1.0) {
        return None;
    }
    let at = o.get("ts").and_then(Value::as_i64)?;
    if now_ms.abs_diff(at) > skew_ms {
        return None;
    }
    let kind = match o.get("kind").and_then(Value::as_str)? {
        "message" => KnockKind::Message,
        "file" => KnockKind::File,
        _ => return None,
    };
    let file_bytes = match opt_str(&o, "data")? {
        Some(d) => Some(STANDARD.decode(d).ok()?),
        None => None,
    };
    Some(Knock {
        id: req_str(&o, "id")?,
        kind,
        from_machine: req_str(&o, "fm")?,
        from_name: opt_str(&o, "fn")?.unwrap_or_default(),
        from_agent: opt_str(&o, "fa")?.unwrap_or_default(),
        from_session_id: opt_str(&o, "fs")?,
        to_tab_id: req_str(&o, "to")?,
        text: opt_str(&o, "text")?.unwrap_or_default(),
        file_name: opt_str(&o, "file")?,
        file_bytes,
    })
}
