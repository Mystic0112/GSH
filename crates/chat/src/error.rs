//! Erros tipados do motor de chat (`gsh-chat`).

use thiserror::Error;

/// Erros que podem ocorrer ao iniciar, operar ou decodificar uma sessão do Claude Code
/// em modo `stream-json`.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ChatError {
    /// O executável configurado para o Claude Code não foi encontrado no sistema.
    #[error("binário não encontrado ('{program}'): {cause}")]
    BinaryNotFound { program: String, cause: String },

    /// Falha de sistema operacional ao iniciar o processo filho.
    #[error("falha ao iniciar processo '{program}': {cause}")]
    SpawnFailed { program: String, cause: String },

    /// Os pipes `stdin` ou `stdout` do processo filho não puderam ser capturados.
    #[error("pipes stdio do processo filho indisponíveis")]
    StdioUnavailable,

    /// Falha de I/O ao escrever uma mensagem no `stdin` do processo filho.
    #[error("falha ao escrever no stdin do claude: {cause}")]
    WriteStdin { cause: String },

    /// Uma linha lida do `stdout` não é um objeto JSON válido do protocolo `stream-json`.
    #[error("linha JSON inválida no stream: '{line}' ({cause})")]
    InvalidJsonLine { line: String, cause: String },

    /// O processo filho encerrou inesperadamente (queda ou código de saída de erro).
    #[error("processo do claude encerrou (código {exit_code:?}): {stderr}")]
    ProcessExited {
        exit_code: Option<i32>,
        stderr: String,
    },

    /// Tentativa de envio após a sessão já ter sido encerrada ou cancelada.
    #[error("sessão de chat já encerrada")]
    SessionClosed,
}
