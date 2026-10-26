#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]

// This is the IO owner. Blocking entry points assert they are off the UI thread.
#![allow(clippy::disallowed_methods)]

#[cfg(not(unix))]
compile_error!("bbt-supervisor currently requires Unix PTYs and Unix sockets");

/// Admitted blocking supervisor connections and bounded event notifications.
pub mod client;
mod durability;
/// Versioned process, lease, capture and input-receipt wire types.
pub mod protocol;
/// Retained capture, checkpoints, isolated replay and explicit history gaps.
pub mod ring;
/// Blocking content-addressed storage primitives for retained capture.
pub mod store;
/// Process/PTY ownership, control admission, capture service and numeric diagnostics.
pub mod supervisor;
/// Bounded versioned JSON framing and explicit Unix control endpoints.
pub mod transport;

pub use client::{SocketSupervisor, SupervisorConnection};
pub use protocol::{
    Action, Attachment, Event, Failure, FailureCode, Frame, InputReceipt, InputState, LaunchSpec,
    Message, Reply, Request, SeatInfo, SeatSize,
};
pub use supervisor::{Client, Config, Supervisor};
pub use ulid::Ulid;
