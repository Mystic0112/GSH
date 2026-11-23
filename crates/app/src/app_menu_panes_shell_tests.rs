//! Submenus "Focar Painel" e "Selecionar Aba" do menu Exibir e o foco direcional entre panes,
//! dirigidos pelo [`Shell`] sem janela.

use iced::keyboard::{key::Named, Key, Modifiers};
use iced::widget::pane_grid::Axis;
use iced::Size;
use iced_test::simulator::Simulator;

use crate::app_menu::Item;
use crate::tests::test_shell;
use crate::{labels, theme, Msg, Shell};

fn exibir(shell: &Shell) -> Vec<Item<Msg>> {
    let mut groups = shell.app_menu_groups();
    assert_eq!(groups.len(), 4, "GSH, Arquivo, Exibir e Janela");
    groups.remove(2).items
}

fn submenu_labels(items: &[Item<Msg>], name: &str) -> Vec<String> {
    items
        .iter()
        .find_map(|item| match item {
            Item::Submenu(submenu) if submenu.label == name => Some(
                submenu
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        Item::Entry(entry) => Some(entry.label.clone()),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

fn top_level_labels(items: &[Item<Msg>]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Entry(entry) => Some(entry.label.clone()),
            Item::Submenu(submenu) => Some(format!("▸ {}", submenu.label)),
            _ => None,
        })
        .collect()
}

/// Dois panes lado a lado, o da direita focado.
fn two_panes() -> (
    Shell,
    iced::widget::pane_grid::Pane,
    iced::widget::pane_grid::Pane,
) {
    let mut shell = test_shell();
    let left = shell.panes.focused();
    let _ = shell.update(Msg::SplitPane(left, Axis::Vertical));
    let right = shell.panes.focused();
    assert_ne!(left, right);
    (shell, left, right)
}

#[test]
fn exibir_tem_focar_painel_e_selecionar_aba_como_submenus() {
    let shell = test_shell();
    let items = exibir(&shell);
    let top = top_level_labels(&items);
    let split_down = top.iter().position(|l| l == "Dividir abaixo").unwrap();
    assert_eq!(
        top.get(split_down + 1).map(String::as_str),
        Some("▸ Focar Painel")
    );
    assert_eq!(
        top.get(split_down + 2).map(String::as_str),
        Some("▸ Selecionar Aba")
    );
    assert!(
        !top.iter().any(|l| l.contains("Selecionar aba")),
        "as abas saíram do nível de cima: {top:?}"
    );
}

#[test]
fn focar_painel_tem_os_quatro_itens_do_flutter() {
    let shell = test_shell();
    assert_eq!(
        submenu_labels(&exibir(&shell), "Focar Painel"),
        [
            "Esquerda  (⌘⌥←)",
            "Direita  (⌘⌥→)",
            "Acima  (⌘⌥↑)",
            "Abaixo  (⌘⌥↓)"
        ]
    );
}

#[test]
fn selecionar_aba_tem_aba_1_a_8_e_ultima_aba() {
    let shell = test_shell();
    assert_eq!(
        submenu_labels(&exibir(&shell), "Selecionar Aba"),
        [
            "Aba 1",
            "Aba 2",
            "Aba 3",
            "Aba 4",
            "Aba 5",
            "Aba 6",
            "Aba 7",
            "Aba 8",
            "Última Aba"
        ]
    );
}

#[test]
fn janela_fica_so_com_minimizar_e_zoom() {
    let shell = test_shell();
    let groups = shell.app_menu_groups();
    let janela = groups.into_iter().nth(3).unwrap().items;
    assert_eq!(
        top_level_labels(&janela),
        ["Minimizar janela", "Alternar tamanho da janela"]
    );
}

#[test]
fn atalhos_das_abas_continuam_com_ctrl_1_a_9() {
    let mut shell = test_shell();
    let _ = shell.update(Msg::NewTerminalTab);
    let _ = shell.update(Msg::NewTerminalTab);
    let _ = shell.update(Msg::AppMenuShortcut(
        Key::Character("1".into()),
        Modifiers::CTRL,
    ));
    assert_eq!(shell.panes.focused_model().active_index(), Some(0));
    let _ = shell.update(Msg::AppMenuShortcut(
        Key::Character("9".into()),
        Modifiers::CTRL,
    ));
    assert_eq!(shell.panes.focused_model().active_index(), Some(2));
}

#[test]
fn ctrl_alt_seta_foca_o_pane_vizinho_e_sem_vizinho_nao_faz_nada() {
    let (mut shell, left, right) = two_panes();
    let ctrl_alt = Modifiers::CTRL | Modifiers::ALT;
    let _ = shell.update(Msg::AppMenuShortcut(Key::Named(Named::ArrowLeft), ctrl_alt));
    assert_eq!(shell.panes.focused(), left);
    let _ = shell.update(Msg::AppMenuShortcut(Key::Named(Named::ArrowLeft), ctrl_alt));
    assert_eq!(shell.panes.focused(), left, "na borda não cicla");
    let _ = shell.update(Msg::AppMenuShortcut(Key::Named(Named::ArrowDown), ctrl_alt));
    assert_eq!(shell.panes.focused(), left, "nada abaixo");
    let _ = shell.update(Msg::AppMenuShortcut(
        Key::Named(Named::ArrowRight),
        ctrl_alt,
    ));
    assert_eq!(shell.panes.focused(), right);
}

#[test]
fn seta_so_com_ctrl_ou_so_com_alt_nao_move_o_foco() {
    let (mut shell, _left, right) = two_panes();
    for modifiers in [
        Modifiers::CTRL,
        Modifiers::ALT,
        Modifiers::CTRL | Modifiers::ALT | Modifiers::SHIFT,
    ] {
        let _ = shell.update(Msg::AppMenuShortcut(
            Key::Named(Named::ArrowLeft),
            modifiers,
        ));
        assert_eq!(shell.panes.focused(), right, "{modifiers:?}");
    }
}

#[test]
fn item_do_submenu_focar_painel_dispara_o_foco_do_vizinho() {
    let (mut shell, left, right) = two_panes();
    let _ = shell.update(Msg::ToggleAppMenu);
    let _ = shell.update(Msg::OpenAppSubmenu(labels::MENU_FOCUS_PANE));
    assert_eq!(shell.app_submenu, Some(labels::MENU_FOCUS_PANE));
    let mut ui = simulator(&shell);
    ui.click(labels::MENU_FOCUS_LEFT).expect("item Esquerda");
    let messages: Vec<_> = ui.into_messages().collect();
    let inner = messages
        .iter()
        .find_map(|m| match m {
            Msg::AppMenuAction(inner) => Some(inner.as_ref().clone()),
            _ => None,
        })
        .expect("ação do menu");
    assert!(
        matches!(&inner, Msg::FocusPane(p) if *p == left),
        "{inner:?}"
    );
    let _ = shell.update(Msg::AppMenuAction(Box::new(inner)));
    assert_eq!(shell.panes.focused(), left);
    assert!(!shell.app_menu_open && shell.app_submenu.is_none());
    let _ = right;
}

#[test]
fn passar_o_mouse_no_item_abre_o_submenu_e_fechar_o_menu_o_fecha() {
    let mut shell = test_shell();
    let _ = shell.update(Msg::ToggleAppMenu);
    let mut ui = simulator(&shell);
    let bounds = ui
        .find(labels::MENU_FOCUS_PANE)
        .expect("item do submenu")
        .bounds();
    ui.point_at(bounds.center());
    let _ = ui.simulate(vec![iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position: bounds.center(),
    })]);
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Msg::OpenAppSubmenu(name) if *name == labels::MENU_FOCUS_PANE)),
        "{messages:?}"
    );
    let _ = shell.update(Msg::OpenAppSubmenu(labels::MENU_FOCUS_PANE));
    let _ = shell.update(Msg::CloseAppMenu);
    assert!(shell.app_submenu.is_none());
}

#[test]
fn submenu_aberto_mostra_os_itens_e_fechado_nao() {
    let mut shell = test_shell();
    let _ = shell.update(Msg::ToggleAppMenu);
    assert!(simulator(&shell).find(labels::MENU_FOCUS_LEFT).is_err());
    let _ = shell.update(Msg::OpenAppSubmenu(labels::MENU_SELECT_TAB));
    let mut ui = simulator(&shell);
    assert!(ui.find("Aba 1").is_ok());
    assert!(ui.find("Última Aba").is_ok());
    assert!(ui.find(labels::MENU_FOCUS_LEFT).is_err());
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

#[test]
#[ignore = "gera o snapshot do menu Exibir com o submenu aberto para revisão manual"]
fn capture_exibir_submenu_snapshot() {
    use std::path::Path;
    theme::set_active(theme::ThemeVariant::Dark);
    let mut shell = test_shell();
    let _ = shell.update(Msg::ToggleAppMenu);
    let _ = shell.update(Msg::OpenAppSubmenu(labels::MENU_SELECT_TAB));
    let directory = Path::new("/home/heliodev/projetos/GSH-wt/shots");
    let target = directory.join("gsh-tabs-menu-exibir.png");
    let generated = directory.join("gsh-tabs-menu-exibir-tiny-skia.png");
    simulator(&shell)
        .snapshot(&iced::Theme::Dark)
        .expect("snapshot render")
        .matches_image(&target)
        .expect("snapshot write");
    std::fs::rename(generated, target).expect("rename snapshot to requested name");
}
