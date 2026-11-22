use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path, PathBuf};

use gsh_config::flutter_state::{
    FlutterDirection, FlutterLayout, FlutterPane, FlutterState, FlutterTab,
};

use super::{Axis, LayoutDoc, LayoutNode, PaneTab, VERSION};

pub(super) fn convert(state: FlutterState) -> Option<LayoutDoc> {
    let active = state.workspaces.get(state.active_workspace)?.path.clone();
    let mut paths = Vec::new();
    let mut names = BTreeMap::new();
    let mut saved = BTreeMap::new();
    let mut doc = empty_doc();
    for workspace in state.workspaces {
        paths.push(workspace.path.clone());
        if let Some(name) = workspace.name.filter(|name| !name.trim().is_empty()) {
            names.insert(workspace.path.clone(), name);
        }
        if let Some(layout) = workspace.layout {
            let layout = convert_layout(&workspace.path, layout);
            if workspace.path == active {
                doc = layout.clone();
            }
            saved.insert(workspace.path, layout);
        }
    }
    doc.workspaces = paths;
    doc.active_workspace = state.active_workspace;
    doc.workspace_names = names;
    doc.imported_layouts = saved;
    Some(doc)
}

fn empty_doc() -> LayoutDoc {
    LayoutDoc {
        version: VERSION,
        tree: LayoutNode::Pane {
            tabs: Vec::new(),
            active: None,
            terminal_labels: Vec::new(),
        },
        focused: Vec::new(),
        workspaces: Vec::new(),
        active_workspace: 0,
        terminal_cwds: Vec::new(),
        remote_terminal_ids: Vec::new(),
        chat_sessions: Vec::new(),
        chat_profiles: BTreeMap::new(),
        query_files: Vec::new(),
        query_connections: Vec::new(),
        workspace_names: BTreeMap::new(),
        workspace_realms: BTreeMap::new(),
        active_channel: None,
        imported_layouts: BTreeMap::new(),
    }
}

fn convert_layout(root: &Path, layout: FlutterLayout) -> LayoutDoc {
    let mut doc = empty_doc();
    let mut sessions = layout.sessions;
    let mut next_id = 1;
    let mut leaf_tabs = HashSet::new();
    doc.tree = node(
        layout.tree,
        &mut |id, tabs, active, position| {
            if layout.focused.as_deref() == Some(id) {
                doc.focused = position.to_vec();
            }
            let mut active_index = None;
            let mut labels = Vec::new();
            let mut result = Vec::new();
            for key in tabs {
                let Some(session) = sessions.remove(&key) else {
                    continue;
                };
                let id = next_id;
                let converted = match session {
                    FlutterTab::Terminal {
                        cwd,
                        sub,
                        label,
                        title,
                    } => {
                        let dir = cwd
                            .filter(|path| path.is_absolute() && path.is_dir())
                            .or_else(|| sub.and_then(|sub| relative_dir(root, &sub)))
                            .unwrap_or_else(|| root.to_owned());
                        doc.terminal_cwds.push((id, dir));
                        if let Some(label) =
                            label.or(title).filter(|label| !label.trim().is_empty())
                        {
                            labels.push((id, label));
                        }
                        PaneTab::Terminal(id)
                    }
                    FlutterTab::Viewer { path } => {
                        let path = if path.is_absolute() {
                            path
                        } else {
                            root.join(path)
                        };
                        if !path.is_file() {
                            continue;
                        }
                        PaneTab::File(path)
                    }
                    FlutterTab::Chat { sid } => {
                        if let Some(sid) = sid.filter(|sid| !sid.is_empty()) {
                            doc.chat_sessions.push((id, sid));
                        }
                        PaneTab::Chat(id)
                    }
                    FlutterTab::Redis { conn } => PaneTab::Redis { id, conn },
                    FlutterTab::Mongo { conn, collection } => PaneTab::Mongo {
                        id,
                        conn,
                        collection,
                    },
                    FlutterTab::Farm => PaneTab::Farm,
                    FlutterTab::AgentGraph => PaneTab::AgentGraph(root.to_owned()),
                    FlutterTab::Harness => PaneTab::Harness(root.to_owned()),
                    FlutterTab::Unsupported => continue,
                };
                // Arquivos e abas únicas não podem aparecer duas vezes na árvore Rust.
                if !leaf_tabs.insert(tab_identity(&converted)) {
                    continue;
                }
                next_id += 1;
                if key == active {
                    active_index = Some(result.len());
                }
                result.push(converted);
            }
            LayoutNode::Pane {
                tabs: result,
                active: active_index,
                terminal_labels: labels,
            }
        },
        &mut Vec::new(),
    );
    doc
}

fn relative_dir(root: &Path, sub: &Path) -> Option<PathBuf> {
    if !sub
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
    {
        return None;
    }
    let path = root.join(sub);
    path.is_dir().then_some(path)
}

fn tab_identity(tab: &PaneTab) -> String {
    match tab {
        PaneTab::File(path) => format!("file:{}", path.display()),
        PaneTab::AgentGraph(_) => "agent_graph".to_owned(),
        PaneTab::Harness(_) => "harness".to_owned(),
        PaneTab::Farm => "farm".to_owned(),
        _ => format!("{tab:?}"),
    }
}

fn node(
    pane: FlutterPane,
    leaf: &mut impl FnMut(&str, Vec<String>, String, &[usize]) -> LayoutNode,
    position: &mut Vec<usize>,
) -> LayoutNode {
    match pane {
        FlutterPane::Leaf { id, tabs, active } => leaf(&id, tabs, active, position),
        FlutterPane::Split { dir, frac, a, b } => {
            position.push(0);
            let a = node(*a, leaf, position);
            position.pop();
            position.push(1);
            let b = node(*b, leaf, position);
            position.pop();
            LayoutNode::Split {
                axis: match dir {
                    FlutterDirection::Horizontal => Axis::Horizontal,
                    FlutterDirection::Vertical => Axis::Vertical,
                },
                ratio: frac,
                a: Box::new(a),
                b: Box::new(b),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_active_tree_and_retains_other_workspace_layout() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        std::fs::create_dir_all(first.join("src")).unwrap();
        std::fs::create_dir(&second).unwrap();
        let leaf =
            |id, tab| serde_json::json!({"k": "leaf", "id": id, "tabs": [tab], "active": tab});
        let layout = serde_json::json!({
            "v": 1, "focused": "right",
            "tree": {"k": "split", "dir": "vertical", "frac": 0.36, "a": leaf("left", "t0"), "b": leaf("right", "a0")},
            "sessions": {
                "t0": {"type": "terminal", "sub": "src", "title": "Terminal", "label": "Worker"},
                "a0": {"type": "chat", "sid": "chat-session"}
            }
        });
        let projects = serde_json::json!({"version": 1, "data": {
            "first": {"id": "first", "path": first, "order": 1, "name": "Code"},
            "second": {"id": "second", "path": second, "order": 2}
        }});
        let layouts = serde_json::json!({"version": 1, "data": {
            "first": layout.to_string(),
            "second": layout.to_string()
        }});
        let projects_path = dir.path().join("projects.json");
        let layouts_path = dir.path().join("layouts.json");
        std::fs::write(&projects_path, projects.to_string()).unwrap();
        std::fs::write(&layouts_path, layouts.to_string()).unwrap();
        let state = gsh_config::flutter_state::load_dir(dir.path())
            .unwrap()
            .unwrap();
        let doc = convert(state).unwrap();
        assert_eq!(
            doc.workspace_names.get(&first).map(String::as_str),
            Some("Code")
        );
        assert_eq!(doc.workspaces, [first.clone(), second.clone()]);
        assert_eq!(doc.focused, [1]);
        assert_eq!(doc.terminal_cwds, [(1, first.join("src"))]);
        assert_eq!(doc.chat_sessions, [(2, "chat-session".to_owned())]);
        assert!(matches!(
            doc.tree,
            LayoutNode::Split {
                axis: Axis::Vertical,
                ..
            }
        ));
        let target = dir.path().join("layout.json");
        doc.write_atomic(&target).unwrap();
        let persisted = LayoutDoc::read(&target).unwrap();
        assert!(persisted.imported_layouts.contains_key(&second));
        assert_eq!(
            std::fs::read_to_string(projects_path).unwrap(),
            projects.to_string()
        );
        assert_eq!(
            std::fs::read_to_string(layouts_path).unwrap(),
            layouts.to_string()
        );
    }
}
