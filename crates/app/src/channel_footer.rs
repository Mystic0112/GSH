//! Seletor compacto de canal usado no rodapé do rail.

use gsh_core::workspace::WorkspaceList;
use iced::widget::{button, container, row, text, Space};
use iced::{Background, Border, Element, Fill, Length, Theme};

use crate::rail::{ChannelAction, ChannelActions};
use crate::{app_menu, labels, theme};

pub(super) fn view<'a, M: Clone + 'a>(
    workspaces: &'a WorkspaceList,
    actions: ChannelActions<'a, M>,
    popup_open: bool,
    on_toggle: M,
    on_close: M,
) -> Element<'a, M> {
    let selector = selector(workspaces, actions.editor.is_some(), on_toggle);
    let popup = popup_open.then(|| popup(workspaces, actions));
    crate::anchored::Anchored::new(selector, popup, on_close.clone())
        .on_escape(on_close)
        .into()
}

fn selector<'a, M: Clone + 'a>(
    workspaces: &'a WorkspaceList,
    editing: bool,
    on_press: M,
) -> Element<'a, M> {
    let label = workspaces
        .active_context_channel()
        .map_or(labels::CHANNELS_ALL, |channel| channel.name());
    button(
        container(
            row![
                online_dot(),
                text(ellipsize_channel_name(label))
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text2)
                    .wrapping(text::Wrapping::None),
                Space::new().width(Fill),
                text(labels::ICON_EXPAND_MORE)
                    .font(theme::FONT_ICON)
                    .size(14.0)
                    .color(theme::palette().text3),
            ]
            .spacing(5.0)
            .align_y(iced::Alignment::Center),
        )
        .width(Fill)
        .height(Fill)
        .center_y(Fill),
    )
    .width(Fill)
    .height(Length::Fixed(26.0))
    .padding([0, 6])
    .on_press(on_press)
    .style(move |_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: (hovered || editing).then_some(Background::Color(theme::palette().panel3)),
            text_color: theme::palette().text2,
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

fn online_dot<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(7.0).height(7.0))
        .width(Length::Fixed(7.0))
        .height(Length::Fixed(7.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().online)),
            border: Border {
                radius: 4.0.into(),
                ..Default::default()
            },
            shadow: iced::Shadow {
                color: theme::palette().online.scale_alpha(0.48),
                offset: iced::Vector::ZERO,
                blur_radius: 5.0,
            },
            ..Default::default()
        })
        .into()
}

/// Iced `Text` não tem truncamento com reticências; usa a largura conhecida do rail.
fn ellipsize_channel_name(name: &str) -> String {
    const MAX_LABEL_CHARS: usize = 18;
    if name.chars().count() <= MAX_LABEL_CHARS {
        return name.to_owned();
    }
    let mut truncated: String = name.chars().take(MAX_LABEL_CHARS - 1).collect();
    truncated.push('…');
    truncated
}

fn popup<'a, M: Clone + 'a>(
    workspaces: &'a WorkspaceList,
    actions: ChannelActions<'a, M>,
) -> Element<'a, M> {
    if let Some(editor) = actions.editor {
        return super::channel_editor::card(workspaces, editor, actions.members, actions.on_action);
    }

    let on_action = actions.on_action;
    let active_channel = workspaces.active_context_channel();
    let mut channels: Vec<_> = workspaces
        .context_channels()
        .iter()
        .filter(|channel| !channel.archived)
        .collect();
    channels.sort_by_cached_key(|channel| channel.name().to_lowercase());

    let mut items = vec![app_menu::Item::Entry(
        app_menu::Entry::new(labels::CHANNELS_ALL, on_action(ChannelAction::Select(None)))
            .selected(active_channel.is_none()),
    )];
    items.extend(channels.iter().map(|channel| {
        app_menu::Item::Entry(
            app_menu::Entry::new(
                channel.name(),
                on_action(ChannelAction::Select(Some(channel.id.clone()))),
            )
            .selected(active_channel.is_some_and(|active| active.id == channel.id)),
        )
    }));
    items.push(app_menu::Item::Divider);
    items.push(app_menu::Item::Entry(app_menu::Entry::new(
        labels::CHANNEL_NEW,
        on_action(ChannelAction::New),
    )));
    if let Some(channel) = active_channel {
        items.push(app_menu::Item::Entry(app_menu::Entry::new(
            labels::CHANNEL_EDIT,
            on_action(ChannelAction::Edit(channel.id.clone())),
        )));
    }
    if let Some(workspace) = workspaces.active() {
        let other_channels: Vec<_> = channels
            .iter()
            .copied()
            .filter(|channel| workspace.channel_id() != Some(&channel.id))
            .collect();
        if !other_channels.is_empty() {
            items.push(app_menu::Item::Divider);
            items.push(app_menu::Item::Custom(
                text(labels::CHANNEL_MOVE_WORKSPACE_SECTION)
                    .font(theme::FONT_BODY_SEMIBOLD)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().text3)
                    .into(),
            ));
            items.extend(other_channels.into_iter().map(|channel| {
                app_menu::Item::Entry(app_menu::Entry::new(
                    channel.name(),
                    on_action(ChannelAction::AssignWorkspace(channel.id.clone())),
                ))
            }));
        }
    }
    app_menu::menu(items, 180.0)
}

#[cfg(test)]
#[test]
#[ignore = "gera as capturas do footer do rail para revisão visual"]
fn capture_channel_footer_snapshots() {
    use gsh_core::workspace::{ChannelDraft, WorkspaceList};
    use iced::{Settings, Size};
    use iced_test::simulator::Simulator;
    use std::path::{Path, PathBuf};

    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    theme::set_active(theme::ThemeVariant::Dark);
    let settings = Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };

    fn make_workspaces() -> WorkspaceList {
        let mut workspaces = super::default_workspace_list();
        let channel_id = workspaces
            .create_channel(ChannelDraft {
                name: "Captura Alfa".to_owned(),
                ..Default::default()
            })
            .expect("create snapshot channel");
        let _ = workspaces
            .create_channel(ChannelDraft {
                name: "Captura Beta".to_owned(),
                ..Default::default()
            })
            .expect("create second snapshot channel");
        let selected_index = workspaces.selected_index();
        workspaces
            .assign_workspace_channel(selected_index, &channel_id)
            .expect("assign snapshot workspace channel");
        workspaces
            .select_context_channel(Some(&channel_id))
            .expect("select snapshot channel filter");
        workspaces
    }

    fn capture(settings: &Settings, filename: &str, popup_open: bool, editing: bool) {
        let workspaces = make_workspaces();
        let editor = editing.then(|| super::ChannelEditor::new(None));
        let view = super::view(
            &workspaces,
            |_| (),
            super::RailActions {
                keep_awake: super::KeepAwakeStatus::default(),
                update_version: None,
                on_add_workspace: (),
                channels: ChannelActions {
                    editor: editor.as_ref(),
                    members: Vec::new(),
                    on_action: |_| (),
                },
                channel_popup_open: popup_open,
                on_toggle_channel_popup: (),
                on_close_channel_popup: (),
                on_toggle_keep_awake: (),
                on_download_update: (),
                on_dismiss_update: (),
                on_settings: (),
                workspace_menu_open: None,
                on_workspace_menu: |_, _| (),
            },
        );
        let mut simulator = Simulator::with_size(settings.clone(), Size::new(252.0, 620.0), view);
        let target = PathBuf::from("/home/heliodev/projetos/GSH-wt/shots").join(filename);
        let generated = Path::new("/home/heliodev/projetos/GSH-wt/shots")
            .join(filename.replace(".png", "-tiny-skia.png"));
        let _ = std::fs::remove_file(&target);
        let _ = std::fs::remove_file(&generated);
        simulator
            .snapshot(&Theme::Dark)
            .expect("render rail footer snapshot")
            .matches_image(&target)
            .expect("write rail footer snapshot");
        std::fs::rename(generated, target).expect("rename rail footer snapshot");
    }

    capture(&settings, "gsh-rail-footer-fechado.png", false, false);
    capture(&settings, "gsh-rail-footer-menu.png", true, false);
    capture(&settings, "gsh-rail-footer-form.png", true, true);
}
