//! Barra inferior, seletores e medidor de uso do composer.

use iced::widget::canvas::{self, Path, Stroke};
use iced::widget::{button, container, row, text, tooltip, Space};
use iced::{
    mouse, Alignment, Background, Border, Element, Fill, Length, Point, Radians, Rectangle,
    Renderer, Theme,
};

use crate::anchored::Anchored;
use crate::app_menu::{self, Entry, Item};
use crate::chat_pane::{ChatMsg, ChatPane, ComposerMenu};
use crate::{labels, theme};

pub(super) fn view_bottom_bar(pane: &ChatPane) -> Element<'_, ChatMsg> {
    let attach = icon_action(labels::ICON_PLUS, labels::CHAT_ATTACH, ChatMsg::Attach);
    let peers = icon_action(
        labels::ICON_FORUM,
        labels::CHAT_PEER_PICKER,
        ChatMsg::TogglePeerPicker,
    );
    let selected_mode = mode_label(pane.mode());
    let mode_content = row![
        text(selected_mode)
            .font(theme::FONT_BODY)
            .size(theme::CHAT_MODE_LABEL_SIZE)
            .color(if pane.mode() == "bypassPermissions" {
                theme::palette().warn
            } else {
                theme::palette().text2
            }),
        Space::new().width(2.0),
        text(labels::ICON_EXPAND_MORE)
            .font(theme::FONT_ICON)
            .size(theme::CHAT_EFFORT_ICON_SIZE)
            .color(theme::palette().text3),
    ]
    .align_y(Alignment::Center);
    let mode = picker(
        mode_content.into(),
        pane.composer_menu() == Some(ComposerMenu::Mode),
        ComposerMenu::Mode,
        app_menu::menu(
            [
                ("default", labels::CHAT_MODE_DEFAULT),
                ("acceptEdits", labels::CHAT_MODE_EDITS),
                ("plan", labels::CHAT_MODE_PLAN),
                ("bypassPermissions", labels::CHAT_MODE_IGNORE),
            ]
            .into_iter()
            .map(|(value, label)| {
                Item::Entry(
                    Entry::new(label, ChatMsg::SetMode(value.into()))
                        .selected(value == pane.mode()),
                )
            })
            .collect(),
            0.0,
        ),
        theme::CHAT_MODE_PADDING,
    );
    let mut bar = row![
        attach,
        Space::new().width(2.0),
        peers,
        Space::new().width(2.0),
        mode,
        Space::new().width(Fill),
    ]
    .align_y(Alignment::Center);
    let model_value = pane.model().unwrap_or_default();
    let selected_model = pane
        .models()
        .iter()
        .find(|model| model.value == model_value || model.resolved.as_deref() == Some(model_value))
        .map(|model| model.display_name.clone())
        .unwrap_or_else(|| short_model_name(pane.model()));
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
                rotation: 0.0,
                track: true,
            })
            .width(Length::Fixed(theme::CHAT_USAGE_RING_SIZE))
            .height(Length::Fixed(theme::CHAT_USAGE_RING_SIZE)),
        )
        .padding(theme::CHAT_ICON_ACTION_PADDING)
        .on_press(ChatMsg::ToggleUsage)
        .style(theme::ghost_button);
        bar = bar.push(tooltip(
            ring,
            crate::pane_chrome::tip_bubble(labels::CHAT_USAGE),
            tooltip::Position::Top,
        ));
    }
    bar = bar.push(Space::new().width(2.0));
    let efforts = pane.efforts();
    if !efforts.is_empty() {
        let mut effort_children = row![text(labels::ICON_SPEED)
            .font(theme::FONT_ICON)
            .size(theme::CHAT_EFFORT_ICON_SIZE)
            .color(theme::palette().text3),]
        .align_y(Alignment::Center);
        if let Some(effort) = pane.effort() {
            effort_children = effort_children.push(Space::new().width(4.0)).push(
                text(effort_label(effort))
                    .font(theme::FONT_BODY)
                    .size(theme::CHAT_EFFORT_LABEL_SIZE)
                    .color(theme::palette().text3),
            );
        }
        let items = efforts
            .iter()
            .map(|effort| {
                Item::Entry(
                    Entry::new(effort_label(effort), ChatMsg::SetEffort(effort.clone()))
                        .selected(pane.effort() == Some(effort.as_str())),
                )
            })
            .collect();
        let effort = tooltip(
            picker(
                effort_children.into(),
                pane.composer_menu() == Some(ComposerMenu::Effort),
                ComposerMenu::Effort,
                app_menu::menu(items, 0.0),
                theme::CHAT_EFFORT_PADDING,
            ),
            crate::pane_chrome::tip_bubble(format!(
                "{}{}\n{}",
                labels::CHAT_EFFORT_TOOLTIP,
                pane.effort()
                    .map(effort_label)
                    .unwrap_or_else(|| labels::CHAT_EFFORT_AUTO.to_string()),
                labels::CHAT_EFFORT_HINT,
            )),
            tooltip::Position::Top,
        );
        bar = bar.push(effort);
    }
    let model_items = pane
        .models()
        .iter()
        .map(|model| {
            Item::Entry(
                Entry::new(&model.display_name, ChatMsg::SetModel(model.value.clone()))
                    .description(model.description.clone())
                    .selected(
                        model.value == model_value
                            || model.resolved.as_deref() == Some(model_value),
                    ),
            )
        })
        .collect();
    let model = picker(
        row![
            text(selected_model)
                .font(theme::FONT_BODY)
                .size(theme::CHAT_MODE_LABEL_SIZE)
                .color(theme::palette().text2),
            Space::new().width(2.0),
            text(labels::ICON_EXPAND_MORE)
                .font(theme::FONT_ICON)
                .size(theme::CHAT_EFFORT_ICON_SIZE)
                .color(theme::palette().text3),
        ]
        .align_y(Alignment::Center)
        .into(),
        pane.composer_menu() == Some(ComposerMenu::Model),
        ComposerMenu::Model,
        app_menu::menu(model_items, 0.0),
        theme::CHAT_MODE_PADDING,
    );
    bar = bar.push(if pane.cost_usd() > 0.0 {
        tooltip(
            model,
            crate::pane_chrome::tip_bubble(labels::format_chat_cost(pane.cost_usd())),
            tooltip::Position::Top,
        )
        .into()
    } else {
        model
    });

    if pane.is_busy() {
        let waiting_permission = pane
            .items()
            .iter()
            .any(|item| matches!(item, crate::chat_pane::ChatItem::Permission { .. }));
        bar = bar
            .push(Space::new().width(theme::CHAT_BUSY_GAP))
            .push(tooltip(
                canvas::Canvas::new(UsageRing {
                    fraction: 0.28,
                    warning: waiting_permission,
                    rotation: pane.spinner_rotation(),
                    track: false,
                })
                .width(Length::Fixed(theme::CHAT_BUSY_INDICATOR_SIZE))
                .height(Length::Fixed(theme::CHAT_BUSY_INDICATOR_SIZE)),
                crate::pane_chrome::tip_bubble(if waiting_permission {
                    labels::CHAT_WAITING_PERMISSION
                } else {
                    labels::CHAT_WORKING
                }),
                tooltip::Position::Top,
            ));
    }

    container(bar)
        .padding(iced::Padding {
            top: 0.0,
            right: theme::CHAT_BOTTOM_GAP,
            bottom: 0.0,
            left: theme::CHAT_BOTTOM_GAP,
        })
        .width(Fill)
        .into()
}

fn mode_label(mode: &str) -> &'static str {
    match mode {
        "acceptEdits" => labels::CHAT_MODE_EDITS,
        "plan" => labels::CHAT_MODE_PLAN,
        "bypassPermissions" => labels::CHAT_MODE_IGNORE,
        _ => labels::CHAT_MODE_DEFAULT,
    }
}

fn icon_action<'a>(
    glyph: &'static str,
    tip: &'static str,
    message: ChatMsg,
) -> Element<'a, ChatMsg> {
    tooltip(
        button(
            text(glyph)
                .font(theme::FONT_ICON)
                .size(theme::CHAT_ICON_ACTION_SIZE)
                .color(theme::palette().text2),
        )
        .padding(theme::CHAT_ICON_ACTION_PADDING)
        .on_press(message)
        .style(theme::ghost_button),
        crate::pane_chrome::tip_bubble(tip),
        tooltip::Position::Top,
    )
    .into()
}

fn picker<'a>(
    content: Element<'a, ChatMsg>,
    open: bool,
    menu_type: ComposerMenu,
    menu: Element<'a, ChatMsg>,
    padding: [f32; 2],
) -> Element<'a, ChatMsg> {
    let trigger = button(content)
        .padding(padding)
        .on_press(ChatMsg::OpenComposerMenu(menu_type))
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| Background::Color(theme::palette().panel3)),
            text_color: theme::palette().text2,
            border: Border {
                radius: theme::CHAT_MODE_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        });
    Anchored::new(trigger, open.then_some(menu), ChatMsg::CloseComposerMenu).into()
}

fn effort_label(value: &str) -> String {
    match value {
        "low" => labels::CHAT_EFFORT_LOW,
        "medium" => labels::CHAT_EFFORT_MEDIUM,
        "high" => labels::CHAT_EFFORT_HIGH,
        "xhigh" => labels::CHAT_EFFORT_XHIGH,
        "max" => labels::CHAT_EFFORT_MAX,
        _ => return value.to_string(),
    }
    .to_string()
}

fn short_model_name(id: Option<&str>) -> String {
    let Some(id) = id.filter(|id| !id.is_empty()) else {
        return "Claude".into();
    };
    for index in 0..id.len() {
        if !id.is_char_boundary(index) {
            continue;
        }
        for family in ["opus", "sonnet", "haiku", "fable"] {
            let Some(version) = id[index..]
                .strip_prefix(family)
                .and_then(|rest| rest.strip_prefix('-'))
            else {
                continue;
            };
            let major_len = version.bytes().take_while(u8::is_ascii_digit).count();
            if !(1..=2).contains(&major_len)
                || version
                    .as_bytes()
                    .get(major_len)
                    .is_some_and(u8::is_ascii_digit)
            {
                continue;
            }
            let major = &version[..major_len];
            let rest = &version[major_len..];
            let minor = rest.strip_prefix('-').and_then(|rest| {
                let len = rest.bytes().take_while(u8::is_ascii_digit).count();
                ((1..=2).contains(&len)
                    && !rest.as_bytes().get(len).is_some_and(u8::is_ascii_digit))
                .then(|| &rest[..len])
            });
            let title = format!("{}{}", family[..1].to_ascii_uppercase(), &family[1..]);
            return minor
                .map(|minor| format!("{title} {major}.{minor}"))
                .unwrap_or_else(|| format!("{title} {major}"));
        }
    }
    id.to_string()
}

#[cfg(test)]
#[test]
fn short_model_name_matches_dart_golden_cases() {
    let cases = [
        (None, "Claude"),
        (Some(""), "Claude"),
        (Some("claude-opus-4-5-20251101"), "Opus 4.5"),
        (Some("claude-sonnet-4"), "Sonnet 4"),
        (Some("prefix-haiku-3-2-suffix"), "Haiku 3.2"),
        (Some("claude-fable-12-3"), "Fable 12.3"),
        (Some("claude-opus-123"), "claude-opus-123"),
        (Some("claude-haiku-4-123"), "Haiku 4"),
        (Some("custom-model"), "custom-model"),
    ];
    for (input, expected) in cases {
        assert_eq!(short_model_name(input).as_str(), expected, "{input:?}");
    }
}

struct UsageRing {
    fraction: f32,
    warning: bool,
    rotation: f32,
    track: bool,
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
        if self.track {
            frame.stroke(
                &Path::circle(center, radius),
                Stroke::default()
                    .with_width(2.0)
                    .with_color(theme::palette().border2),
            );
        }
        let arc = Path::new(|builder| {
            builder.arc(canvas::path::Arc {
                center,
                radius,
                start_angle: Radians(-std::f32::consts::FRAC_PI_2 + self.rotation),
                end_angle: Radians(
                    -std::f32::consts::FRAC_PI_2
                        + self.rotation
                        + self.fraction * std::f32::consts::TAU,
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
