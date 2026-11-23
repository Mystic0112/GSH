//! Persistência do layout da janela principal.
use gsh_core::workspace::ChannelState;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use crate::pane_model::{ChatId, PaneModel, PaneTab, QueryId, TermId};

#[cfg(target_os = "linux")]
mod flutter;
mod workspaces;
pub use workspaces::SessionContext;
use workspaces::SystemWorkspace;

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutDoc {
    pub version: u32,
    pub tree: LayoutNode,
    pub focused: Vec<usize>,
    pub workspaces: Vec<PathBuf>,
    pub active_workspace: usize,
    #[serde(default)]
    pub terminal_cwds: Vec<(TermId, PathBuf)>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remote_terminal_ids: Vec<TermId>,
    #[serde(default)]
    pub chat_sessions: Vec<(ChatId, String)>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub chat_profiles: BTreeMap<ChatId, PathBuf>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub chat_contexts: BTreeMap<ChatId, SessionContext>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub terminal_contexts: BTreeMap<TermId, SessionContext>,
    #[serde(default)]
    pub query_files: Vec<(QueryId, PathBuf)>,
    #[serde(default)]
    pub query_connections: Vec<(QueryId, String)>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub workspace_names: BTreeMap<PathBuf, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub workspace_realms: BTreeMap<PathBuf, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    system_workspaces: Vec<SystemWorkspace>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    channel_state: Option<ChannelState>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub imported_layouts: BTreeMap<PathBuf, LayoutDoc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayoutNode {
    Split {
        axis: Axis,
        ratio: f32,
        a: Box<LayoutNode>,
        b: Box<LayoutNode>,
    },
    Pane {
        tabs: Vec<PaneTab>,
        active: Option<usize>,
        terminal_labels: Vec<(TermId, String)>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl LayoutDoc {
    pub fn read(path: &Path) -> Option<Self> {
        let doc: Self = serde_json::from_slice(&fs::read(path).ok()?).ok()?;
        (doc.version == VERSION).then_some(doc)
    }
    pub fn read_for_startup(path: &Path) -> Option<Self> {
        #[cfg(target_os = "linux")]
        if fs::symlink_metadata(path).is_err_and(|error| error.kind() == io::ErrorKind::NotFound) {
            return match gsh_config::default_flutter_candidates()
                .and_then(|candidates| gsh_config::flutter_state::load(&candidates))
            {
                Ok(state) => state.and_then(flutter::convert),
                Err(error) => {
                    eprintln!("GSH layout import: {error}");
                    None
                }
            };
        }
        Self::read(path)
    }
    pub fn write_atomic(&self, path: &Path) -> io::Result<()> {
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "layout path has no parent")
        })?;
        fs::create_dir_all(parent)?;
        let temp = path.with_extension("json.tmp");
        fs::write(&temp, serde_json::to_vec(self).map_err(io::Error::other)?)?;
        fs::rename(temp, path)
    }
    pub fn default_path() -> Option<PathBuf> {
        std::env::var_os("GSH_LAYOUT_PATH")
            .map(PathBuf::from)
            .or_else(|| {
                dirs::data_dir().map(|d| d.join("work.gorila.gshdesktop").join("layout.json"))
            })
    }
}

impl crate::pane_tree::PaneTree {
    pub fn snapshot(&self) -> LayoutDoc {
        use iced::widget::pane_grid::Node;
        fn node(tree: &crate::pane_tree::PaneTree, n: &Node) -> LayoutNode {
            match n {
                Node::Pane(p) => {
                    let empty = PaneModel::empty();
                    let m = tree.model(*p).unwrap_or(&empty);
                    LayoutNode::Pane {
                        tabs: m.tabs().to_vec(),
                        active: m.active_index(),
                        terminal_labels: m
                            .tabs()
                            .iter()
                            .filter_map(|t| {
                                if let PaneTab::Terminal(id) = t {
                                    m.terminal_label(*id).map(|s| (*id, s.to_owned()))
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    }
                }
                Node::Split {
                    axis, ratio, a, b, ..
                } => LayoutNode::Split {
                    axis: match axis {
                        iced::widget::pane_grid::Axis::Horizontal => Axis::Horizontal,
                        iced::widget::pane_grid::Axis::Vertical => Axis::Vertical,
                    },
                    ratio: *ratio,
                    a: Box::new(node(tree, a)),
                    b: Box::new(node(tree, b)),
                },
            }
        }
        fn path(root: &Node, focused: iced::widget::pane_grid::Pane, out: &mut Vec<usize>) -> bool {
            match root {
                Node::Pane(p) => *p == focused,
                Node::Split { a, b, .. } => {
                    out.push(0);
                    if path(a, focused, out) {
                        true
                    } else {
                        out.pop();
                        out.push(1);
                        if path(b, focused, out) {
                            true
                        } else {
                            out.pop();
                            false
                        }
                    }
                }
            }
        }
        let root = self.grid().layout();
        let mut focused = Vec::new();
        path(root, self.focused(), &mut focused);
        LayoutDoc {
            version: VERSION,
            tree: node(self, root),
            focused,
            workspaces: Vec::new(),
            active_workspace: 0,
            terminal_cwds: Vec::new(),
            remote_terminal_ids: Vec::new(),
            chat_sessions: Vec::new(),
            chat_profiles: BTreeMap::new(),
            chat_contexts: BTreeMap::new(),
            terminal_contexts: BTreeMap::new(),
            query_files: Vec::new(),
            query_connections: Vec::new(),
            workspace_names: BTreeMap::new(),
            workspace_realms: BTreeMap::new(),
            system_workspaces: Vec::new(),
            active_channel: None,
            channel_state: None,
            imported_layouts: BTreeMap::new(),
        }
    }
    pub fn from_doc(doc: &LayoutDoc) -> Self {
        use iced::widget::pane_grid::{self, Configuration};
        let remote_terminals = doc
            .remote_terminal_ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        fn config(
            n: &LayoutNode,
            models: &mut Vec<PaneModel>,
            next_term: &mut TermId,
            query_files: &[(QueryId, PathBuf)],
            remote_terminals: &BTreeSet<TermId>,
        ) -> Configuration<PaneModel> {
            match n {
                LayoutNode::Pane {
                    tabs,
                    active,
                    terminal_labels,
                } => {
                    let kept: Vec<_> = tabs
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| match t {
                            PaneTab::Terminal(id) => !remote_terminals.contains(id),
                            PaneTab::File(p) => p.exists(),
                            PaneTab::Query { id, .. } => query_files
                                .iter()
                                .find(|(saved_id, _)| saved_id == id)
                                .is_some_and(|(_, path)| std::fs::read_to_string(path).is_ok()),
                            PaneTab::Harness(p) | PaneTab::AgentGraph(p) => p.exists(),
                            _ => true,
                        })
                        .map(|(index, tab)| {
                            let tab = match tab {
                                PaneTab::File(path) => PaneTab::File(
                                    path.canonicalize().unwrap_or_else(|_| path.clone()),
                                ),
                                _ => tab.clone(),
                            };
                            (index, tab)
                        })
                        .collect();
                    let restored_active = active.and_then(|active| {
                        kept.iter().position(|(original, _)| *original == active)
                    });
                    let mut restored_tabs: Vec<_> = kept.into_iter().map(|(_, tab)| tab).collect();
                    if restored_tabs.is_empty() {
                        restored_tabs.push(PaneTab::Terminal(*next_term));
                        *next_term = next_term.saturating_add(1);
                    }
                    let labels = terminal_labels
                        .iter()
                        .filter(|(id, _)| !remote_terminals.contains(id))
                        .cloned()
                        .collect();
                    let model = PaneModel::restore(restored_tabs, restored_active, labels);
                    models.push(model.clone());
                    Configuration::Pane(model)
                }
                LayoutNode::Split { axis, ratio, a, b } => Configuration::Split {
                    axis: match axis {
                        Axis::Horizontal => pane_grid::Axis::Horizontal,
                        Axis::Vertical => pane_grid::Axis::Vertical,
                    },
                    ratio: ratio.clamp(0.16, 0.84),
                    a: Box::new(config(a, models, next_term, query_files, remote_terminals)),
                    b: Box::new(config(b, models, next_term, query_files, remote_terminals)),
                },
            }
        }
        fn max_term(node: &LayoutNode, remote_terminals: &BTreeSet<TermId>) -> TermId {
            match node {
                LayoutNode::Pane { tabs, .. } => tabs
                    .iter()
                    .filter_map(|tab| {
                        if let PaneTab::Terminal(id) = tab {
                            (!remote_terminals.contains(id)).then_some(*id)
                        } else {
                            None
                        }
                    })
                    .max()
                    .unwrap_or(0),
                LayoutNode::Split { a, b, .. } => {
                    max_term(a, remote_terminals).max(max_term(b, remote_terminals))
                }
            }
        }
        let mut models = Vec::new();
        let mut next_term = max_term(&doc.tree, &remote_terminals).saturating_add(1);
        let configuration = config(
            &doc.tree,
            &mut models,
            &mut next_term,
            &doc.query_files,
            &remote_terminals,
        );
        let grid = pane_grid::State::with_configuration(configuration);
        let leaves: Vec<_> = {
            fn walk(n: &pane_grid::Node, v: &mut Vec<pane_grid::Pane>) {
                match n {
                    pane_grid::Node::Pane(p) => v.push(*p),
                    pane_grid::Node::Split { a, b, .. } => {
                        walk(a, v);
                        walk(b, v)
                    }
                }
            }
            let mut v = Vec::new();
            walk(grid.layout(), &mut v);
            v
        };
        let focus = doc.focused.iter().fold(grid.layout(), |n, i| match n {
            pane_grid::Node::Split { a, b, .. } => {
                if *i == 0 {
                    a
                } else {
                    b
                }
            }
            x => x,
        });
        let focused = match focus {
            pane_grid::Node::Pane(p) => *p,
            _ => match leaves.first().copied() {
                Some(p) => p,
                None => return Self::default(),
            },
        };
        Self::from_restored_parts(grid, focused, next_term)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_or_unknown_version_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("layout.json");
        fs::write(&p, b"{").unwrap();
        assert!(LayoutDoc::read(&p).is_none());
        fs::write(&p, b"{\"version\":99}").unwrap();
        assert!(LayoutDoc::read(&p).is_none());
    }
    #[test]
    fn atomic_write_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x/layout.json");
        let doc = crate::pane_tree::PaneTree::default().snapshot();
        doc.write_atomic(&p).unwrap();
        assert_eq!(LayoutDoc::read(&p).unwrap().version, VERSION);
        assert!(!p.with_extension("json.tmp").exists());
    }
    #[test]
    fn restored_tree_preserves_tab_order_active_and_focus() {
        let mut tree = crate::pane_tree::PaneTree::default();
        let p = tree.focused();
        tree.model_mut(p).unwrap().open_settings();
        tree.model_mut(p).unwrap().reorder_tab(0, 2);
        tree.split_with_terminal(p, iced::widget::pane_grid::Axis::Vertical);
        let split = match tree.grid().layout() {
            iced::widget::pane_grid::Node::Split { id, .. } => *id,
            _ => unreachable!(),
        };
        tree.resize(split, 0.42);
        let doc = tree.snapshot();
        let restored = crate::pane_tree::PaneTree::from_doc(&doc);
        assert_eq!(restored.leaves().len(), 2);
        assert_eq!(restored.focused(), restored.leaves()[1]);
        match restored.grid().layout() {
            iced::widget::pane_grid::Node::Split { ratio, .. } => {
                assert!((*ratio - 0.42).abs() < f32::EPSILON)
            }
            _ => unreachable!(),
        }
        assert_eq!(
            restored.model(restored.leaves()[0]).unwrap().tabs(),
            tree.model(p).unwrap().tabs()
        );
        assert_eq!(
            restored.model(restored.leaves()[0]).unwrap().active_index(),
            tree.model(p).unwrap().active_index()
        );
    }

    #[test]
    fn missing_files_are_discarded_and_empty_panes_get_terminal() {
        let doc = crate::pane_tree::PaneTree::default().snapshot();
        if let LayoutNode::Pane {
            tabs: _,
            active,
            terminal_labels,
        } = doc.tree
        {
            let missing = std::env::temp_dir().join("gsh-layout-file-absent");
            let doc = LayoutDoc {
                tree: LayoutNode::Pane {
                    tabs: vec![PaneTab::File(missing)],
                    active,
                    terminal_labels,
                },
                ..doc
            };
            let restored = crate::pane_tree::PaneTree::from_doc(&doc);
            assert!(matches!(
                restored.focused_model().tabs(),
                [PaneTab::Terminal(_)]
            ));
        } else {
            unreachable!();
        }
    }

    #[test]
    fn new_session_metadata_round_trips_and_old_v1_documents_still_read() {
        let mut doc = crate::pane_tree::PaneTree::default().snapshot();
        doc.chat_sessions = vec![(7, "session-7".into())];
        doc.chat_profiles
            .insert(7, PathBuf::from("/tmp/claude-work"));
        doc.query_files = vec![(8, PathBuf::from("query.dbq"))];
        doc.query_connections = vec![(8, "db".into())];
        let json = serde_json::to_value(&doc).unwrap();
        assert_eq!(serde_json::from_value::<LayoutDoc>(json).unwrap(), doc);
        let mut old = serde_json::to_value(&doc).unwrap();
        let object = old.as_object_mut().unwrap();
        object.remove("chat_sessions");
        object.remove("chat_profiles");
        object.remove("query_files");
        object.remove("query_connections");
        let decoded: LayoutDoc = serde_json::from_value(old).unwrap();
        assert!(decoded.chat_sessions.is_empty());
        assert!(decoded.query_files.is_empty());
        assert!(decoded.query_connections.is_empty());
    }

    #[test]
    fn scratch_queries_and_queries_missing_a_file_are_discarded() {
        let mut tree = crate::pane_tree::PaneTree::default();
        let pane = tree.focused();
        tree.model_mut(pane)
            .unwrap()
            .open_query(1, "Untitled-1.dbq".into());
        let doc = tree.snapshot();
        let restored = crate::pane_tree::PaneTree::from_doc(&doc);
        assert!(!restored
            .focused_model()
            .tabs()
            .iter()
            .any(|tab| matches!(tab, PaneTab::Query { .. })));
    }

    #[test]
    fn chat_query_redis_and_mongo_tabs_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let query_path = dir.path().join("saved.dbq");
        fs::write(&query_path, "-- db: pg\nselect 1;\n").unwrap();
        let mut tree = crate::pane_tree::PaneTree::default();
        let model = tree.focused_model_mut();
        model.open_chat(12);
        model.open_query(13, "saved.dbq".into());
        model.open_redis(14, "cache".into());
        model.open_mongo(15, "mongo".into(), "items".into());
        let mut doc = tree.snapshot();
        doc.chat_sessions = vec![(12, "session-12".into())];
        doc.query_files = vec![(13, query_path)];
        doc.query_connections = vec![(13, "selected-pg".into())];
        let restored = crate::pane_tree::PaneTree::from_doc(&doc);
        let tabs = restored.focused_model().tabs();
        assert!(tabs.contains(&PaneTab::Chat(12)));
        assert!(tabs.contains(&PaneTab::Query {
            id: 13,
            title: "saved.dbq".into()
        }));
        assert!(tabs.contains(&PaneTab::Redis {
            id: 14,
            conn: "cache".into()
        }));
        assert!(tabs.contains(&PaneTab::Mongo {
            id: 15,
            conn: "mongo".into(),
            collection: "items".into()
        }));
        assert_eq!(doc.chat_sessions[0].1, "session-12");
        assert_eq!(doc.query_connections[0].1, "selected-pg");
    }

    #[test]
    fn remote_terminals_are_excluded_when_restoring_layout() {
        let mut tree = crate::pane_tree::PaneTree::default();
        let model = tree.focused_model_mut();
        model.open_terminal(2);
        model.set_terminal_label(2, "prod · SSH".into());
        let mut doc = tree.snapshot();
        doc.remote_terminal_ids = vec![2];
        let restored = crate::pane_tree::PaneTree::from_doc(&doc);
        assert_eq!(restored.focused_model().tabs(), &[PaneTab::Terminal(1)]);
        assert_eq!(restored.focused_model().terminal_label(2), None);
    }
}
