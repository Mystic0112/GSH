//! Marca de agente na aba (spinner, bolinha amarela, "não visto" e vazada), dirigida pelo [`Shell`] sem janela.

use gsh_core::status::{AgentHarness, HookEvent, StatusUpdate, TurnStatus};
use iced::Size;
use iced_test::simulator::Simulator;

use crate::pane_model::PaneTab;
use crate::pane_tab::TabIndicator;
use crate::tests::test_shell;
use crate::{theme, Msg, Shell};

fn update(pane_id: &str, status: TurnStatus, event: HookEvent, session: bool) -> StatusUpdate {
    StatusUpdate {
        pane_id: pane_id.into(),
        status,
        event: Some(event),
        session_id: session.then(|| "sessao-1".to_owned()),
        transcript_path: None,
        harness: Some(AgentHarness::Claude),
        files: Vec::new(),
    }
}

fn indicator(shell: &Shell, term: u64) -> Option<TabIndicator> {
    shell.tab_indicator_for(&PaneTab::Terminal(term))
}

#[test]
fn terminal_sem_hook_nao_mostra_marca_nem_anima() {
    let shell = test_shell();
    assert_eq!(indicator(&shell, 1), None);
    assert!(!shell.any_tab_animating());
}

#[test]
fn turno_em_andamento_com_sessao_gira_em_verde_e_liga_a_animacao() {
    let mut shell = test_shell();
    let working = update("t1", TurnStatus::Working, HookEvent::UserPromptSubmit, true);
    assert!(shell.status_runtime.apply_update(working, 1_000));
    assert_eq!(
        indicator(&shell, 1),
        Some(TabIndicator::Spinner { agent: true })
    );
    assert!(shell.any_tab_animating());
    let before = shell.tab_anim_ms;
    let _ = shell.update(Msg::TabAnimTick);
    assert!(shell.tab_anim_ms > before);
}

#[test]
fn turno_sem_id_de_sessao_gira_em_accent() {
    let mut shell = test_shell();
    let working = update(
        "t1",
        TurnStatus::Working,
        HookEvent::UserPromptSubmit,
        false,
    );
    assert!(shell.status_runtime.apply_update(working, 1_000));
    assert_eq!(
        indicator(&shell, 1),
        Some(TabIndicator::Spinner { agent: false })
    );
}

#[test]
fn aguardando_permissao_mostra_a_bolinha_amarela_que_pisca() {
    let mut shell = test_shell();
    let waiting = update(
        "t1",
        TurnStatus::Waiting,
        HookEvent::PermissionRequest,
        true,
    );
    assert!(shell.status_runtime.apply_update(waiting, 1_000));
    assert_eq!(indicator(&shell, 1), Some(TabIndicator::Waiting));
    assert!(shell.any_tab_animating());
}

#[test]
fn terminou_e_nao_foi_visto_vira_bolinha_e_ao_visitar_a_aba_fica_vazada() {
    let mut shell = test_shell();
    let working = update("t1", TurnStatus::Working, HookEvent::UserPromptSubmit, true);
    assert!(shell.status_runtime.apply_update(working, 1_000));
    let stop = update("t1", TurnStatus::Idle, HookEvent::Stop, true);
    assert!(shell.status_runtime.apply_update(stop, 2_000));
    assert_eq!(indicator(&shell, 1), Some(TabIndicator::Unseen));
    // bolinha parada: a animação desliga
    assert!(!shell.any_tab_animating());

    let _ = shell.update(Msg::SelectTab(0));
    assert_eq!(indicator(&shell, 1), Some(TabIndicator::Seen));
    assert!(!shell.any_tab_animating());
}

#[test]
fn cada_aba_tem_a_propria_marca() {
    let mut shell = test_shell();
    let _ = shell.update(Msg::NewTerminalTab);
    let working = update("t1", TurnStatus::Working, HookEvent::UserPromptSubmit, true);
    assert!(shell.status_runtime.apply_update(working, 1_000));
    assert_eq!(
        indicator(&shell, 1),
        Some(TabIndicator::Spinner { agent: true })
    );
    assert_eq!(indicator(&shell, 2), None);
}

#[test]
fn a_barra_de_abas_desenha_com_as_marcas() {
    let mut shell = test_shell();
    let waiting = update(
        "t1",
        TurnStatus::Waiting,
        HookEvent::PermissionRequest,
        true,
    );
    assert!(shell.status_runtime.apply_update(waiting, 1_000));
    let _ = simulator(&shell);
}

fn simulator(shell: &Shell) -> Simulator<'_, Msg> {
    simulator_sized(shell, 1280.0)
}

fn simulator_sized(shell: &Shell, width: f32) -> Simulator<'_, Msg> {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    Simulator::with_size(settings, Size::new(width, 200.0), shell.view())
}

#[test]
#[ignore = "gera o snapshot dos estados de agente nas abas para revisão manual"]
fn capture_tab_status_snapshot() {
    use std::path::Path;
    theme::set_active(theme::ThemeVariant::Dark);
    let mut shell = test_shell();
    for _ in 0..5 {
        let _ = shell.update(Msg::NewTerminalTab);
    }
    // 1 trabalhando (agente), 2 trabalhando (sem id), 3 esperando, 4 não visto, 5 parado visto
    let apply = |shell: &mut Shell, id: &str, session: bool, events: &[(TurnStatus, HookEvent)]| {
        for (index, (status, event)) in events.iter().enumerate() {
            let at = 1_000 + u64::try_from(index).unwrap_or(0) * 100;
            let _ = shell
                .status_runtime
                .apply_update(update(id, *status, event.clone(), session), at);
        }
    };
    let working = [(TurnStatus::Working, HookEvent::UserPromptSubmit)];
    apply(&mut shell, "t1", true, &working);
    apply(&mut shell, "t2", false, &working);
    apply(
        &mut shell,
        "t3",
        true,
        &[(TurnStatus::Waiting, HookEvent::PermissionRequest)],
    );
    apply(
        &mut shell,
        "t4",
        true,
        &[
            (TurnStatus::Working, HookEvent::UserPromptSubmit),
            (TurnStatus::Idle, HookEvent::Stop),
        ],
    );
    apply(
        &mut shell,
        "t5",
        true,
        &[
            (TurnStatus::Working, HookEvent::UserPromptSubmit),
            (TurnStatus::Idle, HookEvent::Stop),
        ],
    );
    let _ = shell.status_runtime.mark_pane_seen("t5");
    // fase em que o arco do spinner está aberto e a bolinha amarela, acesa
    shell.tab_anim_ms = 500;
    let directory = Path::new("/home/heliodev/projetos/GSH-wt/shots");
    let target = directory.join("gsh-tabs-status.png");
    let generated = directory.join("gsh-tabs-status-tiny-skia.png");
    simulator_sized(&shell, 2100.0)
        .snapshot(&iced::Theme::Dark)
        .expect("snapshot render")
        .matches_image(&target)
        .expect("snapshot write");
    std::fs::rename(generated, target).expect("rename snapshot to requested name");
}
