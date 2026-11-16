//! Interface do navegador de coleções MongoDB (`db_mongo_view.dart`).

use gsh_db::Engine;
use iced::widget::{
    button, column, container, row, scrollable, text, text_editor, text_input, tooltip, Space,
};
use iced::{Alignment, Background, Border, Color, Element, Fill, Length, Theme};

use crate::db_mongo::{MongoDoc, MongoMsg, MongoTab};
use crate::pane_close_dialog::{self, ButtonKind};
use crate::{db_engine_icon, editor_highlight, labels, pane_chrome, theme};

pub fn view(tab: &MongoTab) -> Element<'_, MongoMsg> {
    let mut col = column![toolbar(tab)];
    if let Some(err) = &tab.error {
        col = col.push(error_banner(err));
    }
    col = col.push(list(tab));
    container(col)
        .width(Fill)
        .height(Fill)
        .style(|_| container::Style {
            background: Some(Background::Color(theme::palette().panel)),
            ..Default::default()
        })
        .into()
}

fn toolbar(tab: &MongoTab) -> Element<'_, MongoMsg> {
    let palette = theme::palette();
    let search_icon = button(
        text(labels::ICON_SEARCH_TAB)
            .font(theme::FONT_ICON)
            .size(13)
            .color(palette.text3),
    )
    .padding(2)
    .on_press(MongoMsg::ApplyFilter)
    .style(ghost_icon_style);

    let filter_input = text_input(labels::MONGO_FILTER_HINT, &tab.filter_input)
        .on_input(MongoMsg::FilterChanged)
        .on_submit(MongoMsg::ApplyFilter)
        .font(theme::FONT_MONO)
        .size(11.5)
        .padding([4, 6])
        .style(plain_input_style);

    let mut filter_box = row![search_icon, Space::new().width(4), filter_input]
        .align_y(Alignment::Center)
        .width(Fill);
    if !tab.filter_input.is_empty() {
        filter_box = filter_box.push(
            button(
                text(labels::ICON_CLOSE)
                    .font(theme::FONT_ICON)
                    .size(12)
                    .color(palette.text3),
            )
            .padding(2)
            .on_press(MongoMsg::ClearFilter)
            .style(ghost_icon_style),
        );
    }

    let filter_wrap = container(filter_box)
        .max_width(340.0)
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
        labels::mongo_doc_count(tab.docs.len()),
        if tab.has_more { "+" } else { "" }
    );

    let refresh_btn = action_tooltip(
        labels::MONGO_REFRESH,
        button(
            text(labels::ICON_REFRESH)
                .font(theme::FONT_ICON)
                .size(15)
                .color(palette.text3),
        )
        .padding(4)
        .on_press_maybe((!tab.loading).then_some(MongoMsg::Refresh))
        .style(hover_icon_style)
        .into(),
    );

    let insert_btn = action_tooltip(
        labels::MONGO_INSERT_DOC,
        button(
            text(labels::ICON_PLUS)
                .font(theme::FONT_ICON)
                .size(15)
                .color(palette.text3),
        )
        .padding(4)
        .on_press(MongoMsg::OpenDraft)
        .style(hover_icon_style)
        .into(),
    );

    container(
        row![
            db_engine_icon::view(Engine::Mongo, 14.0),
            Space::new().width(7),
            text(format!("{} / {}", tab.conn_name, tab.collection))
                .font(theme::FONT_BODY)
                .size(12)
                .color(palette.text2),
            Space::new().width(12),
            container(filter_wrap).width(Fill),
            Space::new().width(8),
            text(count)
                .font(theme::FONT_BODY)
                .size(11)
                .color(palette.text3),
            Space::new().width(8),
            refresh_btn,
            insert_btn,
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

fn error_banner(message: &str) -> Element<'_, MongoMsg> {
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
            .on_press(MongoMsg::DismissError)
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

fn list(tab: &MongoTab) -> Element<'_, MongoMsg> {
    let palette = theme::palette();
    let mut items = column![].spacing(theme::MONGO_CARD_GAP).padding(8);
    if tab.draft_open {
        items = items.push(draft_card(tab));
    }
    for (ix, doc) in tab.docs.iter().enumerate() {
        items = items.push(doc_card(tab, doc, ix));
    }
    if tab.loading {
        items = items.push(
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
    } else if tab.docs.is_empty() && tab.loaded && !tab.draft_open {
        let empty = if tab.filter.is_empty() {
            labels::MONGO_NO_DOCUMENTS
        } else {
            labels::MONGO_NO_DOCUMENTS_MATCH
        };
        items = items.push(
            container(
                text(empty)
                    .font(theme::FONT_BODY)
                    .size(11.5)
                    .color(palette.text3),
            )
            .padding(8),
        );
    } else if tab.has_more {
        items = items.push(
            container(pane_close_dialog::action(
                labels::MONGO_LOAD_MORE,
                ButtonKind::Outline,
                Some(MongoMsg::LoadMore),
            ))
            .width(Fill)
            .center_x(Fill)
            .padding(8),
        );
    }
    scrollable(items.width(Fill)).height(Fill).into()
}

fn doc_card<'a>(tab: &'a MongoTab, doc: &'a MongoDoc, ix: usize) -> Element<'a, MongoMsg> {
    let palette = theme::palette();
    let editing = tab.editing_ix == Some(ix);

    let mut header = row![container(
        text(format!("_id: {}", MongoTab::id_label(&doc.doc)))
            .font(theme::FONT_MONO)
            .size(10.5)
            .color(palette.text3)
            .wrapping(text::Wrapping::None),
    )
    .width(Fill)
    .clip(true),]
    .align_y(Alignment::Center);

    if !editing {
        header = header
            .push(action_tooltip(
                labels::MONGO_EDIT,
                button(
                    text(labels::ICON_EDIT)
                        .font(theme::FONT_ICON)
                        .size(13)
                        .color(palette.text4),
                )
                .padding(3)
                .on_press(MongoMsg::StartEdit(ix))
                .style(hover_icon_style)
                .into(),
            ))
            .push(action_tooltip(
                labels::DB_DIALOG_DELETE,
                button(
                    text(labels::ICON_DELETE)
                        .font(theme::FONT_ICON)
                        .size(13)
                        .color(palette.text4),
                )
                .padding(3)
                .on_press(MongoMsg::AskDelete(ix))
                .style(hover_icon_style)
                .into(),
            ));
    }

    let card_header = container(header)
        .width(Fill)
        .padding([4, 10])
        .style(move |_| container::Style {
            border: Border {
                color: palette.border,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });

    let body: Element<'a, MongoMsg> = if editing {
        let mut col = column![text_editor(&tab.editing_content)
            .on_action(MongoMsg::EditEditor)
            .font(theme::FONT_MONO)
            .size(12)
            .padding([6, 8])
            .height(Length::Fixed(220.0))
            .style(code_editor_style)];
        if let Some(err) = &tab.editing_error {
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
                    Some(MongoMsg::CloseEditors),
                ),
                Space::new().width(6),
                pane_close_dialog::action(
                    labels::DB_DIALOG_SAVE,
                    ButtonKind::Primary,
                    (!tab.editing_busy).then_some(MongoMsg::CommitEdit),
                ),
            ]
            .align_y(Alignment::Center),
        );
        col.into()
    } else {
        text_editor(&doc.content)
            .on_action(move |action| MongoMsg::DocSelectAction(ix, action))
            .font(theme::FONT_MONO)
            .size(12)
            .line_height(1.4)
            .padding(0)
            .height(Length::Shrink)
            .style(readonly_editor_style)
            .highlight_with::<editor_highlight::SyntaxHighlighter>(
                editor_highlight::Settings::new("json".to_string(), doc.json.clone()),
                editor_highlight::format,
            )
            .into()
    };

    container(column![card_header, container(body).padding(10)])
        .width(Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(palette.panel2)),
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: 7.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn draft_card(tab: &MongoTab) -> Element<'_, MongoMsg> {
    let palette = theme::palette();
    let mut col = column![text_editor(&tab.draft_content)
        .on_action(MongoMsg::DraftEditor)
        .font(theme::FONT_MONO)
        .size(12)
        .padding([6, 8])
        .height(Length::Fixed(180.0))
        .style(code_editor_style)];
    if let Some(err) = &tab.draft_error {
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
                Some(MongoMsg::CloseEditors),
            ),
            Space::new().width(6),
            pane_close_dialog::action(
                labels::MONGO_INSERT,
                ButtonKind::Primary,
                (!tab.draft_busy).then_some(MongoMsg::CommitDraft),
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
        .padding(10)
        .style(move |_| container::Style {
            background: Some(Background::Color(tint)),
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: 7.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn action_tooltip<'a>(label: &'static str, widget: Element<'a, MongoMsg>) -> Element<'a, MongoMsg> {
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

fn code_editor_style(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    let palette = theme::palette();
    text_editor::Style {
        background: Background::Color(palette.bg),
        border: Border {
            color: palette.border,
            width: 1.0,
            radius: 5.0.into(),
        },
        placeholder: palette.text4,
        value: palette.text,
        selection: theme::TERM_SELECTION_BG,
    }
}

fn readonly_editor_style(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    let palette = theme::palette();
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        placeholder: palette.text4,
        value: palette.text2,
        selection: theme::TERM_SELECTION_BG,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_db::mongo_store::MongoDatabaseStore;
    use gsh_db::SecretStore;
    use serde_json::Map;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    fn renders_empty_docs_edit_and_draft_states() {
        let mut tab = MongoTab::new(
            "atlas".into(),
            "users".into(),
            None,
            SecretStore::new(PathBuf::from("/nonexistent")),
            MongoDatabaseStore::new(PathBuf::from("/nonexistent/mongo.json")),
            BTreeMap::new(),
        );
        let _ = view(&tab);

        tab.loaded = true;
        tab.error = Some("ERR".into());
        let mut doc = Map::new();
        doc.insert("_id".into(), serde_json::json!({"$oid": "abc"}));
        doc.insert("name".into(), serde_json::json!("Ana"));
        tab.docs.push(MongoDoc::from_map(doc));
        let _ = view(&tab);

        let _ = tab.update(MongoMsg::StartEdit(0));
        let _ = view(&tab);
        let _ = tab.update(MongoMsg::OpenDraft);
        let _ = view(&tab);
    }
}
