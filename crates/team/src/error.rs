//! Erros do time. Texto de terceiro (SO, RNG) entra como campo, nunca como frase fixa.

use std::io;

#[derive(Debug, thiserror::Error)]
pub enum TeamError {
    #[error("{op}: {source}")]
    Io {
        op: &'static str,
        #[source]
        source: io::Error,
    },
    #[error("random bytes unavailable: {detail}")]
    Random { detail: String },
    #[error("could not seal the frame")]
    Seal,
}

impl TeamError {
    pub(crate) fn io(op: &'static str) -> impl FnOnce(io::Error) -> Self {
        move |source| Self::Io { op, source }
    }
}
