//! App process boundaries, IO-owned terminal models and worker-prepared frames.
pub mod attention;
pub mod close;
pub mod close_frame;
pub mod cockpit;
pub mod ctl;
pub mod documents;
pub mod focus_ack;
pub mod e2e_night;
pub mod grids;
pub mod hosts;
pub mod keyboard;
pub mod knowledge;
pub mod launcher;
pub mod messaging;
pub mod model;
pub mod notifications;
pub mod scrollback;
pub mod settings;

#[cfg(test)]
// keeps: invariant kitty-keyboard
#[test]
fn keyboard_adapter_uses_neutral_core_and_live_modes() {
    keyboard::assert_adapter();
}
