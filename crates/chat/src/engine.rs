//! Motor de execução do processo `claude` em `stream-json` (`ClaudeStreamEngine` no Flutter).
//!
//! Spawna o binário configurável com `stdin`/`stdout`/`stderr` em pipe, escreve
//! mensagens do usuário como JSON por linha no `stdin`, lê o `stdout` linha a
//! linha em uma thread dedicada e entrega [`ChatEvent`] num canal `mpsc`.
//! Cancelar mata o processo filho imediatamente.

use std::io::{BufRead, BufReader, ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::error::ChatError;
use crate::protocol::{
    format_user_message_line, parse_stream_line, ChatEvent, DEFAULT_CLAUDE_ARGS,
    DEFAULT_CLAUDE_BINARY,
};

/// Quantidade máxima de linhas finais do `stderr` retidas para diagnóstico.
const MAX_STDERR_LINES: usize = 20;

/// Configuração de inicialização de uma sessão [`ChatEngine`].
#[derive(Debug, Clone)]
pub struct ChatConfig {
    /// Caminho ou nome do binário (padrão: `"claude"`). Injetável para testes.
    pub binary: PathBuf,
    /// Argumentos passados ao processo (padrão: [`DEFAULT_CLAUDE_ARGS`]).
    pub args: Vec<String>,
    /// Diretório de trabalho da sessão.
    pub cwd: Option<PathBuf>,
    /// Variáveis de ambiente extras para o processo filho.
    pub env: Vec<(String, String)>,
    /// ID de sessão anterior para retomar (`--resume <id>`).
    pub resume_session_id: Option<String>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            binary: PathBuf::from(DEFAULT_CLAUDE_BINARY),
            args: DEFAULT_CLAUDE_ARGS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            cwd: None,
            env: Vec::new(),
            resume_session_id: None,
        }
    }
}

impl ChatConfig {
    /// Cria uma configuração com o binário padrão (`claude`) e argumentos `stream-json`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Define um caminho/binário customizado (usado em testes para injetar script falso).
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Define o diretório de trabalho onde o processo será iniciado.
    pub fn with_cwd(mut self, cwd: Option<PathBuf>) -> Self {
        self.cwd = cwd;
        self
    }

    /// Define o `session_id` para continuar uma conversa (`--resume`).
    pub fn with_resume(mut self, session_id: Option<String>) -> Self {
        self.resume_session_id = session_id;
        self
    }

    /// Retorna a lista efetiva de argumentos de linha de comando.
    pub fn effective_args(&self) -> Vec<String> {
        let mut out = self.args.clone();
        if let Some(sid) = &self.resume_session_id {
            if !sid.trim().is_empty() {
                out.push("--resume".to_string());
                out.push(sid.clone());
            }
        }
        out
    }
}

/// Motor ativo de uma conversa com o Claude Code em `stream-json`.
pub struct ChatEngine {
    child: Arc<Mutex<Child>>,
    stdin: Option<ChildStdin>,
    events_rx: Receiver<ChatEvent>,
    cancelled: Arc<AtomicBool>,
    turn_active: Arc<AtomicBool>,
}

impl ChatEngine {
    /// Inicia o processo filho com a [`ChatConfig`] fornecida e dispara as threads
    /// leitoras de `stdout` e `stderr`.
    pub fn spawn(config: ChatConfig) -> Result<Self, ChatError> {
        let program_display = config.binary.display().to_string();
        let mut cmd = Command::new(&config.binary);
        cmd.args(config.effective_args())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if let Some(dir) = &config.cwd {
            if dir.exists() {
                cmd.current_dir(dir);
            }
        }
        for (k, v) in &config.env {
            cmd.env(k, v);
        }
        if let Some(parent) = config
            .binary
            .parent()
            .filter(|p| !p.as_os_str().is_empty() && p.exists())
        {
            prepend_dir_to_path(&mut cmd, parent);
        }

        let mut child = cmd.spawn().map_err(|err| {
            if err.kind() == ErrorKind::NotFound {
                ChatError::BinaryNotFound {
                    program: program_display.clone(),
                    cause: err.to_string(),
                }
            } else {
                ChatError::SpawnFailed {
                    program: program_display,
                    cause: err.to_string(),
                }
            }
        })?;

        let Some(stdin) = child.stdin.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ChatError::StdioUnavailable);
        };
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(ChatError::StdioUnavailable);
        };
        let stderr = child.stderr.take();

        let stderr_buf: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        if let Some(err_pipe) = stderr {
            let buf_clone = Arc::clone(&stderr_buf);
            thread::spawn(move || {
                let reader = BufReader::new(err_pipe);
                for line in reader.lines().map_while(Result::ok) {
                    if let Ok(mut guard) = buf_clone.lock() {
                        if guard.len() >= MAX_STDERR_LINES {
                            let _ = guard.remove(0);
                        }
                        guard.push(line);
                    }
                }
            });
        }

        let (events_tx, events_rx) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let turn_active = Arc::new(AtomicBool::new(false));
        let child_arc = Arc::new(Mutex::new(child));

        spawn_stdout_reader(
            stdout,
            events_tx,
            Arc::clone(&cancelled),
            Arc::clone(&turn_active),
            Arc::clone(&child_arc),
            stderr_buf,
        );

        Ok(Self {
            child: child_arc,
            stdin: Some(stdin),
            events_rx,
            cancelled,
            turn_active,
        })
    }

    /// Escreve uma mensagem do usuário como uma linha JSON no `stdin` do processo.
    pub fn send_user_message(&mut self, text: &str) -> Result<(), ChatError> {
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(ChatError::SessionClosed);
        }
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(ChatError::SessionClosed);
        };
        let line = format_user_message_line(text);
        self.turn_active.store(true, Ordering::SeqCst);
        writeln!(stdin, "{line}")
            .and_then(|()| stdin.flush())
            .map_err(|err| {
                self.turn_active.store(false, Ordering::SeqCst);
                ChatError::WriteStdin {
                    cause: err.to_string(),
                }
            })
    }

    /// Tenta ler o próximo [`ChatEvent`] disponível sem bloquear.
    pub fn try_recv(&self) -> Option<ChatEvent> {
        self.events_rx.try_recv().ok()
    }

    /// Drena todos os [`ChatEvent`] atualmente disponíveis na fila.
    pub fn drain_events(&self) -> Vec<ChatEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.events_rx.try_recv() {
            out.push(ev);
        }
        out
    }

    /// Aguarda o próximo [`ChatEvent`] até `timeout` (útil em testes).
    pub fn recv_timeout(&self, timeout: Duration) -> Option<ChatEvent> {
        self.events_rx.recv_timeout(timeout).ok()
    }

    /// Cancela a resposta em curso matando imediatamente o processo filho.
    pub fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.turn_active.store(false, Ordering::SeqCst);
        let _ = self.stdin.take();
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    /// Informa se o processo filho ainda está em execução.
    pub fn is_running(&self) -> bool {
        if self.cancelled.load(Ordering::SeqCst) {
            return false;
        }
        let Ok(mut child) = self.child.lock() else {
            return false;
        };
        matches!(child.try_wait(), Ok(None))
    }
}

impl Drop for ChatEngine {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn spawn_stdout_reader(
    stdout: std::process::ChildStdout,
    events_tx: Sender<ChatEvent>,
    cancelled: Arc<AtomicBool>,
    turn_active: Arc<AtomicBool>,
    child: Arc<Mutex<Child>>,
    stderr_buf: Arc<Mutex<Vec<String>>>,
) {
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line_res in reader.lines() {
            if cancelled.load(Ordering::SeqCst) {
                return;
            }
            let Ok(line) = line_res else {
                break;
            };
            match parse_stream_line(&line) {
                Ok(events) => {
                    for ev in events {
                        if matches!(ev, ChatEvent::Result { .. }) {
                            turn_active.store(false, Ordering::SeqCst);
                        }
                        if events_tx.send(ev).is_err() {
                            return;
                        }
                    }
                }
                Err(err) => {
                    if events_tx.send(ChatEvent::Error(err)).is_err() {
                        return;
                    }
                }
            }
        }

        if cancelled.load(Ordering::SeqCst) {
            return;
        }

        let exit_code = child
            .lock()
            .ok()
            .and_then(|mut c| c.wait().ok())
            .and_then(|status| status.code());

        let was_active = turn_active.swap(false, Ordering::SeqCst);
        if exit_code != Some(0) || was_active {
            let stderr = stderr_buf
                .lock()
                .map(|lines| lines.join("\n"))
                .unwrap_or_default();
            let _ = events_tx.send(ChatEvent::Error(ChatError::ProcessExited {
                exit_code,
                stderr,
            }));
        }
    });
}

fn prepend_dir_to_path(cmd: &mut Command, dir: &Path) {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let existing = std::env::var("PATH").unwrap_or_default();
    let next_path = if existing.is_empty() {
        dir.display().to_string()
    } else {
        format!("{}{sep}{existing}", dir.display())
    };
    cmd.env("PATH", next_path);
}
