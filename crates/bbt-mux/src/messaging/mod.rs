//! Local signed messaging, durable routing and serialized harness delivery.
//! Local issuer material is distinct from the deferred OpenPGP account chain.

pub mod api;
pub mod keys;
pub mod ledger;
pub mod service;
pub mod tree;
pub mod wire;

pub use service::Messaging;
pub use wire::{Envelope, MessageKind, SendRequest};
