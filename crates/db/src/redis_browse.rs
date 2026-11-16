//! Navegador de chaves Redis (`redis_key.dart` + `redis_browse_service.dart`): SCAN paginado,
//! preview por tipo, leitura completa e CRUD por célula/expansão.

use serde_json::{Map, Value};

use crate::driver::{self, Endpoint};
use crate::error::DbError;

/// COUNT do SCAN (tamanho aproximado da página).
pub const SCAN_COUNT: usize = 100;

/// Elementos de composto lidos para o preview.
const PREVIEW_ELEMS: usize = 20;

/// Bytes de STRING lidos para o preview (`GETRANGE`).
const PREVIEW_CHARS: usize = 300;

/// Tipos de valor do Redis que a tabela conhece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedisValueKind {
    String,
    Hash,
    List,
    Set,
    Zset,
    Other,
}

impl RedisValueKind {
    pub const CREATABLE: [Self; 5] = [Self::String, Self::Hash, Self::List, Self::Set, Self::Zset];

    pub const fn label(self) -> &'static str {
        match self {
            Self::String => "STRING",
            Self::Hash => "HASH",
            Self::List => "LIST",
            Self::Set => "SET",
            Self::Zset => "ZSET",
            Self::Other => "OTHER",
        }
    }

    pub const fn editable(self) -> bool {
        !matches!(self, Self::Other)
    }

    pub fn from_raw(raw: &str) -> Self {
        match raw {
            "string" => Self::String,
            "hash" => Self::Hash,
            "list" => Self::List,
            "set" => Self::Set,
            "zset" => Self::Zset,
            _ => Self::Other,
        }
    }
}

/// Uma linha da tabela Redis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisKeyEntry {
    pub key: String,
    pub kind: RedisValueKind,
    /// TTL em segundos; `-1` = sem expiração.
    pub ttl: i64,
    pub preview: String,
}

/// Uma página do SCAN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedisScanPage {
    pub entries: Vec<RedisKeyEntry>,
    pub cursor: String,
}

impl RedisScanPage {
    pub fn done(&self) -> bool {
        self.cursor == "0"
    }
}

fn scalar_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => other.to_string(),
    }
}

fn ttl_from_reply(v: &Value) -> i64 {
    match v {
        Value::Number(n) => n.as_i64().unwrap_or(-1),
        Value::String(s) => s.parse().unwrap_or(-1),
        _ => -1,
    }
}

fn preview_command(key: &str, kind: RedisValueKind) -> Option<Vec<String>> {
    let s = |v: &str| v.to_string();
    match kind {
        RedisValueKind::String => Some(vec![
            s("GETRANGE"),
            s(key),
            s("0"),
            PREVIEW_CHARS.to_string(),
        ]),
        RedisValueKind::Hash => Some(vec![s("HGETALL"), s(key)]),
        RedisValueKind::List => Some(vec![
            s("LRANGE"),
            s(key),
            s("0"),
            (PREVIEW_ELEMS - 1).to_string(),
        ]),
        RedisValueKind::Set => Some(vec![s("SMEMBERS"), s(key)]),
        RedisValueKind::Zset => Some(vec![
            s("ZRANGE"),
            s(key),
            s("0"),
            (PREVIEW_ELEMS - 1).to_string(),
            s("WITHSCORES"),
        ]),
        RedisValueKind::Other => None,
    }
}

fn full_command(key: &str, kind: RedisValueKind) -> Result<Vec<String>, DbError> {
    let s = |v: &str| v.to_string();
    match kind {
        RedisValueKind::String => Ok(vec![s("GET"), s(key)]),
        RedisValueKind::Hash => Ok(vec![s("HGETALL"), s(key)]),
        RedisValueKind::List => Ok(vec![s("LRANGE"), s(key), s("0"), s("-1")]),
        RedisValueKind::Set => Ok(vec![s("SMEMBERS"), s(key)]),
        RedisValueKind::Zset => Ok(vec![s("ZRANGE"), s(key), s("0"), s("-1"), s("WITHSCORES")]),
        RedisValueKind::Other => Err(DbError::QueryFailed(
            "This value type is read-only in the table.".into(),
        )),
    }
}

fn decode_composite(kind: RedisValueKind, reply: &Value) -> Value {
    match kind {
        RedisValueKind::Hash => {
            let mut map = Map::new();
            match reply {
                Value::Object(obj) => {
                    for (k, v) in obj {
                        map.insert(k.clone(), Value::String(scalar_text(v)));
                    }
                }
                Value::Array(flat) => {
                    for [k, v] in flat.as_chunks::<2>().0 {
                        map.insert(scalar_text(k), Value::String(scalar_text(v)));
                    }
                }
                _ => {}
            }
            Value::Object(map)
        }
        RedisValueKind::Zset => {
            let flat = reply.as_array().map(Vec::as_slice).unwrap_or(&[]);
            let mut out = Vec::new();
            if flat.first().is_some_and(Value::is_array) {
                for item in flat {
                    if let Some(pair) = item.as_array() {
                        if let (Some(val), Some(score)) = (pair.first(), pair.last()) {
                            let mut obj = Map::new();
                            obj.insert("value".into(), Value::String(scalar_text(val)));
                            obj.insert("score".into(), Value::String(scalar_text(score)));
                            out.push(Value::Object(obj));
                        }
                    }
                }
            } else {
                for [val, score] in flat.as_chunks::<2>().0 {
                    let mut obj = Map::new();
                    obj.insert("value".into(), Value::String(scalar_text(val)));
                    obj.insert("score".into(), Value::String(scalar_text(score)));
                    out.push(Value::Object(obj));
                }
            }
            Value::Array(out)
        }
        RedisValueKind::List | RedisValueKind::Set => {
            let list = reply.as_array().map(Vec::as_slice).unwrap_or(&[]);
            Value::Array(list.iter().map(|v| Value::String(scalar_text(v))).collect())
        }
        RedisValueKind::String | RedisValueKind::Other => Value::String(scalar_text(reply)),
    }
}

fn format_preview(kind: RedisValueKind, reply: &Value) -> String {
    if kind == RedisValueKind::String {
        return scalar_text(reply);
    }
    serde_json::to_string(&decode_composite(kind, reply)).unwrap_or_default()
}

fn json_element_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Constrói e valida o comando que recria um valor composto a partir do JSON do editor.
pub fn build_composite_command(
    key: &str,
    kind: RedisValueKind,
    json_text: &str,
) -> Result<Vec<String>, DbError> {
    let decoded: Value = serde_json::from_str(json_text)
        .map_err(|e| DbError::QueryFailed(format!("Invalid JSON: {e}")))?;
    match kind {
        RedisValueKind::Hash => {
            let Some(map) = decoded.as_object().filter(|m| !m.is_empty()) else {
                return Err(DbError::QueryFailed(
                    "A hash is a non-empty JSON object: {\"field\": \"value\"}.".into(),
                ));
            };
            let mut parts = vec!["HSET".to_string(), key.to_string()];
            for (field, val) in map {
                parts.push(field.clone());
                parts.push(json_element_string(val));
            }
            Ok(parts)
        }
        RedisValueKind::List | RedisValueKind::Set => {
            let Some(list) = decoded.as_array().filter(|a| !a.is_empty()) else {
                return Err(DbError::QueryFailed(format!(
                    "A {} is a non-empty JSON array.",
                    kind.label().to_ascii_lowercase()
                )));
            };
            let mut parts = vec![
                if kind == RedisValueKind::List {
                    "RPUSH"
                } else {
                    "SADD"
                }
                .to_string(),
                key.to_string(),
            ];
            for item in list {
                parts.push(json_element_string(item));
            }
            Ok(parts)
        }
        RedisValueKind::Zset => {
            let Some(list) = decoded.as_array().filter(|a| !a.is_empty()) else {
                return Err(DbError::QueryFailed(
                    "A zset is a non-empty JSON array of {\"value\": …, \"score\": …}.".into(),
                ));
            };
            let mut parts = vec!["ZADD".to_string(), key.to_string()];
            for item in list {
                let Some(obj) = item.as_object() else {
                    return Err(DbError::QueryFailed(
                        "Each zset item needs \"value\" and \"score\".".into(),
                    ));
                };
                let Some(val) = obj.get("value") else {
                    return Err(DbError::QueryFailed(
                        "Each zset item needs \"value\" and \"score\".".into(),
                    ));
                };
                let score_raw = obj.get("score").map(scalar_text).unwrap_or_default();
                let score: f64 = score_raw.trim().parse().map_err(|_| {
                    DbError::QueryFailed(format!(
                        "Invalid score for \"{}\".",
                        json_element_string(val)
                    ))
                })?;
                if !score.is_finite() {
                    return Err(DbError::QueryFailed(format!(
                        "Invalid score for \"{}\".",
                        json_element_string(val)
                    )));
                }
                parts.push(score.to_string());
                parts.push(json_element_string(val));
            }
            Ok(parts)
        }
        RedisValueKind::String | RedisValueKind::Other => {
            Err(DbError::QueryFailed("Not a composite value type.".into()))
        }
    }
}

/// Uma página de chaves: `SCAN` + `TYPE`/`TTL`/preview de cada chave.
pub fn scan(endpoint: &Endpoint, pattern: &str, cursor: &str) -> Result<RedisScanPage, DbError> {
    let mut scan_cmd = vec!["SCAN".to_string(), cursor.to_string()];
    if !pattern.is_empty() && pattern != "*" {
        scan_cmd.push("MATCH".to_string());
        scan_cmd.push(pattern.to_string());
    }
    scan_cmd.push("COUNT".to_string());
    scan_cmd.push(SCAN_COUNT.to_string());

    let replies = driver::redis_batch(endpoint, &[scan_cmd])?;
    let page = replies
        .first()
        .and_then(Value::as_array)
        .filter(|a| a.len() >= 2);
    let Some(page) = page else {
        return Err(DbError::QueryFailed(format!(
            "Unexpected SCAN reply: {:?}",
            replies.first()
        )));
    };
    let next = page.first().map(scalar_text).unwrap_or_else(|| "0".into());
    let Some(raw_keys) = page.get(1).and_then(Value::as_array) else {
        return Err(DbError::QueryFailed(format!(
            "Unexpected SCAN reply: {page:?}"
        )));
    };
    let keys: Vec<String> = raw_keys.iter().map(scalar_text).collect();
    if keys.is_empty() {
        return Ok(RedisScanPage {
            entries: Vec::new(),
            cursor: next,
        });
    }

    let mut meta_cmds = Vec::with_capacity(keys.len() * 2);
    for key in &keys {
        meta_cmds.push(vec!["TYPE".to_string(), key.clone()]);
        meta_cmds.push(vec!["TTL".to_string(), key.clone()]);
    }
    let meta = driver::redis_batch(endpoint, &meta_cmds)?;

    let mut kinds = Vec::with_capacity(keys.len());
    let mut ttls = Vec::with_capacity(keys.len());
    for i in 0..keys.len() {
        let kind_raw = meta.get(i * 2).map(scalar_text).unwrap_or_default();
        let ttl = meta.get(i * 2 + 1).map_or(-1, ttl_from_reply);
        kinds.push(RedisValueKind::from_raw(&kind_raw));
        ttls.push(ttl);
    }

    let mut preview_cmds = Vec::new();
    let mut preview_owners = Vec::new();
    for (i, (key, kind)) in keys.iter().zip(kinds.iter().copied()).enumerate() {
        if let Some(cmd) = preview_command(key, kind) {
            preview_cmds.push(cmd);
            preview_owners.push(i);
        }
    }
    let mut previews = vec![String::new(); keys.len()];
    if !preview_cmds.is_empty() {
        let preview_replies = driver::redis_batch(endpoint, &preview_cmds)?;
        for (reply, owner) in preview_replies.iter().zip(preview_owners) {
            if let (Some(slot), Some(&kind)) = (previews.get_mut(owner), kinds.get(owner)) {
                *slot = format_preview(kind, reply);
            }
        }
    }

    let entries = keys
        .into_iter()
        .zip(kinds)
        .zip(ttls)
        .zip(previews)
        .map(|(((key, kind), ttl), preview)| RedisKeyEntry {
            key,
            kind,
            ttl,
            preview,
        })
        .collect();
    Ok(RedisScanPage {
        entries,
        cursor: next,
    })
}

/// Re-lê `TYPE`/`TTL`/preview de uma chave. `None` = chave não existe mais.
pub fn refresh_entry(endpoint: &Endpoint, key: &str) -> Result<Option<RedisKeyEntry>, DbError> {
    let meta = driver::redis_batch(
        endpoint,
        &[
            vec!["TYPE".to_string(), key.to_string()],
            vec!["TTL".to_string(), key.to_string()],
        ],
    )?;
    let raw_kind = meta.first().map(scalar_text).unwrap_or_default();
    if raw_kind == "none" {
        return Ok(None);
    }
    let kind = RedisValueKind::from_raw(&raw_kind);
    let ttl = meta.get(1).map_or(-1, ttl_from_reply);
    let preview = match preview_command(key, kind) {
        Some(cmd) => {
            let replies = driver::redis_batch(endpoint, &[cmd])?;
            replies
                .first()
                .map(|r| format_preview(kind, r))
                .unwrap_or_default()
        }
        None => String::new(),
    };
    Ok(Some(RedisKeyEntry {
        key: key.to_string(),
        kind,
        ttl,
        preview,
    }))
}

/// Valor completo para edição: STRING crua; compostos como JSON indentado.
pub fn read_full(endpoint: &Endpoint, key: &str, kind: RedisValueKind) -> Result<String, DbError> {
    let cmd = full_command(key, kind)?;
    let replies = driver::redis_batch(endpoint, &[cmd])?;
    let reply = replies.first().unwrap_or(&Value::Null);
    if kind == RedisValueKind::String {
        return Ok(scalar_text(reply));
    }
    serde_json::to_string_pretty(&decode_composite(kind, reply))
        .map_err(|e| DbError::QueryFailed(e.to_string()))
}

/// STRING: `SET` preservando o TTL (`KEEPTTL`).
pub fn write_string(endpoint: &Endpoint, key: &str, value: &str) -> Result<(), DbError> {
    driver::redis_batch(
        endpoint,
        &[vec![
            "SET".to_string(),
            key.to_string(),
            value.to_string(),
            "KEEPTTL".to_string(),
        ]],
    )?;
    Ok(())
}

/// Composto: valida JSON antes do `DEL`, lê o TTL e recria aplicando `EXPIRE` se `ttl > 0`.
pub fn write_composite(
    endpoint: &Endpoint,
    key: &str,
    kind: RedisValueKind,
    json_text: &str,
) -> Result<(), DbError> {
    let build = build_composite_command(key, kind, json_text)?;
    let ttl_replies = driver::redis_batch(endpoint, &[vec!["TTL".to_string(), key.to_string()]])?;
    let ttl = ttl_replies.first().map_or(-1, ttl_from_reply);
    let mut cmds = vec![vec!["DEL".to_string(), key.to_string()], build];
    if ttl > 0 {
        cmds.push(vec!["EXPIRE".to_string(), key.to_string(), ttl.to_string()]);
    }
    driver::redis_batch(endpoint, &cmds)?;
    Ok(())
}

/// Renomeia a chave sem sobrescrever destino existente (`RENAMENX`).
pub fn rename(endpoint: &Endpoint, old_key: &str, new_key: &str) -> Result<(), DbError> {
    if new_key.is_empty() {
        return Err(DbError::QueryFailed("Key must not be empty.".into()));
    }
    if new_key == old_key {
        return Ok(());
    }
    let replies = driver::redis_batch(
        endpoint,
        &[vec![
            "RENAMENX".to_string(),
            old_key.to_string(),
            new_key.to_string(),
        ]],
    )?;
    let ok = replies.first().is_some_and(|v| scalar_text(v) != "0");
    if !ok {
        return Err(DbError::QueryFailed(format!(
            "Key \"{new_key}\" already exists."
        )));
    }
    Ok(())
}

/// `seconds > 0` -> `EXPIRE`; `None` ou `< 0` -> `PERSIST`.
pub fn set_ttl(endpoint: &Endpoint, key: &str, seconds: Option<i64>) -> Result<(), DbError> {
    let cmd = match seconds {
        Some(s) if s >= 0 => vec!["EXPIRE".to_string(), key.to_string(), s.to_string()],
        _ => vec!["PERSIST".to_string(), key.to_string()],
    };
    driver::redis_batch(endpoint, &[cmd])?;
    Ok(())
}

pub fn delete(endpoint: &Endpoint, key: &str) -> Result<(), DbError> {
    driver::redis_batch(endpoint, &[vec!["DEL".to_string(), key.to_string()]])?;
    Ok(())
}

/// Cria uma chave nova; recusa se já existir.
pub fn create(
    endpoint: &Endpoint,
    key: &str,
    kind: RedisValueKind,
    value_text: &str,
    ttl: Option<i64>,
) -> Result<(), DbError> {
    if key.is_empty() {
        return Err(DbError::QueryFailed("Key must not be empty.".into()));
    }
    let build = if kind == RedisValueKind::String {
        vec!["SET".to_string(), key.to_string(), value_text.to_string()]
    } else {
        build_composite_command(key, kind, value_text)?
    };
    let exists = driver::redis_batch(endpoint, &[vec!["EXISTS".to_string(), key.to_string()]])?;
    if exists.first().is_some_and(|v| scalar_text(v) == "1") {
        return Err(DbError::QueryFailed(format!(
            "Key \"{key}\" already exists."
        )));
    }
    let mut cmds = vec![build];
    if let Some(secs) = ttl.filter(|s| *s > 0) {
        cmds.push(vec![
            "EXPIRE".to_string(),
            key.to_string(),
            secs.to_string(),
        ]);
    }
    driver::redis_batch(endpoint, &cmds)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn decodes_composite_shapes_matching_flutter() {
        let hash = decode_composite(RedisValueKind::Hash, &json!(["a", "1", "b", "two"]));
        assert_eq!(hash, json!({"a": "1", "b": "two"}));

        let zset = decode_composite(RedisValueKind::Zset, &json!(["m1", "1.5", "m2", "2"]));
        assert_eq!(
            zset,
            json!([
                {"value": "m1", "score": "1.5"},
                {"value": "m2", "score": "2"}
            ])
        );
    }

    #[test]
    fn validates_and_builds_composite_commands() {
        assert_eq!(
            build_composite_command("h", RedisValueKind::Hash, r#"{"k": 1}"#).unwrap(),
            vec!["HSET", "h", "k", "1"]
        );
        assert_eq!(
            build_composite_command("l", RedisValueKind::List, r#"["x", "y"]"#).unwrap(),
            vec!["RPUSH", "l", "x", "y"]
        );
        assert_eq!(
            build_composite_command(
                "z",
                RedisValueKind::Zset,
                r#"[{"value": "a", "score": 2.5}]"#
            )
            .unwrap(),
            vec!["ZADD", "z", "2.5", "a"]
        );
        assert!(build_composite_command("h", RedisValueKind::Hash, "{}").is_err());
        assert!(build_composite_command(
            "z",
            RedisValueKind::Zset,
            r#"[{"value":"a","score":"bad"}]"#
        )
        .is_err());
    }
}
