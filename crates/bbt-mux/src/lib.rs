//! Local PTY ownership and the supervisor/logic boundary.
//!
//! One supervisor thread polls every PTY. Both the in-process client and the
//! optional Unix socket use the same requests, generation checks and leases.
//! A lease permits a PTY write; composer readiness belongs to the later harness
//! adapters. Restartable logic adds durable object stores, an event bus, a
//! repository index and viewport checkpoints; W1.2 supplies the byte ring.

// This is the IO owner. Blocking entry points assert they are off the UI thread.
#![allow(clippy::disallowed_methods)]

#[cfg(not(unix))]
compile_error!("bbt-mux currently requires Unix PTYs and Unix sockets");

pub mod adapters;
pub mod bus;
pub mod index;
pub mod logic;
pub mod messaging;
pub mod protocol;
pub mod registry;
pub mod ring;
pub mod seat_socket;
pub mod store;
pub mod streams;
pub mod supervisor;
pub mod transport;

pub use protocol::{
    Action, Attachment, Event, Failure, FailureCode, Frame, InputReceipt, InputState, LaunchSpec,
    Message, Reply, Request, SeatInfo, SeatSize,
};
pub use supervisor::{Client, Config, Supervisor};
pub use ulid::Ulid;
