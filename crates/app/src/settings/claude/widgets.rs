//! Peças das abas no estilo plano (`settings_widgets.dart`): título de seção em negrito e
//! linhas separadas por um fio, mais os controles que as linhas levam.

use crate::{labels, panel_widgets, theme};
use iced::widget::{button, column, container, pick_list, row, text, text_input, tooltip, Space};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Theme};
use std::fmt;

fn body<'a>(content: impl Into<String>, size: f32, color: Color) -> iced::widget::Text<'a> {
    text(content.into())
        .font(theme::FONT_BODY)
        .size(size)
        .line_height(iced::widget::text::LineHeight::Relative(
            theme::LINE_HEIGHT_BODY,
        ))
        .color(color)
}

fn description<'a>(content: &'static str) -> iced::widget::Text<'a> {
    text(content)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_DESC_SIZE)
        .line_height(iced::widget::text::LineHeight::Relative(
            theme::SETTINGS_DESC_LINE_HEIGHT,
        ))
        .color(theme::palette().text3)
}

/// `SettingsSection`: título, descrição opcional e as linhas com um fio entre elas.
pub fn section<'a, M: 'a>(
    title: &'static str,
    action: Option<Element<'a, M>>,
    rows: Vec<Element<'a, M>>,
) -> Element<'a, M> {
    section_with_desc(title, None, action, true, rows)
}

pub fn section_with_desc<'a, M: 'a>(
    title: &'static str,
    desc: Option<&'static str>,
    action: Option<Element<'a, M>>,
    divided: bool,
    rows: Vec<Element<'a, M>>,
) -> Element<'a, M> {
    let mut heading = row![body(
        title,
        theme::SETTINGS_SECTION_TITLE_SIZE,
        theme::palette().text
    )
    .font(theme::FONT_BODY_SEMIBOLD)
    .width(Fill),]
    .align_y(Alignment::Center);
    if let Some(action) = action {
        heading = heading.push(action);
    }
    let mut list = column![container(heading).padding(iced::Padding {
        bottom: if desc.is_some() {
            theme::SETTINGS_SECTION_DESC_GAP
        } else {
            theme::SETTINGS_SECTION_TITLE_GAP
        },
        ..Default::default()
    })];
    if let Some(desc) = desc {
        list = list.push(container(description(desc)).padding(iced::Padding {
            bottom: theme::SETTINGS_SECTION_TITLE_GAP,
            ..Default::default()
        }));
    }
    for (index, item) in rows.into_iter().enumerate() {
        if index > 0 && divided {
            list = list.push(panel_widgets::hairline_h());
        }
        list = list.push(item);
    }
    container(list.width(Fill))
        .padding(iced::Padding {
            bottom: theme::SETTINGS_SECTION_GAP,
            ..Default::default()
        })
        .width(Fill)
        .into()
}

/// `SettingsRow`: título e descrição à esquerda, controle à direita (centralizado na altura).
pub fn setting_row<'a, M: 'a>(
    title: &'static str,
    desc: Option<&'static str>,
    trailing: Option<Element<'a, M>>,
    leading: Option<Element<'a, M>>,
) -> Element<'a, M> {
    let mut texts =
        column![body(title, theme::SETTINGS_NAV_TEXT_SIZE, theme::palette().text).width(Fill)];
    if let Some(desc) = desc {
        texts = texts
            .push(Space::new().height(theme::SETTINGS_ROW_TITLE_GAP))
            .push(description(desc).width(Fill));
    }
    let mut line = row![];
    if let Some(leading) = leading {
        line = line
            .push(leading)
            .push(Space::new().width(theme::SETTINGS_ROW_LEADING_GAP));
    }
    line = line.push(texts.width(Fill));
    if let Some(trailing) = trailing {
        line = line
            .push(Space::new().width(theme::SETTINGS_ROW_GAP))
            .push(trailing);
    }
    line.width(Fill)
        .align_y(Alignment::Center)
        .padding([theme::SETTINGS_ROW_VERTICAL, 0])
        .into()
}

/// `SettingsNote`: texto apagado (carregando, vazio, aviso) ou de erro dentro de uma seção.
pub fn note<'a, M: 'a>(content: String, error: bool) -> Element<'a, M> {
    let p = theme::palette();
    container(
        text(content)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DESC_SIZE)
            .line_height(iced::widget::text::LineHeight::Relative(
                theme::SETTINGS_DESC_LINE_HEIGHT,
            ))
            .color(if error { p.error } else { p.text3 }),
    )
    .padding([theme::SETTINGS_NOTE_VERTICAL, 0])
    .into()
}

/// `SettingsChip`: selo pequeno e apagado ("team", "2").
pub fn chip<'a, M: 'a>(label: String) -> Element<'a, M> {
    container(
        text(label)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_CHIP_TEXT)
            .line_height(iced::widget::text::LineHeight::Relative(
                theme::LINE_HEIGHT_LABEL,
            ))
            .color(theme::palette().text2),
    )
    .padding(theme::SETTINGS_CHIP_PADDING)
    .style(|_| container::Style {
        background: Some(theme::palette().panel3.into()),
        border: Border {
            radius: theme::SETTINGS_CHIP_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

/// Valor lido da CLI à direita da linha (texto cru, sem tradução); mono vai numa caixinha.
pub fn value<'a, M: 'a>(content: String, mono: bool) -> Element<'a, M> {
    let p = theme::palette();
    if !mono {
        return text(content)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_VALUE_TEXT)
            .line_height(iced::widget::text::LineHeight::Relative(
                theme::LINE_HEIGHT_BODY,
            ))
            .color(p.text2)
            .into();
    }
    container(
        text(content)
            .font(theme::FONT_MONO)
            .size(theme::SETTINGS_VALUE_MONO_TEXT)
            .color(p.text2),
    )
    .padding(theme::SETTINGS_VALUE_MONO_PADDING)
    .style(|_| container::Style {
        background: Some(theme::palette().panel2.into()),
        border: Border {
            radius: theme::SETTINGS_VALUE_MONO_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

#[derive(Clone, Copy)]
pub enum Emphasis {
    Outline,
    Primary,
}

/// `OutlineButton`/`PrimaryButton` com `ButtonSize.small`; sem `on_press` fica desabilitado.
pub fn small_button<'a, M: Clone + 'a>(
    label: &'static str,
    icon: Option<&'static str>,
    emphasis: Emphasis,
    on_press: Option<M>,
) -> Element<'a, M> {
    let p = theme::palette();
    let color = match emphasis {
        Emphasis::Outline => p.text,
        Emphasis::Primary => theme::ON_ACCENT,
    };
    let mut content = row![]
        .spacing(theme::SETTINGS_BUTTON_GAP)
        .align_y(Alignment::Center);
    if let Some(icon) = icon {
        content = content.push(
            text(icon)
                .font(theme::FONT_ICON)
                .size(theme::SETTINGS_BUTTON_ICON)
                .color(color),
        );
    }
    content = content.push(
        text(label)
            .font(theme::FONT_BODY_MEDIUM)
            .size(theme::SETTINGS_BUTTON_TEXT)
            .color(color),
    );
    button(content)
        .padding(theme::SETTINGS_BUTTON_PADDING)
        .on_press_maybe(on_press)
        .style(move |_: &Theme, status| {
            let p = theme::palette();
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            let disabled = matches!(status, button::Status::Disabled);
            let (background, border, text_color) = match emphasis {
                Emphasis::Outline => (
                    hovered.then_some(Background::Color(p.panel3)),
                    p.border,
                    if disabled { p.text3 } else { p.text },
                ),
                Emphasis::Primary => {
                    let fill = if disabled { p.text2 } else { p.accent };
                    let fill = if hovered {
                        Color { a: 0.9, ..fill }
                    } else {
                        fill
                    };
                    (Some(Background::Color(fill)), fill, theme::ON_ACCENT)
                }
            };
            button::Style {
                background,
                text_color,
                border: Border {
                    color: border,
                    width: 1.0,
                    radius: theme::SETTINGS_BUTTON_RADIUS.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

/// "Atualizar" das seções que leem a CLI; desabilitado enquanto lê (o Flutter troca o ícone
/// por um spinner).
pub fn refresh_button<'a, M: Clone + 'a>(loading: bool, on_press: M) -> Element<'a, M> {
    small_button(
        labels::SETTINGS_CLAUDE_REFRESH,
        Some(labels::ICON_REFRESH),
        Emphasis::Outline,
        (!loading).then_some(on_press),
    )
}

/// Opção de um dropdown: o valor e o rótulo mostrado.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub value: Option<String>,
    pub label: String,
}

impl fmt::Display for Choice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn menu_style(_: &Theme) -> iced::widget::overlay::menu::Style {
    let p = theme::palette();
    iced::widget::overlay::menu::Style {
        text_color: p.text,
        background: p.panel2.into(),
        border: Border {
            color: p.border2,
            width: 1.0,
            radius: theme::SETTINGS_DROPDOWN_RADIUS.into(),
        },
        selected_text_color: p.text,
        selected_background: p.panel3.into(),
        shadow: Default::default(),
    }
}

fn dropdown_style(_: &Theme, status: pick_list::Status) -> pick_list::Style {
    let p = theme::palette();
    let hovered = matches!(
        status,
        pick_list::Status::Hovered | pick_list::Status::Opened { .. }
    );
    pick_list::Style {
        text_color: p.text,
        placeholder_color: p.text3,
        handle_color: p.text3,
        background: if hovered { p.panel3 } else { p.panel2 }.into(),
        border: Border {
            color: p.border2,
            width: 1.0,
            radius: theme::SETTINGS_DROPDOWN_RADIUS.into(),
        },
    }
}

/// Gatilho de dropdown plano ("Padrão ⌄"): `SettingsDropdown` (140 de largura) ou, com
/// `width` `Fill`, o do seletor de paleta de código.
pub fn dropdown<'a, M: Clone + 'a>(
    options: Vec<Choice>,
    selected: Choice,
    width: Length,
    padding: [u16; 2],
    text_size: f32,
    icon_size: f32,
    on_pick: impl Fn(Choice) -> M + 'a,
) -> Element<'a, M> {
    pick_list(options, Some(selected), on_pick)
        .width(width)
        .padding(padding)
        .text_size(text_size)
        .font(theme::FONT_BODY)
        .handle(pick_list::Handle::Static(pick_list::Icon {
            font: theme::FONT_ICON,
            code_point: '\u{e313}',
            size: Some(iced::Pixels(icon_size)),
            line_height: iced::widget::text::LineHeight::default(),
            shaping: iced::widget::text::Shaping::Basic,
        }))
        .style(dropdown_style)
        .menu_style(menu_style)
        .into()
}

/// Estilo do campo de texto padrão do shadcn: fundo é a borda a 30% e borda `border`.
fn field_background() -> Color {
    Color {
        a: theme::SETTINGS_TEXT_FIELD_FILL_ALPHA,
        ..theme::palette().border
    }
}

pub fn field_style(radius: f32) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, status| {
        let p = theme::palette();
        text_input::Style {
            background: Background::Color(field_background()),
            border: Border {
                color: if matches!(status, text_input::Status::Focused { .. }) {
                    p.accent
                } else {
                    p.border
                },
                width: 1.0,
                radius: radius.into(),
            },
            icon: p.text3,
            placeholder: p.text3,
            value: p.text,
            selection: p.accent_soft,
        }
    }
}

/// `SettingsTextField`: texto curto digitado à mão (224 de largura), grava ao confirmar.
pub fn text_field<'a, M: Clone + 'a>(
    value: &str,
    placeholder: &str,
    on_input: impl Fn(String) -> M + 'a,
    on_submit: M,
) -> Element<'a, M> {
    text_input(placeholder, value)
        .on_input(on_input)
        .on_submit(on_submit)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_DROPDOWN_TEXT)
        .padding(theme::SETTINGS_TEXT_FIELD_PADDING)
        .width(theme::SETTINGS_TEXT_FIELD_WIDTH)
        .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS))
        .into()
}

/// Caixa do `SettingsSegmented` com ícones no lugar dos rótulos (o rótulo vira dica).
pub fn icon_choice<'a, M: Clone + 'a>(
    icon: &'static str,
    tip: &'static str,
    selected: bool,
    message: M,
) -> Element<'a, M> {
    let p = theme::palette();
    let glyph = text(icon)
        .font(theme::FONT_ICON)
        .size(theme::SETTINGS_NAV_ICON_SIZE)
        .color(if selected { p.text } else { p.text2 });
    let pill = button(glyph)
        .padding([
            theme::SETTINGS_SEGMENT_VERTICAL,
            theme::SETTINGS_SEGMENT_HORIZONTAL,
        ])
        .on_press(message)
        .style(move |_, status| {
            let p = theme::palette();
            button::Style {
                background: if selected {
                    Some(p.panel3.into())
                } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    Some(p.panel.into())
                } else {
                    None
                },
                text_color: if selected { p.text } else { p.text2 },
                border: Border {
                    color: if selected {
                        p.border2
                    } else {
                        Color::TRANSPARENT
                    },
                    width: 1.0,
                    radius: theme::SETTINGS_SEGMENT_RADIUS.into(),
                },
                ..Default::default()
            }
        });
    tooltip(
        pill,
        crate::pane_chrome::tip_bubble(tip),
        tooltip::Position::Bottom,
    )
    .into()
}

/// `McpStatusChip`: ícone + rótulo do status de um servidor MCP.
pub fn mcp_status_chip<'a, M: 'a>(status: super::cli::McpStatus) -> Element<'a, M> {
    use super::cli::McpStatus;
    let p = theme::palette();
    let (label, color, icon) = match status {
        McpStatus::Connected => (
            labels::SETTINGS_CONN_STATUS_CONNECTED,
            p.ok,
            labels::ICON_CHECK,
        ),
        McpStatus::NeedsAuth => (
            labels::SETTINGS_CONN_STATUS_NEEDS_AUTH,
            p.warn,
            labels::ICON_KEY,
        ),
        McpStatus::Failed => (
            labels::SETTINGS_CONN_STATUS_FAILED,
            p.error,
            labels::ICON_ERROR_OUTLINE,
        ),
        McpStatus::Pending => (
            labels::SETTINGS_CONN_STATUS_PENDING,
            p.text3,
            labels::ICON_PAUSE,
        ),
        McpStatus::Unknown => (
            labels::SETTINGS_CONN_STATUS_UNKNOWN,
            p.text3,
            labels::ICON_HELP_OUTLINE,
        ),
    };
    row![
        text(icon).font(theme::FONT_ICON).size(15.0).color(color),
        Space::new().width(6.0),
        text(label).font(theme::FONT_BODY).size(13.0).color(p.text2),
    ]
    .align_y(Alignment::Center)
    .into()
}

/// `catalogMatches` de `settings_catalog.dart`: busca sem diferenciar caixa.
pub fn catalog_matches(query: &str, fields: &[Option<&str>]) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    fields
        .iter()
        .flatten()
        .any(|f| f.to_lowercase().contains(&q))
}

/// `shellQuote` de `settings_catalog.dart`.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}
