//! Agent database commands executed by the app, without exposing credentials.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{json, Value};

mod ordered_command;
use ordered_command::OrderedCommand;

const OUTPUT_LIMIT: usize = 1024 * 1024;

struct CommandContext<'a> {
    root: &'a Path,
    secrets: &'a gsh_db::SecretStore,
    passphrases: &'a BTreeMap<(String, String), String>,
    mongo_store: &'a gsh_db::MongoDatabaseStore,
}

pub(super) fn run(
    root: &Path,
    verb: &str,
    args: &Value,
    secrets: &gsh_db::SecretStore,
    passphrases: &BTreeMap<(String, String), String>,
    mongo_store: &gsh_db::MongoDatabaseStore,
) -> Value {
    let response = run_unbounded(root, verb, args, secrets, passphrases, mongo_store);
    bound_response(response)
}

fn run_unbounded(
    root: &Path,
    verb: &str,
    args: &Value,
    secrets: &gsh_db::SecretStore,
    passphrases: &BTreeMap<(String, String), String>,
    mongo_store: &gsh_db::MongoDatabaseStore,
) -> Value {
    let context = CommandContext {
        root,
        secrets,
        passphrases,
        mongo_store,
    };
    let connections = gsh_db::store::load(root);
    if verb == "db-list" {
        let entries = connections
            .into_iter()
            .filter(|connection| connection.agents)
            .map(|connection| {
                json!({
                    "name": connection.name,
                    "engine": connection.engine.label(),
                    "target": connection.display_target(),
                    "origin": match connection.origin {
                        gsh_db::Origin::Registered => "registered",
                        gsh_db::Origin::Local => "local",
                        gsh_db::Origin::Detected => "detected",
                    },
                    "access": connection.access.name(),
                })
            })
            .collect::<Vec<_>>();
        return json!({"ok": true, "data": entries});
    }

    let dbq = if verb == "db-run" {
        let path = args.get("path").and_then(Value::as_str).unwrap_or_default();
        if path.is_empty() {
            return error("error", "missing .dbq path");
        }
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(err) => return error("error", &format!("cannot read \"{path}\": {err}")),
        };
        let document = gsh_db::DbqDocument::parse(&content);
        if document.db.is_none() {
            return error(
                "unknown_connection",
                &format!(
                    "\"{path}\" has no \"-- db:\" frontmatter — pick a database in the GSH tab or add the line manually"
                ),
            );
        }
        Some(document)
    } else {
        None
    };
    let name = dbq
        .as_ref()
        .and_then(|document| document.db.as_deref())
        .or_else(|| args.get("db").and_then(Value::as_str))
        .unwrap_or_default();
    if name.is_empty() {
        return error("error", "missing --db <name>");
    }
    let visible = connections
        .iter()
        .filter(|connection| connection.agents)
        .collect::<Vec<_>>();
    let Some(connection) = visible
        .iter()
        .find(|connection| connection.name == name)
        .copied()
    else {
        let available = visible
            .iter()
            .map(|connection| connection.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        return error(
            "unknown_connection",
            &format!(
                "no connection named \"{name}\" (available: {})",
                if available.is_empty() {
                    "none"
                } else {
                    &available
                }
            ),
        );
    };

    match verb {
        "redis-cmd" => run_redis(&context, args, connection),
        "mongo-cmd" => run_mongo(&context, args, connection),
        "db-schema" | "db-query" | "db-execute" | "db-run" => {
            run_sql(&context, verb, args, dbq.as_ref(), name, connection)
        }
        _ => error("query_failed", "unknown database command"),
    }
}

fn run_sql(
    context: &CommandContext<'_>,
    verb: &str,
    args: &Value,
    dbq: Option<&gsh_db::DbqDocument>,
    name: &str,
    connection: &gsh_db::Connection,
) -> Value {
    if verb == "db-execute" && connection.access == gsh_db::Access::Read {
        return error(
            "read_only_connection",
            "this connection is read-only for agents — enable Read & write on it in the Database panel",
        );
    }
    let sql = match verb {
        "db-schema" => {
            let table = args
                .get("table")
                .and_then(Value::as_str)
                .filter(|table| !table.is_empty());
            match gsh_db::schema::schema_sql(connection.engine, table, None) {
                Ok(sql) => sql,
                Err(error) => {
                    return db_error(error, Some(connection), context.root, context.secrets)
                }
            }
        }
        "db-query" | "db-execute" => args
            .get("sql")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        "db-run" => dbq.map(|document| document.sql.clone()).unwrap_or_default(),
        _ => return error("query_failed", "unknown database command"),
    };
    let statements = if verb == "db-run" {
        gsh_db::statements::split(&sql)
            .into_iter()
            .map(|statement| statement.text)
            .collect::<Vec<_>>()
    } else {
        vec![sql.clone()]
    };
    if statements.is_empty() && verb == "db-run" {
        return error("query_failed", "nothing to run");
    }
    if verb == "db-query" || (verb == "db-run" && connection.access == gsh_db::Access::Read) {
        for statement in &statements {
            if let Some(message) = gsh_db::access::sql_read_violation(statement) {
                return error("read_only_connection", &message);
            }
        }
    }
    let limit = dbq
        .and_then(|document| document.limit)
        .and_then(|limit| usize::try_from(limit).ok())
        .or_else(|| {
            args.get("limit")
                .and_then(Value::as_u64)
                .and_then(|limit| usize::try_from(limit).ok())
        })
        .unwrap_or(gsh_db::DEFAULT_LIMIT);
    let result = crate::db_state::run_on_with_ssh(
        context.root,
        context.secrets,
        name,
        context.passphrases,
        |endpoint| match verb {
            "db-execute" => gsh_db::driver::execute(&endpoint, &sql),
            "db-run" => gsh_db::driver::run_statements(&endpoint, &statements, Some(limit)),
            _ => gsh_db::driver::query(&endpoint, &sql, limit),
        },
    );
    match result {
        Ok(result) => json!({"ok": true, "data": query_json(result)}),
        Err(error) => db_error(error, Some(connection), context.root, context.secrets),
    }
}

fn run_redis(context: &CommandContext<'_>, args: &Value, connection: &gsh_db::Connection) -> Value {
    if connection.engine != gsh_db::Engine::Redis {
        return error(
            "unsupported_engine",
            &format!(
                "\"{}\" is a {} connection, not Redis",
                connection.name,
                connection.engine.label()
            ),
        );
    }
    let parts = args
        .get("parts")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|part| {
            part.as_str()
                .map_or_else(|| part.to_string(), str::to_owned)
        })
        .collect::<Vec<_>>();
    if connection.access == gsh_db::Access::Read {
        if let Some(message) = gsh_db::access::redis_read_violation(&parts) {
            return error("read_only_connection", &message);
        }
    }
    let result = crate::db_state::run_on_with_ssh(
        context.root,
        context.secrets,
        &connection.name,
        context.passphrases,
        |endpoint| gsh_db::driver::redis_command(&endpoint, &parts),
    );
    match result {
        Ok(reply) => json!({"ok": true, "data": reply}),
        Err(error) => db_error(error, Some(connection), context.root, context.secrets),
    }
}

fn run_mongo(context: &CommandContext<'_>, args: &Value, connection: &gsh_db::Connection) -> Value {
    if connection.engine != gsh_db::Engine::Mongo {
        return error(
            "unsupported_engine",
            &format!(
                "\"{}\" is a {} connection, not MongoDB",
                connection.name,
                connection.engine.label()
            ),
        );
    }
    let raw = args.get("command").and_then(Value::as_str).unwrap_or("{}");
    let command = match serde_json::from_str::<OrderedCommand>(raw) {
        Ok(command) => command.0,
        Err(_) => return error("query_failed", "invalid JSON command"),
    };
    if connection.access == gsh_db::Access::Read {
        if let Some(message) = gsh_db::access::mongo_read_violation(&command) {
            return error("read_only_connection", &message);
        }
    }
    let explicit = args
        .get("database")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    let database = if !explicit.is_empty() {
        explicit.to_string()
    } else if let Some(forced) = gsh_db::access::mongo_forced_database(&command) {
        forced.to_string()
    } else if !connection.database().is_empty() {
        connection.database()
    } else {
        context
            .mongo_store
            .selected(&context.root.to_string_lossy(), &connection.name)
            .unwrap_or_default()
    };
    if database.is_empty() {
        let available = mongo_database_names(context, connection);
        let suffix = if available.is_empty() {
            String::new()
        } else {
            format!(" (available: {})", available.join(", "))
        };
        return error(
            "query_failed",
            &format!(
                "connection \"{}\" has no database — its URL has none in the path and none was picked in the app. Pass --database <name>{suffix}.",
                connection.name
            ),
        );
    }
    let ordered = command
        .iter()
        .map(|(key, value)| (key.as_str(), value.clone()))
        .collect::<Vec<_>>();
    let result = crate::db_state::run_on_with_ssh(
        context.root,
        context.secrets,
        &connection.name,
        context.passphrases,
        |endpoint| gsh_db::driver::mongo_ordered_command(&endpoint, &ordered, Some(&database)),
    );
    match result {
        Ok(reply) => json!({"ok": true, "data": reply}),
        Err(error) => db_error(error, Some(connection), context.root, context.secrets),
    }
}

fn mongo_database_names(
    context: &CommandContext<'_>,
    connection: &gsh_db::Connection,
) -> Vec<String> {
    let command = [("listDatabases", Value::from(1))];
    let result = crate::db_state::run_on_with_ssh(
        context.root,
        context.secrets,
        &connection.name,
        context.passphrases,
        |endpoint| gsh_db::driver::mongo_ordered_command(&endpoint, &command, Some("admin")),
    );
    result
        .ok()
        .and_then(|reply| reply.get("databases").and_then(Value::as_array).cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|database| {
            database
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .collect()
}

fn query_json(mut result: gsh_db::QueryResult) -> Value {
    let mut output = query_value(&result);
    while serde_json::to_vec(&output).is_ok_and(|bytes| bytes.len() > OUTPUT_LIMIT)
        && !result.rows.is_empty()
    {
        result.rows.pop();
        result.truncated = true;
        output = query_value(&result);
    }
    output
}

fn bound_response(response: Value) -> Value {
    if serde_json::to_vec(&response).is_ok_and(|bytes| bytes.len() <= OUTPUT_LIMIT) {
        return response;
    }
    if response.get("ok") == Some(&Value::Bool(true)) {
        json!({
            "ok": true,
            "data": {
                "truncated": true,
                "preview": "[output exceeded 1 MiB]"
            }
        })
    } else {
        error(
            "output_too_large",
            "error details exceeded the output limit",
        )
    }
}

fn query_value(result: &gsh_db::QueryResult) -> Value {
    let columns = result
        .columns
        .iter()
        .map(|column| json!({"name": column.name, "type": column.ty}))
        .collect::<Vec<_>>();
    let mut data = json!({
        "columns": columns,
        "rows": result.rows,
        "rowCount": result.rows.len(),
        "truncated": result.truncated,
        "elapsedMs": result.elapsed.as_millis(),
    });
    if let (Some(affected), Value::Object(map)) = (result.affected_rows, &mut data) {
        map.insert("affectedRows".to_string(), json!(affected));
    }
    data
}

fn error(kind: &str, message: &str) -> Value {
    json!({"ok": false, "error": {"kind": kind, "message": message}})
}

fn db_error(
    db_error: gsh_db::DbError,
    connection: Option<&gsh_db::Connection>,
    root: &Path,
    secrets: &gsh_db::SecretStore,
) -> Value {
    let kind = db_error.kind();
    let mut message = db_error.to_string();
    if let Some(connection) = connection {
        if let Some(password) = gsh_db::resolve_password(connection, root, secrets) {
            redact_secret(&mut message, &password);
        }
        if let Some(password) = connection.url_password() {
            redact_secret(&mut message, &password);
        }
        message = message.replace(&connection.url, &gsh_db::redact_url(&connection.url));
    }
    error(kind, &message)
}

fn redact_secret(message: &mut String, secret: &str) {
    if !secret.is_empty() {
        *message = message.replace(secret, "[REDACTED]");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_errors_redact_passwords_embedded_in_connection_urls() {
        let root = Path::new("/tmp/workspace");
        let secrets = gsh_db::SecretStore::new(root.join("secrets.json"));
        let connection = gsh_db::Connection::from_json(
            &json!({
                "name": "private",
                "url": "postgres://agent:swordfish@db.example.test/app"
            }),
            gsh_db::Origin::Registered,
        )
        .unwrap();
        let output = db_error(
            gsh_db::DbError::ConnectionFailed(
                "could not connect to postgres://agent:swordfish@db.example.test/app".into(),
            ),
            Some(&connection),
            root,
            &secrets,
        );
        let serialized = output.to_string();
        assert!(!serialized.contains("swordfish"));
        assert!(serialized.contains("[REDACTED]") || serialized.contains("agent:***"));
    }

    #[test]
    fn output_cap_marks_successful_responses_as_truncated() {
        let response = json!({"ok":true,"data":{"payload":"x".repeat(OUTPUT_LIMIT + 1)}});
        let bounded = bound_response(response);
        assert_eq!(bounded["data"]["truncated"], true);
        assert!(serde_json::to_vec(&bounded).unwrap().len() <= OUTPUT_LIMIT);
    }
}
