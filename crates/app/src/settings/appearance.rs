//! Validação e persistência da aparência.

use super::Error;
use std::path::Path;

pub fn save(path: &Path, input: &str) -> Result<f64, Error> {
    let size = input
        .trim()
        .parse::<f64>()
        .map_err(|_| Error::InvalidInterfaceSize)?;
    if !size.is_finite() || !(10.0..=20.0).contains(&size) {
        return Err(Error::InvalidInterfaceSize);
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
    document
        .as_table_mut()
        .insert("interfaceSize", toml_edit::value(size));
    let contents = document.to_string();
    let _ = gsh_config::load::parse(path, &contents).map_err(Error::Config)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::Io)?;
    }
    super::write_atomic(path, &contents)?;
    Ok(size)
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
}
