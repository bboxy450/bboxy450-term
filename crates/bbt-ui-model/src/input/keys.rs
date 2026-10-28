// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::Direction;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub command: bool,
    pub alt: bool,
    pub control: bool,
    pub shift: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusContext {
    Terminal,
    Chrome,
    TextField,
    Launcher,
    Overlay,
}

/// Only navigation needs normalisation here. The app keeps the original key
/// and text for terminal encoding or a control's own editing/selection logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationKey {
    Arrow(Direction),
    Tab,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Navigation {
    Direction(Direction),
    Cycle { backwards: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRoute {
    Terminal,
    Pane(Navigation),
    /// The focused field, launcher or overlay handles this key.
    Local,
    /// App chrome or an otherwise unhandled command chord.
    App,
}

/// Pane navigation and terminal pass-through, after the app's global shortcuts
/// (launcher, settings, quit, etc.). Launcher/modal-local keys keep priority;
/// text fields retain plain editing keys but permit explicit pane navigation.
/// This function neither encodes PTY bytes nor changes focus or launches work.
pub fn route_navigation(key: NavigationKey, mods: Modifiers, context: FocusContext) -> KeyRoute {
    if matches!(context, FocusContext::Launcher | FocusContext::Overlay) {
        return KeyRoute::Local;
    }
    let explicit = mods.command && mods.alt && !mods.control;
    let plain = !mods.command && !mods.alt && !mods.control;
    if explicit || (context == FocusContext::Chrome && plain) {
        match key {
            NavigationKey::Tab => {
                return KeyRoute::Pane(Navigation::Cycle {
                    backwards: mods.shift,
                });
            }
            NavigationKey::Arrow(direction) if !mods.shift => {
                return KeyRoute::Pane(Navigation::Direction(direction));
            }
            _ => {}
        }
    }
    match context {
        FocusContext::Terminal if !mods.command => KeyRoute::Terminal,
        FocusContext::TextField if !mods.command => KeyRoute::Local,
        _ => KeyRoute::App,
    }
}
