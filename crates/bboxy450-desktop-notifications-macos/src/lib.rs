// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]
#![cfg(target_os = "macos")]

use block2::{DynBlock, RcBlock};
use objc2::{
    AnyThread, DefinedClass, MainThreadMarker, define_class, msg_send,
    rc::Retained,
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
};
use objc2_foundation::{NSArray, NSBundle, NSError, NSString};
use objc2_user_notifications::{
    UNAlertStyle, UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent,
    UNNotificationDefaultActionIdentifier, UNNotificationDismissActionIdentifier,
    UNNotificationPresentationOptions, UNNotificationRequest, UNNotificationResponse,
    UNNotificationSetting, UNNotificationSettings, UNUserNotificationCenter,
    UNUserNotificationCenterDelegate,
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
    /// The system permits notifications.
    Granted,
    /// The system refuses notifications.
    Denied,
    /// The system has not decided authorization.
    Undetermined,
}

/// One independently configured native notification feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// The system enables the feature.
    Enabled,
    /// The system disables the feature.
    Disabled,
    /// The current system does not support the feature.
    NotSupported,
    /// An unrecognized native value; never treated as enabled.
    Unknown,
}

/// The system's configured visual notification presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertStyle {
    /// No onscreen alert presentation.
    None,
    /// Transient banner presentation.
    Banner,
    /// Persistent alert presentation.
    Alert,
    /// An unrecognized native style.
    Unknown,
}

/// Read-only native settings, independent of a notification delivery receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Scheduling and receiving authorization.
    pub authorization: Authorization,
    /// Whether visual alerts are enabled.
    pub alerts: Setting,
    /// The configured visual alert style.
    pub alert_style: AlertStyle,
    /// Whether Notification Center presentation is enabled.
    pub notification_center: Setting,
    /// Whether sounds are enabled; v1 posts still request no sound.
    pub sound: Setting,
}

/// Native callback values contain only an owned identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// The default action was activated for this stable identifier.
    Clicked(String),
    /// The notification with this stable identifier was dismissed.
    Dismissed(String),
}

/// The facade maps these errors without substituting another mechanism.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The process has no suitable application-bundle identity.
    NoBundle,
    /// The system refused authorization or posting.
    Denied,
    /// The native transport returned a failure.
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
///
/// # Errors
/// Refuses absent bundle identity, calls off the main thread or a second installation.
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

fn normalize_settings(
    authorization: UNAuthorizationStatus,
    alerts: UNNotificationSetting,
    alert_style: UNAlertStyle,
    notification_center: UNNotificationSetting,
    sound: UNNotificationSetting,
) -> Settings {
    let setting = |value| match value {
        UNNotificationSetting::Enabled => Setting::Enabled,
        UNNotificationSetting::Disabled => Setting::Disabled,
        UNNotificationSetting::NotSupported => Setting::NotSupported,
        _ => Setting::Unknown,
    };
    Settings {
        authorization: match authorization {
            UNAuthorizationStatus::Authorized
            | UNAuthorizationStatus::Provisional
            | UNAuthorizationStatus::Ephemeral => Authorization::Granted,
            UNAuthorizationStatus::Denied => Authorization::Denied,
            _ => Authorization::Undetermined,
        },
        alerts: setting(alerts),
        alert_style: match alert_style {
            UNAlertStyle::None => AlertStyle::None,
            UNAlertStyle::Banner => AlertStyle::Banner,
            UNAlertStyle::Alert => AlertStyle::Alert,
            _ => AlertStyle::Unknown,
        },
        notification_center: setting(notification_center),
        sound: setting(sound),
    }
}

/// Read native notification settings without requesting permission.
///
/// This query neither installs a delegate nor calls the authorization or post
/// APIs. Invoke it on an IO worker, never a UI turn.
///
/// # Errors
/// Refuses absent bundle identity or failure to complete the native query within two seconds.
pub fn settings() -> Result<Settings, Error> {
    if NSBundle::mainBundle().bundleIdentifier().is_none() {
        return Err(Error::NoBundle);
    }
    let center = UNUserNotificationCenter::currentNotificationCenter();
    let (send, receive) = mpsc::sync_channel(1);
    let callback = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
        // SAFETY: settings remains non-null and valid for this native callback.
        let settings = unsafe { settings.as_ref() };
        let state = normalize_settings(
            settings.authorizationStatus(),
            settings.alertSetting(),
            settings.alertStyle(),
            settings.notificationCenterSetting(),
            settings.soundSetting(),
        );
        let _ = send.try_send(state);
    });
    center.getNotificationSettingsWithCompletionHandler(&callback);
    completed(receive, "settings")
}

/// The sole post submission port receives the actual platform request. Unit
/// fixtures replace only this IO step; request construction remains production code.
fn submit_request(request: &UNNotificationRequest, completion: &DynBlock<dyn Fn(*mut NSError)>) {
    #[cfg(not(test))]
    {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        center.addNotificationRequest_withCompletionHandler(request, Some(completion));
    }
    #[cfg(test)]
    tests::submit_request(request, completion);
}

impl Handle {
    /// Read permission and, only when undetermined, ask the operator once.
    ///
    /// # Errors
    /// Returns a native error or failure to complete settings or authorization within two seconds.
    pub fn authorize(&self) -> Result<Authorization, Error> {
        let state = settings()?.authorization;
        if state != Authorization::Undetermined {
            return Ok(state);
        }
        let center = UNUserNotificationCenter::currentNotificationCenter();
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
    ///
    /// # Errors
    /// Returns the native scheduling error or a completion timeout.
    pub fn post(&self, id: &str, title: &str, body: &str) -> Result<(), Error> {
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
        submit_request(&request, &callback);
        completed(receive, "post")?
    }

    /// Withdraw both scheduled and delivered instances of this identifier.
    ///
    /// # Errors
    /// The current implementation dispatches both removals and returns success without a receipt.
    pub fn remove(&self, id: &str) -> Result<(), Error> {
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let ids = NSArray::from_retained_slice(&[NSString::from_str(id)]);
        center.removePendingNotificationRequestsWithIdentifiers(&ids);
        center.removeDeliveredNotificationsWithIdentifiers(&ids);
        Ok(())
    }
}

/// Read actual Notification Center delivery state for the opt-in debug probe.
///
/// # Errors
/// Refuses absent bundle identity or a native completion timeout.
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

    #[test]
    fn settings_normalization_keeps_alert_features_independent_of_authorization() {
        let cases = [
            (
                UNAuthorizationStatus::Authorized,
                UNNotificationSetting::Disabled,
                UNAlertStyle::Banner,
                UNNotificationSetting::Enabled,
                UNNotificationSetting::Disabled,
                Settings {
                    authorization: Authorization::Granted,
                    alerts: Setting::Disabled,
                    alert_style: AlertStyle::Banner,
                    notification_center: Setting::Enabled,
                    sound: Setting::Disabled,
                },
            ),
            (
                UNAuthorizationStatus::Authorized,
                UNNotificationSetting::Enabled,
                UNAlertStyle::None,
                UNNotificationSetting::Disabled,
                UNNotificationSetting::Enabled,
                Settings {
                    authorization: Authorization::Granted,
                    alerts: Setting::Enabled,
                    alert_style: AlertStyle::None,
                    notification_center: Setting::Disabled,
                    sound: Setting::Enabled,
                },
            ),
            (
                UNAuthorizationStatus::Provisional,
                UNNotificationSetting::NotSupported,
                UNAlertStyle::Alert,
                UNNotificationSetting::NotSupported,
                UNNotificationSetting(99),
                Settings {
                    authorization: Authorization::Granted,
                    alerts: Setting::NotSupported,
                    alert_style: AlertStyle::Alert,
                    notification_center: Setting::NotSupported,
                    sound: Setting::Unknown,
                },
            ),
            (
                UNAuthorizationStatus::Denied,
                UNNotificationSetting::Enabled,
                UNAlertStyle::Banner,
                UNNotificationSetting::Enabled,
                UNNotificationSetting::Enabled,
                Settings {
                    authorization: Authorization::Denied,
                    alerts: Setting::Enabled,
                    alert_style: AlertStyle::Banner,
                    notification_center: Setting::Enabled,
                    sound: Setting::Enabled,
                },
            ),
            (
                UNAuthorizationStatus::NotDetermined,
                UNNotificationSetting(99),
                UNAlertStyle(99),
                UNNotificationSetting::Disabled,
                UNNotificationSetting::NotSupported,
                Settings {
                    authorization: Authorization::Undetermined,
                    alerts: Setting::Unknown,
                    alert_style: AlertStyle::Unknown,
                    notification_center: Setting::Disabled,
                    sound: Setting::NotSupported,
                },
            ),
        ];
        for (authorization, alerts, style, center, sound, expected) in cases {
            assert_eq!(
                normalize_settings(authorization, alerts, style, center, sound),
                expected,
            );
        }
        for (native, expected) in [
            (UNAuthorizationStatus::Ephemeral, Authorization::Granted),
            (UNAuthorizationStatus(99), Authorization::Undetermined),
        ] {
            assert_eq!(
                normalize_settings(
                    native,
                    UNNotificationSetting::Disabled,
                    UNAlertStyle::None,
                    UNNotificationSetting::Disabled,
                    UNNotificationSetting::Disabled,
                )
                .authorization,
                expected,
            );
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct Submitted {
        id: String,
        title: String,
        body: String,
    }

    std::thread_local! {
        static SUBMISSIONS: RefCell<Option<Vec<Submitted>>> = const { RefCell::new(None) };
    }

    struct Capture;
    impl Capture {
        fn new() -> Self {
            SUBMISSIONS.with(|slot| assert!(slot.replace(Some(Vec::new())).is_none()));
            Self
        }
        fn requests(&self) -> Vec<Submitted> {
            SUBMISSIONS.with(|slot| slot.borrow().as_ref().unwrap().clone())
        }
    }
    impl Drop for Capture {
        fn drop(&mut self) {
            SUBMISSIONS.with(|slot| { slot.take(); });
        }
    }

    // This private per-test-thread port never opens a notification center,
    // authorizes an account or claims OS presentation. It reads the real request.
    pub(super) fn submit_request(
        request: &UNNotificationRequest,
        completion: &DynBlock<dyn Fn(*mut NSError)>,
    ) {
        let content = request.content();
        let actual = Submitted {
            id: request.identifier().to_string(),
            title: content.title().to_string(),
            body: content.body().to_string(),
        };
        SUBMISSIONS.with(|slot| {
            let mut slot = slot.borrow_mut();
            let requests = slot.as_mut().expect("native copy test submission port is not installed");
            assert!(requests.len() < 16, "bounded synthetic native-copy fixture");
            requests.push(actual);
        });
        completion.call((std::ptr::null_mut(),));
    }

    #[test]
    fn native_copy_fields_cross_actual_request_submission() {
        let capture = Capture::new();
        let handle = Handle { _private: () };
        let title = "Säble 🦊 · claude";
        let questions = ["¿Dónde está el café?", "次の一行を確認しますか？"];
        for question in questions {
            handle.post("seat-copy-42", title, question).unwrap();
        }
        let expected: Vec<_> = questions.into_iter().map(|body| Submitted {
            id: "seat-copy-42".into(), title: title.into(), body: body.into(),
        }).collect();
        assert_eq!(capture.requests(), expected);
        assert_eq!(capture.requests()[0].id, capture.requests()[1].id);
    }

    #[test]
    #[ignore = "generated private app witness; make test-notification-copy"]
    #[allow(
        clippy::disallowed_methods,
        reason = "bounded private unit-fixture files; this test never runs on the UI thread"
    )]
    fn native_copy_requests_from_private_app() {
        let input = std::env::var_os("BBOXY_NOTIFICATION_COPY_INPUT")
            .expect("the private app runner supplies its bounded request witness");
        let bytes = std::fs::read(input).unwrap();
        assert!(bytes.len() <= 16 * 4096, "bounded synthetic copy input");
        let text = std::str::from_utf8(&bytes).unwrap();
        let fields: Vec<_> = text.split_terminator('\0').collect();
        assert!(!fields.is_empty() && fields.len().is_multiple_of(3));
        let expected: Vec<_> = fields.as_chunks::<3>().0.iter().map(|row| Submitted {
            id: row[0].into(), title: row[1].into(), body: row[2].into(),
        }).collect();
        let capture = Capture::new();
        let handle = Handle { _private: () };
        for request in &expected {
            handle.post(&request.id, &request.title, &request.body).unwrap();
        }
        let actual = capture.requests();
        assert_eq!(actual, expected);
        if let Some(output) = std::env::var_os("BBOXY_NOTIFICATION_COPY_CAPTURE") {
            let actual: Vec<u8> = actual.iter().flat_map(|request| {
                [&request.id, &request.title, &request.body].into_iter()
                    .flat_map(|field| field.as_bytes().iter().copied().chain([0]))
            }).collect();
            std::fs::write(output, actual).unwrap();
        }
        println!("observed {} actual native requests from the private app witness", expected.len());
    }

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
