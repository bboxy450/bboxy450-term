// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! CEF glue: [`settings`] and [`app`] apply [`crate::defaults`]; [`send`] and
//! [`observer`] carry [`crate::cdp`] bytes to and from a browser through
//! CEF's in-process DevTools channel (no port).
use crate::cdp::Request;
use crate::defaults::{Defaults, SWITCHES, fixed};
pub use cef;
use cef::*;
use std::sync::mpsc::SyncSender;

/// CEF settings with our defaults applied.
pub fn settings(defaults: &Defaults) -> Settings {
    let path = |p: std::path::PathBuf| CefString::from(p.to_string_lossy().as_ref());
    Settings {
        windowless_rendering_enabled: fixed::WINDOWLESS.into(),
        command_line_args_disabled: fixed::COMMAND_LINE_ARGS_DISABLED.into(),
        persist_session_cookies: fixed::PERSIST_SESSION_COOKIES.into(),
        remote_debugging_port: fixed::REMOTE_DEBUGGING_PORT,
        root_cache_path: path(defaults.root.clone()),
        cache_path: path(defaults.profile()),
        log_file: path(defaults.log_file()),
        log_severity: LogSeverity::WARNING,
        ..Default::default()
    }
}

wrap_app! {
    struct DefaultsApp;

    impl App {
        fn on_before_command_line_processing(
            &self,
            _process_type: Option<&CefString>,
            command_line: Option<&mut CommandLine>,
        ) {
            let Some(command_line) = command_line else { return };
            for (name, value) in SWITCHES {
                let name = CefString::from(*name);
                match value {
                    Some(value) => command_line.append_switch_with_value(Some(&name), Some(&CefString::from(*value))),
                    None => command_line.append_switch(Some(&name)),
                }
            }
        }
    }
}

/// The CEF application object for every process: it appends [`SWITCHES`].
pub fn app() -> App {
    DefaultsApp::new()
}

/// Hand one encoded CDP request to `host`. False when CEF refused it.
pub fn send(host: &BrowserHost, request: &Request) -> bool {
    host.send_dev_tools_message(Some(&request.bytes)) != 0
}

wrap_dev_tools_message_observer! {
    struct Forward {
        to: SyncSender<Vec<u8>>,
    }

    impl DevToolsMessageObserver {
        fn on_dev_tools_message(&self, _browser: Option<&mut Browser>, message: Option<&[u8]>) -> ::std::os::raw::c_int {
            if let Some(message) = message {
                let _ = self.to.send(message.to_vec());
            }
            // Handled: CEF does not split it into result/event callbacks.
            1
        }
    }
}

/// An observer that forwards every raw CDP message to `to`, for
/// [`crate::cdp::Client::receive`]. Register it with
/// `host.add_dev_tools_message_observer`; dropping the registration stops it.
pub fn observer(to: SyncSender<Vec<u8>>) -> DevToolsMessageObserver {
    Forward::new(to)
}

/// The CEF and Chromium build linked in, e.g.
/// `156.0.31+ge603d97+chromium-156.0.8078.100`.
pub fn version() -> &'static str {
    std::ffi::CStr::from_bytes_with_nul(sys::CEF_VERSION)
        .ok()
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_names_cef_and_chromium() {
        let v = super::version();
        assert!(v.starts_with("156.") && v.contains("+chromium-156."), "{v}");
    }
}
