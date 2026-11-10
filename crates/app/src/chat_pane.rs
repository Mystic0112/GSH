//! Controlador e estado do pane de chat (`chat_viewmodel.dart` do Flutter).
//!
//! Mantém o histórico da conversa (`items`), campo de texto (`input`), estado de
//! ocupação (`busy`) e a ponte com [`gsh_chat::ChatEngine`]. Toda a lógica de
//! negócio fica separada da view ([`crate::chat_view`]).

use std::path::{Path, PathBuf};
use std::time::Duration;

use gsh_chat::{ChatConfig, ChatEngine, ChatError, ChatEvent, DEFAULT_CLAUDE_BINARY};
use gsh_core::tabs::basename;
use iced::{Element, Subscription, Task};

use crate::chat_view;
use crate::labels;
use crate::terminal_pane::TICK_INTERVAL_MS;

/// Entrada de chamada de ferramenta na transcrição do chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatToolEntry {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub command: Option<String>,
    pub done: bool,
    pub is_error: bool,
    pub result: String,
    pub expanded: bool,
}

/// Um item visível na transcrição da conversa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatItem {
    /// Mensagem enviada pelo usuário.
    User { text: String },
    /// Mensagem do assistente (em streaming ou concluída).
    Assistant { text: String, streaming: bool },
    /// Chamada de ferramenta resumida em uma linha (expansível).
    Tool(ChatToolEntry),
    /// Aviso de erro de processo ou de turno.
    Error { message: String },
}

/// Mensagens de interação e atualização do [`ChatPane`].
#[derive(Debug, Clone)]
pub enum ChatMsg {
    /// Edição no texto do composer.
    InputChanged(String),
    /// Envio da mensagem atual (Enter ou botão Enviar).
    Send,
    /// Cancelamento da resposta em curso (mata o processo filho).
    Cancel,
    /// Alterna expansão dos detalhes de uma ferramenta pelo índice em `items`.
    ToggleTool(usize),
    /// Pulso periódico para drenar eventos do [`ChatEngine`].
    Tick,
}

/// Estado e controlador de uma aba de chat por agente.
pub struct ChatPane {
    cwd: Option<PathBuf>,
    branch: Option<String>,
    binary: PathBuf,
    items: Vec<ChatItem>,
    input: String,
    busy: bool,
    session_id: Option<String>,
    model: Option<String>,
    cost_usd: f64,
    engine: Option<ChatEngine>,
    static_test_mode: bool,
}

impl Default for ChatPane {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl ChatPane {
    /// Cria um novo pane de chat associado ao diretório `cwd` e `branch` do workspace.
    /// O processo do `claude` só é iniciado no primeiro envio.
    pub fn new(cwd: Option<PathBuf>, branch: Option<String>) -> Self {
        Self {
            cwd,
            branch,
            binary: PathBuf::from(DEFAULT_CLAUDE_BINARY),
            items: Vec::new(),
            input: String::new(),
            busy: false,
            session_id: None,
            model: None,
            cost_usd: 0.0,
            engine: None,
            static_test_mode: false,
        }
    }

    /// Define um binário customizado (usado em testes com script falso).
    pub fn with_binary(mut self, binary: impl Into<PathBuf>) -> Self {
        self.binary = binary.into();
        self
    }

    /// Ativa o modo estático de teste (não spawna processos reais).
    pub fn with_static_test_mode(mut self, enabled: bool) -> Self {
        self.static_test_mode = enabled;
        self
    }

    /// Atualiza o diretório e branch quando o workspace ativo muda.
    pub fn set_workspace(&mut self, cwd: Option<PathBuf>, branch: Option<String>) {
        self.cwd = cwd;
        self.branch = branch;
    }

    pub fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    pub fn branch(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    pub fn items(&self) -> &[ChatItem] {
        &self.items
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn is_busy(&self) -> bool {
        self.busy
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub fn cost_usd(&self) -> f64 {
        self.cost_usd
    }

    /// Nome curto da pasta do workspace exibido no cabeçalho e no composer.
    pub fn folder_label(&self) -> &str {
        self.cwd
            .as_deref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(labels::WORKSPACE_GSH)
    }

    /// Título da conversa exibido no cabeçalho (primeira mensagem do usuário ou `"Nova conversa"`).
    pub fn conversation_title(&self) -> &str {
        self.items
            .iter()
            .find_map(|item| match item {
                ChatItem::User { text } => text.lines().next().map(str::trim),
                _ => None,
            })
            .filter(|s| !s.is_empty())
            .unwrap_or(labels::CHAT_NEW_CONVERSATION)
    }

    /// Encerra a sessão de chat e mata o processo filho se estiver em execução.
    pub fn close(&mut self) {
        if let Some(mut engine) = self.engine.take() {
            engine.cancel();
        }
        self.finish_streaming_items();
        self.busy = false;
    }

    /// Processa uma [`ChatMsg`] e atualiza o estado do pane.
    pub fn update(&mut self, msg: ChatMsg) -> Task<ChatMsg> {
        match msg {
            ChatMsg::InputChanged(val) => {
                self.input = val;
                Task::none()
            }
            ChatMsg::Send => {
                let text = self.input.trim().to_string();
                if text.is_empty() || self.busy {
                    return Task::none();
                }
                self.input.clear();
                self.items.push(ChatItem::User { text: text.clone() });
                self.busy = true;

                if !self.static_test_mode {
                    if let Err(err) = self.ensure_engine_and_send(&text) {
                        self.busy = false;
                        self.engine = None;
                        self.items.push(ChatItem::Error {
                            message: labels::format_chat_start_failed(&err.to_string()),
                        });
                    }
                }
                Task::none()
            }
            ChatMsg::Cancel => {
                self.close();
                Task::none()
            }
            ChatMsg::ToggleTool(idx) => {
                if let Some(ChatItem::Tool(tool)) = self.items.get_mut(idx) {
                    tool.expanded = !tool.expanded;
                }
                Task::none()
            }
            ChatMsg::Tick => {
                self.poll_events();
                Task::none()
            }
        }
    }

    /// Drena todos os eventos pendentes do motor e aplica no estado.
    pub fn poll_events(&mut self) {
        let Some(engine) = &self.engine else {
            return;
        };
        let pending = engine.drain_events();
        for ev in pending {
            self.apply_event(ev);
        }
    }

    /// Aplica um [`ChatEvent`] na transcrição da conversa (função de transição de estado).
    pub fn apply_event(&mut self, event: ChatEvent) {
        match event {
            ChatEvent::Ready {
                session_id, model, ..
            } => {
                if !session_id.is_empty() {
                    self.session_id = Some(session_id);
                }
                if !model.is_empty() {
                    self.model = Some(model);
                }
            }
            ChatEvent::TextDelta(delta) => {
                if let Some(ChatItem::Assistant {
                    text,
                    streaming: true,
                }) = self.items.last_mut()
                {
                    text.push_str(&delta);
                } else {
                    self.items.push(ChatItem::Assistant {
                        text: delta,
                        streaming: true,
                    });
                }
            }
            ChatEvent::AssistantText(full_text) => {
                if let Some(ChatItem::Assistant {
                    text,
                    streaming: true,
                }) = self.items.last_mut()
                {
                    *text = full_text;
                    self.finish_last_assistant();
                } else {
                    self.items.push(ChatItem::Assistant {
                        text: full_text,
                        streaming: false,
                    });
                }
            }
            ChatEvent::ThinkingDelta(_) | ChatEvent::Thinking(_) => {}
            ChatEvent::ToolUse {
                id,
                name,
                summary,
                input,
            } => {
                self.finish_last_assistant();
                let command = input
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(ToOwned::to_owned);
                self.items.push(ChatItem::Tool(ChatToolEntry {
                    id,
                    name,
                    summary,
                    command,
                    done: false,
                    is_error: false,
                    result: String::new(),
                    expanded: false,
                }));
            }
            ChatEvent::ToolResult {
                tool_use_id,
                is_error,
                content,
            } => {
                for item in self.items.iter_mut().rev() {
                    if let ChatItem::Tool(tool) = item {
                        if tool.id == tool_use_id {
                            tool.done = true;
                            tool.is_error = is_error;
                            tool.result = content;
                            break;
                        }
                    }
                }
            }
            ChatEvent::Result {
                is_error,
                result,
                session_id,
                cost_usd,
            } => {
                self.finish_streaming_items();
                self.busy = false;
                if !session_id.is_empty() {
                    self.session_id = Some(session_id);
                }
                if let Some(c) = cost_usd {
                    self.cost_usd += c;
                }
                if is_error && !result.trim().is_empty() {
                    self.items.push(ChatItem::Error {
                        message: labels::format_chat_turn_failed(result.trim()),
                    });
                }
            }
            ChatEvent::Error(err) => {
                self.finish_streaming_items();
                self.busy = false;
                let message = match &err {
                    ChatError::ProcessExited { exit_code, stderr } => {
                        self.engine = None;
                        labels::format_chat_engine_exited(*exit_code, stderr)
                    }
                    _ => labels::format_chat_turn_failed(&err.to_string()),
                };
                self.items.push(ChatItem::Error { message });
            }
        }
    }

    /// Ticker periódico (~16 ms) ativo somente enquanto há um turno em execução (`busy`).
    pub fn subscription(&self) -> Subscription<ChatMsg> {
        if self.busy {
            iced::time::every(Duration::from_millis(TICK_INTERVAL_MS)).map(|_| ChatMsg::Tick)
        } else {
            Subscription::none()
        }
    }

    /// Renderiza o pane de chat.
    pub fn view(&self) -> Element<'_, ChatMsg> {
        chat_view::view(self)
    }

    fn ensure_engine_and_send(&mut self, text: &str) -> Result<(), ChatError> {
        if self.engine.is_none() {
            let cfg = ChatConfig::new()
                .with_binary(&self.binary)
                .with_cwd(self.cwd.clone())
                .with_resume(self.session_id.clone());
            self.engine = Some(ChatEngine::spawn(cfg)?);
        }
        if let Some(engine) = self.engine.as_mut() {
            engine.send_user_message(text)?;
        }
        Ok(())
    }

    fn finish_last_assistant(&mut self) {
        if let Some(ChatItem::Assistant { streaming, .. }) = self.items.last_mut() {
            *streaming = false;
        }
    }

    fn finish_streaming_items(&mut self) {
        for item in &mut self.items {
            match item {
                ChatItem::Assistant { streaming, .. } => *streaming = false,
                ChatItem::Tool(tool) => tool.done = true,
                _ => {}
            }
        }
    }
}

/// Auxiliar para exibir o nome base de um caminho quando necessário.
pub fn basename_or_gsh(path: Option<&Path>) -> String {
    path.map(|p| basename(&p.to_string_lossy()))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| labels::WORKSPACE_GSH.to_string())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn fluxo_de_envio_streaming_tool_call_e_cancelamento_no_chat_pane() {
        let mut pane = ChatPane::new(
            Some(PathBuf::from("/home/dev/GSH-Desktop")),
            Some("feat/terminal-ui".to_string()),
        )
        .with_static_test_mode(true);

        assert_eq!(pane.folder_label(), "GSH-Desktop");
        assert_eq!(pane.conversation_title(), labels::CHAT_NEW_CONVERSATION);
        assert!(!pane.is_busy());

        // Usuário digita e envia
        let _ = pane.update(ChatMsg::InputChanged("oi agente".to_string()));
        let _ = pane.update(ChatMsg::Send);
        assert!(pane.input().is_empty());
        assert!(pane.is_busy());
        assert_eq!(pane.conversation_title(), "oi agente");
        assert_eq!(
            pane.items(),
            &[ChatItem::User {
                text: "oi agente".to_string()
            }]
        );

        // Chegam deltas de texto -> acumula num único Assistant em streaming
        pane.apply_event(ChatEvent::TextDelta("Vou ".to_string()));
        pane.apply_event(ChatEvent::TextDelta("checar.".to_string()));
        assert_eq!(
            pane.items().last(),
            Some(&ChatItem::Assistant {
                text: "Vou checar.".to_string(),
                streaming: true,
            })
        );

        // Chega tool_use -> fecha o streaming do texto anterior e adiciona Tool resumida
        pane.apply_event(ChatEvent::ToolUse {
            id: "tool_1".to_string(),
            name: "Bash".to_string(),
            summary: "cargo test".to_string(),
            input: json!({"command": "cargo test"}),
        });
        assert_eq!(
            pane.items().get(1),
            Some(&ChatItem::Assistant {
                text: "Vou checar.".to_string(),
                streaming: false,
            })
        );
        assert!(matches!(
            pane.items().get(2),
            Some(ChatItem::Tool(t)) if t.name == "Bash" && t.summary == "cargo test" && !t.done
        ));

        // Chega tool_result -> atualiza a Tool correspondente como concluída
        pane.apply_event(ChatEvent::ToolResult {
            tool_use_id: "tool_1".to_string(),
            is_error: false,
            content: "ok".to_string(),
        });
        assert!(matches!(
            pane.items().get(2),
            Some(ChatItem::Tool(t)) if t.done && t.result == "ok" && !t.expanded
        ));

        // Expandir/recolher detalhes da ferramenta
        let _ = pane.update(ChatMsg::ToggleTool(2));
        assert!(matches!(
            pane.items().get(2),
            Some(ChatItem::Tool(t)) if t.expanded
        ));

        // Novo stream começa e usuário clica em Cancelar
        pane.apply_event(ChatEvent::TextDelta("Gerando mais...".to_string()));
        assert!(pane.is_busy());
        let _ = pane.update(ChatMsg::Cancel);
        assert!(!pane.is_busy());
        assert_eq!(
            pane.items().last(),
            Some(&ChatItem::Assistant {
                text: "Gerando mais...".to_string(),
                streaming: false,
            })
        );
    }
}
