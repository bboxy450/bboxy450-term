//! Restartable logic above the local supervisor process boundary.
//!
//! Logic owns durable objects, the bus, repository index, harness adapters,
//! messaging and viewport streams. `bbt-supervisor` owns PTYs, leases and ring
//! capture without Tokio; its existing public paths remain re-exported here.

// This is the IO owner. Blocking entry points assert they are off the UI thread.
#![allow(clippy::disallowed_methods)]

#[cfg(not(unix))]
compile_error!("bbt-mux currently requires Unix PTYs and Unix sockets");

pub mod adapters;
pub mod bus;
pub mod identity;
pub mod index;
pub mod logic;
pub mod mcp;
pub mod messaging;
pub mod provider;
pub mod registry;
pub mod seat_socket;
pub mod store;
pub mod streams;

// Preserve the existing client paths while the supervisor is a lower crate.
pub use bbt_supervisor::{protocol, ring, supervisor, transport};

pub use protocol::{
    Action, Attachment, Event, Failure, FailureCode, Frame, InputReceipt, InputState, LaunchSpec,
    Message, Reply, Request, SeatInfo, SeatSize,
};
pub use supervisor::{Client, Config, Supervisor};
pub use ulid::Ulid;
