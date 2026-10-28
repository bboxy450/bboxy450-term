// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Unattended native-object/action probe; no desktop input or operator identity.
#[cfg(target_os = "macos")]
#[allow(
    clippy::disallowed_methods,
    reason = "single-threaded native fixture collector; no operator or rendered UI state"
)]
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
    window_probe();
    run_daemon(Box::new(|_| {}), |stop| {
        assert!(stop.native_error().is_none());
        stop.stop();
    })
    .unwrap();
    println!(
        "native menu11/tray4 callbacks, borderless Fill/Zoom/Minimize/Show, disposal and runloop stop: pass"
    );
}
#[cfg(target_os = "macos")]
fn window_probe() {
    use bboxy450_term_desktop::{
        Action, TitlebarAction, apply_titlebar_action, apply_window_action,
    };
    use objc2::{MainThreadOnly, rc::Retained};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSView, NSWindow,
        NSWindowStyleMask,
    };
    use objc2_foundation::{MainThreadMarker, NSDate, NSPoint, NSRect, NSRunLoop, NSSize};
    use raw_window_handle::{
        AppKitWindowHandle, HandleError, HasWindowHandle, RawWindowHandle, WindowHandle,
    };
    use std::{
        ptr::NonNull,
        time::{Duration, Instant},
    };
    struct Fixture {
        view: Retained<NSView>,
        _window: Retained<NSWindow>,
    }
    impl HasWindowHandle for Fixture {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let handle = AppKitWindowHandle::new(NonNull::from(&*self.view).cast());
            // SAFETY: this fixture owns both NSView and its NSWindow for the borrow.
            Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AppKit(handle)) })
        }
    }
    fn close(a: NSRect, b: NSRect) -> bool {
        (a.origin.x - b.origin.x).abs() < 1.
            && (a.origin.y - b.origin.y).abs() < 1.
            && (a.size.width - b.size.width).abs() < 1.
            && (a.size.height - b.size.height).abs() < 1.
    }
    fn until(mut predicate: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !predicate() {
            assert!(
                Instant::now() < deadline,
                "native window state did not settle"
            );
            NSRunLoop::currentRunLoop().runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(0.01));
        }
    }
    let mtm = MainThreadMarker::new().unwrap();
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.finishLaunching();
    let original = NSRect::new(NSPoint::new(100., 100.), NSSize::new(320., 180.));
    // SAFETY: a private borderless, resizable test window; retain ownership is
    // explicit and close must not release that ownership behind Rust's back.
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            original,
            NSWindowStyleMask::Borderless
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::Miniaturizable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe {
        window.setReleasedWhenClosed(false);
    }
    let view = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0., 0.), original.size),
    );
    window.setContentView(Some(&view));
    let fixture = Fixture {
        view,
        _window: window.clone(),
    };
    assert!(!window.styleMask().contains(NSWindowStyleMask::Titled));
    let original = window.frame();
    apply_titlebar_action(&fixture, TitlebarAction::Fill).unwrap();
    assert!(close(
        window.frame(),
        window.screen().unwrap().visibleFrame()
    ));
    apply_titlebar_action(&fixture, TitlebarAction::Fill).unwrap();
    assert!(close(window.frame(), original));
    apply_titlebar_action(&fixture, TitlebarAction::None).unwrap();
    assert!(close(window.frame(), original));
    apply_window_action(&fixture, Action::ShowWindow).unwrap();
    until(|| window.isVisible());
    apply_window_action(&fixture, Action::Zoom).unwrap();
    until(|| !close(window.frame(), original));
    apply_window_action(&fixture, Action::Zoom).unwrap();
    until(|| close(window.frame(), original));
    apply_window_action(&fixture, Action::Minimize).unwrap();
    until(|| window.isMiniaturized());
    apply_window_action(&fixture, Action::ShowWindow).unwrap();
    until(|| !window.isMiniaturized() && window.isVisible());
    window.close();
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("native desktop probe requires macOS");
    std::process::exit(2);
}
