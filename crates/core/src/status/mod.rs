//! Status de turno dos agentes, lado do app: o que chega pelo socket de status
//! vira tipo (`StatusUpdate`) e o tipo move a máquina de turno (`turn`).
//!
//! Sem socket, sem disco, sem relógio: quem lê a conexão é `gsh-ipc`, e o
//! tempo entra como parâmetro. O cliente (quem escreve a linha) é o
//! `gsh hook`; a tradução evento -> status dele está em [`crate::hook`].
//!
//! Envs que o PTY da aba injeta e a CLI lê (nome e formato; injetar é do
//! terminal, não daqui):
//! - `GSH_PANE_ID`: id da aba, usado no roteamento (`paneId` da linha);
//! - `GSH_TAB_ID`: o mesmo id, nome atual (a CLI cai em `GSH_PANE_ID` sem ele);
//! - `GSH_STATUS_SOCK`: caminho do socket Unix do app;
//! - `GSH_STATUS_PORT` / `GSH_STATUS_TOKEN`: porta TCP no loopback (u16) e
//!   token hex de 32 caracteres, no Windows.
//!
//! Referência de comportamento: `docs/turn-status-hooks.md` e
//! `terminal_status_server_impl.dart` do repo Flutter.

pub mod turn;

use serde_json::Value;
use thiserror::Error;

use crate::hook::str_field;

pub use turn::{Applied, TurnStatus, TurnTracker};

/// Evento de hook que originou o status. O app só distingue
/// [`HookEvent::UserPromptSubmit`] (início de turno) dos demais; o resto vem
/// tipado para quem quiser. Evento fora da lista vira [`HookEvent::Other`] com o
/// nome cru (helper de outra versão), como o app original, que guardava a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookEvent {
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    Notification,
    Stop,
    SessionStart,
    SessionEnd,
    PermissionRequest,
    Other(String),
}

impl HookEvent {
    pub fn from_wire(s: &str) -> HookEvent {
        match s {
            "UserPromptSubmit" => HookEvent::UserPromptSubmit,
            "PreToolUse" => HookEvent::PreToolUse,
            "PostToolUse" => HookEvent::PostToolUse,
            "Notification" => HookEvent::Notification,
            "Stop" => HookEvent::Stop,
            "SessionStart" => HookEvent::SessionStart,
            "SessionEnd" => HookEvent::SessionEnd,
            "PermissionRequest" => HookEvent::PermissionRequest,
            other => HookEvent::Other(other.to_string()),
        }
    }

    pub fn as_wire(&self) -> &str {
        match self {
            HookEvent::UserPromptSubmit => "UserPromptSubmit",
            HookEvent::PreToolUse => "PreToolUse",
            HookEvent::PostToolUse => "PostToolUse",
            HookEvent::Notification => "Notification",
            HookEvent::Stop => "Stop",
            HookEvent::SessionStart => "SessionStart",
            HookEvent::SessionEnd => "SessionEnd",
            HookEvent::PermissionRequest => "PermissionRequest",
            HookEvent::Other(s) => s,
        }
    }
}

/// Quem emitiu o evento. O id de sessão sozinho não diz de qual harness é, e o
/// comando de retomada difere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentHarness {
    Claude,
    Codex,
    Pi,
}

impl AgentHarness {
    /// Nome no wire (`hn`): ausente ou vazio cai em [`AgentHarness::Claude`]
    /// (helpers anteriores à distinção só existiam para o Claude); nome
    /// desconhecido é `None` (virar Claude montaria um resume com o id de outro
    /// harness).
    pub fn from_wire(wire: Option<&str>) -> Option<AgentHarness> {
        match wire {
            None | Some("") | Some("claude") => Some(AgentHarness::Claude),
            Some("codex") => Some(AgentHarness::Codex),
            Some("pi") => Some(AgentHarness::Pi),
            Some(_) => None,
        }
    }
}

/// Uma atualização de status de uma aba, já decodificada da linha do socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusUpdate {
    /// Id da aba (`GSH_PANE_ID` da PTY): a chave de roteamento.
    pub pane_id: String,
    pub status: TurnStatus,
    /// `None` em helper antigo, que não mandava `ev`.
    pub event: Option<HookEvent>,
    pub session_id: Option<String>,
    pub transcript_path: Option<String>,
    /// Já resolvido por [`AgentHarness::from_wire`].
    pub harness: Option<AgentHarness>,
    /// Caminhos absolutos que um `PostToolUse` de edição tocou.
    pub files: Vec<String>,
}

impl StatusUpdate {
    /// `UserPromptSubmit` abre um turno novo: o único `working` que vale mesmo
    /// logo depois de um `idle` (mensagem enfileirada).
    pub fn starts_turn(&self) -> bool {
        self.event == Some(HookEvent::UserPromptSubmit)
    }
}

/// O que uma linha do socket é.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Incoming {
    /// Status do hook: sem resposta.
    Status(StatusUpdate),
    /// `"type":"cmd"` com token válido: pede resposta (a camada de comandos da
    /// CLI é outra, ver `gsh-ipc`).
    Command,
    /// Token ausente ou errado (só quando o servidor exige token). Comando
    /// recebe `invalid token`; status some em silêncio.
    BadToken { command: bool },
    /// Linha vazia, JSON que não é objeto, ou status sem `paneId`/`st`.
    Ignored,
}

/// Linha que não é JSON. O app original registrava um aviso com o trecho.
#[derive(Debug, Error)]
#[error("malformed status line `{sample}`: {source}")]
pub struct LineError {
    /// Primeiros 200 caracteres da linha (com `…` se cortada).
    pub sample: String,
    #[source]
    pub source: serde_json::Error,
}

const SAMPLE_CHARS: usize = 200;

/// Decodifica uma linha do socket. `token` é o que o servidor exige (`Some` no
/// TCP do Windows, `None` no socket Unix, onde a permissão do arquivo protege).
///
/// Mesma ordem do app original: vazio, JSON, objeto, token, tipo.
pub fn decode_line(raw: &str, token: Option<&str>) -> Result<Incoming, LineError> {
    let line = raw.trim();
    if line.is_empty() {
        return Ok(Incoming::Ignored);
    }
    let value: Value = serde_json::from_str(line).map_err(|source| LineError {
        sample: sample(line),
        source,
    })?;
    if !value.is_object() {
        return Ok(Incoming::Ignored);
    }
    let command = value.get("type").and_then(Value::as_str) == Some("cmd");
    if let Some(expected) = token {
        let given = value.get("tok").and_then(Value::as_str);
        if !given.is_some_and(|g| ct_eq(g.as_bytes(), expected.as_bytes())) {
            return Ok(Incoming::BadToken { command });
        }
    }
    if command {
        return Ok(Incoming::Command);
    }
    Ok(decode_status(&value).map_or(Incoming::Ignored, Incoming::Status))
}

fn decode_status(v: &Value) -> Option<StatusUpdate> {
    let pane_id = str_field(v, "paneId");
    let status = str_field(v, "st");
    if pane_id.is_empty() || status.is_empty() {
        return None;
    }
    let opt = |key: &str| Some(str_field(v, key)).filter(|s| !s.is_empty());
    let files = match v.get("files") {
        Some(Value::Array(items)) => items
            .iter()
            .map(|f| match f {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            })
            .collect(),
        _ => Vec::new(),
    };
    Some(StatusUpdate {
        pane_id,
        status: TurnStatus::from_wire(&status),
        event: opt("ev").map(|e| HookEvent::from_wire(&e)),
        session_id: opt("sid"),
        transcript_path: opt("tx"),
        harness: AgentHarness::from_wire(opt("hn").as_deref()),
        files,
    })
}

fn sample(line: &str) -> String {
    let mut chars = line.chars();
    let head: String = chars.by_ref().take(SAMPLE_CHARS).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

/// Igualdade em tempo constante para o token (não sai no primeiro byte
/// diferente). O tamanho do token não é segredo.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(line: &str) -> StatusUpdate {
        match decode_line(line, None) {
            Ok(Incoming::Status(u)) => u,
            other => panic!("esperava Status, veio {other:?}"),
        }
    }

    #[test]
    fn linha_completa_do_hook_vira_tipo() {
        let u = status(
            r#"{"ev":"PostToolUse","files":["/r/a.rs","/r/b.rs"],"hn":"codex","paneId":"t3","sid":"s1","st":"working","tid":"u1","tx":"/t.jsonl"}"#,
        );
        assert_eq!(
            u,
            StatusUpdate {
                pane_id: "t3".into(),
                status: TurnStatus::Working,
                event: Some(HookEvent::PostToolUse),
                session_id: Some("s1".into()),
                transcript_path: Some("/t.jsonl".into()),
                harness: Some(AgentHarness::Codex),
                files: vec!["/r/a.rs".into(), "/r/b.rs".into()],
            }
        );
        assert!(!u.starts_turn());
    }

    #[test]
    fn campos_vazios_ou_ausentes_viram_none_e_harness_cai_em_claude() {
        let u = status(r#"{"paneId":"t1","st":"idle","ev":"","sid":"","tx":"","hn":""}"#);
        assert_eq!(u.event, None);
        assert_eq!(u.session_id, None);
        assert_eq!(u.transcript_path, None);
        assert_eq!(u.harness, Some(AgentHarness::Claude));
        assert!(u.files.is_empty());
        let u = status(r#"{"paneId":"t1","st":"idle"}"#);
        assert_eq!(u.harness, Some(AgentHarness::Claude));
    }

    #[test]
    fn harness_desconhecido_e_none_e_status_desconhecido_e_idle() {
        let u = status(r#"{"paneId":"t1","st":"sleeping","hn":"gemini"}"#);
        assert_eq!(u.harness, None);
        assert_eq!(u.status, TurnStatus::Idle);
        assert_eq!(
            status(r#"{"paneId":"t","st":"x","hn":"pi"}"#).harness,
            Some(AgentHarness::Pi)
        );
    }

    #[test]
    fn user_prompt_submit_inicia_turno() {
        assert!(status(r#"{"paneId":"t","st":"working","ev":"UserPromptSubmit"}"#).starts_turn());
        let u = status(r#"{"paneId":"t","st":"working","ev":"Novo"}"#);
        assert_eq!(u.event, Some(HookEvent::Other("Novo".into())));
        assert!(!u.starts_turn());
    }

    #[test]
    fn os_oito_eventos_ida_e_volta() {
        for ev in [
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "Notification",
            "Stop",
            "SessionStart",
            "SessionEnd",
            "PermissionRequest",
        ] {
            let parsed = HookEvent::from_wire(ev);
            assert!(!matches!(parsed, HookEvent::Other(_)), "{ev}");
            assert_eq!(parsed.as_wire(), ev);
        }
        assert_eq!(HookEvent::from_wire("X").as_wire(), "X");
    }

    #[test]
    fn files_nao_string_vira_forma_json() {
        let u = status(r#"{"paneId":"t","st":"idle","files":["a",7,null]}"#);
        assert_eq!(u.files, ["a", "7", "null"]);
        assert!(status(r#"{"paneId":"t","st":"idle","files":"x"}"#)
            .files
            .is_empty());
    }

    #[test]
    fn paneid_e_st_obrigatorios_e_nao_string_vira_texto() {
        for line in [
            r#"{"st":"idle"}"#,
            r#"{"paneId":"t"}"#,
            r#"{"paneId":"","st":"idle"}"#,
            r#"{"paneId":null,"st":"idle"}"#,
            "",
            "   \n",
            "[1,2]",
            "3",
        ] {
            assert_eq!(
                decode_line(line, None).ok(),
                Some(Incoming::Ignored),
                "{line}"
            );
        }
        assert_eq!(status(r#"{"paneId":7,"st":"idle"}"#).pane_id, "7");
    }

    #[test]
    fn linha_com_quebra_e_cr_e_aceita() {
        assert_eq!(
            status("{\"paneId\":\"t\",\"st\":\"idle\"}\r\n").pane_id,
            "t"
        );
    }

    #[test]
    fn malformada_leva_trecho_cortado_em_200() {
        let err = decode_line("nao e json", None).unwrap_err();
        assert_eq!(err.sample, "nao e json");
        let longa = format!("{{{}", "é".repeat(300));
        let err = decode_line(&longa, None).unwrap_err();
        assert_eq!(err.sample.chars().count(), 201);
        assert!(err.sample.ends_with('…'));
        let exata = format!("{{{}", "x".repeat(199));
        assert_eq!(decode_line(&exata, None).unwrap_err().sample, exata);
    }

    #[test]
    fn comando_e_reconhecido_pelo_type() {
        let line = r#"{"type":"cmd","cmd":"list-panes"}"#;
        assert_eq!(decode_line(line, None).ok(), Some(Incoming::Command));
        // `type` de outro valor é status.
        assert!(matches!(
            decode_line(r#"{"type":"x","paneId":"t","st":"idle"}"#, None).ok(),
            Some(Incoming::Status(_))
        ));
    }

    #[test]
    fn token_exigido_barra_ausente_errado_e_nao_string() {
        let tok = Some("abc123");
        let ok = r#"{"paneId":"t","st":"idle","tok":"abc123"}"#;
        assert!(matches!(
            decode_line(ok, tok).ok(),
            Some(Incoming::Status(_))
        ));
        for line in [
            r#"{"paneId":"t","st":"idle"}"#,
            r#"{"paneId":"t","st":"idle","tok":"abc124"}"#,
            r#"{"paneId":"t","st":"idle","tok":"abc12"}"#,
            r#"{"paneId":"t","st":"idle","tok":123}"#,
        ] {
            assert_eq!(
                decode_line(line, tok).ok(),
                Some(Incoming::BadToken { command: false }),
                "{line}"
            );
        }
        assert_eq!(
            decode_line(r#"{"type":"cmd","cmd":"x"}"#, tok).ok(),
            Some(Incoming::BadToken { command: true })
        );
        assert_eq!(
            decode_line(r#"{"type":"cmd","cmd":"x","tok":"abc123"}"#, tok).ok(),
            Some(Incoming::Command)
        );
        // Sem token exigido, `tok` na linha é ignorado.
        assert!(matches!(
            decode_line(r#"{"paneId":"t","st":"idle","tok":"qualquer"}"#, None).ok(),
            Some(Incoming::Status(_))
        ));
    }

    #[test]
    fn token_so_e_checado_depois_de_json_e_objeto() {
        // Malformada continua erro mesmo com token exigido (o app logava).
        assert!(decode_line("lixo", Some("t")).is_err());
        assert_eq!(decode_line("[1]", Some("t")).ok(), Some(Incoming::Ignored));
    }

    #[test]
    fn ct_eq_compara_conteudo_e_tamanho() {
        assert!(ct_eq(b"", b""));
        assert!(ct_eq(b"abc", b"abc"));
        assert!(!ct_eq(b"abc", b"abd"));
        assert!(!ct_eq(b"abc", b"ab"));
    }
}
