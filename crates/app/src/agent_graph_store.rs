//! Topologia, aparência e log observável do grafo de agentes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

pub type AgentPair = (String, String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalAgent {
    pub app: crate::desktop_agent_apps::DesktopApp,
    pub label: String,
    pub cwd: String,
    pub last_seen_ms: u64,
    pub inbox: Vec<AgentMessage>,
}

pub fn agent_pair(a: &str, b: &str) -> AgentPair {
    if a <= b {
        (a.to_owned(), b.to_owned())
    } else {
        (b.to_owned(), a.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMessage {
    pub from_tab_id: String,
    pub to_tab_id: String,
    pub sent_at_ms: u64,
    pub preview: String,
    pub signed_by: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AgentGraphStore {
    positions: BTreeMap<String, (f32, f32)>,
    links: BTreeSet<AgentPair>,
    maestro: Option<String>,
    colors: BTreeMap<String, u32>,
    unmonitored: BTreeSet<String>,
    messages: Vec<AgentMessage>,
    externals: BTreeMap<String, ExternalAgent>,
}

impl AgentGraphStore {
    /// Posiciona nós vistos pela primeira vez na primeira célula da grade sem sobrepor nós.
    pub fn place_missing(&mut self, ids: &[String]) -> Vec<String> {
        const ORIGIN: f32 = 80.0;
        const COLUMNS: usize = 4;
        const CELL_WIDTH: f32 = 280.0;
        const CELL_HEIGHT: f32 = 170.0;
        let mut placed = Vec::new();
        let missing = ids
            .iter()
            .filter(|id| !self.positions.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>();
        for id in missing {
            let mut slot = 0;
            loop {
                let at = (
                    ORIGIN + (slot % COLUMNS) as f32 * CELL_WIDTH,
                    ORIGIN + (slot / COLUMNS) as f32 * CELL_HEIGHT,
                );
                slot += 1;
                if self.positions.values().all(|position| {
                    (position.0 - at.0).abs() >= CELL_WIDTH / 2.0
                        || (position.1 - at.1).abs() >= CELL_HEIGHT / 2.0
                }) {
                    self.set_position(&id, at.0, at.1);
                    placed.push(id);
                    break;
                }
            }
        }
        placed
    }

    pub fn positions(&self) -> &BTreeMap<String, (f32, f32)> {
        &self.positions
    }
    pub fn links(&self) -> &BTreeSet<AgentPair> {
        &self.links
    }
    pub fn maestro(&self) -> Option<&str> {
        self.maestro.as_deref()
    }
    pub fn messages(&self) -> &[AgentMessage] {
        &self.messages
    }
    pub fn externals(&self) -> &BTreeMap<String, ExternalAgent> {
        &self.externals
    }
    pub fn add_external(
        &mut self,
        app: crate::desktop_agent_apps::DesktopApp,
        label: String,
        cwd: String,
    ) -> String {
        let id = format!("ext:{}", app.id());
        self.externals.insert(
            id.clone(),
            ExternalAgent {
                app,
                label,
                cwd,
                last_seen_ms: 0,
                inbox: Vec::new(),
            },
        );
        id
    }
    pub fn external_seen(&mut self, id: &str, now_ms: u64) -> bool {
        let Some(external) = self.externals.get_mut(id) else {
            return false;
        };
        external.last_seen_ms = now_ms;
        true
    }
    pub fn enqueue_external(&mut self, id: &str, message: AgentMessage) -> bool {
        let Some(external) = self.externals.get_mut(id) else {
            return false;
        };
        external.inbox.push(message);
        if external.inbox.len() > 200 {
            external.inbox.remove(0);
        }
        true
    }
    pub fn take_external_messages(&mut self, id: &str) -> Option<Vec<AgentMessage>> {
        self.externals
            .get_mut(id)
            .map(|external| std::mem::take(&mut external.inbox))
    }
    pub fn remove_agent(&mut self, id: &str) {
        self.positions.remove(id);
        self.links.retain(|(a, b)| a != id && b != id);
        self.colors.remove(id);
        self.unmonitored.remove(id);
        self.externals.remove(id);
        if self.maestro.as_deref() == Some(id) {
            self.maestro = None;
        }
    }
    pub fn rename_external(&mut self, id: &str, label: String) -> bool {
        let Some(external) = self.externals.get_mut(id) else {
            return false;
        };
        external.label = label;
        true
    }
    pub fn position(&self, id: &str) -> Option<(f32, f32)> {
        self.positions.get(id).copied()
    }
    pub fn color(&self, id: &str) -> Option<u32> {
        self.colors.get(id).copied()
    }
    pub fn is_monitored(&self, id: &str) -> bool {
        !self.unmonitored.contains(id)
    }
    pub fn peers(&self, id: &str) -> Vec<&str> {
        self.links
            .iter()
            .filter_map(|(a, b)| {
                if a == id {
                    Some(b.as_str())
                } else if b == id {
                    Some(a.as_str())
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn set_position(&mut self, id: &str, x: f32, y: f32) {
        if x.is_finite() && y.is_finite() {
            self.positions.insert(id.to_owned(), (x, y));
        }
    }
    pub fn add_link(&mut self, a: &str, b: &str) -> bool {
        a != b && self.links.insert(agent_pair(a, b))
    }
    pub fn remove_link(&mut self, a: &str, b: &str) -> bool {
        self.links.remove(&agent_pair(a, b))
    }
    pub fn set_color(&mut self, id: &str, color: Option<u32>) {
        if let Some(color) = color {
            self.colors.insert(id.to_owned(), color);
        } else {
            self.colors.remove(id);
        }
    }
    pub fn set_monitored(&mut self, id: &str, monitored: bool) {
        if monitored {
            self.unmonitored.remove(id);
        } else {
            self.unmonitored.insert(id.to_owned());
        }
    }
    pub fn set_maestro(&mut self, id: Option<&str>, agents: &[String]) -> Vec<String> {
        self.maestro = id.map(str::to_owned);
        let Some(id) = id else { return Vec::new() };
        agents
            .iter()
            .filter(|peer| peer.as_str() != id && self.add_link(id, peer))
            .cloned()
            .collect()
    }
    pub fn record(&mut self, message: AgentMessage) {
        self.messages.push(message);
        if self.messages.len() > 1000 {
            self.messages.remove(0);
        }
    }
    pub fn observed_edges(&self) -> BTreeMap<AgentPair, (usize, &AgentMessage)> {
        let mut edges = BTreeMap::new();
        for message in &self.messages {
            if message.from_tab_id == message.to_tab_id {
                continue;
            }
            let key = agent_pair(&message.from_tab_id, &message.to_tab_id);
            let count = edges.get(&key).map_or(1, |(count, _)| count + 1);
            edges.insert(key, (count, message));
        }
        edges
    }
    /// O objeto interno mantém os mesmos nomes e filtros de ids do Flutter.
    pub fn to_json(&self, live: &BTreeSet<String>) -> Value {
        json!({
            "positions": self.positions.iter().filter(|(id, _)| live.contains(*id)).map(|(id,(x,y))| (id.clone(), json!([x,y]))).collect::<BTreeMap<_,_>>(),
            "links": self.links.iter().filter(|(a,b)| live.contains(a) && live.contains(b)).collect::<Vec<_>>(),
            "maestro": self.maestro.as_ref().filter(|id| live.contains(*id)),
            "colors": self.colors.iter().filter(|(id,_)| live.contains(*id)).collect::<BTreeMap<_,_>>(),
            "unmonitored": self.unmonitored.iter().filter(|id| live.contains(*id)).collect::<Vec<_>>(),
            "externals": self.externals.iter().filter(|(id, _)| live.contains(*id)).map(|(_, external)| json!({"app": external.app.id(), "label": external.label, "cwd": external.cwd})).collect::<Vec<_>>()
        })
    }
    pub fn restore(&mut self, value: &Value, remap: &BTreeMap<String, String>) {
        *self = Self::default();
        let map_id = |id: &str| remap.get(id).cloned().unwrap_or_else(|| id.to_owned());
        if let Some(positions) = value.get("positions").and_then(Value::as_object) {
            for (id, position) in positions {
                if let Some(pair) = position.as_array().filter(|p| p.len() == 2) {
                    if let (Some(x), Some(y)) = (
                        pair.first().and_then(Value::as_f64),
                        pair.get(1).and_then(Value::as_f64),
                    ) {
                        self.set_position(&map_id(id), x as f32, y as f32);
                    }
                }
            }
        }
        if let Some(links) = value.get("links").and_then(Value::as_array) {
            for link in links {
                if let (Some(a), Some(b)) = (
                    link.get(0).and_then(Value::as_str),
                    link.get(1).and_then(Value::as_str),
                ) {
                    self.add_link(&map_id(a), &map_id(b));
                }
            }
        }
        if let Some(id) = value.get("maestro").and_then(Value::as_str) {
            self.maestro = Some(map_id(id));
        }
        if let Some(colors) = value.get("colors").and_then(Value::as_object) {
            for (id, value) in colors {
                if let Some(color) = value.as_u64().and_then(|n| u32::try_from(n).ok()) {
                    self.colors.insert(map_id(id), color);
                }
            }
        }
        if let Some(ids) = value.get("unmonitored").and_then(Value::as_array) {
            for id in ids.iter().filter_map(Value::as_str) {
                self.unmonitored.insert(map_id(id));
            }
        }
        if let Some(externals) = value.get("externals").and_then(Value::as_array) {
            for value in externals {
                let (Some(app), Some(label)) = (
                    value.get("app").and_then(Value::as_str),
                    value.get("label").and_then(Value::as_str),
                ) else {
                    continue;
                };
                let Some(app) = crate::desktop_agent_apps::DesktopApp::from_id(app) else {
                    continue;
                };
                let id = format!("ext:{}", app.id());
                self.externals.insert(
                    id,
                    ExternalAgent {
                        app,
                        label: label.to_owned(),
                        cwd: value
                            .get("cwd")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        last_seen_ms: 0,
                        inbox: Vec::new(),
                    },
                );
            }
        }
    }
}

/// Carrega o campo `agentGraph` do documento `layouts` compatível com o KV Flutter.
pub fn load_workspace(workspace: &Path) -> Option<Value> {
    let path = layouts_path();
    let root: Value = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
    let raw = root
        .get("data")?
        .get(workspace.to_string_lossy().as_ref())?
        .as_str()?;
    serde_json::from_str::<Value>(raw)
        .ok()?
        .get("agentGraph")
        .cloned()
}

/// Atualiza só `agentGraph`, preservando os demais campos do doc e as outras workspaces.
pub fn save_workspace(
    workspace: &Path,
    graph: &AgentGraphStore,
    live: &BTreeSet<String>,
) -> Result<(), String> {
    let path = layouts_path();
    let mut root = match fs::read(&path) {
        Ok(bytes) => serde_json::from_slice::<Value>(&bytes)
            .map_err(|error| format!("{}: {error}", path.display()))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            json!({"version": 1, "data": {}})
        }
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let data = root
        .get_mut("data")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{}: invalid layouts store", path.display()))?;
    let key = workspace.to_string_lossy().into_owned();
    let mut document = data
        .get(&key)
        .and_then(Value::as_str)
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    document
        .as_object_mut()
        .ok_or_else(|| "invalid workspace layout".to_owned())?
        .insert("agentGraph".into(), graph.to_json(live));
    data.insert(
        key,
        Value::String(serde_json::to_string(&document).map_err(|error| error.to_string())?),
    );
    let bytes = serde_json::to_vec(&root).map_err(|error| error.to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "layouts store has no parent".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    let temp = path.with_extension(format!("json.tmp-{}", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| format!("{}: {error}", temp.display()))?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        let _ = fs::remove_file(&temp);
        return Err(format!("{}: {error}", temp.display()));
    }
    if let Err(error) = fs::rename(&temp, &path) {
        let _ = fs::remove_file(&temp);
        return Err(format!("{}: {error}", path.display()));
    }
    Ok(())
}

fn layouts_path() -> PathBuf {
    gsh_ipc::gsh_dir_from_env().join("layouts.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn links_are_unordered_self_links_are_ignored_and_json_filters_dead_ids() {
        let mut graph = AgentGraphStore::default();
        assert!(graph.add_link("t2", "t1"));
        assert!(!graph.add_link("t1", "t2"));
        assert!(!graph.add_link("t1", "t1"));
        graph.set_position("t1", 80.0, 80.0);
        graph.set_position("dead", 10.0, 10.0);
        let json = graph.to_json(&BTreeSet::from(["t1".to_owned(), "t2".to_owned()]));
        assert_eq!(json["positions"]["t1"], json!([80.0, 80.0]));
        assert!(json["positions"].get("dead").is_none());
        assert_eq!(json["links"][0], json!(["t1", "t2"]));
    }
    #[test]
    fn restore_remaps_ids_and_rejects_malformed_coordinates() {
        let mut graph = AgentGraphStore::default();
        graph.restore(&json!({"positions":{"old":[3,4],"bad":[1]},"links":[["old","x"],["x","x"]],"maestro":"old"}), &BTreeMap::from([("old".into(),"new".into())]));
        assert_eq!(graph.position("new"), Some((3.0, 4.0)));
        assert_eq!(graph.maestro(), Some("new"));
        assert_eq!(graph.links(), &BTreeSet::from([agent_pair("new", "x")]));
    }

    #[test]
    fn agent_pair_and_observed_edges_match_dart_golden() {
        let messages = [
            AgentMessage {
                from_tab_id: "t2".into(),
                to_tab_id: "t1".into(),
                sent_at_ms: 1,
                preview: String::new(),
                signed_by: String::new(),
            },
            AgentMessage {
                from_tab_id: "t1".into(),
                to_tab_id: "t2".into(),
                sent_at_ms: 2,
                preview: String::new(),
                signed_by: String::new(),
            },
            AgentMessage {
                from_tab_id: "t1".into(),
                to_tab_id: "t1".into(),
                sent_at_ms: 3,
                preview: String::new(),
                signed_by: String::new(),
            },
            AgentMessage {
                from_tab_id: "t10".into(),
                to_tab_id: "t2".into(),
                sent_at_ms: 4,
                preview: String::new(),
                signed_by: String::new(),
            },
        ];
        let expected: Value =
            serde_json::from_str(include_str!("../../../../golden/agent_graph_golden.json"))
                .expect("Dart golden JSON");
        let graph = AgentGraphStore {
            messages: messages.to_vec(),
            ..Default::default()
        };
        let observed = graph.observed_edges();
        let actual = json!({
            "pairs": [agent_pair("t2", "t1"), agent_pair("t2", "t10")],
            "observed": observed.iter().map(|((a,b),(count,last))| json!({"pair":[a,b],"count":count,"last":[last.from_tab_id,last.to_tab_id]})).collect::<Vec<_>>(),
        });
        assert_eq!(actual, expected);
    }
}
