//! Motor de terminal do GSH (sem UI): PTY nativo via `portable-pty` e emulador
//! VT/ANSI via `alacritty_terminal` (ADR 0002).
//!
//! Expõe apenas tipos próprios na API pública para que a camada de UI desenhe o
//! grid célula por célula sem acoplamento ao motor interno.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

mod error;
mod session;
mod snapshot;

pub use error::TermError;
pub use session::{
    default_shell, IntoShell, Session, ShellCommand, TermSession, DEFAULT_SCROLLBACK_LINES,
    DEFAULT_UNIX_SHELL, DEFAULT_WINDOWS_SHELL,
};
pub use snapshot::{
    indexed_to_rgb, Cell, CellColor, Cursor, GridPos, NamedColor, Rgb, Snapshot, SnapshotCell,
    TermColor, TermSnapshot,
};
