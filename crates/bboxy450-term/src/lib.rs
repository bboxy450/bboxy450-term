// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! App process boundaries, IO-owned terminal models and worker-prepared frames.
pub mod attention;
pub mod close;
pub mod composer_images;
pub mod close_frame;
pub mod cockpit;
pub mod ctl;
pub mod documents;
pub mod e2e_night;
pub mod focus_ack;
pub mod frame_observation;
pub mod grids;
pub mod hosts;
pub mod keyboard;
pub mod knowledge;
pub mod launcher;
pub mod local_time;
pub mod messaging;
pub mod model;
pub mod notifications;
pub mod scrollback;
pub mod terminal_selection;
pub mod settings;
pub mod spawn_params;

#[cfg(test)]
// keeps: invariant kitty-keyboard
#[test]
fn keyboard_adapter_uses_neutral_core_and_live_modes() {
    keyboard::assert_adapter();
}

pub mod starting;

pub mod window_state;

pub mod window_writes;
