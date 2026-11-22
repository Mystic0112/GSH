//! Interface da integração LocalSend nas configurações.

use super::{LocalSendMessage, LocalSendState, Message};
use crate::{labels, theme};
use iced::widget::{button, column, container, progress_bar, row, text, text_input};
use iced::{Alignment, Background, Border, Element, Fill, Theme};

pub(super) fn view(state: &LocalSendState) -> Element<'_, Message> {
    let mut content = column![
        text(labels::LOCALSEND_TITLE)
            .font(theme::FONT_TITLE)
            .size(theme::SIZE_TITLE)
            .color(theme::palette().text),
        setting_row(
            labels::LOCALSEND_ALWAYS_ON,
            super::view::switch(
                state.always_on,
                Message::LocalSend(LocalSendMessage::AlwaysOnChanged(!state.always_on)),
            ),
        ),
    ]
    .spacing(theme::SETTINGS_SECTION_TITLE_GAP)
    .width(Fill);

    {
        content = content
            .push(setting_row(
                labels::LOCALSEND_ALIAS,
                text_input(labels::LOCALSEND_ALIAS_HINT, &state.alias)
                    .on_input(|value| Message::LocalSend(LocalSendMessage::AliasChanged(value)))
                    .style(input_style)
                    .width(240)
                    .into(),
            ))
            .push(setting_row(
                labels::LOCALSEND_RECEIVE_DIR,
                row![
                    text(state.receive_dir.to_string_lossy().to_string())
                        .size(theme::SETTINGS_NAV_TEXT_SIZE)
                        .color(theme::palette().text2)
                        .width(Fill),
                    action_button(
                        labels::LOCALSEND_CHOOSE_DIR,
                        Message::LocalSend(LocalSendMessage::ChooseReceiveDir),
                    )
                ]
                .spacing(theme::SETTINGS_ROW_GAP)
                .align_y(Alignment::Center)
                .into(),
            ))
            .push(setting_row(
                labels::LOCALSEND_LIMITS,
                row![
                    text_input(labels::LOCALSEND_MAX_FILE_MB_HINT, &state.max_file_mb)
                        .on_input(|max_file_mb| Message::LocalSend(
                            LocalSendMessage::LimitsChanged {
                                max_file_mb,
                                max_files: String::new(),
                            }
                        ))
                        .style(input_style)
                        .width(100),
                    text(labels::LOCALSEND_MB),
                    text_input(labels::LOCALSEND_MAX_FILES_HINT, &state.max_files)
                        .on_input(
                            |max_files| Message::LocalSend(LocalSendMessage::LimitsChanged {
                                max_file_mb: String::new(),
                                max_files,
                            })
                        )
                        .style(input_style)
                        .width(90),
                    text(labels::LOCALSEND_FILES),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into(),
            ));

        if let Some(error) = &state.error {
            content = content.push(text(error).size(12).color(theme::palette().warn));
        }

        if let Some(transfer) = &state.transfer {
            content = content.push(
                column![
                    text(format!("{} · {}", transfer.peer_alias, transfer.file_name))
                        .size(theme::SETTINGS_NAV_TEXT_SIZE)
                        .color(theme::palette().text),
                    progress_bar(
                        0.0..=transfer.total_bytes.max(1) as f32,
                        transfer.bytes.min(transfer.total_bytes.max(1)) as f32,
                    ),
                    action_button(
                        labels::LOCALSEND_CANCEL,
                        Message::LocalSend(LocalSendMessage::CancelSend),
                    ),
                ]
                .spacing(8),
            );
        }
    }

    container(content)
        .max_width(theme::SETTINGS_PANEL_MAX_WIDTH)
        .width(Fill)
        .into()
}

fn setting_row<'a>(label: &'static str, control: Element<'a, Message>) -> Element<'a, Message> {
    column![
        row![
            text(label)
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_NAV_TEXT_SIZE)
                .color(theme::palette().text)
                .width(Fill),
            iced::widget::Space::new().width(theme::SETTINGS_ROW_GAP),
            control
        ]
        .width(Fill)
        .align_y(Alignment::Center)
        .padding([theme::SETTINGS_ROW_VERTICAL, 0]),
        crate::panel_widgets::hairline_h()
    ]
    .width(Fill)
    .into()
}

fn action_button(label: &'static str, message: Message) -> Element<'static, Message> {
    button(
        text(label)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::SETTINGS_NAV_TEXT_SIZE),
    )
    .padding([6, 12])
    .on_press(message)
    .style(|_: &Theme, status| button::Style {
        background: Some(
            if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                theme::palette().panel3
            } else {
                theme::palette().panel2
            }
            .into(),
        ),
        text_color: theme::palette().text,
        border: Border {
            color: theme::palette().border2,
            width: 1.0,
            radius: theme::RADIUS_BUTTON.into(),
        },
        ..Default::default()
    })
    .into()
}

fn input_style(theme_value: &Theme, status: text_input::Status) -> text_input::Style {
    let palette = theme::palette();
    let border = match status {
        text_input::Status::Focused { .. } => palette.accent,
        _ => palette.border2,
    };
    let _ = theme_value;
    text_input::Style {
        background: Background::Color(palette.panel2),
        border: Border {
            color: border,
            width: 1.0,
            radius: theme::RADIUS_BUTTON.into(),
        },
        icon: palette.text3,
        placeholder: palette.text4,
        value: palette.text,
        selection: palette.accent,
    }
}

#[cfg(test)]
mod snapshots {
    use super::*;
    use crate::settings::{LocalSendFile, LocalSendIncoming, LocalSendPeer, Section, Settings};
    use crate::theme::{self, ThemeVariant};
    use iced::{Settings as IcedSettings, Size};
    use iced_test::simulator::Simulator;
    use std::path::{Path, PathBuf};

    fn capture<'a, M: 'a>(element: impl Into<Element<'a, M>>, target: &Path, size: Size) {
        std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
        let settings = IcedSettings {
            fonts: theme::EMBEDDED_FONTS
                .iter()
                .map(|font| (*font).into())
                .collect(),
            default_font: theme::FONT_BODY,
            ..Default::default()
        };
        let mut simulator = Simulator::with_size(settings, size, element);
        let generated = target.with_file_name(format!(
            "{}-tiny-skia.png",
            target
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("snapshot")
        ));
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).expect("snapshot directory");
        }
        let _ = std::fs::remove_file(target);
        let _ = std::fs::remove_file(&generated);
        simulator
            .snapshot(&iced::Theme::Dark)
            .expect("snapshot render")
            .matches_image(target)
            .expect("snapshot write");
        std::fs::rename(generated, target).expect("snapshot rename");
    }

    #[test]
    #[ignore = "gera snapshots LocalSend para revisão visual"]
    fn capture_localsend_dialog_and_settings() {
        theme::set_active(ThemeVariant::Dark);
        let state = LocalSendState {
            receive_dir: PathBuf::from("/home/ana/Downloads/GSH"),
            incoming: Some(LocalSendIncoming {
                sender_alias: String::from("Notebook da sala"),
                sender_model: Some(String::from("ThinkPad")),
                sender_ip: String::from("192.168.1.42"),
                files: vec![
                    LocalSendFile {
                        name: String::from("apresentacao.pdf"),
                        size: 2_400_000,
                    },
                    LocalSendFile {
                        name: String::from("imagem.png"),
                        size: 850_000,
                    },
                ],
                total_bytes: 3_250_000,
            }),
            ..LocalSendState::default()
        };
        let dialog = crate::localsend_dialog::view(&state).expect("receive dialog visible");
        capture(
            dialog,
            Path::new("/home/heliodev/projetos/GSH-wt/shots/localsend-dialog.png"),
            Size::new(1000.0, 640.0),
        );

        let settings = Settings {
            section: Section::LocalSend,
            localsend: LocalSendState {
                always_on: true,
                alias: String::from("GSH da sala"),
                receive_dir: PathBuf::from("/home/ana/Downloads/GSH"),
                max_file_mb: String::from("2048"),
                max_files: String::from("100"),
                server_status: Some(String::from("Ativo · porta 53317 · SHA-256 6A:21:9F:04")),
                peers: vec![
                    LocalSendPeer {
                        alias: String::from("Notebook da sala"),
                        device_type: String::from("desktop"),
                        ip: String::from("192.168.1.42"),
                    },
                    LocalSendPeer {
                        alias: String::from("Pixel 9"),
                        device_type: String::from("mobile"),
                        ip: String::from("192.168.1.57"),
                    },
                ],
                ..LocalSendState::default()
            },
            ..Settings::default()
        };
        let page = crate::settings::view::view(&settings);
        capture(
            page,
            Path::new("/home/heliodev/projetos/GSH-wt/shots/localsend-settings.png"),
            Size::new(1280.0, 760.0),
        );
    }
}
