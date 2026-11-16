//! Vista da aba `.dbq` (`db_query_view.dart`): barra de 36px, editor, divisor arrastável, área do
//! resultado (grade, JSON, erro, avisos) e rodapé de 26px.

use gsh_db::{Cell, Connection, QueryResult};
use iced::widget::{
    button, column, container, responsive, row, scrollable, stack, text, text_editor, text_input,
    Space,
};
use iced::{
    alignment::{Horizontal, Vertical},
    Alignment, Background, Border, Color, Element, Fill, Length, Padding, Size, Theme,
};

use crate::anchored::Anchored;
use crate::app_menu::{self, Entry, Item, Leading};
use crate::db_query::{editor_height, DbQuery, QueryMsg, SAVE_NAME_ID};
use crate::drag_handle::{Axis, DragHandle};
use crate::pane_close_dialog::{self, ButtonKind};
use crate::{db_engine_icon, editor_highlight, labels, theme};

pub fn view<'a>(query: &'a DbQuery, connections: &'a [Connection]) -> Element<'a, QueryMsg> {
    let palette = theme::palette();
    container(
        column![
            top_bar(query, connections),
            responsive(move |size| body(query, size)),
            footer(query),
        ]
        .width(Fill)
        .height(Fill),
    )
    .width(Fill)
    .height(Fill)
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(palette.panel)),
        ..Default::default()
    })
    .into()
}

/// Diálogos da aba (nome do "Salvar como" e erro ao salvar), para o shell empilhar.
pub fn overlay(query: &DbQuery) -> Option<Element<'_, QueryMsg>> {
    if let Some(message) = &query.save_error {
        return Some(pane_close_dialog::notice(
            labels::DB_QUERY_COULD_NOT_SAVE,
            message.clone(),
            QueryMsg::DismissSaveError,
            QueryMsg::Noop,
        ));
    }
    let dialog = query.save_dialog.as_ref()?;
    let title = text(labels::DB_QUERY_SAVE_AS)
        .font(theme::FONT_TITLE)
        .size(theme::DIALOG_TITLE_SIZE)
        .color(theme::palette().text);
    let input = text_input("", &dialog.name)
        .id(SAVE_NAME_ID)
        .on_input(QueryMsg::SaveName)
        .on_submit(QueryMsg::SaveSubmit)
        .font(theme::FONT_MONO)
        .size(theme::DB_DIALOG_INPUT_SIZE)
        .padding(theme::DB_DIALOG_INPUT_PADDING)
        .style(|_: &Theme, _| {
            let palette = theme::palette();
            text_input::Style {
                background: Background::Color(Color::TRANSPARENT),
                border: Border {
                    color: palette.border,
                    width: 1.0,
                    radius: 6.0.into(),
                },
                icon: palette.text3,
                placeholder: palette.text4,
                value: palette.text,
                selection: theme::TERM_SELECTION_BG,
            }
        });
    let actions = row![
        pane_close_dialog::action(
            labels::DIALOG_CANCEL,
            ButtonKind::Ghost,
            Some(QueryMsg::SaveCancel)
        ),
        pane_close_dialog::action(
            labels::DB_DIALOG_SAVE,
            ButtonKind::Primary,
            Some(QueryMsg::SaveSubmit)
        ),
    ]
    .spacing(theme::DIALOG_ACTIONS_GAP)
    .align_y(Alignment::Center);
    Some(pane_close_dialog::form(
        title.into(),
        container(input)
            .width(Length::Fixed(theme::DIALOG_CONTENT_WIDTH - 20.0))
            .into(),
        actions.into(),
        None,
        QueryMsg::SaveCancel,
        QueryMsg::Noop,
    ))
}

fn hairline_h<'a>(color: Color) -> Element<'a, QueryMsg> {
    container(Space::new().width(Fill).height(1.0))
        .width(Fill)
        .height(1.0)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            ..Default::default()
        })
        .into()
}

fn hairline_v<'a>(color: Color) -> Element<'a, QueryMsg> {
    container(Space::new().width(1.0).height(Fill))
        .width(1.0)
        .height(Fill)
        .style(move |_: &Theme| container::Style {
            background: Some(Background::Color(color)),
            ..Default::default()
        })
        .into()
}

// ----------------------------------------------------------------------------- barra de cima

fn top_bar<'a>(query: &'a DbQuery, connections: &'a [Connection]) -> Element<'a, QueryMsg> {
    let palette = theme::palette();
    let conn = query.connection(connections);

    let leading: Element<'a, QueryMsg> = match conn {
        Some(conn) => db_engine_icon::view(conn.engine, theme::DB_QUERY_BAR_ICON),
        None => text(labels::ICON_STORAGE)
            .font(theme::FONT_ICON)
            .size(theme::DB_QUERY_BAR_ICON)
            .color(palette.accent)
            .into(),
    };
    let picker = Anchored::new(
        button(
            row![
                text(
                    query
                        .conn
                        .as_deref()
                        .unwrap_or(labels::DB_QUERY_SELECT_DATABASE)
                )
                .font(theme::FONT_BODY)
                .size(theme::DB_QUERY_PICK_SIZE)
                .color(if query.conn.is_none() {
                    palette.warn
                } else {
                    palette.text2
                }),
                Space::new().width(4.0),
                text(labels::ICON_CHEVRON_DOWN)
                    .font(theme::FONT_ICON)
                    .size(theme::DB_QUERY_PICK_ARROW)
                    .color(palette.text3),
            ]
            .align_y(Alignment::Center),
        )
        .padding([
            theme::DB_QUERY_PICK_PADDING[0],
            theme::DB_QUERY_PICK_PADDING[1],
        ])
        .on_press(QueryMsg::OpenPicker)
        .style(|_: &Theme, status| {
            let palette = theme::palette();
            button::Style {
                background: Some(Background::Color(
                    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                        palette.panel
                    } else {
                        palette.panel3
                    },
                )),
                text_color: palette.text2,
                border: Border {
                    radius: theme::DB_QUERY_PICK_RADIUS.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        }),
        query.picker_open.then(|| picker_menu(query, connections)),
        QueryMsg::ClosePicker,
    );

    let mut bar = row![leading, Space::new().width(theme::DB_QUERY_BAR_GAP), picker]
        .align_y(Alignment::Center);
    if let Some(conn) = conn {
        bar = bar.push(Space::new().width(6.0)).push(
            container(
                text(conn.engine.label())
                    .font(theme::FONT_BODY)
                    .size(theme::DB_QUERY_ENGINE_CHIP_SIZE)
                    .color(palette.text3),
            )
            .padding([
                theme::DB_QUERY_ENGINE_CHIP_PADDING[0],
                theme::DB_QUERY_ENGINE_CHIP_PADDING[1],
            ])
            .style(|_: &Theme| container::Style {
                border: Border {
                    color: theme::palette().border2,
                    width: 1.0,
                    radius: theme::DB_CHIP_RADIUS.into(),
                },
                ..Default::default()
            }),
        );
    }
    bar = bar.push(Space::new().width(Fill)).push(run_button(query));

    column![
        container(bar)
            .height(theme::DB_QUERY_BAR_HEIGHT - 1.0)
            .align_y(Vertical::Center)
            .padding([0.0, theme::DB_QUERY_BAR_PADDING]),
        hairline_h(palette.border),
    ]
    .height(theme::DB_QUERY_BAR_HEIGHT)
    .into()
}

fn picker_menu<'a>(query: &DbQuery, connections: &'a [Connection]) -> Element<'a, QueryMsg> {
    // só os motores SQL: a aba executa SQL (conexões com `agents: false` aparecem: o gate é só da CLI)
    let mut items: Vec<Item<QueryMsg>> = connections
        .iter()
        .filter(|c| c.engine.is_sql())
        .map(|c| {
            Item::Entry(
                Entry::new(
                    format!("{} · {}", c.name, c.engine.label()),
                    QueryMsg::PickConnection(c.name.clone()),
                )
                .leading(Leading::Engine(c.engine))
                .selected(query.conn.as_deref() == Some(c.name.as_str())),
            )
        })
        .collect();
    if items.is_empty() {
        items.push(Item::Entry(Entry {
            label: labels::DB_QUERY_NO_SQL_CONNECTIONS.to_string(),
            ..Entry::disabled(None)
        }));
    }
    app_menu::menu(items, 200.0)
}

fn run_button(query: &DbQuery) -> Element<'_, QueryMsg> {
    let palette = theme::palette();
    // o fundo troca com o estado e o texto sai do fundo efetivo: fixo em branco, o desabilitado
    // ficava ilegível no tema claro
    let background = if query.conn.is_none() {
        palette.panel3
    } else {
        palette.accent
    };
    let foreground = theme::on_color(background);
    let selecting = query.editor.content.selection().is_some();
    let label = if query.running {
        labels::DB_QUERY_RUNNING
    } else if selecting {
        labels::DB_QUERY_RUN_SELECTION
    } else {
        labels::DB_QUERY_RUN
    };
    let content = row![
        text(if query.running {
            labels::ICON_HOURGLASS_TOP
        } else {
            labels::ICON_PLAY_ARROW
        })
        .font(theme::FONT_ICON)
        .size(theme::DB_QUERY_RUN_ICON)
        .color(foreground),
        Space::new().width(4.0),
        text(label)
            .font(theme::FONT_BODY)
            .size(theme::DB_QUERY_RUN_SIZE)
            .color(foreground),
        Space::new().width(6.0),
        text(labels::DB_QUERY_RUN_SHORTCUT)
            .font(theme::FONT_BODY)
            .size(theme::DB_QUERY_RUN_SHORTCUT_SIZE)
            .color(Color {
                a: theme::DB_QUERY_RUN_SHORTCUT_ALPHA,
                ..foreground
            }),
    ]
    .align_y(Alignment::Center);
    let enabled = !query.running && query.conn.is_some();
    button(content)
        .padding([
            theme::DB_QUERY_RUN_PADDING[0],
            theme::DB_QUERY_RUN_PADDING[1],
        ])
        .on_press_maybe(enabled.then_some(QueryMsg::Run))
        .style(move |_: &Theme, status| {
            let hovered =
                enabled && matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(Background::Color(if hovered {
                    Color {
                        a: theme::DB_QUERY_RUN_HOVER_ALPHA,
                        ..theme::palette().accent
                    }
                } else {
                    background
                })),
                text_color: foreground,
                border: Border {
                    radius: theme::DB_QUERY_RUN_RADIUS.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

// --------------------------------------------------------------------------------- corpo

fn body(query: &DbQuery, size: Size) -> Element<'_, QueryMsg> {
    query.height.set(size.height);
    let editor_h = editor_height(size.height, query.view.split);
    let palette = theme::palette();
    let divider = DragHandle::new(
        container(
            container(Space::new().width(Fill).height(1.0))
                .width(Fill)
                .height(1.0)
                .style(move |_: &Theme| container::Style {
                    background: Some(Background::Color(palette.border2)),
                    ..Default::default()
                }),
        )
        .width(Fill)
        .height(theme::DB_QUERY_DIVIDER_HEIGHT)
        .align_y(Vertical::Center),
        Axis::Vertical,
        QueryMsg::DragSplit,
    );
    column![
        container(query.editor.view().map(QueryMsg::Editor))
            .width(Fill)
            .height(editor_h),
        divider,
        container(result_area(query, size)).width(Fill).height(Fill),
    ]
    .into()
}

fn centered<'a>(message: String, size: f32, color: Color) -> Element<'a, QueryMsg> {
    container(text(message).font(theme::FONT_BODY).size(size).color(color))
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill)
        .into()
}

fn result_area(query: &DbQuery, size: Size) -> Element<'_, QueryMsg> {
    let palette = theme::palette();
    let view = &query.view;
    if let Some(error) = &view.error {
        let mut column = column![row![
            text(labels::ICON_ERROR_OUTLINE)
                .font(theme::FONT_ICON)
                .size(theme::DB_QUERY_ERROR_ICON)
                .color(palette.error),
            Space::new().width(6.0),
            text(error.kind())
                .font(theme::FONT_BODY)
                .size(theme::DB_QUERY_ERROR_KIND_SIZE)
                .color(palette.error),
        ]
        .align_y(Alignment::Center)]
        .spacing(theme::DB_QUERY_ERROR_GAP);
        if let Some(content) = &view.error_content {
            column = column.push(
                text_editor(content)
                    .on_action(QueryMsg::ErrorAction)
                    .font(theme::FONT_MONO)
                    .size(theme::DB_QUERY_ERROR_SIZE)
                    .padding(0)
                    .style(plain_editor_style),
            );
        }
        return container(column)
            .width(Fill)
            .height(Fill)
            .padding(theme::DB_QUERY_ERROR_PADDING)
            .into();
    }
    let Some(result) = &view.result else {
        return centered(
            if query.conn.is_none() {
                labels::DB_QUERY_PICK_DATABASE_HINT
            } else {
                labels::DB_QUERY_RUN_HINT
            }
            .to_string(),
            theme::DB_QUERY_HINT_SIZE,
            palette.text3,
        );
    };
    if let Some(affected) = result.affected_rows {
        return centered(
            labels::db_rows_affected(affected),
            theme::DB_QUERY_AFFECTED_SIZE,
            palette.text2,
        );
    }
    if result.rows.is_empty() {
        return centered(
            labels::DB_QUERY_NO_ROWS.to_string(),
            theme::DB_QUERY_HINT_SIZE,
            palette.text3,
        );
    }
    if view.as_json {
        return json_view(query, result);
    }
    grid(query, result, size)
}

/// Editor só de leitura, sem moldura: texto selecionável e copiável.
fn plain_editor_style(_: &Theme, _: text_editor::Status) -> text_editor::Style {
    let palette = theme::palette();
    text_editor::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        placeholder: palette.text4,
        value: palette.text2,
        selection: theme::TERM_SELECTION_BG,
    }
}

// ----------------------------------------------------------------------------------- JSON

fn json_view<'a>(query: &'a DbQuery, result: &'a QueryResult) -> Element<'a, QueryMsg> {
    let palette = theme::palette();
    let Some((content, source)) = &query.view.json else {
        return Space::new().into();
    };
    let editor = text_editor(content)
        .on_action(QueryMsg::JsonAction)
        .font(theme::FONT_MONO)
        .size(theme::DB_JSON_SIZE)
        .line_height(theme::DB_JSON_LINE_HEIGHT)
        .padding(0)
        .height(Length::Shrink)
        .style(plain_editor_style)
        .highlight_with::<editor_highlight::SyntaxHighlighter>(
            editor_highlight::Settings::new("json".to_string(), source.clone()),
            editor_highlight::format,
        );
    let mut page = column![].width(Fill).height(Fill);
    if result.rows.len() > theme::DB_JSON_PREVIEW_ROWS {
        page = page.push(
            container(
                text(labels::db_json_preview_limited(
                    theme::DB_JSON_PREVIEW_ROWS,
                    result.rows.len(),
                ))
                .font(theme::FONT_BODY)
                .size(theme::DB_JSON_BANNER_SIZE)
                .color(palette.text3),
            )
            .width(Fill)
            .padding([
                theme::DB_JSON_BANNER_PADDING[0],
                theme::DB_JSON_BANNER_PADDING[1],
            ])
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(palette.panel2)),
                ..Default::default()
            }),
        );
    }
    page = page.push(
        scrollable(
            container(editor)
                .width(Fill)
                .padding(theme::DB_JSON_PADDING),
        )
        .width(Fill)
        .height(Fill),
    );
    stack![
        page,
        container(copy_button(query.copied))
            .width(Fill)
            .align_x(Horizontal::Right)
            .padding(Padding {
                top: theme::DB_COPY_TOP,
                right: theme::DB_COPY_RIGHT,
                ..Padding::ZERO
            })
    ]
    .into()
}

fn copy_button<'a>(copied: bool) -> Element<'a, QueryMsg> {
    let palette = theme::palette();
    button(
        row![
            text(if copied {
                labels::ICON_CHECK
            } else {
                labels::ICON_CONTENT_COPY
            })
            .font(theme::FONT_ICON)
            .size(theme::DB_COPY_ICON)
            .color(if copied {
                palette.online
            } else {
                palette.text3
            }),
            Space::new().width(theme::DB_COPY_GAP),
            text(if copied {
                labels::DB_QUERY_COPIED
            } else {
                labels::DB_QUERY_COPY
            })
            .font(theme::FONT_BODY)
            .size(theme::DB_COPY_SIZE)
            .color(palette.text3),
        ]
        .align_y(Alignment::Center),
    )
    .padding([theme::DB_COPY_PADDING[0], theme::DB_COPY_PADDING[1]])
    .on_press(QueryMsg::CopyJson)
    .style(|_: &Theme, status| {
        let palette = theme::palette();
        button::Style {
            background: Some(Background::Color(
                if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    palette.panel3
                } else {
                    palette.panel2
                },
            )),
            text_color: palette.text3,
            border: Border {
                color: palette.border,
                width: 1.0,
                radius: theme::DB_COPY_RADIUS.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

// ----------------------------------------------------------------------------------- grade

/// Larguras visuais: manuais (a última absorve a sobra) ou as base escaladas para preencher.
pub fn effective_widths(base: &[f32], manual: Option<&[f32]>, available: f32) -> Vec<f32> {
    if let Some(manual) = manual {
        let mut widths = manual.to_vec();
        let sum: f32 = widths.iter().sum();
        if sum < available {
            if let Some(last) = widths.last_mut() {
                *last += available - sum;
            }
        }
        return widths;
    }
    let sum: f32 = base.iter().sum();
    if sum < available && sum > 0.0 {
        base.iter().map(|width| width * available / sum).collect()
    } else {
        base.to_vec()
    }
}

/// Janela de linhas desenhadas (a lista é virtualizada, `itemExtent` 26).
pub fn visible_rows(total: usize, scroll: f32, viewport: f32) -> std::ops::Range<usize> {
    let first = ((scroll / theme::DB_GRID_ROW_HEIGHT).floor().max(0.0) as usize).saturating_sub(3);
    let count = (viewport / theme::DB_GRID_ROW_HEIGHT).ceil().max(0.0) as usize + 7;
    first.min(total)..(first + count).min(total)
}

fn grid<'a>(query: &'a DbQuery, result: &'a QueryResult, size: Size) -> Element<'a, QueryMsg> {
    let view = &query.view;
    let available = size.width - theme::DB_GRID_INDEX_WIDTH;
    let widths = effective_widths(&view.base_widths, view.manual_widths.as_deref(), available);
    let sum: f32 = widths.iter().sum();
    let total = theme::DB_GRID_INDEX_WIDTH + sum;

    let window = visible_rows(result.rows.len(), view.grid_scroll, size.height);
    let mut rows = column![Space::new().height(window.start as f32 * theme::DB_GRID_ROW_HEIGHT)];
    for index in window.clone() {
        if let Some(cells) = result.rows.get(index) {
            rows = rows.push(grid_row(
                index,
                cells,
                &widths,
                view.selected_row == Some(index),
            ));
        }
    }
    let below = result.rows.len().saturating_sub(window.end) as f32 * theme::DB_GRID_ROW_HEIGHT;
    rows = rows.push(Space::new().height(below + theme::DB_GRID_LIST_BOTTOM));

    let content = container(
        column![
            grid_header(result, &widths),
            scrollable(rows)
                .id(query.grid_id())
                .on_scroll(|viewport| QueryMsg::GridScroll(viewport.absolute_offset().y))
                .width(Fill)
                .height(Fill),
        ]
        .height(Fill),
    )
    .width(Length::Fixed(total))
    .height(Fill);
    if sum < available {
        return content.into();
    }
    // scrollbar de 12px sempre visível: sem ela o excesso de colunas é inalcançável com mouse
    scrollable(content)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new()
                .width(theme::DB_GRID_SCROLLBAR)
                .scroller_width(theme::DB_GRID_SCROLLBAR),
        ))
        .width(Fill)
        .height(Fill)
        .into()
}

fn grid_header<'a>(result: &'a QueryResult, widths: &[f32]) -> Element<'a, QueryMsg> {
    let palette = theme::palette();
    let mut line =
        row![Space::new().width(theme::DB_GRID_INDEX_WIDTH)].height(theme::DB_GRID_HEADER_HEIGHT);
    for (index, (column, width)) in result.columns.iter().zip(widths).enumerate() {
        let mut names = column![text(column.name.as_str())
            .font(theme::FONT_BODY)
            .size(theme::DB_GRID_NAME_SIZE)
            .color(palette.text2)
            .wrapping(text::Wrapping::None)];
        if !column.ty.is_empty() {
            names = names.push(
                text(column.ty.as_str())
                    .font(theme::FONT_MONO)
                    .size(theme::DB_GRID_TYPE_SIZE)
                    .color(palette.text4)
                    .wrapping(text::Wrapping::None),
            );
        }
        let handle = DragHandle::new(
            container(hairline_v(palette.border2))
                .width(theme::DB_GRID_HANDLE_WIDTH)
                .height(Fill)
                .align_x(Horizontal::Center),
            Axis::Horizontal,
            move |delta| QueryMsg::ResizeColumn(index, delta),
        )
        .on_start(QueryMsg::ResizeStart(widths.to_vec()));
        let cell = row![
            container(
                container(names)
                    .width(Fill)
                    .clip(true)
                    .padding([0.0, theme::DB_GRID_CELL_PADDING])
            )
            .width(Fill)
            .center_y(Fill),
            handle,
        ];
        line = line.push(
            container(column![
                container(cell).height(theme::DB_GRID_HEADER_HEIGHT - 1.0),
                hairline_h(palette.border2),
            ])
            .width(*width)
            .style(move |_: &Theme| container::Style {
                background: Some(Background::Color(palette.panel2)),
                ..Default::default()
            }),
        );
    }
    line.into()
}

fn grid_row<'a>(
    index: usize,
    cells: &'a [Cell],
    widths: &[f32],
    selected: bool,
) -> Element<'a, QueryMsg> {
    let palette = theme::palette();
    let number = container(
        container(
            text((index + 1).to_string())
                .font(theme::FONT_MONO)
                .size(theme::DB_GRID_INDEX_SIZE)
                .color(palette.text4),
        )
        .width(Fill)
        .height(Fill)
        .center_x(Fill)
        .center_y(Fill),
    )
    .width(theme::DB_GRID_INDEX_WIDTH - 1.0)
    .height(Fill)
    .style(move |_: &Theme| container::Style {
        background: Some(Background::Color(palette.panel2)),
        ..Default::default()
    });
    let mut line = row![number, hairline_v(palette.border)].height(theme::DB_GRID_ROW_HEIGHT - 1.0);
    for (cell, width) in cells.iter().zip(widths) {
        line = line.push(
            container(cell_view(cell))
                .width(*width)
                .height(Fill)
                .center_y(Fill)
                .clip(true)
                .padding([0.0, theme::DB_GRID_CELL_PADDING]),
        );
    }
    button(column![line, hairline_h(palette.border)])
        .padding(0)
        .width(Length::Shrink)
        .height(theme::DB_GRID_ROW_HEIGHT)
        .on_press(QueryMsg::SelectRow(index))
        .style(move |_: &Theme, status| {
            let palette = theme::palette();
            button::Style {
                background: if selected {
                    Some(Background::Color(palette.accent_soft))
                } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    Some(Background::Color(palette.panel3))
                } else {
                    None
                },
                text_color: palette.text2,
                border: Border::default(),
                ..Default::default()
            }
        })
        .into()
}

fn cell_view<'a>(cell: &'a Cell) -> Element<'a, QueryMsg> {
    let palette = theme::palette();
    let base = |content: String, font: iced::Font, color: Color| {
        text(content)
            .font(font)
            .size(theme::DB_GRID_CELL_SIZE)
            .color(color)
            .wrapping(text::Wrapping::None)
    };
    match cell {
        Cell::Null => base(
            labels::DB_NULL_CELL.to_string(),
            theme::FONT_MONO_ITALIC,
            palette.text4,
        )
        .into(),
        Cell::Blob(bytes) => base(
            format!("⟨blob · {} B⟩", bytes.len()),
            theme::FONT_MONO,
            palette.text4,
        )
        .into(),
        cell if cell.is_number() => base(cell.display(), theme::FONT_MONO, palette.text)
            .width(Fill)
            .align_x(Horizontal::Right)
            .into(),
        cell => base(cell.display(), theme::FONT_MONO, palette.text2).into(),
    }
}

// ------------------------------------------------------------------------------- rodapé

fn footer(query: &DbQuery) -> Element<'_, QueryMsg> {
    let palette = theme::palette();
    let result = query.view.result.as_ref();
    let mut info = String::new();
    if let Some(result) = result.filter(|r| r.affected_rows.is_none()) {
        info.push_str(&labels::db_rows_footer(result.rows.len()));
        if result.truncated {
            info.push_str(labels::DB_QUERY_TRUNCATED_SUFFIX);
        }
        info.push_str(&format!(" · {} ms", result.elapsed.as_millis()));
    }
    let mut line = row![
        text(info)
            .font(theme::FONT_BODY)
            .size(theme::DB_QUERY_FOOTER_SIZE)
            .color(palette.text3),
        Space::new().width(Fill),
    ]
    .align_y(Alignment::Center);
    if result.is_some_and(|r| r.affected_rows.is_none()) {
        line = line
            .push(toggle(
                labels::DB_QUERY_TABLE,
                !query.view.as_json,
                QueryMsg::ShowJson(false),
            ))
            .push(toggle(
                labels::DB_QUERY_JSON,
                query.view.as_json,
                QueryMsg::ShowJson(true),
            ))
            .push(Space::new().width(theme::DB_QUERY_TOGGLE_GAP));
    }
    let dirty = query.dirty();
    line = line.push(
        text(if dirty {
            labels::DB_QUERY_UNSAVED
        } else {
            labels::DB_QUERY_SAVED
        })
        .font(theme::FONT_BODY)
        .size(theme::DB_QUERY_FOOTER_SIZE)
        .color(if dirty { palette.warn } else { palette.text4 }),
    );
    column![
        hairline_h(palette.border),
        container(line)
            .width(Fill)
            .height(theme::DB_QUERY_FOOTER_HEIGHT - 1.0)
            .align_y(Vertical::Center)
            .padding([0.0, theme::DB_QUERY_FOOTER_PADDING]),
    ]
    .height(theme::DB_QUERY_FOOTER_HEIGHT)
    .into()
}

/// Botão compacto do alternador Tabela/JSON.
fn toggle<'a>(label: &'static str, active: bool, message: QueryMsg) -> Element<'a, QueryMsg> {
    button(
        text(label)
            .font(theme::FONT_BODY)
            .size(theme::DB_QUERY_FOOTER_SIZE)
            .color(if active {
                theme::palette().text
            } else {
                theme::palette().text4
            }),
    )
    .padding([
        theme::DB_QUERY_TOGGLE_PADDING[0],
        theme::DB_QUERY_TOGGLE_PADDING[1],
    ])
    .on_press(message)
    .style(move |_: &Theme, status| {
        let palette = theme::palette();
        button::Style {
            background: (active
                || matches!(status, button::Status::Hovered | button::Status::Pressed))
            .then_some(Background::Color(palette.panel3)),
            text_color: palette.text,
            border: Border {
                radius: theme::DB_QUERY_TOGGLE_RADIUS.into(),
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
    use crate::db_query::DbQuery;
    use std::time::Duration;

    #[test]
    fn responsive_widths_fill_the_available_space_proportionally() {
        let widths = effective_widths(&[100.0, 300.0], None, 800.0);
        assert_eq!(widths, [200.0, 600.0]);
        // sem sobra, as bases ficam como estão (e haverá rolagem horizontal)
        assert_eq!(
            effective_widths(&[500.0, 500.0], None, 800.0),
            [500.0, 500.0]
        );
    }

    #[test]
    fn manual_widths_are_verbatim_and_the_last_column_takes_the_slack() {
        assert_eq!(
            effective_widths(&[1.0], Some(&[100.0, 200.0]), 500.0),
            [100.0, 400.0]
        );
        assert_eq!(
            effective_widths(&[1.0], Some(&[400.0, 400.0]), 500.0),
            [400.0, 400.0]
        );
    }

    #[test]
    fn only_the_rows_near_the_viewport_are_built() {
        assert_eq!(visible_rows(1000, 0.0, 260.0), 0..17);
        assert_eq!(visible_rows(1000, 2600.0, 260.0), 97..114);
        assert_eq!(visible_rows(5, 0.0, 10_000.0), 0..5);
        assert_eq!(visible_rows(0, 0.0, 100.0), 0..0);
    }

    fn result(rows: usize) -> QueryResult {
        QueryResult::from_rows(
            vec!["id".into(), "name".into()],
            (0..rows)
                .map(|n| vec![Cell::Int(n as i64), Cell::Text(format!("n{n}"))])
                .collect(),
            false,
            Duration::from_millis(3),
        )
    }

    #[test]
    fn view_builds_for_every_result_state() {
        let mut query = DbQuery::scratch(1, 1, "-- db: x\nselect 1;\n", std::path::PathBuf::new());
        let connections = Vec::new();
        let _ = view(&query, &connections);
        let _ = overlay(&query);
        // sem resultado, com erro, vazio, DML, tabela e JSON
        query.view.error = Some(gsh_db::DbError::Timeout(30));
        query.view.error_content = Some(text_editor::Content::with_text("Query exceeded 30s"));
        let _ = view(&query, &connections);
        query.view.error = None;
        let mut empty = result(0);
        empty.rows.clear();
        for shown in [empty, QueryResult::affected(3, Duration::ZERO), result(600)] {
            query.view.base_widths = crate::db_query::base_widths(&shown);
            query.view.result = Some(shown);
            let _ = view(&query, &connections);
            query.view.as_json = true;
            let _ = query.update(
                QueryMsg::ShowJson(true),
                &crate::db_query::Ctx {
                    secrets: &gsh_db::SecretStore::new("/nonexistent/s".into()),
                    ssh_passphrases: &std::collections::BTreeMap::new(),
                },
            );
            let _ = view(&query, &connections);
            query.view.as_json = false;
        }
        query.picker_open = true;
        query.save_dialog = Some(crate::db_query::SaveDialog {
            name: "query.dbq".into(),
        });
        let _ = view(&query, &connections);
        let _ = overlay(&query);
        query.save_error = Some("falhou".into());
        let _ = overlay(&query);
    }
}
