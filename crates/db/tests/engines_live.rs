//! Testes de integração vivos por motor e túnel SSH. Cada teste pula sozinho quando a variável
//! de ambiente correspondente não está definida:
//! - `GSH_TEST_PG_TLS`: URL Postgres com TLS (`sslmode=require|verify-ca|verify-full`)
//! - `GSH_TEST_MYSQL`: URL MySQL (`mysql://...`)
//! - `GSH_TEST_MSSQL`: URL SQL Server (`mssql://...`)
//! - `GSH_TEST_REDIS`: URL Redis (`redis://...`)
//! - `GSH_TEST_MONGO`: URL MongoDB (`mongodb://...`)
//! - `GSH_TEST_SSH` + `GSH_TEST_SSH_KEY` + `GSH_TEST_SSH_HOSTKEY` + `GSH_TEST_SSH_TARGET`:
//!   túnel SSH TOFU + forwarding real até o banco destino (`user@host:port`).

use std::path::Path;

use gsh_db::connection::Origin;
use gsh_db::mongo_browse;
use gsh_db::redis_browse::{self, RedisValueKind};
use gsh_db::ssh::SshTunnelManager;
use gsh_db::ssh_keys::SshHostKeyStore;
use gsh_db::{driver, Cell, Connection, DbError, Endpoint};
use serde_json::{json, Map, Value};

fn endpoint(url: &str) -> Endpoint {
    let conn =
        Connection::from_json(&json!({"name": "live", "url": url}), Origin::Registered).unwrap();
    let password = conn.url_password();
    Endpoint::resolve(&conn, Path::new("/"), password).unwrap()
}

#[test]
fn postgres_tls_connects_and_queries() {
    let Ok(url) = std::env::var("GSH_TEST_PG_TLS") else {
        eprintln!("GSH_TEST_PG_TLS ausente: teste pulado");
        return;
    };
    let ep = endpoint(&url);
    driver::ping(&ep).unwrap();
    let res = driver::query(&ep, "SELECT 1 AS ok", 1).unwrap();
    assert_eq!(res.rows.len(), 1);
    assert_eq!(res.rows[0][0], Cell::Int(1));
}

#[test]
fn mysql_queries_schema_and_dml() {
    let Ok(url) = std::env::var("GSH_TEST_MYSQL") else {
        eprintln!("GSH_TEST_MYSQL ausente: teste pulado");
        return;
    };
    let ep = endpoint(&url);
    driver::ping(&ep).unwrap();

    for sql in [
        "DROP TABLE IF EXISTS gsh_items",
        "CREATE TABLE gsh_items (id INT AUTO_INCREMENT PRIMARY KEY, label VARCHAR(64), amount DECIMAL(10,2), active BOOLEAN)",
        "INSERT INTO gsh_items (label, amount, active) VALUES ('alpha', 12.50, true), ('beta', NULL, false)",
    ] {
        driver::execute(&ep, sql).unwrap();
    }

    let res = driver::query(
        &ep,
        "SELECT id, label, CAST(amount AS CHAR) AS amount, active FROM gsh_items ORDER BY id",
        10,
    )
    .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(res.rows[0][0], Cell::Int(1));
    assert_eq!(res.rows[0][1], Cell::Text("alpha".into()));

    let tables = driver::tables(&ep).unwrap();
    assert!(tables.iter().any(|t| t.name == "gsh_items"), "{tables:?}");

    let cols = driver::columns(&ep, "gsh_items", None).unwrap();
    assert!(
        cols.iter().any(|c| c.name == "id" && c.primary_key),
        "{cols:?}"
    );
}

#[test]
fn mssql_queries_schema_and_dml() {
    let Ok(url) = std::env::var("GSH_TEST_MSSQL") else {
        eprintln!("GSH_TEST_MSSQL ausente: teste pulado");
        return;
    };
    let ep = endpoint(&url);
    driver::ping(&ep).unwrap();

    for sql in [
        "IF OBJECT_ID('dbo.gsh_items', 'U') IS NOT NULL DROP TABLE dbo.gsh_items",
        "CREATE TABLE dbo.gsh_items (id INT IDENTITY(1,1) PRIMARY KEY, label NVARCHAR(64), active BIT)",
        "INSERT INTO dbo.gsh_items (label, active) VALUES ('alpha', 1), ('beta', 0)",
    ] {
        driver::execute(&ep, sql).unwrap();
    }

    let res = driver::query(
        &ep,
        "SELECT id, label, active FROM dbo.gsh_items ORDER BY id",
        10,
    )
    .unwrap();
    assert_eq!(res.rows.len(), 2);
    assert_eq!(res.rows[0][0], Cell::Int(1));
    assert_eq!(res.rows[0][1], Cell::Text("alpha".into()));
    assert_eq!(res.rows[0][2], Cell::Bool(true));

    let tables = driver::tables(&ep).unwrap();
    assert!(tables.iter().any(|t| t.name == "gsh_items"), "{tables:?}");
    let cols = driver::columns(&ep, "gsh_items", Some("dbo")).unwrap();
    assert!(
        cols.iter().any(|c| c.name == "id" && c.primary_key),
        "{cols:?}"
    );
}

#[test]
fn redis_browse_scan_read_and_crud() {
    let Ok(url) = std::env::var("GSH_TEST_REDIS") else {
        eprintln!("GSH_TEST_REDIS ausente: teste pulado");
        return;
    };
    let ep = endpoint(&url);
    driver::ping(&ep).unwrap();

    let _ = redis_browse::delete(&ep, "gsh:str");
    let _ = redis_browse::delete(&ep, "gsh:str2");
    let _ = redis_browse::delete(&ep, "gsh:hash");

    redis_browse::create(&ep, "gsh:str", RedisValueKind::String, "hello", Some(120)).unwrap();
    redis_browse::create(
        &ep,
        "gsh:hash",
        RedisValueKind::Hash,
        r#"{"role":"admin","active":"1"}"#,
        None,
    )
    .unwrap();

    let page = redis_browse::scan(&ep, "gsh:*", "0").unwrap();
    assert!(page.entries.iter().any(|e| e.key == "gsh:str"));
    assert!(page.entries.iter().any(|e| e.key == "gsh:hash"));

    assert_eq!(
        redis_browse::read_full(&ep, "gsh:str", RedisValueKind::String).unwrap(),
        "hello"
    );
    redis_browse::write_string(&ep, "gsh:str", "world").unwrap();
    assert_eq!(
        redis_browse::read_full(&ep, "gsh:str", RedisValueKind::String).unwrap(),
        "world"
    );

    redis_browse::write_composite(&ep, "gsh:hash", RedisValueKind::Hash, r#"{"role":"owner"}"#)
        .unwrap();
    let full_hash = redis_browse::read_full(&ep, "gsh:hash", RedisValueKind::Hash).unwrap();
    assert!(full_hash.contains("owner"), "{full_hash}");

    redis_browse::rename(&ep, "gsh:str", "gsh:str2").unwrap();
    redis_browse::set_ttl(&ep, "gsh:str2", None).unwrap();
    redis_browse::delete(&ep, "gsh:str2").unwrap();
    redis_browse::delete(&ep, "gsh:hash").unwrap();
}

#[test]
fn mongo_databases_collections_and_crud() {
    let Ok(url) = std::env::var("GSH_TEST_MONGO") else {
        eprintln!("GSH_TEST_MONGO ausente: teste pulado");
        return;
    };
    let ep = endpoint(&url);
    driver::ping(&ep).unwrap();

    let mut drop_cmd = Map::new();
    drop_cmd.insert("drop".into(), Value::String("gsh_col".into()));
    let _ = driver::mongo_command(&ep, &drop_cmd, None);

    mongo_browse::insert_one(&ep, "gsh_col", r#"{"name":"Ana","status":"active"}"#, None).unwrap();

    let dbs = mongo_browse::list_databases(&ep).unwrap();
    assert!(!dbs.is_empty(), "esperava ao menos um database");
    let cols = mongo_browse::list_collections(&ep, None).unwrap();
    assert!(cols.iter().any(|c| c == "gsh_col"), "{cols:?}");

    let found = mongo_browse::find(&ep, "gsh_col", r#"{"status":"active"}"#, 0, 10, None).unwrap();
    assert_eq!(found.len(), 1);
    let mut updated = found[0].clone();
    updated.insert("name".into(), Value::String("Bia".into()));
    let replaced = mongo_browse::replace_one(
        &ep,
        "gsh_col",
        &serde_json::to_string(&updated).unwrap(),
        None,
    )
    .unwrap();
    assert_eq!(replaced.get("name").and_then(Value::as_str), Some("Bia"));

    let id = found[0].get("_id").unwrap().clone();
    mongo_browse::delete_one(&ep, "gsh_col", id, None).unwrap();
    let after = mongo_browse::find(&ep, "gsh_col", "", 0, 10, None).unwrap();
    assert!(after.is_empty());
}

#[test]
fn ssh_tunnel_tofu_prompt_and_forward_query() {
    let (Ok(ssh_spec), Ok(key_path), Ok(target_url)) = (
        std::env::var("GSH_TEST_SSH"),
        std::env::var("GSH_TEST_SSH_KEY"),
        std::env::var("GSH_TEST_SSH_TARGET"),
    ) else {
        eprintln!("GSH_TEST_SSH* ausente: teste pulado");
        return;
    };
    // GSH_TEST_SSH formato: user@host:port
    let (user, host_port) = ssh_spec.split_once('@').unwrap();
    let (host, port_str) = host_port.split_once(':').unwrap();
    let port: u16 = port_str.parse().unwrap();

    let conn = Connection::from_json(
        &json!({
            "name": "ssh-live",
            "url": target_url,
            "ssh": {
                "host": host,
                "port": port,
                "user": user,
                "keyPath": key_path,
                "savePassphrase": false
            }
        }),
        Origin::Registered,
    )
    .unwrap();

    let temp = tempfile::tempdir().unwrap();
    let store = SshHostKeyStore::new(temp.path().join("known-hosts.json"));
    let manager = SshTunnelManager::new(store.clone());

    let err =
        Endpoint::resolve_with_tunnel(&conn, Path::new("/"), conn.url_password(), None, &manager)
            .unwrap_err();
    let (ep_key, fingerprint) = match err {
        DbError::SshHostKeyUnknown {
            endpoint,
            fingerprint,
            ..
        } => (endpoint, fingerprint),
        other => panic!("esperava SshHostKeyUnknown, veio {other:?}"),
    };

    store.trust(&ep_key, &fingerprint).unwrap();
    let ep =
        Endpoint::resolve_with_tunnel(&conn, Path::new("/"), conn.url_password(), None, &manager)
            .unwrap();
    driver::ping(&ep).unwrap();
    manager.close_all();
}
