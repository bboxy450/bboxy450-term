//! App lifecycle relayed to the GUI owner: `ctl app …`.
use super::*;

/// Lifecycle operations relayed to the attached GUI owner, including headless.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppRequest {
    /// Read the GUI owner's actual readiness, without any capability material.
    Status,
    /// Raise the existing GUI for a refused second launch.
    Activate,
    /// Read the explicitly selected test notification backend, never the OS.
    Notifications,
    /// On-demand sample by the GUI IO owner, with no layout or paint work.
    HistoryMetrics,
    /// Acknowledge the request before beginning orderly app shutdown.
    Stop,
}

/// Public app readiness. No process credential or endpoint capability belongs here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatus {
    pub running: bool,
    pub mode: String,
    pub unattended: bool,
    pub backend_ready: bool,
    pub gui_ready: bool,
    pub events_ready: bool,
    pub custody_backend: String,
    pub generation: u64,
    pub frame_seq: u64,
    pub windows: usize,
    /// Bounded readback of the retained close/grace UI, absent on older hosts.
    #[serde(default)]
    pub close: serde_json::Value,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub notifications: serde_json::Value,
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub history: serde_json::Value,
}

/// Native GUI discovery, admitted by this run's private capability, never ctl discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiHello {
    pub run_dir: std::path::PathBuf,
    pub account_root: Option<String>,
}

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! app_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// Discover or claim the native GUI attachment after the former GUI died.
            GuiHello { id: Ulid, claim: bool },
            /// Ask the app owner through the same admitted operator endpoint.
            App {
                id: Ulid,
                request: AppRequest,
            },
            /// Only the GUI owner may answer an app lifecycle relay.
            AppReply {
                id: Ulid,
                status: AppStatus,
            },
        ] $($head)*);
    };
}
pub(super) use app_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! app_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// The account/run bound to this admitted native GUI socket.
            GuiHello { op: Ulid, hello: GuiHello },
            /// A lifecycle query for the attached GUI owner.
            AppAsk {
                op: Ulid,
                request: AppRequest,
            },
            /// The stop request was written back to its requester; shutdown may now begin.
            AppStopping {
                op: Ulid,
            },
            /// The app owner's public readiness snapshot.
            App {
                op: Ulid,
                status: AppStatus,
            },
        ] $($head)*);
    };
}
pub(super) use app_replies;
