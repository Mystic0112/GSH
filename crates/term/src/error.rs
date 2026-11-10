//! Erros tipados da sessão de terminal (PTY + emulador VT).

use std::io;

use thiserror::Error;

/// Erros que podem ocorrer ao criar ou operar uma [` crate::TermSession `].
#[derive(Debug, Error)]
pub enum TermError {
    /// Dimensões inválidas para o terminal (mínimo de 2 colunas por 1 linha).
    #[error("invalid terminal dimensions: {cols}x{rows} (min 2x1)")]
    InvalidSize { cols: u16, rows: u16 },

    /// Falha ao abrir o par master/slave do PTY no sistema operacional.
    #[error("open pty ({cols}x{rows}): {cause}")]
    OpenPty { cols: u16, rows: u16, cause: String },

    /// Falha ao iniciar o processo filho no PTY.
    #[error("spawn '{shell}': {cause}")]
    Spawn { shell: String, cause: String },

    /// Falha ao clonar o leitor ou adquirir o escritor do master PTY.
    #[error("pty stream setup: {cause}")]
    PtyStream { cause: String },

    /// Falha ao criar a thread de leitura do PTY.
    #[error("spawn pty reader thread: {0}")]
    ReaderThread(#[source] io::Error),

    /// Falha de I/O ao enviar bytes para o PTY.
    #[error("write to pty: {0}")]
    Write(#[source] io::Error),

    /// O canal de escrita do PTY já foi encerrado.
    #[error("terminal session writer is closed")]
    Closed,

    /// Falha ao redimensionar o PTY no sistema operacional.
    #[error("resize pty ({cols}x{rows}): {cause}")]
    Resize { cols: u16, rows: u16, cause: String },
}
