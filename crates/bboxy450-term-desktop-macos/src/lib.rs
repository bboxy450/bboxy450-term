// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![doc = include_str!("../README.md")]
#![cfg(target_os = "macos")]

use bboxy450_term_desktop_core::{
    Action, Callback, Error, MenuCommand, MenuItem, StopHandle, TitlebarAction, app_menu,
    run_without_native, tray_menu,
};
use objc2::{
    DefinedClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{AnyObject, ProtocolObject, Sel},
    sel,
};
use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSEvent,
    NSEventModifierFlags, NSEventType, NSImage, NSMenu, NSMenuItem, NSStatusBar, NSStatusItem,
    NSView, NSWindow,
};
use objc2_foundation::{
    MainThreadMarker, NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString,
    NSUserDefaults, NSValue, ns_string,
};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::time::Duration;

struct Ivars {
    callback: Callback,
}
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "BBoxy450DesktopMenuTarget"]
    #[thread_kind = MainThreadOnly]
    #[ivars = Ivars]
    struct Target;
    unsafe impl NSObjectProtocol for Target {}
    unsafe impl NSApplicationDelegate for Target {
        #[unsafe(method(applicationShouldHandleReopen:hasVisibleWindows:))]
        fn reopen(&self, _app: &NSApplication, _visible: bool) -> bool {
            // Finder/Dock may address the surviving accessory process after
            // the UI quits. Reopen the same run through its desktop action.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(||
                (self.ivars().callback)(Action::ShowWindow)));
            false
        }
    }
    impl Target {
        #[unsafe(method(activate:))]
        fn activate(&self, sender: &NSMenuItem) {
            if let Ok(index) = usize::try_from(sender.tag())
                && let Some(action) = Action::ALL.get(index) {
                    // A host panic cannot unwind through Objective-C.
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(||
                        (self.ivars().callback)(*action)));
            }
        }
    }
);
impl Target {
    fn new(mtm: MainThreadMarker, callback: Callback) -> Retained<Self> {
        let allocated = Self::alloc(mtm).set_ivars(Ivars { callback });
        // SAFETY: Target is an NSObject subclass; its ivars are initialized above.
        unsafe { msg_send![super(allocated), init] }
    }
}
struct NativeMenu {
    _target: Retained<Target>,
    menus: Vec<Retained<NSMenu>>,
    actions: Vec<(Action, Retained<NSMenu>, isize)>,
}
impl NativeMenu {
    fn new(mtm: MainThreadMarker, callback: Callback) -> Self {
        Self {
            _target: Target::new(mtm, callback),
            menus: vec![],
            actions: vec![],
        }
    }
    fn build(
        &mut self,
        mtm: MainThreadMarker,
        title: &str,
        items: &[MenuItem],
    ) -> Retained<NSMenu> {
        let menu = NSMenu::new(mtm);
        menu.setTitle(&NSString::from_str(title));
        // Native responder entries such as Undo are disabled when unsupported.
        menu.setAutoenablesItems(true);
        for (index, item) in items.iter().enumerate() {
            if item.command == MenuCommand::Separator {
                menu.addItem(&NSMenuItem::separatorItem(mtm));
                continue;
            }
            let selector = match item.command {
                MenuCommand::Action(_) => Some(sel!(activate:)),
                MenuCommand::Responder(name) => Some(Sel::register(
                    &std::ffi::CString::new(name).expect("static menu selector"),
                )),
                MenuCommand::Separator => None,
            };
            // SAFETY: App actions use Target::activate:, responder selectors are
            // conventional NSResponder methods with a single sender argument.
            let native = unsafe {
                NSMenuItem::initWithTitle_action_keyEquivalent(
                    mtm.alloc(),
                    &NSString::from_str(item.title),
                    selector,
                    &NSString::from_str(item.key),
                )
            };
            let mut modifiers = NSEventModifierFlags::Command;
            if item.option {
                modifiers |= NSEventModifierFlags::Option;
            }
            if item.shift {
                modifiers |= NSEventModifierFlags::Shift;
            }
            if item.control {
                modifiers |= NSEventModifierFlags::Control;
            }
            native.setKeyEquivalentModifierMask(modifiers);
            if let MenuCommand::Action(action) = item.command {
                let tag = Action::ALL
                    .iter()
                    .position(|a| *a == action)
                    .expect("known action");
                native.setTag(tag as isize);
                // SAFETY: NativeMenu retains Target for all installed items.
                unsafe {
                    native.setTarget(Some(&self._target));
                }
                self.actions.push((action, menu.clone(), index as isize));
            }
            menu.addItem(&native);
        }
        self.menus.push(menu.clone());
        menu
    }
    fn activate(&self, action: Action) -> Result<(), Error> {
        let (_, menu, index) = self
            .actions
            .iter()
            .find(|(a, _, _)| *a == action)
            .ok_or_else(|| Error::InvalidAction(action.as_str().into()))?;
        menu.performActionForItemAtIndex(*index);
        Ok(())
    }
}

/// Retains the UI process's application menu and callback target on the main thread.
pub struct MenuBar {
    menu: Retained<NSMenu>,
    native: NativeMenu,
}
impl MenuBar {
    pub fn install(callback: Callback) -> Result<Self, Error> {
        let mtm = MainThreadMarker::new().ok_or(Error::MainThreadRequired)?;
        let mut native = NativeMenu::new(mtm, callback);
        let menu = NSMenu::new(mtm);
        for group in app_menu() {
            let sub = native.build(mtm, group.title, &group.items);
            let item = NSMenuItem::new(mtm);
            item.setTitle(&NSString::from_str(group.title));
            item.setSubmenu(Some(&sub));
            menu.addItem(&item);
        }
        NSApplication::sharedApplication(mtm).setMainMenu(Some(&menu));
        Ok(Self { menu, native })
    }
    /// Invoke an actual installed native item without desktop input injection.
    pub fn activate(&self, action: Action) -> Result<(), Error> {
        self.native.activate(action)
    }
}
impl Drop for MenuBar {
    fn drop(&mut self) {
        let app = NSApplication::sharedApplication(MainThreadMarker::from(&*self.menu));
        if app.mainMenu().as_deref() == Some(&*self.menu) {
            app.setMainMenu(None);
        }
    }
}

/// Retains one daemon-owned status item until the supervisor exits.
pub struct Tray {
    item: Retained<NSStatusItem>,
    native: NativeMenu,
}
impl Tray {
    pub fn install(callback: Callback) -> Result<Self, Error> {
        let mtm = MainThreadMarker::new().ok_or(Error::MainThreadRequired)?;
        let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(
            ns_string!("terminal"),
            Some(ns_string!("bboxy450-term")),
        )
        .ok_or_else(|| Error::Native("terminal status symbol unavailable".into()))?;
        image.setTemplate(true);
        image.setSize(NSSize::new(18., 18.));
        let mut native = NativeMenu::new(mtm, callback);
        let menu = native.build(mtm, "bboxy450-term", &tray_menu());
        let bar = NSStatusBar::systemStatusBar();
        let item = bar.statusItemWithLength(-1.);
        let Some(button) = item.button(mtm) else {
            bar.removeStatusItem(&item);
            return Err(Error::Native("status item button unavailable".into()));
        };
        button.setImage(Some(&image));
        button.setToolTip(Some(ns_string!("bboxy450-term — sessions keep running")));
        item.setMenu(Some(&menu));
        Ok(Self { item, native })
    }
    pub fn activate(&self, action: Action) -> Result<(), Error> {
        self.native.activate(action)
    }
}
impl Drop for Tray {
    fn drop(&mut self) {
        NSStatusBar::systemStatusBar().removeStatusItem(&self.item);
    }
}

pub fn run_daemon(
    callback: Callback,
    worker: impl FnOnce(StopHandle) + Send + 'static,
) -> Result<(), Error> {
    let Some(mtm) = MainThreadMarker::new() else {
        return run_without_native(worker, Error::MainThreadRequired);
    };
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    let tray = match Tray::install(callback) {
        Ok(tray) => tray,
        Err(error) => return run_without_native(worker, error),
    };
    app.setDelegate(Some(ProtocolObject::from_ref(&*tray.native._target)));
    let stop = StopHandle::new(Some(stop_main_loop), None);
    let worker = std::thread::spawn({
        let stop = stop.clone();
        move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| worker(stop.clone())));
            stop.stop();
            result
        }
    });
    app.run();
    worker
        .join()
        .map_err(|_| Error::Native("daemon worker panicked".into()))?
        .map_err(|_| Error::Native("daemon worker panicked".into()))?;
    app.setDelegate(None);
    drop(tray);
    Ok(())
}

pub fn stop_main_loop() {
    dispatch2::DispatchQueue::main().exec_async(|| {
        let Some(mtm) = MainThreadMarker::new() else { return; };
        let app = NSApplication::sharedApplication(mtm);
        app.stop(None);
        // stop: alone does not wake a blocked next-event wait. Post one private
        // application-defined event, then the normal runloop observes stop.
        if let Some(event) = NSEvent::otherEventWithType_location_modifierFlags_timestamp_windowNumber_context_subtype_data1_data2(
            NSEventType::ApplicationDefined, NSPoint::new(0., 0.),
            NSEventModifierFlags::empty(), 0., 0, None, 0, 0, 0) {
            app.postEvent_atStart(&event, true);
        }
    });
}

pub fn titlebar_action() -> TitlebarAction {
    let value =
        NSUserDefaults::standardUserDefaults().stringForKey(ns_string!("AppleActionOnDoubleClick"));
    match value.as_ref().map(|s| s.to_string()).as_deref() {
        Some("Fill") => TitlebarAction::Fill,
        Some("Minimize") => TitlebarAction::Minimize,
        Some("None") => TitlebarAction::None,
        _ => TitlebarAction::Zoom,
    }
}
pub fn double_click_interval() -> Duration {
    let seconds = NSEvent::doubleClickInterval();
    if seconds.is_finite() && seconds > 0. {
        Duration::from_secs_f64(seconds)
    } else {
        Duration::from_millis(500)
    }
}
fn native_window(window: &impl HasWindowHandle) -> Result<Retained<NSWindow>, Error> {
    MainThreadMarker::new().ok_or(Error::MainThreadRequired)?;
    let handle = window
        .window_handle()
        .map_err(|e| Error::Native(e.to_string()))?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return Err(Error::Unsupported);
    };
    // SAFETY: HasWindowHandle guarantees this live NSView for the duration of
    // the host borrow. AppKit window commands are restricted to its main thread.
    let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    view.window()
        .ok_or_else(|| Error::Native("view has no window".into()))
}
pub fn apply_window_action(window: &impl HasWindowHandle, action: Action) -> Result<(), Error> {
    let window = native_window(window)?;
    match action {
        Action::ShowWindow => {
            let app = NSApplication::sharedApplication(MainThreadMarker::from(&*window));
            app.unhide(None);
            // AppKit can complete a deminiaturization asynchronously even for
            // an already-visible window; a redundant request can undo a later
            // Minimize. Match the actual native state before issuing it.
            if window.isMiniaturized() {
                window.deminiaturize(None);
            }
            window.makeKeyAndOrderFront(None);
            app.activate();
        }
        Action::HideWindow => {
            NSApplication::sharedApplication(MainThreadMarker::from(&*window)).hide(None)
        }
        Action::Minimize => {
            if !window.isMiniaturized() {
                window.miniaturize(None);
            }
        }
        Action::Zoom => window.zoom(None),
        Action::ToggleFullscreen => window.toggleFullScreen(None),
        _ => return Err(Error::InvalidAction(action.as_str().into())),
    }
    Ok(())
}

// Origin: bboxy450-term desktop Fill/restore adapter. Keep state attached to the
// native window so closing it releases saved geometry; no defaults IO or global
// pointer-keyed cache. https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
static RESTORE_FRAME_KEY: u8 = 0;
#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getAssociatedObject(
        object: *mut AnyObject,
        key: *const std::ffi::c_void,
    ) -> *mut AnyObject;
    fn objc_setAssociatedObject(
        object: *mut AnyObject,
        key: *const std::ffi::c_void,
        value: *mut AnyObject,
        policy: usize,
    );
}
fn fill(window: &NSWindow) -> Result<(), Error> {
    let screen = window
        .screen()
        .ok_or_else(|| Error::Native("window has no screen".into()))?;
    let current = window.frame();
    let visible = screen.visibleFrame();
    let key = std::ptr::addr_of!(RESTORE_FRAME_KEY).cast();
    let object = (window as *const NSWindow).cast_mut().cast();
    // SAFETY: the unique key stores only a retained NSValue on this NSWindow.
    // All access occurs on the main thread, and associations die with the window.
    unsafe {
        let saved = objc_getAssociatedObject(object, key);
        let close = |a: NSRect, b: NSRect| {
            (a.origin.x - b.origin.x).abs() < 1.
                && (a.origin.y - b.origin.y).abs() < 1.
                && (a.size.width - b.size.width).abs() < 1.
                && (a.size.height - b.size.height).abs() < 1.
        };
        if !saved.is_null() && close(current, visible) {
            window.setFrame_display((&*saved.cast::<NSValue>()).rectValue(), true);
            objc_setAssociatedObject(object, key, std::ptr::null_mut(), 1);
        } else {
            let value = NSValue::valueWithRect(current);
            objc_setAssociatedObject(object, key, Retained::as_ptr(&value).cast_mut().cast(), 1);
            window.setFrame_display(visible, true);
        }
    }
    Ok(())
}
pub fn apply_titlebar_action(
    window: &impl HasWindowHandle,
    action: TitlebarAction,
) -> Result<(), Error> {
    match action {
        TitlebarAction::Fill => fill(&*native_window(window)?),
        TitlebarAction::Zoom => apply_window_action(window, Action::Zoom),
        TitlebarAction::Minimize => apply_window_action(window, Action::Minimize),
        TitlebarAction::None => Ok(()),
    }
}
