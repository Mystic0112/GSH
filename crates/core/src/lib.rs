//! Domínio puro do GSH: sem I/O, sem UI.
//!
//! Modelo:
//! - [`ids`]: identificadores estáveis de workspace, pane e aba;
//! - [`pane`]: árvore de divisões (folhas com abas);
//! - [`workspace`]: workspaces, abas e as transições entre eles.
//!
//! Protocolo da CLI `gsh` (sem socket, sem disco):
//! - [`wire`]: nomes de env, montagem de requisição e leitura de resposta;
//! - [`hook`]: eventos de hook dos harnesses -> status de turno, payload do socket;
//! - [`tabs`]: formatação da listagem de abas.
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

pub mod hook;
pub mod ids;
pub mod pane;
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
