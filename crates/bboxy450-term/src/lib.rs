//! App process boundaries, IO-owned terminal models and worker-prepared frames.
pub mod cockpit;
pub mod grids;
pub mod hosts;
pub mod keyboard;
pub mod launcher;
pub mod model;

#[cfg(test)]
#[test]
fn kitty_keyboard_invariant() {
    keyboard::assert_invariant();
}
