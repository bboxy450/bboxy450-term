// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Unattended native-object/action probe; no desktop input or operator identity.
#[cfg(target_os = "macos")]
fn main() {
    use bboxy450_term_desktop::{Action, MenuBar, Tray, run_daemon};
    use std::sync::{Arc, Mutex};
    let actions = Arc::new(Mutex::new(Vec::new()));
    let bar = MenuBar::install({
        let actions = actions.clone();
        Box::new(move |a| actions.lock().unwrap().push(a))
    })
    .unwrap();
    for action in Action::ALL {
        bar.activate(action).unwrap();
    }
    assert_eq!(*actions.lock().unwrap(), Action::ALL);
    actions.lock().unwrap().clear();
    let tray = Tray::install({
        let actions = actions.clone();
        Box::new(move |a| actions.lock().unwrap().push(a))
    })
    .unwrap();
    for action in [
        Action::ShowWindow,
        Action::NewTab,
        Action::Settings,
        Action::HardQuit,
    ] {
        tray.activate(action).unwrap();
    }
    assert_eq!(
        *actions.lock().unwrap(),
        [
            Action::ShowWindow,
            Action::NewTab,
            Action::Settings,
            Action::HardQuit
        ]
    );
    drop(tray);
    drop(bar);
    run_daemon(Box::new(|_| {}), |stop| {
        assert!(stop.native_error().is_none());
        stop.stop();
    })
    .unwrap();
    println!("native menu11/tray4 callbacks, disposal and runloop stop: pass");
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("native desktop probe requires macOS");
    std::process::exit(2);
}
