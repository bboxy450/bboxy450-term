// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
mod attention_observer;
pub mod bus;
mod checkpoint;
pub mod ctl_door;
pub mod debug;
mod gui_capture;
mod gui_events;
mod history_fault;
pub mod identity;
pub mod index;
pub mod knowledge;
pub mod logic;
pub mod mcp;
pub mod messaging;
pub mod plugins;
pub mod provenance;
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

mod window_journal;
