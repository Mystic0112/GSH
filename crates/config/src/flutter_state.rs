//! Leitura do estado do GSH Desktop Flutter (JsonStateStore).

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::error::{ConfigError, JsonParseCause};
use crate::migrate::read_migration_file;

#[derive(Debug)]
pub struct FlutterState {
    pub workspaces: Vec<FlutterWorkspace>,
    pub active_workspace: usize,
}

#[derive(Debug)]
pub struct FlutterWorkspace {
    pub path: PathBuf,
    pub name: Option<String>,
    pub layout: Option<FlutterLayout>,
}

#[derive(Debug, Deserialize)]
pub struct FlutterLayout {
    v: u32,
    pub tree: FlutterPane,
    pub focused: Option<String>,
    pub sessions: BTreeMap<String, FlutterTab>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "k", rename_all = "lowercase")]
pub enum FlutterPane {
    Leaf {
        id: String,
        tabs: Vec<String>,
        active: String,
    },
    Split {
        dir: FlutterDirection,
        frac: f32,
        a: Box<FlutterPane>,
        b: Box<FlutterPane>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlutterDirection {
    Horizontal,
    Vertical,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FlutterTab {
    Terminal {
        cwd: Option<PathBuf>,
        sub: Option<PathBuf>,
        title: Option<String>,
        label: Option<String>,
    },
    #[serde(alias = "neovim", alias = "file_engine", alias = "notebook")]
    Viewer {
        path: PathBuf,
    },
    Chat {
        sid: Option<String>,
    },
    Redis {
        conn: String,
    },
    Mongo {
        conn: String,
        collection: String,
    },
    Farm,
    AgentGraph,
    Harness,
    #[serde(other)]
    Unsupported,
}

#[derive(Deserialize)]
struct Project {
    id: String,
    path: PathBuf,
    name: Option<String>,
    #[serde(rename = "createdAt", default)]
    created_at: i64,
    #[serde(default)]
    order: i64,
}

/// Candidatos são caminhos de settings, para compartilhar a busca da migração
/// existente. Cada importação usa os arquivos de uma única pasta state.
pub fn load(candidates: &[PathBuf]) -> Result<Option<FlutterState>, ConfigError> {
    for dir in candidates.iter().filter_map(|path| path.parent()) {
        if let Some(state) = load_dir(dir)? {
            return Ok(Some(state));
        }
    }
    Ok(None)
}

pub fn load_dir(dir: &Path) -> Result<Option<FlutterState>, ConfigError> {
    let project_path = dir.join("projects.json");
    let Some(projects) = read_store(&project_path)? else {
        return Ok(None);
    };
    let active_realm = read_store(&dir.join("realms.json"))?
        .and_then(|realms| {
            realms
                .get("__active__")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "default".to_owned());
    let selected = projects
        .get(&format!("__last_selected__::{active_realm}"))
        .or_else(|| projects.get("__last_selected__"))
        .and_then(Value::as_str);
    let mut entries = projects
        .values()
        .filter(|value| value.is_object())
        .map(|value| {
            serde_json::from_value::<Project>(value.clone())
                .map_err(|err| json_error(&project_path, err))
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|p| (p.order, p.created_at));

    let layout_path = dir.join("layouts.json");
    let layouts = read_store(&layout_path)?.unwrap_or_default();
    let mut active_workspace = 0;
    let mut workspaces = Vec::new();
    let mut seen = HashSet::new();
    for project in entries {
        if !project.path.is_absolute() || !seen.insert(project.path.clone()) {
            continue;
        }
        let raw = layouts
            .get(&project.id)
            .or_else(|| project.path.to_str().and_then(|path| layouts.get(path)));
        let layout = raw
            .map(|value| decode_layout(&layout_path, value))
            .transpose()?
            .flatten();
        if selected == Some(project.id.as_str())
            || selected.is_some_and(|s| Path::new(s) == project.path)
        {
            active_workspace = workspaces.len();
        }
        workspaces.push(FlutterWorkspace {
            path: project.path,
            name: project.name,
            layout,
        });
    }
    Ok((!workspaces.is_empty()).then_some(FlutterState {
        workspaces,
        active_workspace,
    }))
}

fn decode_layout(path: &Path, stored: &Value) -> Result<Option<FlutterLayout>, ConfigError> {
    let encoded = stored
        .as_str()
        .ok_or_else(|| invalid(path, JsonParseCause::InvalidData))?;
    let value: Value = serde_json::from_str(encoded).map_err(|err| json_error(path, err))?;
    // Flutter can persist a rail toggle before a workspace has a pane tree.
    if value.get("tree").is_none()
        && value
            .get("worktreesExpanded")
            .is_some_and(Value::is_boolean)
    {
        return Ok(None);
    }
    let layout: FlutterLayout =
        serde_json::from_value(value).map_err(|err| json_error(path, err))?;
    if layout.v != 1 {
        return Err(invalid(path, JsonParseCause::UnsupportedVersion));
    }
    Ok(Some(layout))
}

fn read_store(path: &Path) -> Result<Option<Map<String, Value>>, ConfigError> {
    let Some(text) = read_migration_file(path)? else {
        return Ok(None);
    };
    #[derive(Deserialize)]
    struct Store {
        version: u32,
        data: Map<String, Value>,
    }
    let store: Store = serde_json::from_str(&text).map_err(|err| json_error(path, err))?;
    if store.version != 1 {
        return Err(invalid(path, JsonParseCause::UnsupportedVersion));
    }
    Ok(Some(store.data))
}

fn invalid(path: &Path, cause: JsonParseCause) -> ConfigError {
    ConfigError::JsonParse {
        path: path.to_owned(),
        line: None,
        column: None,
        cause,
    }
}

fn json_error(path: &Path, err: serde_json::Error) -> ConfigError {
    ConfigError::JsonParse {
        path: path.to_owned(),
        line: Some(err.line()).filter(|line| *line > 0),
        column: Some(err.column()).filter(|column| *column > 0),
        cause: JsonParseCause::from_category(err.classify()),
    }
}
