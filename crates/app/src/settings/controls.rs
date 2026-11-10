//! Validação por seção e alteração pontual do TOML existente.
use super::{Error, Section, Settings};
use std::path::Path;

pub fn save(path: &Path, settings: &Settings) -> Result<gsh_config::Config, Error> {
    let _ = gsh_config::load(path).map_err(Error::Config)?;
    let original = match std::fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(Error::Io(error)),
    };
    let mut doc = original
        .parse::<toml_edit::DocumentMut>()
        .map_err(Error::Document)?;
    match settings.section {
        Section::Appearance => {
            let size = number(
                &settings.interface_size,
                10.0,
                20.0,
                Error::InvalidInterfaceSize,
            )?;
            doc.as_table_mut()
                .insert("interfaceSize", toml_edit::value(size));
            if settings.theme_mode == gsh_config::schema::AppThemeMode::Dark {
                doc.as_table_mut()
                    .insert("themeMode", toml_edit::value("dark"));
            }
        }
        Section::Terminal => {
            let shell = settings.shell.trim();
            if shell.is_empty() || shell.contains('\0') {
                return Err(Error::InvalidShell);
            }
            let scrollback = settings
                .scrollback
                .trim()
                .parse::<i64>()
                .map_err(|_| Error::InvalidScrollback)?;
            if !(1000..=100_000).contains(&scrollback) {
                return Err(Error::InvalidScrollback);
            }
            let size = number(
                &settings.terminal_size,
                8.0,
                32.0,
                Error::InvalidTerminalSize,
            )?;
            let table = table(&mut doc, "terminal")?;
            table.insert("shell", toml_edit::value(shell));
            table.insert("scrollback", toml_edit::value(scrollback));
            doc.as_table_mut()
                .insert("terminalSize", toml_edit::value(size));
        }
        Section::Editor => {
            let tab = settings
                .tab_size
                .trim()
                .parse::<i64>()
                .map_err(|_| Error::InvalidTabSize)?;
            if tab != 2 && tab != 4 {
                return Err(Error::InvalidTabSize);
            }
            let table = table(&mut doc, "editor")?;
            table.insert("tabSize", toml_edit::value(tab));
            table.insert("syntaxHighlight", toml_edit::value(settings.highlight));
            table.insert("wordWrap", toml_edit::value(settings.wrap));
        }
        Section::Notifications => {
            doc.as_table_mut().insert(
                "notificationsEnabled",
                toml_edit::value(settings.notifications),
            );
        }
        Section::General => return gsh_config::load(path).map_err(Error::Config),
    }
    let contents = doc.to_string();
    let config = gsh_config::load::parse(path, &contents).map_err(Error::Config)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    super::write_atomic(path, &contents)?;
    Ok(config)
}
fn number(value: &str, min: f64, max: f64, error: Error) -> Result<f64, Error> {
    match value.trim().parse::<f64>() {
        Ok(value) if value.is_finite() && (min..=max).contains(&value) => Ok(value),
        _ => Err(error),
    }
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
    use gsh_config::schema::AppThemeMode;

    #[test]
    fn sections_round_trip_and_invalid_values_do_not_write() {
        let dir = std::env::temp_dir().join(format!("gsh-controls-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            "future = 'keep'\n[terminal]\nfutureTerm = 7\n[editor]\nfutureEditor = true\n",
        )
        .unwrap();
        let mut s = Settings {
            section: Section::Appearance,
            interface_size: "18".into(),
            theme_mode: AppThemeMode::Dark,
            ..Settings::default()
        };
        let config = save(&path, &s).unwrap();
        assert_eq!(config.interface_size, 18.0);
        assert_eq!(config.theme_mode, AppThemeMode::Dark);
        s.section = Section::Terminal;
        s.shell = "/bin/sh".into();
        s.scrollback = "1234".into();
        s.terminal_size = "16".into();
        let config = save(&path, &s).unwrap();
        assert_eq!(config.terminal.shell.as_deref(), Some("/bin/sh"));
        assert_eq!(config.terminal.scrollback, 1234);
        assert_eq!(config.terminal_size, Some(16.0));
        s.section = Section::Editor;
        s.tab_size = "2".into();
        s.highlight = false;
        s.wrap = true;
        let config = save(&path, &s).unwrap();
        assert_eq!(config.editor.tab_size, 2);
        assert!(!config.editor.syntax_highlight);
        assert!(config.editor.word_wrap);
        s.section = Section::Notifications;
        s.notifications = false;
        assert!(!save(&path, &s).unwrap().notifications_enabled);
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("future = 'keep'"));
        assert!(saved.contains("futureTerm = 7"));
        assert!(saved.contains("futureEditor = true"));
        s.section = Section::Terminal;
        s.shell = " ".into();
        assert!(matches!(save(&path, &s), Err(Error::InvalidShell)));
        s.shell = "/bin/sh".into();
        s.scrollback = "999".into();
        assert!(matches!(save(&path, &s), Err(Error::InvalidScrollback)));
        s.scrollback = "1234".into();
        s.terminal_size = "NaN".into();
        assert!(matches!(save(&path, &s), Err(Error::InvalidTerminalSize)));
        s.section = Section::Editor;
        s.tab_size = "3".into();
        assert!(matches!(save(&path, &s), Err(Error::InvalidTabSize)));
        s.section = Section::Appearance;
        s.interface_size = "21".into();
        assert!(matches!(save(&path, &s), Err(Error::InvalidInterfaceSize)));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), saved);
        let reread = gsh_config::load(&path).unwrap();
        assert_eq!(reread.terminal.scrollback, 1234);
        assert_eq!(reread.editor.tab_size, 2);
        assert!(!reread.notifications_enabled);
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn old_config_fills_new_defaults() {
        let cfg = gsh_config::load::parse(
            Path::new("config.toml"),
            "[terminal]\nengine = 'xterm'\n[editor]\nengine = 'gsh'\n",
        )
        .unwrap();
        assert!(cfg.terminal.shell.is_none());
        assert_eq!(cfg.terminal.scrollback, 10_000);
        assert_eq!(cfg.editor.tab_size, 4);
        assert!(cfg.editor.syntax_highlight);
        assert!(!cfg.editor.word_wrap);
    }
}
