//! Leitura de layouts .ckp do Flutter no Linux.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use thiserror::Error;
use yaml_rust::scanner::{Scanner, TokenType};
use yaml_rust::{Yaml, YamlLoader};

use crate::migrate::read_migration_file;
use crate::ConfigError;

#[derive(Debug, PartialEq, Eq)]
pub struct CkpLayout {
    pub name: String,
    pub autorun_worktree: bool,
    pub panes: Vec<CkpPane>,
    pub skipped_by_platform: Vec<CkpPane>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CkpPane {
    pub name: String,
    pub cwd: PathBuf,
    pub split: CkpSplit,
    pub command: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum CkpSplit {
    Tab,
    Right,
    Down,
}

#[derive(Debug, Error)]
pub enum CkpError {
    #[error(transparent)]
    Read(#[from] ConfigError),
    #[error("layout file not found: {}", .0.display())]
    Missing(PathBuf),
    #[error("invalid YAML at line {line}, column {column}")]
    InvalidYaml { line: usize, column: usize },
    #[error("layout YAML exceeds nesting limit or contains an alias")]
    ComplexYaml,
    #[error("layout needs one YAML mapping")]
    Mapping,
    #[error("layout needs a non-empty panes list")]
    Panes,
    #[error("invalid autorun (only worktree is supported)")]
    Autorun,
    #[error("invalid panes[{index}].{field}")]
    Pane { index: usize, field: &'static str },
    #[error("duplicated pane name at panes[{0}]")]
    Duplicate(usize),
}

pub fn load(path: &Path) -> Result<CkpLayout, CkpError> {
    let source = read_migration_file(path)?.ok_or_else(|| CkpError::Missing(path.to_owned()))?;
    parse(path, &source)
}

/// Interpreta metadados. Comandos e autorun não são executados ao ler.
pub fn parse(path: &Path, source: &str) -> Result<CkpLayout, CkpError> {
    // O loader expande aliases copiando subárvores, mesmo para campos ignorados.
    let mut depth: usize = 0;
    for token in Scanner::new(source.chars()) {
        match token.1 {
            TokenType::Alias(_) => return Err(CkpError::ComplexYaml),
            TokenType::BlockSequenceStart
            | TokenType::BlockMappingStart
            | TokenType::FlowSequenceStart
            | TokenType::FlowMappingStart => {
                depth += 1;
                if depth > crate::MAX_KEY_DEPTH {
                    return Err(CkpError::ComplexYaml);
                }
            }
            TokenType::BlockEnd | TokenType::FlowSequenceEnd | TokenType::FlowMappingEnd => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    let docs = YamlLoader::load_from_str(source).map_err(|error| CkpError::InvalidYaml {
        line: error.marker().line(),
        column: error.marker().col() + 1,
    })?;
    let [doc] = docs.as_slice() else {
        return Err(CkpError::Mapping);
    };
    let doc = doc.as_hash().ok_or(CkpError::Mapping)?;
    let autorun_worktree = match doc.get(&Yaml::from_str("autorun")) {
        None | Some(Yaml::Null) => false,
        Some(Yaml::String(value)) if value == "worktree" => true,
        _ => return Err(CkpError::Autorun),
    };
    let panes = doc
        .get(&Yaml::from_str("panes"))
        .and_then(Yaml::as_vec)
        .filter(|panes| !panes.is_empty())
        .ok_or(CkpError::Panes)?;
    let mut result = CkpLayout {
        name: path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
        autorun_worktree,
        panes: Vec::new(),
        skipped_by_platform: Vec::new(),
    };
    let mut seen = HashSet::new();
    for (index, pane) in panes.iter().enumerate() {
        let error = |field| CkpError::Pane { index, field };
        let pane = pane.as_hash().ok_or_else(|| error("mapping"))?;
        let get = |key| {
            pane.get(&Yaml::from_str(key))
                .filter(|value| !value.is_null())
        };
        let name = get("name")
            .and_then(Yaml::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .ok_or_else(|| error("name"))?;
        if !seen.insert(name.to_lowercase()) {
            return Err(CkpError::Duplicate(index));
        }
        let cwd = match get("cwd") {
            None => ".",
            Some(Yaml::String(value)) => value,
            _ => return Err(error("cwd")),
        };
        if Path::new(cwd).is_absolute()
            || cwd.contains(['\\', '\0'])
            || matches!(cwd.as_bytes(), [drive, b':', ..] if drive.is_ascii_alphabetic())
        {
            return Err(error("cwd"));
        }
        let split = match get("split") {
            None => CkpSplit::Tab,
            Some(Yaml::String(value)) if value == "tab" => CkpSplit::Tab,
            Some(Yaml::String(value)) if value == "right" => CkpSplit::Right,
            Some(Yaml::String(value)) if value == "down" => CkpSplit::Down,
            _ => return Err(error("split")),
        };
        let command = match get("command") {
            None => None,
            Some(Yaml::String(value)) => Some(value.trim().to_owned()),
            _ => return Err(error("command")),
        };
        let linux = match get("platforms") {
            Some(Yaml::String(value)) => value.eq_ignore_ascii_case("linux"),
            Some(Yaml::Array(values)) if !values.is_empty() => values
                .iter()
                .filter_map(Yaml::as_str)
                .any(|value| value.eq_ignore_ascii_case("linux")),
            _ => true,
        };
        let pane = CkpPane {
            name: name.to_owned(),
            cwd: PathBuf::from(cwd),
            split,
            command,
        };
        if linux {
            result.panes.push(pane);
        } else {
            result.skipped_by_platform.push(pane);
        }
    }
    Ok(result)
}
