// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Shared terminal viewport and GUI wire schema. No transport or renderer.
pub mod events;
pub mod gui;
pub mod hosts;
pub mod identity;
pub mod messaging;
pub mod registry;
pub mod seat_names;
pub mod seat_state;
pub mod streams;
pub use bbt_ui_model::{SeatId, SwarmId, TabId};
pub use gui::{GuiAction, GuiIdentity, GuiIdentityState, GuiReply, GuiRequest, Row};
pub use hosts::{GuiHost, HostState};
pub use registry::SeatRecord;
pub use seat_state::{
    Activity, Grade, GuiSeat, Lifecycle, NotificationEpisode, ProcessStatus, Role, RoleClass,
    SeatState, SpawnSpec, SubagentRow, TransportSpec, WaitKind, Where,
};
pub use streams::{Delta, Screen, Snapshot, Update, View};
