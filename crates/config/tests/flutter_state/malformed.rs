#![cfg(target_os = "linux")]

use super::*;

#[test]
fn flutter_state_rejects_malformed_store_envelopes_and_layout_payloads() {
    let dir = CopyDir::create("flutter-state-malformed");
    let projects_path = dir.0.join("projects.json");
    let realms_path = dir.0.join("realms.json");
    let layouts_path = dir.0.join("layouts.json");

    // 1. projects.json com version != 1.
    fs::write(
        &projects_path,
        json!({"version": 2, "data": {}}).to_string(),
    )
    .unwrap();
    assert!(matches!(
        flutter_state::load_dir(&dir.0),
        Err(ConfigError::JsonParse {
            path,
            cause: JsonParseCause::UnsupportedVersion,
            ..
        }) if path == projects_path
    ));

    // 2. projects.json com objeto de projeto sem campo obrigatório `path`.
    fs::write(
        &projects_path,
        json!({"version": 1, "data": {"p1": {"id": "p1"}}}).to_string(),
    )
    .unwrap();
    assert!(matches!(
        flutter_state::load_dir(&dir.0),
        Err(ConfigError::JsonParse {
            path,
            cause: JsonParseCause::InvalidData,
            ..
        }) if path == projects_path
    ));

    // Restaura um projects.json válido para testar erros em realms.json e layouts.json.
    let valid_projects = json!({
        "version": 1,
        "data": {
            "p1": {"id": "p1", "path": "/tmp/p1", "order": 1}
        }
    });
    fs::write(&projects_path, valid_projects.to_string()).unwrap();

    // 3. realms.json com version != 1.
    fs::write(&realms_path, json!({"version": 9, "data": {}}).to_string()).unwrap();
    assert!(matches!(
        flutter_state::load_dir(&dir.0),
        Err(ConfigError::JsonParse {
            path,
            cause: JsonParseCause::UnsupportedVersion,
            ..
        }) if path == realms_path
    ));
    fs::remove_file(&realms_path).unwrap();

    // 4. layouts.json onde o valor do workspace não é string JSON encodada.
    fs::write(
        &layouts_path,
        json!({"version": 1, "data": {"p1": {"v": 1}}}).to_string(),
    )
    .unwrap();
    assert!(matches!(
        flutter_state::load_dir(&dir.0),
        Err(ConfigError::JsonParse {
            path,
            cause: JsonParseCause::InvalidData,
            ..
        }) if path == layouts_path
    ));

    // 5. layouts.json com layout interno em versão não suportada (`v: 2`).
    let bad_layout_version = json!({
        "v": 2,
        "tree": {"k": "leaf", "id": "l", "tabs": ["t"], "active": "t"},
        "sessions": {}
    });
    fs::write(
        &layouts_path,
        json!({"version": 1, "data": {"p1": bad_layout_version.to_string()}}).to_string(),
    )
    .unwrap();
    assert!(matches!(
        flutter_state::load_dir(&dir.0),
        Err(ConfigError::JsonParse {
            path,
            cause: JsonParseCause::UnsupportedVersion,
            ..
        }) if path == layouts_path
    ));

    // 6. layouts.json com árvore interna malformada (`dir: "diagonal"`).
    let bad_tree = json!({
        "v": 1,
        "tree": {
            "k": "split",
            "dir": "diagonal",
            "frac": 0.5,
            "a": {"k": "leaf", "id": "a", "tabs": ["t"], "active": "t"},
            "b": {"k": "leaf", "id": "b", "tabs": ["t2"], "active": "t2"}
        },
        "sessions": {}
    });
    fs::write(
        &layouts_path,
        json!({"version": 1, "data": {"p1": bad_tree.to_string()}}).to_string(),
    )
    .unwrap();
    assert!(matches!(
        flutter_state::load_dir(&dir.0),
        Err(ConfigError::JsonParse {
            path,
            cause: JsonParseCause::InvalidData,
            ..
        }) if path == layouts_path
    ));

    // 7. layouts.json com JSON interno truncado.
    fs::write(
        &layouts_path,
        json!({"version": 1, "data": {"p1": "{\"v\": 1, \"tree\":"}}).to_string(),
    )
    .unwrap();
    assert!(matches!(
        flutter_state::load_dir(&dir.0),
        Err(ConfigError::JsonParse {
            path,
            cause: JsonParseCause::UnexpectedEof,
            ..
        }) if path == layouts_path
    ));
}

#[test]
fn ckp_golden_layout_and_malformed_payload_rejection() {
    let dir = CopyDir::create("flutter-ckp-golden");
    let path = dir.0.join("fullstack.ckp");

    let golden_yaml = "\
autorun: worktree
panes:
  - name:  Editor
    split: tab
    command:  nvim .
  - name: Server
    cwd: crates/server
    split: right
    command: cargo run
    platforms: LINUX
  - name: Watcher
    cwd: crates/core
    split: down
    platforms: [macos, Linux]
  - name: MacAgent
    cwd: tools/mac
    split: right
    platforms: [macos, windows]
";
    let parsed = ckp::parse(&path, golden_yaml).unwrap();
    assert_eq!(
        parsed,
        CkpLayout {
            name: "fullstack".into(),
            autorun_worktree: true,
            panes: vec![
                CkpPane {
                    name: "Editor".into(),
                    cwd: PathBuf::from("."),
                    split: CkpSplit::Tab,
                    command: Some("nvim .".into()),
                },
                CkpPane {
                    name: "Server".into(),
                    cwd: PathBuf::from("crates/server"),
                    split: CkpSplit::Right,
                    command: Some("cargo run".into()),
                },
                CkpPane {
                    name: "Watcher".into(),
                    cwd: PathBuf::from("crates/core"),
                    split: CkpSplit::Down,
                    command: None,
                },
            ],
            skipped_by_platform: vec![CkpPane {
                name: "MacAgent".into(),
                cwd: PathBuf::from("tools/mac"),
                split: CkpSplit::Right,
                command: None,
            }],
        }
    );

    // Arquivo ausente devolve `CkpError::Missing`.
    let missing_path = dir.0.join("absent.ckp");
    assert!(matches!(
        ckp::load(&missing_path),
        Err(CkpError::Missing(p)) if p == missing_path
    ));

    // Sintaxe YAML inválida.
    assert!(matches!(
        ckp::parse(&path, "panes:\n  - name: [unclosed\n"),
        Err(CkpError::InvalidYaml { .. })
    ));

    // Profundidade acima de MAX_KEY_DEPTH é rejeitada como ComplexYaml.
    let deep_yaml = format!(
        "{}panes{}",
        "[".repeat(MAX_KEY_DEPTH + 1),
        "]".repeat(MAX_KEY_DEPTH + 1)
    );
    assert!(matches!(
        ckp::parse(&path, &deep_yaml),
        Err(CkpError::ComplexYaml)
    ));

    // Raiz não é um único mapping.
    assert!(matches!(
        ckp::parse(&path, "- item1\n- item2\n"),
        Err(CkpError::Mapping)
    ));

    // Autorun inválido.
    assert!(matches!(
        ckp::parse(&path, "autorun: always\npanes:\n  - name: Term\n"),
        Err(CkpError::Autorun)
    ));

    // Panes ausente ou vazio.
    assert!(matches!(
        ckp::parse(&path, "autorun: worktree\n"),
        Err(CkpError::Panes)
    ));
    assert!(matches!(
        ckp::parse(&path, "panes: []\n"),
        Err(CkpError::Panes)
    ));

    // Item de `panes` não é mapping, ou campos inválidos.
    assert!(matches!(
        ckp::parse(&path, "panes:\n  - just_a_string\n"),
        Err(CkpError::Pane {
            index: 0,
            field: "mapping"
        })
    ));
    assert!(matches!(
        ckp::parse(&path, "panes:\n  - name: \"   \"\n"),
        Err(CkpError::Pane {
            index: 0,
            field: "name"
        })
    ));
    assert!(matches!(
        ckp::parse(&path, "panes:\n  - name: Server\n  - name: SERVER\n"),
        Err(CkpError::Duplicate(1))
    ));
    for bad_cwd in ["/abs/path", "sub\\dir", "D:project"] {
        let yaml = format!("panes:\n  - name: Term\n    cwd: '{bad_cwd}'\n");
        assert!(matches!(
            ckp::parse(&path, &yaml),
            Err(CkpError::Pane {
                index: 0,
                field: "cwd"
            })
        ));
    }
    assert!(matches!(
        ckp::parse(&path, "panes:\n  - name: Term\n    split: diagonal\n"),
        Err(CkpError::Pane {
            index: 0,
            field: "split"
        })
    ));
    assert!(matches!(
        ckp::parse(&path, "panes:\n  - name: Term\n    command: [1, 2]\n"),
        Err(CkpError::Pane {
            index: 0,
            field: "command"
        })
    ));
}
