//! Componente visual do composer do chat (`chat_composer.dart` do Flutter).
//!
//! Exibe a faixa de contexto (pasta + branch), a caixa de texto com atalho
//! `Enter` para envio e botão de Enviar/Parar, e a barra inferior de status.

use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::{button, column, container, pick_list, row, text, text_input, tooltip, Space};
use iced::{mouse, Point, Radians, Rectangle, Renderer};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

use crate::chat_pane::{ChatMsg, ChatPane};
use crate::labels;
use crate::theme;

/// Renderiza o composer completo na parte inferior do [`ChatPane`].
pub fn view(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let context_strip = view_context_strip(pane);
    let input_box = view_input_box(pane);
    let bottom_bar = view_bottom_bar(pane);

    let mut content = column![context_strip, Space::new().height(6.0)];
    if !pane.slash_matches().is_empty() {
        let mut commands = column![].spacing(2);
        for (index, command) in pane.slash_matches().into_iter().enumerate() {
            let label = if command.argument_hint.is_empty() {
                format!("/{}  {}", command.name, command.description)
            } else {
                format!(
                    "/{} {}  {}",
                    command.name, command.argument_hint, command.description
                )
            };
            let item = button(text(label).size(theme::SIZE_LABEL))
                .on_press(ChatMsg::SelectCommand(format!("/{} ", command.name)))
                .style(theme::ghost_button);
            commands = commands.push(if index == pane.slash_index() {
                item.style(|_: &Theme, _status| button::Style {
                    background: Some(Background::Color(theme::palette().panel3)),
                    text_color: theme::palette().text,
                    border: Border {
                        radius: theme::RADIUS_BUTTON.into(),
                        ..Default::default()
                    },
                    ..button::Style::default()
                })
            } else {
                item
            });
        }
        content = content.push(container(commands).width(Fill).padding(6));
    }

    content
        .push(input_box)
        .push(Space::new().height(4.0))
        .push(bottom_bar)
        .push(if pane.peer_picker_open() {
            view_peer_picker(pane)
        } else {
            Space::new().height(0.0).into()
        })
        .push(if pane.usage_open() {
            view_usage_details(pane)
        } else {
            Space::new().height(0.0).into()
        })
        .width(Fill)
        .into()
}

fn view_context_strip(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut strip_row = row![text(pane.folder_label())
        .font(theme::FONT_MONO)
        .size(theme::SIZE_TAB)
        .color(theme::palette().text3)]
    .align_y(Alignment::Center);

    if let Some(branch) = pane.branch().filter(|b| !b.is_empty()) {
        strip_row = strip_row.push(Space::new().width(10.0)).push(
            text(branch)
                .font(theme::FONT_MONO)
                .size(theme::SIZE_TAB)
                .color(theme::palette().text3),
        );
    }

    container(strip_row)
        .width(Fill)
        .padding([9, 12])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn view_input_box(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut input = text_input(labels::CHAT_PLACEHOLDER, pane.input())
        .on_input(ChatMsg::InputChanged)
        .font(theme::FONT_BODY)
        .size(theme::SIZE_BODY)
        .padding([6, 8])
        .style(|_: &Theme, _status| text_input::Style {
            background: Background::Color(theme::palette().panel2),
            border: Border::default(),
            icon: theme::palette().text3,
            placeholder: theme::palette().text4,
            value: theme::palette().text,
            selection: theme::palette().accent_soft,
        });

    if !pane.is_busy() && pane.slash_matches().is_empty() {
        input = input.on_submit(ChatMsg::Send);
    }

    let action_btn = if pane.is_busy() {
        view_stop_button()
    } else {
        view_send_button(!pane.input().trim().is_empty())
    };

    let mut controls = row![input, Space::new().width(4.0), action_btn]
        .width(Fill)
        .align_y(Alignment::Center);
    if !pane.attachments().is_empty() {
        let names =
            pane.attachments()
                .iter()
                .enumerate()
                .fold(row![], |row, (index, attachment)| {
                    row.push(
                        button(text(format!("{} ×", attachment.name)).size(theme::SIZE_TAB))
                            .on_press(ChatMsg::RemoveAttachment(index))
                            .style(theme::ghost_button),
                    )
                });
        controls = row![column![names, controls].spacing(3)];
    }
    container(controls)
        .width(Fill)
        .padding([4, 6])
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                color: theme::palette().border2,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn view_send_button<'a>(enabled: bool) -> Element<'a, ChatMsg> {
    let mut send = button(
        text(labels::ICON_KEYBOARD_RETURN)
            .font(theme::FONT_ICON)
            .size(theme::SIZE_ICON)
            .color(theme::palette().text2),
    )
    .padding(5)
    .style(theme::ghost_button);
    if enabled {
        send = send.on_press(ChatMsg::Send);
    }
    send.into()
}

fn view_stop_button<'a>() -> Element<'a, ChatMsg> {
    button(
        text(labels::ICON_STOP)
            .font(theme::FONT_ICON)
            .size(theme::SIZE_ICON)
            .color(theme::palette().text2),
    )
    .padding(5)
    .on_press(ChatMsg::Cancel)
    .style(theme::ghost_button)
    .into()
}

fn view_bottom_bar(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let modes = vec![
        labels::CHAT_MODE_ASK.to_string(),
        labels::CHAT_MODE_EDITS.to_string(),
        labels::CHAT_MODE_PLAN.to_string(),
        labels::CHAT_MODE_IGNORE.to_string(),
    ];
    let selected = Some(
        match pane.mode() {
            "acceptEdits" => labels::CHAT_MODE_EDITS,
            "plan" => labels::CHAT_MODE_PLAN,
            "bypassPermissions" => labels::CHAT_MODE_IGNORE,
            _ => labels::CHAT_MODE_ASK,
        }
        .to_string(),
    );
    let mode_picker = iced::widget::pick_list(modes, selected, |mode| {
        ChatMsg::SetMode(
            match mode.as_str() {
                "Aceitar edições" => "acceptEdits",
                "Planejar" => "plan",
                "Ignorar permissões" => "bypassPermissions",
                _ => "default",
            }
            .to_string(),
        )
    })
    .width(Length::Shrink)
    .padding([0, 2])
    .text_size(theme::SIZE_LABEL)
    .style(|_: &Theme, status| {
        chat_pick_list_style(status, pane.mode() == "bypassPermissions", true)
    });
    let attach = tooltip(
        button(
            text(labels::ICON_PLUS)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON),
        )
        .padding(5)
        .on_press(ChatMsg::Attach)
        .style(theme::ghost_button),
        crate::pane_chrome::tip_bubble(labels::CHAT_ATTACH),
        tooltip::Position::Top,
    );
    let peers = tooltip(
        button(
            text(labels::ICON_FORUM)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON),
        )
        .padding(5)
        .on_press(ChatMsg::TogglePeerPicker)
        .style(theme::ghost_button),
        crate::pane_chrome::tip_bubble(labels::CHAT_PEER_PICKER),
        tooltip::Position::Top,
    );
    let mut bar = row![
        attach,
        Space::new().width(2.0),
        peers,
        Space::new().width(2.0),
        mode_picker,
        Space::new().width(Fill),
    ]
    .height(Length::Fixed(22.0))
    .align_y(Alignment::Center);
    let model_choices = if pane.models().is_empty() {
        vec!["Claude".to_string()]
    } else {
        pane.models()
            .iter()
            .map(|model| model.display_name.clone())
            .collect()
    };
    let model_value = pane.model().unwrap_or_default();
    let selected_model = pane
        .models()
        .iter()
        .find(|model| model.value == model_value || model.resolved.as_deref() == Some(model_value))
        .map(|model| model.display_name.clone())
        .unwrap_or_else(|| short_model_name(pane.model()));
    let efforts = pane
        .models()
        .iter()
        .find(|m| {
            m.value == pane.model().unwrap_or_default() || m.resolved.as_deref() == pane.model()
        })
        .map(|m| m.efforts.clone())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| {
            vec![
                "low".into(),
                "medium".into(),
                "high".into(),
                "xhigh".into(),
                "max".into(),
            ]
        });
    let selected_effort = Some(effort_label(pane.effort().unwrap_or("high")).to_string());
    let effort_choices = efforts
        .iter()
        .map(|effort| effort_label(effort).to_string())
        .collect::<Vec<_>>();
    {
        let fraction = pane.usage_fraction().unwrap_or(0.0).clamp(0.0, 1.0);
        let warning = fraction >= 0.8
            || pane
                .usage()
                .is_some_and(|u| u.limits.iter().any(|l| l.warning));
        let ring = button(
            canvas::Canvas::new(UsageRing {
                fraction: fraction.clamp(0.0, 1.0),
                warning,
            })
            .width(Length::Fixed(15.0))
            .height(Length::Fixed(15.0)),
        )
        .padding(5)
        .on_press(ChatMsg::ToggleUsage)
        .style(theme::ghost_button);
        bar = bar.push(tooltip(
            ring,
            crate::pane_chrome::tip_bubble(labels::CHAT_USAGE),
            tooltip::Position::Top,
        ));
    }
    bar = bar.push(
        text(labels::ICON_SPEED)
            .font(theme::FONT_ICON)
            .size(theme::SIZE_LABEL)
            .color(theme::palette().text3),
    );
    bar = bar.push(
        iced::widget::pick_list(effort_choices, selected_effort, move |label| {
            efforts
                .iter()
                .find(|effort| effort_label(effort) == label)
                .cloned()
                .map(ChatMsg::SetEffort)
                .unwrap_or(ChatMsg::RefreshUsage)
        })
        .width(Length::Shrink)
        .padding([0, 2])
        .text_size(theme::SIZE_LABEL)
        .style(|_: &Theme, status| chat_pick_list_style(status, false, false)),
    );
    bar = bar.push(
        iced::widget::pick_list(model_choices, Some(selected_model), |name| {
            pane.models()
                .iter()
                .find(|model| model.display_name == name)
                .map(|model| ChatMsg::SetModel(model.value.clone()))
                .unwrap_or(ChatMsg::UsageRefresh)
        })
        .width(Length::Shrink)
        .padding([0, 2])
        .text_size(theme::SIZE_LABEL)
        .style(|_: &Theme, status| chat_pick_list_style(status, false, true)),
    );

    if pane.is_busy() {
        bar = bar.push(Space::new().width(10.0)).push(
            text(labels::CHAT_WORKING)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().accent_text),
        );
    }

    container(bar).padding([0, 6]).width(Fill).into()
}

fn chat_pick_list_style(
    status: pick_list::Status,
    warning: bool,
    show_handle: bool,
) -> pick_list::Style {
    let p = theme::palette();
    pick_list::Style {
        text_color: if warning { p.warn } else { p.text2 },
        placeholder_color: p.text3,
        handle_color: if show_handle {
            p.text3
        } else {
            iced::Color::TRANSPARENT
        },
        background: (if matches!(status, pick_list::Status::Hovered) {
            p.panel3
        } else {
            iced::Color::TRANSPARENT
        })
        .into(),
        border: Border {
            color: p.bg,
            width: 0.0,
            radius: 0.0.into(),
        },
    }
}

fn effort_label(value: &str) -> &str {
    match value {
        "low" => "Baixo",
        "medium" => "Médio",
        "high" => "Alto",
        "xhigh" => "Muito alto",
        "max" => "Máximo",
        _ => "Padrão",
    }
}

fn short_model_name(id: Option<&str>) -> String {
    let Some(id) = id.filter(|id| !id.is_empty()) else {
        return "Claude".into();
    };
    let parts = id.split('-').collect::<Vec<_>>();
    if parts.len() >= 3 && parts.first() == Some(&"claude") {
        let family = parts.get(1).copied().unwrap_or_default();
        let version = parts.get(2).copied().unwrap_or_default();
        if ["opus", "sonnet", "haiku", "fable"].contains(&family)
            && version.chars().all(|c| c.is_ascii_digit())
        {
            let minor = parts
                .get(3)
                .copied()
                .filter(|minor| minor.chars().all(|c| c.is_ascii_digit()));
            let family = family
                .chars()
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + family.get(1..).unwrap_or(""))
                .unwrap_or_default();
            return minor
                .map(|minor| format!("{family} {version}.{minor}"))
                .unwrap_or_else(|| format!("{family} {version}"));
        }
    }
    id.to_string()
}

struct UsageRing {
    fraction: f32,
    warning: bool,
}

impl canvas::Program<ChatMsg> for UsageRing {
    type State = ();
    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = 6.0;
        frame.stroke(
            &Path::circle(center, radius),
            Stroke::default()
                .with_width(2.0)
                .with_color(theme::palette().border2),
        );
        let arc = Path::new(|builder| {
            builder.arc(canvas::path::Arc {
                center,
                radius,
                start_angle: Radians(-std::f32::consts::FRAC_PI_2),
                end_angle: Radians(
                    -std::f32::consts::FRAC_PI_2 + self.fraction * std::f32::consts::TAU,
                ),
            })
        });
        frame.stroke(
            &arc,
            Stroke::default()
                .with_width(2.2)
                .with_color(if self.warning {
                    theme::palette().warn
                } else {
                    theme::palette().accent_text
                }),
        );
        vec![frame.into_geometry()]
    }
}

fn view_peer_picker(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let mut list = column![text(labels::CHAT_PEER_PICKER).size(theme::SIZE_LABEL)];
    let sessions = pane.peer_sessions();
    if sessions.is_empty() {
        list = list.push(text(labels::CHAT_NO_PEER_SESSIONS).size(theme::SIZE_TAB));
    }
    for session in sessions {
        list = list.push(
            button(
                text(format!(
                    "{} · {} · {}",
                    session.title,
                    gsh_core::tabs::basename(&session.project),
                    time_ago(session.last_seen_ms)
                ))
                .size(theme::SIZE_TAB),
            )
            .on_press(ChatMsg::PickPeerSession(session.id))
            .style(theme::ghost_button),
        );
    }
    list = list.push(
        button(text(labels::CHAT_LIST_ALL_SESSIONS).size(theme::SIZE_TAB))
            .on_press(ChatMsg::PickPeerSession(String::new()))
            .style(theme::ghost_button),
    );
    container(list.spacing(3)).padding(8).width(Fill).into()
}

fn time_ago(at_ms: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let seconds = now.saturating_sub(at_ms) / 1000;
    if seconds < 60 {
        "agora".into()
    } else if seconds < 3600 {
        format!("há {} min", seconds / 60)
    } else {
        format!("há {} h", seconds / 3600)
    }
}

fn view_usage_details(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let (tokens, window) = pane.context();
    let context = if window > 0 {
        format!(
            "{} · {}%",
            labels::format_chat_context(tokens, window),
            (tokens.saturating_mul(100) / window).min(100)
        )
    } else {
        labels::CHAT_CONTEXT_UNAVAILABLE.to_string()
    };
    let mut rows = column![
        text(
            pane.usage()
                .and_then(|u| u.plan.as_deref())
                .unwrap_or(labels::CHAT_USAGE)
        )
        .font(theme::FONT_BODY_MEDIUM)
        .size(theme::SIZE_LABEL),
        text(context)
            .font(theme::FONT_BODY)
            .size(theme::SIZE_TAB)
            .color(theme::palette().text2)
    ];
    if pane.usage().is_none() {
        rows = rows.push(
            text(labels::CHAT_USAGE_UNAVAILABLE)
                .size(theme::SIZE_TAB)
                .color(theme::palette().text3),
        );
    }
    for limit in pane.usage().into_iter().flat_map(|u| &u.limits) {
        let label = match limit.kind {
            gsh_chat::LimitKind::Session => labels::CHAT_USAGE_SESSION,
            gsh_chat::LimitKind::Weekly => labels::CHAT_USAGE_WEEKLY,
            gsh_chat::LimitKind::WeeklyModel => {
                limit.model.as_deref().unwrap_or(labels::CHAT_USAGE_WEEKLY)
            }
            gsh_chat::LimitKind::Other => continue,
        };
        let reset = limit
            .resets_at_ms
            .map(|ms| format!(" · {}", reset_relative(ms)))
            .unwrap_or_default();
        rows = rows.push(
            text(format!("{label}: {}%{}", limit.percent, reset))
                .font(theme::FONT_BODY)
                .size(theme::SIZE_TAB)
                .color(if limit.warning || limit.percent >= 80 {
                    theme::palette().warn
                } else {
                    theme::palette().text2
                }),
        );
    }
    container(rows.spacing(4))
        .padding(12)
        .width(Length::Fixed(300.0))
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                color: theme::palette().border2,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn reset_relative(at_ms: u64) -> String {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let at = crate::clock::local_time(at_ms);
    let now = crate::clock::local_time(now_ms);
    let time = format!("{:02}:{:02}", at.hour, at.minute);
    if at.year == now.year && at.month == now.month && at.day == now.day {
        format!("{}{time}", labels::CHAT_USAGE_RESET_TODAY)
    } else {
        format!(
            "{}{:02}/{:02} às {time}",
            labels::CHAT_USAGE_RESET_ON,
            at.day,
            at.month
        )
    }
}
