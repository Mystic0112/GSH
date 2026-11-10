//! Estado e persistência das configurações gerais.

mod appearance;
pub mod view;

use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    InvalidLocale,
    InvalidInterfaceSize,
    Config(gsh_config::ConfigError),
    Io(std::io::Error),
    Document(toml_edit::TomlError),
}

#[derive(Debug, Clone)]
pub enum Message {
    LocaleChanged(String),
    Save,
    ShowAppearance(bool),
    InterfaceSizeChanged(String),
    SaveAppearance,
}

pub struct Settings {
    path: Result<PathBuf, gsh_config::ConfigError>,
    pub locale: String,
    pub appearance: bool,
    pub interface_size: String,
    pub scale: f32,
    pub error: Option<Error>,
    pub saved: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            path: gsh_config::default_path(),
            locale: String::new(),
            appearance: false,
            interface_size: String::from("14"),
            scale: 1.0,
            error: None,
            saved: false,
        }
    }
}

impl Settings {
    pub fn reload(&mut self) {
        self.saved = false;
        match &self.path {
            Ok(path) => match gsh_config::load(path) {
                Ok(config) => {
                    self.locale = config.locale.unwrap_or_default();
                    self.interface_size = config.interface_size.to_string();
                    self.scale = (config.interface_size.clamp(10.0, 20.0) / 14.0) as f32;
                    self.error = None;
                }
                Err(error) => self.error = Some(Error::Config(error)),
            },
            Err(_) => self.error = Some(Error::Config(gsh_config::ConfigError::NoHome)),
        }
    }

    pub fn update(&mut self, message: Message) {
        match message {
            Message::ShowAppearance(value) => {
                self.appearance = value;
                self.error = None;
                self.saved = false;
            }
            Message::InterfaceSizeChanged(value) => {
                self.interface_size = value;
                self.error = None;
                self.saved = false;
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
            }
            Message::Save => {
                let result = match &self.path {
                    Ok(path) => save_locale(path, &self.locale),
                    Err(_) => Err(Error::Config(gsh_config::ConfigError::NoHome)),
                };
                self.saved = result.is_ok();
                self.error = result.err();
            }
        }
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

fn write_atomic(path: &Path, contents: &str) -> Result<(), Error> {
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
}
