//! # Albedo Core Engine - ace_ipc
//!
//! Barramento de comunicação in-process (IPC) fortemente tipado, lock-free
//! e de altíssimo rendimento para coordenação entre threads e subsistemas do Albedo.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod channel;
pub mod error;
pub mod protocol;
pub mod router;

pub use channel::{bounded_channel, unbounded_channel, IpcReceiver, IpcSender};
pub use error::IpcError;
pub use protocol::{
    DomMutationPayload, DomMutationType, InputEventPayload, IpcMessage, RenderFramePayload,
    ResourceDataPayload, ResourceRequestPayload,
};
pub use router::{IpcRouter, Topic};

/// Inicialização do subsistema de IPC.
pub fn init() {}
