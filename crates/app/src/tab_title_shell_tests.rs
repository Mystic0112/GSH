//! Título da aba com reticências, tooltip com o título completo e ícone SVG de arquivo,
//! dirigidos pelo [`Shell`] sem janela.

use iced::Size;
use iced_test::selector::Candidate;
use iced_test::simulator::Simulator;

use crate::pane_model::PaneTab;
use crate::tests::test_shell;
use crate::{labels, theme, FilesMsg, Msg, Shell};

const LONG: &str = "agente-com-um-nome-extremamente-longo-que-nao-cabe-na-aba";

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

/// Texto desenhado que termina em reticências e começa com `prefix`.
fn ellipsized(ui: &mut Simulator<'_, Msg>, prefix: &'static str) -> Option<String> {
    ui.find(move |candidate: Candidate<'_>| match candidate {
        Candidate::Text { content, .. }
            if content.starts_with(prefix) && content.ends_with('…') =>
        {
            Some(content.to_owned())
        }
        _ => None,
    })
    .ok()
}

#[test]
fn titulo_longo_e_cortado_com_reticencias_e_o_inteiro_nao_aparece() {
    let mut shell = test_shell();
    shell.apply_terminal_label(1, Some(LONG.into()));
    let mut ui = simulator(&shell);
    let cut = ellipsized(&mut ui, "agente-").expect("título cortado com …");
    assert!(cut.chars().count() < LONG.chars().count(), "{cut}");
    assert!(LONG.starts_with(cut.trim_end_matches('…')));
    assert!(
        ui.find(LONG).is_err(),
        "o título inteiro não é desenhado na aba"
    );
}

#[test]
fn titulo_curto_fica_inteiro_sem_reticencias() {
    let mut shell = test_shell();
    shell.apply_terminal_label(1, Some("api".into()));
    let mut ui = simulator(&shell);
    assert!(ui.find("api").is_ok());
    assert!(ellipsized(&mut ui, "api").is_none());
}

fn hover_title<'a>(shell: &'a Shell, text: &'static str) -> Simulator<'a, Msg> {
    let mut ui = simulator(shell);
    let bounds = ui
        .find(move |candidate: Candidate<'_>| match candidate {
            Candidate::Text {
                content, bounds, ..
            } if content.starts_with(text) => Some(bounds),
            _ => None,
        })
        .expect("título da aba");
    let at = bounds.center();
    ui.point_at(at);
    let _ = ui.simulate(vec![iced::Event::Mouse(iced::mouse::Event::CursorMoved {
        position: at,
    })]);
    // o balão abre no quadro seguinte (`RedrawRequested`), como no loop da janela
    let _ = ui.simulate(vec![iced::Event::Window(
        iced::window::Event::RedrawRequested(std::time::Instant::now()),
    )]);
    ui
}

/// `true` se o quadro com o mouse sobre o título difere do quadro sem o mouse: o balão é uma
/// camada de overlay que o seletor de widgets não enxerga, então compara-se o desenho.
fn hover_changes_the_frame(shell: &Shell, tag: &str) -> bool {
    let path = std::env::temp_dir().join(format!("gsh-tab-tip-{}-{tag}.png", std::process::id()));
    let generated = path.with_file_name(format!(
        "gsh-tab-tip-{}-{tag}-tiny-skia.png",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&generated);
    let theme = iced::Theme::Dark;
    let mut hovered = hover_title(shell, "agente-");
    hovered
        .snapshot(&theme)
        .expect("quadro com o mouse")
        .matches_image(&path)
        .expect("grava o quadro de referência");
    let mut idle = simulator(shell);
    let same = idle
        .snapshot(&theme)
        .expect("quadro sem o mouse")
        .matches_image(&path)
        .expect("compara com o quadro de referência");
    let _ = std::fs::remove_file(generated);
    !same
}

#[test]
fn pairar_no_titulo_desenha_o_balao_com_o_titulo_completo() {
    let mut shell = test_shell();
    shell.apply_terminal_label(1, Some(LONG.into()));
    assert!(hover_changes_the_frame(&shell, "balao"));
}

#[test]
fn balao_nao_vaza_por_cima_de_um_dialogo() {
    let mut shell = test_shell();
    shell.apply_terminal_label(1, Some(LONG.into()));
    let _ = shell.update(Msg::NewTerminalTab);
    let pane = shell.panes.focused();
    let _ = shell.update(Msg::TabMenuAction(
        pane,
        0,
        crate::tab_menu::TabAction::CloseOthers,
    ));
    assert!(shell.pending_close_tabs.is_some());
    assert!(!hover_changes_the_frame(&shell, "dialogo"));
}

#[test]
fn aba_de_arquivo_usa_o_svg_do_tema_e_nao_o_glifo() {
    let dir = std::env::temp_dir().join(format!("gsh-tab-icon-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("main.rs");
    std::fs::write(&file, "fn main() {}\n").unwrap();
    let mut shell = test_shell();
    let _ = shell.update(Msg::Files(FilesMsg::SelectFile(file.clone())));
    assert!(shell
        .panes
        .focused_model()
        .tabs()
        .contains(&PaneTab::File(file.clone())));
    let glyph = labels::file_icon_for_name("main.rs");
    let mut ui = simulator(&shell);
    assert!(ui.find("main.rs").is_ok(), "o título da aba");
    assert!(
        ui.find(glyph).is_err(),
        "o glifo mono-bold deu lugar ao SVG"
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
#[ignore = "gera o snapshot de títulos longos, ícone de arquivo e tooltip para revisão manual"]
fn capture_tab_title_snapshot() {
    use std::path::Path;
    theme::set_active(theme::ThemeVariant::Dark);
    let dir = std::env::temp_dir().join(format!("gsh-tab-title-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut shell = test_shell();
    shell.apply_terminal_label(1, Some(LONG.into()));
    for name in ["main.rs", "package.json", "README.md", "Dockerfile"] {
        let file = dir.join(name);
        std::fs::write(&file, "x\n").unwrap();
        let _ = shell.update(Msg::Files(FilesMsg::SelectFile(file)));
    }
    let _ = shell.update(Msg::SelectTab(0));
    let mut ui = hover_title(&shell, "agente-");
    let directory = Path::new("/home/heliodev/projetos/GSH-wt/shots");
    let target = directory.join("gsh-tabs-title.png");
    let generated = directory.join("gsh-tabs-title-tiny-skia.png");
    ui.snapshot(&iced::Theme::Dark)
        .expect("snapshot render")
        .matches_image(&target)
        .expect("snapshot write");
    std::fs::rename(generated, target).expect("rename snapshot to requested name");
    let _ = std::fs::remove_dir_all(dir);
}
