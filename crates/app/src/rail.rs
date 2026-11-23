//! Rail de projetos/workspaces à esquerda (`projects_rail.dart` do Flutter).

use gsh_core::workspace::{Workspace, WorkspaceList};
use iced::widget::{button, column, container, row, scrollable, text, tooltip, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Padding, Theme};

use crate::labels;
use crate::theme;

#[path = "channel_editor.rs"]
mod channel_editor;
pub use channel_editor::{ChannelAction, ChannelActions, ChannelEditor, ChannelMemberOption};

/// Estado visual do botão de manter acordado no rodapé do rail.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeepAwakeStatus {
    pub active: bool,
    pub on_battery: bool,
}

pub fn keep_awake_tooltip(status: KeepAwakeStatus) -> &'static str {
    match (status.active, status.on_battery) {
        (false, _) => concat!(
            "Manter este computador acordado para acesso remoto. Desliga sozinho quando o GSH Desktop reinicia.\n",
            "Fechar a tampa do notebook continua colocando-o para dormir."
        ),
        (true, false) => concat!(
            "Mantendo este computador acordado para acesso remoto. Clique para deixá-lo dormir de novo.\n",
            "Fechar a tampa do notebook continua colocando-o para dormir."
        ),
        (true, true) => concat!(
            "Mantendo este computador acordado para acesso remoto. Clique para deixá-lo dormir de novo.\n",
            "Acordado na bateria. Isso consome a bateria.\n",
            "Fechar a tampa do notebook continua colocando-o para dormir."
        ),
    }
}

/// Lista inicial de workspaces do shell.
pub fn default_workspace_list() -> WorkspaceList {
    let mut gsh = Workspace::system(
        "gsh",
        labels::WORKSPACE_GSH,
        Some(labels::WORKSPACE_GSH_BRANCH.to_string()),
    );
    if let Ok(cwd) = std::env::current_dir() {
        gsh = gsh.with_path(cwd);
    }
    let example = Workspace::system(
        "example",
        labels::WORKSPACE_EXAMPLE,
        Some(labels::WORKSPACE_EXAMPLE_BRANCH.to_string()),
    );
    WorkspaceList::new(vec![gsh, example])
}

/// Estado visual e callbacks do rodapé/avisos do rail lateral.
pub struct RailActions<'a, M> {
    pub keep_awake: KeepAwakeStatus,
    pub update_version: Option<&'a str>,
    pub on_add_workspace: M,
    pub channels: ChannelActions<'a, M>,
    pub on_toggle_keep_awake: M,
    pub on_download_update: M,
    pub on_dismiss_update: M,
    pub on_settings: M,
}

/// Renderiza o rail lateral de workspaces com o botão "+" de abrir projeto.
pub fn view<'a, M: Clone + 'a>(
    workspaces: &'a WorkspaceList,
    on_select: impl Fn(usize) -> M,
    actions: RailActions<'a, M>,
) -> Element<'a, M> {
    let RailActions {
        keep_awake,
        update_version,
        on_add_workspace,
        channels,
        on_toggle_keep_awake,
        on_download_update,
        on_dismiss_update,
        on_settings,
    } = actions;
    let header = container(
        row![
            text(labels::ICON_LAYERS)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON)
                .color(theme::palette().text2),
            Space::new().width(9.0),
            text(labels::WORKSPACES_HEADER)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            Space::new().width(Fill),
            footer_icon_button(labels::ICON_PLUS, on_add_workspace),
        ]
        .align_y(Alignment::Center),
    )
    .padding(Padding {
        top: 16.0,
        right: 12.0,
        bottom: 12.0,
        left: 16.0,
    });

    let selected_idx = workspaces.selected_index();
    let visible = workspaces.visible_indices();
    let mut list = column![].spacing(2.0).padding([0, 8]);
    if visible.is_empty() && workspaces.active_context_channel().is_some() {
        list = list.push(text(labels::CHANNELS_EMPTY).size(theme::SIZE_LABEL));
    }
    for idx in visible {
        if let Some(workspace) = workspaces.items().get(idx) {
            list = list.push(workspace_row(
                workspace,
                idx == selected_idx,
                on_select(idx),
            ));
        }
    }

    let bolt_color = if !keep_awake.active {
        theme::palette().text3
    } else if keep_awake.on_battery {
        theme::palette().warn
    } else {
        theme::palette().accent
    };
    let bolt_tip = keep_awake_tooltip(keep_awake);
    let keep_awake_btn = tooltip(
        footer_icon_button_colored(labels::ICON_BOLT, bolt_color, on_toggle_keep_awake),
        crate::pane_chrome::tip_bubble(bolt_tip),
        tooltip::Position::Top,
    );
    let settings_btn = tooltip(
        footer_icon_button(labels::ICON_SETTINGS, on_settings),
        crate::pane_chrome::tip_bubble(labels::RAIL_SETTINGS_TOOLTIP),
        tooltip::Position::Top,
    );
    let footer = container(
        row![Space::new().width(Fill), keep_awake_btn, settings_btn]
            .spacing(4)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::FOOTER_HEIGHT))
    .padding([0, 12]);

    let content = column![channel_editor::card(workspaces, channels), list].spacing(8);
    let mut main_col = column![header, scrollable(content).width(Fill).height(Fill)];
    if let Some(version) = update_version {
        main_col = main_col.push(update_card(version, on_download_update, on_dismiss_update));
    }
    main_col = main_col.push(hairline_h()).push(footer);

    let body = container(main_col)
        .width(Length::Fixed(theme::RAIL_WIDTH - 1.0))
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().bg)),
            ..Default::default()
        });

    row![body, hairline_v()]
        .width(Length::Fixed(theme::RAIL_WIDTH))
        .height(Fill)
        .into()
}

fn workspace_row<'a, M: Clone + 'a>(
    item: &'a Workspace,
    selected: bool,
    on_press: M,
) -> Element<'a, M> {
    let avatar = container(
        text(item.initial())
            .font(theme::FONT_TITLE)
            .size(theme::WORKSPACE_AVATAR_SIZE * theme::WORKSPACE_AVATAR_INITIAL_SCALE)
            .color(theme::palette().text),
    )
    .width(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
    .height(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
    .center_x(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
    .center_y(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        border: Border {
            radius: theme::RADIUS_CARD.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let mut details = column![text(&item.name)
        .font(if selected {
            theme::FONT_BODY_MEDIUM
        } else {
            theme::FONT_BODY
        })
        .size(theme::SIZE_RAIL_ITEM)
        .color(theme::palette().text),]
    .spacing(4);

    if let Some(branch) = &item.git_branch {
        let branch_badge = container(
            row![
                text(labels::ICON_GIT_BRANCH)
                    .font(theme::FONT_ICON)
                    .size(theme::GIT_BADGE_ICON_SIZE)
                    .color(theme::palette().text3),
                text(branch)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_BADGE)
                    .color(theme::palette().text3)
            ]
            .spacing(theme::GIT_BADGE_ICON_GAP)
            .align_y(Alignment::Center),
        )
        .padding(Padding {
            top: 1.0,
            right: 5.0,
            bottom: 1.0,
            left: 4.0,
        })
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel3)),
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        });
        details = details.push(branch_badge);
    }

    button(
        row![avatar, container(details).width(Fill)]
            .spacing(10)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([7, 9])
    .on_press(on_press)
    .style(move |_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let bg = if selected {
            Some(Background::Color(theme::palette().panel2))
        } else if hovered {
            Some(Background::Color(theme::palette().panel3))
        } else {
            None
        };
        button::Style {
            background: bg,
            text_color: theme::palette().text,
            border: Border {
                radius: theme::RADIUS_CARD.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

/// Botão pequeno do rodapé (`_SmallIcon` de `projects_rail.dart`): 26×26,
/// ícone 16 em `text3`, raio 5, hover `panel3`.
fn footer_icon_button<'a, M: Clone + 'a>(glyph: &'static str, on_press: M) -> Element<'a, M> {
    footer_icon_button_colored(glyph, theme::palette().text3, on_press)
}

fn footer_icon_button_colored<'a, M: Clone + 'a>(
    glyph: &'static str,
    color: Color,
    on_press: M,
) -> Element<'a, M> {
    button(
        container(
            text(glyph)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON)
                .color(color),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(26.0))
    .height(Length::Fixed(26.0))
    .padding(0)
    .on_press(on_press)
    .style(move |_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::palette().panel3))
            } else {
                None
            },
            text_color: color,
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

fn update_card<'a, M: Clone + 'a>(
    version: &'a str,
    on_download: M,
    on_dismiss: M,
) -> Element<'a, M> {
    let subtitle = format!("v{version} — {}", labels::UPDATE_CARD_CLICK_TO_DOWNLOAD);
    let download_btn = button(
        column![
            text(labels::UPDATE_CARD_TITLE)
                .font(theme::FONT_BODY_SEMIBOLD)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text),
            text(subtitle)
                .font(theme::FONT_BODY)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        ]
        .spacing(1.0)
        .width(Fill),
    )
    .width(Fill)
    .padding(0)
    .on_press(on_download)
    .style(|_: &Theme, _| button::Style {
        background: None,
        text_color: theme::palette().text,
        ..Default::default()
    });
    let dismiss_btn = tooltip(
        footer_icon_button(labels::ICON_CLOSE, on_dismiss),
        crate::pane_chrome::tip_bubble(labels::UPDATE_CARD_DISMISS_TOOLTIP),
        tooltip::Position::Top,
    );
    container(
        container(
            row![download_btn, dismiss_btn]
                .spacing(6.0)
                .align_y(Alignment::Center),
        )
        .padding([8, 10])
        .width(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                color: theme::palette().accent,
                width: 1.0,
                radius: theme::RADIUS_CARD.into(),
            },
            ..Default::default()
        }),
    )
    .padding(Padding {
        top: 0.0,
        right: 12.0,
        bottom: 8.0,
        left: 12.0,
    })
    .width(Fill)
    .into()
}

fn hairline_h<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(Length::Fixed(1.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        })
        .into()
}

fn hairline_v<'a, M: 'a>() -> Element<'a, M> {
    container(Space::new().width(1.0).height(Fill))
        .width(Length::Fixed(1.0))
        .height(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().border)),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_workspace_list_starts_with_gsh_and_allows_opening_project_folder() {
        let mut list = default_workspace_list();
        assert_eq!(
            list.active().map(|w| w.name.as_str()),
            Some(labels::WORKSPACE_GSH)
        );
        let idx = list.open_folder("/tmp/novo-projeto");
        assert_eq!(idx, 2);
        assert_eq!(list.active().map(|w| w.name.as_str()), Some("novo-projeto"));
    }

    #[test]
    fn keep_awake_tooltip_includes_battery_warning_only_when_active_on_battery() {
        let off = keep_awake_tooltip(KeepAwakeStatus {
            active: false,
            on_battery: true,
        });
        assert!(off.contains(labels::RAIL_KEEP_AWAKE_OFF));
        assert!(!off.contains(labels::RAIL_KEEP_AWAKE_BATTERY));
        assert!(off.contains(labels::RAIL_KEEP_AWAKE_LID));

        let on_ac = keep_awake_tooltip(KeepAwakeStatus {
            active: true,
            on_battery: false,
        });
        assert!(on_ac.contains(labels::RAIL_KEEP_AWAKE_ON));
        assert!(!on_ac.contains(labels::RAIL_KEEP_AWAKE_BATTERY));

        let on_bat = keep_awake_tooltip(KeepAwakeStatus {
            active: true,
            on_battery: true,
        });
        assert!(on_bat.contains(labels::RAIL_KEEP_AWAKE_ON));
        assert!(on_bat.contains(labels::RAIL_KEEP_AWAKE_BATTERY));
        assert!(on_bat.contains(labels::RAIL_KEEP_AWAKE_LID));
    }
}
