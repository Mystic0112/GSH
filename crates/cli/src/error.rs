//! Erros da CLI e o código de saída de cada um (mesmos da CLI original).

use thiserror::Error;

use crate::transport::TransportError;

#[derive(Debug, Error)]
pub enum CliError {
    /// Erro de uso (exit 2). A mensagem já vem completa, com prefixo.
    #[error("{0}")]
    Usage(String),
    /// O hook bloqueou o comando do agente (exit 2); o texto vai pro agente.
    #[error("{0}")]
    Blocked(String),
    /// O app respondeu `ok: false` (exit 1).
    #[error("gsh: {0}")]
    App(String),
    /// Sem app para falar, ou conexão falhou (exit 3).
    #[error("gsh: {0}")]
    Transport(#[from] TransportError),
}

impl CliError {
    pub fn exit_code(&self) -> u8 {
        match self {
            CliError::Usage(_) | CliError::Blocked(_) => 2,
            CliError::App(_) => 1,
            CliError::Transport(_) => 3,
        }
    }

    pub fn usage(msg: impl Into<String>) -> CliError {
        CliError::Usage(msg.into())
    }
}
