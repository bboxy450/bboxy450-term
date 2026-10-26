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
    }
    // keeps: spec desktop-notifications §2 — dyn Notifier is a Send + Sync public port
    #[test]
    fn notifier_port_is_send_and_sync() {
        fn port<T: Send + Sync + ?Sized>() {}
        port::<dyn Notifier>();
    }
}
