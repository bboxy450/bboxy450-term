// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! App lifecycle relayed to the GUI owner: `ctl app …`.
use super::*;

pub(super) fn is_false(value: &bool) -> bool {
    !*value
}

/// Lifecycle operations relayed to the attached GUI owner, including headless.
/// Stable destination and utility accordion identities, independent of focus.
pub const SIDEBAR_SECTIONS: [&str; 4] = [
    "providers/Hosts",
    "providers/Cloud",
    "panels/Tasks",
    "panels/Chat",
];
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

/// Identity of the GUI process that actually produced readiness, separate from backend generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuiProcess {
    pub pid: u32,
    pub instance: Ulid,
    pub source_revision: String,
}

/// Public app readiness. No process credential or endpoint capability belongs here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatus {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gui: Option<GuiProcess>,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuiHello {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowSnapshot>,
    pub run_dir: std::path::PathBuf,
    pub account_root: Option<String>,
}

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! app_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            /// Discover or claim the native GUI attachment after the former GUI died.
            GuiHello {
                id: Ulid,
                claim: bool,
                /// A restricted current-attention connection; never a GUI owner.
                #[serde(default, skip_serializing_if = "app::is_false")]
                observer: bool,
            },
            /// Coalesced native GUI frame data; no terminal bytes or authority.
            WindowSave { id: Ulid, epoch: Ulid, state: WindowState },
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
            /// Durable completion for a native frame snapshot.
            WindowSaved { op: Ulid, error: Option<String> },
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

pub const WINDOW_STATE_VERSION: u16 = 5;

/// A view preference only; process ownership is independent of this face.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneFace {
    Terminal,
    HarnessRich,
}

/// Replay cursor of a disposable rich subscriber; never input authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RichCursor {
    pub session: Ulid,
    pub generation: u64,
    pub epoch: u32,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Durable layout and view preferences; seat authority remains in the registry.
pub struct WindowState {
    pub version: u16,
    pub size: [f32; 2],
    pub sidebar_width: f32,
    pub sidebar_scroll: f32,
    pub sidebar_shown: bool,
    pub open_swarms: Vec<Ulid>,
    /// Closed non-swarm sections; an absent field keeps older windows expanded.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sidebar_sections_closed: Vec<String>,
    pub swarms: Vec<WindowSwarm>,
    pub stage: WindowStage,
    pub scroll: Vec<WindowScroll>,
    /// Explicit face choices; absent entries follow native harness announcement.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub pane_faces: std::collections::BTreeMap<Ulid, PaneFace>,
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub rich_cursors: std::collections::BTreeMap<Ulid, RichCursor>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowSwarm {
    pub id: Ulid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workdir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_environment: Option<Vec<(String, String)>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_directories: Vec<String>,
    pub tabs: Vec<WindowTab>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowTab {
    /// Additive pane content. The split tree keeps its existing stable leaf ids.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub canvases: std::collections::BTreeMap<Ulid, super::UiCanvasSource>,
    pub id: Ulid,
    pub name: String,
    pub shortcut: Option<u8>,
    pub expanded: bool,
    pub seats: Vec<Ulid>,
    pub tree: WindowTree,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WindowTree {
    Seat {
        id: Ulid,
    },
    Split {
        row: bool,
        ratio: f32,
        a: Box<Self>,
        b: Box<Self>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowStage {
    pub swarm: Option<Ulid>,
    pub tab: Option<Ulid>,
    pub focused: Option<Ulid>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WindowScroll {
    pub seat: Ulid,
    pub top_row: Option<u64>,
    pub live: bool,
}
/// An epoch fences writes from a GUI which disconnected during a database job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowSnapshot {
    pub epoch: Ulid,
    pub state: Option<WindowState>,
}

#[cfg(test)]
mod sidebar_state_compatibility {
    use super::*;

    #[test]
    fn pre_accordion_window_schemas_default_provider_sections_to_open() {
        for version in [1, 2, 3] {
            let state: WindowState = serde_json::from_value(serde_json::json!({
                "version": version,
                "size": [1280., 720.],
                "sidebar_width": 300.,
                "sidebar_scroll": 0.,
                "sidebar_shown": true,
                "open_swarms": [],
                "swarms": [],
                "stage": {"swarm": null, "tab": null, "focused": null},
                "scroll": []
            }))
            .unwrap();
            assert!(state.sidebar_sections_closed.is_empty());
            assert!(
                serde_json::to_value(&state)
                    .unwrap()
                    .get("sidebar_sections_closed")
                    .is_none()
            );
        }
    }
}

#[cfg(test)]
mod canvas_state_compatibility {
    use super::*;

    // Captured seat-only WindowTab shape from the version-5 schema preceding
    // canvas support; this fixture deliberately has no new fields.
    fn seat_only() -> serde_json::Value {
        serde_json::json!({
            "id": "00000000000000000000000001",
            "name": "kept terminal",
            "shortcut": 1,
            "expanded": true,
            "seats": ["00000000000000000000000002"],
            "tree": {"kind": "seat", "id": "00000000000000000000000002"}
        })
    }

    #[test]
    fn canvas_additive_state_preserves_seat_only_tab_fields() {
        let old = seat_only();
        let decoded: WindowTab = serde_json::from_value(old.clone()).unwrap();
        assert!(decoded.canvases.is_empty());
        assert_eq!(serde_json::to_value(decoded).unwrap(), old);
    }

    #[test]
    fn canvas_seat_only_window_serializes_byte_identically_to_v5() {
        // Frozen version-5 serializer output: absent optional fields stay absent.
        const OLD: &[u8] = br#"{"version":5,"size":[1280.0,720.0],"sidebar_width":300.0,"sidebar_scroll":0.0,"sidebar_shown":true,"open_swarms":["00000000000000000000000003"],"swarms":[{"id":"00000000000000000000000003","tabs":[{"id":"00000000000000000000000001","name":"kept terminal","shortcut":1,"expanded":true,"seats":["00000000000000000000000002"],"tree":{"kind":"seat","id":"00000000000000000000000002"}}]}],"stage":{"swarm":"00000000000000000000000003","tab":"00000000000000000000000001","focused":"00000000000000000000000002"},"scroll":[]}"#;
        let state: WindowState=serde_json::from_slice(OLD).unwrap();
        assert_eq!(serde_json::to_vec(&state).unwrap(),OLD);
        assert!(state.swarms[0].tabs[0].canvases.is_empty());
    }

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct LegacyWindowTab {
        id: Ulid,
        name: String,
        shortcut: Option<u8>,
        expanded: bool,
        seats: Vec<Ulid>,
        tree: WindowTree,
    }

    #[test]
    fn canvas_downgrade_refuses_canvas_tab_but_preserves_seat_only_format() {
        // Older GUIs strictly decode persisted tabs during attachment. A canvas
        // field refuses that attachment; this does not run an older binary.
        let old = seat_only();
        let old_bytes = serde_json::to_vec(&old).unwrap();
        let loaded: LegacyWindowTab = serde_json::from_slice(&old_bytes).unwrap();
        assert_eq!(serde_json::to_value(loaded).unwrap(), old);

        let mut canvas = old;
        canvas["canvases"] = serde_json::json!({
            "00000000000000000000000002": {
                "root": "/private/canvas-fixture/.bboxy",
                "swarm": "00000000000000000000000003",
                "id": "00000000000000000000000004",
                "title": "Kept canvas"
            }
        });
        let bytes = serde_json::to_vec(&canvas).unwrap();
        let current: WindowTab = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(current.canvases.len(), 1);
        let error = serde_json::from_slice::<LegacyWindowTab>(&bytes)
            .err()
            .unwrap();
        assert!(error.to_string().contains("unknown field `canvases`"));

    }
}
