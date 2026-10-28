// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![doc = include_str!("../README.md")]

use serde::{Deserialize, Serialize};
use std::{
    fmt,
    str::FromStr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

/// Owned application intentions shared by native backends and the terminal host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    ShowWindow,
    NewTab,
    SplitRight,
    SplitDown,
    Settings,
    HideWindow,
    CloseWindow,
    HardQuit,
    ToggleFullscreen,
    Minimize,
    Zoom,
}
impl Action {
    pub const ALL: [Self; 11] = [
        Self::ShowWindow,
        Self::NewTab,
        Self::SplitRight,
        Self::SplitDown,
        Self::Settings,
        Self::HideWindow,
        Self::CloseWindow,
        Self::HardQuit,
        Self::ToggleFullscreen,
        Self::Minimize,
        Self::Zoom,
    ];
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ShowWindow => "show_window",
            Self::NewTab => "new_tab",
            Self::SplitRight => "split_right",
            Self::SplitDown => "split_down",
            Self::Settings => "settings",
            Self::HideWindow => "hide_window",
            Self::CloseWindow => "close_window",
            Self::HardQuit => "hard_quit",
            Self::ToggleFullscreen => "toggle_fullscreen",
            Self::Minimize => "minimize",
            Self::Zoom => "zoom",
        }
    }
    pub fn parse(value: &str) -> Result<Self, Error> {
        value.parse()
    }
}
impl FromStr for Action {
    type Err = Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|a| a.as_str() == value)
            .ok_or_else(|| Error::InvalidAction(value.into()))
    }
}

/// Menu entries distinguish host actions from the operating system responder chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuCommand {
    Action(Action),
    Responder(&'static str),
    Separator,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem {
    pub title: &'static str,
    pub command: MenuCommand,
    pub key: &'static str,
    pub option: bool,
    pub shift: bool,
    pub control: bool,
}
impl MenuItem {
    const fn action(title: &'static str, action: Action, key: &'static str) -> Self {
        Self {
            title,
            command: MenuCommand::Action(action),
            key,
            option: false,
            shift: false,
            control: false,
        }
    }
    const fn responder(title: &'static str, selector: &'static str, key: &'static str) -> Self {
        Self {
            title,
            command: MenuCommand::Responder(selector),
            key,
            option: false,
            shift: false,
            control: false,
        }
    }
    const fn separator() -> Self {
        Self {
            title: "",
            command: MenuCommand::Separator,
            key: "",
            option: false,
            shift: false,
            control: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuGroup {
    pub title: &'static str,
    pub items: Vec<MenuItem>,
}

/// The host and native adapter consume this same menu model.
pub fn app_menu() -> Vec<MenuGroup> {
    use Action::*;
    vec![
        MenuGroup {
            title: "bboxy450-term",
            items: vec![
                MenuItem::action("Settings…", Settings, ","),
                MenuItem::separator(),
                MenuItem::action("Hide bboxy450-term", HideWindow, "h"),
                MenuItem::action("Quit bboxy450-term (Keep Sessions)", CloseWindow, "q"),
                MenuItem {
                    option: true,
                    ..MenuItem::action("Quit and Stop All Sessions", HardQuit, "q")
                },
            ],
        },
        MenuGroup {
            title: "Edit",
            items: vec![
                MenuItem::responder("Undo", "undo:", "z"),
                MenuItem {
                    shift: true,
                    ..MenuItem::responder("Redo", "redo:", "z")
                },
                MenuItem::separator(),
                MenuItem::responder("Cut", "cut:", "x"),
                MenuItem::responder("Copy", "copy:", "c"),
                MenuItem::responder("Paste", "paste:", "v"),
                MenuItem::responder("Select All", "selectAll:", "a"),
            ],
        },
        MenuGroup {
            title: "View",
            items: vec![MenuItem {
                control: true,
                ..MenuItem::action("Toggle Full Screen", ToggleFullscreen, "f")
            }],
        },
        MenuGroup {
            title: "Window",
            items: vec![
                MenuItem::action("New Tab", NewTab, "t"),
                MenuItem::action("Split Right", SplitRight, "d"),
                MenuItem {
                    shift: true,
                    ..MenuItem::action("Split Down", SplitDown, "d")
                },
                MenuItem::separator(),
                MenuItem::action("Minimize", Minimize, "m"),
                MenuItem::action("Zoom", Zoom, ""),
                MenuItem::action("Show Terminal", ShowWindow, ""),
            ],
        },
    ]
}
pub fn tray_menu() -> Vec<MenuItem> {
    vec![
        MenuItem::action("Show bboxy450-term", Action::ShowWindow, ""),
        MenuItem::action("New Tab", Action::NewTab, ""),
        MenuItem::action("Settings…", Action::Settings, ""),
        MenuItem::separator(),
        MenuItem::action("Quit and Stop All Sessions", Action::HardQuit, ""),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Unsupported,
    MainThreadRequired,
    Native(String),
    InvalidAction(String),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported => f.write_str("desktop integration is unsupported on this platform"),
            Self::MainThreadRequired => {
                f.write_str("native desktop integration requires the main thread")
            }
            Self::Native(reason) => write!(f, "native desktop integration: {reason}"),
            Self::InvalidAction(action) => write!(f, "unknown desktop action: {action}"),
        }
    }
}
impl std::error::Error for Error {}

pub type Callback = Box<dyn Fn(Action) + Send + Sync + 'static>;

/// Wake the daemon's main loop after its asynchronous worker has shut down.
#[derive(Clone)]
pub struct StopHandle {
    state: Arc<AtomicBool>,
    wake: Option<fn()>,
    native_error: Option<Error>,
}
impl StopHandle {
    pub fn stop(&self) {
        if self.state.swap(true, Ordering::AcqRel) {
            return;
        }
        if let Some(wake) = self.wake {
            wake();
        }
    }
    pub fn native_error(&self) -> Option<&Error> {
        self.native_error.as_ref()
    }
    /// Construct a backend stop port. `wake` must only enqueue a nonblocking
    /// event onto the backend's native main loop; it is called at most once.
    pub fn new(wake: Option<fn()>, native_error: Option<Error>) -> Self {
        Self {
            state: Arc::new(AtomicBool::new(false)),
            wake,
            native_error,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitlebarAction {
    Fill,
    Zoom,
    Minimize,
    None,
}

/// Run an IO worker even when no native desktop backend is available.
pub fn run_without_native(
    worker: impl FnOnce(StopHandle) + Send + 'static,
    error: Error,
) -> Result<(), Error> {
    let stop = StopHandle::new(None, Some(error.clone()));
    let handle = std::thread::spawn({
        let stop = stop.clone();
        move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| worker(stop.clone())));
            stop.stop();
            result
        }
    });
    handle
        .join()
        .map_err(|_| Error::Native("daemon worker panicked".into()))?
        .map_err(|_| Error::Native("daemon worker panicked".into()))?;
    Err(error)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn action_names_round_trip_and_menus_preserve_session_semantics() {
        for action in Action::ALL {
            assert_eq!(Action::parse(action.as_str()), Ok(action));
            assert_eq!(
                serde_json::to_string(&action).unwrap(),
                format!("\"{}\"", action.as_str())
            );
            assert_eq!(
                serde_json::from_str::<Action>(&format!("\"{}\"", action.as_str())).unwrap(),
                action
            );
        }
        assert!(Action::parse("terminate").is_err());
        let menu = app_menu();
        let items: Vec<_> = menu.iter().flat_map(|group| &group.items).collect();
        let quit = items
            .iter()
            .find(|item| item.key == "q" && !item.option)
            .unwrap();
        assert_eq!(quit.command, MenuCommand::Action(Action::CloseWindow));
        let hard = items
            .iter()
            .find(|item| item.command == MenuCommand::Action(Action::HardQuit))
            .unwrap();
        assert!(hard.option && hard.key == "q" && !hard.shift);
        assert!(
            tray_menu()
                .iter()
                .any(|item| item.command == MenuCommand::Action(Action::ShowWindow))
        );
        for selector in ["cut:", "copy:", "paste:", "selectAll:"] {
            assert!(
                items
                    .iter()
                    .any(|item| item.command == MenuCommand::Responder(selector))
            );
        }
    }
}
