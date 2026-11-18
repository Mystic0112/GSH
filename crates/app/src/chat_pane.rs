//! Controlador e estado do pane de chat (`chat_viewmodel.dart` do Flutter).
//!
//! Mantém o histórico da conversa (`items`), campo de texto (`input`), estado de
//! ocupação (`busy`) e a ponte com [`gsh_chat::ChatEngine`]. Toda a lógica de
//! negócio fica separada da view ([`crate::chat_view`]).

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use gsh_chat::{
    Attachment, ChatConfig, ChatEngine, ChatError, ChatEvent, CommandOption, LaunchOptions,
    ModelOption, PlanUsage, SessionState, DEFAULT_CLAUDE_BINARY,
};
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
    pub input: serde_json::Map<String, serde_json::Value>,
}

/// Um item visível na transcrição da conversa.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatItem {
    /// Mensagem enviada pelo usuário.
    User {
        text: String,
    },
    /// Mensagem do assistente (em streaming ou concluída).
    Assistant {
        text: String,
        streaming: bool,
    },
    /// Chamada de ferramenta resumida em uma linha (expansível).
    Tool(ChatToolEntry),
    /// Aviso de erro de processo ou de turno.
    Error {
        message: String,
    },
    Thinking {
        text: String,
    },
    Permission {
        request_id: String,
        tool: String,
        description: String,
        always_rule: Option<String>,
        input: serde_json::Map<String, serde_json::Value>,
        question: Option<String>,
        options: Vec<(String, String)>,
    },
    Questions {
        request_id: String,
        input: serde_json::Map<String, serde_json::Value>,
        questions: Vec<ChatQuestion>,
        answered: Option<BTreeMap<String, String>>,
    },
    Plan {
        request_id: String,
        plan: String,
        input: serde_json::Map<String, serde_json::Value>,
        feedback: String,
    },
    ResumePrompt {
        session_id: String,
    },
    Info {
        text: String,
        warning: bool,
    },
    Peer {
        from: String,
        text: String,
    },
    Classified {
        state: SessionState,
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatQuestion {
    pub question: String,
    pub header: String,
    pub options: Vec<(String, String)>,
    pub multi_select: bool,
}

/// Mensagens de interação e atualização do [`ChatPane`].
#[derive(Debug, Clone)]
pub enum ChatMsg {
    /// Edição no texto do composer.
    InputChanged(String),
    InputAction(iced::widget::text_editor::Action),
    /// Envio da mensagem atual (Enter ou botão Enviar).
    Send,
    /// Cancelamento da resposta em curso (mata o processo filho).
    Cancel,
    /// Alterna expansão dos detalhes de uma ferramenta pelo índice em `items`.
    ToggleTool(usize),
    ToggleEditTools(Vec<usize>),
    ToggleEditCard(usize),
    /// Pulso periódico para drenar eventos do [`ChatEngine`].
    Tick,
    PermissionAnswer {
        request_id: String,
        allow: bool,
        always: bool,
    },
    AnswerQuestion {
        request_id: String,
        input: serde_json::Map<String, serde_json::Value>,
    },
    SetMode(String),
    SetModel(String),
    SetEffort(String),
    OpenComposerMenu(ComposerMenu),
    CloseComposerMenu,
    Attach,
    PickFiles(Option<Vec<PathBuf>>),
    RemoveAttachment(usize),
    ToggleHistory,
    TogglePeerPicker,
    ToggleUsage,
    ResumeSession(String),
    RefreshUsage,
    SelectCommand(String),
    SlashKey(SlashKey),
    PasteImage,
    ClipboardImage(Result<Option<Vec<u8>>, String>),
    BeginRename,
    RenameChanged(String),
    SaveRename,
    CancelRename,
    RefreshAccountResult(Result<Option<gsh_chat::AccountProbe>, String>),
    UsageRefresh,
    HandOff,
    PickPeerSession(String),
    QuestionToggle {
        request_id: String,
        question: String,
        option: String,
    },
    QuestionFreeText {
        request_id: String,
        question: String,
        value: String,
    },
    SubmitQuestions {
        request_id: String,
    },
    PlanFeedback {
        request_id: String,
        value: String,
    },
    PlanDecision {
        request_id: String,
        approve: bool,
    },
    OpenLink(String),
    RemoteImageLoaded {
        url: String,
        bytes: Result<Vec<u8>, String>,
    },
    ConfirmResume {
        session_id: String,
        confirm: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposerMenu {
    Mode,
    Effort,
    Model,
    RefreshingEffort,
    RefreshingModel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlashKey {
    Up,
    Down,
    Enter,
    Tab,
    Escape,
}

/// Estado e controlador de uma aba de chat por agente.
pub struct ChatPane {
    cwd: Option<PathBuf>,
    branch: Option<String>,
    binary: PathBuf,
    items: Vec<ChatItem>,
    input: String,
    input_content: iced::widget::text_editor::Content,
    busy: bool,
    session_id: Option<String>,
    model: Option<String>,
    effort: Option<String>,
    cost_usd: f64,
    engine: Option<ChatEngine>,
    static_test_mode: bool,
    attachments: Vec<Attachment>,
    models: Vec<ModelOption>,
    commands: Vec<CommandOption>,
    mode: String,
    context: (u64, u64),
    usage: Option<PlanUsage>,
    session_state: Option<(SessionState, String)>,
    title: Option<String>,
    history_open: bool,
    peer_picker_open: bool,
    usage_open: bool,
    composer_menu: Option<ComposerMenu>,
    spinner_rotation: f32,
    slash_index: usize,
    slash_dismissed_at: Option<String>,
    rename_open: bool,
    rename_text: String,
    handoff_requested: bool,
    remote_control_url: Option<String>,
    markdown: HashMap<usize, iced::widget::markdown::Content>,
    remote_images: HashMap<String, Vec<u8>>,
    remote_images_pending: std::collections::HashSet<String>,
    collapsed_edit_cards: std::collections::HashSet<usize>,
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
            input_content: iced::widget::text_editor::Content::new(),
            busy: false,
            session_id: None,
            model: None,
            effort: Some("high".to_string()),
            cost_usd: 0.0,
            engine: None,
            static_test_mode: false,
            attachments: Vec::new(),
            models: Vec::new(),
            commands: Vec::new(),
            mode: "default".to_string(),
            context: (0, 0),
            usage: None,
            session_state: None,
            title: None,
            history_open: false,
            peer_picker_open: false,
            usage_open: false,
            composer_menu: None,
            spinner_rotation: 0.0,
            slash_index: 0,
            slash_dismissed_at: None,
            rename_open: false,
            rename_text: String::new(),
            handoff_requested: false,
            remote_control_url: None,
            markdown: HashMap::new(),
            remote_images: HashMap::new(),
            remote_images_pending: std::collections::HashSet::new(),
            collapsed_edit_cards: std::collections::HashSet::new(),
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

    pub fn with_defaults(mut self, mode: impl Into<String>, effort: impl Into<String>) -> Self {
        self.mode = mode.into();
        self.effort = Some(effort.into());
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
    pub fn input_content(&self) -> &iced::widget::text_editor::Content {
        &self.input_content
    }

    fn set_input(&mut self, value: String) {
        self.input_content = iced::widget::text_editor::Content::with_text(&value);
        self.input = value;
    }

    pub fn working_directory(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }

    pub fn deliver_agent_text(&mut self, text: String) -> Result<(), String> {
        self.ensure_engine_and_send(&text)
            .map_err(|error| error.to_string())?;
        self.items.push(ChatItem::User { text });
        self.busy = true;
        Ok(())
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
    pub fn effort(&self) -> Option<&str> {
        self.effort.as_deref()
    }
    pub fn composer_menu(&self) -> Option<ComposerMenu> {
        self.composer_menu
    }
    pub fn spinner_rotation(&self) -> f32 {
        self.spinner_rotation
    }
    pub fn efforts(&self) -> Vec<String> {
        if self.models.is_empty() {
            return ["low", "medium", "high", "xhigh", "max"]
                .into_iter()
                .map(str::to_owned)
                .collect();
        }
        self.models
            .iter()
            .find(|model| {
                model.value == self.model.as_deref().unwrap_or_default()
                    || model.resolved.as_deref() == self.model.as_deref()
            })
            .map(|model| model.efforts.clone())
            .filter(|efforts| !efforts.is_empty())
            .unwrap_or_default()
    }

    pub fn cost_usd(&self) -> f64 {
        self.cost_usd
    }

    pub fn models(&self) -> &[ModelOption] {
        &self.models
    }
    pub fn commands(&self) -> &[CommandOption] {
        &self.commands
    }
    pub fn mode(&self) -> &str {
        &self.mode
    }
    pub fn context(&self) -> (u64, u64) {
        self.context
    }
    pub fn attachments(&self) -> &[Attachment] {
        &self.attachments
    }
    pub fn usage(&self) -> Option<&PlanUsage> {
        self.usage.as_ref()
    }
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }
    pub fn rename_open(&self) -> bool {
        self.rename_open
    }
    pub fn rename_text(&self) -> &str {
        &self.rename_text
    }
    pub fn slash_index(&self) -> usize {
        self.slash_index
    }
    pub fn slash_matches(&self) -> Vec<&CommandOption> {
        let text = self.input.as_str();
        if self.slash_dismissed_at.as_deref() == Some(text)
            || !text.starts_with('/')
            || text.chars().any(char::is_whitespace)
        {
            return Vec::new();
        }
        let query = text.trim_start_matches('/').to_lowercase();
        let mut buckets: [Vec<&CommandOption>; 4] =
            [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
        for command in &self.commands {
            let name = command.name.to_lowercase();
            if name.starts_with(&query) {
                buckets[0].push(command);
            } else if command
                .aliases
                .iter()
                .any(|a| a.to_lowercase().starts_with(&query))
            {
                buckets[1].push(command);
            } else if name.contains(&query) {
                buckets[2].push(command);
            } else if query.chars().count() >= 3
                && command.description.to_lowercase().contains(&query)
            {
                buckets[3].push(command);
            }
        }
        buckets.into_iter().flatten().take(8).collect()
    }
    pub fn usage_fraction(&self) -> Option<f32> {
        if self.context.1 > 0 && self.context.0 > 0 {
            return Some(self.context.0 as f32 / self.context.1 as f32);
        }
        self.usage
            .as_ref()?
            .limits
            .iter()
            .find(|l| l.kind == gsh_chat::LimitKind::Session)
            .map(|l| f32::from(l.percent) / 100.0)
    }
    pub fn handoff_requested(&self) -> bool {
        self.handoff_requested
    }
    pub fn take_handoff_requested(&mut self) -> bool {
        std::mem::take(&mut self.handoff_requested)
    }
    pub fn markdown(&self, index: usize) -> Option<&iced::widget::markdown::Content> {
        self.markdown.get(&index)
    }
    pub fn remote_image(&self, url: &str) -> Option<&[u8]> {
        self.remote_images.get(url).map(Vec::as_slice)
    }
    pub fn edit_card_collapsed(&self, index: usize) -> bool {
        self.collapsed_edit_cards.contains(&index)
    }
    pub fn history_open(&self) -> bool {
        self.history_open
    }
    pub fn peer_picker_open(&self) -> bool {
        self.peer_picker_open
    }
    pub fn usage_open(&self) -> bool {
        self.usage_open
    }
    pub fn peer_sessions(&self) -> Vec<gsh_chat::SessionEntry> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.recent_sessions()
            .into_iter()
            .filter(|s| {
                Some(s.id.as_str()) != self.session_id()
                    && now.saturating_sub(s.last_seen_ms) < 24 * 60 * 60 * 1000
            })
            .take(15)
            .collect()
    }
    pub fn recent_sessions(&self) -> Vec<gsh_chat::SessionEntry> {
        gsh_chat::history::projects_root()
            .map(|p| gsh_chat::recent_sessions(&p, Duration::from_secs(7 * 24 * 60 * 60)))
            .unwrap_or_default()
            .into_iter()
            .filter(|s| {
                Path::new(&s.project).is_absolute() && Some(s.id.as_str()) != self.session_id()
            })
            .take(30)
            .collect()
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
        if let Some(engine) = self.engine.take() {
            engine.close();
        }
        self.finish_streaming_items();
        self.busy = false;
    }

    /// Processa uma [`ChatMsg`] e atualiza o estado do pane.
    pub fn update(&mut self, msg: ChatMsg) -> Task<ChatMsg> {
        match msg {
            ChatMsg::InputChanged(val) => {
                self.set_input(val);
                self.slash_index = 0;
                self.slash_dismissed_at = None;
                Task::none()
            }
            ChatMsg::InputAction(action) => {
                self.input_content.perform(action);
                self.input = self.input_content.text();
                self.slash_index = 0;
                self.slash_dismissed_at = None;
                Task::none()
            }
            ChatMsg::Send => {
                let text = self.input.trim().to_string();
                if (text.is_empty() && self.attachments.is_empty()) || self.busy {
                    return Task::none();
                }
                self.set_input(String::new());
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
                if let Some(engine) = &self.engine {
                    engine.interrupt();
                }
                self.finish_streaming_items();
                self.busy = false;
                Task::none()
            }
            ChatMsg::ToggleTool(idx) => {
                if let Some(ChatItem::Tool(tool)) = self.items.get_mut(idx) {
                    tool.expanded = !tool.expanded;
                }
                Task::none()
            }
            ChatMsg::ToggleEditTools(indices) => {
                let expanded = indices
                    .iter()
                    .find_map(|i| {
                        if let Some(ChatItem::Tool(t)) = self.items.get(*i) {
                            Some(t.expanded)
                        } else {
                            None
                        }
                    })
                    .map(|v| !v)
                    .unwrap_or(true);
                for index in indices {
                    if let Some(ChatItem::Tool(tool)) = self.items.get_mut(index) {
                        tool.expanded = expanded;
                    }
                }
                Task::none()
            }
            ChatMsg::ToggleEditCard(index) => {
                if !self.collapsed_edit_cards.insert(index) {
                    self.collapsed_edit_cards.remove(&index);
                }
                Task::none()
            }
            ChatMsg::Tick => {
                self.spinner_rotation = (self.spinner_rotation + 0.1) % std::f32::consts::TAU;
                self.poll_events();
                self.download_remote_images()
            }
            ChatMsg::PermissionAnswer {
                request_id,
                allow,
                always,
            } => {
                if let Some(engine) = &self.engine {
                    engine.answer_permission(&request_id, allow, always, None, None);
                }
                self.items.retain(|item| !matches!(item, ChatItem::Permission { request_id: id, .. } if id == &request_id));
                Task::none()
            }
            ChatMsg::AnswerQuestion { request_id, input } => {
                if let Some(engine) = &self.engine {
                    engine.answer_permission(&request_id, true, false, Some(input), None);
                }
                self.items.retain(|item| !matches!(item, ChatItem::Permission { request_id: id, .. } if id == &request_id));
                Task::none()
            }
            ChatMsg::SetMode(mode) => {
                self.composer_menu = None;
                self.mode = mode.clone();
                if let Some(engine) = &self.engine {
                    engine.set_permission_mode(&mode);
                }
                Task::none()
            }
            ChatMsg::SetModel(model) => {
                self.composer_menu = None;
                self.model = Some(model.clone());
                if let Some(engine) = &self.engine {
                    engine.set_model(&model);
                }
                Task::none()
            }
            ChatMsg::SetEffort(effort) => {
                self.composer_menu = None;
                self.effort = Some(effort.clone());
                if let Some(engine) = &self.engine {
                    engine.set_effort(&effort);
                }
                Task::none()
            }
            ChatMsg::OpenComposerMenu(menu) => {
                self.composer_menu = Some(match menu {
                    ComposerMenu::Model => ComposerMenu::RefreshingModel,
                    ComposerMenu::Effort => ComposerMenu::RefreshingEffort,
                    other => other,
                });
                if matches!(menu, ComposerMenu::Model | ComposerMenu::Effort) {
                    self.update(ChatMsg::UsageRefresh)
                } else {
                    Task::none()
                }
            }
            ChatMsg::CloseComposerMenu => {
                self.composer_menu = None;
                Task::none()
            }
            ChatMsg::Attach => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_files()
                        .await
                        .map(|files| files.into_iter().map(|f| f.path().to_path_buf()).collect())
                },
                ChatMsg::PickFiles,
            ),
            ChatMsg::PickFiles(paths) => {
                if let Some(paths) = paths {
                    self.attachments
                        .extend(paths.iter().map(|p| Attachment::file(p)));
                }
                Task::none()
            }
            ChatMsg::RemoveAttachment(index) => {
                if index < self.attachments.len() {
                    self.attachments.remove(index);
                }
                Task::none()
            }
            ChatMsg::ToggleHistory => {
                self.history_open = !self.history_open;
                Task::none()
            }
            ChatMsg::ResumeSession(id) => {
                if let Some(session) = self.recent_sessions().into_iter().find(|s| s.id == id) {
                    let age = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0)
                        .saturating_sub(session.last_seen_ms);
                    if age < 120_000 {
                        self.items.push(ChatItem::ResumePrompt { session_id: id });
                    } else {
                        self.load_session(session);
                        return self.download_remote_images();
                    }
                }
                Task::none()
            }
            ChatMsg::RefreshUsage => {
                if let Some(engine) = &self.engine {
                    engine.request_usage();
                    Task::none()
                } else {
                    self.update(ChatMsg::UsageRefresh)
                }
            }
            ChatMsg::SelectCommand(command) => {
                self.set_input(command);
                self.slash_dismissed_at = None;
                self.slash_index = 0;
                Task::none()
            }
            ChatMsg::SlashKey(key) => {
                let matches = self.slash_matches();
                if matches.is_empty() {
                    return Task::none();
                }
                match key {
                    SlashKey::Up => {
                        self.slash_index = if self.slash_index == 0 {
                            matches.len() - 1
                        } else {
                            self.slash_index - 1
                        }
                    }
                    SlashKey::Down => self.slash_index = (self.slash_index + 1) % matches.len(),
                    SlashKey::Enter | SlashKey::Tab => {
                        if let Some(command) = matches.get(self.slash_index) {
                            self.set_input(format!("/{} ", command.name));
                        }
                        self.slash_index = 0;
                        self.slash_dismissed_at = None;
                    }
                    SlashKey::Escape => self.slash_dismissed_at = Some(self.input.clone()),
                }
                Task::none()
            }
            ChatMsg::PasteImage => Task::perform(
                async {
                    tokio::task::spawn_blocking(read_clipboard_png)
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                },
                ChatMsg::ClipboardImage,
            ),
            ChatMsg::ClipboardImage(Ok(Some(png))) => {
                self.attachments
                    .push(Attachment::pasted_image(png, "clipboard.png".into()));
                Task::none()
            }
            ChatMsg::ClipboardImage(Ok(None)) => Task::none(),
            ChatMsg::ClipboardImage(Err(error)) => {
                self.items.push(ChatItem::Error {
                    message: labels::format_chat_clipboard_failed(&error),
                });
                Task::none()
            }
            ChatMsg::BeginRename => {
                self.rename_text = self
                    .title
                    .clone()
                    .unwrap_or_else(|| self.conversation_title().to_string());
                self.rename_open = true;
                Task::none()
            }
            ChatMsg::RenameChanged(value) => {
                self.rename_text = value;
                Task::none()
            }
            ChatMsg::SaveRename => {
                let title = self.rename_text.trim().to_string();
                if title.is_empty() {
                    return Task::none();
                }
                if let (Some(cwd), Some(session_id)) =
                    (self.cwd.as_ref(), self.session_id.as_deref())
                {
                    if let Err(error) =
                        gsh_chat::write_title(&cwd.to_string_lossy(), session_id, &title)
                    {
                        self.items.push(ChatItem::Error {
                            message: labels::format_chat_title_failed(&error.to_string()),
                        });
                        return Task::none();
                    }
                }
                self.title = Some(title);
                self.rename_open = false;
                Task::none()
            }
            ChatMsg::CancelRename => {
                self.rename_open = false;
                Task::none()
            }
            ChatMsg::RefreshAccountResult(Ok(Some(probe))) => {
                self.models = probe.models;
                self.commands = probe.commands;
                self.usage = probe.usage;
                self.composer_menu = match self.composer_menu {
                    Some(ComposerMenu::RefreshingModel) if !self.models.is_empty() => {
                        Some(ComposerMenu::Model)
                    }
                    Some(ComposerMenu::RefreshingEffort) if !self.efforts().is_empty() => {
                        Some(ComposerMenu::Effort)
                    }
                    Some(ComposerMenu::RefreshingModel | ComposerMenu::RefreshingEffort) => None,
                    other => other,
                };
                Task::none()
            }
            ChatMsg::RefreshAccountResult(Ok(None)) => {
                self.composer_menu = match self.composer_menu {
                    Some(ComposerMenu::RefreshingModel) if !self.models.is_empty() => {
                        Some(ComposerMenu::Model)
                    }
                    Some(ComposerMenu::RefreshingEffort) if !self.efforts().is_empty() => {
                        Some(ComposerMenu::Effort)
                    }
                    Some(ComposerMenu::RefreshingModel | ComposerMenu::RefreshingEffort) => None,
                    other => other,
                };
                Task::none()
            }
            ChatMsg::RefreshAccountResult(Err(error)) => {
                self.composer_menu = None;
                self.items.push(ChatItem::Error {
                    message: labels::format_chat_account_failed(&error),
                });
                Task::none()
            }
            ChatMsg::UsageRefresh => {
                let binary = self.binary.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || gsh_chat::probe_account(&binary, &[]))
                            .await
                            .map_err(|e| e.to_string())
                    },
                    ChatMsg::RefreshAccountResult,
                )
            }
            ChatMsg::HandOff => {
                if let Some(url) = &self.remote_control_url {
                    self.items.push(ChatItem::Info {
                        text: format!("{}{url}", labels::CHAT_HANDOFF_ACTIVE),
                        warning: false,
                    });
                    return Task::none();
                }
                if self.session_id.is_none() {
                    self.items.push(ChatItem::Info {
                        text: labels::CHAT_HANDOFF_NO_SESSION.into(),
                        warning: true,
                    });
                    return Task::none();
                }
                if self.busy {
                    self.items.push(ChatItem::Info {
                        text: labels::CHAT_HANDOFF_BUSY.into(),
                        warning: true,
                    });
                    return Task::none();
                }
                self.handoff_requested = true;
                if self.handoff_requested {
                    self.items.push(ChatItem::Info {
                        text: labels::CHAT_HANDOFF_NOTICE.into(),
                        warning: false,
                    });
                    self.close();
                }
                Task::none()
            }
            ChatMsg::PickPeerSession(id) => {
                if let Some(session) = self.peer_sessions().into_iter().find(|s| s.id == id) {
                    self.set_input(format!(
                        "{}{}{}",
                        labels::CHAT_SEND_TO_SESSION_PREFIX,
                        session.title,
                        labels::CHAT_SEND_TO_SESSION_SUFFIX
                    ));
                } else {
                    self.set_input(labels::CHAT_ASK_LIST_SESSIONS.to_string());
                }
                self.peer_picker_open = false;
                Task::none()
            }
            ChatMsg::TogglePeerPicker => {
                self.peer_picker_open = !self.peer_picker_open;
                Task::none()
            }
            ChatMsg::ToggleUsage => {
                self.usage_open = !self.usage_open;
                if self.usage_open {
                    self.update(ChatMsg::RefreshUsage)
                } else {
                    Task::none()
                }
            }
            ChatMsg::QuestionToggle {
                request_id,
                question,
                option,
            } => {
                if let Some(ChatItem::Questions { input, questions, answered:None, .. }) = self.items.iter_mut().find(|item| matches!(item, ChatItem::Questions { request_id: id, .. } if id == &request_id)) {
                    let multi=questions.iter().find(|q|q.question==question).is_some_and(|q|q.multi_select);
                    let draft_answers=input.entry("draftAnswers").or_insert_with(||serde_json::json!({}));
                    let draft=draft_answers.as_object_mut().map(|m|m.entry(question).or_insert_with(||serde_json::json!({})));
                    if let Some(draft)=draft.and_then(serde_json::Value::as_object_mut) {
                        let selected=draft.entry("selected").or_insert_with(||serde_json::json!([]));
                        if let Some(list)=selected.as_array_mut() {
                            if list.iter().any(|v|v.as_str()==Some(&option)) {list.retain(|v|v.as_str()!=Some(&option));}
                            else if multi {list.push(serde_json::Value::String(option));}
                            else {list.clear();list.push(serde_json::Value::String(option));draft.insert("other".into(),serde_json::Value::String(String::new()));}
                        }
                    }
                }
                Task::none()
            }
            ChatMsg::QuestionFreeText {
                request_id,
                question,
                value,
            } => {
                if let Some(ChatItem::Questions { input, answered:None, .. }) = self.items.iter_mut().find(|item| matches!(item, ChatItem::Questions { request_id: id, .. } if id == &request_id)) {
                    let drafts=input.entry("draftAnswers").or_insert_with(||serde_json::json!({}));
                    if let Some(map)=drafts.as_object_mut() { let draft=map.entry(question).or_insert_with(||serde_json::json!({})); if let Some(draft)=draft.as_object_mut(){draft.insert("other".into(),serde_json::Value::String(value));} }
                }
                Task::none()
            }
            ChatMsg::SubmitQuestions { request_id } => {
                if let Some(ChatItem::Questions {
                    input,
                    questions,
                    answered: None,
                    ..
                }) = self
                    .items
                    .iter()
                    .find(|i| matches!(i,ChatItem::Questions{request_id:id,..} if id==&request_id))
                    .cloned()
                {
                    let mut answers = BTreeMap::new();
                    for q in &questions {
                        let draft = input.get("draftAnswers").and_then(|v| v.get(&q.question));
                        let selected = draft
                            .and_then(|v| v.get("selected"))
                            .and_then(serde_json::Value::as_array)
                            .into_iter()
                            .flatten()
                            .filter_map(serde_json::Value::as_str)
                            .collect::<Vec<_>>();
                        let other = draft
                            .and_then(|v| v.get("other"))
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("")
                            .trim();
                        let mut parts = selected;
                        if !other.is_empty() {
                            parts.push(other);
                        }
                        if !parts.is_empty() {
                            answers.insert(q.question.clone(), parts.join(", "));
                        }
                    }
                    if answers.len() == questions.len() {
                        let mut updated = input.clone();
                        updated.remove("draftAnswers");
                        updated.insert(
                            "answers".into(),
                            serde_json::to_value(&answers).unwrap_or(serde_json::Value::Null),
                        );
                        if let Some(engine) = &self.engine {
                            engine.answer_permission(&request_id, true, false, Some(updated), None);
                        }
                        if let Some(ChatItem::Questions{answered,..})=self.items.iter_mut().find(|i|matches!(i,ChatItem::Questions{request_id:id,..} if id==&request_id)){*answered=Some(answers);}
                    }
                }
                Task::none()
            }
            ChatMsg::PlanFeedback { request_id, value } => {
                if let Some(ChatItem::Plan { feedback, .. }) = self
                    .items
                    .iter_mut()
                    .find(|i| matches!(i, ChatItem::Plan { request_id:id, .. } if id==&request_id))
                {
                    *feedback = value;
                }
                Task::none()
            }
            ChatMsg::PlanDecision {
                request_id,
                approve,
            } => {
                let (feedback, input) = self
                    .items
                    .iter()
                    .find_map(|i| {
                        if let ChatItem::Plan {
                            request_id: id,
                            feedback,
                            input,
                            ..
                        } = i
                        {
                            (id == &request_id).then(|| (feedback.clone(), input.clone()))
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                if let Some(engine) = &self.engine {
                    engine.answer_permission(
                        &request_id,
                        approve,
                        false,
                        approve.then_some(input),
                        (!approve).then_some(feedback.as_str()),
                    );
                }
                self.items
                    .retain(|i| !matches!(i,ChatItem::Plan{request_id:id,..} if id==&request_id));
                Task::none()
            }
            ChatMsg::OpenLink(url) => {
                if url.starts_with("https://")
                    || url.starts_with("http://")
                    || url.starts_with("mailto:")
                {
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || open_external_url(&url))
                                .await
                                .map_err(|e| e.to_string())
                                .and_then(|r| r)
                        },
                        |_| ChatMsg::Tick,
                    )
                } else {
                    Task::none()
                }
            }
            ChatMsg::RemoteImageLoaded { url, bytes } => {
                self.remote_images_pending.remove(&url);
                if let Ok(bytes) = bytes {
                    self.remote_images.insert(url, bytes);
                }
                Task::none()
            }
            ChatMsg::ConfirmResume {
                session_id,
                confirm,
            } => {
                if confirm {
                    if let Some(session) = self
                        .recent_sessions()
                        .into_iter()
                        .find(|s| s.id == session_id)
                    {
                        self.load_session(session);
                        return self.download_remote_images();
                    }
                } else {
                    self.items.retain(
                        |i| !matches!(i,ChatItem::ResumePrompt{session_id:id} if id==&session_id),
                    );
                }
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
                session_id,
                model,
                permission_mode,
            } => {
                if !session_id.is_empty() {
                    self.session_id = Some(session_id);
                }
                if !model.is_empty() {
                    self.model = Some(model);
                }
                if let Some(mode) = permission_mode {
                    self.mode = mode;
                }
            }
            ChatEvent::Models(models) => self.models = models,
            ChatEvent::Commands(commands) => self.commands = commands,
            ChatEvent::ModeChanged(mode) => self.mode = mode,
            ChatEvent::ModelChanged(model) => self.model = Some(model),
            ChatEvent::Context { tokens, window } => self.context = (tokens, window),
            ChatEvent::PlanUsage(usage) => self.usage = usage,
            ChatEvent::Text { text, .. } => {
                if let Some(ChatItem::Assistant {
                    text: current,
                    streaming: true,
                }) = self.items.last_mut()
                {
                    *current = text;
                } else {
                    self.items.push(ChatItem::Assistant {
                        text,
                        streaming: false,
                    });
                }
            }
            ChatEvent::UserText { text, .. } => self.items.push(ChatItem::User { text }),
            ChatEvent::Renamed(title) => self.title = Some(title),
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
            ChatEvent::ThinkingDelta(delta) => {
                if let Some(ChatItem::Thinking { text }) = self.items.last_mut() {
                    text.push_str(&delta);
                } else {
                    self.items.push(ChatItem::Thinking { text: delta });
                }
            }
            ChatEvent::Thinking(text) => self.items.push(ChatItem::Thinking { text }),
            ChatEvent::SessionClassified { state, detail } => {
                self.session_state = Some((state, detail.clone()));
                self.items.push(ChatItem::Classified { state, detail });
            }
            ChatEvent::PeerMessage {
                from_name, text, ..
            } => self.items.push(ChatItem::Peer {
                from: from_name,
                text,
            }),
            ChatEvent::Info { text, warning } => {
                if let Some(start) = text.find("https://claude.ai/code/session_") {
                    let end = text[start..]
                        .find(char::is_whitespace)
                        .map(|n| start + n)
                        .unwrap_or(text.len());
                    self.remote_control_url = text.get(start..end).map(ToOwned::to_owned);
                }
                self.items.push(ChatItem::Info { text, warning });
            }
            ChatEvent::PermissionAsk {
                request_id,
                tool_name,
                description,
                always_rule,
                input,
            } => {
                if tool_name == "AskUserQuestion" {
                    let questions = input
                        .get("questions")
                        .and_then(serde_json::Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|q| {
                            let q = q.as_object()?;
                            let question = q.get("question")?.as_str()?.to_string();
                            let options = q
                                .get("options")
                                .and_then(serde_json::Value::as_array)
                                .into_iter()
                                .flatten()
                                .filter_map(|o| {
                                    let o = o.as_object()?;
                                    Some((
                                        o.get("label")?.as_str()?.to_string(),
                                        o.get("description")
                                            .and_then(serde_json::Value::as_str)
                                            .unwrap_or("")
                                            .to_string(),
                                    ))
                                })
                                .collect();
                            Some(ChatQuestion {
                                question,
                                header: q
                                    .get("header")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                options,
                                multi_select: q
                                    .get("multiSelect")
                                    .and_then(serde_json::Value::as_bool)
                                    == Some(true),
                            })
                        })
                        .collect();
                    self.items.push(ChatItem::Questions {
                        request_id,
                        input,
                        questions,
                        answered: None,
                    });
                    return;
                }
                if tool_name == "ExitPlanMode" {
                    let plan = input
                        .get("plan")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("")
                        .to_string();
                    self.items.push(ChatItem::Plan {
                        request_id,
                        plan,
                        input,
                        feedback: String::new(),
                    });
                    return;
                }
                let first_question = input
                    .get("questions")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|questions| questions.first())
                    .and_then(serde_json::Value::as_object);
                let question = first_question
                    .and_then(|q| q.get("question"))
                    .and_then(serde_json::Value::as_str)
                    .map(ToOwned::to_owned);
                let options = first_question
                    .and_then(|q| q.get("options"))
                    .and_then(serde_json::Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(|v| {
                                let o = v.as_object()?;
                                let label = o.get("label")?.as_str()?.to_string();
                                let description = o
                                    .get("description")
                                    .and_then(serde_json::Value::as_str)
                                    .unwrap_or("")
                                    .to_string();
                                Some((label, description))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                self.items.push(ChatItem::Permission {
                    request_id,
                    tool: tool_name,
                    description: description.unwrap_or_default(),
                    always_rule,
                    input,
                    question,
                    options,
                });
            }
            ChatEvent::ToolCall { id, name, input } => {
                let summary = gsh_chat::protocol::tool_summary(&input);
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
                    input,
                }));
            }
            ChatEvent::ToolResult {
                tool_use_id,
                is_error,
                text,
            } => {
                for item in self.items.iter_mut().rev() {
                    if let ChatItem::Tool(tool) = item {
                        if tool.id == tool_use_id {
                            tool.done = true;
                            tool.is_error = is_error;
                            tool.result = text;
                            break;
                        }
                    }
                }
            }
            ChatEvent::TurnEnd {
                is_error,
                text,
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
                if is_error && !text.trim().is_empty() {
                    self.items.push(ChatItem::Error {
                        message: labels::format_chat_turn_failed(text.trim()),
                    });
                }
            }
            ChatEvent::EngineFailed { exit_code, stderr } => {
                self.finish_streaming_items();
                self.busy = false;
                self.engine = None;
                self.items.push(ChatItem::Error {
                    message: labels::format_chat_engine_exited(exit_code, &stderr),
                });
            }
        }
        self.refresh_markdown();
    }

    /// Ticker periódico (~16 ms) ativo somente enquanto há um turno em execução (`busy`).
    pub fn subscription(&self) -> Subscription<ChatMsg> {
        let tick = if self.busy {
            iced::time::every(Duration::from_millis(TICK_INTERVAL_MS)).map(|_| ChatMsg::Tick)
        } else {
            Subscription::none()
        };
        Subscription::batch([tick, iced::event::listen_with(file_drop)])
    }

    /// Renderiza o pane de chat.
    pub fn view(&self, transcript_width: f32) -> Element<'_, ChatMsg> {
        chat_view::view(self, transcript_width)
    }

    fn ensure_engine_and_send(&mut self, text: &str) -> Result<(), ChatError> {
        if self.engine.is_none() {
            let cfg = ChatConfig {
                binary: gsh_chat::resolve_executable(&self.binary.to_string_lossy()),
                cwd: self.cwd.clone(),
                env: Vec::new(),
                launch: LaunchOptions {
                    resume_session_id: self.session_id.clone(),
                    model: self.model.clone(),
                    effort: self.effort.clone(),
                    accept_peers: gsh_chat::settings::accept_peers(),
                    ..LaunchOptions::default()
                },
            };
            self.engine = Some(ChatEngine::spawn(cfg)?);
            if let Some(engine) = &self.engine {
                engine.set_permission_mode(&self.mode);
            }
        }
        if let Some(engine) = self.engine.as_mut() {
            engine.send(text, &self.attachments)?;
            self.attachments.clear();
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

    fn refresh_markdown(&mut self) {
        self.markdown.clear();
        for (index, item) in self.items.iter().enumerate() {
            let markdown_text = match item {
                ChatItem::Assistant { text, .. } | ChatItem::Plan { plan: text, .. } => {
                    Some(text.as_str())
                }
                _ => None,
            };
            if let Some(text) = markdown_text {
                self.markdown
                    .insert(index, iced::widget::markdown::Content::parse(text));
            }
        }
    }

    fn download_remote_images(&mut self) -> Task<ChatMsg> {
        const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
        let mut urls = std::collections::HashSet::new();
        for item in &self.items {
            let content = match item {
                ChatItem::Assistant { text, .. } | ChatItem::Plan { plan: text, .. } => text,
                _ => continue,
            };
            for event in pulldown_cmark::Parser::new(content) {
                if let pulldown_cmark::Event::Start(pulldown_cmark::Tag::Image {
                    dest_url, ..
                }) = event
                {
                    if (dest_url.starts_with("https://") || dest_url.starts_with("http://"))
                        && !self.remote_images.contains_key(dest_url.as_ref())
                        && !self.remote_images_pending.contains(dest_url.as_ref())
                    {
                        urls.insert(dest_url.into_string());
                    }
                }
            }
        }
        let tasks = urls.into_iter().map(|url| {
            self.remote_images_pending.insert(url.clone());
            Task::perform(
                async move {
                    let result = async {
                        let client = reqwest::Client::builder()
                            .timeout(Duration::from_secs(8))
                            .build()
                            .map_err(|error| error.to_string())?;
                        let mut response = client
                            .get(&url)
                            .send()
                            .await
                            .map_err(|error| error.to_string())?;
                        if !response.status().is_success() {
                            return Err(format!("HTTP {}", response.status()));
                        }
                        if !response
                            .headers()
                            .get(reqwest::header::CONTENT_TYPE)
                            .and_then(|value| value.to_str().ok())
                            .is_some_and(|value| value.starts_with("image/"))
                        {
                            return Err("Resposta não é uma imagem".to_string());
                        }
                        if response
                            .content_length()
                            .is_some_and(|size| size > MAX_IMAGE_BYTES as u64)
                        {
                            return Err("Imagem excede 8 MiB".to_string());
                        }
                        let mut bytes = Vec::new();
                        while let Some(chunk) =
                            response.chunk().await.map_err(|error| error.to_string())?
                        {
                            if bytes.len().saturating_add(chunk.len()) > MAX_IMAGE_BYTES {
                                return Err("Imagem excede 8 MiB".to_string());
                            }
                            bytes.extend_from_slice(&chunk);
                        }
                        let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                            .with_guessed_format()
                            .map_err(|error| error.to_string())?;
                        let mut limits = image::Limits::default();
                        limits.max_image_width = Some(4096);
                        limits.max_image_height = Some(4096);
                        limits.max_alloc = Some(128 * 1024 * 1024);
                        reader.limits(limits);
                        let decoded = reader.decode().map_err(|error| error.to_string())?;
                        let thumbnail = decoded.thumbnail(1600, 1200);
                        let mut normalized = std::io::Cursor::new(Vec::new());
                        thumbnail
                            .write_to(&mut normalized, image::ImageFormat::Png)
                            .map_err(|error| error.to_string())?;
                        Ok(normalized.into_inner())
                    }
                    .await;
                    (url, result)
                },
                |(url, bytes)| ChatMsg::RemoteImageLoaded { url, bytes },
            )
        });
        Task::batch(tasks)
    }

    fn load_session(&mut self, session: gsh_chat::SessionEntry) {
        self.close();
        self.items.clear();
        self.session_id = Some(session.id.clone());
        self.cwd = Some(PathBuf::from(&session.project));
        self.title = Some(session.title);
        let cwd = self
            .cwd
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default();
        for event in gsh_chat::read_history(&cwd, &session.id) {
            self.apply_event(event);
        }
        self.history_open = false;
        self.busy = false;
        self.refresh_markdown();
    }
}

fn open_external_url(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let (program, args) = ("open", vec![url]);
    #[cfg(target_os = "windows")]
    let (program, args) = ("rundll32", vec!["url.dll,FileProtocolHandler", url]);
    #[cfg(all(unix, not(target_os = "macos")))]
    let (program, args) = ("xdg-open", vec![url]);
    std::process::Command::new(program)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn read_clipboard_png() -> Result<Option<Vec<u8>>, String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    let image = match clipboard.get_image() {
        Ok(image) => image,
        Err(arboard::Error::ContentNotAvailable) => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let width = u32::try_from(image.width).map_err(|e| e.to_string())?;
    let height = u32::try_from(image.height).map_err(|e| e.to_string())?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > 40_000_000 {
        return Err("dimensões de imagem inválidas ou acima do limite".into());
    }
    let pixels = image.bytes.into_owned();
    let rgba = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| "dados RGBA inválidos".to_string())?;
    let mut output = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut output, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let png = output.into_inner();
    if png.len() > gsh_chat::MAX_IMAGE_BYTES {
        return Err("a imagem excede o limite de 5 MiB".into());
    }
    Ok(Some(png))
}

fn file_drop(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<ChatMsg> {
    match event {
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(ChatMsg::PickFiles(Some(vec![path])))
        }
        _ => None,
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
        pane.apply_event(ChatEvent::ToolCall {
            id: "tool_1".to_string(),
            name: "Bash".to_string(),
            input: json!({"command": "cargo test"})
                .as_object()
                .cloned()
                .unwrap_or_default(),
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
            text: "ok".to_string(),
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

    #[test]
    fn composer_editor_preserves_multiline_input() {
        let mut pane = ChatPane::default();
        for action in [
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Insert('a')),
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Enter),
            iced::widget::text_editor::Action::Edit(iced::widget::text_editor::Edit::Insert('b')),
        ] {
            let _ = pane.update(ChatMsg::InputAction(action));
        }
        assert_eq!(pane.input(), "a\nb");
        assert_eq!(pane.input_content().line_count(), 2);
    }

    #[test]
    fn pergunta_multiplas_perguntas_multiselect_e_resposta_livre() {
        let mut pane = ChatPane::new(None, None).with_static_test_mode(true);
        pane.apply_event(ChatEvent::PermissionAsk{request_id:"ask-1".into(),tool_name:"AskUserQuestion".into(),description:Some(String::new()),always_rule:None,input:json!({"questions":[{"header":"ESCOPO","question":"Quais?","multiSelect":true,"options":[{"label":"A","description":"um"},{"label":"B","description":"dois"}]},{"header":"PRAZO","question":"Quando?","multiSelect":false,"options":[]}]}).as_object().cloned().unwrap_or_default()});
        let _ = pane.update(ChatMsg::QuestionToggle {
            request_id: "ask-1".into(),
            question: "Quais?".into(),
            option: "A".into(),
        });
        let _ = pane.update(ChatMsg::QuestionToggle {
            request_id: "ask-1".into(),
            question: "Quais?".into(),
            option: "B".into(),
        });
        let _ = pane.update(ChatMsg::QuestionFreeText {
            request_id: "ask-1".into(),
            question: "Quando?".into(),
            value: "amanhã".into(),
        });
        let _ = pane.update(ChatMsg::SubmitQuestions {
            request_id: "ask-1".into(),
        });
        assert!(
            matches!(pane.items().last(),Some(ChatItem::Questions{answered:Some(a),..}) if a.get("Quais?").is_some_and(|v|v=="A, B") && a.get("Quando?").is_some_and(|v|v=="amanhã"))
        );
    }
}
