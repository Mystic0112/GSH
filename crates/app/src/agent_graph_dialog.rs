//! Modelo e formulário de criação de agente do grafo.

use std::path::Path;

use iced::widget::{
    button, checkbox, column, container, row, scrollable, svg, text, text_input, tooltip, Space,
};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};

use crate::{labels, pane_close_dialog, theme};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub name: String,
    pub command: String,
    pub cwd: String,
    pub monitor: bool,
    pub maestro: bool,
    pub color: Option<usize>,
    pub role: String,
    pub task: String,
    pub tab: usize,
    pub error: Option<String>,
    pub auto_name: Option<String>,
    pub desktop_apps: Vec<crate::desktop_agent_apps::Integration>,
    pub desktop: Option<crate::desktop_agent_apps::Integration>,
    pub consent: bool,
}

impl Draft {
    pub fn new(root: &Path, desktop_apps: Vec<crate::desktop_agent_apps::Integration>) -> Self {
        Self {
            name: String::new(),
            command: String::new(),
            cwd: root.to_string_lossy().into_owned(),
            monitor: true,
            maestro: false,
            color: None,
            role: String::new(),
            task: String::new(),
            tab: 0,
            error: None,
            auto_name: None,
            desktop_apps,
            desktop: None,
            consent: false,
        }
    }

    pub fn valid(&self, taken: &[String]) -> bool {
        !self.name.trim().is_empty()
            && !taken
                .iter()
                .any(|name| name.eq_ignore_ascii_case(self.name.trim()))
            && if self.desktop.is_some() {
                self.consent
            } else {
                Path::new(self.cwd.trim()).is_dir()
            }
    }

    pub fn quick_start(&mut self, name: &'static str, command: &'static str, taken: &[String]) {
        if self.name.trim().is_empty() || self.auto_name.as_deref() == Some(&self.name) {
            let next = default_name(name, taken);
            self.name.clone_from(&next);
            self.auto_name = Some(next);
        }
        self.command = command.to_owned();
        self.desktop = None;
        self.consent = false;
        self.error = None;
    }

    pub fn pick_desktop(&mut self, index: usize, taken: &[String]) {
        let Some(integration) = self.desktop_apps.get(index).cloned() else {
            return;
        };
        if self.name.trim().is_empty() || self.auto_name.as_deref() == Some(self.name.as_str()) {
            self.name = default_name(integration.app.label(), taken);
            self.auto_name = Some(self.name.clone());
        }
        self.desktop = Some(integration);
        self.consent = false;
    }
}

pub fn default_name(base: &str, taken: &[String]) -> String {
    if !taken.iter().any(|name| name.eq_ignore_ascii_case(base)) {
        return base.to_owned();
    }
    (2_u64..)
        .map(|index| format!("{base} {index}"))
        .find(|candidate| {
            !taken
                .iter()
                .any(|name| name.eq_ignore_ascii_case(candidate))
        })
        .unwrap_or_else(|| base.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message {
    Cancel,
    Create,
    NameChanged(String),
    CommandChanged(String),
    CwdChanged(String),
    RoleChanged(String),
    TaskChanged(String),
    SetTab(usize),
    SetMonitor(bool),
    SetMaestro(bool),
    SetColor(Option<usize>),
    QuickStart(&'static str, &'static str),
    Browse,
    FolderPicked(Option<String>),
    PickDesktop(usize),
    SetConsent(bool),
}

pub fn view<'a>(draft: &'a Draft, taken: &[String], maestro: Option<&str>) -> Element<'a, Message> {
    let can_create = draft.valid(taken);
    let title = text(labels::AGENT_GRAPH_NEW_AGENT)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text)
        .into();
    let mut quick = row![].spacing(8).align_y(Alignment::Center);
    for (name, command, icon) in [
        (
            "Antigravity",
            "agy",
            include_bytes!("../assets/harness_icons/antigravity.svg") as &[u8],
        ),
        (
            "Claude Code",
            "claude",
            include_bytes!("../assets/harness_icons/claudecode.svg") as &[u8],
        ),
        (
            "Codex",
            "codex",
            include_bytes!("../assets/harness_icons/codex.svg") as &[u8],
        ),
        (
            "Cursor",
            "cursor-agent",
            include_bytes!("../assets/harness_icons/cursor.svg") as &[u8],
        ),
        (
            "OpenCode",
            "opencode",
            include_bytes!("../assets/harness_icons/opencode.svg") as &[u8],
        ),
        (
            "Pi",
            "pi",
            include_bytes!("../assets/harness_icons/pi.svg") as &[u8],
        ),
    ] {
        quick = quick.push(quick_button(name, command, Some(icon)));
    }
    quick = quick.push(quick_button(labels::AGENT_GRAPH_SHELL, "", None));
    let mut desktop_apps = row![].spacing(6).align_y(Alignment::Center);
    for (index, integration) in draft.desktop_apps.iter().enumerate() {
        desktop_apps = desktop_apps.push(desktop_button(
            integration,
            draft.desktop.as_ref() == Some(integration),
            index,
        ));
    }
    let mut tabs = row![].spacing(3);
    for (idx, label) in [
        labels::AGENT_GRAPH_DETAILS,
        labels::AGENT_GRAPH_APPEARANCE,
        labels::AGENT_GRAPH_AGENT,
    ]
    .into_iter()
    .enumerate()
    {
        tabs = tabs.push(
            button(text(label).size(theme::SIZE_LABEL))
                .padding([7, 10])
                .on_press(Message::SetTab(idx))
                .style(move |_: &Theme, status| {
                    let active = draft.tab == idx;
                    button::Style {
                        background: Some(Background::Color(if active {
                            theme::palette().panel3
                        } else if matches!(
                            status,
                            button::Status::Hovered | button::Status::Pressed
                        ) {
                            theme::palette().panel2
                        } else {
                            theme::palette().panel
                        })),
                        text_color: if active {
                            theme::palette().text
                        } else {
                            theme::palette().text3
                        },
                        border: Border {
                            radius: theme::RADIUS_BUTTON.into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                }),
        );
    }
    let body: Element<'_, Message> = match draft.tab {
        0 => details(draft, taken, maestro),
        1 => appearance(draft),
        _ => agent(draft),
    };
    let desktop_row: Element<'_, Message> = if draft.desktop_apps.is_empty() {
        Space::new().height(0).into()
    } else {
        column![
            caption("APPS DESKTOP"),
            scrollable(desktop_apps).direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new().width(0).scroller_width(0)
            ))
        ]
        .into()
    };
    let content = column![
        caption(labels::AGENT_GRAPH_QUICK_START),
        quick.width(Fill).wrap().vertical_spacing(8),
        desktop_row,
        Space::new().height(16),
        tabs,
        Space::new().height(10),
        body,
    ]
    .spacing(4)
    .width(Length::Fixed(520.0));
    let actions = row![
        Space::new().width(Fill),
        pane_close_dialog::action(
            labels::DIALOG_CANCEL,
            pane_close_dialog::ButtonKind::Outline,
            Some(Message::Cancel)
        ),
        pane_close_dialog::action(
            labels::HARNESS_CREATE,
            pane_close_dialog::ButtonKind::Primary,
            can_create.then_some(Message::Create)
        ),
    ]
    .spacing(8);
    pane_close_dialog::form(
        title,
        content.into(),
        actions.into(),
        Some(520.0),
        Message::Cancel,
        Message::Cancel,
    )
}

fn details<'a>(draft: &'a Draft, taken: &[String], maestro: Option<&str>) -> Element<'a, Message> {
    if let Some(integration) = &draft.desktop {
        let name = text_input(labels::AGENT_GRAPH_NAME_PLACEHOLDER, &draft.name)
            .on_input(Message::NameChanged)
            .padding(theme::AGENT_GRAPH_FIELD_PADDING)
            .style(graph_input_style);
        let name_error = taken
            .iter()
            .any(|value| value.eq_ignore_ascii_case(draft.name.trim()));
        let snippet = container(
            text(integration.entry())
                .font(theme::FONT_MONO)
                .size(theme::SIZE_BADGE)
                .color(theme::palette().text),
        )
        .padding(10)
        .width(Fill)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel3)),
            border: Border {
                color: theme::palette().border,
                width: 1.0,
                radius: theme::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        });
        let mut content = column![
            caption(labels::AGENT_GRAPH_AGENT_NAME),
            name,
            caption(if name_error {
                labels::AGENT_GRAPH_NAME_TAKEN
            } else {
                ""
            }),
            caption(labels::AGENT_GRAPH_INTEGRATION),
            text(integration.config_path.to_string_lossy())
                .font(theme::FONT_MONO)
                .size(theme::SIZE_BADGE)
                .color(theme::palette().text3),
            snippet,
            text(labels::AGENT_GRAPH_INTEGRATION_HINT)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
            checkbox(draft.consent)
                .label(labels::agent_graph_integration_consent(
                    integration.app.label()
                ))
                .on_toggle(Message::SetConsent)
                .style(checkbox_style),
            toggle_row(
                labels::AGENT_GRAPH_MAESTRO,
                None,
                Some(labels::AGENT_GRAPH_MAESTRO_TOOLTIP),
                draft.maestro,
                Message::SetMaestro(!draft.maestro),
            ),
        ]
        .spacing(6);
        if let Some(error) = &draft.error {
            content = content.push(
                text(error)
                    .size(theme::SIZE_LABEL)
                    .color(theme::palette().error),
            );
        }
        if draft.maestro {
            if let Some(current) = maestro {
                content = content.push(
                    text(labels::agent_graph_maestro_replaces(current))
                        .size(theme::SIZE_LABEL)
                        .color(theme::palette().warn),
                );
            }
        }
        return container(content).width(Fill).into();
    }
    let name = text_input(labels::AGENT_GRAPH_NAME_PLACEHOLDER, &draft.name)
        .on_input(Message::NameChanged)
        .padding(theme::AGENT_GRAPH_FIELD_PADDING)
        .style(graph_input_style);
    let command = text_input(labels::AGENT_GRAPH_COMMAND_PLACEHOLDER, &draft.command)
        .on_input(Message::CommandChanged)
        .padding(theme::AGENT_GRAPH_FIELD_PADDING)
        .font(theme::FONT_MONO)
        .size(12.5)
        .style(graph_input_style);
    let cwd = text_input("", &draft.cwd)
        .on_input(Message::CwdChanged)
        .padding(theme::AGENT_GRAPH_FIELD_PADDING)
        .font(theme::FONT_MONO)
        .size(12.5)
        .style(graph_input_style);
    let name_error = if draft.name.trim().is_empty() {
        None
    } else if taken
        .iter()
        .any(|value| value.eq_ignore_ascii_case(draft.name.trim()))
    {
        Some(labels::AGENT_GRAPH_NAME_TAKEN)
    } else {
        None
    };
    let cwd_error = (draft.cwd.trim().is_empty() || !Path::new(draft.cwd.trim()).is_dir())
        .then_some(labels::AGENT_GRAPH_CWD_MISSING);
    let cwd_row = row![
        cwd,
        pane_close_dialog::action(
            labels::AGENT_GRAPH_BROWSE,
            pane_close_dialog::ButtonKind::Outline,
            Some(Message::Browse),
        )
    ]
    .spacing(7);
    let mut content = column![
        caption(labels::AGENT_GRAPH_NAME),
        name,
        caption(name_error.unwrap_or("")),
        caption(labels::AGENT_GRAPH_COMMAND),
        command,
        caption(labels::AGENT_GRAPH_CWD),
        cwd_row,
        text(cwd_error.unwrap_or(""))
            .size(theme::SIZE_SECTION_HEADER)
            .color(if cwd_error.is_some() {
                theme::palette().error
            } else {
                theme::palette().text2
            }),
        toggle_row(
            labels::AGENT_GRAPH_MONITOR,
            Some(labels::AGENT_GRAPH_MONITOR_HINT),
            None,
            draft.monitor,
            Message::SetMonitor(!draft.monitor),
        ),
        toggle_row(
            labels::AGENT_GRAPH_MAESTRO,
            None,
            Some(labels::AGENT_GRAPH_MAESTRO_TOOLTIP),
            draft.maestro,
            Message::SetMaestro(!draft.maestro),
        ),
    ]
    .spacing(5);
    if let Some(name) = maestro.filter(|_| draft.maestro) {
        content = content.push(
            text(labels::agent_graph_maestro_replaces(name))
                .size(theme::SIZE_LABEL)
                .color(theme::palette().warn),
        );
    }
    container(content).width(Fill).into()
}

fn appearance<'a>(draft: &'a Draft) -> Element<'a, Message> {
    let colors = gsh_core::agent_color::AGENT_PALETTE;
    let mut swatches = row![button(
        container(
            text(labels::ICON_AUTO_AWESOME)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON)
                .color(theme::palette().text2),
        )
        .width(28)
        .height(28)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(|_: &Theme| container::Style {
            background: Some(Background::Color(theme::palette().panel2)),
            border: Border {
                color: if draft.color.is_none() {
                    theme::palette().text
                } else {
                    theme::palette().border
                },
                width: if draft.color.is_none() { 2.0 } else { 1.0 },
                radius: theme::RADIUS_BUTTON.into(),
            },
            ..Default::default()
        }),
    )
    .on_press(Message::SetColor(None))
    .padding(0)
    .style(|_: &Theme, _| button::Style::default())]
    .spacing(8);
    for (index, color) in colors.into_iter().enumerate() {
        let color = iced::Color::from_rgb8(color.r, color.g, color.b);
        let selected = draft.color == Some(index);
        let check: Element<'_, Message> = if selected {
            text(labels::ICON_CHECK)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON)
                .color(theme::on_color(color))
                .into()
        } else {
            Space::new().into()
        };
        swatches = swatches.push(
            button(
                container(check)
                    .width(28)
                    .height(28)
                    .align_x(iced::alignment::Horizontal::Center)
                    .align_y(iced::alignment::Vertical::Center)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(color)),
                        border: Border {
                            color: if selected {
                                theme::palette().text
                            } else {
                                theme::palette().border
                            },
                            width: if selected { 2.0 } else { 1.0 },
                            radius: theme::RADIUS_BUTTON.into(),
                        },
                        ..Default::default()
                    }),
            )
            .padding(0)
            .on_press(Message::SetColor(Some(index)))
            .style(|_: &Theme, _| button::Style::default()),
        );
    }
    column![caption(labels::AGENT_GRAPH_COLOR), swatches]
        .spacing(8)
        .into()
}

fn agent<'a>(draft: &'a Draft) -> Element<'a, Message> {
    column![
        caption(labels::AGENT_GRAPH_ROLE),
        text_input(labels::AGENT_GRAPH_ROLE_PLACEHOLDER, &draft.role)
            .on_input(Message::RoleChanged)
            .padding(theme::AGENT_GRAPH_FIELD_PADDING)
            .style(graph_input_style),
        caption(labels::AGENT_GRAPH_TASK),
        text_input(labels::AGENT_GRAPH_TASK_PLACEHOLDER, &draft.task)
            .on_input(Message::TaskChanged)
            .padding(theme::AGENT_GRAPH_FIELD_PADDING)
            .style(graph_input_style),
    ]
    .spacing(8)
    .into()
}

fn caption<'a>(label: &'static str) -> Element<'a, Message> {
    text(label)
        .font(theme::FONT_BODY_MEDIUM)
        .size(theme::SIZE_SECTION_HEADER)
        .color(theme::palette().text2)
        .into()
}

fn toggle_row<'a>(
    title: &'static str,
    hint: Option<&'static str>,
    info: Option<&'static str>,
    value: bool,
    message: Message,
) -> Element<'a, Message> {
    let mut heading = row![text(title)
        .size(theme::SIZE_BODY)
        .color(theme::palette().text)]
    .spacing(5)
    .align_y(Alignment::Center);
    if let Some(info) = info {
        heading = heading.push(tooltip(
            text(labels::ICON_INFO_OUTLINE)
                .font(theme::FONT_ICON)
                .size(14),
            text(info),
            tooltip::Position::Top,
        ));
    }
    let mut labels = column![heading].spacing(2);
    if let Some(hint) = hint {
        labels = labels.push(
            text(hint)
                .size(theme::SIZE_LABEL)
                .color(theme::palette().text3),
        );
    }
    row![
        labels.width(Fill),
        crate::settings::view::switch(value, message)
    ]
    .align_y(Alignment::Center)
    .into()
}

fn graph_input_style(
    _: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let palette = theme::palette();
    iced::widget::text_input::Style {
        background: palette.panel2.into(),
        border: Border {
            color: if matches!(status, iced::widget::text_input::Status::Focused { .. }) {
                palette.accent
            } else {
                palette.border2
            },
            width: 1.0,
            radius: theme::AGENT_GRAPH_FIELD_RADIUS.into(),
        },
        icon: palette.text3,
        placeholder: palette.text3,
        value: palette.text,
        selection: palette.accent_soft,
    }
}

fn quick_button<'a>(
    name: &'static str,
    command: &'static str,
    icon: Option<&'static [u8]>,
) -> Element<'a, Message> {
    let leading: Element<'a, Message> = icon.map_or_else(
        || {
            text(labels::ICON_TERMINAL)
                .font(theme::FONT_ICON)
                .size(theme::SIZE_ICON)
                .into()
        },
        |bytes| {
            svg(svg::Handle::from_memory(bytes))
                .width(16)
                .height(16)
                .into()
        },
    );
    button(
        row![leading, text(name).size(10.5)]
            .spacing(6)
            .align_y(Alignment::Center),
    )
    .padding([6, 8])
    .on_press(Message::QuickStart(name, command))
    .style(|_: &Theme, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(Background::Color(theme::palette().panel3)),
        text_color: theme::palette().text,
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: theme::RADIUS_BUTTON.into(),
        },
        ..Default::default()
    })
    .into()
}

fn desktop_button<'a>(
    integration: &crate::desktop_agent_apps::Integration,
    selected: bool,
    index: usize,
) -> Element<'a, Message> {
    let icon = svg(svg::Handle::from_memory(integration.app.icon()))
        .width(16)
        .height(16);
    let check: Element<'a, Message> = if selected {
        text("✓")
            .size(theme::SIZE_LABEL)
            .color(theme::palette().accent)
            .into()
    } else {
        Space::new().width(0).into()
    };
    button(
        row![
            icon,
            text(integration.app.label()).size(theme::SIZE_LABEL),
            check
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([6, 8])
    .on_press(Message::PickDesktop(index))
    .style(|_: &Theme, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then_some(Background::Color(theme::palette().panel3)),
        text_color: theme::palette().text,
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: theme::RADIUS_BUTTON.into(),
        },
        ..Default::default()
    })
    .into()
}

fn checkbox_style(
    _: &Theme,
    status: iced::widget::checkbox::Status,
) -> iced::widget::checkbox::Style {
    let checked = matches!(
        status,
        iced::widget::checkbox::Status::Active { is_checked: true }
            | iced::widget::checkbox::Status::Hovered { is_checked: true }
            | iced::widget::checkbox::Status::Disabled { is_checked: true }
    );
    iced::widget::checkbox::Style {
        background: theme::palette().panel2.into(),
        icon_color: theme::palette().accent_text,
        border: Border {
            color: if checked {
                theme::palette().accent
            } else {
                theme::palette().border2
            },
            width: 1.0,
            radius: theme::RADIUS_BADGE.into(),
        },
        text_color: Some(theme::palette().text),
    }
}
