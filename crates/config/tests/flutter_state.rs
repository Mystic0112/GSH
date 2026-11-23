#![cfg(target_os = "linux")]

use std::{fs, path::PathBuf};

use gsh_config::{
    ckp::{self, CkpError, CkpLayout, CkpPane, CkpSplit},
    flutter_state::{self, FlutterDirection, FlutterPane, FlutterTab},
    migrate_if_needed, ConfigError, JsonParseCause, MigrationOutcome, MAX_KEY_DEPTH,
};
use serde_json::json;

struct CopyDir(PathBuf);

impl CopyDir {
    fn create(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("gsh-{label}-{}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for CopyDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn reads_order_selection_and_both_workspace_layouts() {
    let dir = CopyDir::create("flutter-state");
    let projects = json!({"version": 1, "data": {
        "__last_selected__::default": "second",
        "first": {"id": "first", "path": "/tmp/alpha", "name": "Alpha", "order": 1},
        "second": {"id": "second", "path": "/tmp/beta", "name": "Beta", "order": 2},
        "third": {"id": "third", "path": "/tmp/gamma", "order": 3},
        "invalid": {"id": "invalid", "path": "relative"}
    }});
    let tree = json!({
        "k": "split", "id": "split", "dir": "vertical", "frac": 0.4,
        "a": {"id": "left", "k": "leaf", "tabs": ["p0"], "active": "p0"},
        "b": {"id": "right", "k": "leaf", "tabs": ["p1"], "active": "p1"}
    });
    let layout = json!({"v": 1, "tree": tree, "focused": "right", "sessions": {
        "p0": {"type": "terminal", "sub": "src"},
        "p1": {"type": "chat", "sid": "example"}
    }});
    let files = [
        (dir.0.join("projects.json"), projects.to_string()),
        (
            dir.0.join("layouts.json"),
            json!({"version": 1, "data": {
                "first": layout.to_string(), "/tmp/beta": layout.to_string(),
                "third": json!({"worktreesExpanded": false}).to_string()
            }})
            .to_string(),
        ),
    ];
    for (path, text) in &files {
        fs::write(path, text).unwrap();
    }
    let state = flutter_state::load_dir(&dir.0).unwrap().unwrap();
    assert_eq!(state.active_workspace, 1);
    assert_eq!(state.workspaces.len(), 3);
    assert_eq!(state.workspaces[0].name.as_deref(), Some("Alpha"));
    assert!(state.workspaces[2].layout.is_none());
    for workspace in state.workspaces.iter().take(2) {
        let layout = workspace.layout.as_ref().unwrap();
        assert_eq!(layout.focused.as_deref(), Some("right"));
        assert!(matches!(layout.tree, FlutterPane::Split { .. }));
        assert!(matches!(
            layout.sessions.get("p1"),
            Some(FlutterTab::Chat { .. })
        ));
    }
    for (path, text) in files {
        assert_eq!(fs::read_to_string(path).unwrap(), text);
    }
}

#[test]
fn does_not_restore_global_selection_from_a_different_channel() {
    let dir = CopyDir::create("flutter-channel-selection");
    fs::write(
        dir.0.join("realms.json"),
        json!({"version": 1, "data": {
            "__active__": "personal",
            "personal": {"id": "personal", "name": "Pessoal"}
        }})
        .to_string(),
    )
    .unwrap();
    let projects = |personal: bool| {
        let mut data = json!({
            "__last_selected__": "work",
            "work": {"id": "work", "path": "/tmp/work", "order": 0, "realm": "work"},
            "default": {"id": "default", "path": "/tmp/default", "order": 2}
        });
        if personal {
            data["personal"] =
                json!({"id": "personal", "path": "/tmp/personal", "order": 1, "realm": "personal"});
        }
        json!({"version": 1, "data": data}).to_string()
    };
    fs::write(dir.0.join("projects.json"), projects(true)).unwrap();
    let state = flutter_state::load_dir(&dir.0).unwrap().unwrap();
    assert_eq!(state.active_realm.as_deref(), Some("personal"));
    assert_eq!(
        state.workspaces[state.active_workspace].path,
        PathBuf::from("/tmp/personal")
    );

    fs::write(dir.0.join("projects.json"), projects(false)).unwrap();
    let state = flutter_state::load_dir(&dir.0).unwrap().unwrap();
    assert_eq!(state.active_realm, None);
    assert_eq!(
        state.workspaces[state.active_workspace].path,
        PathBuf::from("/tmp/default")
    );

    fs::write(
        dir.0.join("realms.json"),
        json!({"version": 1, "data": {"__active__": "missing"}}).to_string(),
    )
    .unwrap();
    let state = flutter_state::load_dir(&dir.0).unwrap().unwrap();
    assert_eq!(state.active_realm.as_deref(), Some("default"));
    assert_eq!(
        state.workspaces[state.active_workspace].path,
        PathBuf::from("/tmp/default")
    );
}

#[test]
fn reads_gallery_ckp_from_copy_and_rejects_invalid_values() {
    let dir = CopyDir::create("flutter-ckp");
    let path = dir.0.join("dev.ckp");
    let source = include_str!("../../app/assets/gallery/layout.tpl");
    fs::write(&path, source).unwrap();
    let layout = ckp::load(&path).unwrap();
    assert_eq!(layout.name, "dev");
    assert_eq!(layout.panes[1].split, CkpSplit::Right);
    assert_eq!(layout.panes[1].command.as_deref(), Some("claude"));
    assert_eq!(fs::read_to_string(&path).unwrap(), source);

    let layout = ckp::parse(&path, "autorun: worktree\npanes:\n  - name: Linux\n    platforms: [linux, windows]\n  - name: Windows\n    platforms: windows\n").unwrap();
    assert!(layout.autorun_worktree);
    assert_eq!(layout.panes.len(), 1);
    assert_eq!(layout.skipped_by_platform.len(), 1);
    assert!(matches!(
        ckp::parse(&path, "panes:\n  - name: Bad\n    cwd: C:/abs\n"),
        Err(CkpError::Pane { field: "cwd", .. })
    ));
    assert!(matches!(
        ckp::parse(&path, "name: &name Linux\npanes:\n  - name: *name\n"),
        Err(CkpError::ComplexYaml)
    ));
}

#[test]
#[ignore = "requires GSH_FLUTTER_TEST_STATE_DIR pointing at a Flutter state directory"]
fn migrates_copied_flutter_state_and_leaves_source_unchanged() {
    let source = PathBuf::from(std::env::var_os("GSH_FLUTTER_TEST_STATE_DIR").unwrap());
    let dir = CopyDir::create("real-flutter-state");
    let names = ["projects.json", "layouts.json", "settings.json"];
    let originals: Vec<_> = names
        .iter()
        .map(|name| fs::read(source.join(name)).unwrap())
        .collect();
    for name in names {
        fs::copy(source.join(name), dir.0.join(name)).unwrap();
    }
    let state = flutter_state::load_dir(&dir.0).unwrap().unwrap();
    assert!(!state.workspaces.is_empty());
    assert!(state
        .workspaces
        .iter()
        .any(|workspace| workspace.layout.is_some()));
    let target = dir.0.join("config.toml");
    assert_eq!(
        migrate_if_needed(&target, [dir.0.join("settings.json")]).unwrap(),
        MigrationOutcome::Imported
    );
    assert_eq!(
        migrate_if_needed(&target, [dir.0.join("settings.json")]).unwrap(),
        MigrationOutcome::AlreadyMigrated
    );
    assert!(gsh_config::load(&target).is_ok());
    for (name, original) in names.into_iter().zip(originals) {
        assert!(fs::read(source.join(name)).unwrap() == original);
        assert!(fs::read(dir.0.join(name)).unwrap() == original);
    }
}

#[test]
fn flutter_state_golden_preserves_workspaces_realms_dedup_and_all_tab_sessions() {
    let dir = CopyDir::create("flutter-state-golden");
    let realms = json!({
        "version": 1,
        "data": {
            "__active__": "prod",
            "prod": {"id": "prod", "name": "Produção"}
        }
    });
    // `p_late` e `p_early` empatam em `order = 1`; `createdAt` desempata (`p_early` vem antes).
    // `p_dup` repete o path `/srv/early` e deve ser descartado.
    // `__last_selected__::prod` aponta pelo path `/srv/gamma`, selecionando o 3º workspace.
    let projects = json!({
        "version": 1,
        "data": {
            "__last_selected__::default": "p_early",
            "__last_selected__::prod": "/srv/gamma",
            "p_late": {
                "id": "p_late",
                "path": "/srv/late",
                "name": "Late",
                "order": 1,
                "createdAt": 200
            },
            "p_early": {
                "id": "p_early",
                "path": "/srv/early",
                "name": "Early",
                "order": 1,
                "createdAt": 100
            },
            "p_dup": {
                "id": "p_dup",
                "path": "/srv/early",
                "name": "DuplicatePath",
                "order": 2,
                "createdAt": 300
            },
            "p_gamma": {
                "id": "p_gamma",
                "path": "/srv/gamma",
                "realm": "prod",
                "order": 3,
                "createdAt": 400
            },
            "p_rel": {
                "id": "p_rel",
                "path": "relative/workspace",
                "order": 0
            }
        }
    });

    let layout_early = json!({
        "v": 1,
        "focused": "leaf_br",
        "tree": {
            "k": "split",
            "id": "sp_root",
            "dir": "horizontal",
            "frac": 0.35,
            "a": {
                "k": "leaf",
                "id": "leaf_top",
                "tabs": ["t_term", "t_nvim", "t_file", "t_note"],
                "active": "t_nvim"
            },
            "b": {
                "k": "split",
                "id": "sp_bottom",
                "dir": "vertical",
                "frac": 0.65,
                "a": {
                    "k": "leaf",
                    "id": "leaf_bl",
                    "tabs": ["t_chat", "t_redis", "t_mongo"],
                    "active": "t_mongo"
                },
                "b": {
                    "k": "leaf",
                    "id": "leaf_br",
                    "tabs": ["t_farm", "t_graph", "t_harness", "t_future"],
                    "active": "t_graph"
                }
            }
        },
        "sessions": {
            "t_term": {
                "type": "terminal",
                "cwd": "/srv/early",
                "sub": "crates/core",
                "title": "cargo test",
                "label": "Core"
            },
            "t_nvim": {
                "type": "neovim",
                "path": "/srv/early/src/lib.rs"
            },
            "t_file": {
                "type": "file_engine",
                "path": "/srv/early/README.md"
            },
            "t_note": {
                "type": "notebook",
                "path": "/srv/early/notes.notebook"
            },
            "t_chat": {
                "type": "chat",
                "sid": "session-42"
            },
            "t_redis": {
                "type": "redis",
                "conn": "redis://localhost:6379"
            },
            "t_mongo": {
                "type": "mongo",
                "conn": "mongodb://localhost:27017",
                "collection": "events"
            },
            "t_farm": {
                "type": "farm"
            },
            "t_graph": {
                "type": "agent_graph"
            },
            "t_harness": {
                "type": "harness"
            },
            "t_future": {
                "type": "experimental_widget",
                "extra": 123
            }
        }
    });

    let layout_gamma = json!({
        "v": 1,
        "focused": "only",
        "tree": {
            "k": "leaf",
            "id": "only",
            "tabs": ["v1"],
            "active": "v1"
        },
        "sessions": {
            "v1": {
                "type": "viewer",
                "path": "/srv/gamma/Cargo.toml"
            }
        }
    });

    let layouts = json!({
        "version": 1,
        "data": {
            "p_early": layout_early.to_string(),
            "p_late": json!({"worktreesExpanded": true}).to_string(),
            "/srv/gamma": layout_gamma.to_string()
        }
    });

    fs::write(dir.0.join("realms.json"), realms.to_string()).unwrap();
    fs::write(dir.0.join("projects.json"), projects.to_string()).unwrap();
    fs::write(dir.0.join("layouts.json"), layouts.to_string()).unwrap();

    // Testa `flutter_state::load` via lista de candidatos a `settings.json`.
    let missing_candidate = dir.0.join("missing-subdir").join("settings.json");
    let valid_candidate = dir.0.join("settings.json");
    let state = flutter_state::load(&[missing_candidate, valid_candidate])
        .unwrap()
        .expect("estado Flutter deve ser carregado");

    assert_eq!(state.active_workspace, 2);
    assert_eq!(state.workspaces.len(), 3);
    assert_eq!(state.workspaces[0].path, PathBuf::from("/srv/early"));
    assert_eq!(state.workspaces[0].name.as_deref(), Some("Early"));
    assert_eq!(state.workspaces[1].path, PathBuf::from("/srv/late"));
    assert_eq!(state.workspaces[1].name.as_deref(), Some("Late"));
    assert!(state.workspaces[1].layout.is_none());
    assert_eq!(state.workspaces[2].path, PathBuf::from("/srv/gamma"));
    assert_eq!(state.workspaces[2].name, None);

    // Verifica a árvore e cada sessão do primeiro workspace.
    let early = state.workspaces[0].layout.as_ref().unwrap();
    assert_eq!(early.focused.as_deref(), Some("leaf_br"));
    match &early.tree {
        FlutterPane::Split { dir, frac, a, b } => {
            assert!(matches!(dir, FlutterDirection::Horizontal));
            assert!((*frac - 0.35).abs() < 1e-6);
            match a.as_ref() {
                FlutterPane::Leaf { id, tabs, active } => {
                    assert_eq!(id, "leaf_top");
                    assert_eq!(tabs, &["t_term", "t_nvim", "t_file", "t_note"]);
                    assert_eq!(active, "t_nvim");
                }
                other => panic!("esperava Leaf no topo, veio {other:?}"),
            }
            match b.as_ref() {
                FlutterPane::Split { dir, frac, a, b } => {
                    assert!(matches!(dir, FlutterDirection::Vertical));
                    assert!((*frac - 0.65).abs() < 1e-6);
                    assert!(matches!(a.as_ref(), FlutterPane::Leaf { id, .. } if id == "leaf_bl"));
                    assert!(matches!(b.as_ref(), FlutterPane::Leaf { id, .. } if id == "leaf_br"));
                }
                other => panic!("esperava Split embaixo, veio {other:?}"),
            }
        }
        other => panic!("esperava Split na raiz, veio {other:?}"),
    }

    assert!(matches!(
        early.sessions.get("t_term"),
        Some(FlutterTab::Terminal {
            cwd: Some(cwd),
            sub: Some(sub),
            title: Some(title),
            label: Some(label),
        }) if cwd == &PathBuf::from("/srv/early")
            && sub == &PathBuf::from("crates/core")
            && title == "cargo test"
            && label == "Core"
    ));
    for (id, expected_path) in [
        ("t_nvim", "/srv/early/src/lib.rs"),
        ("t_file", "/srv/early/README.md"),
        ("t_note", "/srv/early/notes.notebook"),
    ] {
        assert!(matches!(
            early.sessions.get(id),
            Some(FlutterTab::Viewer { path }) if path == &PathBuf::from(expected_path)
        ));
    }
    assert!(matches!(
        early.sessions.get("t_chat"),
        Some(FlutterTab::Chat { sid: Some(sid) }) if sid == "session-42"
    ));
    assert!(matches!(
        early.sessions.get("t_redis"),
        Some(FlutterTab::Redis { conn }) if conn == "redis://localhost:6379"
    ));
    assert!(matches!(
        early.sessions.get("t_mongo"),
        Some(FlutterTab::Mongo { conn, collection })
            if conn == "mongodb://localhost:27017" && collection == "events"
    ));
    assert!(matches!(
        early.sessions.get("t_farm"),
        Some(FlutterTab::Farm)
    ));
    assert!(matches!(
        early.sessions.get("t_graph"),
        Some(FlutterTab::AgentGraph)
    ));
    assert!(matches!(
        early.sessions.get("t_harness"),
        Some(FlutterTab::Harness)
    ));
    assert!(matches!(
        early.sessions.get("t_future"),
        Some(FlutterTab::Unsupported)
    ));

    // O 3º workspace teve o layout resolvido pelo fallback de `project.path` (`/srv/gamma`).
    let gamma = state.workspaces[2].layout.as_ref().unwrap();
    assert_eq!(gamma.focused.as_deref(), Some("only"));
    assert!(matches!(
        gamma.sessions.get("v1"),
        Some(FlutterTab::Viewer { path }) if path == &PathBuf::from("/srv/gamma/Cargo.toml")
    ));
}

#[path = "flutter_state/malformed.rs"]
mod flutter_state_malformed;
