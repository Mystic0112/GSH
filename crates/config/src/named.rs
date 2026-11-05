//! Enums gravados pelo nome (`AppThemeMode.system.name` no Dart) e a leitura
//! tolerante do Flutter: nome desconhecido não é erro (arquivo de versão mais
//! nova, variante removida) e cai no padrão, ou é ignorado em listas e mapas.
//! Tipo errado (número no lugar do nome) continua sendo erro.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer};

/// Enum com nome estável no arquivo.
pub(crate) trait Named: Sized + Copy + 'static {
    const ALL: &'static [Self];

    fn name(self) -> &'static str;

    fn from_name(raw: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|v| v.name() == raw)
    }
}

/// Declara o enum, o `name()` público e o [`Named`]. O nome é o do Dart.
macro_rules! named_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident => $text:literal ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $name {
            $( $(#[$vmeta])* $variant ),+
        }

        impl $name {
            /// Nome gravado no arquivo (o `.name` do enum no Dart).
            pub fn name(self) -> &'static str {
                match self {
                    $( Self::$variant => $text ),+
                }
            }
        }

        impl $crate::named::Named for $name {
            const ALL: &'static [Self] = &[$( Self::$variant ),+];

            fn name(self) -> &'static str {
                $name::name(self)
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.name())
            }
        }
    };
}

pub(crate) use named_enum;

/// Campo enum: nome desconhecido = padrão (`_enumByName` do Dart).
pub(crate) fn or_default<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Named + Default,
{
    let raw = String::deserialize(de)?;
    Ok(T::from_name(&raw).unwrap_or_default())
}

/// Lista de nomes como conjunto: desconhecidos são ignorados.
pub(crate) fn known_set<'de, D, T>(de: D) -> Result<BTreeSet<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Named + Ord,
{
    let raw = Vec::<String>::deserialize(de)?;
    Ok(raw.iter().filter_map(|n| T::from_name(n)).collect())
}

/// Mapa por nome do enum: chaves desconhecidas são ignoradas
/// (`_soundEventMap` do Dart).
pub(crate) fn known_map<'de, D, K, V>(de: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Named + Ord,
    V: Deserialize<'de>,
{
    let raw = BTreeMap::<String, V>::deserialize(de)?;
    Ok(raw
        .into_iter()
        .filter_map(|(k, v)| K::from_name(&k).map(|k| (k, v)))
        .collect())
}
