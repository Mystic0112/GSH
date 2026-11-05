//! Sockets do app: o servidor que recebe o status de turno do `gsh hook` (e, no
//! mesmo socket, os comandos da CLI `gsh`).
//!
//! Fica fora de `gsh-core` porque é I/O (socket, arquivo, threads); o core só
//! decodifica a linha ([`gsh_core::status`]). Só stdlib: uma thread aceita e
//! uma thread curta por conexão, entregando [`Delivery`] num
//! `std::sync::mpsc`. Sem async: o volume é um hook por chamada de ferramenta.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

mod server;

pub use server::{
    gsh_dir_from_env, Delivery, Endpoint, ServerConfig, ServerError, StatusServer, Transport,
};
