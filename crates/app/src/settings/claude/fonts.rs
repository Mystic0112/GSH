//! Catálogo de fontes do sistema (`font_catalog.dart`), `SettingsFontField` e
//! `FontPickerDialog` (`font_picker_dialog.dart`).

use super::widgets::{field_style, small_button, Emphasis};
use super::Message;
use crate::{labels, pane_chrome, theme};
use iced::widget::{
    button, column, container, mouse_area, opaque, row, scrollable, stack, text, text_input,
    tooltip, Space,
};
use iced::{Alignment, Background, Border, Element, Fill, Length, Theme};
use std::collections::BTreeSet;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontTarget {
    Interface,
    Chat,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontOption {
    pub family: String,
    pub monospaced: bool,
    pub bundled: bool,
}

pub struct FontPickerState {
    pub target: FontTarget,
    pub current: Option<String>,
    pub default_label: &'static str,
    pub search: String,
    pub custom: String,
    pub options: Vec<FontOption>,
}

impl FontPickerState {
    pub fn open(target: FontTarget, current: Option<String>) -> Self {
        let default_label = match target {
            FontTarget::Interface => labels::SETTINGS_GC_UI_FONT_HINT,
            FontTarget::Chat => labels::SETTINGS_GC_CHAT_FONT_DEFAULT,
        };
        let custom = current.clone().unwrap_or_default();
        Self {
            target,
            current,
            default_label,
            search: String::new(),
            custom,
            options: available_fonts(),
        }
    }

    pub fn visible(&self) -> Vec<&FontOption> {
        let q = self.search.trim().to_lowercase();
        self.options
            .iter()
            .filter(|o| q.is_empty() || o.family.to_lowercase().contains(&q))
            .collect()
    }
}

const APP_FONTS: [(&str, bool); 3] = [
    ("JetBrains Mono", true),
    ("Montserrat", false),
    ("Hanken Grotesk", false),
];

const CANDIDATES: &[(&str, bool)] = &[
    ("JetBrains Mono", true),
    ("Montserrat", false),
    ("Hanken Grotesk", false),
    ("Fira Code", true),
    ("Fira Mono", true),
    ("IBM Plex Mono", true),
    ("Source Code Pro", true),
    ("Roboto Mono", true),
    ("Ubuntu Mono", true),
    ("Inconsolata", true),
    ("Hack", true),
    ("Iosevka", true),
    ("Cascadia Code", true),
    ("Cascadia Mono", true),
    ("MesloLGS NF", true),
    ("Victor Mono", true),
    ("Anonymous Pro", true),
    ("PT Mono", true),
    ("Courier New", true),
    ("DejaVu Sans Mono", true),
    ("Liberation Mono", true),
    ("Noto Sans Mono", true),
    ("Ubuntu", false),
    ("Cantarell", false),
    ("SF Mono", true),
    ("Menlo", true),
    ("Monaco", true),
    ("Andale Mono", true),
    ("Helvetica Neue", false),
    ("SF Pro Text", false),
    ("Avenir Next", false),
    ("Consolas", true),
    ("Lucida Console", true),
    ("Segoe UI", false),
    ("Segoe UI Mono", true),
    ("Inter", false),
    ("Roboto", false),
    ("Open Sans", false),
    ("Lato", false),
    ("Nunito Sans", false),
];

fn system_families() -> &'static BTreeSet<String> {
    static FAMILIES: OnceLock<BTreeSet<String>> = OnceLock::new();
    FAMILIES.get_or_init(|| {
        let mut set = BTreeSet::new();
        for (name, _) in APP_FONTS {
            set.insert(name.to_lowercase());
        }
        if let Ok(output) = Command::new("fc-list")
            .args([":", "family"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                for part in line.split(',') {
                    let clean = part.trim().to_lowercase();
                    if !clean.is_empty() {
                        set.insert(clean);
                    }
                }
            }
        }
        set
    })
}

pub fn is_font_available(family: &str) -> bool {
    let key = family.trim().to_lowercase();
    if key.is_empty() {
        return false;
    }
    APP_FONTS.iter().any(|(name, _)| name.to_lowercase() == key) || system_families().contains(&key)
}

pub fn available_fonts() -> Vec<FontOption> {
    let mut seen = BTreeSet::new();
    let mut options = Vec::new();
    for &(family, mono) in CANDIDATES {
        if !seen.insert(family) {
            continue;
        }
        let bundled = APP_FONTS.iter().any(|(b, _)| *b == family);
        if !bundled && !is_font_available(family) {
            continue;
        }
        options.push(FontOption {
            family: family.to_owned(),
            monospaced: mono,
            bundled,
        });
    }
    options.sort_by(|a, b| match (a.bundled, b.bundled) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.family.to_lowercase().cmp(&b.family.to_lowercase()),
    });
    options
}

/// `SettingsFontField`: pílula clicável que abre o seletor de fontes e avisa quando a família
/// escolhida não existe nesta máquina.
pub fn font_field<'a>(
    current: Option<&'a str>,
    hint: &'static str,
    target: FontTarget,
) -> Element<'a, Message> {
    let p = theme::palette();
    let missing = current.is_some_and(|f| !is_font_available(f));
    let label_text = current.unwrap_or(hint);
    let label_color = if current.is_some() { p.text } else { p.text3 };

    let trigger = button(
        container(
            row![
                text(label_text)
                    .font(theme::FONT_BODY)
                    .size(theme::SIZE_MONO)
                    .color(label_color)
                    .width(Fill),
                text("\u{e313}")
                    .font(theme::FONT_ICON)
                    .size(theme::SETTINGS_DROPDOWN_ICON)
                    .color(p.text3),
            ]
            .align_y(Alignment::Center),
        )
        .width(Length::Fixed(theme::SETTINGS_FONT_FIELD_WIDTH)),
    )
    .padding([8, 11])
    .on_press(Message::FontPickerOpen(target))
    .style(|_: &Theme, status| {
        let p = theme::palette();
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(Background::Color(if hovered { p.panel2 } else { p.panel3 })),
            text_color: p.text,
            border: Border {
                color: p.border2,
                width: if hovered { 1.0 } else { 0.0 },
                radius: theme::SETTINGS_DROPDOWN_RADIUS.into(),
            },
            ..Default::default()
        }
    });

    if !missing {
        return trigger.into();
    }
    let warn = tooltip(
        text(labels::ICON_WARNING_AMBER)
            .font(theme::FONT_ICON)
            .size(15.0)
            .color(p.warn),
        pane_chrome::tip_bubble(labels::SETTINGS_FONT_MISSING),
        tooltip::Position::Bottom,
    );
    row![warn, Space::new().width(6.0), trigger]
        .align_y(Alignment::Center)
        .into()
}

/// Modal `_FontPickerDialog` (460×460).
pub fn dialog(picker: &FontPickerState) -> Element<'_, Message> {
    let p = theme::palette();
    let visible = picker.visible();
    let default_tile = option_tile(
        picker.default_label,
        picker.current.is_none(),
        None,
        Message::FontPick(None),
    );
    let list_body: Element<'_, Message> = if visible.is_empty() {
        container(
            text(labels::SETTINGS_FONT_PICKER_EMPTY)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(p.text3),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
    } else {
        let items = visible
            .into_iter()
            .fold(column![].spacing(2.0), |col, opt| {
                let selected = picker.current.as_deref() == Some(opt.family.as_str());
                let badge = opt.bundled.then_some(labels::SETTINGS_FONT_PICKER_BUNDLED);
                col.push(option_tile(
                    &opt.family,
                    selected,
                    badge,
                    Message::FontPick(Some(opt.family.clone())),
                ))
            });
        scrollable(items).height(Fill).width(Fill).into()
    };

    let can_use_custom = !picker.custom.trim().is_empty();
    let custom_row = row![
        text_input(labels::SETTINGS_FONT_PICKER_CUSTOM_HINT, &picker.custom)
            .on_input(Message::FontPickerCustomInput)
            .on_submit(Message::FontPickerUseCustom)
            .font(theme::FONT_BODY)
            .size(theme::SETTINGS_DROPDOWN_TEXT)
            .padding(theme::SETTINGS_TEXT_FIELD_PADDING)
            .width(Fill)
            .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS)),
        Space::new().width(8.0),
        small_button(
            labels::SETTINGS_FONT_PICKER_USE,
            None,
            Emphasis::Primary,
            can_use_custom.then_some(Message::FontPickerUseCustom),
        ),
    ]
    .align_y(Alignment::Center);

    let content = container(
        column![
            text(labels::SETTINGS_FONT_PICKER_TITLE)
                .font(theme::FONT_TITLE)
                .size(theme::DIALOG_TITLE_SIZE)
                .color(p.text),
            Space::new().height(12.0),
            text_input(labels::SETTINGS_FONT_PICKER_SEARCH, &picker.search)
                .on_input(Message::FontPickerSearch)
                .font(theme::FONT_BODY)
                .size(theme::SETTINGS_DROPDOWN_TEXT)
                .padding(theme::SETTINGS_TEXT_FIELD_PADDING)
                .width(Fill)
                .style(field_style(theme::SETTINGS_TEXT_FIELD_RADIUS)),
            Space::new().height(10.0),
            default_tile,
            Space::new().height(6.0),
            list_body,
            Space::new().height(10.0),
            text(labels::SETTINGS_FONT_PICKER_CUSTOM)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(p.text3),
            Space::new().height(6.0),
            custom_row,
            Space::new().height(14.0),
            row![
                Space::new().width(Fill),
                small_button(
                    labels::DIALOG_CANCEL,
                    None,
                    Emphasis::Outline,
                    Some(Message::FontPickerClose),
                )
            ],
        ]
        .width(Length::Fixed(theme::SETTINGS_FONT_DIALOG_SIZE))
        .height(Length::Fixed(theme::SETTINGS_FONT_DIALOG_SIZE)),
    )
    .padding(theme::DIALOG_PADDING)
    .style(|_: &Theme| container::Style {
        background: Some(Background::Color(theme::palette().panel)),
        border: Border {
            color: theme::palette().panel2,
            width: 1.0,
            radius: theme::DIALOG_RADIUS.into(),
        },
        ..Default::default()
    });

    let scrim = mouse_area(
        container(Space::new().width(Fill).height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|_: &Theme| container::Style {
                background: Some(Background::Color(theme::palette().scrim)),
                ..Default::default()
            }),
    )
    .on_press(Message::FontPickerClose);

    stack![
        scrim,
        container(opaque(content))
            .width(Fill)
            .height(Fill)
            .center_x(Fill)
            .center_y(Fill)
    ]
    .into()
}

fn option_tile<'a>(
    label: &'a str,
    selected: bool,
    badge: Option<&'static str>,
    on_press: Message,
) -> Element<'a, Message> {
    let p = theme::palette();
    let mut line = row![text(label)
        .font(theme::FONT_BODY)
        .size(theme::SETTINGS_NAV_TEXT_SIZE)
        .color(p.text)
        .width(Fill),]
    .align_y(Alignment::Center);
    if let Some(badge) = badge {
        line = line.push(Space::new().width(8.0)).push(
            text(badge)
                .font(theme::FONT_BODY_MEDIUM)
                .size(theme::SIZE_LABEL)
                .color(p.text3),
        );
    }
    if selected {
        line = line.push(Space::new().width(8.0)).push(
            text(labels::ICON_CHECK)
                .font(theme::FONT_ICON)
                .size(15.0)
                .color(p.accent),
        );
    }
    button(line)
        .width(Fill)
        .padding([9, 10])
        .on_press(on_press)
        .style(move |_: &Theme, status| {
            let p = theme::palette();
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: if selected || hovered {
                    Some(Background::Color(p.panel3))
                } else {
                    None
                },
                text_color: p.text,
                border: Border {
                    radius: theme::SETTINGS_DROPDOWN_RADIUS.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_fonts_are_always_available_and_sorted_first() {
        let fonts = available_fonts();
        assert!(fonts.len() >= 3);
        assert!(fonts[0].bundled && fonts[1].bundled && fonts[2].bundled);
        assert!(is_font_available("Montserrat"));
        assert!(is_font_available("JetBrains Mono"));
        assert!(!is_font_available("__gsh_absent_family__"));
    }
}
