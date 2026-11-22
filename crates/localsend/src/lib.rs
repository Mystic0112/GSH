//! LocalSend Protocol v2 support for GSH.
//!
//! The crate exposes protocol/domain types separately from networking so the
//! GSH UI can provide an explicit receive decision callback later.

pub(crate) mod filename;
mod http;
pub mod identity;
pub mod network;
pub mod protocol;
pub mod receive;

pub use identity::{Identity, IdentityError};
pub use network::{
    send_files, send_open_files, send_open_files_until, Discovery, NetworkError, Peer,
};
pub use protocol::{Announcement, DeviceInfo, FileMetadata};
pub use receive::{
    Limits, LocalSendServer, ReceiveDecision, Receiver, ServerError, RECEIVE_DECISION_TIMEOUT,
};
