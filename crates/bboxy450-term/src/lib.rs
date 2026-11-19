// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! App process boundaries, IO-owned terminal models and worker-prepared frames.
#[cfg(any(feature = "desktop", target_os = "linux"))]
pub mod attention;
#[cfg(feature = "desktop")]
pub mod canvas_host;
pub mod chat_scroll;
#[cfg(feature = "desktop")]
pub mod close;
#[cfg(feature = "desktop")]
pub mod close_frame;
pub mod cockpit;
#[cfg(feature = "desktop")]
pub mod composer_images;
pub mod ctl;
#[cfg(feature = "desktop")]
pub mod documents;
#[cfg(feature = "desktop")]
pub mod e2e_night;
#[cfg(feature = "desktop")]
pub mod focus_ack;
#[cfg(feature = "desktop")]
pub mod frame_observation;
#[cfg(feature = "desktop")]
pub mod grids;
#[cfg(feature = "desktop")]
pub mod hosts;
#[cfg(feature = "desktop")]
pub mod keyboard;
pub mod knowledge;
#[cfg(feature = "desktop")]
pub mod launcher;
pub mod local_time;
#[cfg(feature = "desktop")]
pub mod messaging;
#[cfg(feature = "desktop")]
pub mod model;
#[cfg(any(feature = "desktop", target_os = "linux"))]
pub mod notifications;
pub mod scrollback;
#[cfg(feature = "desktop")]
pub mod settings;
#[cfg(feature = "desktop")]
pub mod spawn_params;
#[cfg(feature = "desktop")]
pub mod terminal_selection;

#[cfg(all(test, feature = "desktop"))]
// keeps: invariant kitty-keyboard
#[test]
fn keyboard_adapter_uses_neutral_core_and_live_modes() {
    keyboard::assert_adapter();
}

#[cfg(feature = "desktop")]
pub mod starting;

#[cfg(feature = "desktop")]
pub mod window_state;

#[cfg(feature = "desktop")]
pub mod window_writes;

#[cfg(feature = "desktop")]
pub mod rich_pane;

pub mod gui_build;
