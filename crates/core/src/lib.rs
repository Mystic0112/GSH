//! Domínio puro do GSH: sem I/O, sem UI.
//!
//! - [`workspace`], [`panes`] e [`tabs`]: estado e transições de workspaces, painéis e abas;
//! - [`wire`]: nomes de env, montagem de requisição e leitura de resposta;
//! - [`hook`]: eventos de hook dos harnesses -> status de turno, payload do socket;
//!
//! Lado do app do status de turno (sem socket, sem relógio):
//! - [`status`]: linha do socket -> `StatusUpdate` tipado e máquina de turno.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod agent_color;
pub mod file_activity;
pub mod hook;
pub mod panes;
pub mod status;
pub mod tabs;
pub mod wire;
pub mod workspace;

/// Versão do crate, exposta para a CLI e para diagnósticos.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!VERSION.is_empty());
    }
}
