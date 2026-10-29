// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![doc = include_str!("../README.md")]

pub use bboxy450_term_desktop_core::*;

#[cfg(target_os = "macos")]
pub use bboxy450_term_desktop_macos::{
    MenuBar, Tray, apply_titlebar_action, apply_window_action, double_click_interval, run_daemon,
    titlebar_action, read_clipboard, clipboard_image_png,
};

#[cfg(not(target_os = "macos"))]
mod unsupported {
    use super::*;
    use raw_window_handle::HasWindowHandle;
    use std::time::Duration;
    pub fn read_clipboard() -> Result<Option<ClipboardContent>, Error> { Err(Error::Unsupported) }
    pub fn clipboard_image_png(_bytes: &[u8], _format: ClipboardImageFormat) -> Result<Vec<u8>, Error> { Err(Error::Unsupported) }
    pub struct MenuBar;
    pub struct Tray;
    impl MenuBar {
        pub fn current_key_scancode() -> Option<u32> {
            None
        }
        pub fn install(_callback: Callback) -> Result<Self, Error> {
            Err(Error::Unsupported)
        }
        pub fn activate(&self, _action: Action) -> Result<(), Error> {
            Err(Error::Unsupported)
        }
    }
    impl Tray {
        pub fn install(_callback: Callback) -> Result<Self, Error> {
            Err(Error::Unsupported)
        }
        pub fn activate(&self, _action: Action) -> Result<(), Error> {
            Err(Error::Unsupported)
        }
    }
    pub fn run_daemon(
        callback: Callback,
        worker: impl FnOnce(StopHandle) + Send + 'static,
    ) -> Result<(), Error> {
        let _ = callback;
        run_without_native(worker, Error::Unsupported)
    }
    pub fn titlebar_action() -> TitlebarAction {
        TitlebarAction::Zoom
    }
    pub fn double_click_interval() -> Duration {
        Duration::from_millis(500)
    }
    pub fn apply_window_action(window: &impl HasWindowHandle, action: Action) -> Result<(), Error> {
        let _ = (window, action);
        Err(Error::Unsupported)
    }
    pub fn apply_titlebar_action(
        window: &impl HasWindowHandle,
        action: TitlebarAction,
    ) -> Result<(), Error> {
        let _ = (window, action);
        Err(Error::Unsupported)
    }
}
#[cfg(not(target_os = "macos"))]
pub use unsupported::*;
