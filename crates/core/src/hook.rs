//! Lógica pura do `gsh hook`: o envelope JSON que os harnesses (Claude Code,
//! Codex CLI) mandam no stdin vira status de turno, payload do socket e
//! registro da fazenda. Ler stdin, abrir socket e gravar arquivo é da CLI.
//!
//! Referência de comportamento: `docs/turn-status-hooks.md` do repo Flutter.

use serde_json::{json, Map, Value};

/// Campo string do envelope, `""` quando ausente ou nulo (o Dart fazia
/// `(json['x'] ?? '').toString()`); valor não-string vira sua forma JSON.
pub fn str_field(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => other.to_string(),
    }
}

/// Mapeia o evento de hook num status de turno, ou `None` se o evento não deve
/// mover o indicador. Cobre a união dos eventos do Claude Code e do Codex CLI.
///
/// - `Notification` só existe no Claude; `PermissionRequest` só no Codex (os
///   dois significam "precisa do usuário").
/// - `PreToolUse` de ferramenta que bloqueia esperando o usuário
///   (`AskUserQuestion`, `ExitPlanMode`) vira `waiting`: elas não emitem
///   `Notification`, e sem este desvio o app ficaria em `working` para sempre.
/// - Subagente e compactação (`Subagent*`, `*Compact`) são inertes de propósito:
///   o indicador representa a sessão principal.
pub fn status_for(event: &str, envelope: &Value) -> Option<&'static str> {
    match event {
        "UserPromptSubmit" | "PostToolUse" => Some("working"),
        "PreToolUse" => {
            const BLOCKING: [&str; 2] = ["AskUserQuestion", "ExitPlanMode"];
            let tool = str_field(envelope, "tool_name");
            Some(if BLOCKING.contains(&tool.as_str()) {
                "waiting"
            } else {
                "working"
            })
        }
        "Notification" => {
            // Cobre "precisa de aprovação" e "ocioso esperando input".
            let hint = format!(
                "{} {}",
                str_field(envelope, "notification_type"),
                str_field(envelope, "message")
            )
            .to_lowercase();
            Some(if hint.contains("idle") {
                "idle"
            } else {
                "waiting"
            })
        }
        "PermissionRequest" => Some("waiting"),
        "Stop" | "SessionStart" | "SessionEnd" => Some("idle"),
        _ => None,
    }
}

/// Payload que o hook manda pelo socket (sem `type`: é status, não comando).
///
/// `ev` é o evento cru: o app distingue início de turno de atividade mid-turn e
/// descarta um `working` tardio que chega depois do `idle`, porque cada hook é
/// um processo separado, sem ordem garantida. `hn` diz de quem veio, para
/// retomar a sessão com o comando certo.
pub fn status_payload(
    pane_id: &str,
    status: &str,
    event: &str,
    envelope: &Value,
    harness: &str,
    token: Option<&str>,
) -> Value {
    let mut payload = Map::new();
    payload.insert("paneId".into(), json!(pane_id));
    payload.insert("st".into(), json!(status));
    payload.insert("ev".into(), json!(event));
    payload.insert("sid".into(), json!(str_field(envelope, "session_id")));
    payload.insert("tx".into(), json!(str_field(envelope, "transcript_path")));
    payload.insert("hn".into(), json!(harness));
    // Só o Codex manda `turn_id`.
    let turn = str_field(envelope, "turn_id");
    if !turn.is_empty() {
        payload.insert("tid".into(), json!(turn));
    }
    let files = touched_files(event, envelope);
    if !files.is_empty() {
        payload.insert("files".into(), json!(files));
    }
    if let Some(tok) = token {
        payload.insert("tok".into(), json!(tok));
    }
    Value::Object(payload)
}

/// Lista de hosts protegidos (`~/.gsh/protected-hosts.json`, array de strings).
/// Conteúdo inválido = nenhum host.
pub fn protected_hosts(raw: &str) -> Vec<String> {
    let Ok(v) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Ferramentas que executam comando: o guard olha só o input delas (um `Read`
/// de um arquivo chamado prod.md não é acesso ao host).
const SHELL_TOOLS: [&str; 3] = ["Bash", "shell", "exec_command"];

/// O host protegido que o input da ferramenta cita, se algum. O host conta como
/// palavra: "prod" casa com "ssh prod", "user@prod" e "prod.x.com", mas não com
/// "production" nem "--prod".
pub fn guard_hit(envelope: &Value, hosts: &[String]) -> Option<String> {
    if hosts.is_empty() || !SHELL_TOOLS.contains(&str_field(envelope, "tool_name").as_str()) {
        return None;
    }
    let input = envelope
        .get("tool_input")
        .map(Value::to_string)
        .unwrap_or_default()
        .to_lowercase();
    let is_word = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    for host in hosts {
        let mut from = 0;
        while let Some(i) = input.get(from..).and_then(|rest| rest.find(host.as_str())) {
            let start = from + i;
            let end = start + host.len();
            let before_ok = input
                .get(..start)
                .and_then(|s| s.chars().next_back())
                .is_none_or(|c| !is_word(c));
            let after_ok = input
                .get(end..)
                .and_then(|s| s.chars().next())
                .is_none_or(|c| !is_word(c));
            if before_ok && after_ok {
                return Some(host.clone());
            }
            from = end;
        }
    }
    None
}

/// O que fazer com o arquivo da fazenda (`~/.gsh/farm/<sid>.json`).
#[derive(Debug, PartialEq)]
pub enum FarmAction {
    /// Nada a fazer (id inválido ou evento sem status).
    Skip,
    /// `SessionEnd`: apagar `file_name`.
    Remove { file_name: String },
    /// Gravar `body` em `file_name` (escrita atômica é da CLI).
    Write { file_name: String, body: String },
}

/// Decide o registro da fazenda. O id vira nome de arquivo, então só aceita o
/// formato de UUID/slug. `now_ms` é o relógio (ms desde a época), vindo de fora.
pub fn farm_action(
    sid: &str,
    event: &str,
    status: Option<&str>,
    harness: &str,
    now_ms: u64,
) -> FarmAction {
    if sid.is_empty()
        || !sid
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return FarmAction::Skip;
    }
    let file_name = format!("{sid}.json");
    if event == "SessionEnd" {
        return FarmAction::Remove { file_name };
    }
    match status {
        Some(st) => FarmAction::Write {
            file_name,
            body: json!({"st": st, "ev": event, "ts": now_ms, "hn": harness}).to_string(),
        },
        None => FarmAction::Skip,
    }
}

const PATCH_BEGIN: &str = "*** Begin Patch";

/// Caminhos **absolutos** que um `PostToolUse` de edição tocou. Claude:
/// `Edit`/`Write`/`MultiEdit` (`file_path`) e `NotebookEdit` (`notebook_path`).
/// Codex: o patch do `apply_patch` (cabeçalhos `*** Add/Update/Delete File:` e
/// `*** Move to:`), achado em qualquer string do `tool_input`. Relativo resolve
/// contra o `cwd` do envelope; formato desconhecido = vazio (nunca falha).
pub fn touched_files(event: &str, envelope: &Value) -> Vec<String> {
    if event != "PostToolUse" {
        return Vec::new();
    }
    let input = envelope.get("tool_input").unwrap_or(&Value::Null);
    let raw: Vec<String> = match str_field(envelope, "tool_name").as_str() {
        "Edit" | "Write" | "MultiEdit" => str_at(input, "file_path"),
        "NotebookEdit" => str_at(input, "notebook_path"),
        _ => {
            let mut out = Vec::new();
            patch_paths(input, &mut out);
            out
        }
    };
    let cwd = str_field(envelope, "cwd");
    let mut files: Vec<String> = Vec::new();
    for p in raw {
        let path = std::path::Path::new(&p);
        let abs = if path.is_absolute() {
            p.clone()
        } else if !cwd.is_empty() {
            std::path::Path::new(&cwd)
                .join(path)
                .to_string_lossy()
                .into_owned()
        } else {
            continue; // sem cwd não dá pra ancorar
        };
        if !files.contains(&abs) {
            files.push(abs);
        }
    }
    files
}

fn str_at(v: &Value, key: &str) -> Vec<String> {
    match v.get(key).and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => vec![s.trim().to_string()],
        _ => Vec::new(),
    }
}

/// Varre o `tool_input` atrás de um patch do Codex (string que começa com
/// `*** Begin Patch`, ou um `apply_patch <<EOF` de shell) e coleta os caminhos.
fn patch_paths(v: &Value, out: &mut Vec<String>) {
    match v {
        Value::String(s) => {
            let Some(start) = s.find(PATCH_BEGIN) else {
                return;
            };
            let before = s.get(..start).unwrap_or_default();
            if !(before.trim().is_empty() || before.contains("apply_patch")) {
                return;
            }
            for line in s.get(start..).unwrap_or_default().lines() {
                let line = line.trim_end_matches('\r');
                for prefix in [
                    "*** Add File: ",
                    "*** Update File: ",
                    "*** Delete File: ",
                    "*** Move to: ",
                ] {
                    if let Some(p) = line.strip_prefix(prefix) {
                        if !p.trim().is_empty() {
                            out.push(p.trim().to_string());
                        }
                    }
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|i| patch_paths(i, out)),
        Value::Object(map) => map.values().for_each(|i| patch_paths(i, out)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {

    /// Caminho absoluto do SO da máquina (`/x` no Unix, `C:\x` no Windows),
    /// para os testes valerem nos três sistemas.
    fn abs(p: &str) -> String {
        if cfg!(windows) {
            format!("C:{}", p.replace('/', "\\"))
        } else {
            p.to_string()
        }
    }

    /// Caminho relativo ancorado no `cwd`, do mesmo jeito que o código faz.
    fn joined(cwd: &str, rel: &str) -> String {
        std::path::Path::new(&abs(cwd))
            .join(rel)
            .to_string_lossy()
            .into_owned()
    }

    use super::*;

    #[test]
    fn hosts_protegidos_bloqueiam_so_comando_que_cita_o_host() {
        let hosts = vec!["prod".to_string(), "homolog.gorila.com".to_string()];
        let cmd = |c: &str| json!({"tool_name": "Bash", "tool_input": {"command": c}});
        assert_eq!(guard_hit(&cmd("ssh prod"), &hosts).as_deref(), Some("prod"));
        assert_eq!(
            guard_hit(&cmd("scp a.txt deploy@prod.gorila.com:/x"), &hosts).as_deref(),
            Some("prod")
        );
        assert_eq!(
            guard_hit(
                &cmd("gsh new-workspace . --host homolog.gorila.com"),
                &hosts
            )
            .as_deref(),
            Some("homolog.gorila.com")
        );
        assert_eq!(guard_hit(&cmd("npm run build --prod"), &hosts), None);
        assert_eq!(guard_hit(&cmd("echo production"), &hosts), None);
        // Só ferramentas de shell: ler um arquivo com o nome não é acesso.
        let read = json!({"tool_name": "Read", "tool_input": {"file_path": "/docs/prod.md"}});
        assert_eq!(guard_hit(&read, &hosts), None);
        // Codex: input com argv.
        let codex = json!({"tool_name": "shell", "tool_input": {"argv": ["ssh", "prod"]}});
        assert_eq!(guard_hit(&codex, &hosts).as_deref(), Some("prod"));
        assert_eq!(guard_hit(&cmd("ssh prod"), &[]), None);
    }

    #[test]
    fn hosts_protegidos_normaliza_e_ignora_lixo() {
        assert_eq!(
            protected_hosts(r#"[" Prod ", "", 3, "x.y"]"#),
            ["prod", "x.y"]
        );
        assert!(protected_hosts("lixo").is_empty());
        assert!(protected_hosts(r#"{"a":1}"#).is_empty());
    }

    #[test]
    fn fazenda_grava_remove_e_recusa_id_que_nao_e_slug() {
        let w = farm_action("abc-123", "Notification", Some("waiting"), "claude", 42);
        let FarmAction::Write { file_name, body } = w else {
            panic!("esperava Write");
        };
        assert_eq!(file_name, "abc-123.json");
        let v: Value = serde_json::from_str(&body).unwrap();
        assert_eq!(
            v,
            json!({"st":"waiting","ev":"Notification","ts":42,"hn":"claude"})
        );
        assert_eq!(
            farm_action("abc-123", "SubagentStart", None, "claude", 1),
            FarmAction::Skip
        );
        assert_eq!(
            farm_action("abc-123", "SessionEnd", Some("idle"), "claude", 1),
            FarmAction::Remove {
                file_name: "abc-123.json".into()
            }
        );
        for bad in ["", "../x", "a/b", "a b", "á"] {
            assert_eq!(
                farm_action(bad, "Stop", Some("idle"), "claude", 1),
                FarmAction::Skip,
                "{bad:?}"
            );
        }
    }

    #[test]
    fn eventos_de_trabalho() {
        assert_eq!(status_for("UserPromptSubmit", &json!({})), Some("working"));
        assert_eq!(status_for("PostToolUse", &json!({})), Some("working"));
    }

    #[test]
    fn pretooluse_bloqueante_vira_waiting() {
        let bloqueia = json!({"tool_name": "AskUserQuestion"});
        assert_eq!(status_for("PreToolUse", &bloqueia), Some("waiting"));
        let plano = json!({"tool_name": "ExitPlanMode"});
        assert_eq!(status_for("PreToolUse", &plano), Some("waiting"));
        let comum = json!({"tool_name": "Bash"});
        assert_eq!(status_for("PreToolUse", &comum), Some("working"));
        assert_eq!(status_for("PreToolUse", &json!({})), Some("working"));
    }

    #[test]
    fn notification_distingue_idle_de_waiting() {
        let idle = json!({"message": "Claude is idle waiting for input"});
        assert_eq!(status_for("Notification", &idle), Some("idle"));
        let tipo_idle = json!({"notification_type": "IDLE"});
        assert_eq!(status_for("Notification", &tipo_idle), Some("idle"));
        let aprovacao = json!({"message": "needs your approval"});
        assert_eq!(status_for("Notification", &aprovacao), Some("waiting"));
    }

    #[test]
    fn fim_de_turno_e_sessao_sao_idle() {
        for ev in ["Stop", "SessionStart", "SessionEnd"] {
            assert_eq!(status_for(ev, &json!({})), Some("idle"));
        }
    }

    #[test]
    fn evento_desconhecido_nao_move_indicador() {
        assert_eq!(status_for("", &json!({})), None);
        assert_eq!(status_for("Whatever", &json!({})), None);
    }

    #[test]
    fn permission_request_do_codex_e_waiting() {
        let ev = json!({"hook_event_name": "PermissionRequest", "tool_name": "shell"});
        assert_eq!(status_for("PermissionRequest", &ev), Some("waiting"));
    }

    #[test]
    fn subagente_e_compactacao_do_codex_sao_inertes() {
        for ev in ["SubagentStart", "SubagentStop", "PreCompact", "PostCompact"] {
            assert_eq!(
                status_for(ev, &json!({})),
                None,
                "{ev} não deve mover a aba"
            );
        }
    }

    #[test]
    fn eventos_do_codex_cobrem_o_ciclo_de_turno() {
        let ciclo = [
            ("SessionStart", "idle"),
            ("UserPromptSubmit", "working"),
            ("PreToolUse", "working"),
            ("PostToolUse", "working"),
            ("PermissionRequest", "waiting"),
            ("Stop", "idle"),
            ("SessionEnd", "idle"),
        ];
        for (ev, esperado) in ciclo {
            assert_eq!(status_for(ev, &json!({})), Some(esperado), "evento {ev}");
        }
    }

    #[test]
    fn payload_minimo_tem_so_os_campos_do_fio() {
        let env = json!({"session_id": "s1", "transcript_path": "/t.jsonl"});
        let p = status_payload("t3", "idle", "Stop", &env, "claude", None);
        assert_eq!(
            p.to_string(),
            r#"{"ev":"Stop","hn":"claude","paneId":"t3","sid":"s1","st":"idle","tx":"/t.jsonl"}"#
        );
    }

    #[test]
    fn payload_leva_turn_id_arquivos_e_token_quando_existem() {
        let env = json!({"session_id": "s", "turn_id": "u1", "cwd": abs("/r"),
            "tool_name": "Write", "tool_input": {"file_path": "a.rs"}});
        let p = status_payload("t3", "working", "PostToolUse", &env, "codex", Some("tk"));
        assert_eq!(p["tid"], "u1");
        assert_eq!(p["files"], json!([joined("/r", "a.rs")]));
        assert_eq!(p["tok"], "tk");
        assert_eq!(p["hn"], "codex");
    }

    #[test]
    fn touched_files_do_claude() {
        let edit = json!({"tool_name": "Edit", "cwd": abs("/repo"),
            "tool_input": {"file_path": abs("/repo/a.rs"), "old_string": "x"}});
        assert_eq!(touched_files("PostToolUse", &edit), vec![abs("/repo/a.rs")]);
        let write = json!({"tool_name": "Write", "cwd": abs("/repo"),
            "tool_input": {"file_path": "src/b.rs"}});
        assert_eq!(
            touched_files("PostToolUse", &write),
            vec![joined("/repo", "src/b.rs")]
        );
        let multi = json!({"tool_name": "MultiEdit", "tool_input": {"file_path": abs("/r/c.rs")}});
        assert_eq!(touched_files("PostToolUse", &multi), vec![abs("/r/c.rs")]);
        let nb = json!({"tool_name": "NotebookEdit",
            "tool_input": {"notebook_path": abs("/r/n.ipynb")}});
        assert_eq!(touched_files("PostToolUse", &nb), vec![abs("/r/n.ipynb")]);
        // PreToolUse não conta: o arquivo ainda não mudou.
        assert!(touched_files("PreToolUse", &edit).is_empty());
    }

    #[test]
    fn touched_files_do_patch_do_codex() {
        let old = abs("/abs/old.rs");
        let patch = format!(
            "*** Begin Patch\n*** Add File: new.txt\n+oi\n\
            *** Update File: src/a.rs\n*** Move to: src/b.rs\n@@\n-x\n+y\n\
            *** Delete File: {old}\n*** End Patch\n"
        );
        let ev = json!({"tool_name": "apply_patch", "cwd": abs("/repo"),
            "tool_input": {"command": patch}});
        assert_eq!(
            touched_files("PostToolUse", &ev),
            vec![
                joined("/repo", "new.txt"),
                joined("/repo", "src/a.rs"),
                joined("/repo", "src/b.rs"),
                old.clone(),
            ]
        );
        let shell = json!({"tool_name": "shell", "cwd": abs("/repo"),
            "tool_input": {"command": ["apply_patch", patch]}});
        assert_eq!(touched_files("PostToolUse", &shell).len(), 4);
    }

    #[test]
    fn touched_files_formato_desconhecido_e_vazio() {
        for ev in [
            json!({}),
            json!({"tool_name": "Bash", "tool_input": {"command": "ls"}}),
            json!({"tool_name": "Edit", "tool_input": "lixo"}),
            json!({"tool_name": "Write", "tool_input": {"file_path": "rel.rs"}}), // sem cwd
            json!({"tool_name": "Bash", "tool_input": {"command": "echo '*** Begin Patch'"}}),
        ] {
            assert!(touched_files("PostToolUse", &ev).is_empty(), "{ev}");
        }
    }

    #[test]
    fn str_field_normaliza_ausente_e_nao_string() {
        assert_eq!(str_field(&json!({}), "x"), "");
        assert_eq!(str_field(&json!({"x": null}), "x"), "");
        assert_eq!(str_field(&json!({"x": "v"}), "x"), "v");
        assert_eq!(str_field(&json!({"x": 7}), "x"), "7");
    }
}
