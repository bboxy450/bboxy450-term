//! Native UserNotifications transport; no application or window types.
//!
//! The delegate is installed once on the main thread, before the event loop.
//! Its retained +1 ownership intentionally lasts for the process: the center's
//! delegate property is weak. Methods construct native objects on the calling
//! IO thread rather than sending retained Objective-C objects between threads.
#![cfg(target_os = "macos")]

use block2::{DynBlock, RcBlock};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send,
    rc::Retained,
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_foundation::{NSArray, NSBundle, NSError, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent,
    UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNNotificationSettings, UNUserNotificationCenter, UNUserNotificationCenterDelegate,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

const COMPLETION_TIMEOUT: Duration = Duration::from_secs(2);
static OPENED: AtomicBool = AtomicBool::new(false);

/// Native authorization state mapped by the facade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authorization {
    Granted,
    Denied,
    Undetermined,
}

/// Native callback values contain only an owned identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Clicked(String),
    Dismissed(String),
}

/// The facade maps these errors without substituting another mechanism.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    NoBundle,
    Denied,
    Backend(String),
}

type Callback = Box<dyn Fn(Response) + Send + Sync + 'static>;
struct Ivars {
    callback: Callback,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "BBoxy450NotificationDelegate"]
    #[ivars = Ivars]
    struct Delegate;

    unsafe impl NSObjectProtocol for Delegate {}
    unsafe impl UNUserNotificationCenterDelegate for Delegate {
        #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
        fn response(
            &self,
            _center: &UNUserNotificationCenter,
            response: &UNNotificationResponse,
            completion: &DynBlock<dyn Fn()>,
        ) {
            let action = response.actionIdentifier();
            let id = response.notification().request().identifier().to_string();
            let response =
                if action.isEqualToString(unsafe { UNNotificationDefaultActionIdentifier }) {
                    Some(Response::Clicked(id))
                } else if action.isEqualToString(unsafe { UNNotificationDismissActionIdentifier }) {
                    Some(Response::Dismissed(id))
                } else {
                    None
                };
            deliver(response, &self.ivars().callback, || completion.call(()));
        }

        #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
        fn foreground(
            &self,
            _center: &UNUserNotificationCenter,
            _notification: &objc2_user_notifications::UNNotification,
            completion: &DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
        ) {
            // App preferences decide whether to post. No sound is requested.
            completion
                .call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List,));
        }
    }
);

fn deliver(response: Option<Response>, callback: &dyn Fn(Response), complete: impl FnOnce()) {
    // Never unwind across an Objective-C delegate boundary, including when
    // a caller's callback panics. The native completion still runs exactly once.
    if let Some(response) = response {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(response)));
    }
    complete();
}

/// A thread-safe port with no retained native object crossing thread boundaries.
#[derive(Debug)]
pub struct Handle {
    _private: (),
}

/// Install the process delegate after proving this is an actual app bundle.
pub fn open(callback: Callback) -> Result<Handle, Error> {
    let bundle = NSBundle::mainBundle();
    if bundle.bundleIdentifier().is_none() {
        return Err(Error::NoBundle);
    }
    let _main = MainThreadMarker::new()
        .ok_or_else(|| Error::Backend("notification open requires the main thread".into()))?;
    if OPENED.swap(true, Ordering::AcqRel) {
        return Err(Error::Backend(
            "notification delegate already installed".into(),
        ));
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let delegate = Delegate::alloc().set_ivars(Ivars { callback });
    // SAFETY: NSObject's designated initializer preserves the allocated class.
    let delegate: Retained<Delegate> = unsafe { msg_send![super(delegate), init] };
    center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    // The center does not retain its delegate. Keep it alive until process exit.
    let _ = Retained::into_raw(delegate);
    Ok(Handle { _private: () })
}

fn error(error: *mut NSError) -> Result<(), Error> {
    // SAFETY: completion-handler NSError is either nil or live for the callback.
    match unsafe { error.as_ref() } {
        Some(error) => Err(Error::Backend(error.localizedDescription().to_string())),
        None => Ok(()),
    }
}

#[allow(
    clippy::disallowed_methods,
    reason = "bounded native completion on the caller's notification IO worker, never the UI thread"
)]
fn completed<T>(receiver: mpsc::Receiver<T>, operation: &str) -> Result<T, Error> {
    receiver
        .recv_timeout(COMPLETION_TIMEOUT)
        .map_err(|reason| Error::Backend(format!("{operation} completion: {reason}")))
}

impl Handle {
    /// Read permission and, only when undetermined, ask the operator once.
    pub fn authorize(&self) -> Result<Authorization, Error> {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let (send, receive) = mpsc::sync_channel(1);
        let callback = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
            // SAFETY: settings is non-null and valid for this native callback.
            let status = unsafe { settings.as_ref() }.authorizationStatus();
            let state = match status {
                UNAuthorizationStatus::Authorized
                | UNAuthorizationStatus::Provisional
                | UNAuthorizationStatus::Ephemeral => Authorization::Granted,
                UNAuthorizationStatus::Denied => Authorization::Denied,
                _ => Authorization::Undetermined,
            };
            let _ = send.try_send(state);
        });
        center.getNotificationSettingsWithCompletionHandler(&callback);
        let state = completed(receive, "settings")?;
        if state != Authorization::Undetermined {
            return Ok(state);
        }
        let (send, receive) = mpsc::sync_channel(1);
        let callback = RcBlock::new(move |granted: objc2::runtime::Bool, native: *mut NSError| {
            let result = error(native).map(|()| {
                if granted.as_bool() {
                    Authorization::Granted
                } else {
                    Authorization::Denied
                }
            });
            let _ = send.try_send(result);
        });
        center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &callback,
        );
        completed(receive, "authorization")?
    }

    /// Schedule a silent notification using the caller's replace identifier.
    pub fn post(&self, id: &str, title: &str, body: &str) -> Result<(), Error> {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(title));
        content.setBody(&NSString::from_str(body));
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(id),
            &content,
            None,
        );
        let (send, receive) = mpsc::sync_channel(1);
        let callback = RcBlock::new(move |native: *mut NSError| {
            let _ = send.try_send(error(native));
        });
        center.addNotificationRequest_withCompletionHandler(&request, Some(&callback));
        completed(receive, "post")?
    }

    /// Withdraw both scheduled and delivered instances of this identifier.
    pub fn remove(&self, id: &str) -> Result<(), Error> {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let ids = NSArray::from_retained_slice(&[NSString::from_str(id)]);
        center.removePendingNotificationRequestsWithIdentifiers(&ids);
        center.removeDeliveredNotificationsWithIdentifiers(&ids);
        Ok(())
    }
}

/// Read actual Notification Center delivery state for the opt-in debug probe.
#[cfg(all(feature = "probe", debug_assertions))]
pub fn delivered_ids() -> Result<Vec<String>, Error> {
    if NSBundle::mainBundle().bundleIdentifier().is_none() {
        return Err(Error::NoBundle);
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let (send, receive) = mpsc::sync_channel(1);
    let callback = RcBlock::new(
        move |notifications: std::ptr::NonNull<
            NSArray<objc2_user_notifications::UNNotification>,
        >| {
            // SAFETY: the collection remains live for the native callback.
            let ids = unsafe { notifications.as_ref() }
                .iter()
                .map(|notification| notification.request().identifier().to_string())
                .collect();
            let _ = send.try_send(ids);
        },
    );
    center.getDeliveredNotificationsWithCompletionHandler(&callback);
    completed(receive, "delivered ids")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    // keeps: spec desktop-notifications §3/§5.2 — response carries the posted id, native completion follows callback
    #[test]
    fn delegate_forwards_the_posted_id_before_completing() {
        let calls = RefCell::new(vec![]);
        deliver(
            Some(Response::Clicked("seat-42".into())),
            &|response| {
                assert_eq!(response, Response::Clicked("seat-42".into()));
                calls.borrow_mut().push("response");
            },
            || calls.borrow_mut().push("completion"),
        );
        assert_eq!(*calls.borrow(), ["response", "completion"]);
    }

    // keeps: spec desktop-notifications §3 — native completion still runs when the callback panics
    #[test]
    fn a_panicking_callback_does_not_skip_native_completion() {
        let completed = std::cell::Cell::new(false);
        deliver(
            Some(Response::Dismissed("seat".into())),
            &|_| panic!("fixture"),
            || completed.set(true),
        );
        assert!(completed.get());
    }
}
