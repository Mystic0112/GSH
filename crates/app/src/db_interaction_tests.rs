//! Interação real (headless, renderer em CPU) com os widgets próprios do banco: o popup ancorado
//! recebe cliques na camada de overlay e a alça de arrasto converte o movimento do cursor.

use std::path::PathBuf;
use std::time::Duration;

use gsh_db::{Access, Cell, Connection, Engine, Origin, QueryResult, SecretStore};
use iced::{mouse, Element, Event, Point, Size};
use iced_test::simulator::{click, Simulator};

use crate::db_query::{Ctx, DbQuery, QueryMsg};
use crate::db_state::{DbMsg, DbState, MenuState};
use crate::theme;

fn simulator<'a, M>(size: Size, element: impl Into<Element<'a, M>>) -> Simulator<'a, M> {
    std::env::set_var("ICED_TEST_BACKEND", "tiny-skia");
    let settings = iced::Settings {
        fonts: theme::EMBEDDED_FONTS
            .iter()
            .map(|font| (*font).into())
            .collect(),
        default_font: theme::FONT_BODY,
        ..Default::default()
    };
    Simulator::with_size(settings, size, element)
}

fn panel() -> DbState {
    let mut db = DbState::new(SecretStore::new(PathBuf::from("/nonexistent/secrets.json")));
    let _ = db.set_workspace(Some(PathBuf::from("/nonexistent")));
    db.connections = vec![Connection::sqlite(
        "app.db",
        "data/app.db",
        Origin::Registered,
        Access::Read,
        true,
    )];
    db
}

#[test]
fn the_add_menu_item_is_clickable_in_the_overlay_layer() {
    let mut db = panel();
    db.menu = Some(MenuState::Add);
    let mut ui = simulator(Size::new(260.0, 400.0), crate::db_panel::view(&db));
    // "Postgres" é o segundo item do menu, logo abaixo do "+" do cabeçalho
    ui.point_at(Point::new(120.0, 86.0));
    let _ = ui.simulate(click());
    let messages: Vec<DbMsg> = ui.into_messages().collect();
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, DbMsg::AddEngine(Engine::Postgres))),
        "{messages:?}"
    );
}

#[test]
fn clicking_outside_the_menu_dismisses_it_without_reaching_the_panel() {
    let mut db = panel();
    db.menu = Some(MenuState::Add);
    let mut ui = simulator(Size::new(260.0, 400.0), crate::db_panel::view(&db));
    // em cima da linha da conexão: sem o popup o clique expandiria a conexão
    ui.point_at(Point::new(20.0, 100.0));
    let _ = ui.simulate(click());
    let messages: Vec<DbMsg> = ui.into_messages().collect();
    assert!(
        messages.iter().any(|m| matches!(m, DbMsg::CloseMenu)),
        "{messages:?}"
    );
    assert!(
        !messages.iter().any(|m| matches!(m, DbMsg::Toggle(_))),
        "{messages:?}"
    );
}

#[test]
fn dragging_the_divider_publishes_the_cursor_travel() {
    let connections = vec![Connection::sqlite(
        "app.db",
        "a.db",
        Origin::Registered,
        Access::Read,
        true,
    )];
    let mut query = DbQuery::scratch(1, 1, "-- db: app.db\nSELECT 1;\n", PathBuf::new());
    let secrets = SecretStore::new(PathBuf::from("/nonexistent/secrets.json"));
    let rows = (0..5)
        .map(|n| vec![Cell::Int(n), Cell::Text(format!("n{n}"))])
        .collect();
    let result = QueryResult::from_rows(
        vec!["id".into(), "name".into()],
        rows,
        false,
        Duration::ZERO,
    );
    let _ = query.update(QueryMsg::Done(Ok(result)), &Ctx { secrets: &secrets });

    let mut ui = simulator(
        Size::new(900.0, 560.0),
        crate::db_query_view::view(&query, &connections),
    );
    let move_to = |y: f32| {
        Event::Mouse(mouse::Event::CursorMoved {
            position: Point::new(450.0, y),
        })
    };
    // barra de 36 + editor de 249: o divisor de 7px fica em y ~ 285..292
    ui.point_at(Point::new(450.0, 288.0));
    let _ = ui.simulate([
        move_to(288.0),
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        move_to(308.0),
        move_to(318.0),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        move_to(400.0),
    ]);
    let travelled: f32 = ui
        .into_messages()
        .filter_map(|message| match message {
            QueryMsg::DragSplit(delta) => Some(delta),
            _ => None,
        })
        .sum();
    assert!((travelled - 30.0).abs() < 0.01, "{travelled}");
}
