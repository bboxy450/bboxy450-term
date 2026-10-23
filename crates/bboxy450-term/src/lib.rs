//! App process boundaries, IO-owned terminal models and worker-prepared frames.
pub mod cockpit;
pub mod grids;
pub mod hosts;
pub mod keyboard;
pub mod launcher;
pub mod model;

#[cfg(test)]
#[test]
fn keyboard_adapter_uses_neutral_core_and_live_modes() {
    keyboard::assert_adapter();
}
