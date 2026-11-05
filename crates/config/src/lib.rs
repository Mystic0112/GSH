//! Configuração do GSH em arquivo (`~/.gsh/config.toml`), estilo Zed: o
//! usuário edita o TOML, o app relê ao salvar.
//!
//! Fica fora de `gsh-core` porque é I/O (disco, thread de polling, home do
//! usuário); o core é só domínio e não pode depender de `std::fs`. O schema
//! mora aqui junto do loader porque é o contrato do arquivo, não regra de
//! negócio.
//!
//! - [`schema`]: [`Config`] tipado, com os padrões do `AppSettings` do Flutter;
//! - [`load`]: caminho padrão, leitura e [`ConfigStore`] (guarda a última
//!   configuração válida);
//! - [`watch`]: [`ConfigWatcher`], polling de mtime numa thread, entregando
//!   [`ConfigEvent`] num `std::sync::mpsc`. Sem async, sem crate de watch.
//!
//! As chaves do TOML são as mesmas do JSON que o Flutter grava: o ponto da
//! chave vira tabela (`chat.font` = `[chat] font`).

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
pub mod load;
mod named;
pub mod schema;
pub mod watch;

pub use error::ConfigError;
pub use load::{default_path, load, ConfigStore};
pub use schema::Config;
pub use watch::{ConfigEvent, ConfigWatcher, POLL_INTERVAL};
