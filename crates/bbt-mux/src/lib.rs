//! Local PTY ownership and the supervisor/logic boundary.
//!
//! One supervisor thread polls every PTY. Both the in-process client and the
//! optional Unix socket use the same requests, generation checks and leases.
//! A lease permits a PTY write; composer readiness belongs to the later harness
//! adapters. There is no byte ring or crash persistence in this first layer.

#[cfg(not(unix))]
compile_error!("bbt-mux currently requires Unix PTYs and Unix sockets");

pub mod protocol;
pub mod supervisor;
pub mod transport;

pub use protocol::{
    Action, Attachment, Event, Failure, FailureCode, Frame, InputReceipt, InputState, LaunchSpec,
    Message, Reply, Request, SeatInfo, SeatSize,
};
pub use supervisor::{Client, Config, Supervisor};
pub use ulid::Ulid;
