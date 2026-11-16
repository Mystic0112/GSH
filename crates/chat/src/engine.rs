//! Motor de execução do processo `claude` em `stream-json` (`ClaudeStreamEngine` do Flutter).
//!
//! Spawna o binário com `stdin`/`stdout`/`stderr` em pipe, escreve as linhas do protocolo
//! (mensagem do usuário, `control_request`, resposta de permissão), lê o `stdout` linha a
//! linha numa thread e entrega [`ChatEvent`] num canal. O handle é barato de clonar: a UI
//! passa uma cópia para tarefas em segundo plano (`send` bloqueia enquanto lê e codifica os
//! anexos). O processo morre quando a última cópia some.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

use serde_json::{Map, Value};

use crate::account::{parse_commands, parse_models, parse_plan_usage};
use crate::attachment::{build_user_content, Attachment};
use crate::error::ChatError;
use crate::event::ChatEvent;
use crate::json::{obj_of, str_of, string_of};
use crate::protocol::{
    add_rules, get_usage_line, initialize_line, interrupt_line, launch_args, parse_json_line,
    parse_stream_object, permission_response_line, set_effort_line, set_model_line,
    set_permission_mode_line, user_message_line, LaunchOptions, INIT_REQUEST_ID,
    MODEL_REQUEST_PREFIX, USAGE_REQUEST_PREFIX,
};

/// Quantidade máxima de linhas finais do `stderr` retidas para diagnóstico.
const MAX_STDERR_LINES: usize = 20;

/// Configuração de inicialização de uma conversa.
#[derive(Debug, Clone)]
pub struct ChatConfig {
    /// Executável do `claude` (a UI resolve com [`crate::resolve_executable`]).
    pub binary: PathBuf,
    /// Pasta de trabalho da conversa.
    pub cwd: Option<PathBuf>,
    /// Ambiente extra (o da CLI `gsh`: aba, socket e PATH).
    pub env: Vec<(String, String)>,
    pub launch: LaunchOptions,
}

impl ChatConfig {
    pub fn new(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
            cwd: None,
            env: Vec::new(),
            launch: LaunchOptions::default(),
        }
    }
}

/// Pedido em aberto: o input da ferramenta (`allow` devolve em `updatedInput`) e a sugestão
/// `addRules` que "sempre permitir" manda em `updatedPermissions`.
type Asked = (Map<String, Value>, Option<Value>);

struct Inner {
    child: Mutex<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    events: Mutex<Receiver<ChatEvent>>,
    asked: Arc<Mutex<HashMap<String, Asked>>>,
    closed: Arc<AtomicBool>,
    seq: AtomicU64,
}

/// Conversa viva com o Claude Code: fica de pé entre turnos; cada [`ChatEngine::send`] é um
/// turno novo na mesma sessão.
#[derive(Clone)]
pub struct ChatEngine {
    inner: Arc<Inner>,
}

impl std::fmt::Debug for ChatEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatEngine").finish_non_exhaustive()
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // Um panic em outra thread não corrompe estes dados (só filas e flags): segue.
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl ChatEngine {
    /// Sobe o `claude` e manda o `initialize` (lista de modelos e comandos da conta).
    pub fn spawn(config: ChatConfig) -> Result<Self, ChatError> {
        let program = config.binary.display().to_string();
        let mut cmd = Command::new(&config.binary);
        cmd.args(launch_args(&config.launch))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(dir) = config.cwd.as_ref().filter(|d| d.is_dir()) {
            cmd.current_dir(dir);
        }
        for (k, v) in crate::resolve::env_with_dir_on_path(&config.binary, &config.env) {
            cmd.env(k, v);
        }
        let mut child = cmd.spawn().map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                ChatError::BinaryNotFound {
                    program: program.clone(),
                    cause: e.to_string(),
                }
            } else {
                ChatError::SpawnFailed {
                    program: program.clone(),
                    cause: e.to_string(),
                }
            }
        })?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ChatError::StdioUnavailable);
        };
        let stderr_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        if let Some(pipe) = child.stderr.take() {
            let lines = Arc::clone(&stderr_lines);
            thread::spawn(move || {
                for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                    let mut g = lock(&lines);
                    if g.len() >= MAX_STDERR_LINES {
                        g.remove(0);
                    }
                    g.push(line);
                }
            });
        }
        let (tx, rx) = mpsc::channel();
        let closed = Arc::new(AtomicBool::new(false));
        let asked: Arc<Mutex<HashMap<String, Asked>>> = Arc::new(Mutex::new(HashMap::new()));
        let inner = Arc::new(Inner {
            child: Mutex::new(child),
            stdin: Mutex::new(Some(stdin)),
            events: Mutex::new(rx),
            asked: Arc::clone(&asked),
            closed: Arc::clone(&closed),
            seq: AtomicU64::new(0),
        });
        let engine = Self { inner };
        // O leitor só segura referências fracas ao processo: o último handle mata o filho
        // (`Drop`), e aí o stdout fecha e a thread termina.
        spawn_reader(
            stdout,
            tx,
            asked,
            closed,
            Arc::downgrade(&engine.inner),
            stderr_lines,
        );
        engine.write_line(&initialize_line())?;
        Ok(engine)
    }

    fn next_seq(&self) -> u64 {
        self.inner.seq.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::SeqCst)
    }

    fn write_line(&self, line: &str) -> Result<(), ChatError> {
        if self.is_closed() {
            return Err(ChatError::SessionClosed);
        }
        let mut guard = lock(&self.inner.stdin);
        let Some(stdin) = guard.as_mut() else {
            return Err(ChatError::SessionClosed);
        };
        writeln!(stdin, "{line}")
            .and_then(|()| stdin.flush())
            .map_err(|e| ChatError::WriteStdin {
                cause: e.to_string(),
            })
    }

    /// Manda uma mensagem do usuário (começa um turno), com anexos. Lê e codifica os anexos:
    /// chame fora da thread da UI. Falha com [`ChatError::AttachmentTooLarge`] se um anexo
    /// passa do teto da API.
    pub fn send(&self, text: &str, attachments: &[Attachment]) -> Result<(), ChatError> {
        let content = build_user_content(text, attachments)?;
        self.write_line(&user_message_line(&content))
    }

    /// Responde um [`ChatEvent::PermissionAsk`]. `always` grava a regra sugerida; `input`
    /// substitui o input da ferramenta no `allow` (é como as respostas do `AskUserQuestion`
    /// voltam); `message` explica um `deny` ao agente.
    pub fn answer_permission(
        &self,
        request_id: &str,
        allow: bool,
        always: bool,
        input: Option<Map<String, Value>>,
        message: Option<&str>,
    ) {
        let (asked_input, rule) = lock(&self.inner.asked)
            .remove(request_id)
            .unwrap_or_default();
        let rule = if always { rule } else { None };
        let line = permission_response_line(
            request_id,
            allow,
            rule.as_ref(),
            Value::Object(input.unwrap_or(asked_input)),
            message,
        );
        // Processo já morto: o `EngineFailed` do leitor avisa a UI.
        let _ = self.write_line(&line);
    }

    /// Troca o modo de permissão da sessão (`plan` = só planeja).
    pub fn set_permission_mode(&self, mode: &str) {
        let _ = self.write_line(&set_permission_mode_line(self.next_seq(), mode));
    }

    /// Troca o modelo da conversa (um `ModelOption::value`).
    pub fn set_model(&self, model: &str) {
        let _ = self.write_line(&set_model_line(self.next_seq(), model));
    }

    /// Troca o esforço da conversa (`low`...`max`).
    pub fn set_effort(&self, level: &str) {
        let _ = self.write_line(&set_effort_line(self.next_seq(), level));
    }

    /// Pede os limites do plano; a resposta chega como [`ChatEvent::PlanUsage`].
    pub fn request_usage(&self) {
        let _ = self.write_line(&get_usage_line(self.next_seq()));
    }

    /// Interrompe o turno em andamento.
    pub fn interrupt(&self) {
        let _ = self.write_line(&interrupt_line(self.next_seq()));
    }

    /// Drena todos os eventos disponíveis, sem bloquear.
    pub fn drain_events(&self) -> Vec<ChatEvent> {
        let rx = lock(&self.inner.events);
        let mut out = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            out.push(ev);
        }
        out
    }

    /// Espera o próximo evento até `timeout` (útil em testes).
    pub fn recv_timeout(&self, timeout: Duration) -> Option<ChatEvent> {
        lock(&self.inner.events).recv_timeout(timeout).ok()
    }

    /// Encerra a conversa: fecha o stdin e mata o processo.
    pub fn close(&self) {
        self.inner.close();
    }

    /// O processo ainda está de pé?
    pub fn is_running(&self) -> bool {
        !self.is_closed() && matches!(lock(&self.inner.child).try_wait(), Ok(None))
    }
}

impl Inner {
    fn close(&self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return;
        }
        drop(lock(&self.stdin).take());
        let mut child = lock(&self.child);
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for Inner {
    fn drop(&mut self) {
        self.close();
    }
}

fn spawn_reader(
    stdout: std::process::ChildStdout,
    tx: Sender<ChatEvent>,
    asked: Arc<Mutex<HashMap<String, Asked>>>,
    closed: Arc<AtomicBool>,
    inner: std::sync::Weak<Inner>,
    stderr_lines: Arc<Mutex<Vec<String>>>,
) {
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            // Linha que não é JSON: o Dart ignora.
            let Ok(Some(o)) = parse_json_line(&line) else {
                continue;
            };
            let events = if str_of(&o, "type") == Some("control_response") {
                on_control_response(&o)
            } else {
                parse_stream_object(&o)
            };
            for e in events {
                if let ChatEvent::PermissionAsk {
                    request_id, input, ..
                } = &e
                {
                    let rule = o.get("request").and_then(add_rules);
                    lock(&asked).insert(request_id.clone(), (input.clone(), rule));
                }
                if tx.send(e).is_err() {
                    return;
                }
            }
        }
        // stdout fechou. Sem a gente ter pedido, o motor caiu.
        let code = inner
            .upgrade()
            .and_then(|i| lock(&i.child).wait().ok())
            .and_then(|status| status.code());
        if !closed.load(Ordering::SeqCst) {
            let stderr = lock(&stderr_lines).join("\n");
            let _ = tx.send(ChatEvent::EngineFailed {
                exit_code: code,
                stderr,
            });
        }
    });
}

/// Respostas aos nossos `control_request` (init, troca de modelo, uso).
fn on_control_response(o: &Map<String, Value>) -> Vec<ChatEvent> {
    let Some(r) = obj_of(o, "response") else {
        return Vec::new();
    };
    let id = string_of(r, "request_id");
    let body = r.get("response").cloned().unwrap_or(Value::Null);
    let mut out = Vec::new();
    if id == INIT_REQUEST_ID {
        let models = parse_models(&body);
        if !models.is_empty() {
            out.push(ChatEvent::Models(models));
        }
        let commands = parse_commands(&body);
        if !commands.is_empty() {
            out.push(ChatEvent::Commands(commands));
        }
    } else if id.starts_with(MODEL_REQUEST_PREFIX) {
        if let Some(m) = body
            .get("model")
            .and_then(Value::as_str)
            .filter(|m| !m.is_empty())
        {
            out.push(ChatEvent::ModelChanged(m.to_string()));
        }
    } else if id.starts_with(USAGE_REQUEST_PREFIX) {
        out.push(ChatEvent::PlanUsage(parse_plan_usage(&body)));
    }
    out
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::sync::atomic::AtomicU64;
    use std::time::Instant;

    use serde_json::json;

    use super::*;

    static SEQ: AtomicU64 = AtomicU64::new(1);

    /// Script que age como o `claude`: guarda o que recebe em `log` e responde.
    fn fake_claude(body: &str) -> (PathBuf, PathBuf, PathBuf) {
        use std::os::unix::fs::PermissionsExt;
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("gsh-chat-fake-{}-{n}", std::process::id()));
        fs::create_dir_all(&dir).expect("dir");
        let script = dir.join("claude");
        let log = dir.join("stdin.log");
        let src = format!("#!/bin/sh\nLOG='{}'\n{body}\n", log.display());
        fs::write(&script, src).expect("script");
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("chmod");
        (dir, script, log)
    }

    /// Repete o spawn se o exec cair em `ETXTBSY` (outro teste herdou o fd de escrita).
    fn spawn(script: &Path) -> ChatEngine {
        for _ in 0..50 {
            match ChatEngine::spawn(ChatConfig::new(script)) {
                Ok(e) => return e,
                Err(ChatError::SpawnFailed { cause, .. }) if cause.contains("Text file busy") => {
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => panic!("spawn: {e}"),
            }
        }
        panic!("ETXTBSY persistente");
    }

    fn collect_until(e: &ChatEngine, stop: impl Fn(&ChatEvent) -> bool) -> Vec<ChatEvent> {
        let mut got = Vec::new();
        let end = Instant::now() + Duration::from_secs(5);
        while Instant::now() < end {
            if let Some(ev) = e.recv_timeout(Duration::from_millis(50)) {
                let done = stop(&ev);
                got.push(ev);
                if done {
                    break;
                }
            }
        }
        got
    }

    const SCRIPT: &str = r#"
while IFS= read -r line; do
  echo "$line" >> "$LOG"
  case "$line" in
    *gsh-init*)
      echo '{"type":"control_response","response":{"subtype":"success","request_id":"gsh-init","response":{"models":[{"value":"opus","displayName":"Opus"}],"commands":[{"name":"review","description":"Revisa"}]}}}' ;;
    *gsh-usage*)
      echo '{"type":"control_response","response":{"subtype":"success","request_id":"gsh-usage-1","response":{"limits":[{"kind":"session","percent":7}]}}}' ;;
    *gsh-model*)
      echo '{"type":"control_response","response":{"subtype":"success","request_id":"gsh-model-1","response":{"model":"claude-opus-5-5"}}}' ;;
    *'"type":"user"'*)
      echo '{"type":"system","subtype":"init","session_id":"fake-1","model":"claude-opus-5-5"}'
      echo '{"type":"control_request","request_id":"req-1","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"rm teste.txt"},"permission_suggestions":[{"type":"addRules","behavior":"allow","rules":[{"toolName":"Bash","ruleContent":"rm *"}]}]}}' ;;
    *'"behavior":"allow"'*)
      echo '{"type":"result","is_error":false,"result":"feito","session_id":"fake-1","total_cost_usd":0.5}' ;;
  esac
done
"#;

    #[test]
    fn fluxo_completo_init_pedido_de_permissao_resposta_e_fim_de_turno() {
        let (dir, script, log) = fake_claude(SCRIPT);
        let e = spawn(&script);

        let first = collect_until(&e, |ev| matches!(ev, ChatEvent::Commands(_)));
        assert!(
            matches!(&first[..], [ChatEvent::Models(m), ChatEvent::Commands(c)]
            if m[0].value == "opus" && c[0].name == "review")
        );

        e.send("apaga o teste.txt", &[]).expect("send");
        let got = collect_until(&e, |ev| matches!(ev, ChatEvent::PermissionAsk { .. }));
        assert!(matches!(&got[0], ChatEvent::Ready { session_id, .. } if session_id == "fake-1"));
        let ChatEvent::PermissionAsk {
            request_id,
            always_rule,
            ..
        } = got.last().expect("pedido")
        else {
            panic!("esperava o pedido de permissão");
        };
        assert_eq!(request_id, "req-1");
        assert_eq!(always_rule.as_deref(), Some("Bash(rm *)"));

        // "Sempre permitir": o input do pedido volta em updatedInput + a regra sugerida.
        e.answer_permission("req-1", true, true, None, None);
        let end = collect_until(&e, |ev| matches!(ev, ChatEvent::TurnEnd { .. }));
        assert!(
            matches!(end.last(), Some(ChatEvent::TurnEnd { cost_usd: Some(c), .. }) if *c == 0.5)
        );

        e.request_usage();
        e.set_model("opus");
        let more = collect_until(&e, |ev| matches!(ev, ChatEvent::ModelChanged(_)));
        assert!(more.iter().any(|ev| matches!(ev,
            ChatEvent::PlanUsage(Some(u)) if u.limits.first().map(|l| l.percent) == Some(7))));
        assert!(more.contains(&ChatEvent::ModelChanged("claude-opus-5-5".into())));

        let logged = fs::read_to_string(&log).expect("log");
        let lines: Vec<Value> = logged
            .lines()
            .map(|l| serde_json::from_str(l).expect("json"))
            .collect();
        assert_eq!(lines[0]["request"]["subtype"], "initialize");
        assert_eq!(
            lines[1],
            json!({"type": "user", "message": {"role": "user", "content": "apaga o teste.txt"}})
        );
        let allow = &lines[2]["response"]["response"];
        assert_eq!(allow["behavior"], "allow");
        assert_eq!(allow["updatedInput"], json!({"command": "rm teste.txt"}));
        assert_eq!(
            allow["updatedPermissions"][0]["rules"][0]["ruleContent"],
            "rm *"
        );

        e.close();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn controles_vao_no_formato_do_agent_sdk() {
        let (dir, script, log) = fake_claude(SCRIPT);
        let e = spawn(&script);
        e.set_permission_mode("plan");
        e.set_effort("max");
        e.interrupt();
        let end = Instant::now() + Duration::from_secs(5);
        let mut text = String::new();
        while Instant::now() < end && text.lines().count() < 4 {
            thread::sleep(Duration::from_millis(30));
            text = fs::read_to_string(&log).unwrap_or_default();
        }
        let l: Vec<Value> = text
            .lines()
            .filter_map(|x| serde_json::from_str(x).ok())
            .collect();
        assert_eq!(
            l[1]["request"],
            json!({"subtype": "set_permission_mode", "mode": "plan"})
        );
        assert_eq!(
            l[2]["request"],
            json!({"subtype": "apply_flag_settings", "settings": {"effortLevel": "max"}})
        );
        assert_eq!(l[3]["request"], json!({"subtype": "interrupt"}));
        e.close();
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn queda_do_processo_sem_pedir_emite_engine_failed_com_stderr() {
        let (dir, script, _log) =
            fake_claude("IFS= read -r line\necho 'falha fatal simulada' >&2\nsleep 0.2\nexit 17");
        let e = spawn(&script);
        let got = collect_until(&e, |ev| matches!(ev, ChatEvent::EngineFailed { .. }));
        assert_eq!(
            got.last(),
            Some(&ChatEvent::EngineFailed {
                exit_code: Some(17),
                stderr: "falha fatal simulada".into()
            })
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fechar_mata_o_processo_e_nao_emite_falha() {
        let (dir, script, _log) = fake_claude("while IFS= read -r line; do sleep 30; done");
        let e = spawn(&script);
        assert!(e.is_running());
        let t = Instant::now();
        e.close();
        assert!(t.elapsed() < Duration::from_secs(2));
        assert!(!e.is_running());
        assert_eq!(e.send("depois", &[]), Err(ChatError::SessionClosed));
        assert!(e.recv_timeout(Duration::from_millis(300)).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn binario_inexistente_vira_erro_tipado() {
        let res = ChatEngine::spawn(ChatConfig::new("/caminho/inexistente/claude-99999"));
        assert!(matches!(res, Err(ChatError::BinaryNotFound { .. })));
    }
}
