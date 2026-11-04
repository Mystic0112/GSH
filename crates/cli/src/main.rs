//! `gsh`: CLI interna do GSH, e o helper de hook dos harnesses.
//!
//! Fica visível só dentro dos terminais que o app spawna e fala com ele por um
//! socket (`GSH_STATUS_SOCK` no POSIX; `GSH_STATUS_PORT` + `GSH_STATUS_TOKEN` no
//! Windows). A unidade que a CLI endereça é uma **tab** (sessão de
//! terminal/agente); `GSH_PANE_ID` e `list-panes` ficam como aliases legados.
//!
//! Saída em inglês e estável: quem lê é agente e script.
//!
//! Divisão: `gsh-core` tem a lógica de protocolo sem I/O (montar requisição,
//! ler resposta, status do hook, formatar listagem); este crate tem o parse de
//! argumentos, o socket, o stdin e os arquivos.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]

mod args;
mod commands;
mod env;
mod error;
mod hook;
mod transport;

use std::io::Write;
use std::process::ExitCode;

fn main() -> ExitCode {
    // Lossy: um argumento fora de UTF-8 não pode derrubar a CLI com panic.
    let argv: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();

    match args::parse(&argv).and_then(commands::run) {
        Ok(out) => {
            if let Some(out) = out {
                // Pipe fechado pelo leitor não é erro nosso.
                let _ = std::io::stdout().write_all(out.as_bytes());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(e.exit_code())
        }
    }
}
