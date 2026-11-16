//! Interface do navegador de chaves Redis (`db_redis_table.dart`).

use gsh_db::redis_browse::{RedisKeyEntry, RedisValueKind};
use gsh_db::Engine;
use iced::widget::{
    button, column, container, row, scrollable, text, text_editor, text_input, tooltip, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Theme};

use crate::anchored::Anchored;
use crate::app_menu::{self, Entry, Item};
use crate::db_redis::{NewKeyDraft, RedisMsg, RedisTab};
use crate::pane_close_dialog::{self, ButtonKind};
use crate::{db_engine_icon, labels, pane_chrome, theme};

const COL_KEY_WIDTH: f32 = 170.0;
const COL_TYPE_WIDTH: f32 = 78.0;
const COL_TTL_WIDTH: f32 = 64.0;

pub fn view(tab: &RedisTab) -> Element<'_, RedisMsg> {
    let mut col = column![toolbar(tab)];
    if let Some(err) = &tab.error {
        col = col.push(error_banner(err));
    }
    col = col.push(header()).push(table(tab));
    container(col)
        .width(Fill)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::palette().panel)),
            ..Default::default()
        })
        .into()
}

fn toolbar(tab: &RedisTab) -> Element<'_, RedisMsg> {
    let palette = theme::palette();
    let search_icon = button(
        text(labels::ICON_SEARCH_TAB)
            .font(theme::FONT_ICON)
            .size(13)
            .color(palette.text3),
    )
    .padding(2)
    .on_press(RedisMsg::ApplyPattern)
    .style(ghost_icon_style);

    let pattern_input = text_input(labels::REDIS_SEARCH_HINT, &tab.pattern_input)
        .on_input(RedisMsg::PatternChanged)
        .on_submit(RedisMsg::ApplyPattern)
        .font(theme::FONT_MONO)
        .size(11.5)
        .padding([4, 6])
        .style(plain_input_style);

    let mut search_box = row![search_icon, Space::new().width(4), pattern_input]
        .align_y(Alignment::Center)
        .width(Length::Fixed(240.0));
    if !tab.pattern_input.is_empty() {
        search_box = search_box.push(
            button(
                text(labels::ICON_CLOSE)
                    .font(theme::FONT_ICON)
                    .size(12)
                    .color(palette.text3),
            )
            .padding(2)
            .on_press(RedisMsg::ClearPattern)
            .style(ghost_icon_style),
        );
    }

    let search_wrap = container(search_box)
        .padding([0, 6])
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.bg)),
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: 5.0.into(),
            },
            ..Default::default()
        });

    let count = format!(
        "{}{}",
        labels::redis_key_count(tab.entries.len()),
        if tab.cursor == "0" { "" } else { "+" }
    );

    let refresh_btn = action_tooltip(
        labels::REDIS_REFRESH,
        button(
            text(labels::ICON_REFRESH)
                .font(theme::FONT_ICON)
                .size(15)
                .color(palette.text3),
        )
        .padding(4)
        .on_press_maybe((!tab.loading).then_some(RedisMsg::Refresh))
        .style(hover_icon_style)
        .into(),
    );

    let new_btn = action_tooltip(
        labels::REDIS_NEW_KEY,
        button(
            text(labels::ICON_PLUS)
                .font(theme::FONT_ICON)
                .size(15)
                .color(palette.text3),
        )
        .padding(4)
        .on_press(RedisMsg::OpenDraft)
        .style(hover_icon_style)
        .into(),
    );

    container(
        row![
            db_engine_icon::view(Engine::Redis, 14.0),
            Space::new().width(7),
            text(&tab.conn_name)
                .font(theme::FONT_BODY)
                .size(12)
                .color(palette.text2),
            Space::new().width(12),
            search_wrap,
            Space::new().width(Fill),
            text(count)
                .font(theme::FONT_BODY)
                .size(11)
                .color(palette.text3),
            Space::new().width(8),
            refresh_btn,
            new_btn,
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(34.0))
    .padding([0, 8])
    .style(move |_| container::Style {
        background: Some(Background::Color(palette.panel2)),
        border: Border {
            color: palette.border,
            width: 1.0,
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn error_banner(message: &str) -> Element<'_, RedisMsg> {
    let palette = theme::palette();
    let bg = Color {
        a: 0.12,
        ..palette.error
    };
    container(
        row![
            text(labels::ICON_ERROR_OUTLINE)
                .font(theme::FONT_ICON)
                .size(13)
                .color(palette.error),
            Space::new().width(6),
            text(message)
                .font(theme::FONT_MONO)
                .size(11)
                .color(palette.error)
                .width(Fill),
            button(
                text(labels::ICON_CLOSE)
                    .font(theme::FONT_ICON)
                    .size(13)
                    .color(palette.error),
            )
            .padding(2)
            .on_press(RedisMsg::DismissError)
            .style(ghost_icon_style),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([6, 10])
    .style(move |_| container::Style {
        background: Some(Background::Color(bg)),
        ..Default::default()
    })
    .into()
}

fn header<'a>() -> Element<'a, RedisMsg> {
    let palette = theme::palette();
    let label = |t: &'static str| {
        text(t)
            .font(theme::FONT_BODY)
            .size(10.5)
            .color(palette.text3)
    };
    container(
        row![
            Space::new().width(10),
            container(label(labels::REDIS_COLUMN_KEY)).width(Length::Fixed(COL_KEY_WIDTH)),
            container(label(labels::REDIS_COLUMN_VALUE)).width(Fill),
            container(label(labels::REDIS_COLUMN_TYPE)).width(Length::Fixed(COL_TYPE_WIDTH)),
            container(label(labels::REDIS_COLUMN_TTL)).width(Length::Fixed(COL_TTL_WIDTH)),
            Space::new().width(30),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .height(Length::Fixed(26.0))
    .style(move |_| container::Style {
        background: Some(Background::Color(palette.panel2)),
        border: Border {
            color: palette.border,
            width: 1.0,
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn table(tab: &RedisTab) -> Element<'_, RedisMsg> {
    let palette = theme::palette();
    let mut list = column![];
    if let Some(d) = &tab.draft {
        list = list.push(draft_row(d));
    }
    for (idx, entry) in tab.entries.iter().enumerate() {
        list = list.push(entry_row(tab, entry, idx));
    }
    if tab.loading {
        list = list.push(
            container(
                text(labels::DB_LOADING)
                    .font(theme::FONT_BODY)
                    .size(11.5)
                    .color(palette.text3),
            )
            .width(Fill)
            .center_x(Fill)
            .padding(14),
        );
    } else if tab.entries.is_empty() && tab.loaded && tab.draft.is_none() {
        let empty = if tab.pattern.is_empty() {
            labels::REDIS_NO_KEYS.to_string()
        } else {
            labels::redis_no_keys_match(&tab.pattern)
        };
        list = list.push(
            container(
                text(empty)
                    .font(theme::FONT_BODY)
                    .size(11.5)
                    .color(palette.text3),
            )
            .padding(16),
        );
    } else if tab.cursor != "0" {
        list = list.push(
            container(pane_close_dialog::action(
                labels::REDIS_LOAD_MORE,
                ButtonKind::Outline,
                Some(RedisMsg::LoadMore),
            ))
            .width(Fill)
            .center_x(Fill)
            .padding(8),
        );
    }
    scrollable(list.width(Fill)).height(Fill).into()
}

fn entry_row<'a>(
    tab: &'a RedisTab,
    entry: &'a RedisKeyEntry,
    index: usize,
) -> Element<'a, RedisMsg> {
    let palette = theme::palette();
    let flashing = tab.flash_key.as_deref() == Some(entry.key.as_str());
    let expanded = tab.expanded_key.as_deref() == Some(entry.key.as_str());

    let key_cell: Element<'a, RedisMsg> = if tab.rename_key.as_deref() == Some(entry.key.as_str()) {
        inline_editor(
            &tab.rename_input,
            RedisMsg::KeyInput,
            RedisMsg::CommitKeyEdit,
        )
    } else {
        button(
            container(
                text(&entry.key)
                    .font(theme::FONT_MONO)
                    .size(12)
                    .color(palette.text)
                    .wrapping(text::Wrapping::None),
            )
            .width(Fill)
            .clip(true),
        )
        .width(Fill)
        .padding([4, 4])
        .on_press(RedisMsg::StartKeyEdit(entry.key.clone()))
        .style(hover_icon_style)
        .into()
    };

    let value_cell: Element<'a, RedisMsg> =
        if tab.editing_key.as_deref() == Some(entry.key.as_str()) {
            if tab.editing_busy {
                text("…")
                    .font(theme::FONT_MONO)
                    .size(12)
                    .color(palette.text2)
                    .into()
            } else {
                inline_editor(
                    &tab.editing_value,
                    RedisMsg::StringInput,
                    RedisMsg::CommitStringEdit,
                )
            }
        } else {
            let on_tap = if !entry.kind.editable() {
                None
            } else if entry.kind == RedisValueKind::String {
                Some(RedisMsg::StartStringEdit(entry.key.clone()))
            } else {
                Some(RedisMsg::ToggleExpanded(entry.key.clone()))
            };
            let preview = entry.preview.replace('\n', "␤");
            button(
                container(
                    text(preview)
                        .font(theme::FONT_MONO)
                        .size(12)
                        .color(palette.text2)
                        .wrapping(text::Wrapping::None),
                )
                .width(Fill)
                .clip(true),
            )
            .width(Fill)
            .padding([4, 4])
            .on_press_maybe(on_tap)
            .style(hover_icon_style)
            .into()
        };

    let ttl_cell: Element<'a, RedisMsg> = if tab.ttl_key.as_deref() == Some(entry.key.as_str()) {
        inline_editor(&tab.ttl_input, RedisMsg::TtlInput, RedisMsg::CommitTtlEdit)
    } else {
        let ttl_text = if entry.ttl < 0 {
            "∞".to_string()
        } else {
            entry.ttl.to_string()
        };
        let ttl_color = if entry.ttl < 0 {
            palette.text4
        } else {
            palette.text2
        };
        button(
            text(ttl_text)
                .font(theme::FONT_MONO)
                .size(12)
                .color(ttl_color),
        )
        .width(Fill)
        .padding([4, 4])
        .on_press(RedisMsg::StartTtlEdit(entry.key.clone()))
        .style(hover_icon_style)
        .into()
    };

    let bg = if flashing {
        Some(Color {
            a: 0.14,
            ..palette.accent
        })
    } else if index % 2 == 1 {
        Some(Color {
            a: 0.35,
            ..palette.panel2
        })
    } else {
        None
    };

    let row_widget = container(
        row![
            Space::new().width(10),
            container(key_cell).width(Length::Fixed(COL_KEY_WIDTH)),
            container(value_cell).width(Fill),
            container(
                text(entry.kind.label())
                    .font(theme::FONT_BODY)
                    .size(10.5)
                    .color(kind_color(entry.kind)),
            )
            .width(Length::Fixed(COL_TYPE_WIDTH)),
            container(ttl_cell).width(Length::Fixed(COL_TTL_WIDTH)),
            container(
                button(
                    text(labels::ICON_DELETE)
                        .font(theme::FONT_ICON)
                        .size(13)
                        .color(palette.text4),
                )
                .padding(3)
                .on_press(RedisMsg::AskDelete(entry.key.clone()))
                .style(hover_icon_style),
            )
            .width(Length::Fixed(30.0)),
        ]
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .style(move |_| container::Style {
        background: bg.map(Background::Color),
        border: Border {
            color: Color {
                a: 0.5,
                ..palette.border
            },
            width: 0.5,
            ..Default::default()
        },
        ..Default::default()
    });

    if !expanded {
        return container(row_widget).padding([1, 0]).into();
    }

    column![row_widget, expanded_editor(tab, &entry.key)].into()
}

fn expanded_editor<'a>(tab: &'a RedisTab, key: &'a str) -> Element<'a, RedisMsg> {
    let palette = theme::palette();
    let inner: Element<'a, RedisMsg> = if tab.expanded_loading {
        container(
            text(labels::REDIS_LOADING_FULL_VALUE)
                .font(theme::FONT_BODY)
                .size(11)
                .color(palette.text3),
        )
        .padding(8)
        .into()
    } else {
        let mut col = column![text_editor(&tab.expanded_content)
            .on_action(RedisMsg::ExpandedEditor)
            .font(theme::FONT_MONO)
            .size(12)
            .padding([6, 8])
            .height(Length::Fixed(180.0))
            .style(move |_: &Theme, _| text_editor::Style {
                background: Background::Color(palette.bg),
                border: Border {
                    color: palette.border,
                    width: 1.0,
                    radius: 5.0.into(),
                },
                placeholder: palette.text4,
                value: palette.text,
                selection: theme::TERM_SELECTION_BG,
            })];
        if let Some(err) = &tab.expanded_error {
            col = col.push(Space::new().height(6)).push(
                text(err)
                    .font(theme::FONT_MONO)
                    .size(11)
                    .color(palette.error),
            );
        }
        col = col.push(Space::new().height(8)).push(
            row![
                Space::new().width(Fill),
                pane_close_dialog::action(
                    labels::DIALOG_CANCEL,
                    ButtonKind::Outline,
                    Some(RedisMsg::CloseEditors),
                ),
                Space::new().width(6),
                pane_close_dialog::action(
                    labels::DB_DIALOG_SAVE,
                    ButtonKind::Primary,
                    Some(RedisMsg::CommitExpanded(key.to_string())),
                ),
            ]
            .align_y(Alignment::Center),
        );
        col.into()
    };

    container(inner)
        .width(Fill)
        .padding([8, 10])
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.panel3)),
            border: Border {
                color: palette.border,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn draft_row(draft: &NewKeyDraft) -> Element<'_, RedisMsg> {
    let palette = theme::palette();
    let kind_button = button(
        row![
            text(draft.kind.label())
                .font(theme::FONT_BODY)
                .size(11)
                .color(palette.text),
            Space::new().width(4),
            text(labels::ICON_EXPAND_MORE)
                .font(theme::FONT_ICON)
                .size(12)
                .color(palette.text3),
        ]
        .align_y(Alignment::Center),
    )
    .padding([5, 10])
    .on_press(RedisMsg::ToggleDraftKindMenu)
    .style(|_: &Theme, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then(|| Background::Color(theme::palette().panel3)),
        text_color: theme::palette().text,
        border: Border {
            color: theme::palette().border,
            width: 1.0,
            radius: 5.0.into(),
        },
        ..Default::default()
    });

    let kind_menu = draft.kind_menu_open.then(|| {
        app_menu::menu(
            RedisValueKind::CREATABLE
                .into_iter()
                .map(|k| Item::Entry(Entry::new(k.label(), RedisMsg::DraftKind(k))))
                .collect(),
            120.0,
        )
    });

    let top = row![
        container(
            text_input(labels::REDIS_KEY_FIELD_HINT, &draft.key)
                .on_input(RedisMsg::DraftKey)
                .font(theme::FONT_MONO)
                .size(12)
                .padding([5, 8])
                .style(draft_input_style),
        )
        .width(Length::Fixed(COL_KEY_WIDTH + 30.0)),
        Space::new().width(8),
        Anchored::new(kind_button, kind_menu, RedisMsg::CloseDraftKindMenu),
        Space::new().width(8),
        container(
            text_input(labels::REDIS_TTL_FIELD_HINT, &draft.ttl)
                .on_input(RedisMsg::DraftTtl)
                .font(theme::FONT_MONO)
                .size(12)
                .padding([5, 8])
                .style(draft_input_style),
        )
        .width(Length::Fixed(110.0)),
    ]
    .align_y(Alignment::Center);

    let hint = match draft.kind {
        RedisValueKind::String => labels::REDIS_VALUE_FIELD_HINT,
        RedisValueKind::Hash => "{\"field\": \"value\"}",
        RedisValueKind::Zset => "[{\"value\": \"member\", \"score\": 1}]",
        _ => "[\"item1\", \"item2\"]",
    };

    let value_height = if draft.kind == RedisValueKind::String {
        34.0
    } else {
        110.0
    };
    let value_editor = text_editor(&draft.value)
        .placeholder(hint)
        .on_action(RedisMsg::DraftValue)
        .font(theme::FONT_MONO)
        .size(12)
        .padding([5, 8])
        .height(Length::Fixed(value_height))
        .style(move |_: &Theme, _| text_editor::Style {
            background: Background::Color(palette.bg),
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: 5.0.into(),
            },
            placeholder: palette.text4,
            value: palette.text,
            selection: theme::TERM_SELECTION_BG,
        });

    let mut col = column![top, Space::new().height(6), value_editor];
    if let Some(err) = &draft.error {
        col = col.push(Space::new().height(6)).push(
            text(err)
                .font(theme::FONT_MONO)
                .size(11)
                .color(palette.error),
        );
    }
    col = col.push(Space::new().height(8)).push(
        row![
            Space::new().width(Fill),
            pane_close_dialog::action(
                labels::DIALOG_CANCEL,
                ButtonKind::Outline,
                Some(RedisMsg::CloseDraft),
            ),
            Space::new().width(6),
            pane_close_dialog::action(
                labels::REDIS_ADD_KEY,
                ButtonKind::Primary,
                (!draft.busy).then_some(RedisMsg::CommitDraft),
            ),
        ]
        .align_y(Alignment::Center),
    );

    let tint = Color {
        a: 0.25,
        ..palette.accent_soft
    };
    container(col)
        .width(Fill)
        .padding([8, 10])
        .style(move |_| container::Style {
            background: Some(Background::Color(tint)),
            border: Border {
                color: palette.border,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn inline_editor<'a>(
    value: &'a str,
    on_input: fn(String) -> RedisMsg,
    on_submit: RedisMsg,
) -> Element<'a, RedisMsg> {
    row![
        text_input("", value)
            .on_input(on_input)
            .on_submit(on_submit.clone())
            .font(theme::FONT_MONO)
            .size(12)
            .padding([3, 6])
            .width(Fill)
            .style(draft_input_style),
        Space::new().width(2),
        button(
            text(labels::ICON_CHECK)
                .font(theme::FONT_ICON)
                .size(12)
                .color(theme::palette().online),
        )
        .padding(3)
        .on_press(on_submit)
        .style(hover_icon_style),
        button(
            text(labels::ICON_CLOSE)
                .font(theme::FONT_ICON)
                .size(12)
                .color(theme::palette().text3),
        )
        .padding(3)
        .on_press(RedisMsg::CloseEditors)
        .style(hover_icon_style),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn kind_color(kind: RedisValueKind) -> Color {
    match kind {
        RedisValueKind::String => theme::REDIS_KIND_STRING_COLOR,
        RedisValueKind::Hash => theme::REDIS_KIND_HASH_COLOR,
        RedisValueKind::List => theme::REDIS_KIND_LIST_COLOR,
        RedisValueKind::Set => theme::REDIS_KIND_SET_COLOR,
        RedisValueKind::Zset => theme::REDIS_KIND_ZSET_COLOR,
        RedisValueKind::Other => theme::palette().text3,
    }
}

fn action_tooltip<'a>(label: &'static str, widget: Element<'a, RedisMsg>) -> Element<'a, RedisMsg> {
    tooltip(
        widget,
        pane_chrome::tip_bubble(label),
        tooltip::Position::Bottom,
    )
    .into()
}

fn ghost_icon_style(_: &Theme, _: button::Status) -> button::Style {
    button::Style::default()
}

fn hover_icon_style(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then(|| Background::Color(theme::palette().panel3)),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn plain_input_style(_: &Theme, _: text_input::Status) -> text_input::Style {
    let palette = theme::palette();
    text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: palette.text3,
        placeholder: palette.text4,
        value: palette.text,
        selection: theme::TERM_SELECTION_BG,
    }
}

fn draft_input_style(_: &Theme, _: text_input::Status) -> text_input::Style {
    let palette = theme::palette();
    text_input::Style {
        background: Background::Color(palette.bg),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 5.0.into(),
        },
        icon: palette.text3,
        placeholder: palette.text4,
        value: palette.text,
        selection: theme::TERM_SELECTION_BG,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_db::SecretStore;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn renders_empty_populated_draft_and_expanded_states() {
        let mut tab = RedisTab::new(
            "cache".into(),
            None,
            SecretStore::new(PathBuf::from("/nonexistent")),
            BTreeMap::new(),
        );
        let _ = view(&tab);

        tab.loaded = true;
        tab.error = Some("ERR".into());
        tab.entries = vec![
            RedisKeyEntry {
                key: "user:1".into(),
                kind: RedisValueKind::String,
                ttl: 60,
                preview: "Ana".into(),
            },
            RedisKeyEntry {
                key: "session".into(),
                kind: RedisValueKind::Hash,
                ttl: -1,
                preview: "{a: 1}".into(),
            },
        ];
        let _ = view(&tab);

        tab.editing_key = Some("user:1".into());
        let _ = view(&tab);
        tab.editing_key = None;

        tab.expanded_key = Some("session".into());
        tab.expanded_content = text_editor::Content::with_text("{\"a\": \"1\"}");
        let _ = view(&tab);

        tab.draft = Some(NewKeyDraft::default());
        let _ = view(&tab);
    }
}
