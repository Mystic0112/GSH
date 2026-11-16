//! Estado e persistência das configurações gerais.

mod appearance;
pub mod claude;
mod controls;
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
    Claude(claude::Message),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Appearance,
    Terminal,
    Editor,
    Notifications,
    Team,
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
        }
    }
}

impl Settings {
    pub fn reload(&mut self) {
        self.saved = false;
        self.claude.reset();
        match &self.path {
            Ok(path) => match gsh_config::load(path) {
                Ok(config) => {
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
                    theme::set_active(ThemeVariant::from_mode(config.theme_mode));
                    self.current = config.clone();
                    self.interface_size = config.interface_size.to_string();
                    self.scale = (config.interface_size.clamp(10.0, 20.0) / 14.0) as f32;
                    self.error = None;
                }
                Err(error) => self.error = Some(Error::Config(error)),
            },
            Err(_) => self.error = Some(Error::Config(gsh_config::ConfigError::NoHome)),
        }
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
            theme::set_active(ThemeVariant::from_mode(*saved));
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
}
