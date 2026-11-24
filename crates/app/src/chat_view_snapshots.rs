use std::path::Path;

use iced::{Element, Settings, Size};
use iced_test::simulator::Simulator;

use super::{view_transcript, TRANSCRIPT_MAX_WIDTH};
use crate::chat_pane::ChatPane;
use crate::theme;
use gsh_chat::ChatEvent;

fn capture(name: &str, view: Element<'_, super::ChatMsg>) {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    let mut simulator = Simulator::with_size(settings, Size::new(1024.0, 720.0), view);
    let target = Path::new("/home/heliodev/projetos/GSH-wt/shots").join(name);
    let generated = target.with_file_name(format!(
        "{}-tiny-skia.png",
        target
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("snapshot")
    ));
    std::fs::create_dir_all(target.parent().unwrap_or_else(|| Path::new("/tmp")))
        .expect("snapshot directory");
    let _ = std::fs::remove_file(&target);
    let _ = std::fs::remove_file(&generated);
    let iced_theme = match theme::active() {
        theme::ThemeVariant::Light => iced::Theme::Light,
        theme::ThemeVariant::Dark => iced::Theme::Dark,
    };
    simulator
        .snapshot(&iced_theme)
        .expect("snapshot render")
        .matches_image(&target)
        .expect("snapshot write");
    std::fs::rename(generated, target).expect("snapshot rename");
}

#[test]
fn captures_gsh_chat_edit_card_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    let mut pane = ChatPane::default();
    pane.apply_event(ChatEvent::ToolCall {
        id: "edit-1".to_owned(),
        name: "Edit".to_owned(),
        input: serde_json::json!({
            "file_path": "/workspace/src/main.rs",
            "old_string": "let count = 1;\n",
            "new_string": "let count = 2;\nlet ready = true;\n"
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    });
    pane.apply_event(ChatEvent::ToolResult {
        tool_use_id: "edit-1".to_owned(),
        is_error: false,
        text: String::new(),
    });
    capture(
        "gsh-chat-edit-card.png",
        view_transcript(&pane, TRANSCRIPT_MAX_WIDTH),
    );
}

#[test]
fn captures_gsh_chat_message_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    let mut pane = ChatPane::default();
    pane.apply_event(ChatEvent::UserText {
        text: "Como o auto-scroll deve se comportar?".to_owned(),
        at_ms: Some(1_795_500_000_000),
    });
    pane.apply_event(ChatEvent::Text {
        text: "Ele acompanha o fim do stream quando você já está perto do fim.".to_owned(),
        at_ms: Some(1_795_500_000_000),
    });
    capture(
        "gsh-chat-message.png",
        view_transcript(&pane, TRANSCRIPT_MAX_WIDTH),
    );
}

#[test]
fn captures_gsh_chat_markdown_light_snapshot() {
    theme::set_active(theme::ThemeVariant::Light);
    let mut pane = ChatPane::default();
    pane.apply_event(ChatEvent::Text {
        text: "# Título\n\nTexto com `código inline` e [link](https://example.com).\n\n- primeiro item\n- segundo item\n\n```rust\nfn resposta() -> bool {\n    true\n}\n```".to_owned(),
        at_ms: Some(1_795_500_000_000),
    });
    capture(
        "gsh-chat-markdown-claro.png",
        view_transcript(&pane, TRANSCRIPT_MAX_WIDTH),
    );
}

#[test]
fn captures_gsh_chat_attachments_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    let directory = tempfile::tempdir().expect("diretório temporário");
    let image_path = directory.path().join("imagem.png");
    let document_path = directory.path().join("notas.pdf");
    image::RgbaImage::from_pixel(600, 400, image::Rgba([40, 160, 210, 255]))
        .save(&image_path)
        .expect("imagem de teste");
    std::fs::write(&document_path, b"PDF de teste").expect("documento de teste");

    let mut pane = ChatPane::default().with_static_test_mode(true);
    let _ = pane.update(crate::chat_pane::ChatMsg::PickFiles(Some(vec![
        image_path.clone()
    ])));
    let _ = pane.update(crate::chat_pane::ChatMsg::InputChanged(
        "Confira esta imagem".to_owned(),
    ));
    let _ = pane.update(crate::chat_pane::ChatMsg::Send);
    let _ = pane.update(crate::chat_pane::ChatMsg::Cancel);
    let _ = pane.update(crate::chat_pane::ChatMsg::RemoveAttachment(0));
    let _ = pane.update(crate::chat_pane::ChatMsg::PickFiles(Some(vec![
        document_path,
    ])));
    assert!(matches!(
        pane.items().first(),
        Some(crate::chat_pane::ChatItem::User { attachments, .. })
            if attachments.len() == 1 && attachments[0].name == "imagem.png"
    ));
    capture(
        "gsh-chat-anexos.png",
        pane.view(TRANSCRIPT_MAX_WIDTH, false),
    );
}

#[test]
fn captures_gsh_chat_permission_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    let mut pane = ChatPane::default();
    pane.apply_event(ChatEvent::PermissionAsk {
        request_id: "permission-1".to_owned(),
        tool_name: "Bash".to_owned(),
        description: Some("cargo test -p gsh-app".to_owned()),
        always_rule: Some("Bash(cargo test *)".to_owned()),
        input: serde_json::json!({"command":"cargo test -p gsh-app"})
            .as_object()
            .cloned()
            .unwrap_or_default(),
    });
    capture(
        "gsh-chat-permissao.png",
        view_transcript(&pane, TRANSCRIPT_MAX_WIDTH),
    );
}

#[test]
fn captures_gsh_chat_plan_snapshot() {
    theme::set_active(theme::ThemeVariant::Dark);
    let mut pane = ChatPane::default();
    pane.apply_event(ChatEvent::PermissionAsk {
        request_id: "plan-1".to_owned(),
        tool_name: "ExitPlanMode".to_owned(),
        description: None,
        always_rule: None,
        input: serde_json::json!({
            "plan":"# Plano do agente\n\n1. Criar o módulo.\n2. Rodar os testes."
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    });
    capture(
        "gsh-chat-plano.png",
        view_transcript(&pane, TRANSCRIPT_MAX_WIDTH),
    );
}
