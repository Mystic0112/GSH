//! Motor de chat com o Claude Code em `stream-json` (sem UI, sem `iced`).
//!
//! - [`protocol`]: formatação de entrada (`stdin`) e parse puro de linhas `stream-json` (`stdout`).
//! - [`engine`]: spawn do processo filho, escrita no `stdin`, leitura assíncrona em thread e
//!   cancelamento matando o processo filho.
//! - [`error`]: erros tipados com `thiserror`.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod engine;
pub mod error;
pub mod protocol;

pub use engine::{ChatConfig, ChatEngine};
pub use error::ChatError;
pub use protocol::{
    format_user_message_line, parse_stream_line, tool_summary, ChatEvent, DEFAULT_CLAUDE_ARGS,
    DEFAULT_CLAUDE_BINARY,
};

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    use serde_json::json;

    use super::*;

    static SCRIPT_SEQ: AtomicU64 = AtomicU64::new(1);

    #[cfg(unix)]
    fn write_fake_claude_script(body: &str) -> (PathBuf, PathBuf) {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let seq = SCRIPT_SEQ.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("gsh-chat-fake-claude-{}-{seq}", std::process::id()));
        fs::create_dir_all(&dir).expect("create temp script dir");
        let script_path = dir.join("fake-claude.sh");
        let content = format!("#!/bin/sh\n{body}\n");
        {
            let mut file = fs::File::create(&script_path).expect("create fake script");
            file.write_all(content.as_bytes())
                .expect("write fake script");
            file.sync_all().expect("sync fake script");
        }
        let mut perms = fs::metadata(&script_path)
            .expect("script metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script_path, perms).expect("chmod +x fake script");
        (dir, script_path)
    }

    /// Sobe o script falso, repetindo se o exec cair em `ETXTBSY`: outro teste
    /// em paralelo pode ter herdado o fd de escrita do script durante o fork.
    #[cfg(unix)]
    fn spawn_fake(script: &std::path::Path) -> ChatEngine {
        for _ in 0..50 {
            match ChatEngine::spawn(ChatConfig::new().with_binary(script)) {
                Ok(engine) => return engine,
                Err(ChatError::SpawnFailed { cause, .. }) if cause.contains("Text file busy") => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(err) => panic!("spawn fake claude: {err}"),
            }
        }
        panic!("spawn fake claude: ETXTBSY persistente");
    }

    #[test]
    fn parse_linhas_validas_delta_texto_tool_use_tool_result_e_resultado_final() {
        // 1. system/init -> Ready
        let init_line = r#"{"type":"system","subtype":"init","session_id":"sess-1","model":"claude-sonnet-4-5","permissionMode":"default"}"#;
        assert_eq!(
            parse_stream_line(init_line),
            Ok(vec![ChatEvent::Ready {
                session_id: "sess-1".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                permission_mode: Some("default".to_string()),
            }])
        );

        // 2. stream_event/text_delta -> TextDelta
        let delta_line = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Olá, "}}}"#;
        assert_eq!(
            parse_stream_line(delta_line),
            Ok(vec![ChatEvent::TextDelta("Olá, ".to_string())])
        );

        // 3. assistant com texto + tool_use
        let assistant_line = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Olá, mundo!"},{"type":"tool_use","id":"toolu_01","name":"Bash","input":{"command":"cargo test --workspace","description":"Roda os testes"}}]}}"#;
        assert_eq!(
            parse_stream_line(assistant_line),
            Ok(vec![
                ChatEvent::AssistantText("Olá, mundo!".to_string()),
                ChatEvent::ToolUse {
                    id: "toolu_01".to_string(),
                    name: "Bash".to_string(),
                    summary: "cargo test --workspace".to_string(),
                    input: json!({
                        "command": "cargo test --workspace",
                        "description": "Roda os testes"
                    }),
                },
            ])
        );

        // 4. user com tool_result
        let tool_res_line = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"toolu_01","is_error":false,"content":"test result: ok"}]}}"#;
        assert_eq!(
            parse_stream_line(tool_res_line),
            Ok(vec![ChatEvent::ToolResult {
                tool_use_id: "toolu_01".to_string(),
                is_error: false,
                content: "test result: ok".to_string(),
            }])
        );

        // 5. result final
        let result_line = r#"{"type":"result","subtype":"success","is_error":false,"result":"Olá, mundo!","session_id":"sess-1","total_cost_usd":0.0125}"#;
        assert_eq!(
            parse_stream_line(result_line),
            Ok(vec![ChatEvent::Result {
                is_error: false,
                result: "Olá, mundo!".to_string(),
                session_id: "sess-1".to_string(),
                cost_usd: Some(0.0125),
            }])
        );
    }

    #[test]
    fn parse_ignora_eventos_desconhecidos_e_linhas_vazias_sem_erro() {
        assert_eq!(parse_stream_line(""), Ok(Vec::new()));
        assert_eq!(parse_stream_line("   \n  "), Ok(Vec::new()));

        // Eventos desconhecidos do protocolo devem ser ignorados sem erro
        let unknown_hook = r#"{"type":"system","subtype":"hook_started","hook_name":"PreToolUse"}"#;
        assert_eq!(parse_stream_line(unknown_hook), Ok(Vec::new()));

        let unknown_rate = r#"{"type":"rate_limit_event","retry_after":5}"#;
        assert_eq!(parse_stream_line(unknown_rate), Ok(Vec::new()));

        // Eco de mensagem do próprio usuário (isReplay: true sem peer) é ignorado
        let replay = r#"{"type":"user","isReplay":true,"message":{"role":"user","content":"oi"}}"#;
        assert_eq!(parse_stream_line(replay), Ok(Vec::new()));
    }

    #[test]
    fn parse_retorna_erro_tipado_para_linha_invalida_ou_json_nao_objeto() {
        let malformed = parse_stream_line("{nao eh json valido");
        assert!(matches!(
            malformed,
            Err(ChatError::InvalidJsonLine { ref line, .. }) if line == "{nao eh json valido"
        ));

        let non_object = parse_stream_line("[1, 2, 3]");
        assert!(matches!(
            non_object,
            Err(ChatError::InvalidJsonLine { ref line, .. }) if line == "[1, 2, 3]"
        ));
    }

    #[test]
    fn format_user_message_line_gera_json_esperado_pelo_claude_stream_json() {
        let line = format_user_message_line("explica o main.rs");
        let val: serde_json::Value = serde_json::from_str(&line).expect("valid json");
        assert_eq!(
            val,
            json!({
                "type": "user",
                "message": {
                    "role": "user",
                    "content": "explica o main.rs"
                }
            })
        );
    }

    #[test]
    fn binario_inexistente_retorna_erro_binary_not_found() {
        let cfg = ChatConfig::new().with_binary("/caminho/inexistente/claude-nao-existe-99999");
        let res = ChatEngine::spawn(cfg);
        assert!(matches!(res, Err(ChatError::BinaryNotFound { .. })));
    }

    #[cfg(unix)]
    #[test]
    fn motor_envia_json_no_stdin_e_recebe_stream_completo_de_script_falso() {
        let (dir, script) = write_fake_claude_script(
            r#"
# Confirma que os argumentos padrão do stream-json foram passados
case "$*" in
  *"-p --input-format stream-json --output-format stream-json --verbose"*) ;;
  *) echo "args inesperados: $*" >&2; exit 2 ;;
esac

echo '{"type":"system","subtype":"init","session_id":"fake-s1","model":"claude-sonnet-4-5"}'
while IFS= read -r line; do
  echo '{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"Recebi: "}}}'
  echo '{"type":" assistant_unknown_ignored"}' >/dev/null
  echo '{"type":"rate_limit_unknown"}'
  echo '{"type":"assistant","message":{"content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/tmp/a.rs"}}]}}'
  echo '{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","is_error":false,"content":"fn main() {}"}]}}'
  echo '{"type":"assistant","message":{"content":[{"type":"text","text":"Recebi: tudo certo"}]}}'
  echo '{"type":"result","is_error":false,"result":"Recebi: tudo certo","session_id":"fake-s1","total_cost_usd":0.005}'
done
"#,
        );

        let mut engine = spawn_fake(&script);
        engine
            .send_user_message("oi claude")
            .expect("send user message");

        let mut collected = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if let Some(ev) = engine.recv_timeout(Duration::from_millis(50)) {
                let done = matches!(ev, ChatEvent::Result { .. });
                collected.push(ev);
                if done {
                    break;
                }
            }
        }

        assert_eq!(
            collected,
            vec![
                ChatEvent::Ready {
                    session_id: "fake-s1".to_string(),
                    model: "claude-sonnet-4-5".to_string(),
                    permission_mode: None,
                },
                ChatEvent::TextDelta("Recebi: ".to_string()),
                ChatEvent::ToolUse {
                    id: "t1".to_string(),
                    name: "Read".to_string(),
                    summary: "/tmp/a.rs".to_string(),
                    input: json!({"file_path": "/tmp/a.rs"}),
                },
                ChatEvent::ToolResult {
                    tool_use_id: "t1".to_string(),
                    is_error: false,
                    content: "fn main() {}".to_string(),
                },
                ChatEvent::AssistantText("Recebi: tudo certo".to_string()),
                ChatEvent::Result {
                    is_error: false,
                    result: "Recebi: tudo certo".to_string(),
                    session_id: "fake-s1".to_string(),
                    cost_usd: Some(0.005),
                },
            ]
        );

        engine.cancel();
        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn cancelar_mata_processo_filho_durante_stream() {
        let (dir, script) = write_fake_claude_script(
            r#"
while IFS= read -r line; do
  echo '{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"parte 1..."}}}'
  sleep 30
done
"#,
        );

        let mut engine = spawn_fake(&script);
        engine.send_user_message("demora").expect("send");

        let first = engine
            .recv_timeout(Duration::from_secs(3))
            .expect("receive first delta");
        assert_eq!(first, ChatEvent::TextDelta("parte 1...".to_string()));
        assert!(engine.is_running());

        // Cancela: deve matar o processo filho imediatamente sem esperar o sleep 30
        let cancel_start = Instant::now();
        engine.cancel();
        assert!(cancel_start.elapsed() < Duration::from_secs(2));
        assert!(!engine.is_running());
        assert_eq!(
            engine.send_user_message("apos cancel"),
            Err(ChatError::SessionClosed)
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn queda_do_processo_filho_emite_erro_process_exited_com_stderr() {
        let (dir, script) = write_fake_claude_script(
            r#"
IFS= read -r line
echo "falha fatal simulada" >&2
exit 17
"#,
        );

        let mut engine = spawn_fake(&script);
        engine.send_user_message("vai cair").expect("send");

        let ev = engine
            .recv_timeout(Duration::from_secs(3))
            .expect("receive process exit error");
        assert_eq!(
            ev,
            ChatEvent::Error(ChatError::ProcessExited {
                exit_code: Some(17),
                stderr: "falha fatal simulada".to_string(),
            })
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
