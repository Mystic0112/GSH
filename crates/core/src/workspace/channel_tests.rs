use super::*;

#[test]
fn canais_predefinidos_existem_sem_workspaces_e_nome_invalido_nao_move_item() {
    let mut list = WorkspaceList::new(vec![Workspace::system("sys", "Terminal", None)]);
    assert!(DEFAULT_CONTEXT_CHANNELS
        .iter()
        .all(|name| list.channels().contains(&name.to_string())));
    assert!(!list.set_workspace_realm(0, " \n "));
    assert!(!list.set_workspace_realm(0, "Nome\nInválido"));
    assert_eq!(
        ContextChannel::parse("Nome\nInválido"),
        ContextChannel::Default
    );
    assert_eq!(list.active().unwrap().realm(), CHANNEL_DEFAULT);
}

#[test]
fn workspace_from_path_extrai_nome_e_inicial() {
    let ws = Workspace::from_path("ws-1", "/home/dev/projetos/meu-app");
    assert_eq!(ws.name, "meu-app");
    assert_eq!(ws.initial(), "M");
    assert_eq!(ws.path, Some(PathBuf::from("/home/dev/projetos/meu-app")));
    assert_eq!(ws.kind, WorkspaceKind::Project);
    assert_eq!(ws.realm, CHANNEL_DEFAULT);
}

#[test]
fn workspace_list_abre_seleciona_e_evita_duplicatas_de_caminho() {
    let initial = vec![Workspace::system("gsh", "GSH", Some("main".into()))];
    let mut list = WorkspaceList::new(initial);
    assert_eq!(list.selected_index(), 0);
    assert_eq!(list.active().map(|w| w.name.as_str()), Some("GSH"));

    let idx = list.open_folder("/tmp/projeto-alfa");
    assert_eq!(idx, 1);
    assert_eq!(list.selected_index(), 1);
    assert_eq!(list.active().map(|w| w.name.as_str()), Some("projeto-alfa"));

    let _ = list.select(0);
    list.set_active_channel(Some(CHANNEL_WORK.to_owned()));
    let reused_idx = list.open_folder("/tmp/projeto-alfa");
    assert_eq!(reused_idx, 1);
    assert_eq!(list.items().len(), 2);
    assert_eq!(list.active_channel(), Some(CHANNEL_DEFAULT));

    assert!(list.select(99).is_none());
    assert_eq!(list.selected_index(), 1);
}

#[test]
fn canais_por_contexto_filtram_sem_clonar_nem_perder_selecao_ou_workspaces() {
    let ws_padrao = Workspace::system("sys", "Terminal", None);
    let ws_trabalho_1 = Workspace::from_path("ws-1", "/work/erp").with_realm(CHANNEL_WORK);
    let ws_pessoal = Workspace::from_path("ws-2", "/home/eu/blog").with_realm(CHANNEL_PERSONAL);
    let ws_projetos = Workspace::from_path("ws-3", "/home/eu/oss/gsh").with_realm(CHANNEL_PROJECTS);
    let ws_trabalho_2 = Workspace::from_path("ws-4", "/work/billing").with_realm("trabalho");

    let mut list = WorkspaceList::new(vec![
        ws_padrao,
        ws_trabalho_1,
        ws_pessoal,
        ws_projetos,
        ws_trabalho_2,
    ]);

    let _ = list.select(2);
    assert_eq!(list.selected_index(), 2);
    assert_eq!(list.active_channel(), None);
    assert_eq!(list.visible_indices(), vec![0, 1, 2, 3, 4]);
    assert_eq!(
        list.channels(),
        vec![
            CHANNEL_DEFAULT.to_string(),
            CHANNEL_PERSONAL.to_string(),
            CHANNEL_PROJECTS.to_string(),
            CHANNEL_WORK.to_string(),
        ]
    );

    list.set_active_channel(Some("trabalho".to_string()));
    assert_eq!(list.active_channel(), Some(CHANNEL_WORK));
    assert_eq!(list.visible_indices(), vec![1, 4]);
    assert_eq!(list.items().len(), 5);
    assert_eq!(list.selected_index(), 1);

    list.set_active_channel(Some(CHANNEL_PERSONAL.to_string()));
    assert_eq!(list.visible_indices(), vec![2]);

    list.set_active_channel(Some(CHANNEL_PROJECTS.to_string()));
    assert_eq!(list.visible_indices(), vec![3]);

    assert!(list.set_workspace_realm(0, CHANNEL_PROJECTS));
    assert!(!list.set_workspace_realm(99, CHANNEL_PROJECTS));
    assert_eq!(list.visible_indices(), vec![0, 3]);
    assert_eq!(list.items().len(), 5);

    list.set_active_channel(Some("Projeto-X".to_string()));
    assert!(list.visible_indices().is_empty());
    assert!(list.channels().contains(&"Projeto-X".to_string()));
    assert!(list.set_workspace_realm(0, "projeto-x"));
    assert_eq!(list.visible_indices(), vec![0]);
    let opened = list.open_folder("/tmp/projeto-x");
    assert_eq!(list.visible_indices(), vec![0, opened]);
    assert_eq!(list.items()[opened].realm(), "Projeto-X");

    list.set_active_channel(None);
    assert_eq!(list.active_channel(), None);
    assert_eq!(list.visible_indices(), vec![0, 1, 2, 3, 4, opened]);
}

#[test]
fn channel_selection_restores_last_workspace_and_empty_channel_has_no_active_workspace() {
    let first = Workspace::from_path("a", "/tmp/team-a").with_realm(CHANNEL_WORK);
    let second = Workspace::from_path("b", "/tmp/team-b").with_realm(CHANNEL_WORK);
    let mut list = WorkspaceList::new(vec![first, second]);
    let work = list.items()[0].channel_id().unwrap().clone();
    let empty = list
        .create_channel(ChannelDraft {
            name: "Pessoal vazio".into(),
            folder: Some("/tmp/pessoal".into()),
            ..ChannelDraft::default()
        })
        .unwrap();
    list.select_context_channel(Some(&work)).unwrap();
    assert_eq!(list.active().unwrap().id, "a");
    list.select(1).unwrap();
    list.select_context_channel(Some(&empty)).unwrap();
    assert!(list.active().is_none());
    assert!(list.select(1).is_none());
    assert!(list.visible_indices().is_empty());
    assert_eq!(
        list.active_context_channel()
            .unwrap()
            .draft
            .folder
            .as_deref(),
        Some(Path::new("/tmp/pessoal"))
    );
    list.select_context_channel(Some(&work)).unwrap();
    assert_eq!(list.active().unwrap().id, "b");
}

#[test]
fn invalid_drafts_have_no_effect_and_archiving_keeps_member_bindings() {
    let workspace = Workspace::from_path("a", "/tmp/team");
    let member = ChannelMember::Session {
        workspace: workspace.reference(),
        session_id: "existing-session".into(),
        label: "Ferro".into(),
    };
    let mut list = WorkspaceList::new(vec![workspace]);
    let draft = ChannelDraft {
        name: " Canais externos ".into(),
        instructions: "Responda em Português".into(),
        members: vec![member.clone(), member.clone()],
        ..ChannelDraft::default()
    };
    let id = list.create_channel(draft.clone()).unwrap();
    assert_eq!(
        list.channel(&id).unwrap().draft.members,
        vec![member.clone()]
    );
    assert_eq!(list.channel(&id).unwrap().name(), "Canais externos");
    assert_eq!(
        list.create_channel(draft.clone()),
        Err(ChannelError::DuplicateName)
    );
    let conflicting_member = ChannelMember::Session {
        workspace: WorkspaceRef::Project("/tmp/team".into()),
        session_id: "different-session".into(),
        label: "ferro".into(),
    };
    let duplicate_label = ChannelDraft {
        members: vec![member.clone(), conflicting_member],
        ..draft.clone()
    };
    assert_eq!(
        list.update_channel(&id, duplicate_label),
        Err(ChannelError::DuplicateMemberLabel)
    );
    let mut invalid = draft.clone();
    invalid.folder = Some("relative/path".into());
    let before = list.clone();
    assert_eq!(
        list.update_channel(&id, invalid),
        Err(ChannelError::InvalidFolder)
    );
    assert_eq!(list, before);
    list.assign_workspace_channel(0, &id).unwrap();
    list.select_context_channel(Some(&id)).unwrap();
    list.set_channel_archived(&id, true).unwrap();
    assert_eq!(list.active_channel(), None);
    assert_eq!(
        list.select_context_channel(Some(&id)),
        Err(ChannelError::Archived)
    );
    assert_eq!(list.items()[0].channel_id(), Some(&id));
    assert_eq!(list.channel(&id).unwrap().draft.members, vec![member]);
    list.set_channel_archived(&id, false).unwrap();
    list.select_context_channel(Some(&id)).unwrap();
    assert_eq!(list.active().unwrap().id, "a");
}
