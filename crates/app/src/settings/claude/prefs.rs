//! Preferências do GSH que a aba Claude grava no `config.toml` (as do `SettingsController`
//! do Flutter): uma chave por vez, preservando o resto do arquivo.

use crate::settings::{write_atomic, Error};
use gsh_config::schema::{TranscriptView, TranscriptWidth};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub enum Pref {
    ClassifySessionStates(bool),
    ChatRemoteControl(bool),
    OpenLinksInApp(bool),
    TranscriptView(TranscriptView),
    TranscriptWidth(TranscriptWidth),
    /// `None` volta ao padrão (apaga a chave).
    CodeFont(Option<String>),
    InterfaceFont(Option<String>),
    ChatFont(Option<String>),
    CodeTheme {
        dark: bool,
        name: Option<String>,
    },
    ThemeId(String),
    InterfaceSize(f64),
    Notifications(bool),
    PermissionSound(bool),
    LaunchAtStartup(bool),
    DeveloperMode(bool),
}

/// Grava `pref` no `config.toml` e devolve a configuração relida do texto novo. Se o arquivo
/// atual ou o resultado não forem uma configuração válida, nada é gravado.
pub fn save(path: &Path, pref: &Pref) -> Result<gsh_config::Config, Error> {
    let _ = gsh_config::load(path).map_err(Error::Config)?;
    let original = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(Error::Io(e)),
    };
    let mut doc = original
        .parse::<toml_edit::DocumentMut>()
        .map_err(Error::Document)?;
    apply(&mut doc, pref)?;
    let contents = doc.to_string();
    let config = gsh_config::load::parse(path, &contents).map_err(Error::Config)?;
    write_atomic(path, &contents)?;
    Ok(config)
}

fn apply(doc: &mut toml_edit::DocumentMut, pref: &Pref) -> Result<(), Error> {
    match pref {
        Pref::ClassifySessionStates(on) => set(
            doc,
            Some("chat"),
            "classifySessionStates",
            Some(toml_edit::value(*on)),
        ),
        Pref::ChatRemoteControl(on) => set(
            doc,
            Some("chat"),
            "remoteControl",
            Some(toml_edit::value(*on)),
        ),
        Pref::OpenLinksInApp(on) => {
            set(doc, Some("links"), "openInApp", Some(toml_edit::value(*on)))
        }
        Pref::TranscriptView(view) => set(
            doc,
            Some("chat"),
            "transcriptView",
            Some(toml_edit::value(view.name())),
        ),
        Pref::TranscriptWidth(width) => set(
            doc,
            Some("chat"),
            "transcriptWidth",
            Some(toml_edit::value(width.name())),
        ),
        Pref::CodeFont(font) => set(doc, None, "codeFont", font.as_deref().map(toml_edit::value)),
        Pref::InterfaceFont(font) => set(
            doc,
            None,
            "interfaceFont",
            font.as_deref().map(toml_edit::value),
        ),
        Pref::ChatFont(font) => set(
            doc,
            Some("chat"),
            "font",
            font.as_deref().map(toml_edit::value),
        ),
        Pref::CodeTheme { dark, name } => set(
            doc,
            Some("code"),
            if *dark { "themeDark" } else { "themeLight" },
            name.as_deref().map(toml_edit::value),
        ),
        Pref::ThemeId(id) => set(doc, None, "themeId", Some(toml_edit::value(id.as_str()))),
        Pref::InterfaceSize(size) => set(doc, None, "interfaceSize", Some(toml_edit::value(*size))),
        Pref::Notifications(on) => set(
            doc,
            None,
            "notificationsEnabled",
            Some(toml_edit::value(*on)),
        ),
        Pref::LaunchAtStartup(on) => set(doc, None, "launchAtStartup", Some(toml_edit::value(*on))),
        Pref::DeveloperMode(on) => set(doc, None, "developerMode", Some(toml_edit::value(*on))),
        Pref::PermissionSound(on) => {
            let sound = table(doc, "sound")?;
            if !sound.contains_key("events") {
                sound.insert("events", toml_edit::Item::Table(toml_edit::Table::new()));
            }
            sound
                .get_mut("events")
                .and_then(toml_edit::Item::as_table_like_mut)
                .ok_or(Error::InvalidSection)?
                .insert("actionRequired", toml_edit::value(*on));
            Ok(())
        }
    }
}

/// `item` `None` apaga a chave (e a tabela, se ficar vazia e foi criada por nós não importa:
/// tabela vazia é válida).
fn set(
    doc: &mut toml_edit::DocumentMut,
    table_name: Option<&str>,
    key: &str,
    item: Option<toml_edit::Item>,
) -> Result<(), Error> {
    let target: &mut dyn toml_edit::TableLike = match table_name {
        Some(name) => {
            if item.is_none() && !doc.contains_key(name) {
                return Ok(());
            }
            table(doc, name)?
        }
        None => doc.as_table_mut(),
    };
    match item {
        Some(item) => {
            target.insert(key, item);
        }
        None => {
            target.remove(key);
        }
    }
    Ok(())
}

fn table<'a>(
    doc: &'a mut toml_edit::DocumentMut,
    key: &str,
) -> Result<&'a mut dyn toml_edit::TableLike, Error> {
    if !doc.contains_key(key) {
        doc.as_table_mut()
            .insert(key, toml_edit::Item::Table(toml_edit::Table::new()));
    }
    doc.get_mut(key)
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or(Error::InvalidSection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_config::schema::SoundEvent;

    #[test]
    fn each_pref_lands_in_its_key_and_unknown_keys_survive() {
        let dir = std::env::temp_dir().join(format!("gsh-claude-prefs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "future = 'keep'\n[chat]\nfutureChat = 1\n").unwrap();
        save(&path, &Pref::ClassifySessionStates(false)).unwrap();
        save(&path, &Pref::ChatRemoteControl(true)).unwrap();
        save(&path, &Pref::OpenLinksInApp(true)).unwrap();
        save(&path, &Pref::TranscriptView(TranscriptView::Verbose)).unwrap();
        save(&path, &Pref::CodeFont(Some("JetBrains Mono".into()))).unwrap();
        save(
            &path,
            &Pref::CodeTheme {
                dark: true,
                name: Some("oneDark".into()),
            },
        )
        .unwrap();
        save(&path, &Pref::PermissionSound(false)).unwrap();
        let config = save(&path, &Pref::TranscriptWidth(TranscriptWidth::Wide)).unwrap();
        assert!(!config.chat.classify_session_states);
        assert!(config.chat.remote_control);
        assert!(config.links.open_in_app);
        assert_eq!(config.chat.transcript_view, TranscriptView::Verbose);
        assert_eq!(config.chat.transcript_width, TranscriptWidth::Wide);
        assert_eq!(config.code_font.as_deref(), Some("JetBrains Mono"));
        assert_eq!(config.code.theme_dark.as_deref(), Some("oneDark"));
        assert_eq!(
            config.sound.events.get(&SoundEvent::ActionRequired),
            Some(&false)
        );
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("future = 'keep'") && text.contains("futureChat = 1"));

        let config = save(&path, &Pref::CodeFont(None)).unwrap();
        assert_eq!(config.code_font, None);
        let config = save(
            &path,
            &Pref::CodeTheme {
                dark: true,
                name: None,
            },
        )
        .unwrap();
        assert_eq!(config.code.theme_dark, None);
        // apagar o que nunca existiu não cria tabela nem mexe no arquivo
        let before = std::fs::read_to_string(&path).unwrap();
        save(
            &path,
            &Pref::CodeTheme {
                dark: false,
                name: None,
            },
        )
        .unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_existing_file_is_not_overwritten() {
        let dir = std::env::temp_dir().join(format!("gsh-claude-prefs-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "chat = [").unwrap();
        assert!(save(&path, &Pref::OpenLinksInApp(true)).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "chat = [");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
