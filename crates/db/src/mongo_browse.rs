//! Navegador de coleções MongoDB (`mongo_browse_service.dart`): `listDatabases`,
//! `listCollections`, `find` paginado por `skip`/`limit`, `insertOne`, `replaceOne` e
//! `deleteOne` com relaxed extended JSON preservado.

use serde_json::{Map, Value};

use crate::driver::{self, Endpoint};
use crate::error::DbError;

/// Documentos por página do `find`.
pub const PAGE_SIZE: usize = 50;

fn check_ok(doc: &Map<String, Value>) -> Result<(), DbError> {
    if let Some(ok) = doc.get("ok").and_then(Value::as_f64) {
        if (ok - 1.0).abs() > f64::EPSILON {
            let msg = doc
                .get("errmsg")
                .or_else(|| doc.get("codeName"))
                .and_then(Value::as_str)
                .unwrap_or("Command failed.");
            return Err(DbError::QueryFailed(msg.to_string()));
        }
    }
    if let Some(write_errors) = doc.get("writeErrors").and_then(Value::as_array) {
        if let Some(first) = write_errors.first() {
            let msg = first
                .get("errmsg")
                .and_then(Value::as_str)
                .map_or_else(|| first.to_string(), str::to_string);
            return Err(DbError::QueryFailed(msg));
        }
    }
    Ok(())
}

fn run_ordered(
    endpoint: &Endpoint,
    entries: &[(&str, Value)],
    database: Option<&str>,
) -> Result<Map<String, Value>, DbError> {
    let reply = driver::mongo_ordered_command(endpoint, entries, database)?;
    let Value::Object(doc) = reply else {
        return Err(DbError::QueryFailed(format!("Unexpected reply: {reply}")));
    };
    check_ok(&doc)?;
    Ok(doc)
}

fn cursor_batch(reply: &Map<String, Value>) -> &[Value] {
    let Some(cursor) = reply.get("cursor").and_then(Value::as_object) else {
        return &[];
    };
    cursor
        .get("firstBatch")
        .or_else(|| cursor.get("nextBatch"))
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// Databases do deployment, sem os de sistema (`admin`, `config`, `local`), ordenados.
pub fn list_databases(endpoint: &Endpoint) -> Result<Vec<String>, DbError> {
    let reply = run_ordered(
        endpoint,
        &[
            ("listDatabases", Value::from(1)),
            ("nameOnly", Value::from(true)),
        ],
        Some("admin"),
    )?;
    let mut names: Vec<String> = reply
        .get("databases")
        .and_then(Value::as_array)
        .map_or(&[][..], Vec::as_slice)
        .iter()
        .filter_map(|entry| entry.get("name").and_then(Value::as_str))
        .filter(|name| !matches!(*name, "admin" | "config" | "local"))
        .map(str::to_string)
        .collect();
    names.sort();
    Ok(names)
}

/// Nomes das collections, ordenados.
pub fn list_collections(
    endpoint: &Endpoint,
    database: Option<&str>,
) -> Result<Vec<String>, DbError> {
    let reply = run_ordered(
        endpoint,
        &[
            ("listCollections", Value::from(1)),
            ("nameOnly", Value::from(true)),
        ],
        database,
    )?;
    let mut names: Vec<String> = cursor_batch(&reply)
        .iter()
        .filter_map(|entry| entry.get("name").and_then(Value::as_str))
        .map(str::to_string)
        .collect();
    names.sort();
    Ok(names)
}

/// Filter bar -> mapa JSON. Vazio = `{}`.
pub fn parse_filter(text: &str) -> Result<Map<String, Value>, DbError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Map::new());
    }
    let decoded: Value = serde_json::from_str(trimmed)
        .map_err(|e| DbError::QueryFailed(format!("Invalid filter JSON: {e}")))?;
    match decoded {
        Value::Object(map) => Ok(map),
        _ => Err(DbError::QueryFailed(
            "The filter must be a JSON object, e.g. {\"status\": \"active\"}.".into(),
        )),
    }
}

/// Editor de documento -> mapa JSON obrigatório.
pub fn parse_document(text: &str) -> Result<Map<String, Value>, DbError> {
    let decoded: Value = serde_json::from_str(text.trim())
        .map_err(|e| DbError::QueryFailed(format!("Invalid document JSON: {e}")))?;
    match decoded {
        Value::Object(map) => Ok(map),
        _ => Err(DbError::QueryFailed(
            "A document must be a JSON object.".into(),
        )),
    }
}

/// Uma página de documentos.
pub fn find(
    endpoint: &Endpoint,
    collection: &str,
    filter_json: &str,
    skip: usize,
    limit: usize,
    database: Option<&str>,
) -> Result<Vec<Map<String, Value>>, DbError> {
    let filter = parse_filter(filter_json)?;
    let mut entries = vec![
        ("find", Value::String(collection.to_string())),
        ("filter", Value::Object(filter)),
    ];
    if skip > 0 {
        entries.push(("skip", Value::from(skip as u64)));
    }
    entries.push(("limit", Value::from(limit as u64)));
    entries.push(("batchSize", Value::from(limit as u64)));
    entries.push(("singleBatch", Value::Bool(true)));

    let reply = run_ordered(endpoint, &entries, database)?;
    Ok(cursor_batch(&reply)
        .iter()
        .filter_map(|v| v.as_object().cloned())
        .collect())
}

/// Substitui o documento inteiro, ancorado no `_id` do próprio JSON editado.
pub fn replace_one(
    endpoint: &Endpoint,
    collection: &str,
    doc_json: &str,
    database: Option<&str>,
) -> Result<Map<String, Value>, DbError> {
    let doc = parse_document(doc_json)?;
    let Some(id) = doc.get("_id").filter(|v| !v.is_null()).cloned() else {
        return Err(DbError::QueryFailed(
            "The document must keep its \"_id\" field.".into(),
        ));
    };
    let mut query = Map::new();
    query.insert("_id".into(), id);
    let mut update = Map::new();
    update.insert("q".into(), Value::Object(query));
    update.insert("u".into(), Value::Object(doc.clone()));

    let reply = run_ordered(
        endpoint,
        &[
            ("update", Value::String(collection.to_string())),
            ("updates", Value::Array(vec![Value::Object(update)])),
        ],
        database,
    )?;
    if reply.get("n").and_then(Value::as_i64) == Some(0) {
        return Err(DbError::QueryFailed(
            "No document matched this \"_id\" (was it deleted?).".into(),
        ));
    }
    Ok(doc)
}

/// Insere um documento.
pub fn insert_one(
    endpoint: &Endpoint,
    collection: &str,
    doc_json: &str,
    database: Option<&str>,
) -> Result<(), DbError> {
    let doc = parse_document(doc_json)?;
    run_ordered(
        endpoint,
        &[
            ("insert", Value::String(collection.to_string())),
            ("documents", Value::Array(vec![Value::Object(doc)])),
        ],
        database,
    )?;
    Ok(())
}

/// Remove o documento com `_id == id`.
pub fn delete_one(
    endpoint: &Endpoint,
    collection: &str,
    id: Value,
    database: Option<&str>,
) -> Result<(), DbError> {
    let mut query = Map::new();
    query.insert("_id".into(), id);
    let mut del = Map::new();
    del.insert("q".into(), Value::Object(query));
    del.insert("limit".into(), Value::from(1));

    run_ordered(
        endpoint,
        &[
            ("delete", Value::String(collection.to_string())),
            ("deletes", Value::Array(vec![Value::Object(del)])),
        ],
        database,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_and_document_parsers_validate_json_object() {
        assert!(parse_filter("").unwrap().is_empty());
        assert!(parse_filter("   ").unwrap().is_empty());
        assert_eq!(
            parse_filter(r#"{"active": true}"#).unwrap().get("active"),
            Some(&Value::Bool(true))
        );
        assert!(parse_filter("[1, 2]").is_err());
        assert!(parse_filter("{bad").is_err());

        assert!(parse_document(r#"{"_id": 1}"#).is_ok());
        assert!(parse_document("123").is_err());
    }
}
