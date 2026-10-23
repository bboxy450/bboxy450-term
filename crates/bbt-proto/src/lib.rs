//! Shared terminal viewport and GUI wire schema. No transport or renderer.
pub mod events;
pub mod gui;
pub mod hosts;
pub mod messaging;
pub mod registry;
pub mod seat_state;
pub mod streams;
pub use bbt_ui_model::{SeatId, SwarmId, TabId};
pub use gui::{GuiAction, GuiIdentity, GuiIdentityState, GuiReply, GuiRequest, Row};
pub use hosts::{GuiHost, HostState};
pub use registry::SeatRecord;
pub use seat_state::{
    Activity, Grade, GuiSeat, Lifecycle, ProcessStatus, Role, RoleClass, SeatState, SpawnSpec,
    SubagentRow, WaitKind, Where,
};
pub use streams::{Delta, Screen, Snapshot, Update, View};
