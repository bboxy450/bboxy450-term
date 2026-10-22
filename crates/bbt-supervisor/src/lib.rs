//! Local PTY ownership and the supervisor side of the process boundary.
//!
//! One mio actor owns every master, child and writer lease; one disk writer
//! captures rings. Clients share the same bounded protocol over in-process
//! channels or a private Unix socket. No Tokio runtime belongs in this crate.

// This is the IO owner. Blocking entry points assert they are off the UI thread.
#![allow(clippy::disallowed_methods)]

#[cfg(not(unix))]
compile_error!("bbt-supervisor currently requires Unix PTYs and Unix sockets");

pub mod client;
pub mod protocol;
pub mod ring;
pub mod store;
pub mod supervisor;
pub mod transport;

pub use client::{SocketSupervisor, SupervisorConnection};
pub use protocol::{
    Action, Attachment, Event, Failure, FailureCode, Frame, InputReceipt, InputState, LaunchSpec,
    Message, Reply, Request, SeatInfo, SeatSize,
};
pub use supervisor::{Client, Config, Supervisor};
pub use ulid::Ulid;
