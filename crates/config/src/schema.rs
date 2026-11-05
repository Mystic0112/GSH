//! Schema do `config.toml`: o subconjunto de configuração do `AppSettings` do
//! Flutter (`lib/app/core/domain/entities/app_settings.dart`), com os mesmos
//! padrões e a mesma normalização do `fromJson`.
//!
//! Chaves = as do JSON do Flutter; o ponto vira tabela TOML:
//!
//! ```toml
//! themeMode = "dark"
//! interfaceSize = 15
//!
//! [chat]
//! font = "Inter"
//!
//! [features]
//! hidden = ["farm", "browser"]
//! ```
//!
//! Todo campo é opcional (`#[serde(default)]` no container): arquivo parcial
//! preenche o resto com o padrão. Chave desconhecida é ignorada.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

use crate::named::{self, named_enum, Named};

named_enum! {
    /// Modo de tema (`AppThemeMode`).
    pub enum AppThemeMode {
        System => "system",
        Light => "light",
        Dark => "dark",
    }
}

named_enum! {
    /// Motor VT de terminais novos (`TerminalEngine`).
    pub enum TerminalEngine {
        Ghostty => "ghostty",
        Xterm => "xterm",
    }
}

named_enum! {
    /// Motor de arquivos de texto editáveis (`FileEditorEngine`).
    pub enum FileEditorEngine {
        Gsh => "gsh",
        Neovim => "neovim",
    }
}

named_enum! {
    /// Peso do traço da fonte do terminal (`TerminalFontWeight`). `Auto`
    /// resolve por densidade da tela, na UI.
    pub enum TerminalFontWeight {
        Auto => "auto",
        Light => "light",
        Normal => "normal",
        Medium => "medium",
        SemiBold => "semiBold",
    }
}

named_enum! {
    /// Layout inicial do Source Control (`SourceControlViewMode`).
    pub enum SourceControlViewMode {
        List => "list",
        Tree => "tree",
    }
}

named_enum! {
    /// Frequência de verificação de atualização (`UpdateCheckFrequency`).
    pub enum UpdateCheckFrequency {
        Daily => "daily",
        Weekly => "weekly",
        Monthly => "monthly",
        Never => "never",
    }
}

named_enum! {
    /// Como o chat abre a conversa (`TranscriptView`).
    pub enum TranscriptView {
        Normal => "normal",
        Thinking => "thinking",
        Verbose => "verbose",
    }
}

named_enum! {
    /// Largura máxima da coluna do chat (`TranscriptWidth`).
    pub enum TranscriptWidth {
        Narrow => "narrow",
        Medium => "medium",
        Wide => "wide",
    }
}

named_enum! {
    /// Recursos que o usuário pode tirar da barra (`AppFeature`).
    pub enum AppFeature {
        Chat => "chat",
        Farm => "farm",
        AgentGraph => "agentGraph",
        Browser => "browser",
        Harness => "harness",
        AgentIsland => "agentIsland",
    }
}

named_enum! {
    /// Cenário da fazenda de agentes (`FarmScenery`).
    pub enum FarmScenery {
        Farm => "farm",
        Office => "office",
    }
}

named_enum! {
    /// Eventos que tocam som (`SoundEvent`).
    pub enum SoundEvent {
        TurnDone => "turnDone",
        ActionRequired => "actionRequired",
    }
}

named_enum! {
    /// Harness das automações (`HarnessKind`).
    pub enum HarnessKind {
        Antigravity => "antigravity",
        ClaudeCode => "claudeCode",
        Codex => "codex",
        Cursor => "cursor",
        GitHubCopilot => "gitHubCopilot",
        OpenCode => "openCode",
        Pi => "pi",
    }
}

macro_rules! default_variant {
    ($($ty:ident => $variant:ident),+ $(,)?) => {
        $(impl Default for $ty {
            fn default() -> Self {
                Self::$variant
            }
        })+
    };
}

default_variant! {
    AppThemeMode => System,
    TerminalEngine => Ghostty,
    FileEditorEngine => Gsh,
    TerminalFontWeight => Auto,
    SourceControlViewMode => List,
    UpdateCheckFrequency => Daily,
    TranscriptView => Normal,
    TranscriptWidth => Medium,
    FarmScenery => Farm,
}

impl TranscriptWidth {
    /// Largura em px lógicos (o `px` do enum no Dart).
    pub fn px(self) -> f64 {
        match self {
            Self::Narrow => 680.0,
            Self::Medium => 760.0,
            Self::Wide => 900.0,
        }
    }
}

/// Id do tema oficial (espelha `theme_registry.dart`).
pub const DEFAULT_THEME_ID: &str = "gsh";

/// Configuração do GSH. Campos `Option` = "usar o padrão do design" (fonte),
/// "herdar" (tamanho do terminal) ou "seguir o SO" (locale), como no Flutter.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Config {
    #[serde(deserialize_with = "named::or_default")]
    pub theme_mode: AppThemeMode,
    /// Id do tema ativo (built-in ou importado). Vazio = [`DEFAULT_THEME_ID`].
    pub theme_id: String,
    /// `None` = Montserrat/Hanken.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interface_font: Option<String>,
    pub interface_size: f64,
    /// `None` = JetBrains Mono.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_font: Option<String>,
    pub code_size: f64,
    /// `None` = mono padrão do terminal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_font: Option<String>,
    /// `None` = herda [`Config::code_size`]. Ver [`Config::effective_terminal_size`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_size: Option<f64>,
    #[serde(deserialize_with = "named::or_default")]
    pub terminal_font_weight: TerminalFontWeight,
    pub pin_user_message: bool,
    pub format_on_save: bool,
    pub close_tab_on_shell_exit: bool,
    pub notifications_enabled: bool,
    pub swap_side_panels: bool,
    pub show_gsh: bool,
    pub developer_mode: bool,
    pub launch_at_startup: bool,
    /// Código do idioma (`en`, `pt-BR`). `None` = locale do SO.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(deserialize_with = "named::or_default")]
    pub update_check_frequency: UpdateCheckFrequency,
    /// Comando do language server por `languageId`. Valor em branco é descartado.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub lsp_commands: BTreeMap<String, String>,
    /// Formatador externo por `languageId` (placeholder `%FILE%`).
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub lsp_formatters: BTreeMap<String, String>,
    pub editor: Editor,
    pub terminal: Terminal,
    pub sound: Sound,
    pub source_control: SourceControl,
    pub automation: Automation,
    pub chat: Chat,
    pub code: Code,
    pub features: Features,
    pub links: Links,
    pub farm: Farm,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            theme_mode: AppThemeMode::default(),
            theme_id: DEFAULT_THEME_ID.to_owned(),
            interface_font: None,
            interface_size: 14.0,
            code_font: None,
            code_size: 13.0,
            terminal_font: None,
            terminal_size: None,
            terminal_font_weight: TerminalFontWeight::default(),
            pin_user_message: true,
            format_on_save: false,
            close_tab_on_shell_exit: false,
            notifications_enabled: true,
            swap_side_panels: false,
            show_gsh: true,
            developer_mode: false,
            launch_at_startup: false,
            locale: None,
            update_check_frequency: UpdateCheckFrequency::default(),
            lsp_commands: BTreeMap::new(),
            lsp_formatters: BTreeMap::new(),
            editor: Editor::default(),
            terminal: Terminal::default(),
            sound: Sound::default(),
            source_control: SourceControl::default(),
            automation: Automation::default(),
            chat: Chat::default(),
            code: Code::default(),
            features: Features::default(),
            links: Links::default(),
            farm: Farm::default(),
        }
    }
}

/// `[editor]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Editor {
    #[serde(deserialize_with = "named::or_default")]
    pub engine: FileEditorEngine,
}

/// `[terminal]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Terminal {
    /// Perfil padrão do `+`. `None` = fallback de plataforma. A chave é
    /// snake_case no Flutter (`terminal.default_profile_id`).
    #[serde(rename = "default_profile_id", skip_serializing_if = "Option::is_none")]
    pub default_profile_id: Option<String>,
    #[serde(deserialize_with = "named::or_default")]
    pub engine: TerminalEngine,
}

/// `[sound]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Sound {
    /// Volume em % (0–100, fora disso é limitado).
    pub volume: f64,
    /// Som por evento. Ausente = ligado.
    #[serde(
        deserialize_with = "named::known_map",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub events: BTreeMap<SoundEvent, bool>,
    /// Tocar mesmo com a aba do evento ativa. Ausente = não.
    #[serde(
        deserialize_with = "named::known_map",
        skip_serializing_if = "BTreeMap::is_empty"
    )]
    pub on_active_tab: BTreeMap<SoundEvent, bool>,
}

impl Default for Sound {
    fn default() -> Self {
        Self {
            volume: 50.0,
            events: BTreeMap::new(),
            on_active_tab: BTreeMap::new(),
        }
    }
}

/// `[sourceControl]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SourceControl {
    #[serde(deserialize_with = "named::or_default")]
    pub view_mode: SourceControlViewMode,
}

/// `[automation]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Automation {
    /// `None` = automações desligadas. Aceita os nomes antigos do Flutter.
    #[serde(deserialize_with = "harness", skip_serializing_if = "Option::is_none")]
    pub harness_id: Option<HarnessKind>,
    /// Vazio = o harness decide. Ver [`Automation::model`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

impl Automation {
    /// Modelo efetivo: `None` quando ausente ou vazio (`selectedAutomationModelId`).
    pub fn model(&self) -> Option<&str> {
        self.model_id.as_deref().filter(|m| !m.is_empty())
    }
}

/// `[chat]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Chat {
    pub classify_session_states: bool,
    pub remote_control: bool,
    #[serde(deserialize_with = "named::or_default")]
    pub transcript_view: TranscriptView,
    /// `None` = a fonte da interface.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(deserialize_with = "named::or_default")]
    pub transcript_width: TranscriptWidth,
}

impl Default for Chat {
    fn default() -> Self {
        Self {
            classify_session_states: true,
            remote_control: false,
            transcript_view: TranscriptView::default(),
            font: None,
            transcript_width: TranscriptWidth::default(),
        }
    }
}

/// `[code]`: paleta de syntax por modo. `None` = a do tema ativo.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Code {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_light: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_dark: Option<String>,
}

/// `[features]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Features {
    /// Recursos tirados da barra. Vazio = todos visíveis.
    #[serde(deserialize_with = "named::known_set")]
    pub hidden: BTreeSet<AppFeature>,
}

/// `[links]`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Links {
    pub open_in_app: bool,
}

/// `[farm]`.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Farm {
    #[serde(deserialize_with = "named::or_default")]
    pub scenery: FarmScenery,
    pub team: FarmTeam,
}

impl fmt::Debug for Farm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Farm")
            .field("scenery", &self.scenery)
            .field("team", &self.team)
            .finish()
    }
}

/// `[farm.team]`: fazenda do time na LAN. Strings vazias = não configurado.
#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FarmTeam {
    pub enabled: bool,
    /// Nome do time.
    pub name: String,
    /// Nome exibido para os colegas (vazio = usuário do SO).
    pub me: String,
    pub show_titles: bool,
    /// Faixa da rede (`192.168.10.0/24`).
    pub network: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub secret: String,
    pub door_ask: bool,
    /// Quem pode bater na porta (nomes, vírgula). Vazio = todo o time.
    pub door_allow: String,
}

impl fmt::Debug for FarmTeam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FarmTeam")
            .field("enabled", &self.enabled)
            .field("name", &self.name)
            .field("me", &self.me)
            .field("show_titles", &self.show_titles)
            .field("network", &self.network)
            .field("secret", &"<redacted>")
            .field("door_ask", &self.door_ask)
            .field("door_allow", &self.door_allow)
            .finish()
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("theme_mode", &self.theme_mode)
            .field("theme_id", &self.theme_id)
            .field("interface_font", &self.interface_font)
            .field("interface_size", &self.interface_size)
            .field("code_font", &self.code_font)
            .field("code_size", &self.code_size)
            .field("terminal_font", &self.terminal_font)
            .field("terminal_size", &self.terminal_size)
            .field("terminal_font_weight", &self.terminal_font_weight)
            .field("pin_user_message", &self.pin_user_message)
            .field("format_on_save", &self.format_on_save)
            .field("close_tab_on_shell_exit", &self.close_tab_on_shell_exit)
            .field("notifications_enabled", &self.notifications_enabled)
            .field("swap_side_panels", &self.swap_side_panels)
            .field("show_gsh", &self.show_gsh)
            .field("developer_mode", &self.developer_mode)
            .field("launch_at_startup", &self.launch_at_startup)
            .field("locale", &self.locale)
            .field("update_check_frequency", &self.update_check_frequency)
            .field("lsp_commands", &self.lsp_commands)
            .field("lsp_formatters", &self.lsp_formatters)
            .field("editor", &self.editor)
            .field("terminal", &self.terminal)
            .field("sound", &self.sound)
            .field("source_control", &self.source_control)
            .field("automation", &self.automation)
            .field("chat", &self.chat)
            .field("code", &self.code)
            .field("features", &self.features)
            .field("links", &self.links)
            .field("farm", &self.farm)
            .finish()
    }
}

impl Config {
    /// Recurso visível na barra (`showsFeature`).
    pub fn shows_feature(&self, feature: AppFeature) -> bool {
        !self.features.hidden.contains(&feature)
    }

    /// Som do evento ligado (`soundEnabledFor`): ausente = ligado.
    pub fn sound_enabled_for(&self, event: SoundEvent) -> bool {
        self.sound.events.get(&event).copied().unwrap_or(true)
    }

    /// Som do evento toca com a aba ativa: ausente = não.
    pub fn sound_on_active_tab_for(&self, event: SoundEvent) -> bool {
        self.sound
            .on_active_tab
            .get(&event)
            .copied()
            .unwrap_or(false)
    }

    /// Tamanho do terminal: o próprio ou, ausente, o do código.
    pub fn effective_terminal_size(&self) -> f64 {
        self.terminal_size.unwrap_or(self.code_size)
    }

    /// Aplica a normalização do `fromJson` do Flutter e rejeita número não
    /// finito. O erro é a chave (como no arquivo) e o valor.
    pub(crate) fn normalized(mut self) -> Result<Self, (&'static str, f64)> {
        for opt in [
            &mut self.interface_font,
            &mut self.code_font,
            &mut self.terminal_font,
            &mut self.locale,
            &mut self.terminal.default_profile_id,
            &mut self.code.theme_light,
            &mut self.code.theme_dark,
            &mut self.chat.font,
        ] {
            *opt = trimmed_non_empty(opt.take());
        }
        self.theme_id = match self.theme_id.trim() {
            "" => DEFAULT_THEME_ID.to_owned(),
            id => id.to_owned(),
        };
        self.automation.model_id = self.automation.model_id.map(|m| m.trim().to_owned());
        self.lsp_commands.retain(|_, v| !v.trim().is_empty());
        self.lsp_formatters.retain(|_, v| !v.trim().is_empty());

        finite("interfaceSize", self.interface_size)?;
        finite("codeSize", self.code_size)?;
        if let Some(size) = self.terminal_size {
            finite("terminalSize", size)?;
        }
        finite("sound.volume", self.sound.volume)?;
        self.sound.volume = self.sound.volume.clamp(0.0, 100.0);
        Ok(self)
    }
}

fn trimmed_non_empty(s: Option<String>) -> Option<String> {
    s.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

fn finite(key: &'static str, value: f64) -> Result<(), (&'static str, f64)> {
    if value.is_finite() {
        Ok(())
    } else {
        Err((key, value))
    }
}

/// Nomes que versões antigas do Flutter gravavam (`_legacyHarnessNames`).
/// `gemini` não tem equivalente e cai em "não configurado".
const LEGACY_HARNESS_NAMES: &[(&str, HarnessKind)] = &[
    ("claude", HarnessKind::ClaudeCode),
    ("copilot", HarnessKind::GitHubCopilot),
    ("opencode", HarnessKind::OpenCode),
    ("codex", HarnessKind::Codex),
    ("pi", HarnessKind::Pi),
];

fn harness<'de, D>(de: D) -> Result<Option<HarnessKind>, D::Error>
where
    D: Deserializer<'de>,
{
    let raw = String::deserialize(de)?;
    Ok(HarnessKind::from_name(&raw).or_else(|| {
        LEGACY_HARNESS_NAMES
            .iter()
            .find(|(name, _)| *name == raw)
            .map(|(_, kind)| *kind)
    }))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::load::parse;

    fn from(text: &str) -> Config {
        parse(Path::new("config.toml"), text).unwrap()
    }

    /// Padrões do construtor de `AppSettings` no Dart, campo a campo.
    #[test]
    fn defaults_match_flutter_app_settings() {
        let c = Config::default();
        assert_eq!(c.theme_mode, AppThemeMode::System);
        assert_eq!(c.theme_id, "gsh");
        assert_eq!(c.interface_font, None);
        assert_eq!(c.interface_size, 14.0);
        assert_eq!(c.code_font, None);
        assert_eq!(c.code_size, 13.0);
        assert_eq!(c.terminal_font, None);
        assert_eq!(c.terminal_size, None);
        assert_eq!(c.terminal_font_weight, TerminalFontWeight::Auto);
        assert!(c.pin_user_message);
        assert!(c.lsp_commands.is_empty() && c.lsp_formatters.is_empty());
        assert!(!c.format_on_save);
        assert!(!c.close_tab_on_shell_exit);
        assert_eq!(c.editor.engine, FileEditorEngine::Gsh);
        assert!(c.notifications_enabled);
        assert!(c.sound.events.is_empty() && c.sound.on_active_tab.is_empty());
        assert_eq!(c.sound.volume, 50.0);
        assert!(!c.swap_side_panels);
        assert!(c.show_gsh);
        assert!(!c.developer_mode);
        assert!(!c.launch_at_startup);
        assert_eq!(c.terminal.default_profile_id, None);
        assert_eq!(c.terminal.engine, TerminalEngine::Ghostty);
        assert_eq!(c.locale, None);
        assert_eq!(c.source_control.view_mode, SourceControlViewMode::List);
        assert_eq!(c.automation.harness_id, None);
        assert_eq!(c.automation.model_id, None);
        assert_eq!(c.update_check_frequency, UpdateCheckFrequency::Daily);
        assert!(c.chat.classify_session_states);
        assert!(!c.chat.remote_control);
        assert_eq!(c.code.theme_light, None);
        assert_eq!(c.code.theme_dark, None);
        assert_eq!(c.chat.transcript_view, TranscriptView::Normal);
        assert_eq!(c.chat.font, None);
        assert_eq!(c.chat.transcript_width, TranscriptWidth::Medium);
        assert_eq!(c.chat.transcript_width.px(), 760.0);
        assert!(c.features.hidden.is_empty());
        assert!(!c.links.open_in_app);
        assert_eq!(c.farm.scenery, FarmScenery::Farm);
        assert_eq!(c.farm.team, FarmTeam::default());
        assert!(!c.farm.team.enabled && c.farm.team.secret.is_empty());
    }

    #[test]
    fn empty_file_is_all_defaults() {
        assert_eq!(from(""), Config::default());
        assert_eq!(from("# só comentário\n"), Config::default());
    }

    /// Chaves do JSON do Flutter, com o ponto virando tabela.
    #[test]
    fn keys_mirror_flutter_json_keys() {
        let c = from(
            r#"
            themeMode = "light"
            themeId = "nord"
            interfaceFont = "Inter"
            terminalSize = 12.5
            terminalFontWeight = "semiBold"
            lspCommands = { dart = "dart language-server" }
            locale = "pt-BR"
            updateCheckFrequency = "never"
            editor.engine = "neovim"
            terminal.default_profile_id = "zsh"
            terminal.engine = "xterm"
            sourceControl.viewMode = "tree"
            automation.harnessId = "codex"
            automation.modelId = " gpt-5 "
            chat.classifySessionStates = false
            chat.transcriptView = "verbose"
            chat.transcriptWidth = "wide"
            code.themeDark = "dracula"
            links.openInApp = true
            farm.scenery = "office"

            [sound]
            events = { turnDone = false }
            onActiveTab = { actionRequired = true }

            [farm.team]
            enabled = true
            name = "core"
            me = "helio"
            showTitles = true
            network = "192.168.10.0/24"
            doorAsk = true
            doorAllow = "ana, bia"
            "#,
        );
        assert_eq!(c.theme_mode, AppThemeMode::Light);
        assert_eq!(c.theme_id, "nord");
        assert_eq!(c.interface_font.as_deref(), Some("Inter"));
        assert_eq!(c.effective_terminal_size(), 12.5);
        assert_eq!(c.terminal_font_weight, TerminalFontWeight::SemiBold);
        assert_eq!(
            c.lsp_commands.get("dart").map(String::as_str),
            Some("dart language-server")
        );
        assert_eq!(c.locale.as_deref(), Some("pt-BR"));
        assert_eq!(c.update_check_frequency, UpdateCheckFrequency::Never);
        assert_eq!(c.editor.engine, FileEditorEngine::Neovim);
        assert_eq!(c.terminal.default_profile_id.as_deref(), Some("zsh"));
        assert_eq!(c.terminal.engine, TerminalEngine::Xterm);
        assert_eq!(c.source_control.view_mode, SourceControlViewMode::Tree);
        assert_eq!(c.automation.harness_id, Some(HarnessKind::Codex));
        assert_eq!(c.automation.model(), Some("gpt-5"));
        assert!(!c.chat.classify_session_states);
        assert_eq!(c.chat.transcript_view, TranscriptView::Verbose);
        assert_eq!(c.chat.transcript_width, TranscriptWidth::Wide);
        assert_eq!(c.code.theme_dark.as_deref(), Some("dracula"));
        assert!(c.links.open_in_app);
        assert_eq!(c.farm.scenery, FarmScenery::Office);
        assert!(!c.sound_enabled_for(SoundEvent::TurnDone));
        assert!(c.sound_enabled_for(SoundEvent::ActionRequired));
        assert!(c.sound_on_active_tab_for(SoundEvent::ActionRequired));
        assert!(!c.sound_on_active_tab_for(SoundEvent::TurnDone));
        assert!(c.farm.team.enabled && c.farm.team.show_titles && c.farm.team.door_ask);
        assert_eq!(c.farm.team.name, "core");
        assert_eq!(c.farm.team.me, "helio");
        assert_eq!(c.farm.team.network, "192.168.10.0/24");
        assert_eq!(c.farm.team.door_allow, "ana, bia");
    }

    #[test]
    fn integer_sizes_are_accepted() {
        let c = from("interfaceSize = 16\ncodeSize = 12\nterminalSize = 11\n");
        assert_eq!(c.interface_size, 16.0);
        assert_eq!(c.code_size, 12.0);
        assert_eq!(c.terminal_size, Some(11.0));
    }

    /// `str()` do `fromJson`: trim e vazio = ausente; `themeId` vazio = "gsh".
    #[test]
    fn strings_are_normalized_like_flutter() {
        let c = from(
            "interfaceFont = '  '\ncodeFont = ' Fira Code '\nthemeId = ' '\nlocale = ''\nchat.font = ' '\n",
        );
        assert_eq!(c.interface_font, None);
        assert_eq!(c.code_font.as_deref(), Some("Fira Code"));
        assert_eq!(c.theme_id, "gsh");
        assert_eq!(c.locale, None);
        assert_eq!(c.chat.font, None);
    }

    #[test]
    fn blank_lsp_entries_are_dropped() {
        let c = from("lspCommands = { dart = '  ', rust = 'rust-analyzer' }\n");
        assert_eq!(c.lsp_commands.len(), 1);
        assert!(c.lsp_commands.contains_key("rust"));
    }

    #[test]
    fn volume_is_clamped() {
        assert_eq!(from("sound.volume = 150\n").sound.volume, 100.0);
        assert_eq!(from("sound.volume = -3\n").sound.volume, 0.0);
    }

    /// `_enumByName`: nome desconhecido cai no padrão; listas e mapas ignoram.
    #[test]
    fn unknown_names_fall_back_like_flutter() {
        let c = from(
            "themeMode = 'neon'\nfeatures.hidden = ['chat', 'nope']\nsound.events = { ghost = false }\nautomation.harnessId = 'gemini'\n",
        );
        assert_eq!(c.theme_mode, AppThemeMode::System);
        assert!(c.features.hidden.contains(&AppFeature::Chat));
        assert_eq!(c.features.hidden.len(), 1);
        assert!(!c.shows_feature(AppFeature::Chat));
        assert!(c.shows_feature(AppFeature::Farm));
        assert!(c.sound.events.is_empty());
        assert_eq!(c.automation.harness_id, None);
    }

    #[test]
    fn legacy_harness_names_are_accepted() {
        for (raw, kind) in LEGACY_HARNESS_NAMES {
            let c = from(&format!("automation.harnessId = '{raw}'\n"));
            assert_eq!(c.automation.harness_id, Some(*kind));
        }
        let c = from("automation.harnessId = 'gitHubCopilot'\n");
        assert_eq!(c.automation.harness_id, Some(HarnessKind::GitHubCopilot));
    }

    #[test]
    fn empty_model_means_harness_decides() {
        let c = from("automation.harnessId = 'claudeCode'\nautomation.modelId = ''\n");
        assert_eq!(c.automation.model_id.as_deref(), Some(""));
        assert_eq!(c.automation.model(), None);
    }

    #[test]
    fn unknown_and_legacy_keys_are_ignored() {
        let c = from(
            "soundEnabled = false\nsyntaxTheme = 'one'\nfuture.key = 1\nlastOpenAppId = 'vscode'\n",
        );
        assert_eq!(c, Config::default());
    }

    #[test]
    fn wrong_type_is_an_error() {
        let path = Path::new("x.toml");
        assert!(parse(path, "themeMode = 3\n").is_err());
        assert!(parse(path, "features.hidden = 'chat'\n").is_err());
        assert!(parse(path, "showGsh = 'yes'\n").is_err());
    }

    #[test]
    fn enum_names_round_trip() {
        for f in AppFeature::ALL {
            assert_eq!(AppFeature::from_name(f.name()), Some(*f));
        }
        assert_eq!(TerminalFontWeight::SemiBold.name(), "semiBold");
        assert_eq!(AppFeature::AgentIsland.name(), "agentIsland");
    }
}
