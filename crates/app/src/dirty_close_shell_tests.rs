//! Diálogo de 3 botões ao fechar arquivo com alteração não salva (aba e janela), dirigido pelo
//! [`Shell`] sem janela.

use std::path::{Path, PathBuf};

use iced::widget::text_editor::{Action, Edit};
use iced::Size;
use iced_test::simulator::Simulator;

use crate::pane_model::PaneTab;
use crate::tests::test_shell;
use crate::{editor_pane, labels, theme, AfterCloseFile, CloseChoice, Msg, Shell};
use crate::{FilesMsg, PendingCloseFile};

/// Arquivo temporário que some no fim do teste.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str, content: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("gsh-dirty-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("diretório de teste");
        let path = dir.join("arquivo.txt");
        std::fs::write(&path, content).expect("arquivo de teste");
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Some(dir) = self.0.parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
}

fn open_dirty(shell: &mut Shell, path: &PathBuf) {
    let _ = shell.update(Msg::Files(FilesMsg::SelectFile(path.clone())));
    let _ = shell.update(Msg::Editor(
        path.clone(),
        editor_pane::EditorMsg::Action(Action::Edit(Edit::Insert('x'))),
    ));
    assert!(shell
        .editors
        .get(path)
        .is_some_and(|e| e.state.buffer.is_dirty()));
}

fn file_tab_index(shell: &Shell, path: &Path) -> usize {
    let tab = PaneTab::File(path.to_path_buf());
    shell
        .panes
        .focused_model()
        .tabs()
        .iter()
        .position(|open| *open == tab)
        .expect("aba do arquivo")
}

fn has_file_tab(shell: &Shell, path: &Path) -> bool {
    shell
        .panes
        .focused_model()
        .tabs()
        .contains(&PaneTab::File(path.to_path_buf()))
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
fn fechar_arquivo_sujo_abre_o_dialogo_em_vez_do_aviso_inline() {
    let file = Scratch::new("dialogo", "original\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &file.0);
    let index = file_tab_index(&shell, &file.0);
    let _ = shell.update(Msg::CloseTab(index));
    assert!(shell.pending_close_file.is_some());
    assert!(has_file_tab(&shell, &file.0), "a aba segue aberta");
    assert!(shell.tab_modal_open(), "o diálogo é modal");
    assert!(
        shell.editors[&file.0].state.notice.is_none(),
        "sem aviso inline no editor"
    );

    let mut ui = simulator(&shell);
    assert!(ui.find(labels::UNSAVED_CHANGES_TITLE).is_ok());
    assert!(ui
        .find("“arquivo.txt” tem alterações não salvas. Salvar antes de fechar?")
        .is_ok());
    assert!(ui.find(labels::UNSAVED_DONT_SAVE).is_ok());
    assert!(ui.find(labels::DIALOG_CANCEL).is_ok());
    assert!(ui.find(labels::UNSAVED_SAVE_AND_CLOSE).is_ok());
}

#[test]
fn clicar_em_salvar_e_fechar_publica_a_escolha() {
    let file = Scratch::new("clique", "original\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &file.0);
    let index = file_tab_index(&shell, &file.0);
    let _ = shell.update(Msg::CloseTab(index));
    let mut ui = simulator(&shell);
    ui.click(labels::UNSAVED_SAVE_AND_CLOSE).expect("botão");
    let messages: Vec<_> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Msg::CloseFile(CloseChoice::SaveAndClose))),
        "{messages:?}"
    );
}

#[test]
fn cancelar_mantem_a_aba_e_o_texto_editado() {
    let file = Scratch::new("cancelar", "original\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &file.0);
    let index = file_tab_index(&shell, &file.0);
    let _ = shell.update(Msg::CloseTab(index));
    let _ = shell.update(Msg::CloseFile(CloseChoice::Cancel));
    assert!(shell.pending_close_file.is_none());
    assert!(has_file_tab(&shell, &file.0));
    assert!(shell.editors[&file.0].state.buffer.is_dirty());
}

#[test]
fn nao_salvar_fecha_a_aba_e_preserva_o_arquivo_em_disco() {
    let file = Scratch::new("descartar", "original\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &file.0);
    let index = file_tab_index(&shell, &file.0);
    let _ = shell.update(Msg::CloseTab(index));
    let _ = shell.update(Msg::CloseFile(CloseChoice::DontSave));
    assert!(shell.pending_close_file.is_none());
    assert!(!has_file_tab(&shell, &file.0));
    assert!(!shell.editors.contains_key(&file.0));
    assert_eq!(std::fs::read_to_string(&file.0).unwrap(), "original\n");
}

#[test]
fn salvar_e_fechar_grava_o_arquivo_e_fecha_a_aba() {
    let file = Scratch::new("salvar", "original\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &file.0);
    let index = file_tab_index(&shell, &file.0);
    let _ = shell.update(Msg::CloseTab(index));
    let _ = shell.update(Msg::CloseFile(CloseChoice::SaveAndClose));
    assert!(!has_file_tab(&shell, &file.0));
    assert!(std::fs::read_to_string(&file.0).unwrap().contains('x'));
}

#[test]
fn salvar_com_erro_de_gravacao_mantem_a_aba_aberta() {
    let file = Scratch::new("erro", "original\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &file.0);
    let index = file_tab_index(&shell, &file.0);
    let _ = shell.update(Msg::CloseTab(index));
    // a pasta some: gravar falha e o texto editado não pode ser perdido
    std::fs::remove_dir_all(file.0.parent().unwrap()).unwrap();
    let _ = shell.update(Msg::CloseFile(CloseChoice::SaveAndClose));
    assert!(has_file_tab(&shell, &file.0));
    assert!(shell.editors[&file.0].state.buffer.is_dirty());
    assert!(shell.pending_close_file.is_none());
}

#[test]
fn arquivo_limpo_fecha_sem_perguntar() {
    let file = Scratch::new("limpo", "original\n");
    let mut shell = test_shell();
    let _ = shell.update(Msg::Files(FilesMsg::SelectFile(file.0.clone())));
    let index = file_tab_index(&shell, &file.0);
    let _ = shell.update(Msg::CloseTab(index));
    assert!(shell.pending_close_file.is_none());
    assert!(!has_file_tab(&shell, &file.0));
}

#[test]
fn fechar_janela_suja_usa_o_mesmo_dialogo_sem_a_faixa_crua() {
    let file = Scratch::new("janela", "original\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &file.0);
    let id = iced::window::Id::unique();
    let _ = shell.update(Msg::WindowCloseRequested(id));
    assert!(matches!(
        shell.pending_close_file,
        Some(PendingCloseFile {
            then: AfterCloseFile::Window { .. },
            ..
        })
    ));
    let mut ui = simulator(&shell);
    assert!(ui.find(labels::UNSAVED_CHANGES_TITLE).is_ok());
    assert!(ui.find("Descartar e fechar").is_err(), "a faixa crua saiu");
    drop(ui);

    // perder o foco da janela não cancela mais (é um diálogo modal, como no Flutter)
    let _ = shell.update(Msg::WindowUnfocused);
    assert!(shell.pending_close_file.is_some());

    let _ = shell.update(Msg::CloseFile(CloseChoice::Cancel));
    assert!(shell.pending_close_file.is_none());
    assert!(shell.editors[&file.0].state.buffer.is_dirty());
}

#[test]
fn fechar_janela_com_dois_arquivos_sujos_pergunta_um_de_cada_vez() {
    let first = Scratch::new("janela-a", "a\n");
    let second = Scratch::new("janela-b", "b\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &first.0);
    open_dirty(&mut shell, &second.0);
    let id = iced::window::Id::unique();
    let _ = shell.update(Msg::WindowCloseRequested(id));
    let asked_first = shell.pending_close_file.as_ref().map(|p| p.path.clone());
    let _ = shell.update(Msg::CloseFile(CloseChoice::DontSave));
    let asked_second = shell.pending_close_file.as_ref().map(|p| p.path.clone());
    assert!(asked_first.is_some() && asked_second.is_some());
    assert_ne!(asked_first, asked_second, "pergunta o outro arquivo");
    // "Cancelar" no segundo aborta o fechamento
    let _ = shell.update(Msg::CloseFile(CloseChoice::Cancel));
    assert!(shell.pending_close_file.is_none());
}

#[test]
fn janela_limpa_fecha_sem_dialogo() {
    let mut shell = test_shell();
    let id = iced::window::Id::unique();
    let _ = shell.update(Msg::WindowCloseRequested(id));
    assert!(shell.pending_close_file.is_none());
}

#[test]
#[ignore = "gera o snapshot do diálogo de alteração não salva para revisão manual"]
fn capture_dirty_close_dialog_snapshot() {
    use std::path::Path;
    theme::set_active(theme::ThemeVariant::Dark);
    let file = Scratch::new("snapshot", "fn main() {}\n");
    let mut shell = test_shell();
    open_dirty(&mut shell, &file.0);
    let index = file_tab_index(&shell, &file.0);
    let _ = shell.update(Msg::CloseTab(index));
    let directory = Path::new("/home/heliodev/projetos/GSH-wt/shots");
    let target = directory.join("gsh-tabs-dirty-dialog.png");
    let generated = directory.join("gsh-tabs-dirty-dialog-tiny-skia.png");
    simulator(&shell)
        .snapshot(&iced::Theme::Dark)
        .expect("snapshot render")
        .matches_image(&target)
        .expect("snapshot write");
    std::fs::rename(generated, target).expect("rename snapshot to requested name");
}
