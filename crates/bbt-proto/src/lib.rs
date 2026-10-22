//! Shared terminal viewport and GUI wire schema. No transport or renderer.
pub mod registry;
pub mod streams;
pub use bbt_ui_model::{SeatId, SwarmId, TabId};
pub use registry::SeatRecord;
pub use streams::{Delta, Screen, Snapshot, Update, View};
