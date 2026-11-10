//! Painel expandido da ilha de agentes (`agent_island_panel.dart` no Flutter).
//!
//! Exibe as seções na ordem exata do Flutter:
//! 1. Erro tipado do servidor de status (se o `StatusServer` falhou ao subir)
//! 2. "Pedem permissão" (`IslandAgentState::AskingPermission`)
//! 3. "Esperando você" (`IslandAgentState::Waiting`)
//! 4. "Terminaram" (`IslandAgentState::Finished`)
//! 5. "Trabalhando" (`IslandAgentState::Working`)

use iced::widget::{button, column, container, mouse_area, row, scrollable, text, Space};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Shadow, Shrink, Theme, Vector,
};

use crate::agent_status::IslandServerError;
use crate::island::{
    IslandAgent, IslandAgentState, IslandMsg, PermissionDecision, COLOR_ACCENT, COLOR_BORDER2,
    COLOR_ERROR, COLOR_ONLINE, COLOR_PANEL, COLOR_PANEL2, COLOR_PANEL3, COLOR_SHADOW, COLOR_TEXT,
    COLOR_TEXT2, COLOR_TEXT3, COLOR_WARN, PANEL_WIDTH,
};
use crate::{labels, theme};

/// Renderiza o painel aberto da ilha com a lista de agentes ativos (e eventual
/// erro tipado do servidor de status).
pub fn view_panel<'a>(
    agents: &'a [IslandAgent],
    server_error: Option<&'a IslandServerError>,
) -> Element<'a, IslandMsg> {
    let asking: Vec<&IslandAgent> = agents
        .iter()
        .filter(|a| a.state == IslandAgentState::AskingPermission)
        .collect();
    let waiting: Vec<&IslandAgent> = agents
        .iter()
        .filter(|a| a.state == IslandAgentState::Waiting)
        .collect();
    let finished: Vec<&IslandAgent> = agents
        .iter()
        .filter(|a| a.state == IslandAgentState::Finished)
        .collect();
    let working: Vec<&IslandAgent> = agents
        .iter()
        .filter(|a| a.state == IslandAgentState::Working)
        .collect();

    let mut sections: Vec<Element<'_, IslandMsg>> = Vec::new();

    if let Some(err) = server_error {
        sections.push(section_header(labels::SECTION_STATUS_ERROR, COLOR_ERROR));
        sections.push(error_card(err));
    }

    if !asking.is_empty() {
        sections.push(section_header(labels::SECTION_ASKS, COLOR_WARN));
        for agent in asking {
            sections.push(ask_card(agent));
        }
    }

    if !waiting.is_empty() {
        sections.push(section_header(labels::SECTION_WAITING, COLOR_WARN));
        for agent in waiting {
            sections.push(agent_row(agent, COLOR_WARN, true, false));
        }
    }

    if !finished.is_empty() {
        sections.push(section_header(labels::SECTION_FINISHED, COLOR_ONLINE));
        for agent in finished {
            sections.push(agent_row(agent, COLOR_ONLINE, false, false));
        }
    }

    if !working.is_empty() {
        sections.push(section_header(labels::SECTION_WORKING, COLOR_ACCENT));
        for agent in working {
            sections.push(agent_row(agent, COLOR_ACCENT, false, true));
        }
    }

    if sections.is_empty() {
        sections.push(
            container(text(labels::ISLAND_EMPTY).size(13).color(COLOR_TEXT3))
                .padding(8)
                .into(),
        );
    }

    let content = scrollable(column(sections).spacing(2).padding(8));

    let panel_box = container(content)
        .width(Length::Fixed(PANEL_WIDTH))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(COLOR_PANEL)),
            border: Border {
                color: COLOR_BORDER2,
                width: 1.0,
                radius: 12.0.into(),
            },
            shadow: Shadow {
                color: COLOR_SHADOW,
                offset: Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            },
            ..Default::default()
        });

    // Absorve cliques dentro do painel para não acionar o fechamento do backdrop.
    mouse_area(panel_box).into()
}

/// Card de erro tipado do servidor de status na ilha (`theme::ERROR`).
fn error_card<'a>(error: &IslandServerError) -> Element<'a, IslandMsg> {
    let line = row![
        status_dot(COLOR_ERROR),
        container(
            text(error.display_message())
                .size(theme::SIZE_LABEL)
                .color(COLOR_TEXT2)
        )
        .width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(line)
        .width(Fill)
        .padding([8, 10])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(COLOR_PANEL2)),
            border: Border {
                color: COLOR_ERROR,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Cabeçalho de seção do painel (`_Header` em `agent_island_panel.dart`).
fn section_header<'a>(label: &str, color: Color) -> Element<'a, IslandMsg> {
    container(
        text(label.to_uppercase())
            .font(theme::FONT_TITLE)
            .size(11)
            .color(color),
    )
    .padding([5, 8])
    .into()
}

/// Bolinha de estado à esquerda do nome do agente (`_Leading` em
/// `agent_island_panel.dart`).
fn status_dot<'a>(color: Color) -> Element<'a, IslandMsg> {
    let dot =
        container(Space::new().width(7.0).height(7.0)).style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            border: Border {
                radius: 3.5.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    container(dot)
        .width(Length::Fixed(16.0))
        .height(Length::Fixed(16.0))
        .center_x(Length::Fixed(16.0))
        .center_y(Length::Fixed(16.0))
        .into()
}

/// Linha com nome do agente e workspace (`_Title` em `agent_island_panel.dart`).
fn title_row<'a>(title: &str, workspace: &str, compact: bool) -> Element<'a, IslandMsg> {
    let title_size = if compact { 12.5 } else { 13.0 };
    let title_color = if compact { COLOR_TEXT2 } else { COLOR_TEXT };
    let mut r =
        row![container(text(title.to_string()).size(title_size).color(title_color)).width(Fill)]
            .align_y(Alignment::Center);

    if !workspace.is_empty() {
        r = r.push(
            container(text(workspace.to_string()).size(11.5).color(COLOR_TEXT3)).padding([0, 4]),
        );
    }
    r.into()
}

/// Linha clicável de agente nas seções "Esperando você", "Terminaram" e
/// "Trabalhando" (`_AgentRow` em `agent_island_panel.dart`).
fn agent_row<'a>(
    agent: &IslandAgent,
    color: Color,
    show_go_to_tab: bool,
    compact: bool,
) -> Element<'a, IslandMsg> {
    let mut line = row![
        status_dot(color),
        title_row(&agent.name, &agent.workspace, compact),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    if show_go_to_tab {
        line = line.push(text(labels::ACTION_GO_TO_TAB).size(12).color(COLOR_WARN));
    }

    let v_pad = if compact { 4 } else { 7 };
    button(line)
        .width(Fill)
        .padding([v_pad, 8])
        .on_press(IslandMsg::RevealTab(agent.tab_id.clone()))
        .style(|_: &Theme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(Background::Color(COLOR_PANEL3))
                }
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: COLOR_TEXT,
                border: Border {
                    radius: 7.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

/// Card de pedido de permissão na seção "Pedem permissão" (`_AskCard` em
/// `agent_island_panel.dart`).
fn ask_card<'a>(agent: &IslandAgent) -> Element<'a, IslandMsg> {
    let header = row![
        status_dot(COLOR_WARN),
        title_row(&agent.name, &agent.workspace, false),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let (tool_line, always_rule) = match &agent.ask {
        Some(ask) => (
            labels::format_tool_line(&ask.tool_name, &ask.input_summary),
            ask.always_rule.clone(),
        ),
        None => (agent.name.clone(), None),
    };

    let code_box = container(
        text(tool_line)
            .font(theme::FONT_MONO)
            .size(theme::SIZE_LABEL)
            .color(COLOR_TEXT2),
    )
    .width(Fill)
    .padding([6, 8])
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(COLOR_PANEL3)),
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        ..Default::default()
    });

    let mut actions = row![action_button(
        labels::ACTION_ALLOW,
        IslandMsg::DecidePermission {
            tab_id: agent.tab_id.clone(),
            decision: PermissionDecision::Allow,
        },
        AskButtonKind::Primary,
    )]
    .spacing(6)
    .align_y(Alignment::Center);

    if always_rule.is_some() {
        actions = actions.push(action_button(
            labels::ACTION_ALWAYS,
            IslandMsg::DecidePermission {
                tab_id: agent.tab_id.clone(),
                decision: PermissionDecision::AllowAlways,
            },
            AskButtonKind::Outline,
        ));
    }

    actions = actions
        .push(action_button(
            labels::ACTION_DENY,
            IslandMsg::DecidePermission {
                tab_id: agent.tab_id.clone(),
                decision: PermissionDecision::Deny,
            },
            AskButtonKind::Outline,
        ))
        .push(action_button(
            labels::ACTION_IN_TERMINAL,
            IslandMsg::DecidePermission {
                tab_id: agent.tab_id.clone(),
                decision: PermissionDecision::Pass,
            },
            AskButtonKind::Ghost,
        ));

    container(column![header, code_box, actions].spacing(8))
        .width(Fill)
        .padding(10)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(COLOR_PANEL2)),
            border: Border {
                color: theme::WARN_BORDER,
                width: 1.0,
                radius: 9.0.into(),
            },
            ..Default::default()
        })
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AskButtonKind {
    Primary,
    Outline,
    Ghost,
}

fn action_button<'a>(
    label: &'static str,
    msg: IslandMsg,
    kind: AskButtonKind,
) -> Element<'a, IslandMsg> {
    button(text(label).size(theme::SIZE_LABEL))
        .width(Shrink)
        .padding([4, 10])
        .on_press(msg)
        .style(move |_: &Theme, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            match kind {
                AskButtonKind::Primary => button::Style {
                    background: Some(Background::Color(if hovered {
                        theme::ACCENT_HOVER
                    } else {
                        COLOR_ACCENT
                    })),
                    text_color: theme::ON_ACCENT,
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                AskButtonKind::Outline => button::Style {
                    background: Some(Background::Color(if hovered {
                        COLOR_PANEL3
                    } else {
                        COLOR_PANEL2
                    })),
                    text_color: COLOR_TEXT,
                    border: Border {
                        color: COLOR_BORDER2,
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                },
                AskButtonKind::Ghost => button::Style {
                    background: if hovered {
                        Some(Background::Color(COLOR_PANEL3))
                    } else {
                        None
                    },
                    text_color: COLOR_TEXT2,
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            }
        })
        .into()
}
