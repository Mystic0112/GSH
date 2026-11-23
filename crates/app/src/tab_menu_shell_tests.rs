//! Menu de contexto, fechar em lote e renomear da aba, dirigidos pelo [`Shell`] sem janela.

use std::time::{Duration, Instant};

use iced::Size;
use iced_test::simulator::Simulator;

use crate::pane_model::PaneTab;
use crate::tab_menu::TabAction;
use crate::tests::test_shell;
use crate::{labels, theme, Msg, Shell};

/// Shell com os terminais 1, 2 e 3 no pane focado, o 1 ativo.
fn shell_with_three_terminals() -> Shell {
    let mut shell = test_shell();
    let _ = shell.update(Msg::NewTerminalTab);
    let _ = shell.update(Msg::NewTerminalTab);
    let _ = shell.update(Msg::SelectTab(0));
    shell
}

fn open_tabs(shell: &Shell) -> Vec<PaneTab> {
    shell.panes.focused_model().tabs().to_vec()
}

fn terminal_ids(shell: &Shell) -> Vec<u64> {
    open_tabs(shell)
        .iter()
        .filter_map(|tab| match tab {
            PaneTab::Terminal(id) => Some(*id),
            _ => None,
        })
        .collect()
}

fn menu_labels(shell: &Shell, index: usize) -> Vec<&'static str> {
    let pane = shell.panes.focused();
    crate::tab_menu::menu_entries(pane, index, shell.panes.focused_model())
        .into_iter()
        .map(|entry| entry.label)
        .collect()
}

fn action(shell: &mut Shell, index: usize, action: TabAction) {
    let pane = shell.panes.focused();
    let _ = shell.update(Msg::TabMenuAction(pane, index, action));
}

#[test]
fn botao_direito_abre_o_menu_da_aba_e_clicar_fora_fecha() {
    let mut shell = shell_with_three_terminals();
    let pane = shell.panes.focused();
    let _ = shell.update(Msg::TabContextMenu(pane, 1));
    assert_eq!(shell.tab_menu, Some((pane, 1)));
    let _ = shell.update(Msg::CloseTabContextMenu);
    assert_eq!(shell.tab_menu, None);
}

#[test]
fn menu_do_terminal_tem_os_rotulos_e_a_ordem_do_flutter() {
    let shell = shell_with_three_terminals();
    assert_eq!(
        menu_labels(&shell, 1),
        [
            "Renomear",
            "Copiar Id",
            "Reiniciar",
            "Fechar",
            "Fechar as outras",
            "Fechar à direita",
            "Fechar todas",
        ]
    );
    // a última aba não tem nada à direita
    assert!(!menu_labels(&shell, 2).contains(&"Fechar à direita"));
}

#[test]
fn menu_da_unica_aba_so_oferece_o_que_faz_algo() {
    let shell = test_shell();
    assert_eq!(
        menu_labels(&shell, 0),
        ["Renomear", "Copiar Id", "Reiniciar", "Fechar"]
    );
}

#[test]
fn redefinir_titulo_aparece_so_depois_de_renomear() {
    let mut shell = shell_with_three_terminals();
    assert!(!menu_labels(&shell, 1).contains(&"Redefinir título"));
    shell.apply_terminal_label(2, Some("api".into()));
    assert_eq!(menu_labels(&shell, 1).get(1), Some(&"Redefinir título"));
}

#[test]
fn fechar_do_menu_fecha_so_aquela_aba() {
    let mut shell = shell_with_three_terminals();
    action(&mut shell, 1, TabAction::Close);
    assert_eq!(terminal_ids(&shell).len(), 2);
    assert!(shell.pending_close_tabs.is_none());
}

#[test]
fn fechar_as_outras_pergunta_com_a_contagem_e_so_fecha_ao_confirmar() {
    let mut shell = shell_with_three_terminals();
    let kept = open_tabs(&shell)[1].clone();
    action(&mut shell, 1, TabAction::CloseOthers);
    assert_eq!(
        shell.pending_close_tabs.as_ref().map(|p| p.tabs.len()),
        Some(2)
    );
    assert_eq!(open_tabs(&shell).len(), 3);

    let _ = shell.update(Msg::CancelCloseTabs);
    assert!(shell.pending_close_tabs.is_none());
    assert_eq!(open_tabs(&shell).len(), 3);

    action(&mut shell, 1, TabAction::CloseOthers);
    let _ = shell.update(Msg::ConfirmCloseTabs);
    assert_eq!(open_tabs(&shell), vec![kept]);
    assert_eq!(shell.panes.focused_model().active_index(), Some(0));
}

#[test]
fn fechar_a_direita_mantem_as_da_esquerda_e_a_propria() {
    let mut shell = shell_with_three_terminals();
    let before = open_tabs(&shell);
    action(&mut shell, 0, TabAction::CloseToTheRight);
    assert_eq!(
        shell.pending_close_tabs.as_ref().map(|p| p.tabs.len()),
        Some(2)
    );
    let _ = shell.update(Msg::ConfirmCloseTabs);
    assert_eq!(open_tabs(&shell), before[..1].to_vec());
    // as sessões das abas fechadas foram descartadas
    assert_eq!(shell.terminals.len(), 1);
}

#[test]
fn fechar_todas_do_unico_pane_deixa_um_terminal_novo() {
    let mut shell = shell_with_three_terminals();
    let old = terminal_ids(&shell);
    action(&mut shell, 0, TabAction::CloseAll);
    let _ = shell.update(Msg::ConfirmCloseTabs);
    let now = terminal_ids(&shell);
    assert_eq!(now.len(), 1);
    assert!(old.iter().all(|id| !now.contains(id)));
    assert_eq!(shell.terminals.len(), 1);
}

#[test]
fn aba_com_alteracao_nao_salva_fica_aberta_e_pergunta_no_fechar_em_lote() {
    use iced::widget::text_editor::{Action, Edit};
    let mut shell = shell_with_three_terminals();
    let path = std::env::temp_dir().join(format!("gsh-tab-batch-{}", std::process::id()));
    std::fs::write(&path, "original").unwrap();
    let _ = shell.update(Msg::Files(crate::FilesMsg::SelectFile(path.clone())));
    let _ = shell.update(Msg::Editor(
        path.clone(),
        crate::editor_pane::EditorMsg::Action(Action::Edit(Edit::Insert('x'))),
    ));
    let file_tab = PaneTab::File(path.clone());
    assert!(open_tabs(&shell).contains(&file_tab));

    action(&mut shell, 0, TabAction::CloseAll);
    assert_eq!(
        shell.pending_close_tabs.as_ref().map(|p| p.tabs.len()),
        Some(4)
    );
    let _ = shell.update(Msg::ConfirmCloseTabs);

    // os terminais saíram; o arquivo sujo continua, com o aviso do próprio editor
    assert_eq!(open_tabs(&shell), vec![file_tab]);
    let editor = shell.editors.get(&path).expect("editor segue aberto");
    assert!(editor.state.buffer.is_dirty());
    let _ = std::fs::remove_file(path);
}

#[test]
fn copiar_id_e_reiniciar_nao_mexem_nas_abas() {
    let mut shell = shell_with_three_terminals();
    let before = open_tabs(&shell);
    action(&mut shell, 1, TabAction::CopyId);
    assert_eq!(open_tabs(&shell), before);
    assert_eq!(shell.tab_menu, None);
}

#[test]
fn reiniciar_cria_processo_novo_na_mesma_aba_zerando_o_status() {
    use gsh_core::status::{AgentHarness, HookEvent, StatusUpdate, TurnStatus};
    let mut shell = shell_with_three_terminals();
    shell.apply_terminal_label(2, Some("api".into()));
    let tabs = open_tabs(&shell);
    assert!(shell.status_runtime.apply_update(
        StatusUpdate {
            pane_id: "t2".into(),
            status: TurnStatus::Working,
            event: Some(HookEvent::UserPromptSubmit),
            session_id: None,
            transcript_path: None,
            harness: Some(AgentHarness::Claude),
            files: Vec::new(),
        },
        crate::now_epoch_ms()
    ));
    assert_eq!(
        shell.status_runtime.pane_record("t2").map(|r| r.status()),
        Some(TurnStatus::Working)
    );

    action(&mut shell, 1, TabAction::Restart);

    assert_eq!(open_tabs(&shell), tabs, "mesma aba, mesma posição");
    assert_eq!(shell.panes.focused_model().terminal_label(2), Some("api"));
    assert_eq!(shell.terminals.get(&2).map(|t| t.pane_id()), Some("t2"));
    assert_eq!(
        shell.status_runtime.pane_record("t2").map(|r| r.status()),
        Some(TurnStatus::Idle)
    );
}

#[test]
fn duplo_clique_em_menos_de_300ms_renomeia_a_aba_de_terminal() {
    let mut shell = shell_with_three_terminals();
    let pane = shell.panes.focused();
    let _ = shell.update(Msg::SelectTabIn(pane, 1));
    assert!(shell.tab_rename.is_none());
    let _ = shell.update(Msg::SelectTabIn(pane, 1));
    let rename = shell.tab_rename.as_ref().expect("entrou em edição");
    assert_eq!((rename.pane, rename.term), (pane, 2));
    assert_eq!(rename.text, "Terminal");
    // o segundo clique foi consumido: não é "primeiro clique" de um novo duplo
    assert!(shell.last_tab_click.is_none());
}

#[test]
fn cliques_lentos_ou_em_abas_diferentes_so_selecionam() {
    let mut shell = shell_with_three_terminals();
    let pane = shell.panes.focused();
    let now = Instant::now();
    assert!(!shell.register_tab_click(pane, 1, now));
    assert!(!shell.register_tab_click(pane, 1, now + Duration::from_millis(300)));
    assert!(!shell.register_tab_click(pane, 2, now + Duration::from_millis(350)));
    assert!(shell.register_tab_click(pane, 2, now + Duration::from_millis(400)));
    // aba que não é terminal não renomeia
    let _ = shell.update(Msg::OpenSettings);
    let settings = shell.panes.focused_model().tabs().len() - 1;
    assert!(!shell.register_tab_click(pane, settings, now));
    assert!(!shell.register_tab_click(pane, settings, now));
}

#[test]
fn renomear_do_menu_semeia_com_o_nome_atual() {
    let mut shell = shell_with_three_terminals();
    shell.apply_terminal_label(3, Some("deploy".into()));
    action(&mut shell, 2, TabAction::Rename);
    assert_eq!(
        shell.tab_rename.as_ref().map(|r| (r.term, r.text.as_str())),
        Some((3, "deploy"))
    );
}

#[test]
fn enter_grava_o_nome_com_hifen_no_modelo_e_no_mapa_da_cli() {
    let mut shell = shell_with_three_terminals();
    action(&mut shell, 1, TabAction::Rename);
    // o campo troca o espaço por `-` ao digitar (`FilteringTextInputFormatter`)
    let _ = shell.update(Msg::TabRenameChanged("meu agente".into()));
    assert_eq!(
        shell.tab_rename.as_ref().map(|r| r.text.as_str()),
        Some("meu-agente")
    );
    let _ = shell.update(Msg::TabRenameSubmit);
    assert!(shell.tab_rename.is_none());
    assert_eq!(
        shell.panes.focused_model().terminal_label(2),
        Some("meu-agente")
    );
    assert_eq!(
        shell.terminal_labels.get(&2).map(String::as_str),
        Some("meu-agente")
    );
    assert_eq!(
        shell
            .panes
            .focused_model()
            .display_title(&PaneTab::Terminal(2)),
        "meu-agente"
    );
}

#[test]
fn esc_cancela_sem_mudar_o_nome() {
    let mut shell = shell_with_three_terminals();
    shell.apply_terminal_label(2, Some("antes".into()));
    action(&mut shell, 1, TabAction::Rename);
    let _ = shell.update(Msg::TabRenameChanged("depois".into()));
    let _ = shell.update(Msg::TabRenameCancel);
    assert!(shell.tab_rename.is_none());
    assert_eq!(shell.panes.focused_model().terminal_label(2), Some("antes"));
}

#[test]
fn nome_vazio_nao_muda_nada_como_no_flutter() {
    let mut shell = shell_with_three_terminals();
    shell.apply_terminal_label(2, Some("antes".into()));
    action(&mut shell, 1, TabAction::Rename);
    let _ = shell.update(Msg::TabRenameChanged(String::new()));
    let _ = shell.update(Msg::TabRenameSubmit);
    assert!(shell.tab_rename.is_none());
    assert_eq!(shell.panes.focused_model().terminal_label(2), Some("antes"));
}

#[test]
fn redefinir_titulo_volta_ao_automatico() {
    let mut shell = shell_with_three_terminals();
    shell.apply_terminal_label(2, Some("api".into()));
    action(&mut shell, 1, TabAction::ResetLabel);
    assert_eq!(shell.panes.focused_model().terminal_label(2), None);
    assert!(!shell.terminal_labels.contains_key(&2));
}

#[test]
fn clicar_em_outra_parte_confirma_a_edicao() {
    let mut shell = shell_with_three_terminals();
    action(&mut shell, 1, TabAction::Rename);
    let _ = shell.update(Msg::TabRenameChanged("novo".into()));
    let pane = shell.panes.focused();
    let _ = shell.update(Msg::FocusPane(pane));
    assert!(shell.tab_rename.is_none());
    assert_eq!(shell.panes.focused_model().terminal_label(2), Some("novo"));
}

#[test]
fn modal_de_renomear_ou_de_fechar_abas_bloqueia_o_teclado_do_terminal() {
    let mut shell = shell_with_three_terminals();
    assert!(!shell.tab_modal_open());
    action(&mut shell, 1, TabAction::Rename);
    assert!(shell.tab_modal_open());
    let _ = shell.update(Msg::TabRenameCancel);
    action(&mut shell, 1, TabAction::CloseOthers);
    assert!(shell.tab_modal_open());
}

fn simulator(shell: &Shell) -> Simulator<'_, Msg> {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    Simulator::with_size(settings, Size::new(1280.0, 720.0), shell.view())
}

fn mouse_release(button: iced::mouse::Button) -> Vec<iced::Event> {
    vec![
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(button)),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(button)),
    ]
}

/// Centro do título da aba `index` (todas se chamam "Terminal"; cada uma tem 188 px).
fn tab_center(ui: &mut Simulator<'_, Msg>, index: usize) -> iced::Point {
    let first = ui
        .find(labels::DEFAULT_TAB_TITLE)
        .expect("título da primeira aba")
        .bounds();
    iced::Point::new(
        first.center_x() + index as f32 * theme::TAB_WIDTH,
        first.center_y(),
    )
}

#[test]
fn botao_direito_na_aba_publica_a_mensagem_do_menu() {
    let shell = shell_with_three_terminals();
    let pane = shell.panes.focused();
    let mut ui = simulator(&shell);
    let point = tab_center(&mut ui, 1);
    ui.point_at(point);
    let _ = ui.simulate(mouse_release(iced::mouse::Button::Right));
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Msg::TabContextMenu(p, 1) if *p == pane)),
        "{messages:?}"
    );
}

#[test]
fn botao_do_meio_na_aba_pede_para_fechar_aquela_aba() {
    let shell = shell_with_three_terminals();
    let pane = shell.panes.focused();
    let mut ui = simulator(&shell);
    let point = tab_center(&mut ui, 2);
    ui.point_at(point);
    let _ = ui.simulate(mouse_release(iced::mouse::Button::Middle));
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Msg::CloseTabIn(p, 2) if *p == pane)),
        "{messages:?}"
    );
}

fn escape_event() -> iced::Event {
    use iced::keyboard::{self, key};
    iced::Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(key::Named::Escape),
        modified_key: keyboard::Key::Named(key::Named::Escape),
        physical_key: key::Physical::Code(key::Code::Escape),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::NONE,
        text: None,
        repeat: false,
    })
}

#[test]
fn esc_e_clique_fora_fecham_o_menu_da_aba() {
    let mut shell = shell_with_three_terminals();
    let pane = shell.panes.focused();
    let _ = shell.update(Msg::TabContextMenu(pane, 1));
    let mut ui = simulator(&shell);
    let _ = ui.simulate([escape_event()]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Msg::CloseTabContextMenu)),
        "{messages:?}"
    );

    let mut ui = simulator(&shell);
    ui.point_at(iced::Point::new(900.0, 400.0));
    let _ = ui.simulate([iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
        iced::mouse::Button::Left,
    ))]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Msg::CloseTabContextMenu)),
        "{messages:?}"
    );
}

#[test]
fn clicar_no_primeiro_item_do_menu_publica_renomear_daquela_aba() {
    let mut shell = shell_with_three_terminals();
    let pane = shell.panes.focused();
    let _ = shell.update(Msg::TabContextMenu(pane, 1));
    let mut ui = simulator(&shell);
    let tab = tab_center(&mut ui, 1);
    // o popup abre 4 px abaixo da barra de 40 px, com 4 px de moldura antes do primeiro item
    ui.point_at(iced::Point::new(tab.x, tab.y + 20.0 + 4.0 + 4.0 + 14.0));
    let _ = ui.simulate(mouse_release(iced::mouse::Button::Left));
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Msg::TabMenuAction(p, 1, TabAction::Rename) if *p == pane)),
        "{messages:?}"
    );
}

#[test]
fn edicao_desenha_o_campo_com_o_texto_digitado() {
    let mut shell = shell_with_three_terminals();
    action(&mut shell, 1, TabAction::Rename);
    let _ = shell.update(Msg::TabRenameChanged("agente-x".into()));
    let mut ui = simulator(&shell);
    assert!(
        ui.find("agente-x").is_ok(),
        "o campo mostra o texto digitado"
    );
}

#[test]
#[ignore = "gera snapshots do menu da aba e da edição inline para revisão manual"]
fn capture_tab_menu_snapshots() {
    use std::path::Path;
    theme::set_active(theme::ThemeVariant::Dark);
    let capture = |shell: &Shell, name: &str| {
        let mut ui = simulator(shell);
        let directory = Path::new("/home/heliodev/projetos/GSH-wt/shots");
        let target = directory.join(format!("{name}.png"));
        let generated = directory.join(format!("{name}-tiny-skia.png"));
        ui.snapshot(&iced::Theme::Dark)
            .expect("snapshot render")
            .matches_image(&target)
            .expect("snapshot write");
        std::fs::rename(generated, target).expect("rename snapshot to requested name");
    };
    let mut shell = shell_with_three_terminals();
    shell.apply_terminal_label(3, Some("deploy".into()));
    let pane = shell.panes.focused();
    let _ = shell.update(Msg::TabContextMenu(pane, 1));
    capture(&shell, "gsh-tabs-menu");

    let _ = shell.update(Msg::CloseTabContextMenu);
    action(&mut shell, 1, TabAction::Rename);
    let _ = shell.update(Msg::TabRenameChanged("meu agente".into()));
    capture(&shell, "gsh-tabs-rename");
}
