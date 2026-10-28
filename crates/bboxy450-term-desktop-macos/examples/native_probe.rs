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
    reason = "private fixture collector is main-thread-only; the verdict wait runs on the daemon worker, never its native main loop"
)]
fn main() {
    use bboxy450_term_desktop_core::Action;
    use bboxy450_term_desktop_macos::{MenuBar, Tray, run_daemon};
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
        let (sent, received) = std::sync::mpsc::sync_channel(1);
        native::start(sent, stop.clone());
        let passed = received
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap_or(false);
        // Also release the main loop on a failed/missing fixture callback.
        stop.stop();
        assert!(passed, "native window commands failed");
    })
    .unwrap();
    println!(
        "native menu12/tray4 callbacks, borderless Fill/Zoom/Minimize/Show, disposal and runloop stop: pass"
    );
}

#[cfg(target_os = "macos")]
mod native {
    use bboxy450_term_desktop_core::{Action, StopHandle, TitlebarAction};
    use bboxy450_term_desktop_macos::{apply_titlebar_action, apply_window_action};
    use dispatch2::{DispatchQueue, DispatchTime};
    use objc2::{MainThreadOnly, define_class, msg_send, rc::Retained};
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSView, NSWindow,
        NSWindowStyleMask,
    };
    use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize};
    use raw_window_handle::{
        AppKitWindowHandle, HandleError, HasWindowHandle, RawWindowHandle, WindowHandle,
    };
    use std::{cell::RefCell, ptr::NonNull, sync::mpsc::SyncSender, time::Duration};

    // Match production winit's macOS WinitWindow key/main eligibility. Source:
    // https://github.com/rust-windowing/winit/blob/v0.30.13/src/platform_impl/macos/window.rs
    define_class!(
        #[unsafe(super(NSWindow))]
        #[name = "BBoxy450DesktopProbeWindow"]
        #[thread_kind = MainThreadOnly]
        struct ProbeWindow;
        unsafe impl NSObjectProtocol for ProbeWindow {}
        impl ProbeWindow {
            #[unsafe(method(canBecomeKeyWindow))]
            fn can_become_key(&self) -> bool { true }
            #[unsafe(method(canBecomeMainWindow))]
            fn can_become_main(&self) -> bool { true }
        }
    );
    impl ProbeWindow {
        fn make_window(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSWindow> {
            let allocated = Self::alloc(mtm).set_ivars(());
            let style = NSWindowStyleMask::Borderless
                | NSWindowStyleMask::Resizable
                | NSWindowStyleMask::Miniaturizable;
            // SAFETY: initialize the NSWindow superclass exactly once, then keep
            // this private fixture under Rust retain ownership until explicit close.
            let window: Retained<Self> = unsafe {
                msg_send![super(allocated),
                initWithContentRect: frame, styleMask: style,
                backing: NSBackingStoreType::Buffered, defer: false]
            };
            window.into_super()
        }
    }
    struct Fixture {
        view: Retained<NSView>,
        window: Retained<NSWindow>,
        original: NSRect,
    }
    impl HasWindowHandle for Fixture {
        fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
            let handle = AppKitWindowHandle::new(NonNull::from(&*self.view).cast());
            // SAFETY: this main-thread fixture owns the view/window for the borrow.
            Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::AppKit(handle)) })
        }
    }
    thread_local! {
        // AppKit ownership never crosses a Send callback or the verdict worker.
        static WINDOW: RefCell<Option<Fixture>> = const { RefCell::new(None) };
    }
    fn close(a: NSRect, b: NSRect) -> bool {
        (a.origin.x - b.origin.x).abs() < 1.
            && (a.origin.y - b.origin.y).abs() < 1.
            && (a.size.width - b.size.width).abs() < 1.
            && (a.size.height - b.size.height).abs() < 1.
    }
    type Step = fn(&SyncSender<bool>, &StopHandle);
    fn schedule(delay: Duration, step: Step, verdict: SyncSender<bool>, stop: StopHandle) {
        let callback = move || {
            // A failing assertion must not unwind through a libdispatch callback.
            if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| step(&verdict, &stop)))
                .is_err()
            {
                let _ = std::panic::catch_unwind(|| {
                    WINDOW.with(|slot| {
                        if let Some(fixture) = slot.borrow_mut().take() {
                            fixture.window.close();
                        }
                    })
                });
                let _ = verdict.try_send(false);
                stop.stop();
            }
        };
        if delay.is_zero() {
            DispatchQueue::main().exec_async(callback);
        } else {
            DispatchQueue::main()
                .after(DispatchTime::try_from(delay).unwrap(), callback)
                .unwrap();
        }
    }
    pub(super) fn start(verdict: SyncSender<bool>, stop: StopHandle) {
        schedule(Duration::ZERO, prepare, verdict, stop);
    }
    fn next(step: Step, verdict: &SyncSender<bool>, stop: &StopHandle) {
        schedule(
            Duration::from_millis(700),
            step,
            verdict.clone(),
            stop.clone(),
        );
    }
    fn diagnose(label: &str, fixture: &Fixture) {
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        eprintln!(
            "native-window {label}: frame={:?} style={:?} mini={} miniaturizable={} visible={} zoomed={} key={} main={} app_active={} app_hidden={}",
            fixture.window.frame(),
            fixture.window.styleMask(),
            fixture.window.isMiniaturized(),
            fixture
                .window
                .styleMask()
                .contains(NSWindowStyleMask::Miniaturizable),
            fixture.window.isVisible(),
            fixture.window.isZoomed(),
            fixture.window.isKeyWindow(),
            fixture.window.isMainWindow(),
            app.isActive(),
            app.isHidden()
        );
    }
    fn prepare(verdict: &SyncSender<bool>, stop: &StopHandle) {
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
        next(create_and_fill, verdict, stop);
    }
    fn create_and_fill(verdict: &SyncSender<bool>, stop: &StopHandle) {
        let mtm = MainThreadMarker::new().unwrap();
        let frame = NSRect::new(NSPoint::new(100., 100.), NSSize::new(320., 180.));
        let window = ProbeWindow::make_window(mtm, frame);
        unsafe {
            window.setReleasedWhenClosed(false);
        }
        let view = NSView::initWithFrame(
            NSView::alloc(mtm),
            NSRect::new(NSPoint::new(0., 0.), frame.size),
        );
        window.setContentView(Some(&view));
        let original = window.frame();
        let fixture = Fixture {
            view,
            window,
            original,
        };
        assert!(
            !fixture
                .window
                .styleMask()
                .contains(NSWindowStyleMask::Titled)
        );
        apply_titlebar_action(&fixture, TitlebarAction::Fill).unwrap();
        assert!(close(
            fixture.window.frame(),
            fixture.window.screen().unwrap().visibleFrame()
        ));
        apply_titlebar_action(&fixture, TitlebarAction::Fill).unwrap();
        assert!(close(fixture.window.frame(), original));
        apply_titlebar_action(&fixture, TitlebarAction::None).unwrap();
        assert!(close(fixture.window.frame(), original));
        apply_window_action(&fixture, Action::ShowWindow).unwrap();
        diagnose("created/filled/restored/shown", &fixture);
        assert!(fixture.window.isVisible());
        WINDOW.with(|slot| *slot.borrow_mut() = Some(fixture));
        next(zoom_out, verdict, stop);
    }
    fn zoom_out(verdict: &SyncSender<bool>, stop: &StopHandle) {
        WINDOW.with(|slot| {
            let borrow = slot.borrow();
            let fixture = borrow.as_ref().unwrap();
            diagnose("before zoom", fixture);
            assert!(close(fixture.window.frame(), fixture.original));
            apply_window_action(fixture, Action::Zoom).unwrap();
        });
        next(zoom_restore, verdict, stop);
    }
    fn zoom_restore(verdict: &SyncSender<bool>, stop: &StopHandle) {
        WINDOW.with(|slot| {
            let borrow = slot.borrow();
            let fixture = borrow.as_ref().unwrap();
            diagnose("zoomed", fixture);
            assert!(!close(fixture.window.frame(), fixture.original));
            apply_window_action(fixture, Action::Zoom).unwrap();
        });
        next(minimize, verdict, stop);
    }
    fn minimize(verdict: &SyncSender<bool>, stop: &StopHandle) {
        WINDOW.with(|slot| {
            let borrow = slot.borrow();
            let fixture = borrow.as_ref().unwrap();
            diagnose("zoom restored/before minimize", fixture);
            assert!(close(fixture.window.frame(), fixture.original));
            apply_window_action(fixture, Action::Minimize).unwrap();
        });
        // Return to the real outer AppKit loop. No nested pumping or main-queue
        // blocking while Dock/WindowServer finishes these asynchronous requests.
        next(verify_minimized, verdict, stop);
    }
    fn verify_minimized(verdict: &SyncSender<bool>, stop: &StopHandle) {
        WINDOW.with(|slot| {
            let borrow = slot.borrow();
            let fixture = borrow.as_ref().unwrap();
            diagnose("after minimize", fixture);
            assert!(
                fixture.window.isMiniaturized(),
                "native window did not minimize"
            );
            apply_window_action(fixture, Action::ShowWindow).unwrap();
        });
        next(verify_restored, verdict, stop);
    }
    fn verify_restored(verdict: &SyncSender<bool>, stop: &StopHandle) {
        WINDOW.with(|slot| {
            let fixture = slot.borrow_mut().take().unwrap();
            diagnose("shown/restored", &fixture);
            assert!(
                !fixture.window.isMiniaturized() && fixture.window.isVisible(),
                "native window did not restore"
            );
            assert!(close(fixture.window.frame(), fixture.original));
            fixture.window.close();
        });
        verdict.try_send(true).unwrap();
        stop.stop();
    }
}
#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("native desktop probe requires macOS");
    std::process::exit(2);
}
