//! Rail de projetos/workspaces à esquerda (`projects_rail.dart` do Flutter).

use gsh_core::workspace::{Workspace, WorkspaceKind, WorkspaceList};
use iced::widget::{button, column, container, row, scrollable, text, tooltip, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Padding, Theme};

use crate::labels;
use crate::theme;

#[path = "channel_editor.rs"]
mod channel_editor;
#[path = "channel_footer.rs"]
mod channel_footer;
pub use channel_editor::{ChannelAction, ChannelActions, ChannelEditor, ChannelMemberOption};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceMenuAction {
    Configure,
    CopyId,
    Close,
    MoveTo(String),
    Toggle,
    Dismiss,
}

/// Ações do menu do "+" do cabeçalho (`newWorkspaceMenu` do Flutter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewWorkspaceMenuAction {
    Toggle,
    Dismiss,
    Local,
    Remote,
}

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

/// Lista inicial de workspaces do shell: apenas o workspace de sistema "GSH Desktop"
/// (terminal-only, sem pasta e sem workspaces de exemplo, espelhando o Flutter).
pub fn default_workspace_list() -> WorkspaceList {
    WorkspaceList::new(vec![Workspace::system("gsh", labels::WORKSPACE_GSH, None)])
}

/// Estado visual e callbacks do rodapé/avisos do rail lateral.
pub struct RailActions<'a, M> {
    pub keep_awake: KeepAwakeStatus,
    pub update_version: Option<&'a str>,
    pub new_workspace_menu_open: bool,
    pub on_new_workspace_menu: fn(NewWorkspaceMenuAction) -> M,
    pub channels: ChannelActions<'a, M>,
    pub channel_popup_open: bool,
    pub on_toggle_channel_popup: M,
    pub on_close_channel_popup: M,
    pub on_toggle_keep_awake: M,
    pub on_download_update: M,
    pub on_dismiss_update: M,
    pub on_settings: M,
    pub workspace_menu_open: Option<usize>,
    pub on_workspace_menu: fn(usize, WorkspaceMenuAction) -> M,
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
        new_workspace_menu_open,
        on_new_workspace_menu,
        channels,
        channel_popup_open,
        on_toggle_channel_popup,
        on_close_channel_popup,
        on_toggle_keep_awake,
        on_download_update,
        on_dismiss_update,
        on_settings,
        workspace_menu_open,
        on_workspace_menu,
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
            new_workspace_button(new_workspace_menu_open, on_new_workspace_menu),
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
    let gsh_slot = workspaces
        .items()
        .iter()
        .enumerate()
        .find(|(_, ws)| ws.kind == WorkspaceKind::SystemTerminal)
        .map(|(idx, _)| gsh_slot_view(idx == selected_idx, on_select(idx)));

    let project_indices: Vec<usize> = workspaces
        .visible_indices()
        .into_iter()
        .filter(|&idx| {
            workspaces
                .items()
                .get(idx)
                .is_some_and(|ws| ws.kind != WorkspaceKind::SystemTerminal)
        })
        .collect();

    let projects_area: Element<'a, M> = if project_indices.is_empty() {
        empty_rail()
    } else {
        let mut list = column![].spacing(2.0).padding([0, 8]);
        for idx in project_indices {
            if let Some(workspace) = workspaces.items().get(idx) {
                list = list.push(workspace_row(
                    workspace,
                    idx == selected_idx,
                    on_select(idx),
                    workspace_menu_open == Some(idx),
                    &workspaces.channels(),
                    idx,
                    on_workspace_menu,
                ));
            }
        }
        scrollable(column![list]).width(Fill).height(Fill).into()
    };

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
        row![
            channel_footer::view(
                workspaces,
                channels,
                channel_popup_open,
                on_toggle_channel_popup,
                on_close_channel_popup,
            ),
            keep_awake_btn,
            settings_btn,
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::FOOTER_HEIGHT))
    .center_y(Length::Fixed(theme::FOOTER_HEIGHT))
    .padding([0, 12]);

    let mut main_col = column![header];
    if let Some(slot) = gsh_slot {
        main_col = main_col.push(slot);
    }
    main_col = main_col.push(projects_area);
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

/// Slot fixo do workspace de sistema "GSH Desktop" (`_GshSlot` em `projects_rail.dart`).
fn gsh_slot_view<'a, M: Clone + 'a>(selected: bool, on_press: M) -> Element<'a, M> {
    let icon_box = container(
        text(labels::ICON_TERMINAL)
            .font(theme::FONT_ICON)
            .size(17.0)
            .color(theme::palette().text2),
    )
    .width(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
    .height(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
    .center_x(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
    .center_y(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: theme::RADIUS_CARD.into(),
        },
        ..Default::default()
    });

    let slot = button(
        row![
            icon_box,
            text(labels::WORKSPACE_GSH)
                .font(if selected {
                    theme::FONT_BODY_MEDIUM
                } else {
                    theme::FONT_BODY
                })
                .size(theme::SIZE_RAIL_ITEM)
                .color(theme::palette().text),
        ]
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
    });

    container(slot)
        .padding(Padding {
            top: 0.0,
            right: 8.0,
            bottom: 4.0,
            left: 8.0,
        })
        .width(Fill)
        .into()
}

/// Estado vazio da lista de workspaces (`_EmptyRail` em `projects_rail.dart`).
fn empty_rail<'a, M: 'a>() -> Element<'a, M> {
    container(
        text(labels::RAIL_NO_WORKSPACES)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3)
            .align_x(iced::alignment::Horizontal::Center),
    )
    .padding(16)
    .width(Fill)
    .height(Fill)
    .center_x(Fill)
    .center_y(Fill)
    .into()
}

fn workspace_row<'a, M: Clone + 'a>(
    item: &'a Workspace,
    selected: bool,
    on_press: M,
    menu_open: bool,
    channels: &[String],
    index: usize,
    on_menu: fn(usize, WorkspaceMenuAction) -> M,
) -> Element<'a, M> {
    let avatar_content: Element<'a, M> =
        match item.image_path.as_ref().filter(|path| path.is_file()) {
            Some(path) => iced::widget::image(iced::widget::image::Handle::from_path(path))
                .width(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
                .height(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
                .into(),
            None => text(item.initial())
                .font(theme::FONT_TITLE)
                .size(theme::WORKSPACE_AVATAR_SIZE * theme::WORKSPACE_AVATAR_INITIAL_SCALE)
                .color(theme::palette().text)
                .into(),
        };
    let avatar_color = item.color.map_or(theme::palette().panel, |value| {
        iced::Color::from_rgba8(
            (value >> 16) as u8,
            (value >> 8) as u8,
            value as u8,
            ((value >> 24) as u8) as f32 / 255.0,
        )
    });
    let avatar = container(avatar_content)
        .width(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
        .height(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
        .center_x(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
        .center_y(Length::Fixed(theme::WORKSPACE_AVATAR_SIZE))
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(avatar_color)),
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

    let select = button(
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
    });
    let trigger: Element<'a, M> = if item.kind == WorkspaceKind::Project {
        button(text("⋮").size(18).color(theme::palette().text3))
            .padding([4, 8])
            .on_press(on_menu(index, WorkspaceMenuAction::Toggle))
            .style(|_: &Theme, _| button::Style {
                background: None,
                text_color: theme::palette().text,
                ..Default::default()
            })
            .into()
    } else {
        Space::new().width(0).into()
    };
    let base = iced::widget::mouse_area(row![select, trigger].align_y(Alignment::Center))
        .on_right_press(on_menu(index, WorkspaceMenuAction::Toggle));
    let menu = (menu_open && item.kind == WorkspaceKind::Project).then(|| {
        let mut items = vec![
            crate::app_menu::Item::Entry(crate::app_menu::Entry::new(
                labels::WORKSPACE_MENU_CONFIGURE,
                on_menu(index, WorkspaceMenuAction::Configure),
            )),
            crate::app_menu::Item::Entry(crate::app_menu::Entry::new(
                labels::WORKSPACE_MENU_COPY_ID,
                on_menu(index, WorkspaceMenuAction::CopyId),
            )),
            crate::app_menu::Item::Divider,
        ];
        for channel in channels {
            items.push(crate::app_menu::Item::Entry(crate::app_menu::Entry::new(
                format!("{} {channel}", labels::WORKSPACE_MENU_MOVE_TO),
                on_menu(index, WorkspaceMenuAction::MoveTo(channel.clone())),
            )));
        }
        items.push(crate::app_menu::Item::Divider);
        items.push(crate::app_menu::Item::Entry(
            crate::app_menu::Entry::new("Fechar", on_menu(index, WorkspaceMenuAction::Close))
                .danger(),
        ));
        crate::app_menu::menu(items, 190.0)
    });
    crate::anchored::Anchored::new(base, menu, on_menu(index, WorkspaceMenuAction::Dismiss))
        .side()
        .on_escape(on_menu(index, WorkspaceMenuAction::Dismiss))
        .into()
}

/// "+" do cabeçalho: abre o menu Local/Remoto ancorado no botão.
fn new_workspace_button<'a, M: Clone + 'a>(
    open: bool,
    on_menu: fn(NewWorkspaceMenuAction) -> M,
) -> Element<'a, M> {
    let base = footer_icon_button(labels::ICON_PLUS, on_menu(NewWorkspaceMenuAction::Toggle));
    let menu = open.then(|| {
        crate::app_menu::menu(
            vec![
                crate::app_menu::Item::Entry(
                    crate::app_menu::Entry::new(
                        labels::NEW_WORKSPACE_LOCAL,
                        on_menu(NewWorkspaceMenuAction::Local),
                    )
                    .leading(crate::app_menu::Leading::Glyph(labels::ICON_FOLDER)),
                ),
                crate::app_menu::Item::Entry(
                    crate::app_menu::Entry::new(
                        labels::NEW_WORKSPACE_REMOTE,
                        on_menu(NewWorkspaceMenuAction::Remote),
                    )
                    .leading(crate::app_menu::Leading::Glyph(labels::ICON_CLOUD)),
                ),
            ],
            140.0,
        )
    });
    crate::anchored::Anchored::new(base, menu, on_menu(NewWorkspaceMenuAction::Dismiss))
        .on_escape(on_menu(NewWorkspaceMenuAction::Dismiss))
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
    fn default_workspace_list_has_only_gsh_system_slot_without_fake_examples() {
        let mut list = default_workspace_list();
        assert_eq!(list.items().len(), 1);
        let gsh = list.active().expect("gsh system workspace");
        assert_eq!(gsh.id, "gsh");
        assert_eq!(gsh.name, labels::WORKSPACE_GSH);
        assert_eq!(gsh.kind, WorkspaceKind::SystemTerminal);
        assert!(gsh.path.is_none());
        assert!(gsh.git_branch.is_none());

        let idx = list.open_folder("/tmp/novo-projeto");
        assert_eq!(idx, 1);
        assert_eq!(list.active().map(|w| w.name.as_str()), Some("novo-projeto"));
    }

    #[test]
    fn empty_rail_shows_gsh_slot_and_flutter_empty_message() {
        let workspaces = default_workspace_list();
        let view = view(
            &workspaces,
            |_| (),
            RailActions {
                keep_awake: KeepAwakeStatus::default(),
                update_version: None,
                new_workspace_menu_open: false,
                on_new_workspace_menu: |_| (),
                channels: ChannelActions {
                    editor: None,
                    members: Vec::new(),
                    on_action: |_| (),
                },
                channel_popup_open: false,
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
        let mut ui = iced_test::simulator(view);
        assert!(ui.find(labels::WORKSPACE_GSH).is_ok());
        assert!(ui.find(labels::RAIL_NO_WORKSPACES).is_ok());
        assert!(ui.find("Projeto de exemplo").is_err());
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
