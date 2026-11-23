use crate::localsend_popup::{self, PopupActions, PopupTab};
use crate::settings::{LocalSendPeer, LocalSendState};
use crate::{pane_close_dialog, theme};
use iced::{Element, Settings as IcedSettings, Size};
use iced_test::simulator::Simulator;
use std::path::{Path, PathBuf};

fn actions() -> PopupActions<()> {
    PopupActions {
        close: (),
        receive: (),
        send: (),
        settings_receive: (),
        settings_network: (),
        pick_files: (),
        pick_folder: (),
        text_changed: |_| (),
        send_text: (),
        paste: Some(()),
        refresh: (),
        send_to: |_| (),
        choose_receive_dir: (),
        alias_changed: |_| (),
        always_on_changed: |_| (),
        limits_changed: |_, _| (),
        server_restart: (),
        server_stop: (),
        cancel_send: (),
    }
}

fn render_snapshot(state: &LocalSendState, tab: PopupTab, path: &Path) {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = IcedSettings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let popup = localsend_popup::view(state, tab, &state.draft_text, actions());
    let modal: Element<'_, ()> = pane_close_dialog::modal(popup, (), ());
    let mut simulator = Simulator::with_size(settings, Size::new(1280.0, 760.0), modal);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("snapshot directory");
    }
    let generated = path.with_file_name(format!(
        "{}-tiny-skia.png",
        path.file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("snapshot")
    ));
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(&generated);
    simulator
        .snapshot(&iced::Theme::Dark)
        .expect("popup render")
        .matches_image(path)
        .expect("popup snapshot write");
    std::fs::rename(generated, path).expect("snapshot rename");
}

fn sample_state() -> LocalSendState {
    LocalSendState {
        alias: "Hélio".to_owned(),
        always_on: true,
        receive_dir: PathBuf::from("/home/ana/Downloads/GSH"),
        max_file_mb: "2048".to_owned(),
        max_files: "100".to_owned(),
        server_status: Some("Ativo na porta 53317 · certificado 6A:21:9F:04".to_owned()),
        server_alias: Some("Hélio".to_owned()),
        peers: vec![
            LocalSendPeer {
                alias: "Notebook da sala".to_owned(),
                device_type: "Desktop".to_owned(),
                ip: "192.168.1.42".to_owned(),
            },
            LocalSendPeer {
                alias: "Pixel 9".to_owned(),
                device_type: "Celular".to_owned(),
                ip: "192.168.1.57".to_owned(),
            },
        ],
        selected_files: vec![PathBuf::from("/home/ana/Downloads/apresentacao.pdf")],
        transfer_status: Some(crate::labels::LOCALSEND_AWAITING_ACCEPTANCE.to_owned()),
        ..LocalSendState::default()
    }
}

#[test]
fn localsend_popup_receive_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    render_snapshot(
        &sample_state(),
        PopupTab::Receive,
        Path::new("/home/heliodev/projetos/GSH-wt/shots/gsh-localsend-popup-receber.png"),
    );
}

#[test]
fn localsend_popup_send_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    render_snapshot(
        &sample_state(),
        PopupTab::Send,
        Path::new("/home/heliodev/projetos/GSH-wt/shots/gsh-localsend-popup-enviar.png"),
    );
}

#[test]
fn localsend_popup_receive_settings_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    render_snapshot(
        &sample_state(),
        PopupTab::SettingsReceive,
        Path::new("/home/heliodev/projetos/GSH-wt/shots/gsh-localsend-popup-config-recepcao.png"),
    );
}

#[test]
fn localsend_popup_network_settings_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    render_snapshot(
        &sample_state(),
        PopupTab::SettingsNetwork,
        Path::new("/home/heliodev/projetos/GSH-wt/shots/gsh-localsend-popup-config-rede.png"),
    );
}

#[test]
fn localsend_popup_contains_no_process_spawn_path() {
    let process_constructor = concat!("Command", "::new");
    let popup = include_str!("localsend_popup.rs");
    let popup_settings = include_str!("localsend_popup/settings.rs");
    assert!(!popup.contains(process_constructor));
    assert!(!popup_settings.contains(process_constructor));
}

#[test]
fn localsend_folder_selection_is_recursive_and_respects_configured_limits() {
    let directory = tempfile::tempdir().expect("folder selection fixture");
    let nested = directory.path().join("nested");
    std::fs::create_dir_all(&nested).expect("nested fixture directory");
    std::fs::write(directory.path().join("one.txt"), b"one").expect("first fixture file");
    std::fs::write(nested.join("two.txt"), b"two").expect("second fixture file");

    let files = crate::collect_localsend_folder(Some(directory.path().to_path_buf()), 2, 3)
        .expect("collect recursive folder");
    assert_eq!(files.len(), 2);
    assert!(crate::collect_localsend_folder(Some(directory.path().to_path_buf()), 1, 3).is_err());
    assert!(crate::collect_localsend_folder(Some(directory.path().to_path_buf()), 2, 2).is_err());
}

#[test]
fn localsend_text_selection_creates_a_bounded_txt_file() {
    let files = crate::write_localsend_text("mensagem de teste".to_owned(), 128)
        .expect("create text payload");
    let path = files.first().expect("one text file");
    assert_eq!(
        path.extension().and_then(|value| value.to_str()),
        Some("txt")
    );
    assert_eq!(
        std::fs::read_to_string(path).expect("read text payload"),
        "mensagem de teste"
    );
    let _ = std::fs::remove_file(path);
    assert!(crate::write_localsend_text("mensagem longa".to_owned(), 2).is_err());
}
