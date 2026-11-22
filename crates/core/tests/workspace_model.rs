use gsh_core::panes::{PaneMove, PaneNode, SplitDirection};
use gsh_core::tabs::{format_tab_list, NewTab, TabKind, EMPTY_TAB_TITLE};
use gsh_core::workspace::{WorkspaceError, WorkspaceState, Workspaces};

fn setup() -> (Workspaces, String, String) {
    let mut state = Workspaces::new();
    let workspace = state.create_workspace("Projeto", "/tmp/projeto").unwrap();
    let pane = state.selected().unwrap().focused_pane_id().to_owned();
    (state, workspace, pane)
}

fn assert_consistent(state: &WorkspaceState) {
    let leaves = state.tree().leaves();
    assert!(leaves.iter().any(|leaf| leaf.id == state.focused_pane_id()));
    let mut tree_ids: Vec<_> = leaves
        .iter()
        .flat_map(|leaf| {
            assert!(leaf.tabs.contains(&leaf.active));
            leaf.tabs.iter().map(String::as_str)
        })
        .collect();
    let mut tab_ids: Vec<_> = state.tabs().iter().map(|tab| tab.id.as_str()).collect();
    tree_ids.sort_unstable();
    tab_ids.sort_unstable();
    assert_eq!(tree_ids, tab_ids);
    tree_ids.dedup();
    assert_eq!(tree_ids, tab_ids);
}

#[test]
fn manages_workspace_selection_order_and_counter() {
    let (mut state, first, _) = setup();
    assert_eq!(first, "ws0");
    let second = state.create_workspace(" ", "/tmp/outro/").unwrap();
    assert_eq!(second, "ws1");
    assert_eq!(state.selected().unwrap().metadata().name, "outro");
    assert_eq!(
        state.create_workspace("", ""),
        Err(WorkspaceError::EmptyName)
    );
    assert_eq!(
        state.create_workspace("Duplicado", "/tmp/projeto").unwrap(),
        first
    );
    assert_eq!(state.items().len(), 2);
    state.rename_workspace(&first, "  Raiz  ").unwrap();
    assert_eq!(state.selected().unwrap().metadata().name, "Raiz");
    state.reorder_workspace(&second, &first, true).unwrap();
    assert_eq!(state.items()[0].metadata().id, second);
    assert_eq!(state.selected_id(), Some(first.as_str()));
    assert_eq!(state.remove_workspace(&first).unwrap().metadata().id, first);
    assert_eq!(state.selected_id(), Some(second.as_str()));
    assert_eq!(state.create_workspace("Terminal", "").unwrap(), "ws2");
    state.select_workspace(&second).unwrap();
    assert_eq!(state.selected_id(), Some(second.as_str()));
}

#[test]
fn newly_created_and_reset_workspaces_show_new_tab_in_cli() {
    let (mut state, workspace, pane) = setup();
    let initial = state.selected().unwrap().active_tab().unwrap().id.clone();
    assert_eq!(
        format_tab_list(&state.tab_list()),
        ["  a0     other     projeto        New tab"]
    );

    let mut editor = state.edit(&workspace).unwrap();
    let closed = editor.close_tab(&pane, &initial).unwrap();
    assert_eq!(closed.tab.id, initial);
    assert!(!closed.pane_removed);
    assert_ne!(closed.placeholder.as_deref(), Some(initial.as_str()));
    assert_eq!(
        closed.placeholder.as_deref(),
        editor.workspace().active_tab_id()
    );
    assert_eq!(
        editor.workspace().active_tab().unwrap().title,
        EMPTY_TAB_TITLE
    );
    let remaining = editor.close_pane(&pane).unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(editor.workspace().focused_pane_id(), pane);
    assert_consistent(editor.workspace());
    assert!(format_tab_list(&state.tab_list())[0].ends_with("New tab"));
}

#[test]
fn applies_flutter_replacement_rules_and_keeps_manual_names() {
    let (mut state, workspace, pane) = setup();
    let mut editor = state.edit(&workspace).unwrap();
    let file = editor
        .open_tab(&pane, NewTab::new(TabKind::File, "README"))
        .unwrap();
    let empty = editor.new_empty_tab(&pane).unwrap();
    let another = editor
        .open_tab(&pane, NewTab::new(TabKind::File, "Guide"))
        .unwrap();
    assert!(editor.workspace().tab(&empty).is_some());
    editor.select_tab(&pane, &empty).unwrap();
    let terminal = editor
        .open_tab(
            &pane,
            NewTab::new(TabKind::Terminal, "shell")
                .with_label("  Worker  ")
                .with_working_directory("/tmp/projeto"),
        )
        .unwrap();
    assert!(editor.workspace().tab(&empty).is_none());
    let reopened = editor
        .replace_tab(&pane, &file, NewTab::new(TabKind::File, "Updated"))
        .unwrap();
    assert_eq!(
        editor.workspace().tree().find_leaf(&pane).unwrap().tabs,
        [reopened, terminal.clone(), another]
    );
    editor.set_title(&terminal, "shell changed").unwrap();
    let tab = editor.workspace().active_tab().unwrap();
    assert_eq!(tab.display_title(), "Worker");
    assert_eq!(tab.working_directory, "/tmp/projeto");
    assert!(tab.title_locked());
    editor.clear_label(&terminal).unwrap();
    editor.set_label(&terminal, "Runner").unwrap();
    assert_eq!(
        editor.workspace().active_tab().unwrap().display_title(),
        "Runner"
    );
    assert_consistent(editor.workspace());
}

#[test]
fn moves_tabs_across_splits_and_collapses_empty_source() {
    let (mut state, workspace, pane) = setup();
    let mut editor = state.edit(&workspace).unwrap();
    let terminal = editor
        .open_tab(&pane, NewTab::new(TabKind::Terminal, "shell"))
        .unwrap();
    let empty = editor.new_empty_tab(&pane).unwrap();
    let file = editor
        .fill_empty(&pane, &empty, NewTab::new(TabKind::File, "main.rs"))
        .unwrap();
    editor.reorder_tab(&pane, &file, 0).unwrap();
    let split = editor
        .split_pane(&pane, SplitDirection::Vertical, NewTab::empty())
        .unwrap();
    assert_eq!(editor.focus_toward(PaneMove::Left).unwrap(), pane);
    let new_pane = editor
        .move_tab_to_new_split(&pane, &file, &split.pane, SplitDirection::Horizontal, true)
        .unwrap();
    assert_eq!(editor.workspace().focused_pane_id(), new_pane);
    assert_eq!(
        editor.workspace().tree().find_leaf(&pane).unwrap().active,
        terminal
    );
    editor
        .move_tab(&pane, &terminal, &new_pane, Some(0))
        .unwrap();
    assert!(editor.workspace().tree().find_leaf(&pane).is_none());
    let closed = editor.close_tab(&new_pane, &terminal).unwrap();
    assert!(!closed.pane_removed);
    assert_eq!(editor.workspace().active_tab_id(), Some(file.as_str()));
    let closed = editor.close_tab(&new_pane, &file).unwrap();
    assert!(closed.pane_removed);
    assert_eq!(editor.workspace().focused_pane_id(), split.pane);
    assert_consistent(editor.workspace());
}

#[test]
fn failed_requests_leave_state_and_ids_intact() {
    let (mut state, workspace, pane) = setup();
    let initial = state.selected().unwrap().active_tab().unwrap().id.clone();
    let before = state.clone();
    assert_eq!(
        state.select_workspace("missing"),
        Err(WorkspaceError::WorkspaceNotFound("missing".into()))
    );
    assert_eq!(
        state.rename_workspace(&workspace, "  "),
        Err(WorkspaceError::EmptyName)
    );
    assert_eq!(
        state.reveal_tab("missing"),
        Err(WorkspaceError::TabNotFound("missing".into()))
    );
    let mut editor = state.edit(&workspace).unwrap();
    assert_eq!(
        editor.open_tab("missing", NewTab::empty()),
        Err(WorkspaceError::PaneNotFound("missing".into()))
    );
    assert_eq!(
        editor.move_tab_to_new_split(&pane, &initial, &pane, SplitDirection::Vertical, true),
        Err(WorkspaceError::SplitOntoSelf(pane.clone()))
    );
    assert_eq!(
        editor.focus_toward(PaneMove::Up),
        Err(WorkspaceError::NoNeighbor {
            pane: pane.clone(),
            direction: PaneMove::Up
        })
    );
    assert_eq!(
        editor.set_title(&initial, "Different"),
        Err(WorkspaceError::PlaceholderMetadata)
    );
    assert_eq!(
        editor.open_tab(
            &pane,
            NewTab::new(TabKind::Terminal, "shell").with_label(" ")
        ),
        Err(WorkspaceError::EmptyLabel)
    );
    assert_eq!(state, before);
}

#[test]
fn focuses_between_workspaces_and_checks_split_resizing() {
    let (mut state, first, pane) = setup();
    let other = state.create_workspace("Other", "").unwrap();
    let mut editor = state.edit(&first).unwrap();
    let created = editor
        .split_pane(&pane, SplitDirection::Vertical, NewTab::empty())
        .unwrap();
    let PaneNode::Split(split) = editor.workspace().tree() else {
        panic!("expected split");
    };
    let split_id = split.id.clone();
    editor.resize_split(&split_id, 2.0).unwrap();
    assert_eq!(
        editor
            .workspace()
            .tree()
            .find_split(&split_id)
            .unwrap()
            .fraction,
        0.84
    );
    editor.resize_split_by(&split_id, -2.0).unwrap();
    assert_eq!(
        editor
            .workspace()
            .tree()
            .find_split(&split_id)
            .unwrap()
            .fraction,
        0.16
    );
    assert_eq!(
        editor.resize_split(&split_id, f64::NAN),
        Err(WorkspaceError::InvalidFraction)
    );
    assert_eq!(
        editor.resize_split(&pane, 0.5),
        Err(WorkspaceError::SplitNotFound(pane.clone()))
    );
    assert_eq!(
        editor.select_tab(&pane, &created.tab),
        Err(WorkspaceError::TabNotInPane {
            tab: created.tab.clone(),
            pane
        })
    );
    assert_eq!(state.selected_id(), Some(other.as_str()));
    assert_eq!(
        state.reveal_tab(&created.tab).unwrap(),
        (first.clone(), created.pane)
    );
    assert_eq!(state.selected_id(), Some(first.as_str()));
    assert_eq!(
        state.selected().unwrap().active_tab_id(),
        Some(created.tab.as_str())
    );
    let list = state.tab_list();
    assert_eq!(
        list.iter().filter(|entry| entry["focused"] == true).count(),
        1
    );
    assert!(list
        .iter()
        .any(|entry| entry["id"] == created.tab && entry["focused"] == true));
}

#[path = "workspace_model/tab_strip_golden.rs"]
mod tab_strip_golden;
