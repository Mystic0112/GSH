use gsh_core::panes::{
    reorder_tabs, PaneLeaf, PaneModelError, PaneMove, PaneNode, PaneSplit, SplitDirection,
};
use serde_json::json;

fn reference_tree() -> PaneNode {
    PaneNode::Split(PaneSplit {
        id: "s1".into(),
        direction: SplitDirection::Horizontal,
        fraction: 0.6,
        a: Box::new(PaneNode::leaf("p1", vec!["t1".into(), "t2".into()], "t2")),
        b: Box::new(PaneNode::Split(PaneSplit {
            id: "s2".into(),
            direction: SplitDirection::Vertical,
            fraction: 0.4,
            a: Box::new(PaneNode::leaf("p2", vec!["t3".into()], "t3")),
            b: Box::new(PaneNode::leaf("p3", vec!["t4".into(), "t5".into()], "t4")),
        })),
    })
}

#[test]
fn matches_flutter_pane_node_golden_behavior() {
    let tree = reference_tree();
    let extra = gsh_core::panes::PaneLeaf {
        id: "p4".into(),
        tabs: vec!["t6".into()],
        active: "t6".into(),
    };
    let inserted = tree.split_leaf("p1", SplitDirection::Vertical, extra, Some("s3"), true);
    let simple = PaneNode::leaf("p", vec!["a".into(), "b".into()], "a");
    let other = gsh_core::panes::PaneLeaf {
        id: "q".into(),
        tabs: vec!["c".into()],
        active: "c".into(),
    };
    let default_split =
        simple.split_leaf("p", SplitDirection::Vertical, other.clone(), None, false);
    let before_split =
        simple.split_leaf("p", SplitDirection::Horizontal, other, Some("custom"), true);
    let updated = simple.update_leaf("p", |leaf| {
        let mut changed = leaf.clone();
        changed.active = "b".into();
        changed
    });
    let unknown_kind =
        PaneNode::from_json(&json!({"k":"future-kind","id":"p","tabs":["t"],"active":"t"}))
            .expect("Flutter treats non-split kinds as leaves");
    let neighbors = |leaf, movement| tree.neighbor_leaf(leaf, movement);
    let actual = json!({
        "tree": tree.to_json(),
        "leaves": tree.leaves().iter().map(|leaf| leaf.id.as_str()).collect::<Vec<_>>(),
        "splitBefore": inserted.to_json(),
        "removeLeaf": tree.remove_leaf("p1").to_json(),
        "reorderTabs": reorder_tabs(&["t1".into(), "t2".into(), "t3".into()], "t1", 3),
        "neighbors": {
            "p1Down": neighbors("p1", PaneMove::Down),
            "p2Right": neighbors("p2", PaneMove::Right),
            "p3Left": neighbors("p3", PaneMove::Left),
            "p2Left": neighbors("p2", PaneMove::Left),
            "p3Right": neighbors("p3", PaneMove::Right),
            "p2Up": neighbors("p2", PaneMove::Up),
            "p1Up": neighbors("p1", PaneMove::Up),
            "missingRight": neighbors("missing", PaneMove::Right),
        },
        "defaultSplit": default_split.to_json(),
        "beforeCustomSplit": before_split.to_json(),
        "updatedLeaf": updated.to_json(),
        "unknownKindFallback": unknown_kind.to_json(),
    });
    let flutter_golden: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/panes_flutter_golden.json"))
            .expect("captured Flutter golden is valid JSON");
    assert_eq!(actual, flutter_golden);
}

#[test]
fn matches_flutter_mutation_defaults_and_noops() {
    let leaf = PaneNode::leaf("p", vec!["a".into(), "b".into()], "a");
    let other = gsh_core::panes::PaneLeaf {
        id: "q".into(),
        tabs: vec!["c".into()],
        active: "c".into(),
    };
    let default_split = leaf.split_leaf("p", SplitDirection::Vertical, other.clone(), None, false);
    let PaneNode::Split(split) = &default_split else {
        panic!("split expected");
    };
    assert_eq!(split.id, "sp_p_SplitDir.vertical");
    assert_eq!(split.fraction, 0.5);
    assert_eq!(split.a.as_ref(), &leaf);
    assert_eq!(split.b.as_ref(), &PaneNode::Leaf(other.clone()));

    let before = leaf.split_leaf(
        "p",
        SplitDirection::Horizontal,
        other.clone(),
        Some("custom"),
        true,
    );
    let PaneNode::Split(before_split) = &before else {
        panic!("split expected");
    };
    assert_eq!(before_split.id, "custom");
    assert_eq!(before_split.a.as_ref(), &PaneNode::Leaf(other.clone()));
    assert_eq!(before_split.b.as_ref(), &leaf);
    assert_eq!(
        leaf.split_leaf(
            "missing",
            SplitDirection::Vertical,
            other.clone(),
            None,
            false
        ),
        leaf
    );

    assert_eq!(
        default_split
            .set_fraction("sp_p_SplitDir.vertical", 0.7)
            .find_split("sp_p_SplitDir.vertical")
            .map(|split| split.fraction),
        Some(0.7)
    );
    assert_eq!(default_split.set_fraction("missing", 0.7), default_split);
    let updated = leaf.update_leaf("p", |pane| {
        let mut changed = pane.clone();
        changed.active = "b".into();
        changed
    });
    assert_eq!(
        updated.find_leaf("p").map(|pane| pane.active.as_str()),
        Some("b")
    );
    assert_eq!(leaf.update_leaf("missing", Clone::clone), leaf);

    let tabs = ["a".to_owned(), "b".to_owned(), "c".to_owned()];
    assert_eq!(reorder_tabs(&tabs, "a", 0), ["a", "b", "c"]);
    assert_eq!(reorder_tabs(&tabs, "c", -1), ["c", "a", "b"]);
    assert_eq!(reorder_tabs(&tabs, "a", 2), ["b", "a", "c"]);
    assert_eq!(reorder_tabs(&tabs, "c", 0), ["c", "a", "b"]);
    assert_eq!(reorder_tabs(&tabs, "absent", 2), tabs);
}

#[test]
fn directional_navigation_respects_nested_split_boundaries() {
    let tree = reference_tree();
    assert_eq!(tree.neighbor_leaf("p2", PaneMove::Left), None);
    assert_eq!(tree.neighbor_leaf("p3", PaneMove::Right), None);
    assert_eq!(
        tree.neighbor_leaf("p2", PaneMove::Up).as_deref(),
        Some("p1")
    );
    assert_eq!(tree.neighbor_leaf("p1", PaneMove::Up), None);
    assert_eq!(tree.neighbor_leaf("missing", PaneMove::Right), None);
}

#[test]
fn json_parser_rejects_invalid_shapes() {
    assert!(PaneNode::from_json(&json!({"k":"leaf","id":"p","tabs":[1],"active":"t"})).is_err());
    assert!(PaneNode::from_json(
        &json!({"k":"split","id":"s","dir":"diagonal","frac":0.5,"a":{},"b":{}})
    )
    .is_err());
    assert_eq!(
        PaneNode::from_json(&json!({"k":"future-kind","id":"p","tabs":["t"],"active":"t"}))
            .unwrap(),
        PaneNode::leaf("p", vec!["t".into()], "t")
    );
    assert_eq!(
        PaneNode::from_json(&json!({"id":"p","tabs":["t"],"active":"t"})).unwrap(),
        PaneNode::leaf("p", vec!["t".into()], "t")
    );
}

#[test]
fn pane_node_json_round_trip_and_golden_transitions_match_flutter() {
    // 1. Começa em folha única como no Flutter e aplica splits, frações 0.1..=0.9 e colapsos.
    let root = PaneNode::leaf("pane_root", vec!["t1".into(), "t2".into()], "t1");
    assert_eq!(
        root.to_json(),
        json!({
            "k": "leaf",
            "id": "pane_root",
            "tabs": ["t1", "t2"],
            "active": "t1"
        })
    );

    // Split vertical padrão (before = false -> novo leaf em `b`, id = `sp_pane_root_SplitDir.vertical`).
    let after_v_split = root.split_leaf(
        "pane_root",
        SplitDirection::Vertical,
        PaneLeaf {
            id: "pane_right".into(),
            tabs: vec!["t3".into()],
            active: "t3".into(),
        },
        None,
        false,
    );
    let expected_v_split_json = json!({
        "k": "split",
        "id": "sp_pane_root_SplitDir.vertical",
        "dir": "vertical",
        "frac": 0.5,
        "a": {"k": "leaf", "id": "pane_root", "tabs": ["t1", "t2"], "active": "t1"},
        "b": {"k": "leaf", "id": "pane_right", "tabs": ["t3"], "active": "t3"}
    });
    assert_eq!(after_v_split.to_json(), expected_v_split_json);
    assert_eq!(
        PaneNode::from_json(&expected_v_split_json).unwrap(),
        after_v_split
    );

    // Split horizontal no filho direito com `before = true` e ID padrão do Dart.
    let after_h_split = after_v_split.split_leaf(
        "pane_right",
        SplitDirection::Horizontal,
        PaneLeaf {
            id: "pane_top_right".into(),
            tabs: vec!["t4".into(), "t5".into()],
            active: "t5".into(),
        },
        None,
        true,
    );

    // Ajusta ratios nos extremos 0.1 e 0.9 e verifica round-trip JSON idêntico ao Flutter.
    let resized = after_h_split
        .set_fraction("sp_pane_root_SplitDir.vertical", 0.1)
        .set_fraction("sp_pane_right_SplitDir.horizontal", 0.9);
    let expected_resized_json = json!({
        "k": "split",
        "id": "sp_pane_root_SplitDir.vertical",
        "dir": "vertical",
        "frac": 0.1,
        "a": {"k": "leaf", "id": "pane_root", "tabs": ["t1", "t2"], "active": "t1"},
        "b": {
            "k": "split",
            "id": "sp_pane_right_SplitDir.horizontal",
            "dir": "horizontal",
            "frac": 0.9,
            "a": {"k": "leaf", "id": "pane_top_right", "tabs": ["t4", "t5"], "active": "t5"},
            "b": {"k": "leaf", "id": "pane_right", "tabs": ["t3"], "active": "t3"}
        }
    });
    assert_eq!(resized.to_json(), expected_resized_json);
    let decoded = PaneNode::from_json(&expected_resized_json).unwrap();
    assert_eq!(decoded, resized);
    assert_eq!(decoded.to_json(), expected_resized_json);

    // Colapso do filho `a` do split interno (`pane_top_right`) promove `pane_right` para filho `b` do split raiz.
    let collapsed_inner = decoded.remove_leaf("pane_top_right");
    let expected_collapsed_inner = json!({
        "k": "split",
        "id": "sp_pane_root_SplitDir.vertical",
        "dir": "vertical",
        "frac": 0.1,
        "a": {"k": "leaf", "id": "pane_root", "tabs": ["t1", "t2"], "active": "t1"},
        "b": {"k": "leaf", "id": "pane_right", "tabs": ["t3"], "active": "t3"}
    });
    assert_eq!(collapsed_inner.to_json(), expected_collapsed_inner);

    // Colapso do filho `b` do split raiz (`pane_right`) promove `pane_root` de volta à raiz.
    let collapsed_to_single = collapsed_inner.remove_leaf("pane_right");
    assert_eq!(collapsed_to_single, root);
    // Remover uma folha quando a árvore inteira já é uma única folha é no-op em `PaneNode::remove_leaf`.
    assert_eq!(collapsed_to_single.remove_leaf("pane_root"), root);
}

#[test]
fn pane_node_from_json_reports_exact_flutter_schema_errors() {
    assert_eq!(
        PaneNode::from_json(&json!("not-an-object")),
        Err(PaneModelError::NotObject)
    );
    assert_eq!(
        PaneNode::from_json(&json!([1, 2, 3])),
        Err(PaneModelError::NotObject)
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "leaf", "tabs": ["t1"], "active": "t1"})),
        Err(PaneModelError::Missing("id"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "leaf", "id": 42, "tabs": ["t1"], "active": "t1"})),
        Err(PaneModelError::Invalid("id"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "leaf", "id": "p1", "active": "t1"})),
        Err(PaneModelError::Missing("tabs"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "leaf", "id": "p1", "tabs": "t1", "active": "t1"})),
        Err(PaneModelError::Invalid("tabs"))
    );
    assert_eq!(
        PaneNode::from_json(
            &json!({"k": "leaf", "id": "p1", "tabs": ["t1", null], "active": "t1"})
        ),
        Err(PaneModelError::Invalid("tabs"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "leaf", "id": "p1", "tabs": ["t1"]})),
        Err(PaneModelError::Missing("active"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "leaf", "id": "p1", "tabs": ["t1"], "active": false})),
        Err(PaneModelError::Invalid("active"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "split", "id": "s1", "frac": 0.5})),
        Err(PaneModelError::Missing("dir"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "split", "id": "s1", "dir": "diagonal", "frac": 0.5})),
        Err(PaneModelError::Invalid("dir"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "split", "id": "s1", "dir": "vertical"})),
        Err(PaneModelError::Missing("frac"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "split", "id": "s1", "dir": "vertical", "frac": "half"})),
        Err(PaneModelError::Invalid("frac"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({"k": "split", "id": "s1", "dir": "vertical", "frac": 0.5})),
        Err(PaneModelError::Missing("a"))
    );
    assert_eq!(
        PaneNode::from_json(&json!({
            "k": "split",
            "id": "s1",
            "dir": "vertical",
            "frac": 0.5,
            "a": {"k": "leaf", "id": "p1", "tabs": ["t1"], "active": "t1"}
        })),
        Err(PaneModelError::Missing("b"))
    );
}

#[test]
fn reorder_tabs_matches_flutter_tab_strip_slot_indexing_golden() {
    let tabs: Vec<String> = vec!["t1".into(), "t2".into(), "t3".into(), "t4".into()];

    // Mover a primeira aba ("t1", from = 0) para cada slot:
    // slots <= 1 mantêm "t1" no índice 0 (pois slot 1 é logo após "t1").
    assert_eq!(reorder_tabs(&tabs, "t1", -2), ["t1", "t2", "t3", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t1", 0), ["t1", "t2", "t3", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t1", 1), ["t1", "t2", "t3", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t1", 2), ["t2", "t1", "t3", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t1", 3), ["t2", "t3", "t1", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t1", 4), ["t2", "t3", "t4", "t1"]);
    assert_eq!(reorder_tabs(&tabs, "t1", 99), ["t2", "t3", "t4", "t1"]);

    // Mover uma aba do meio ("t3", from = 2) para antes e para depois:
    assert_eq!(reorder_tabs(&tabs, "t3", 0), ["t3", "t1", "t2", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t3", 1), ["t1", "t3", "t2", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t3", 2), ["t1", "t2", "t3", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t3", 3), ["t1", "t2", "t3", "t4"]);
    assert_eq!(reorder_tabs(&tabs, "t3", 4), ["t1", "t2", "t4", "t3"]);
}
