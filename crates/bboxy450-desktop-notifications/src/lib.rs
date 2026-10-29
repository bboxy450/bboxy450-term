// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]

#[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
compile_error!("desktop notifications: no backend for this target OS");

/// A caller-owned stable identifier. Reposting it replaces the notification.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NotificationId(pub String);

/// A silent notification with a default click action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notification {
    /// Stable replacement and removal key owned by the caller.
    pub id: NotificationId,
    /// Displayed notification title.
    pub title: String,
    /// Displayed notification body.
    pub body: String,
}

/// A native response; it carries no window or application-specific identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// The default action was activated for this stable identifier.
    Clicked(NotificationId),
    /// The notification with this stable identifier was dismissed.
    Dismissed(NotificationId),
}

/// The system's authorization state, not an application preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Authorization {
    /// The system permits notifications.
    Granted,
    /// The system refuses notifications.
    Denied,
    /// The system has not decided authorization.
    Undetermined,
}

/// One system-controlled notification feature, independent of app authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// The system enables this feature for the app.
    Enabled,
    /// The system disables this feature for the app.
    Disabled,
    /// This feature is unavailable on the current system.
    NotSupported,
    /// The system returned a value this version does not recognize.
    Unknown,
}

/// The configured visual presentation, independent of scheduling permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertStyle {
    /// Notifications have no onscreen alert presentation.
    None,
    /// Notifications use transient banners.
    Banner,
    /// Notifications use persistent alerts.
    Alert,
    /// The system returned a style this version does not recognize.
    Unknown,
}

/// A read-only native settings snapshot, not proof that a banner was presented.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// Whether the system permits scheduling and receiving notifications.
    pub authorization: Authorization,
    /// Whether visual alerts are enabled.
    pub alerts: Setting,
    /// The visual presentation configured by the operator.
    pub alert_style: AlertStyle,
    /// Whether notifications may appear in Notification Center.
    pub notification_center: Setting,
    /// Whether notification sounds are enabled; posts remain silent in v1.
    pub sound: Setting,
}

/// Read native settings without requesting permission or installing a delegate.
///
/// This synchronous query belongs on an IO worker. It never calls [`open`],
/// authorizes an account, posts a notification or changes system settings.
///
/// # Errors
/// Returns a named unsupported platform, absent bundle identity or native completion failure.
pub fn settings() -> Result<Settings, Error> {
    #[cfg(target_os = "macos")]
    {
        bboxy450_desktop_notifications_macos::settings()
            .map(Settings::from)
            .map_err(native_error)
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(Error::Unsupported {
            os: std::env::consts::OS,
            note: "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md",
        })
    }
}

/// A named unsupported platform or native failure. There is no fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The selected platform has no implemented native backend.
    Unsupported {
        /// The unavailable platform.
        os: &'static str,
        /// The backend research marker.
        note: &'static str,
    },
    /// The process has no suitable application-bundle identity.
    NoBundle,
    /// The system refused authorization or posting.
    Denied,
    /// The native transport returned a failure.
    Backend(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported { os, note } => {
                write!(f, "notifications unsupported on {os}: {note}")
            }
            Self::NoBundle => f.write_str("notifications unavailable: no application bundle"),
            Self::Denied => f.write_str("notifications denied by the operator"),
            Self::Backend(reason) => write!(f, "notifications backend: {reason}"),
        }
    }
}
impl std::error::Error for Error {}

/// Synchronous completion port. Invoke it from an IO worker, never a UI turn.
pub trait Notifier: Send + Sync {
    /// Read or request native authorization.
    ///
    /// # Errors
    /// Returns a named transport or platform failure.
    fn authorize(&self) -> Result<Authorization, Error>;
    /// Post or replace the silent notification under its stable identifier.
    ///
    /// # Errors
    /// Returns denial, unsupported platform or native transport failure.
    fn post(&self, notification: &Notification) -> Result<(), Error>;
    /// Withdraw pending and delivered notifications under this identifier.
    ///
    /// # Errors
    /// Returns a named transport or platform failure.
    fn remove(&self, id: &NotificationId) -> Result<(), Error>;
}

/// Opt-in, debug-only native evidence port. It cannot authorize or post.
#[cfg(all(feature = "probe", debug_assertions))]
pub mod probe {
    use super::*;
    /// Query actual delivered identifiers, not an in-process post counter.
    ///
    /// # Errors
    /// Returns the named unsupported platform, absent bundle identity or native completion failure.
    pub fn delivered_ids() -> Result<Vec<NotificationId>, Error> {
        #[cfg(target_os = "macos")]
        {
            bboxy450_desktop_notifications_macos::delivered_ids()
                .map(|ids| ids.into_iter().map(NotificationId).collect())
                .map_err(native_error)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(Error::Unsupported {
                os: std::env::consts::OS,
                note: "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md",
            })
        }
    }
}

/// Install the response delegate once, before the application finishes launch.
///
/// # Errors
/// Refuses an unsupported backend, absent bundle or main-thread context, or repeated installation.
pub fn open(
    on_response: Box<dyn Fn(Response) + Send + Sync + 'static>,
) -> Result<Box<dyn Notifier>, Error> {
    #[cfg(target_os = "macos")]
    {
        use bboxy450_desktop_notifications_macos as backend;
        let handle = backend::open(Box::new(move |response| {
            on_response(match response {
                backend::Response::Clicked(id) => Response::Clicked(NotificationId(id)),
                backend::Response::Dismissed(id) => Response::Dismissed(NotificationId(id)),
            });
        }))
        .map_err(native_error)?;
        Ok(Box::new(Mac(handle)))
    }
    #[cfg(target_os = "windows")]
    {
        let _ = on_response;
        let error = bboxy450_desktop_notifications_windows::open().unwrap_err();
        Err(Error::Unsupported {
            os: error.os,
            note: error.note,
        })
    }
    #[cfg(target_os = "linux")]
    {
        let _ = on_response;
        let error = bboxy450_desktop_notifications_linux::open().unwrap_err();
        Err(Error::Unsupported {
            os: error.os,
            note: error.note,
        })
    }
}

#[cfg(target_os = "macos")]
fn native_error(error: bboxy450_desktop_notifications_macos::Error) -> Error {
    use bboxy450_desktop_notifications_macos::Error as Native;
    match error {
        Native::NoBundle => Error::NoBundle,
        Native::Denied => Error::Denied,
        Native::Backend(reason) => Error::Backend(reason),
    }
}

#[cfg(target_os = "macos")]
impl From<bboxy450_desktop_notifications_macos::Settings> for Settings {
    fn from(settings: bboxy450_desktop_notifications_macos::Settings) -> Self {
        use bboxy450_desktop_notifications_macos as native;
        let setting = |value| match value {
            native::Setting::Enabled => Setting::Enabled,
            native::Setting::Disabled => Setting::Disabled,
            native::Setting::NotSupported => Setting::NotSupported,
            native::Setting::Unknown => Setting::Unknown,
        };
        Self {
            authorization: match settings.authorization {
                native::Authorization::Granted => Authorization::Granted,
                native::Authorization::Denied => Authorization::Denied,
                native::Authorization::Undetermined => Authorization::Undetermined,
            },
            alerts: setting(settings.alerts),
            alert_style: match settings.alert_style {
                native::AlertStyle::None => AlertStyle::None,
                native::AlertStyle::Banner => AlertStyle::Banner,
                native::AlertStyle::Alert => AlertStyle::Alert,
                native::AlertStyle::Unknown => AlertStyle::Unknown,
            },
            notification_center: setting(settings.notification_center),
            sound: setting(settings.sound),
        }
    }
}

#[cfg(target_os = "macos")]
struct Mac(bboxy450_desktop_notifications_macos::Handle);
#[cfg(target_os = "macos")]
impl Notifier for Mac {
    fn authorize(&self) -> Result<Authorization, Error> {
        use bboxy450_desktop_notifications_macos::Authorization as Native;
        self.0
            .authorize()
            .map(|state| match state {
                Native::Granted => Authorization::Granted,
                Native::Denied => Authorization::Denied,
                Native::Undetermined => Authorization::Undetermined,
            })
            .map_err(native_error)
    }
    fn post(&self, n: &Notification) -> Result<(), Error> {
        self.0
            .post(&n.id.0, &n.title, &n.body)
            .map_err(native_error)
    }
    fn remove(&self, id: &NotificationId) -> Result<(), Error> {
        self.0.remove(&id.0).map_err(native_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(not(target_os = "macos"))]
    // keeps: spec desktop-notifications §4 — unsupported backends report the named research marker
    #[test]
    fn known_dummy_is_named_and_requires_research() {
        let error = match open(Box::new(|_| {})) {
            Err(error) => error,
            Ok(_) => panic!("dummy unexpectedly opened"),
        };
        assert!(matches!(
            error,
            Error::Unsupported {
                note: "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md",
                ..
            }
        ));
        assert!(matches!(
            settings(),
            Err(Error::Unsupported {
                note: "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md",
                ..
            })
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn native_settings_conversion_does_not_turn_scheduling_permission_into_alerts() {
        use bboxy450_desktop_notifications_macos as native;
        let actual = Settings::from(native::Settings {
            authorization: native::Authorization::Granted,
            alerts: native::Setting::Disabled,
            alert_style: native::AlertStyle::None,
            notification_center: native::Setting::Enabled,
            sound: native::Setting::NotSupported,
        });
        assert_eq!(
            actual,
            Settings {
                authorization: Authorization::Granted,
                alerts: Setting::Disabled,
                alert_style: AlertStyle::None,
                notification_center: Setting::Enabled,
                sound: Setting::NotSupported,
            }
        );
        let unknown = Settings::from(native::Settings {
            authorization: native::Authorization::Undetermined,
            alerts: native::Setting::Unknown,
            alert_style: native::AlertStyle::Unknown,
            notification_center: native::Setting::Unknown,
            sound: native::Setting::Unknown,
        });
        assert_eq!(unknown.alerts, Setting::Unknown);
        assert_eq!(unknown.alert_style, AlertStyle::Unknown);
        assert_eq!(unknown.notification_center, Setting::Unknown);
        assert_eq!(unknown.sound, Setting::Unknown);
    }
    // keeps: spec desktop-notifications §2 — dyn Notifier is a Send + Sync public port
    #[test]
    fn notifier_port_is_send_and_sync() {
        fn port<T: Send + Sync + ?Sized>() {}
        port::<dyn Notifier>();
    }
}
