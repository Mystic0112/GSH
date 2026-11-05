//! Caminho, leitura e a última configuração válida.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::error::ConfigError;
use crate::schema::Config;

/// Pasta do GSH na home (a mesma do `status.sock` e do `storage_root`).
const GSH_DIR: &str = ".gsh";
const FILE_NAME: &str = "config.toml";

/// `~/.gsh/config.toml`.
pub fn default_path() -> Result<PathBuf, ConfigError> {
    dirs::home_dir()
        .map(|home| home.join(GSH_DIR).join(FILE_NAME))
        .ok_or(ConfigError::NoHome)
}

/// Lê o arquivo. Ausente = padrões (o arquivo não é criado).
pub fn load(path: &Path) -> Result<Config, ConfigError> {
    match fs::read_to_string(path) {
        Ok(text) => parse(path, &text),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(source) => Err(ConfigError::Read {
            path: path.to_owned(),
            source,
        }),
    }
}

/// Decodifica o texto do arquivo; `path` só entra no erro.
pub fn parse(path: &Path, text: &str) -> Result<Config, ConfigError> {
    let config: Config = toml::from_str(text).map_err(|e| {
        let (line, column) = e
            .span()
            .map(|span| line_column(text, span.start))
            .map_or((None, None), |(l, c)| (Some(l), Some(c)));
        ConfigError::Parse {
            path: path.to_owned(),
            line,
            column,
            cause: e.message().to_owned(),
        }
    })?;
    config
        .normalized()
        .map_err(|(key, value)| ConfigError::NonFinite {
            path: path.to_owned(),
            key,
            value,
        })
}

/// Linha e coluna (1-based, coluna em caracteres) do byte `offset`.
fn line_column(text: &str, offset: usize) -> (usize, usize) {
    let before = text.get(..offset).unwrap_or(text);
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    let column = before.get(line_start..).map_or(0, |s| s.chars().count()) + 1;
    (line, column)
}

/// Estado do arquivo no disco que o polling compara. `None` = não existe.
/// O tamanho entra junto do mtime para pegar duas gravações no mesmo tick de
/// um sistema de arquivos de resolução grossa.
pub(crate) type Fingerprint = Option<(SystemTime, u64)>;

pub(crate) fn fingerprint(path: &Path) -> Fingerprint {
    let meta = fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Configuração corrente de um arquivo. Recarga com erro mantém a última
/// válida.
#[derive(Debug)]
pub struct ConfigStore {
    path: PathBuf,
    current: Config,
    seen: Fingerprint,
}

impl ConfigStore {
    /// Abre o arquivo em `path`. Com arquivo inválido, o store fica com os
    /// padrões e o erro volta junto, para a UI avisar.
    pub fn open(path: impl Into<PathBuf>) -> (Self, Result<(), ConfigError>) {
        let mut store = Self {
            path: path.into(),
            current: Config::default(),
            seen: None,
        };
        let result = store.reload().map(|_| ());
        (store, result)
    }

    /// Relê o arquivo. `Ok` troca a configuração corrente; `Err` a mantém.
    pub fn reload(&mut self) -> Result<&Config, ConfigError> {
        // Stat antes de ler: se o arquivo mudar no meio, o próximo poll vê um
        // fingerprint diferente e relê.
        self.seen = fingerprint(&self.path);
        self.current = load(&self.path)?;
        Ok(&self.current)
    }

    pub fn current(&self) -> &Config {
        &self.current
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Fingerprint do arquivo na última leitura.
    pub(crate) fn seen(&self) -> Fingerprint {
        self.seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_column_is_one_based_and_counts_chars() {
        let text = "a = 1\nção = x\n";
        assert_eq!(line_column(text, 0), (1, 1));
        assert_eq!(line_column(text, 6), (2, 1));
        // "ção = " tem 6 caracteres e 8 bytes.
        assert_eq!(line_column(text, 6 + 8), (2, 7));
        // Offset fora do texto não estoura.
        assert_eq!(line_column(text, 999).0, 3);
    }

    #[test]
    fn default_path_ends_in_gsh_config_toml() {
        let path = default_path().unwrap();
        assert!(path.ends_with(Path::new(".gsh").join("config.toml")));
    }

    #[test]
    fn parse_error_display_carries_path_and_position() {
        let err = parse(Path::new("/x/config.toml"), "a = \n").unwrap_err();
        let shown = err.to_string();
        assert!(shown.starts_with("/x/config.toml:1:"), "{shown}");
    }
}
