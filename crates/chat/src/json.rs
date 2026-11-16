//! Leitura tolerante de JSON (`o['k'] as String?` do Dart): campo ausente ou de outro tipo
//! vira `None`, nunca erro.

use serde_json::{Map, Value};

pub(crate) fn str_of<'a>(m: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    m.get(key).and_then(Value::as_str)
}

pub(crate) fn string_of(m: &Map<String, Value>, key: &str) -> String {
    str_of(m, key).unwrap_or("").to_string()
}

pub(crate) fn obj_of<'a>(m: &'a Map<String, Value>, key: &str) -> Option<&'a Map<String, Value>> {
    m.get(key).and_then(Value::as_object)
}

pub(crate) fn arr_of<'a>(m: &'a Map<String, Value>, key: &str) -> Option<&'a Vec<Value>> {
    m.get(key).and_then(Value::as_array)
}

/// `v is num ? v.toInt() : 0`.
pub(crate) fn int_of(m: &Map<String, Value>, key: &str) -> u64 {
    match m.get(key) {
        Some(Value::Number(n)) => n
            .as_u64()
            .or_else(|| n.as_f64().map(|f| if f > 0.0 { f as u64 } else { 0 }))
            .unwrap_or(0),
        _ => 0,
    }
}
