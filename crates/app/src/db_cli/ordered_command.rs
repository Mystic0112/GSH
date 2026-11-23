use std::fmt;

use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

pub(super) struct OrderedCommand(pub(super) Vec<(String, Value)>);

impl<'de> Deserialize<'de> for OrderedCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct OrderedCommandVisitor;

        impl<'de> Visitor<'de> for OrderedCommandVisitor {
            type Value = OrderedCommand;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a MongoDB command object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut entries = Vec::new();
                while let Some(entry) = map.next_entry::<String, Value>()? {
                    entries.push(entry);
                }
                Ok(OrderedCommand(entries))
            }
        }

        deserializer.deserialize_map(OrderedCommandVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::OrderedCommand;

    #[test]
    fn mongo_parser_preserves_the_run_command_first_key() {
        let command: OrderedCommand =
            serde_json::from_str(r#"{"listDatabases":1,"nameOnly":true}"#).unwrap();
        assert_eq!(
            command.0.first().map(|(name, _)| name.as_str()),
            Some("listDatabases")
        );
    }

    #[test]
    fn mongo_parser_rejects_non_object_json() {
        assert!(serde_json::from_str::<OrderedCommand>("[]").is_err());
    }
}
