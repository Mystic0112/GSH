use super::*;
use serde_json::json;

#[test]
fn tab_strip_close_activation_for_previous_active_and_next_tabs_matches_flutter() {
    let (mut state, workspace, pane) = setup();
    let mut editor = state.edit(&workspace).unwrap();

    // O primeiro `open_tab` de Terminal substitui o placeholder inicial "New tab".
    let t1 = editor
        .open_tab(&pane, NewTab::new(TabKind::Terminal, "one"))
        .unwrap();
    let t2 = editor
        .open_tab(&pane, NewTab::new(TabKind::Terminal, "two"))
        .unwrap();
    let t3 = editor
        .open_tab(&pane, NewTab::new(TabKind::Terminal, "three"))
        .unwrap();
    let t4 = editor
        .open_tab(&pane, NewTab::new(TabKind::Terminal, "four"))
        .unwrap();
    let t5 = editor
        .open_tab(&pane, NewTab::new(TabKind::Terminal, "five"))
        .unwrap();

    // Seleciona `t3` (no meio da faixa: [t1, t2, t3, t4, t5]).
    editor.select_tab(&pane, &t3).unwrap();
    assert_eq!(
        editor.workspace().tree().find_leaf(&pane).unwrap().tabs,
        vec![t1.clone(), t2.clone(), t3.clone(), t4.clone(), t5.clone()]
    );
    assert_eq!(editor.workspace().active_tab_id(), Some(t3.as_str()));

    // 1. Fechar uma aba ANTERIOR à ativa (`t1`) preserva `t3` como ativa.
    let closed_prev = editor.close_tab(&pane, &t1).unwrap();
    assert_eq!(closed_prev.tab.id, t1);
    assert!(!closed_prev.pane_removed);
    assert!(closed_prev.placeholder.is_none());
    assert_eq!(
        editor.workspace().tree().find_leaf(&pane).unwrap().tabs,
        vec![t2.clone(), t3.clone(), t4.clone(), t5.clone()]
    );
    assert_eq!(editor.workspace().active_tab_id(), Some(t3.as_str()));

    // 2. Fechar uma aba POSTERIOR à ativa (`t5`) preserva `t3` como ativa.
    let closed_next = editor.close_tab(&pane, &t5).unwrap();
    assert_eq!(closed_next.tab.id, t5);
    assert!(!closed_next.pane_removed);
    assert!(closed_next.placeholder.is_none());
    assert_eq!(
        editor.workspace().tree().find_leaf(&pane).unwrap().tabs,
        vec![t2.clone(), t3.clone(), t4.clone()]
    );
    assert_eq!(editor.workspace().active_tab_id(), Some(t3.as_str()));

    // 3. Fechar a PRÓPRIA aba ativa no meio (`t3`, índice 1) ativa a vizinha anterior (`t2`, índice 0).
    let closed_active_mid = editor.close_tab(&pane, &t3).unwrap();
    assert_eq!(closed_active_mid.tab.id, t3);
    assert!(!closed_active_mid.pane_removed);
    assert!(closed_active_mid.placeholder.is_none());
    assert_eq!(
        editor.workspace().tree().find_leaf(&pane).unwrap().tabs,
        vec![t2.clone(), t4.clone()]
    );
    assert_eq!(editor.workspace().active_tab_id(), Some(t2.as_str()));

    // 4. Fechar a PRÓPRIA aba ativa quando ela é a primeira (`t2`, índice 0) ativa a nova primeira (`t4`).
    let closed_active_first = editor.close_tab(&pane, &t2).unwrap();
    assert_eq!(closed_active_first.tab.id, t2);
    assert!(!closed_active_first.pane_removed);
    assert!(closed_active_first.placeholder.is_none());
    assert_eq!(
        editor.workspace().tree().find_leaf(&pane).unwrap().tabs,
        vec![t4.clone()]
    );
    assert_eq!(editor.workspace().active_tab_id(), Some(t4.as_str()));

    // 5. Fechar a última aba do único pane recria o placeholder "New tab".
    let closed_last = editor.close_tab(&pane, &t4).unwrap();
    assert_eq!(closed_last.tab.id, t4);
    assert!(!closed_last.pane_removed);
    let placeholder_id = closed_last.placeholder.expect("deve criar New tab");
    assert_eq!(
        editor.workspace().active_tab_id(),
        Some(placeholder_id.as_str())
    );
    assert_eq!(
        editor.workspace().active_tab().unwrap().kind,
        TabKind::Empty
    );
    assert_eq!(
        editor.workspace().active_tab().unwrap().title,
        EMPTY_TAB_TITLE
    );
    assert_consistent(editor.workspace());
}

#[test]
fn nested_splits_collapse_on_last_tab_close_and_clamp_ratios_within_bounds() {
    let (mut state, workspace, pane_left) = setup();
    let mut editor = state.edit(&workspace).unwrap();
    let tab_left = editor
        .open_tab(&pane_left, NewTab::new(TabKind::Terminal, "left"))
        .unwrap();

    let right_split = editor
        .split_pane(
            &pane_left,
            SplitDirection::Vertical,
            NewTab::new(TabKind::Terminal, "top-right"),
        )
        .unwrap();
    let bottom_right_split = editor
        .split_pane(
            &right_split.pane,
            SplitDirection::Horizontal,
            NewTab::new(TabKind::File, "notes.md"),
        )
        .unwrap();

    // Extrai IDs dos splits na árvore JSON e verifica estrutura inicial idêntica ao Flutter.
    let initial_json = editor.workspace().tree().to_json();
    let root_split_id = initial_json["id"].as_str().unwrap().to_owned();
    let inner_split_id = initial_json["b"]["id"].as_str().unwrap().to_owned();
    assert_eq!(
        initial_json,
        json!({
            "k": "split",
            "id": root_split_id,
            "dir": "vertical",
            "frac": 0.5,
            "a": {
                "k": "leaf",
                "id": pane_left,
                "tabs": [tab_left],
                "active": tab_left
            },
            "b": {
                "k": "split",
                "id": inner_split_id,
                "dir": "horizontal",
                "frac": 0.5,
                "a": {
                    "k": "leaf",
                    "id": right_split.pane,
                    "tabs": [right_split.tab],
                    "active": right_split.tab
                },
                "b": {
                    "k": "leaf",
                    "id": bottom_right_split.pane,
                    "tabs": [bottom_right_split.tab],
                    "active": bottom_right_split.tab
                }
            }
        })
    );

    // Testa clamp de ratio: o WorkspaceEditor mantém a fração estritamente em [0.16, 0.84] (subconjunto de 0.1..=0.9).
    for (input, expected) in [
        (-1.0, 0.16),
        (0.0, 0.16),
        (0.1, 0.16),
        (0.16, 0.16),
        (0.25, 0.25),
        (0.5, 0.5),
        (0.75, 0.75),
        (0.84, 0.84),
        (0.9, 0.84),
        (1.0, 0.84),
    ] {
        editor.resize_split(&root_split_id, input).unwrap();
        let actual = editor
            .workspace()
            .tree()
            .find_split(&root_split_id)
            .unwrap()
            .fraction;
        assert_eq!(actual, expected);
        assert!((0.1..=0.9).contains(&actual));
    }

    editor.resize_split(&root_split_id, 0.35).unwrap();
    editor.resize_split_by(&inner_split_id, 0.9).unwrap();
    assert_eq!(
        editor
            .workspace()
            .tree()
            .find_split(&inner_split_id)
            .unwrap()
            .fraction,
        0.84
    );

    // Fechar a última aba de `right_split.pane` colapsa `inner_split_id` e promove `bottom_right_split.pane`.
    let closed_mid = editor
        .close_tab(&right_split.pane, &right_split.tab)
        .unwrap();
    assert!(closed_mid.pane_removed);
    assert!(closed_mid.placeholder.is_none());
    assert_eq!(
        editor.workspace().tree().to_json(),
        json!({
            "k": "split",
            "id": root_split_id,
            "dir": "vertical",
            "frac": 0.35,
            "a": {
                "k": "leaf",
                "id": pane_left,
                "tabs": [tab_left],
                "active": tab_left
            },
            "b": {
                "k": "leaf",
                "id": bottom_right_split.pane,
                "tabs": [bottom_right_split.tab],
                "active": bottom_right_split.tab
            }
        })
    );

    // Fechar a última aba de `bottom_right_split.pane` colapsa `root_split_id` restando apenas `pane_left`.
    let closed_right = editor
        .close_tab(&bottom_right_split.pane, &bottom_right_split.tab)
        .unwrap();
    assert!(closed_right.pane_removed);
    assert!(closed_right.placeholder.is_none());
    assert_eq!(editor.workspace().focused_pane_id(), pane_left);
    assert_eq!(
        editor.workspace().tree().to_json(),
        json!({
            "k": "leaf",
            "id": pane_left,
            "tabs": [tab_left],
            "active": tab_left
        })
    );
    assert_consistent(editor.workspace());
}

#[test]
fn tab_strip_move_tab_and_format_tab_list_golden_comparison() {
    let mut state = Workspaces::new();
    let ws_proj = state.create_workspace("App", "/workspace/gsh-app").unwrap();
    let pane_a = state.selected().unwrap().focused_pane_id().to_owned();

    let (t_term, t_nvim, t_file, pane_b, t_chat) = {
        let mut editor = state.edit(&ws_proj).unwrap();
        let t_term = editor
            .open_tab(
                &pane_a,
                NewTab::new(TabKind::Terminal, "zsh").with_label("Build"),
            )
            .unwrap();
        let t_nvim = editor
            .open_tab(&pane_a, NewTab::new(TabKind::Neovim, "init.lua"))
            .unwrap();
        let t_file = editor
            .open_tab(&pane_a, NewTab::new(TabKind::File, "Cargo.toml"))
            .unwrap();
        let split_b = editor
            .split_pane(
                &pane_a,
                SplitDirection::Vertical,
                NewTab::new(TabKind::Chat, "Claude"),
            )
            .unwrap();

        // Seleciona a aba do meio (`t_nvim`) em `pane_a` e move-a para o início de `pane_b`.
        editor.select_tab(&pane_a, &t_nvim).unwrap();
        editor
            .move_tab(&pane_a, &t_nvim, &split_b.pane, Some(0))
            .unwrap();

        // Como `t_nvim` (índice 1) era a ativa em `pane_a`, `pane_a` ativa a vizinha anterior (`t_term`, índice 0).
        assert_eq!(
            editor.workspace().tree().find_leaf(&pane_a).unwrap().tabs,
            vec![t_term.clone(), t_file.clone()]
        );
        assert_eq!(
            editor.workspace().tree().find_leaf(&pane_a).unwrap().active,
            t_term
        );
        assert_eq!(
            editor
                .workspace()
                .tree()
                .find_leaf(&split_b.pane)
                .unwrap()
                .tabs,
            vec![t_nvim.clone(), split_b.tab.clone()]
        );
        assert_eq!(
            editor
                .workspace()
                .tree()
                .find_leaf(&split_b.pane)
                .unwrap()
                .active,
            t_nvim
        );
        assert_eq!(editor.workspace().focused_pane_id(), split_b.pane);
        assert_consistent(editor.workspace());

        (t_term, t_nvim, t_file, split_b.pane, split_b.tab)
    };

    // Cria um segundo workspace sintético (sem path) com placeholder "New tab".
    let ws_sys = state.create_workspace("Scratch", "").unwrap();
    let empty_id = state.workspace(&ws_sys).unwrap().tabs()[0].id.clone();

    assert_eq!(
        format_tab_list(&state.tab_list()),
        vec![
            format!("  {t_term:<6} terminal  gsh-app        ⚲ Build"),
            format!("  {t_nvim:<6} terminal  gsh-app        init.lua"),
            format!("  {t_file:<6} file      gsh-app        Cargo.toml"),
            format!("  {t_chat:<6} chat      gsh-app        Claude"),
            format!("  {empty_id:<6} other     {ws_sys:<14} New tab"),
        ]
    );
    let _ = pane_b;

    // Golden estrito do `format_tab_list` para payloads completos do Flutter CLI (`gsh list-tabs`).
    let flutter_wire_payload = vec![
        json!({
            "id": "t1",
            "kind": TabKind::Terminal.cli_kind(),
            "title": "cargo watch",
            "label": "Server",
            "workspacePath": "/home/dev/gsh",
            "workspaceId": "ws-0",
            "working": true,
            "harness": "claudeCode"
        }),
        json!({
            "id": "v2",
            "kind": TabKind::Redis.cli_kind(),
            "title": "redis://127.0.0.1:6379",
            "workspacePath": "/home/dev/gsh/",
            "workspaceId": "ws-0",
            "working": false
        }),
        json!({
            "id": "a9",
            "kind": TabKind::Empty.cli_kind(),
            "title": EMPTY_TAB_TITLE,
            "workspaceId": "ws-system"
        }),
    ];
    assert_eq!(
        format_tab_list(&flutter_wire_payload),
        vec![
            "● t1     terminal  gsh            ⚲ Server  [claudeCode]",
            "  v2     redis     gsh            redis://127.0.0.1:6379",
            "  a9     other     ws-system      New tab",
        ]
    );
}
