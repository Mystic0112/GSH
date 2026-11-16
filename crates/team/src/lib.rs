//! Time na LAN: a fazenda dos colegas (multicast UDP) e o correio entre agentes (TCP),
//! compativeis com o GSH Flutter (protocolo v2 do `team_crypto.dart`).
//!
//! Camadas: `config`, `cidr`, `crypto`, `packet` e `knock` sao puras (so bytes e valores);
//! `channel`, `mail`, `network` e `node` tocam socket. Sem async: uma thread por socket e
//! `std::sync::mpsc`, como o `gsh-ipc`.

#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing
    )
)]
#![deny(unsafe_code)]

mod channel;
mod cidr;
mod config;
mod crypto;
mod error;
mod knock;
mod mail;
mod network;
mod node;
mod packet;

pub use channel::{Channel, GROUP, PORT};
pub use cidr::ipv4_in_cidr;
pub use config::TeamConfig;
pub use crypto::{derive_key, open, seal, TeamKey};
pub use error::TeamError;
pub use knock::{decode_knock, encode_knock, Knock, KnockKind};
pub use mail::{Mail, SendError};
pub use network::allowed_network;
pub use node::{outgoing, Node, Peer, PEER_TIMEOUT, PUBLISH_EVERY};
pub use packet::{
    decode_packet, encode_packet, packet_body, Activity, FarmLoop, Packet, TeamAgent, MAX_LOOPS,
};

#[cfg(test)]
mod golden_tests;
