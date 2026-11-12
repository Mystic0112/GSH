//! Rail de projetos/workspaces à esquerda (`projects_rail.dart` do Flutter).
//!
//! Superfície `BG`, divisória direita de 1px `BORDER`, cabeçalho "Workspaces"
//! com botão "+" para abrir pasta de projeto, lista de itens (item selecionado
//! com fundo `PANEL2` e borda `ACCENT` 1px, hover `PANEL3`) e rodapé de 34px.

use gsh_core::workspace::{Workspace, WorkspaceList};
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

use crate::labels;
use crate::theme;

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

/// Renderiza o rail lateral de workspaces com o botão "+" de abrir projeto.
pub fn view<'a, M: Clone + 'a>(
    workspaces: &'a WorkspaceList,
    on_select: impl Fn(usize) -> M,
    on_add_workspace: M,
) -> Element<'a, M> {
    let add_button = button(
        container(
            text(labels::ICON_PLUS)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TOPBAR_TITLE)
                .color(theme::palette().text2),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(Length::Fixed(26.0))
    .height(Length::Fixed(26.0))
    .padding(0)
    .on_press(on_add_workspace)
    .style(|_: &Theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: if hovered {
                Some(Background::Color(theme::palette().panel3))
            } else {
                None
            },
            text_color: if hovered {
                theme::palette().text
            } else {
                theme::palette().text2
            },
            border: Border {
                radius: theme::RADIUS_BUTTON.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    });

    let header = container(
        row![
            text(labels::ICON_LAYERS)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text3),
            Space::new().width(7.0),
            text(labels::WORKSPACES_HEADER)
                .font(theme::FONT_TITLE)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text),
            Space::new().width(Fill),
            add_button,
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
    let mut list = column![].spacing(2.0).padding([4, 8]);
    for (idx, item) in workspaces.items().iter().enumerate() {
        let is_selected = idx == selected_idx;
        list = list.push(workspace_row(item, is_selected, on_select(idx)));
    }

    let footer = container(
        row![
            text(labels::ICON_LAYERS)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
            Space::new().width(6.0),
            text(labels::DEFAULT_REALM)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text2),
            Space::new().width(4.0),
            text(labels::ICON_CHEVRON_DOWN)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
            Space::new().width(Fill),
            text(labels::ICON_SETTINGS)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_TITLE)
                .color(theme::palette().text3),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(theme::FOOTER_HEIGHT))
    .padding([0, 12]);

    let body = container(column![
        header,
        scrollable(list).width(Fill).height(Fill),
        hairline_h(),
        footer,
    ])
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
            .size(theme::SIZE_LABEL)
            .color(if selected {
                theme::palette().accent_text
            } else {
                theme::palette().text2
            }),
    )
    .width(Length::Fixed(30.0))
    .height(Length::Fixed(30.0))
    .center_x(Length::Fixed(30.0))
    .center_y(Length::Fixed(30.0))
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        border: Border {
            radius: theme::RADIUS_BUTTON.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let mut content = row![
        avatar,
        container(
            text(&item.name)
                .font(if selected {
                    theme::FONT_BODY_MEDIUM
                } else {
                    theme::FONT_BODY
                })
                .size(theme::SIZE_RAIL_ITEM)
                .color(theme::palette().text2),
        )
        .width(Fill),
    ]
    .spacing(10.0)
    .align_y(Alignment::Center);

    if let Some(branch) = &item.git_branch {
        let branch_badge = container(
            text(branch)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_BADGE)
                .color(theme::palette().text3),
        )
        .padding([2, 6])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel3)),
            border: Border {
                radius: theme::RADIUS_BADGE.into(),
                ..Default::default()
            },
            ..Default::default()
        });
        content = content.push(branch_badge);
    }

    let indicator =
        container(Space::new().width(2).height(20)).style(move |_: &Theme| container::Style {
            background: selected.then_some(Background::Color(theme::palette().accent)),
            ..Default::default()
        });
    button(
        row![indicator, content]
            .spacing(7)
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
}
