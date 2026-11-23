//! Estado e persistência das configurações gerais.

mod appearance;
pub mod claude;
mod controls;
pub mod localsend;
pub mod view;

use crate::theme::{self, ThemeVariant};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    InvalidLocale,
    InvalidInterfaceSize,
    InvalidTheme,
    InvalidShell,
    InvalidScrollback,
    InvalidTerminalSize,
    InvalidTabSize,
    InvalidLocalSendLimit,
    InvalidLspSettings,
    InvalidAutomation,
    InvalidSection,
    Config(gsh_config::ConfigError),
    Io(std::io::Error),
    Document(toml_edit::TomlError),
}

#[derive(Debug, Clone)]
pub enum Message {
    Back,
    SearchChanged(String),
    SideChanged(bool),
    LocaleChanged(String),
    Save,
    ShowAppearance(bool),
    InterfaceSizeChanged(String),
    SaveAppearance,
    Select(Section),
    ShellChanged(String),
    ScrollbackChanged(String),
    TerminalSizeChanged(String),
    TabSizeChanged(String),
    HighlightChanged(bool),
    WrapChanged(bool),
    NotificationsChanged(bool),
    PermissionSoundChanged(bool),
    TranscriptWidthChanged(gsh_config::schema::TranscriptWidth),
    ThemeSelected(gsh_config::schema::AppThemeMode),
    TeamEnabledChanged(bool),
    TeamNameChanged(String),
    TeamDisplayNameChanged(String),
    TeamShowTitlesChanged(bool),
    TeamSecretChanged(String),
    TeamNetworkChanged(String),
    TeamDoorAllowChanged(String),
    TeamDoorAskChanged(bool),
    SaveControls,
    AutomationHarnessChanged(String),
    AutomationModelChanged(String),
    RoutineNameChanged(String),
    RoutineTimeChanged(String),
    RoutineWeekdayToggled(usize),
    RoutineWorkspaceChanged(String),
    RoutineHarnessChanged(String),
    RoutinePromptChanged(String),
    RoutineAdd,
    RoutineToggle(String, bool),
    RoutineRunNow(String),
    RoutineRemove(String),
    RoutineOpenLog(PathBuf),
    FeatureVisibilityChanged(gsh_config::schema::AppFeature, bool),
    LspCommandsAction(iced::widget::text_editor::Action),
    LspFormattersAction(iced::widget::text_editor::Action),
    LocalSend(LocalSendMessage),
    Claude(claude::Message),
}

#[derive(Debug, Clone)]
pub enum LocalSendMessage {
    PopupTabChanged(LocalSendPopupTab),
    ClosePopup,
    PickFiles,
    PickFolder,
    FolderPicked(Option<PathBuf>),
    FolderFilesCollected(Result<Vec<PathBuf>, String>),
    FilesPicked(Vec<PathBuf>),
    FilesPickedResult(Result<Vec<PathBuf>, String>),
    TextChanged(String),
    SendText,
    PasteClipboard,
    ClipboardLoaded(Result<Vec<PathBuf>, String>),
    RefreshPeers,
    SendSelectedTo(String),
    RestartServer,
    StopServer,
    AlwaysOnChanged(bool),
    AliasChanged(String),
    ChooseReceiveDir,
    ReceiveDirPicked(Option<PathBuf>),
    LimitsChanged {
        max_file_mb: String,
        max_files: String,
    },
    SendTo(String),
    SendFilesPicked(String, Vec<PathBuf>),
    CancelSend,
    AcceptReceive,
    RejectReceive,
    ShowReceived,
    OpenReceivedFolder,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocalSendPopupTab {
    #[default]
    Receive,
    Send,
    SettingsReceive,
    SettingsNetwork,
}

#[derive(Debug, Clone)]
pub struct LocalSendState {
    pub popup_open: bool,
    pub popup_tab: LocalSendPopupTab,
    pub draft_text: String,
    pub selected_files: Vec<PathBuf>,
    pub transfer_status: Option<String>,
    pub server_alias: Option<String>,
    pub always_on: bool,
    pub alias: String,
    pub receive_dir: PathBuf,
    pub max_file_mb: String,
    pub max_files: String,
    pub server_status: Option<String>,
    pub error: Option<String>,
    pub peers: Vec<LocalSendPeer>,
    pub incoming: Option<LocalSendIncoming>,
    pub transfer: Option<LocalSendTransfer>,
    pub received_dir: Option<PathBuf>,
}

impl Default for LocalSendState {
    fn default() -> Self {
        let config = gsh_config::schema::LocalSend::default();
        Self {
            popup_open: false,
            popup_tab: LocalSendPopupTab::Receive,
            draft_text: String::new(),
            selected_files: Vec::new(),
            transfer_status: None,
            server_alias: None,
            always_on: config.always_on,
            alias: config.alias,
            receive_dir: PathBuf::from(config.receive_dir),
            max_file_mb: config.max_file_mb.to_string(),
            max_files: config.max_files.to_string(),
            server_status: None,
            error: None,
            peers: Vec::new(),
            incoming: None,
            transfer: None,
            received_dir: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSendPeer {
    pub alias: String,
    pub device_type: String,
    pub ip: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSendFile {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSendIncoming {
    pub sender_alias: String,
    pub sender_model: Option<String>,
    pub sender_ip: String,
    pub files: Vec<LocalSendFile>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSendTransfer {
    pub peer_alias: String,
    pub file_name: String,
    pub bytes: u64,
    pub total_bytes: u64,
    pub sending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Appearance,
    Terminal,
    Editor,
    Notifications,
    Team,
    LocalSend,
    Automations,
    LanguageServers,
    Features,
}

pub struct Settings {
    path: Result<PathBuf, gsh_config::ConfigError>,
    pub nav_search: String,
    pub nav_claude: bool,
    pub locale: String,
    pub appearance: bool,
    pub section: Section,
    pub shell: String,
    pub scrollback: String,
    pub terminal_size: String,
    pub tab_size: String,
    pub highlight: bool,
    pub wrap: bool,
    pub notifications: bool,
    pub permission_sound: bool,
    pub transcript_width: gsh_config::schema::TranscriptWidth,
    pub theme_mode: gsh_config::schema::AppThemeMode,
    pub current: gsh_config::Config,
    pub interface_size: String,
    pub scale: f32,
    pub error: Option<Error>,
    pub saved: bool,
    pub claude: claude::ClaudeState,
    pub localsend: LocalSendState,
    pub lsp_commands: iced::widget::text_editor::Content,
    pub lsp_formatters: iced::widget::text_editor::Content,
    pub routines: crate::scheduled_automations::ScheduledStore,
    pub routine_name: String,
    pub routine_time: String,
    pub routine_weekdays: [bool; 7],
    pub routine_workspace: String,
    pub routine_harness: String,
    pub routine_prompt: String,
    pub routine_error: Option<String>,
    pub pending_routine_run: Option<crate::scheduled_automations::ScheduledRoutine>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path: gsh_config::default_path(),
            nav_search: String::new(),
            nav_claude: false,
            locale: String::new(),
            appearance: false,
            section: Section::General,
            shell: gsh_term::default_shell().to_owned(),
            scrollback: String::from("10000"),
            terminal_size: String::from("13"),
            tab_size: String::from("4"),
            highlight: true,
            wrap: false,
            notifications: true,
            permission_sound: true,
            transcript_width: gsh_config::schema::TranscriptWidth::Medium,
            theme_mode: gsh_config::schema::AppThemeMode::System,
            current: gsh_config::Config::default(),
            interface_size: String::from("14"),
            scale: 1.0,
            error: None,
            saved: false,
            claude: claude::ClaudeState::default(),
            localsend: LocalSendState::default(),
            lsp_commands: iced::widget::text_editor::Content::new(),
            lsp_formatters: iced::widget::text_editor::Content::new(),
            routines: crate::scheduled_automations::ScheduledStore::load_default(),
            routine_name: String::new(),
            routine_time: String::from("09:00"),
            routine_weekdays: crate::scheduled_automations::every_day(),
            routine_workspace: String::new(),
            routine_harness: String::from("claude"),
            routine_prompt: String::new(),
            routine_error: None,
            pending_routine_run: None,
        }
    }
}

impl Settings {
    pub fn persist_chat_preference(&mut self, key: &str, value: &str) {
        let result = match &self.path {
            Ok(path) => controls::save_chat_preference(path, key, value),
            Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
        };
        match result {
            Ok(config) => {
                self.current = config;
                self.error = None;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn reload(&mut self) {
        self.saved = false;
        self.claude.reset();
        match self.read_config() {
            Ok(config) => self.apply_config(config),
            Err(error) => self.error = Some(Error::Config(error)),
        }
    }

    /// Reconsulta o disco porque um aviso do observador pode chegar depois de outras gravações.
    pub fn reload_watched(&mut self) -> Option<iced::Task<Message>> {
        let config = match self.read_config() {
            Ok(config) => config,
            Err(error) => {
                self.error = Some(Error::Config(error));
                return None;
            }
        };
        if config == self.current {
            if matches!(self.error, Some(Error::Config(_))) {
                self.error = None;
            }
            return None;
        }
        let profile_changed = claude::update::profile_binding(&self.current.claude)
            != claude::update::profile_binding(&config.claude);
        self.saved = false;
        self.apply_config(config);
        Some(if profile_changed {
            claude::update::refresh_after_profile_change(self).map(Message::Claude)
        } else {
            iced::Task::none()
        })
    }

    fn read_config(&self) -> Result<gsh_config::Config, gsh_config::ConfigError> {
        let path = self
            .path
            .as_ref()
            .map_err(|_| gsh_config::ConfigError::NoHome)?;
        gsh_config::load(path)
    }

    fn apply_config(&mut self, config: gsh_config::Config) {
        self.locale = config.locale.clone().unwrap_or_default();
        self.shell = config
            .terminal
            .shell
            .clone()
            .unwrap_or_else(|| gsh_term::default_shell().to_owned());
        self.scrollback = config.terminal.scrollback.to_string();
        self.terminal_size = config.effective_terminal_size().to_string();
        self.tab_size = config.editor.tab_size.to_string();
        self.highlight = config.editor.syntax_highlight;
        self.wrap = config.editor.word_wrap;
        self.notifications = config.notifications_enabled;
        self.permission_sound = config
            .sound
            .events
            .get(&gsh_config::schema::SoundEvent::ActionRequired)
            .copied()
            .unwrap_or(true);
        self.transcript_width = config.chat.transcript_width;
        self.theme_mode = config.theme_mode;
        let variant = ThemeVariant::from_mode(config.theme_mode);
        let spec = claude::themes::selected_theme_json(&self.claude.home, &config.theme_id);
        if !theme::set_theme_palette(&config.theme_id, spec.as_deref(), variant) {
            theme::set_active(variant);
        }
        self.lsp_commands =
            iced::widget::text_editor::Content::with_text(&format_lsp_map(&config.lsp_commands));
        self.lsp_formatters =
            iced::widget::text_editor::Content::with_text(&format_lsp_map(&config.lsp_formatters));
        self.localsend.always_on = config.localsend.always_on;
        self.localsend.alias.clone_from(&config.localsend.alias);
        self.localsend.receive_dir = PathBuf::from(&config.localsend.receive_dir);
        self.localsend.max_file_mb = config.localsend.max_file_mb.to_string();
        self.localsend.max_files = config.localsend.max_files.to_string();
        self.interface_size = config.interface_size.to_string();
        self.scale = (config.interface_size.clamp(10.0, 20.0) / 14.0) as f32;
        self.current = config;
        self.error = None;
    }

    /// Grava o tema escolhido e o aplica ao vivo; se não gravar, mantém o anterior.
    pub(super) fn select_theme(&mut self, mode: gsh_config::schema::AppThemeMode) {
        let result = match &self.path {
            Ok(path) => appearance::save_theme(path, mode.name()),
            Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
        };
        if let Ok(saved) = &result {
            self.theme_mode = *saved;
            self.current.theme_mode = *saved;
            let variant = ThemeVariant::from_mode(*saved);
            let spec =
                claude::themes::selected_theme_json(&self.claude.home, &self.current.theme_id);
            if !theme::set_theme_palette(&self.current.theme_id, spec.as_deref(), variant) {
                theme::set_active(variant);
            }
        }
        self.saved = result.is_ok();
        self.error = result.err();
    }

    pub fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::Back => return claude::update::commit_pending(self).map(Message::Claude),
            Message::SearchChanged(value) => self.nav_search = value,
            Message::SideChanged(claude) => {
                let task = claude::update::commit_pending(self).map(Message::Claude);
                if self.nav_claude && !claude {
                    self.section = Section::General;
                } else if !self.nav_claude && claude {
                    self.claude.tab = claude::Tab::General;
                    self.claude.refresh_snapshot();
                }
                self.nav_claude = claude;
                return task;
            }
            Message::Claude(message) => {
                return claude::update::update(self, message).map(Message::Claude);
            }
            Message::Select(section) => {
                self.section = section;
                self.error = None;
                self.saved = false;
            }
            Message::ShellChanged(v) => {
                self.shell = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::ScrollbackChanged(v) => {
                self.scrollback = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::TerminalSizeChanged(v) => {
                self.terminal_size = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::TabSizeChanged(v) => {
                self.tab_size = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::HighlightChanged(v) => {
                self.highlight = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::WrapChanged(v) => {
                self.wrap = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::NotificationsChanged(v) => {
                self.notifications = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::PermissionSoundChanged(v) => {
                self.permission_sound = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::TranscriptWidthChanged(v) => {
                self.transcript_width = v;
                self.saved = false;
                self.error = None;
                let _ = self.update(Message::SaveControls);
            }
            Message::ThemeSelected(mode) => self.select_theme(mode),
            Message::AutomationHarnessChanged(value) => {
                self.current.automation.harness_id = match value.as_str() {
                    "Antigravity" => Some(gsh_config::schema::HarnessKind::Antigravity),
                    "Claude Code" => Some(gsh_config::schema::HarnessKind::ClaudeCode),
                    "Codex" => Some(gsh_config::schema::HarnessKind::Codex),
                    "Cursor" => Some(gsh_config::schema::HarnessKind::Cursor),
                    "GitHub Copilot" => Some(gsh_config::schema::HarnessKind::GitHubCopilot),
                    "OpenCode" => Some(gsh_config::schema::HarnessKind::OpenCode),
                    "Pi" => Some(gsh_config::schema::HarnessKind::Pi),
                    _ => None,
                };
                self.save_current_section(Section::Automations);
            }
            Message::AutomationModelChanged(value) => {
                let model = value.trim();
                if model.len() > 128 || model.chars().any(char::is_control) {
                    self.error = Some(Error::InvalidAutomation);
                } else {
                    self.current.automation.model_id =
                        (!model.is_empty()).then(|| model.to_owned());
                    self.save_current_section(Section::Automations);
                }
            }
            Message::RoutineNameChanged(v) => {
                self.routine_name = v;
                self.routine_error = None;
            }
            Message::RoutineTimeChanged(v) => {
                self.routine_time = v;
                self.routine_error = None;
            }
            Message::RoutineWeekdayToggled(day) => {
                if let Some(selected) = self.routine_weekdays.get_mut(day) {
                    *selected = !*selected;
                    self.routine_error = None;
                }
            }
            Message::RoutineWorkspaceChanged(v) => {
                self.routine_workspace = v;
                self.routine_error = None;
            }
            Message::RoutineHarnessChanged(v) => {
                self.routine_harness = v;
                self.routine_error = None;
            }
            Message::RoutinePromptChanged(v) => {
                self.routine_prompt = v;
                self.routine_error = None;
            }
            Message::RoutineAdd => {
                let ws = if self.routine_workspace.trim().is_empty() {
                    self.claude
                        .workspace
                        .as_ref()
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default()
                } else {
                    self.routine_workspace.clone()
                };
                match self.routines.add(
                    &self.routine_name,
                    &self.routine_time,
                    self.routine_weekdays,
                    &ws,
                    &self.routine_harness,
                    &self.routine_prompt,
                ) {
                    Ok(_) => {
                        self.routine_name.clear();
                        self.routine_prompt.clear();
                        self.routine_error = None;
                    }
                    Err(err) => {
                        self.routine_error = Some(err);
                    }
                }
            }
            Message::RoutineToggle(id, enabled) => {
                let _ = self.routines.set_enabled(&id, enabled);
            }
            Message::RoutineRunNow(id) => {
                self.pending_routine_run = self.routines.manual_run(&id);
            }
            Message::RoutineRemove(id) => {
                let _ = self.routines.remove(&id);
            }
            Message::RoutineOpenLog(_) => {}
            Message::FeatureVisibilityChanged(feature, visible) => {
                if visible {
                    self.current.features.hidden.remove(&feature);
                } else {
                    self.current.features.hidden.insert(feature);
                }
                self.save_current_section(Section::Features);
            }
            Message::LspCommandsAction(action) => {
                self.lsp_commands.perform(action);
                self.save_lsp_map(true);
            }
            Message::LspFormattersAction(action) => {
                self.lsp_formatters.perform(action);
                self.save_lsp_map(false);
            }
            Message::LocalSend(message) => self.update_localsend(message),
            Message::TeamEnabledChanged(value) => {
                self.current.farm.team.enabled = value;
                self.save_team();
            }
            Message::TeamNameChanged(value) => {
                self.current.farm.team.name = value;
                self.save_team();
            }
            Message::TeamDisplayNameChanged(value) => {
                self.current.farm.team.me = value;
                self.save_team();
            }
            Message::TeamShowTitlesChanged(value) => {
                self.current.farm.team.show_titles = value;
                self.save_team();
            }
            Message::TeamSecretChanged(value) => {
                self.current.farm.team.secret = value;
                self.save_team();
            }
            Message::TeamNetworkChanged(value) => {
                self.current.farm.team.network = value;
                self.save_team();
            }
            Message::TeamDoorAllowChanged(value) => {
                self.current.farm.team.door_allow = value;
                self.save_team();
            }
            Message::TeamDoorAskChanged(value) => {
                self.current.farm.team.door_ask = value;
                self.save_team();
            }
            Message::SaveControls => {
                let result = match &self.path {
                    Ok(path) => controls::save(path, self),
                    Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
                };
                if let Ok(config) = &result {
                    self.current = config.clone();
                    self.scale = (config.interface_size.clamp(10.0, 20.0) / 14.0) as f32;
                }
                self.saved = result.is_ok();
                self.error = result.err();
            }
            Message::ShowAppearance(value) => {
                self.appearance = value;
                self.error = None;
                self.saved = false;
            }
            Message::InterfaceSizeChanged(value) => {
                self.interface_size = value;
                self.error = None;
                self.saved = false;
                let _ = self.update(Message::SaveControls);
            }
            Message::SaveAppearance => {
                let result = match &self.path {
                    Ok(path) => appearance::save(path, &self.interface_size),
                    Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
                };
                if let Ok(size) = &result {
                    self.scale = (*size / 14.0) as f32;
                }
                self.saved = result.is_ok();
                self.error = result.err();
            }
            Message::LocaleChanged(value) => {
                self.locale = value;
                self.error = None;
                self.saved = false;
                let _ = self.update(Message::Save);
            }
            Message::Save => {
                let result = match &self.path {
                    Ok(path) => save_locale(path, &self.locale),
                    Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
                };
                if result.is_ok() {
                    self.current.locale = if self.locale.is_empty() {
                        None
                    } else {
                        Some(self.locale.clone())
                    };
                }
                self.saved = result.is_ok();
                self.error = result.err();
            }
        }
        iced::Task::none()
    }

    fn save_team(&mut self) {
        self.error = None;
        self.saved = false;
        let result = match &self.path {
            Ok(path) => controls::save_team(path, &self.current.farm.team),
            Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
        };
        self.saved = result.is_ok();
        self.error = result.err();
    }

    fn save_current_section(&mut self, section: Section) {
        self.section = section;
        self.saved = false;
        self.error = None;
        let result = match &self.path {
            Ok(path) => controls::save(path, self),
            Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
        };
        match result {
            Ok(config) => {
                self.current = config;
                self.saved = true;
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn save_lsp_map(&mut self, commands: bool) {
        let text = if commands {
            self.lsp_commands.text()
        } else {
            self.lsp_formatters.text()
        };
        match parse_lsp_map(&text) {
            Ok(map) => {
                if commands {
                    self.current.lsp_commands = map;
                } else {
                    self.current.lsp_formatters = map;
                }
                self.save_current_section(Section::LanguageServers);
            }
            Err(error) => self.error = Some(error),
        }
    }

    fn update_localsend(&mut self, message: LocalSendMessage) {
        let settings_changed = match message {
            LocalSendMessage::AlwaysOnChanged(always_on) => {
                self.localsend.always_on = always_on;
                self.current.localsend.always_on = always_on;
                true
            }
            LocalSendMessage::AliasChanged(alias) => {
                self.localsend.alias = alias.clone();
                self.current.localsend.alias = alias;
                true
            }
            LocalSendMessage::ReceiveDirPicked(Some(path)) => {
                self.localsend.receive_dir.clone_from(&path);
                self.current.localsend.receive_dir = path.to_string_lossy().into_owned();
                true
            }
            LocalSendMessage::LimitsChanged {
                max_file_mb,
                max_files,
            } => {
                if !max_file_mb.is_empty() {
                    self.localsend.max_file_mb.clone_from(&max_file_mb);
                    match max_file_mb.parse::<u32>() {
                        Ok(value) if (1..=2048).contains(&value) => {
                            self.current.localsend.max_file_mb = value;
                        }
                        _ => {
                            self.localsend.error = Some(String::from(
                                crate::labels::SETTINGS_INVALID_LOCALSEND_LIMIT,
                            ));
                            return;
                        }
                    }
                }
                if !max_files.is_empty() {
                    self.localsend.max_files.clone_from(&max_files);
                    match max_files.parse::<usize>() {
                        Ok(value) if (1..=100).contains(&value) => {
                            self.current.localsend.max_files = value;
                        }
                        _ => {
                            self.localsend.error = Some(String::from(
                                crate::labels::SETTINGS_INVALID_LOCALSEND_LIMIT,
                            ));
                            return;
                        }
                    }
                }
                true
            }
            _ => false,
        };
        if settings_changed {
            self.localsend.error = None;
            self.saved = false;
            let result = match &self.path {
                Ok(path) => controls::save_localsend(path, &self.current.localsend),
                Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
            };
            match result {
                Ok(config) => {
                    self.current = config;
                    self.saved = true;
                }
                Err(error) => self.error = Some(error),
            }
        }
    }
}

fn format_lsp_map(map: &std::collections::BTreeMap<String, String>) -> String {
    map.iter()
        .map(|(language, command)| format!("{language} = {command}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn parse_lsp_map(input: &str) -> Result<std::collections::BTreeMap<String, String>, Error> {
    let mut result = std::collections::BTreeMap::new();
    for line in input.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((language, command)) = line.split_once('=') else {
            return Err(Error::InvalidLspSettings);
        };
        let language = language.trim();
        let command = command.trim();
        if language.is_empty()
            || language.len() > 32
            || !language
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
            || command.is_empty()
            || command.len() > 512
            || command.chars().any(char::is_control)
            || result
                .insert(language.to_owned(), command.to_owned())
                .is_some()
        {
            return Err(Error::InvalidLspSettings);
        }
    }
    Ok(result)
}

fn save_locale(path: &Path, value: &str) -> Result<(), Error> {
    let value = value.trim();
    if !value.is_empty()
        && (value.len() > 35
            || !value.split('-').all(|part| {
                !part.is_empty()
                    && part.len() <= 8
                    && part.bytes().all(|byte| byte.is_ascii_alphabetic())
            }))
    {
        return Err(Error::InvalidLocale);
    }
    let _ = gsh_config::load(path).map_err(Error::Config)?;
    let original = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(Error::Io(error)),
    };
    let mut document = original
        .parse::<toml_edit::DocumentMut>()
        .map_err(Error::Document)?;
    if value.is_empty() {
        document.remove("locale");
    } else {
        document
            .as_table_mut()
            .insert("locale", toml_edit::value(value));
    }
    let contents = document.to_string();
    let _ = gsh_config::load::parse(path, &contents).map_err(Error::Config)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    write_atomic(path, &contents)
}

pub(super) fn write_atomic(path: &Path, contents: &str) -> Result<(), Error> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        if let Ok(metadata) = std::fs::metadata(path) {
            file.set_permissions(metadata.permissions())?;
        }
        file.write_all(contents.as_bytes())?;
        std::fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map_err(Error::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_is_saved_and_invalid_input_does_not_write() {
        let dir = std::env::temp_dir().join(format!("gsh-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "futureKey = 'keep'\n[future]\nnested = 42\n").unwrap();
        save_locale(&path, "pt-BR").unwrap();
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("futureKey = 'keep'"));
        assert!(saved.contains("nested = 42"));
        assert_eq!(
            gsh_config::load(&path).unwrap().locale.as_deref(),
            Some("pt-BR")
        );
        let original = std::fs::read(&path).unwrap();
        assert!(matches!(
            save_locale(&path, "pt BR"),
            Err(Error::InvalidLocale)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), original);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn selected_theme_is_saved_applied_and_survives_reload() {
        use gsh_config::schema::AppThemeMode;
        let _guard = crate::theme::test_guard();
        let dir = std::env::temp_dir().join(format!("gsh-settings-reload-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "futureKey = 'keep'\n").unwrap();
        let mut settings = Settings {
            path: Ok(path.clone()),
            ..Settings::default()
        };
        let _ = settings.update(Message::ThemeSelected(AppThemeMode::Light));
        assert!(settings.saved && settings.error.is_none());
        assert_eq!(settings.theme_mode, AppThemeMode::Light);
        assert_eq!(crate::theme::active(), ThemeVariant::Light);
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("futureKey = 'keep'"));

        crate::theme::set_active(ThemeVariant::Dark);
        let mut reloaded = Settings {
            path: Ok(path.clone()),
            ..Settings::default()
        };
        reloaded.reload();
        assert_eq!(reloaded.theme_mode, AppThemeMode::Light);
        assert_eq!(crate::theme::active(), ThemeVariant::Light);

        let _ = reloaded.update(Message::ThemeSelected(AppThemeMode::System));
        assert!(reloaded.error.is_none());
        assert_eq!(reloaded.theme_mode, AppThemeMode::System);
        assert_eq!(
            gsh_config::load(&path).unwrap().theme_mode,
            AppThemeMode::System
        );

        crate::theme::set_active(ThemeVariant::Dark);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn changes_save_immediately_and_preserve_unknown_keys() {
        let dir = std::env::temp_dir().join(format!("gsh-settings-auto-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "futureKey = 'keep'\n").unwrap();
        let mut settings = Settings {
            path: Ok(path.clone()),
            ..Settings::default()
        };
        let _ = settings.update(Message::LocaleChanged("es".into()));
        assert_eq!(
            gsh_config::load(&path).unwrap().locale.as_deref(),
            Some("es")
        );
        let _ = settings.update(Message::Select(Section::Notifications));
        let _ = settings.update(Message::NotificationsChanged(false));
        assert!(!gsh_config::load(&path).unwrap().notifications_enabled);
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("futureKey = 'keep'"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn lsp_settings_round_trip_and_reject_malformed_entries() {
        let parsed = parse_lsp_map("rust = rust-analyzer\npython = pyright\n").unwrap();
        assert_eq!(
            parsed.get("rust").map(String::as_str),
            Some("rust-analyzer")
        );
        assert!(matches!(
            parse_lsp_map("bad entry"),
            Err(Error::InvalidLspSettings)
        ));
        assert!(matches!(
            parse_lsp_map("rust = a\nrust = b"),
            Err(Error::InvalidLspSettings)
        ));
        assert_eq!(parse_lsp_map(&format_lsp_map(&parsed)).unwrap(), parsed);
    }

    #[test]
    fn automation_and_toolbar_visibility_save_to_config() {
        let dir =
            std::env::temp_dir().join(format!("gsh-settings-features-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "[future]\nkeep = true\n[features]\nother = 3\n").unwrap();
        let mut settings = Settings {
            path: Ok(path.clone()),
            ..Settings::default()
        };
        let _ = settings.update(Message::AutomationHarnessChanged("Codex".into()));
        let _ = settings.update(Message::AutomationModelChanged("gpt-test".into()));
        let _ = settings.update(Message::FeatureVisibilityChanged(
            gsh_config::schema::AppFeature::Browser,
            false,
        ));
        let saved = gsh_config::load(&path).unwrap();
        assert_eq!(
            saved.automation.harness_id,
            Some(gsh_config::schema::HarnessKind::Codex)
        );
        assert_eq!(saved.automation.model(), Some("gpt-test"));
        assert!(!saved.shows_feature(gsh_config::schema::AppFeature::Browser));
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("keep = true"));
        assert!(contents.contains("other = 3"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
