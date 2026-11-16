//! Corpo da aba **Banco de dados** do painel direito (`db_panel.dart`): conexões do workspace
//! (registradas, locais e SQLite detectados), árvore de schema sob demanda e rodapé com a contagem.
//!
//! O "+" abre o menu de motor e depois o diálogo; clicar numa conexão com "…" abre o menu dela
//! (Editar, Copiar nome, Nova query, Excluir). Texto cortado não ganha reticências: o iced não as
//! desenha.

use gsh_db::schema::{SchemaColumn, SchemaTable};
use gsh_db::{Connection, Engine, Origin};
use iced::widget::{button, column, container, row, scrollable, text, tooltip, Space};
use iced::{Alignment, Background, Border, Element, Fill, Length, Padding, Theme};

use crate::anchored::Anchored;
use crate::app_menu::{self, Entry, Item, Leading};
use crate::db_state::{table_key, ColumnsKey, DbMsg, DbState, Load, MenuState};
use crate::{db_engine_icon, labels, pane_chrome, panel_widgets, theme};

/// Largura mínima dos menus do painel (`showAppMenu` sem `minWidth`).
const MENU_MIN_WIDTH: f32 = 200.0;

pub fn view(db: &DbState) -> Element<'_, DbMsg> {
    if db.root().is_none() {
        return panel_widgets::empty_label(labels::FILES_NO_FOLDER);
    }
    let palette = theme::palette();

    let refresh = header_action(labels::ICON_REFRESH, DbMsg::Reload);
    let add = Anchored::new(
        header_action(labels::ICON_PLUS, DbMsg::OpenAddMenu),
        (db.menu == Some(MenuState::Add)).then(add_menu),
        DbMsg::CloseMenu,
    );
    let header = panel_widgets::section_header(
        labels::DB_SECTION,
        vec![
            refresh,
            Space::new().width(theme::DB_HEADER_ACTION_GAP).into(),
            add.into(),
        ],
    );

    let body: Element<'_, DbMsg> = if db.connections.is_empty() {
        container(
            text(labels::DB_NO_CONNECTIONS)
                .font(theme::FONT_BODY)
                .size(theme::DB_EMPTY_SIZE)
                .line_height(1.5)
                .color(palette.text3),
        )
        .padding(theme::DB_EMPTY_PADDING)
        .width(Fill)
        .height(Fill)
        .into()
    } else {
        scrollable(
            db.connections
                .iter()
                .fold(column![], |list, conn| list.push(tile(db, conn)))
                .padding([0.0, theme::DB_LIST_PADDING_X]),
        )
        .width(Fill)
        .height(Fill)
        .into()
    };

    let count = db.connections.len();
    let footer = container(
        text(if count == 1 {
            labels::DB_FOOTER_ONE.to_string()
        } else {
            labels::db_footer(count)
        })
        .font(theme::FONT_BODY)
        .size(theme::DB_FOOTER_SIZE)
        .color(palette.text4),
    )
    .width(Fill)
    .padding([theme::DB_FOOTER_PADDING[0], theme::DB_FOOTER_PADDING[1]]);

    column![header, body, panel_widgets::hairline_h(), footer,]
        .height(Fill)
        .into()
}

/// Ícone 14px do cabeçalho com hover (`HoverTap` com padding 3).
fn header_action<'a>(glyph: &'static str, message: DbMsg) -> Element<'a, DbMsg> {
    hover_button(
        icon(
            glyph,
            theme::FILE_PANEL_HEADER_ACTION_SIZE,
            theme::palette().text3,
        ),
        Padding::new(theme::FILE_PANEL_HEADER_ACTION_PADDING),
        Some(message),
    )
}

fn icon<'a, M: 'a>(glyph: &'static str, size: f32, color: iced::Color) -> Element<'a, M> {
    text(glyph)
        .font(theme::FONT_ICON)
        .size(size)
        .color(color)
        .into()
}

/// Área clicável com realce de hover no `panel3` (o `HoverTap` do Flutter, raio 7).
fn hover_button<'a>(
    content: Element<'a, DbMsg>,
    padding: Padding,
    on_press: Option<DbMsg>,
) -> Element<'a, DbMsg> {
    button(content)
        .padding(padding)
        .on_press_maybe(on_press)
        .style(|_: &Theme, status| button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| Background::Color(theme::palette().panel3)),
            text_color: theme::palette().text,
            border: Border {
                radius: theme::DB_HOVER_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn add_menu<'a>() -> Element<'a, DbMsg> {
    app_menu::menu(
        Engine::ALL
            .into_iter()
            .map(|engine| {
                Item::Entry(
                    Entry::new(engine.label(), DbMsg::AddEngine(engine))
                        .leading(Leading::Engine(engine)),
                )
            })
            .collect(),
        MENU_MIN_WIDTH,
    )
}

fn conn_menu<'a>(conn: &Connection) -> Element<'a, DbMsg> {
    let name = conn.name.clone();
    let mut items = vec![
        Item::Entry(
            Entry::new(labels::DB_EDIT, DbMsg::Edit(name.clone()))
                .leading(Leading::Glyph(labels::ICON_EDIT)),
        ),
        // o nome é o identificador que a CLI usa (`--db <nome>`)
        Item::Entry(
            Entry::new(labels::DB_COPY_NAME, DbMsg::CopyName(name.clone()))
                .leading(Leading::Glyph(labels::ICON_CONTENT_COPY)),
        ),
    ];
    if conn.engine.is_sql() {
        items.push(Item::Entry(
            Entry::new(labels::DB_NEW_QUERY, DbMsg::NewQuery(name.clone(), None))
                .leading(Leading::Glyph(labels::ICON_PLUS)),
        ));
    } else if conn.engine == Engine::Redis {
        items.push(Item::Entry(
            Entry::new(labels::DB_BROWSE_KEYS, DbMsg::BrowseRedis(name.clone()))
                .leading(Leading::Glyph(labels::ICON_GRID_ON)),
        ));
    }
    items.push(Item::Divider);
    items.push(Item::Entry(
        Entry::new(labels::DB_DIALOG_DELETE, DbMsg::AskDelete(name))
            .leading(Leading::Glyph(labels::ICON_DELETE))
            .danger(),
    ));
    app_menu::menu(items, MENU_MIN_WIDTH)
}

/// Selo da linha (`_Chip`): borda `border2`, raio 4, texto 9.5 `text3`.
fn chip<'a>(label: &'static str) -> Element<'a, DbMsg> {
    container(
        text(label)
            .font(theme::FONT_BODY)
            .size(theme::DB_CHIP_SIZE)
            .color(theme::palette().text3),
    )
    .padding(theme::DB_CHIP_PADDING)
    .style(|_: &Theme| container::Style {
        border: Border {
            color: theme::palette().border2,
            width: 1.0,
            radius: theme::DB_CHIP_RADIUS.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Linha da conexão e, expandida, a árvore de schema/coleções embaixo.
fn tile<'a>(db: &'a DbState, conn: &'a Connection) -> Element<'a, DbMsg> {
    let palette = theme::palette();
    let is_mongo = conn.engine == Engine::Mongo;
    let is_redis = conn.engine == Engine::Redis;
    let expandable = conn.engine.is_sql() || is_mongo;
    let interactive = expandable || is_redis;
    let expanded = db.expanded.contains(&conn.name);

    let leading: Element<'a, DbMsg> = if expandable {
        row![
            icon(
                if expanded {
                    labels::ICON_EXPAND_MORE
                } else {
                    labels::ICON_CHEVRON_RIGHT
                },
                theme::DB_TILE_CHEVRON_SIZE,
                palette.text4
            ),
            Space::new().width(theme::DB_TILE_CHEVRON_GAP),
        ]
        .into()
    } else {
        row![
            icon(labels::ICON_DOT, theme::DB_TILE_DOT_SIZE, palette.text4),
            Space::new().width(theme::DB_TILE_DOT_GAP),
        ]
        .into()
    };

    let mut title = row![text(conn.name.as_str())
        .font(theme::FONT_BODY)
        .size(theme::DB_TILE_NAME_SIZE)
        .line_height(theme::LINE_HEIGHT_BODY)
        .color(palette.text2)
        .wrapping(text::Wrapping::None)]
    .align_y(Alignment::Center);
    let chips = [
        (conn.origin == Origin::Detected, labels::DB_CHIP_DETECTED),
        (conn.origin == Origin::Local, labels::DB_CHIP_LOCAL),
        // só os desvios do padrão (leitura + visível) ganham selo, senão vira ruído
        (
            conn.access == gsh_db::Access::ReadWrite,
            labels::DB_CHIP_READWRITE,
        ),
        (!conn.agents, labels::DB_CHIP_NO_AGENTS),
    ];
    for (show, label) in chips {
        if show {
            title = title
                .push(Space::new().width(theme::DB_CHIP_GAP))
                .push(chip(label));
        }
    }
    let target = text(format!(
        "{} · {}",
        conn.engine.label(),
        conn.display_target()
    ))
    .font(theme::FONT_MONO)
    .size(theme::DB_TILE_TARGET_SIZE)
    .line_height(theme::LINE_HEIGHT_MONO)
    .color(palette.text4)
    .wrapping(text::Wrapping::None);

    let more = Anchored::new(
        hover_button(
            icon(
                labels::ICON_MORE_HORIZ,
                theme::DB_TILE_MORE_SIZE,
                palette.text4,
            ),
            Padding::new(theme::DB_TILE_MORE_PADDING),
            Some(DbMsg::OpenConnMenu(conn.name.clone())),
        ),
        (db.menu == Some(MenuState::Conn(conn.name.clone()))).then(|| conn_menu(conn)),
        DbMsg::CloseMenu,
    );

    let line = row![
        leading,
        db_engine_icon::view(conn.engine, theme::DB_TILE_ENGINE_ICON),
        Space::new().width(theme::DB_TILE_ENGINE_GAP),
        container(column![title, target]).width(Fill).clip(true),
        more,
    ]
    .align_y(Alignment::Center);

    let on_press = if expandable {
        Some(DbMsg::Toggle(conn.name.clone()))
    } else if is_redis {
        Some(DbMsg::BrowseRedis(conn.name.clone()))
    } else {
        None
    };

    let tile_button = button(line)
        .width(Fill)
        .padding(theme::DB_TILE_PADDING)
        .on_press_maybe(on_press)
        .style(move |_: &Theme, status| button::Style {
            background: (interactive
                && matches!(status, button::Status::Hovered | button::Status::Pressed))
            .then(|| Background::Color(theme::palette().panel3)),
            text_color: theme::palette().text,
            border: Border {
                radius: theme::DB_HOVER_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    let mut tile = column![container(tile_button).padding([theme::DB_TILE_GAP, 0.0])];
    if expanded && expandable {
        tile = tile.push(if is_mongo {
            mongo_tree(db, conn)
        } else {
            schema_tree(db, conn)
        });
    }
    tile.into()
}

/// Árvore de databases -> collections MongoDB (`_MongoCollections` em `db_panel.dart`).
fn mongo_tree<'a>(db: &'a DbState, conn: &'a Connection) -> Element<'a, DbMsg> {
    let body: Element<'a, DbMsg> = if !DbState::mongo_needs_database(conn) {
        mongo_collection_list(db, &conn.name, "")
    } else {
        match db.mongo_databases.get(&conn.name) {
            None | Some(Load::Loading) => hint(labels::DB_LOADING.to_string(), false),
            Some(Load::Failed(error)) => hint(labels::db_error_message(error), true),
            Some(Load::Ready(names)) if names.is_empty() => {
                hint(labels::DB_NO_DATABASES.to_string(), false)
            }
            Some(Load::Ready(names)) => names
                .iter()
                .fold(column![], |col, db_name| {
                    col.push(mongo_database_node(db, conn, db_name))
                })
                .into(),
        }
    };
    container(body)
        .width(Fill)
        .padding(Padding {
            left: theme::DB_TREE_LEFT,
            bottom: theme::DB_TREE_BOTTOM,
            ..Padding::ZERO
        })
        .into()
}

fn mongo_database_node<'a>(
    db: &'a DbState,
    conn: &'a Connection,
    database: &'a str,
) -> Element<'a, DbMsg> {
    let palette = theme::palette();
    let open = db
        .mongo_open_dbs
        .get(&conn.name)
        .is_some_and(|set| set.contains(database));
    let header = hover_button(
        row![
            icon(
                if open {
                    labels::ICON_EXPAND_MORE
                } else {
                    labels::ICON_CHEVRON_RIGHT
                },
                theme::DB_TREE_CHEVRON_SIZE,
                palette.text4,
            ),
            Space::new().width(2),
            icon(
                labels::ICON_STORAGE,
                theme::DB_TREE_ICON_SIZE,
                palette.text4
            ),
            Space::new().width(theme::DB_TREE_ICON_GAP),
            container(
                text(database)
                    .font(theme::FONT_MONO)
                    .size(theme::DB_TREE_TABLE_SIZE)
                    .color(palette.text2)
                    .wrapping(text::Wrapping::None),
            )
            .width(Fill)
            .clip(true),
        ]
        .align_y(Alignment::Center)
        .into(),
        Padding::from(theme::DB_TREE_ROW_PADDING),
        Some(DbMsg::ToggleMongoDatabase(
            conn.name.clone(),
            database.to_string(),
        )),
    );
    let mut node = column![header];
    if open {
        node = node.push(
            container(mongo_collection_list(db, &conn.name, database)).padding(Padding {
                left: 15.0,
                ..Padding::ZERO
            }),
        );
    }
    node.into()
}

fn mongo_collection_list<'a>(
    db: &'a DbState,
    conn_name: &'a str,
    database: &'a str,
) -> Element<'a, DbMsg> {
    let palette = theme::palette();
    let key = (conn_name.to_string(), database.to_string());
    match db.mongo_collections.get(&key) {
        None | Some(Load::Loading) => hint(labels::DB_LOADING.to_string(), false),
        Some(Load::Failed(error)) => hint(labels::db_error_message(error), true),
        Some(Load::Ready(names)) if names.is_empty() => {
            hint(labels::DB_NO_COLLECTIONS.to_string(), false)
        }
        Some(Load::Ready(names)) => names
            .iter()
            .fold(column![], |col, name| {
                col.push(hover_button(
                    row![
                        icon(
                            labels::ICON_DATA_OBJECT,
                            theme::DB_TREE_ICON_SIZE,
                            palette.text4,
                        ),
                        Space::new().width(theme::DB_TREE_ICON_GAP),
                        container(
                            text(name.as_str())
                                .font(theme::FONT_MONO)
                                .size(theme::DB_TREE_TABLE_SIZE)
                                .color(palette.text2)
                                .wrapping(text::Wrapping::None),
                        )
                        .width(Fill)
                        .clip(true),
                    ]
                    .align_y(Alignment::Center)
                    .into(),
                    Padding::from(theme::DB_TREE_ROW_PADDING),
                    Some(DbMsg::OpenMongoCollection(
                        conn_name.to_string(),
                        database.to_string(),
                        name.clone(),
                    )),
                ))
            })
            .into(),
    }
}

/// Mensagem pequena dentro da árvore (carregando, vazio ou erro).
fn hint<'a>(message: String, error: bool) -> Element<'a, DbMsg> {
    let palette = theme::palette();
    container(
        text(message)
            .font(theme::FONT_BODY)
            .size(theme::DB_TREE_HINT_SIZE)
            .color(if error { palette.error } else { palette.text4 }),
    )
    .padding(theme::DB_TREE_HINT_PADDING)
    .into()
}

/// Árvore de tabelas → colunas de uma conexão.
fn schema_tree<'a>(db: &'a DbState, conn: &'a Connection) -> Element<'a, DbMsg> {
    let body: Element<'a, DbMsg> = match db.tables.get(&conn.name) {
        None | Some(Load::Loading) => hint(labels::DB_LOADING.to_string(), false),
        Some(Load::Failed(error)) => hint(labels::db_error_message(error), true),
        Some(Load::Ready(tables)) if tables.is_empty() => {
            hint(labels::DB_NO_TABLES.to_string(), false)
        }
        Some(Load::Ready(tables)) => tables_view(db, conn, tables),
    };
    container(body)
        .width(Fill)
        .padding(Padding {
            left: theme::DB_TREE_LEFT,
            bottom: theme::DB_TREE_BOTTOM,
            ..Padding::ZERO
        })
        .into()
}

fn tables_view<'a>(
    db: &'a DbState,
    conn: &'a Connection,
    tables: &'a [SchemaTable],
) -> Element<'a, DbMsg> {
    // agrupa por schema só quando há mais de um distinto; o caso comum (public/dbo, ou SQLite)
    // segue lista plana
    let mut schemas: Vec<&str> = tables
        .iter()
        .filter_map(|table| table.schema.as_deref())
        .collect();
    schemas.sort_unstable();
    schemas.dedup();
    if schemas.len() <= 1 {
        return tables
            .iter()
            .fold(column![], |list, table| {
                list.push(table_rows(db, conn, table, false))
            })
            .into();
    }
    let mut groups: Vec<(String, Vec<&SchemaTable>)> = Vec::new();
    for table in tables {
        let key = table.schema.clone().unwrap_or_default();
        match groups.iter_mut().find(|(name, _)| *name == key) {
            Some((_, list)) => list.push(table),
            None => groups.push((key, vec![table])),
        }
    }
    let collapsed = db.collapsed_schemas.get(&conn.name);
    let mut out = column![];
    for (schema, list) in groups {
        let is_collapsed = collapsed.is_some_and(|set| set.contains(&schema));
        out = out.push(schema_header(conn, &schema, list.len(), is_collapsed));
        if !is_collapsed {
            for table in list {
                out = out.push(table_rows(db, conn, table, true));
            }
        }
    }
    out.into()
}

fn tree_row_padding() -> Padding {
    Padding::from([theme::DB_TREE_ROW_PADDING[0], theme::DB_TREE_ROW_PADDING[1]])
}

fn schema_header<'a>(
    conn: &Connection,
    schema: &str,
    count: usize,
    collapsed: bool,
) -> Element<'a, DbMsg> {
    let palette = theme::palette();
    let label = if schema.is_empty() {
        labels::DB_DEFAULT_SCHEMA.to_string()
    } else {
        schema.to_string()
    };
    let content = row![
        icon(
            if collapsed {
                labels::ICON_CHEVRON_RIGHT
            } else {
                labels::ICON_EXPAND_MORE
            },
            theme::DB_TREE_CHEVRON_SIZE,
            palette.text4
        ),
        Space::new().width(theme::DB_TREE_CHEVRON_GAP),
        icon(labels::ICON_FOLDER, theme::DB_TREE_ICON_SIZE, palette.text4),
        Space::new().width(theme::DB_TREE_ICON_GAP),
        text(label)
            .font(theme::FONT_MONO_SEMIBOLD)
            .size(theme::DB_TREE_SCHEMA_SIZE)
            .line_height(theme::LINE_HEIGHT_MONO)
            .color(palette.text3)
            .wrapping(text::Wrapping::None),
        Space::new().width(theme::DB_TREE_ICON_GAP),
        text(count.to_string())
            .font(theme::FONT_BODY)
            .size(theme::DB_TREE_COUNT_SIZE)
            .color(palette.text4),
    ]
    .align_y(Alignment::Center);
    button(container(content).width(Fill).clip(true))
        .width(Fill)
        .padding(tree_row_padding())
        .on_press(DbMsg::ToggleSchema(conn.name.clone(), schema.to_string()))
        .style(row_style)
        .into()
}

fn row_style(_: &Theme, status: button::Status) -> button::Style {
    button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed)
            .then(|| Background::Color(theme::palette().panel3)),
        text_color: theme::palette().text,
        border: Border {
            radius: theme::DB_HOVER_RADIUS.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Linha da tabela e, aberta, a lista de colunas.
fn table_rows<'a>(
    db: &'a DbState,
    conn: &'a Connection,
    table: &'a SchemaTable,
    indent: bool,
) -> Element<'a, DbMsg> {
    let palette = theme::palette();
    let open = db
        .open_tables
        .get(&conn.name)
        .is_some_and(|keys| keys.contains(&table_key(table)));
    let left = if indent { theme::DB_TREE_INDENT } else { 0.0 };
    let new_query = tooltip(
        hover_button(
            icon(labels::ICON_PLUS, theme::DB_TREE_ADD_SIZE, palette.text4),
            Padding::new(theme::DB_TREE_ADD_PADDING),
            Some(DbMsg::NewQuery(conn.name.clone(), Some(table.clone()))),
        ),
        pane_chrome::tip_bubble(labels::DB_NEW_QUERY),
        tooltip::Position::Bottom,
    );
    let content = row![
        icon(
            if open {
                labels::ICON_EXPAND_MORE
            } else {
                labels::ICON_CHEVRON_RIGHT
            },
            theme::DB_TREE_CHEVRON_SIZE,
            palette.text4
        ),
        Space::new().width(theme::DB_TREE_CHEVRON_GAP),
        icon(
            if table.is_view {
                labels::ICON_VISIBILITY
            } else {
                labels::ICON_TABLE_CHART
            },
            theme::DB_TREE_ICON_SIZE,
            palette.text4
        ),
        Space::new().width(theme::DB_TREE_ICON_GAP),
        text(table.name.as_str())
            .font(theme::FONT_MONO)
            .size(theme::DB_TREE_TABLE_SIZE)
            .line_height(theme::LINE_HEIGHT_MONO)
            .color(palette.text2)
            .wrapping(text::Wrapping::None),
        Space::new().width(theme::DB_TREE_ADD_GAP),
        new_query,
    ]
    .align_y(Alignment::Center);
    let line = button(container(content).width(Fill).clip(true))
        .width(Fill)
        .padding(tree_row_padding())
        .on_press(DbMsg::ToggleTable(conn.name.clone(), table.clone()))
        .style(row_style);
    let mut out = column![container(line).padding(Padding {
        left,
        ..Padding::ZERO
    })];
    if open {
        out = out.push(container(columns_view(db, conn, table)).padding(Padding {
            left,
            ..Padding::ZERO
        }));
    }
    out.into()
}

fn columns_view<'a>(
    db: &'a DbState,
    conn: &'a Connection,
    table: &'a SchemaTable,
) -> Element<'a, DbMsg> {
    let palette = theme::palette();
    let key: ColumnsKey = (conn.name.clone(), table.schema.clone(), table.name.clone());
    let list: Element<'a, DbMsg> = match db.columns.get(&key) {
        None | Some(Load::Loading) => container(
            text(labels::DB_LOADING)
                .font(theme::FONT_BODY)
                .size(theme::DB_COLUMN_LOADING_SIZE)
                .color(palette.text4),
        )
        .padding([2.0, theme::DB_COLUMN_ROW_PADDING[1]])
        .into(),
        // erro ao ler as colunas não aparece: a lista só fica vazia, como no Flutter
        Some(Load::Failed(_)) => column![].into(),
        Some(Load::Ready(columns)) => columns
            .iter()
            .fold(column![], |list, column| list.push(column_row(column)))
            .into(),
    };
    container(list)
        .padding(Padding {
            left: theme::DB_COLUMNS_LEFT,
            ..Padding::ZERO
        })
        .into()
}

fn column_row<'a>(column: &'a SchemaColumn) -> Element<'a, DbMsg> {
    let palette = theme::palette();
    let mut line = row![
        text(column.name.as_str())
            .font(theme::FONT_MONO)
            .size(theme::DB_COLUMN_NAME_SIZE)
            .line_height(theme::LINE_HEIGHT_MONO)
            .color(palette.text2)
            .wrapping(text::Wrapping::None),
        Space::new().width(theme::DB_COLUMN_TYPE_GAP),
        text(column.ty.as_str())
            .font(theme::FONT_MONO)
            .size(theme::DB_COLUMN_TYPE_SIZE)
            .line_height(theme::LINE_HEIGHT_MONO)
            .color(palette.text4)
            .wrapping(text::Wrapping::None),
    ]
    .align_y(Alignment::Center);
    if column.primary_key {
        line = line.push(Space::new().width(theme::DB_COLUMN_PK_GAP)).push(
            text(labels::DB_PRIMARY_KEY)
                .font(theme::FONT_BODY)
                .size(theme::DB_COLUMN_PK_SIZE)
                .color(palette.accent),
        );
    }
    container(container(line).width(Length::Fill).clip(true))
        .padding([
            theme::DB_COLUMN_ROW_PADDING[0],
            theme::DB_COLUMN_ROW_PADDING[1],
        ])
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gsh_db::{Access, SecretStore};
    use std::path::PathBuf;

    fn state(connections: Vec<Connection>) -> DbState {
        let mut db = DbState::new(SecretStore::new(PathBuf::from("/nonexistent/secrets.json")));
        let _ = db.set_workspace(Some(PathBuf::from("/nonexistent")));
        db.connections = connections;
        db
    }

    fn sqlite(name: &str) -> Connection {
        Connection::sqlite(name, "a.db", Origin::Detected, Access::ReadWrite, false)
    }

    #[test]
    fn builds_empty_populated_and_without_workspace() {
        let _ = view(&DbState::new(SecretStore::new(PathBuf::from("/x"))));
        let _ = view(&state(Vec::new()));
        let mut db = state(vec![sqlite("a"), sqlite("b")]);
        db.expanded.insert("a".into());
        let _ = view(&db);
    }

    #[test]
    fn builds_the_tree_in_every_load_state_and_with_grouped_schemas() {
        let mut db = state(vec![sqlite("a")]);
        db.expanded.insert("a".into());
        let table = |schema: Option<&str>, name: &str, view: bool| SchemaTable {
            name: name.into(),
            schema: schema.map(str::to_string),
            is_view: view,
        };
        db.tables.insert("a".into(), Load::Loading);
        let _ = view(&db);
        db.tables.insert(
            "a".into(),
            Load::Failed(gsh_db::DbError::UnsupportedEngine(Engine::Mysql)),
        );
        let _ = view(&db);
        db.tables.insert("a".into(), Load::Ready(Vec::new()));
        let _ = view(&db);
        let tables = vec![
            table(Some("public"), "users", false),
            table(Some("sales"), "v_orders", true),
        ];
        db.tables.insert("a".into(), Load::Ready(tables.clone()));
        db.open_tables
            .entry("a".into())
            .or_default()
            .insert(table_key(&tables[0]));
        db.columns.insert(
            ("a".into(), Some("public".into()), "users".into()),
            Load::Ready(vec![SchemaColumn {
                name: "id".into(),
                ty: "integer".into(),
                primary_key: true,
            }]),
        );
        db.collapsed_schemas
            .entry("a".into())
            .or_default()
            .insert("sales".into());
        let _ = view(&db);
        db.menu = Some(MenuState::Conn("a".into()));
        let _ = view(&db);
        db.menu = Some(MenuState::Add);
        let _ = view(&db);
    }

    #[test]
    fn renders_redis_and_mongo_trees_in_all_states() {
        let redis = Connection::from_json(
            &serde_json::json!({"name": "cache", "url": "redis://127.0.0.1:6379/0"}),
            Origin::Registered,
        )
        .unwrap();
        let mongo = Connection::from_json(
            &serde_json::json!({"name": "atlas", "url": "mongodb://127.0.0.1:27017"}),
            Origin::Registered,
        )
        .unwrap();
        let mut db = state(vec![redis, mongo]);
        db.menu = Some(MenuState::Conn("cache".into()));
        let _ = view(&db);

        db.menu = None;
        db.expanded.insert("atlas".into());
        db.mongo_databases
            .insert("atlas".into(), Load::Ready(vec!["prod".into()]));
        db.mongo_open_dbs
            .entry("atlas".into())
            .or_default()
            .insert("prod".into());
        db.mongo_collections.insert(
            ("atlas".into(), "prod".into()),
            Load::Ready(vec!["users".into()]),
        );
        let _ = view(&db);
    }
}
