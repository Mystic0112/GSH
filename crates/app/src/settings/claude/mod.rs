//! Lado "Claude" das Configurações: todas as abas de Configurações, Este computador,
//! Personalização e Plataforma (`settings_nav.dart`).
//!
//! Porta de `claude_code_panel.dart`, `claude_code_setting_switch.dart`,
//! `claude_info_viewmodel.dart`, `claude_cli_info_source.dart`, `account_panel.dart`,
//! `privacy_panel.dart`, `general_panel.dart`, `usage_panel.dart`, `resources_panel.dart`,
//! `memory_panel.dart`, `design_systems_panel.dart`, `placeholder_panel.dart`,
//! `system_panel.dart`, `developer_panel.dart`, `skills_panel.dart`,
//! `connectors_panel.dart`, `plugins_panel.dart`, `plugin_directory.dart` e
//! `font_picker_dialog.dart`.

mod catalog;
pub mod cli;
mod code_theme;
mod file;
mod files;
pub mod fonts;
mod palettes;
mod panels;
mod prefs;
pub(crate) mod profiles;
mod secondary;
mod switches;
pub mod themes;
pub(super) mod update;
pub mod view;
mod widgets;

use crate::bag_mode::{poll_and_heal_bag_mode_blocking, BagModeState, BagPoll};
use crate::labels;
use cli::{Account, InfoError, MarketplacePlugin, McpServerInfo, PluginInfo, Usage};
use files::{AutoMemoryEntry, MemoryFileInfo, SkillInfo};
use fonts::{FontPickerState, FontTarget};
use gsh_harness::Json;
use iced::widget::text_editor;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{path::PathBuf, time::Instant};

pub use prefs::Pref;
pub use switches::Switch;

/// Abas do lado Claude, na ordem do menu lateral (`settings_nav.dart`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    // Grupo: Configurações
    General,
    Account,
    Privacy,
    Usage,
    Resources,
    Memory,
    DesignSystems,
    ClaudeCode,
    Cowork,
    // Grupo: Este computador
    System,
    Extensions,
    Developer,
    // Grupo: Personalização
    Skills,
    Connectors,
    Plugins,
}

impl Tab {
    pub const ALL: [Self; 15] = [
        Self::General,
        Self::Account,
        Self::Privacy,
        Self::Usage,
        Self::Resources,
        Self::Memory,
        Self::DesignSystems,
        Self::ClaudeCode,
        Self::Cowork,
        Self::System,
        Self::Extensions,
        Self::Developer,
        Self::Skills,
        Self::Connectors,
        Self::Plugins,
    ];

    pub const GROUP_SETTINGS: [Self; 9] = [
        Self::General,
        Self::Account,
        Self::Privacy,
        Self::Usage,
        Self::Resources,
        Self::Memory,
        Self::DesignSystems,
        Self::ClaudeCode,
        Self::Cowork,
    ];

    pub const GROUP_COMPUTER: [Self; 3] = [Self::System, Self::Extensions, Self::Developer];

    pub const GROUP_PERSONALIZATION: [Self; 3] = [Self::Skills, Self::Connectors, Self::Plugins];

    pub fn label(self) -> &'static str {
        match self {
            Self::General => labels::SETTINGS_GENERAL,
            Self::Account => labels::SETTINGS_NAV_ACCOUNT,
            Self::Privacy => labels::SETTINGS_NAV_PRIVACY,
            Self::Usage => labels::SETTINGS_NAV_USAGE,
            Self::Resources => labels::SETTINGS_NAV_RESOURCES,
            Self::Memory => labels::SETTINGS_NAV_MEMORY,
            Self::DesignSystems => labels::SETTINGS_NAV_DESIGN_SYSTEMS,
            Self::ClaudeCode => labels::SETTINGS_NAV_CLAUDE_CODE,
            Self::Cowork => labels::SETTINGS_NAV_COWORK,
            Self::System => labels::SETTINGS_NAV_SYSTEM,
            Self::Extensions => labels::SETTINGS_NAV_EXTENSIONS,
            Self::Developer => labels::SETTINGS_NAV_DEVELOPER,
            Self::Skills => labels::SETTINGS_NAV_SKILLS,
            Self::Connectors => labels::SETTINGS_NAV_CONNECTORS,
            Self::Plugins => labels::SETTINGS_NAV_PLUGINS,
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Self::General => labels::ICON_SETTINGS,
            Self::Account => labels::ICON_ACCOUNT_CIRCLE,
            Self::Privacy => labels::ICON_SHIELD,
            Self::Usage => labels::ICON_DATA_USAGE,
            Self::Resources => labels::ICON_WORK_OUTLINE,
            Self::Memory => labels::ICON_PSYCHOLOGY,
            Self::DesignSystems => labels::ICON_APPEARANCE,
            Self::ClaudeCode => labels::ICON_CODE,
            Self::Cowork => labels::ICON_CHECKLIST,
            Self::System => labels::ICON_DESKTOP_WINDOWS,
            Self::Extensions => labels::ICON_EXTENSION,
            Self::Developer => labels::ICON_BUILD,
            Self::Skills => labels::ICON_HISTORY_EDU,
            Self::Connectors => labels::ICON_WIDGETS,
            Self::Plugins => labels::ICON_POWER,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamePromptKind {
    Skill,
    Marketplace,
}

#[derive(Debug, Clone)]
pub enum Message {
    Select(Tab),
    /// Liga/desliga uma chave booleana do `~/.claude/settings.json`.
    Switch(Switch, bool),
    /// Grava uma preferência do GSH no `config.toml`.
    Pref(Pref),
    Theme(gsh_config::schema::AppThemeMode),
    CodeFontInput(String),
    CodeFontCommit,
    HostsInput(String),
    HostsCommit,
    /// Estilo de saída; `None` = Padrão (apaga a chave).
    OutputStyle(Option<String>),
    CrossSession(String),
    Instructions(text_editor::Action),
    InstructionsCommit,
    ProfileName(String),
    ProfileDirectory(String),
    AddProfile,
    SelectProfile(Option<String>),
    LoginProfile(String),
    LoginAccount,
    RemoveProfile(String),
    RefreshAccount,
    AccountLoaded(u64, Result<Account, InfoError>),
    LogoutAsk,
    LogoutCancel,
    LogoutConfirm,
    LoggedOut(u64, bool, Result<(), InfoError>),
    SwitchAccountAsk,
    SwitchAccountCancel,
    SwitchAccountConfirm,
    /// Uso (`get_usage`).
    RefreshUsage,
    UsageLoaded(u64, Result<Usage, InfoError>),
    /// MCP (`claude mcp list`).
    RefreshMcp,
    McpLoaded(Result<Vec<McpServerInfo>, InfoError>),
    SelectDevMcp(String),
    /// Skills e Plugins.
    RefreshSkills,
    SkillsLoaded(Result<Vec<SkillInfo>, InfoError>),
    RefreshPlugins,
    PluginsLoaded(Result<Vec<PluginInfo>, InfoError>),
    DirectoryLoaded(Result<Vec<MarketplacePlugin>, InfoError>),
    CatalogDiscover(bool),
    CatalogSearch(String),
    CatalogAddMenu(bool),
    SelectMarketplace(Option<String>),
    NamePromptOpen(NamePromptKind),
    NamePromptInput(String),
    NamePromptSubmit,
    NamePromptCancel,
    /// Formulário de Conectores (`claude mcp add`).
    McpFormName(String),
    McpFormTarget(String),
    McpFormHttp(bool),
    McpFormLocalScope(bool),
    /// Design systems.
    DsSearch(String),
    ImportThemePick,
    ThemeFilePicked(Option<PathBuf>),
    DeleteTheme(String),
    /// Sistema e Armazenamento.
    KeepAwakeToggle(bool),
    LidSessionDuration(themes::LidSessionDuration),
    LidSessionStart,
    LidSessionStop,
    BagModeToggle(bool),
    BagHotspotInput(String),
    BagPreferredInput(String),
    BagApplyHotspot,
    BagHotspotApplied(bool, Result<(), String>),
    BagPollFinished(Result<BagPoll, String>),
    StorageUseDefault,
    StorageChangePick,
    StorageFolderPicked(Option<PathBuf>),
    StorageResetAsk,
    StorageResetCancel,
    StorageResetConfirm,
    StorageRestartDismiss,
    StorageRestartQuit,
    /// Seletor de fontes (`FontPickerDialog`).
    FontPickerOpen(FontTarget),
    FontPickerSearch(String),
    FontPickerCustomInput(String),
    FontPickerUseCustom,
    FontPick(Option<String>),
    FontPickerClose,
    /// Ações que delegam ao shell: terminal, abrir arquivo no editor ou URL no navegador.
    RunInTerminal(String),
    OpenFile(PathBuf),
    OpenUrl(String),
    NoticeClose,
    ToastClose(u64),
}

/// Texto de `~/.claude/CLAUDE.md` no editor da aba Conta.
pub(super) struct Instructions {
    saved: String,
    content: text_editor::Content,
}

pub struct ClaudeState {
    pub tab: Tab,
    /// `$HOME`; vazio = sem home (nada do usuário é lido).
    pub home: PathBuf,
    /// Diretório do workspace ativo no shell, para abas que dependem dele (Memória, Skills, Plugins).
    pub workspace: Option<PathBuf>,
    snapshot: file::Snapshot,
    output_styles: Vec<String>,
    hosts: String,
    hosts_draft: String,
    code_font_draft: Option<String>,
    account: Option<Result<Account, InfoError>>,
    account_generation: u64,
    account_loading: bool,
    logout_error: Option<InfoError>,
    logout_ask: bool,
    switch_account_ask: bool,
    profile_name: String,
    profile_directory: String,
    profile_error: Option<String>,
    instructions: Option<Instructions>,
    instructions_error: Option<InfoError>,
    usage: Option<Result<Usage, InfoError>>,
    usage_loading: bool,
    usage_at: Option<i64>,
    mcp: Option<Result<Vec<McpServerInfo>, InfoError>>,
    mcp_loading: bool,
    dev_selected_mcp: Option<String>,
    plugins: Option<Result<Vec<PluginInfo>, InfoError>>,
    plugins_loading: bool,
    directory: Option<Result<Vec<MarketplacePlugin>, InfoError>>,
    directory_loading: bool,
    skills: Option<Result<Vec<SkillInfo>, InfoError>>,
    skills_loading: bool,
    memory: Vec<MemoryFileInfo>,
    auto_memory: Vec<AutoMemoryEntry>,
    project_plugins: Vec<String>,
    catalog_discover: bool,
    catalog_search: String,
    catalog_add_menu: bool,
    selected_marketplace: Option<String>,
    name_prompt: Option<(NamePromptKind, String)>,
    mcp_form_name: String,
    mcp_form_target: String,
    mcp_form_http: bool,
    mcp_form_local_scope: bool,
    themes: Vec<themes::ThemeEntry>,
    ds_search: String,
    keep_awake: themes::KeepAwake,
    keep_awake_active: bool,
    bag_mode: BagModeState,
    storage_reset_ask: bool,
    storage_restart_msg: Option<(String, bool)>,
    font_picker: Option<FontPickerState>,
    notice: bool,
    toast: Option<(u64, String)>,
    next_toast: u64,
    pending_terminal: Option<String>,
    pending_auth_terminal: Option<Option<PathBuf>>,
    pending_open_file: Option<PathBuf>,
    pending_quit: bool,
}

impl Default for ClaudeState {
    fn default() -> Self {
        let home = home_dir();
        let themes = themes::load_themes(&home);
        Self {
            tab: Tab::General,
            home,
            workspace: None,
            snapshot: Ok(Json::Obj(Vec::new())),
            output_styles: vec![String::from("Explanatory"), String::from("Learning")],
            hosts: String::new(),
            hosts_draft: String::new(),
            code_font_draft: None,
            account: None,
            account_generation: 0,
            account_loading: false,
            logout_error: None,
            logout_ask: false,
            switch_account_ask: false,
            profile_name: String::new(),
            profile_directory: String::new(),
            profile_error: None,
            instructions: None,
            instructions_error: None,
            usage: None,
            usage_loading: false,
            usage_at: None,
            mcp: None,
            mcp_loading: false,
            dev_selected_mcp: None,
            plugins: None,
            plugins_loading: false,
            directory: None,
            directory_loading: false,
            skills: None,
            skills_loading: false,
            memory: Vec::new(),
            auto_memory: Vec::new(),
            project_plugins: Vec::new(),
            catalog_discover: false,
            catalog_search: String::new(),
            catalog_add_menu: false,
            selected_marketplace: None,
            name_prompt: None,
            mcp_form_name: String::new(),
            mcp_form_target: String::new(),
            mcp_form_http: true,
            mcp_form_local_scope: false,
            themes,
            ds_search: String::new(),
            keep_awake: themes::KeepAwake::default(),
            keep_awake_active: false,
            bag_mode: BagModeState::default(),
            storage_reset_ask: false,
            storage_restart_msg: None,
            font_picker: None,
            notice: false,
            toast: None,
            next_toast: 0,
            pending_terminal: None,
            pending_auth_terminal: None,
            pending_open_file: None,
            pending_quit: false,
        }
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

impl ClaudeState {
    /// Janela de Configurações aberta de novo: a página do Flutter nasce do zero
    /// (preservando `home`, `workspace` e a sessão de tampa/Modo bolsa).
    pub fn reset(&mut self) {
        let home = self.home.clone();
        let workspace = self.workspace.clone();
        let keep_awake = std::mem::take(&mut self.keep_awake);
        let bag_mode = std::mem::take(&mut self.bag_mode);
        *self = Self {
            home,
            workspace,
            keep_awake,
            bag_mode,
            account_generation: self.account_generation.wrapping_add(1),
            ..Self::default()
        };
        self.poll_keep_awake();
        self.reload_files();
    }

    pub(super) fn invalidate_account_data(&mut self) {
        self.account_generation = self.account_generation.wrapping_add(1);
        self.account = None;
        self.account_loading = false;
        self.usage = None;
        self.usage_loading = false;
        self.usage_at = None;
        self.logout_error = None;
        self.logout_ask = false;
        self.switch_account_ask = false;
    }

    pub(super) fn settings_path(&self) -> PathBuf {
        file::settings_path(&self.home)
    }

    pub(super) fn refresh_snapshot(&mut self) {
        self.snapshot = file::read(&self.settings_path());
    }

    pub(super) fn reload_files(&mut self) {
        self.refresh_snapshot();
        self.output_styles = files::output_styles(&self.home);
        let hosts = files::read_hosts(&self.home).join(", ");
        self.hosts = hosts.clone();
        self.hosts_draft = hosts;
        self.themes = themes::load_themes(&self.home);
        self.memory = files::memory_files(&self.home, self.workspace.as_deref());
        self.auto_memory = files::auto_memory(&self.home, self.workspace.as_deref());
        self.project_plugins = files::project_plugins(self.workspace.as_deref());
    }

    pub fn toast_text(&self) -> Option<&str> {
        self.toast.as_ref().map(|(_, text)| text.as_str())
    }

    pub fn show_no_workspace(&mut self) {
        self.notice = true;
    }

    pub fn take_terminal_command(&mut self) -> Option<String> {
        self.pending_terminal.take()
    }

    pub fn take_auth_terminal_request(&mut self) -> Option<Option<PathBuf>> {
        self.pending_auth_terminal.take()
    }

    pub fn report_auth_terminal_error(&mut self, error: InfoError) {
        self.profile_error = Some(labels::claude_info_error(&error));
        self.logout_error = Some(error);
    }

    pub fn take_open_file(&mut self) -> Option<PathBuf> {
        self.pending_open_file.take()
    }

    pub fn take_quit_request(&mut self) -> bool {
        std::mem::replace(&mut self.pending_quit, false)
    }

    pub fn keep_awake_active(&self) -> bool {
        self.keep_awake_active
    }

    pub fn keep_awake_on_battery(&self) -> bool {
        self.keep_awake_active && self.keep_awake.on_battery()
    }

    pub fn poll_keep_awake(&mut self) {
        self.keep_awake_active = self.keep_awake.is_active();
        if self.bag_mode.config.enabled && self.keep_awake.lid_remaining_minutes().is_none() {
            self.stop_bag_mode();
            self.bag_mode.status = Some(Ok("Sessão de tampa fechada encerrada.".to_owned()));
        }
    }

    fn stop_bag_mode(&mut self) {
        self.bag_mode.config.enabled = false;
        self.bag_mode.status = None;
        self.bag_mode.poll_at = None;
        self.bag_mode.preferred_retry_at = None;
        self.bag_mode.cancel_work();
    }

    pub fn poll_bag_mode(&mut self) -> Option<iced::Task<Message>> {
        let now = Instant::now();
        if !self.bag_mode.config.enabled
            || self.bag_mode.work.is_some()
            || self.bag_mode.poll_at.is_some_and(|due| due > now)
        {
            return None;
        }

        let cancelled = self.start_bag_work();
        let config = self.bag_mode.config.clone();
        let defer_preferred = self
            .bag_mode
            .preferred_retry_at
            .is_some_and(|due| due > now);
        Some(iced::Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    poll_and_heal_bag_mode_blocking(&config, defer_preferred, &cancelled)
                })
                .await
                .map_err(|err| format!("Falha na vigia do Modo bolsa: {err}"))?
            },
            Message::BagPollFinished,
        ))
    }

    fn start_bag_work(&mut self) -> Arc<AtomicBool> {
        let cancelled = Arc::new(AtomicBool::new(false));
        self.bag_mode.work = Some(cancelled.clone());
        cancelled
    }

    fn finish_bag_work(&mut self) -> bool {
        let work = self.bag_mode.work.take();
        work.is_some_and(|work| !work.load(Ordering::Relaxed))
    }
}
