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
    color_accent, color_border2, color_error, color_online, color_panel, color_panel2,
    color_panel3, color_text, color_text2, color_text3, color_warn, IslandAgent, IslandAgentState,
    IslandMsg, PermissionDecision, COLOR_SHADOW, PANEL_WIDTH,
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
        sections.push(section_header(labels::SECTION_STATUS_ERROR, color_error()));
        sections.push(error_card(err));
    }

    if !asking.is_empty() {
        sections.push(section_header(labels::SECTION_ASKS, color_warn()));
        for agent in asking {
            sections.push(ask_card(agent));
        }
    }

    if !waiting.is_empty() {
        sections.push(section_header(labels::SECTION_WAITING, color_warn()));
        for agent in waiting {
            sections.push(agent_row(agent, color_warn(), true, false));
        }
    }

    if !finished.is_empty() {
        sections.push(section_header(labels::SECTION_FINISHED, color_online()));
        for agent in finished {
            sections.push(agent_row(agent, color_online(), false, false));
        }
    }

    if !working.is_empty() {
        sections.push(section_header(labels::SECTION_WORKING, color_accent()));
        for agent in working {
            sections.push(agent_row(agent, color_accent(), false, true));
        }
    }

    if sections.is_empty() {
        sections.push(
            container(text(labels::ISLAND_EMPTY).size(13).color(color_text3()))
                .padding(8)
                .into(),
        );
    }

    let content = scrollable(column(sections).spacing(2).padding(8));

    let panel_box = container(content)
        .width(Length::Fixed(PANEL_WIDTH))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(color_panel())),
            border: Border {
                color: color_border2(),
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

/// Card de erro tipado do servidor de status na ilha (`theme::palette().error`).
fn error_card<'a>(error: &IslandServerError) -> Element<'a, IslandMsg> {
    let line = row![
        status_dot(color_error()),
        container(
            text(error.display_message())
                .size(theme::SIZE_LABEL)
                .color(color_text2())
        )
        .width(Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(line)
        .width(Fill)
        .padding([8, 10])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(color_panel2())),
            border: Border {
                color: color_error(),
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
    let title_color = if compact { color_text2() } else { color_text() };
    let mut r =
        row![container(text(title.to_string()).size(title_size).color(title_color)).width(Fill)]
            .align_y(Alignment::Center);

    if !workspace.is_empty() {
        r = r.push(
            container(text(workspace.to_string()).size(11.5).color(color_text3())).padding([0, 4]),
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
        line = line.push(text(labels::ACTION_GO_TO_TAB).size(12).color(color_warn()));
    }

    let v_pad = if compact { 4 } else { 7 };
    button(line)
        .width(Fill)
        .padding([v_pad, 8])
        .on_press(IslandMsg::RevealTab(agent.tab_id.clone()))
        .style(|_: &Theme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(Background::Color(color_panel3()))
                }
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: color_text(),
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
        status_dot(color_warn()),
        title_row(&agent.name, &agent.workspace, false),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut card = column![header].spacing(8);
    let mut actions = row![].spacing(6).align_y(Alignment::Center);
    let terminal = if let Some(ask) = &agent.ask {
        let tool_line = labels::format_tool_line(&ask.tool_name, &ask.input_summary);
        card = card.push(
            container(
                text(tool_line)
                    .font(theme::FONT_MONO)
                    .size(theme::SIZE_LABEL)
                    .color(color_text2()),
            )
            .width(Fill)
            .padding([6, 8])
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(color_panel3())),
                border: Border {
                    radius: 6.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }),
        );
        let decide = |decision| IslandMsg::DecidePermission {
            ask_id: ask.id.clone(),
            decision,
        };
        actions = actions.push(action_button(
            labels::ACTION_ALLOW,
            decide(PermissionDecision::Allow),
            AskButtonKind::Primary,
        ));
        if let Some(rule) = &ask.always_rule {
            card = card.push(text(format!("Sempre: {rule}")).size(theme::SIZE_LABEL));
            actions = actions.push(action_button(
                labels::ACTION_ALWAYS,
                decide(PermissionDecision::AllowAlways),
                AskButtonKind::Outline,
            ));
        }
        actions = actions.push(action_button(
            labels::ACTION_DENY,
            decide(PermissionDecision::Deny),
            AskButtonKind::Outline,
        ));
        decide(PermissionDecision::Pass)
    } else {
        IslandMsg::RevealTab(agent.tab_id.clone())
    };
    card = card.push(actions.push(action_button(
        labels::ACTION_IN_TERMINAL,
        terminal,
        AskButtonKind::Ghost,
    )));

    container(card)
        .width(Fill)
        .padding(10)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color_panel2())),
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
                        color_accent()
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
                        color_panel3()
                    } else {
                        color_panel2()
                    })),
                    text_color: color_text(),
                    border: Border {
                        color: color_border2(),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                },
                AskButtonKind::Ghost => button::Style {
                    background: if hovered {
                        Some(Background::Color(color_panel3()))
                    } else {
                        None
                    },
                    text_color: color_text2(),
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
