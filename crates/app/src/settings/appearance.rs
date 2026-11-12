//! Validação e persistência da aparência.

use super::Error;
use gsh_config::schema::AppThemeMode;
use std::path::Path;

pub fn save(path: &Path, input: &str) -> Result<f64, Error> {
    let size = input
        .trim()
        .parse::<f64>()
        .map_err(|_| Error::InvalidInterfaceSize)?;
    if !size.is_finite() || !(10.0..=20.0).contains(&size) {
        return Err(Error::InvalidInterfaceSize);
    }
    write_key(path, "interfaceSize", toml_edit::value(size))?;
    Ok(size)
}

/// Aceita os modos de tema oferecidos pela interface.
pub fn parse_theme(input: &str) -> Result<AppThemeMode, Error> {
    [
        AppThemeMode::System,
        AppThemeMode::Light,
        AppThemeMode::Dark,
    ]
    .into_iter()
    .find(|mode| mode.name() == input)
    .ok_or(Error::InvalidTheme)
}

/// Valida e grava `themeMode`, preservando o resto do arquivo.
pub fn save_theme(path: &Path, input: &str) -> Result<AppThemeMode, Error> {
    let mode = parse_theme(input)?;
    write_key(path, "themeMode", toml_edit::value(mode.name()))?;
    Ok(mode)
}

fn write_key(path: &Path, key: &str, item: toml_edit::Item) -> Result<(), Error> {
    let _ = gsh_config::load(path).map_err(Error::Config)?;
    let original = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(Error::Io(error)),
    };
    let mut document = original
        .parse::<toml_edit::DocumentMut>()
        .map_err(Error::Document)?;
    document.as_table_mut().insert(key, item);
    let contents = document.to_string();
    let _ = gsh_config::load::parse(path, &contents).map_err(Error::Config)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    super::write_atomic(path, &contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_rejects_invalid_and_preserves_unknown() {
        let dir =
            std::env::temp_dir().join(format!("gsh-settings-appearance-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "futureKey = 'keep'\n[future]\nnested = 42\n").unwrap();
        save(&path, "18").unwrap();
        assert_eq!(gsh_config::load(&path).unwrap().interface_size, 18.0);
        let original = std::fs::read_to_string(&path).unwrap();
        assert!(original.contains("futureKey = 'keep'"));
        assert!(original.contains("nested = 42"));
        for value in ["9", "21", "NaN", "wrong"] {
            assert!(matches!(
                save(&path, value),
                Err(Error::InvalidInterfaceSize)
            ));
            assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn theme_is_saved_reread_and_invalid_values_do_not_write() {
        let dir = std::env::temp_dir().join(format!("gsh-settings-theme-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "futureKey = 'keep'\n[future]\nnested = 42\n").unwrap();
        assert_eq!(save_theme(&path, "light").unwrap(), AppThemeMode::Light);
        assert_eq!(
            gsh_config::load(&path).unwrap().theme_mode,
            AppThemeMode::Light
        );
        let saved = std::fs::read_to_string(&path).unwrap();
        assert!(saved.contains("futureKey = 'keep'"));
        assert!(saved.contains("nested = 42"));
        for value in ["neon", "", "LIGHT", "3"] {
            assert!(matches!(save_theme(&path, value), Err(Error::InvalidTheme)));
            assert_eq!(std::fs::read_to_string(&path).unwrap(), saved);
        }
        assert_eq!(save_theme(&path, "dark").unwrap(), AppThemeMode::Dark);
        assert_eq!(
            gsh_config::load(&path).unwrap().theme_mode,
            AppThemeMode::Dark
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
