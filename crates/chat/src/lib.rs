//! Motor de chat com o Claude Code em `stream-json` (sem UI, sem `iced`).
//!
//! - [`protocol`]: argumentos do processo, linhas de `stdin` e tradução das linhas de `stdout`
//!   em eventos (puro).
//! - [`event`]: [`ChatEvent`] e os tipos de conta (modelos, comandos, limites do plano).
//! - [`engine`]: spawn do processo filho, escrita no `stdin`, leitura em thread.
//! - [`account`]: parsers de `initialize`/`get_usage` e o probe da conta sem conversa.
//! - [`attachment`]: anexos (imagem, PDF, arquivo) e o conteúdo da mensagem.
//! - [`history`]: transcript do Claude Code (histórico, título) e lista de sessões.
//! - [`settings`]: leitura do `~/.claude/settings.json`.
//! - [`resolve`]: onde está o `claude` e com que ambiente roda.
//! - [`error`]: erros tipados com `thiserror`.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

pub mod account;
pub mod attachment;
pub mod engine;
pub mod error;
pub mod event;
pub mod history;
mod json;
pub mod protocol;
pub mod resolve;
pub mod settings;
pub mod time;

pub use account::probe_account;
pub use attachment::{
    build_user_content, Attachment, AttachmentKind, MAX_IMAGE_BYTES, MAX_PDF_BYTES,
};
pub use engine::{ChatConfig, ChatEngine};
pub use error::ChatError;
pub use event::{
    AccountProbe, ChatEvent, CommandOption, Limit, LimitKind, ModelOption, PlanUsage, SessionState,
};
pub use history::{read_history, recent_sessions, transcript_file, write_title, SessionEntry};
pub use protocol::{tool_summary, LaunchOptions, DEFAULT_CLAUDE_BINARY};
pub use resolve::{env_with_dir_on_path, resolve_executable};
