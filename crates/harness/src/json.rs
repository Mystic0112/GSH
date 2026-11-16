//! JSON que guarda a ordem das chaves do arquivo.
//!
//! O `serde_json::Value` ordena por chave, mas o Dart lista os itens e regrava o
//! `settings.json` na ordem em que estão no arquivo.

use std::fmt;

use serde::de::{Deserialize, Deserializer, Error, MapAccess, SeqAccess, Visitor};
use serde_json::Number;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(Number),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn parse(text: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(text)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Self::Obj(items) => items.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_arr(&self) -> Option<&[Json]> {
        match self {
            Self::Arr(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_obj(&self) -> Option<&[(String, Json)]> {
        match self {
            Self::Obj(items) => Some(items),
            _ => None,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Valor do `key` quando existe e não é `null` (o `??` do Dart).
    pub fn non_null(&self, key: &str) -> Option<&Json> {
        self.get(key).filter(|value| !value.is_null())
    }

    /// Como o `toString()` do Dart interpola o valor.
    pub fn dart_string(&self) -> String {
        match self {
            Self::Null => String::from("null"),
            Self::Bool(b) => b.to_string(),
            Self::Num(n) => n.to_string(),
            Self::Str(s) => s.clone(),
            Self::Arr(items) => {
                let parts: Vec<String> = items.iter().map(Json::dart_string).collect();
                format!("[{}]", parts.join(", "))
            }
            Self::Obj(items) => {
                let parts: Vec<String> = items
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.dart_string()))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
        }
    }

    /// `JsonEncoder.withIndent('  ')` do Dart: 2 espaços, `{}` e `[]` vazios.
    pub fn to_pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, 0);
        out
    }

    fn write(&self, out: &mut String, depth: usize) {
        match self {
            Self::Null => out.push_str("null"),
            Self::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Self::Num(n) => out.push_str(&n.to_string()),
            Self::Str(s) => out.push_str(&quote(s)),
            Self::Arr(items) if items.is_empty() => out.push_str("[]"),
            Self::Obj(items) if items.is_empty() => out.push_str("{}"),
            Self::Arr(items) => {
                out.push_str("[\n");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(",\n");
                    }
                    indent(out, depth + 1);
                    item.write(out, depth + 1);
                }
                out.push('\n');
                indent(out, depth);
                out.push(']');
            }
            Self::Obj(items) => {
                out.push_str("{\n");
                for (i, (key, value)) in items.iter().enumerate() {
                    if i > 0 {
                        out.push_str(",\n");
                    }
                    indent(out, depth + 1);
                    out.push_str(&quote(key));
                    out.push_str(": ");
                    value.write(out, depth + 1);
                }
                out.push('\n');
                indent(out, depth);
                out.push('}');
            }
        }
    }
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| String::from("\"\""))
}

impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("um valor JSON")
    }

    fn visit_bool<E: Error>(self, v: bool) -> Result<Json, E> {
        Ok(Json::Bool(v))
    }

    fn visit_i64<E: Error>(self, v: i64) -> Result<Json, E> {
        Ok(Json::Num(Number::from(v)))
    }

    fn visit_u64<E: Error>(self, v: u64) -> Result<Json, E> {
        Ok(Json::Num(Number::from(v)))
    }

    fn visit_f64<E: Error>(self, v: f64) -> Result<Json, E> {
        Number::from_f64(v)
            .map(Json::Num)
            .ok_or_else(|| E::custom("número não finito"))
    }

    fn visit_str<E: Error>(self, v: &str) -> Result<Json, E> {
        Ok(Json::Str(v.to_owned()))
    }

    fn visit_string<E: Error>(self, v: String) -> Result<Json, E> {
        Ok(Json::Str(v))
    }

    fn visit_unit<E: Error>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_none<E: Error>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut items = Vec::new();
        while let Some(item) = seq.next_element()? {
            items.push(item);
        }
        Ok(Json::Arr(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut items: Vec<(String, Json)> = Vec::new();
        while let Some((key, value)) = map.next_entry::<String, Json>()? {
            // ponytail: busca linear; chave repetida troca o valor e mantém a posição
            // (como o Map do Dart). Vira índice se alguém tiver objeto com milhares de chaves.
            match items.iter_mut().find(|(k, _)| *k == key) {
                Some(slot) => slot.1 = value,
                None => items.push((key, value)),
            }
        }
        Ok(Json::Obj(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_key_order_and_repeats_keep_first_position() {
        let json = Json::parse(r#"{"b":1,"a":2,"b":3}"#).unwrap();
        let keys: Vec<&str> = json
            .as_obj()
            .unwrap()
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(keys, ["b", "a"]);
        assert_eq!(json.get("b"), Some(&Json::Num(Number::from(3))));
    }

    #[test]
    fn pretty_matches_dart_indent_encoder() {
        let json = Json::parse(r#"{"a":[1,"x"],"b":{},"c":[],"d":null,"e":{"f":true}}"#).unwrap();
        assert_eq!(
            json.to_pretty(),
            "{\n  \"a\": [\n    1,\n    \"x\"\n  ],\n  \"b\": {},\n  \"c\": [],\n  \"d\": null,\n  \"e\": {\n    \"f\": true\n  }\n}"
        );
    }

    #[test]
    fn invalid_and_empty_text_fail() {
        assert!(Json::parse("{inválido").is_err());
        assert!(Json::parse("").is_err());
    }

    #[test]
    fn dart_string_of_scalars_and_collections() {
        let json = Json::parse(r#"{"n":5,"f":1.5,"l":["a",2],"m":{"k":"v"},"z":null}"#).unwrap();
        assert_eq!(json.get("n").unwrap().dart_string(), "5");
        assert_eq!(json.get("f").unwrap().dart_string(), "1.5");
        assert_eq!(json.get("l").unwrap().dart_string(), "[a, 2]");
        assert_eq!(json.get("m").unwrap().dart_string(), "{k: v}");
        assert!(json.non_null("z").is_none());
    }
}
