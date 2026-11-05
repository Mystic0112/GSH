//! Identificadores estáveis. São `String` opacas (a CLI e o socket as trocam como
//! texto: `t3`, `pane7`, `ws0`); quem as gera é o estado, por contador, nunca a
//! posição na lista.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! id_type {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(id: impl Into<String>) -> Self {
                Self(id.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(id: &str) -> Self {
                Self::new(id)
            }
        }
    };
}

id_type!(
    /// Id de workspace (`ws0`, `ws1`...). Opaco: no Flutter é um UUID.
    WorkspaceId
);
id_type!(
    /// Id de nó da árvore de divisões: folha (`pane3`) ou divisão (`sp4`).
    PaneId
);
id_type!(
    /// Id de aba (`t3`, `a0`, `v5`...). Único entre todos os workspaces.
    TabId
);

/// Contador compartilhado por abas, folhas e divisões (como o `_seq` do Flutter):
/// o prefixo diz o tipo, o número nunca se repete.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct IdGen {
    next: u64,
}

impl IdGen {
    pub(crate) fn next(&mut self, prefix: &str) -> String {
        let n = self.next;
        // Saturar em vez de estourar: 2^64 ids não acontecem.
        self.next = self.next.saturating_add(1);
        format!("{prefix}{n}")
    }

    /// Avança o contador além do sufixo numérico de `id` (restauração: o próximo
    /// id gerado não pode colidir com um que veio do arquivo).
    pub(crate) fn bump_past(&mut self, id: &str) {
        if let Some(n) = numeric_suffix(id) {
            self.next = self.next.max(n.saturating_add(1));
        }
    }
}

/// Dígitos finais de `id` como número (`t12` -> 12), ou `None` sem sufixo.
pub(crate) fn numeric_suffix(id: &str) -> Option<u64> {
    let start = id
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_digit())
        .last()
        .map(|(i, _)| i)?;
    id.get(start..)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contador_unico_entre_prefixos() {
        let mut g = IdGen::default();
        assert_eq!(g.next("a"), "a0");
        assert_eq!(g.next("pane"), "pane1");
        assert_eq!(g.next("t"), "t2");
    }

    #[test]
    fn bump_past_ignora_sem_sufixo_e_nunca_recua() {
        let mut g = IdGen::default();
        g.bump_past("t12");
        g.bump_past("semnumero");
        g.bump_past("t3");
        assert_eq!(g.next("t"), "t13");
    }

    #[test]
    fn sufixo_numerico() {
        assert_eq!(numeric_suffix("pane7"), Some(7));
        assert_eq!(numeric_suffix("12"), Some(12));
        assert_eq!(numeric_suffix("abc"), None);
        assert_eq!(numeric_suffix(""), None);
        assert_eq!(numeric_suffix("t99999999999999999999999"), None);
    }

    #[test]
    fn id_serializa_como_texto() {
        let id = TabId::new("t1");
        assert_eq!(serde_json::to_string(&id).ok().as_deref(), Some("\"t1\""));
        assert_eq!(id.to_string(), "t1");
    }
}
