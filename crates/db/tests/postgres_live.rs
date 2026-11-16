//! Contra um Postgres de verdade. Pula sozinho sem `GSH_TEST_PG` (ex.:
//! `GSH_TEST_PG=postgres://gsh:gshpw@127.0.0.1:55432/gshdb cargo test -p gsh-db --test postgres_live`).

use std::path::Path;

use gsh_db::connection::Origin;
use gsh_db::schema::{qualified_ident, SchemaTable};
use gsh_db::{driver, Cell, Connection, DbError, Endpoint};

fn endpoint(url: &str) -> Endpoint {
    let conn = Connection::from_json(
        &serde_json::json!({"name": "live", "url": url}),
        Origin::Registered,
    )
    .unwrap();
    let password = conn.url_password();
    Endpoint::resolve(&conn, Path::new("/"), password).unwrap()
}

#[test]
fn queries_schema_and_errors_against_a_real_server() {
    let Ok(url) = std::env::var("GSH_TEST_PG") else {
        eprintln!("GSH_TEST_PG ausente: teste pulado");
        return;
    };
    let ep = endpoint(&url);
    driver::ping(&ep).unwrap();

    for sql in [
        "DROP SCHEMA IF EXISTS gsh_t CASCADE",
        "CREATE SCHEMA gsh_t",
        "CREATE TABLE gsh_t.\"Orders\" (id serial PRIMARY KEY, total numeric(10,2), paid boolean, note text, raw bytea, score float8)",
        "INSERT INTO gsh_t.\"Orders\" (total, paid, note, raw, score) VALUES (10.50, true, 'a', '\\x0102', 1.5), (NULL, false, NULL, NULL, NULL), (3, NULL, 'c', NULL, 2)",
        "CREATE VIEW gsh_t.big AS SELECT id FROM gsh_t.\"Orders\" WHERE total > 5",
    ] {
        driver::execute(&ep, sql).unwrap();
    }

    let result = driver::query(
        &ep,
        "SELECT id, total, paid, note, raw, score FROM gsh_t.\"Orders\" ORDER BY id",
        10,
    )
    .unwrap();
    let names: Vec<_> = result.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, ["id", "total", "paid", "note", "raw", "score"]);
    assert_eq!(result.rows[0][0], Cell::Int(1));
    assert_eq!(result.rows[0][1], Cell::Text("10.50".into()));
    assert_eq!(result.rows[0][2], Cell::Bool(true));
    assert_eq!(result.rows[0][4], Cell::Blob(vec![1, 2]));
    assert_eq!(result.rows[0][5], Cell::Real(1.5));
    assert_eq!(result.rows[1][1], Cell::Null);

    let limited = driver::query(&ep, "SELECT id FROM gsh_t.\"Orders\" ORDER BY id", 2).unwrap();
    assert!(limited.truncated && limited.rows.len() == 2);

    let tables = driver::tables(&ep).unwrap();
    let mine: Vec<&SchemaTable> = tables
        .iter()
        .filter(|t| t.schema.as_deref() == Some("gsh_t"))
        .collect();
    assert_eq!(
        mine.iter()
            .map(|t| (t.name.as_str(), t.is_view))
            .collect::<Vec<_>>(),
        [("Orders", false), ("big", true)]
    );
    let columns = driver::columns(&ep, "Orders", Some("gsh_t")).unwrap();
    assert_eq!(
        columns.first().map(|c| (c.name.as_str(), c.primary_key)),
        Some(("id", true))
    );
    assert_eq!(columns.iter().filter(|c| c.primary_key).count(), 1);

    // o SELECT que a árvore monta para "New query" roda com o identificador qualificado
    let select = format!(
        "SELECT * FROM {} LIMIT 100",
        qualified_ident(gsh_db::Engine::Postgres, Some("gsh_t"), "Orders")
    );
    assert_eq!(driver::query(&ep, &select, 100).unwrap().rows.len(), 3);

    let affected = driver::execute(&ep, "UPDATE gsh_t.\"Orders\" SET note = 'z'").unwrap();
    assert_eq!(affected.affected_rows, Some(3));

    let syntax = driver::query(&ep, "SELEC 1", 1).unwrap_err();
    assert!(
        matches!(&syntax, DbError::QueryFailed(text) if text.contains("syntax")),
        "{syntax:?}"
    );

    let bad_password = url.replacen(
        url.split(':')
            .nth(2)
            .unwrap_or("")
            .split('@')
            .next()
            .unwrap_or("x"),
        "wrong",
        1,
    );
    let denied = driver::ping(&endpoint(&bad_password)).unwrap_err();
    assert!(matches!(denied, DbError::ConnectionFailed(_)), "{denied:?}");

    driver::execute(&ep, "DROP SCHEMA gsh_t CASCADE").unwrap();
}
