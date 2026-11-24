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
    simulator
        .snapshot(&iced::Theme::Dark)
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
