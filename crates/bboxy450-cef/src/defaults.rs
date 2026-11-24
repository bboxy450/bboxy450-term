// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The one place bboxy450's Chromium defaults live.
//!
//! Every browser we start is private, offscreen and prompt-free: its profile
//! sits under a root the caller owns, it never reads the login Keychain, it
//! listens on no port, and the process's own command line cannot change any
//! of this.
use std::path::{Path, PathBuf};

/// Chromium switches appended to every process's command line, as
/// `(name, value)`. Each one answers a failure we refuse.
pub const SWITCHES: &[(&str, Option<&str>)] = &[
    // The login Keychain prompts the operator (Chrome Safe Storage); a test
    // that prompts is a design bug. Cookies stay in the private profile.
    ("use-mock-keychain", None),
    ("password-store", Some("basic")),
    // No first-run UI, no default-browser nag.
    ("no-first-run", None),
    ("no-default-browser-check", None),
    // Nothing leaves the machine on Chromium's own initiative.
    ("disable-background-networking", None),
    ("disable-component-update", None),
    ("disable-sync", None),
    ("metrics-recording-only", None),
];

/// What a caller chooses; everything else is fixed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defaults {
    /// Private root for every profile, cache and log of this engine.
    pub root: PathBuf,
}

impl Defaults {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
    /// Profile directory: cookies, storage and cache. Always under [`Self::root`].
    pub fn profile(&self) -> PathBuf {
        self.root.join("profile")
    }
    /// Chromium's log file. Always under [`Self::root`].
    pub fn log_file(&self) -> PathBuf {
        self.root.join("chromium.log")
    }
    /// True when `path` is inside this engine's private root.
    pub fn owns(&self, path: &Path) -> bool {
        path.starts_with(&self.root)
    }
}

/// Fixed engine settings, named so a reader and a test can see them.
pub mod fixed {
    /// Pages render offscreen into frames we composite; there is no Chromium window.
    pub const WINDOWLESS: bool = true;
    /// CDP travels inside the process; no TCP port is ever opened.
    pub const REMOTE_DEBUGGING_PORT: i32 = 0;
    /// The host process's argv cannot add Chromium switches.
    pub const COMMAND_LINE_ARGS_DISABLED: bool = true;
    /// Session cookies die with the browser.
    pub const PERSIST_SESSION_COOKIES: bool = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_keep_the_keychain_and_network_quiet() {
        let has = |name: &str, value: Option<&str>| SWITCHES.contains(&(name, value));
        assert!(has("use-mock-keychain", None));
        assert!(has("password-store", Some("basic")));
        assert!(has("disable-background-networking", None));
        assert!(!SWITCHES.iter().any(|(n, _)| n.starts_with("remote-debugging")));
        let mut names: Vec<_> = SWITCHES.iter().map(|(n, _)| *n).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SWITCHES.len(), "one entry per switch");
    }

    #[test]
    fn every_path_stays_under_the_root() {
        let d = Defaults::new("/tmp/bboxy-cef-root");
        assert!(d.owns(&d.profile()));
        assert!(d.owns(&d.log_file()));
        assert!(!d.owns(Path::new("/Users/someone/Library/Application Support/Google/Chrome")));
        assert_eq!(fixed::REMOTE_DEBUGGING_PORT, 0);
        const { assert!(fixed::WINDOWLESS && fixed::COMMAND_LINE_ARGS_DISABLED && !fixed::PERSIST_SESSION_COOKIES) };
    }
}
