// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The one admitted GUI connection multiplexes state and watched view updates.
//!
//! Dispatch by noun (decision #320): each verb family's module owns its request and reply
//! types and lists its own `GuiAction` and `GuiReply` variants in one `macro_rules!` list,
//! and `flat_enum` splices those lists into the two flat enums below. Adding a verb edits
//! its noun's file only. The wire is the flat enums' own derive, so it is unchanged by where
//! a variant is listed (pinned by `tests/gui_wire_golden.rs`), and a variant name listed by
//! two nouns is a duplicate-variant compile error. Every call site keeps `GuiAction::X`.
/// Terminal snapshots for supported desktop grids exceed the MCP/control
/// request bound. GUI replies have their own bounded transport budget.
pub const MAX_REPLY_BYTES: usize = 64 * 1024 * 1024;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::seat_state::{GuiSeat, SpawnSpec, text_id};
use crate::{SeatId, Update, View};

mod app;
mod debug;
mod events;
mod host;
mod identity;
mod message;
mod plugin;
mod record;
mod roles;
mod seat;
mod swarm;
mod task;
mod ui;
use self::app::{app_actions, app_replies};
use self::debug::{debug_actions, debug_replies};
use self::events::{events_actions, events_replies};
use self::host::{host_actions, host_replies};
use self::identity::{identity_actions, identity_replies};
use self::message::{message_actions, message_replies};
use self::plugin::{plugin_actions, plugin_replies};
use self::record::{record_actions, record_replies};
use self::roles::{role_actions, role_replies};
use self::seat::{seat_actions, seat_replies};
pub use self::swarm::{SWARM_METADATA_VERSION, SWARM_WORKSPACE_VERSION, SwarmScope};
use self::swarm::{swarm_actions, swarm_replies};
use self::task::{task_actions, task_replies};
use self::ui::{ui_actions, ui_replies};
pub use self::{
    app::*, debug::*, events::*, identity::*, plugin::*, record::*, roles::*, seat::*, ui::*,
};

pub type Row = Vec<bbt_core::Cell>;

/// The admitted GUI schema, including explicit pending message grades and logical operators.
pub const VERSION: u16 = 3;

#[derive(Debug, Serialize, Deserialize)]
pub struct GuiRequest {
    pub version: u16,
    pub capability: [u8; 32],
    pub action: GuiAction,
}

/// Splices per-noun variant lists into one flat enum. Each list is a `macro_rules!` that takes
/// this macro's name, the lists still to run and the variants so far, appends its own variants
/// and calls back; with no lists left, the enum is emitted with its attributes.
macro_rules! flat_enum {
    ($(#[$meta:meta])* pub enum $name:ident from [$($list:ident),* $(,)?]) => {
        flat_enum!(@next [$($list)*] [] $(#[$meta])* pub enum $name);
    };
    (@next [] [$($variants:tt)*] $(#[$meta:meta])* pub enum $name:ident) => {
        $(#[$meta])*
        pub enum $name {
            $($variants)*
        }
    };
    (@next [$list:ident $($rest:ident)*] [$($variants:tt)*] $($head:tt)*) => {
        $list!(flat_enum, [$($rest)*] [$($variants)*] $($head)*);
    };
}

/// Replies every noun shares.
macro_rules! common_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Ok {
                op: Ulid,
            },
            Error {
                op: Ulid,
                reason: String,
            },
        ] $($head)*);
    };
}

flat_enum! {
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body")]
    pub enum GuiAction from [ui_actions, app_actions, debug_actions, plugin_actions, identity_actions, host_actions, seat_actions, swarm_actions, events_actions, message_actions, task_actions, record_actions, role_actions]
}

flat_enum! {
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "body")]
#[allow(
    clippy::large_enum_variant,
    reason = "keep the decided C1 shared API; transport bounds each message"
)]
    pub enum GuiReply from [ui_replies, app_replies, debug_replies, plugin_replies, identity_replies, host_replies, seat_replies, swarm_replies, events_replies, message_replies, task_replies, record_replies, role_replies, common_replies]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{registry::SeatRecord, seat_state::*};
    // keeps: spec ctl-cli §1
    #[test]
    fn terminal_selection_controls_are_bounded_and_private_receipt_is_explicit() {
        for request in [
            UiRequest::TerminalSelect {
                from: [1, 2],
                to: [300, 400],
                shift: false,
            },
            UiRequest::Copy,
            UiRequest::ClipboardRead,
        ] {
            assert!(request.validate().is_ok());
            assert_eq!(
                serde_json::from_value::<UiRequest>(serde_json::to_value(&request).unwrap())
                    .unwrap(),
                request
            );
        }
        assert!(
            UiRequest::TerminalSelect {
                from: [32769, 0],
                to: [1, 2],
                shift: true
            }
            .validate()
            .is_err()
        );
        assert!(UiRequest::Key(vec!["cmd-c".into()]).validate().is_ok());
    }
    #[test]
    fn identity_status_is_portable_and_contains_only_state_and_reason() {
        for (state, name) in [
            (GuiIdentityState::LocalDev, "local-dev"),
            (GuiIdentityState::Pending, "pending"),
            (GuiIdentityState::Certified, "certified"),
            (GuiIdentityState::Held, "held"),
        ] {
            let identity = GuiIdentity {
                state,
                reason: Some("Keychain unavailable".into()),
            };
            let value = serde_json::to_value(GuiReply::Identity(identity.clone())).unwrap();
            assert_eq!(value["body"]["state"], name);
            assert_eq!(value["body"].as_object().unwrap().len(), 2);
            let GuiReply::Identity(decoded) = serde_json::from_value(value).unwrap() else {
                panic!("identity reply changed kind")
            };
            assert_eq!(identity, decoded);
        }
    }
    // keeps: spec ctl-cli §1
    #[test]
    fn gui_state_round_trips_portable_enums_without_a_mux_dependency() {
        for activity in [
            Activity::Active,
            Activity::Idle,
            Activity::Errored,
            Activity::Waiting(WaitKind::Permission),
            Activity::Waiting(WaitKind::Question),
            Activity::Waiting(WaitKind::Auth),
        ] {
            for grade in [Grade::Reported, Grade::Shim, Grade::Inferred] {
                for process in [
                    ProcessStatus::Running { pid: 7 },
                    ProcessStatus::Exited { status: 137 },
                    ProcessStatus::ClosedGrace { until_ms: 99 },
                    ProcessStatus::Retained,
                ] {
                    let state = SeatState {
                        generation: 1,
                        process,
                        activity,
                        grade,
                        title: "zsh".into(),
                        detail: None,
                        turn_open: false,
                        observed_ms: 3,
                        subagents: vec![],
                    };
                    let seat = GuiSeat {
                        attention: None,
                        record: SeatRecord::new(Ulid::from(1), Ulid::from(2), "zsh", "zsh"),
                        state,
                    };
                    let wire = serde_json::to_vec(&GuiReply::Seat(seat.clone())).unwrap();
                    let GuiReply::Seat(decoded) = serde_json::from_slice(&wire).unwrap() else {
                        panic!()
                    };
                    assert_eq!(decoded, seat);
                }
            }
        }
    }
    // keeps: spec messaging-attention §7 MA-6 — older seat replies omit the optional view
    #[test]
    fn attention_view_is_optional_and_round_trips_on_an_explicit_seat_read() {
        let value = serde_json::json!({
            "record":SeatRecord::new(Ulid::from(1), Ulid::from(2), "receiver", "zsh"),
            "state":{"generation":1,"process":{"kind":"retained"},"activity":{"kind":"idle"},
                "grade":"reported","title":"fixture","detail":null,"turn_open":false,
                "observed_ms":0,"subagents":[]}
        });
        let mut seat: GuiSeat = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(seat.attention, None);
        assert_eq!(serde_json::to_value(&seat).unwrap(), value);
        seat.attention =
            Some(serde_json::json!({"policy":{"rules":[]},"defaults":{"event":"inbox"}}));
        let wire = serde_json::to_value(&seat).unwrap();
        assert_eq!(wire["attention"], seat.attention.clone().unwrap());
        assert_eq!(serde_json::from_value::<GuiSeat>(wire).unwrap(), seat);
    }
    // keeps: spec ctl-cli §1
    #[test]
    fn close_readback_is_optional_for_older_app_status_replies() {
        let mut wire = serde_json::json!({
            "running":true,"mode":"headless","unattended":false,
            "backend_ready":true,"gui_ready":true,"events_ready":true,
            "custody_backend":"memory","generation":1,"frame_seq":1,"windows":0
        });
        let legacy: AppStatus = serde_json::from_value(wire.clone()).unwrap();
        assert!(legacy.close.is_null());
        wire["close"] = serde_json::json!({"slots":[],"grace":[],"notices":[],"complete":true});
        let current: AppStatus = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(current).unwrap(), wire);
    }
    // keeps: spec ctl-cli §1
    #[test]
    fn gui_ids_keep_canonical_ulid_text_and_reject_malformed_ids() {
        let seat = SeatId::from_u128(42);
        let value = serde_json::to_value(GuiAction::Unwatch { seat }).unwrap();
        assert_eq!(value["body"]["seat"], seat.to_string());
        let decoded: GuiAction = serde_json::from_value(value.clone()).unwrap();
        assert!(matches!(decoded,GuiAction::Unwatch {seat:id} if id==seat));
        let mut bad = value;
        bad["body"]["seat"] = serde_json::json!("not-a-seat");
        assert!(serde_json::from_value::<GuiAction>(bad).is_err());
    }
    // keeps: spec ctl-cli §1
    #[test]
    fn input_and_resize_require_an_explicit_displayed_generation() {
        let seat = SeatId::from_u128(42);
        for action in [
            GuiAction::Input {
                seat,
                generation: 7,
                id: Ulid::new(),
                bytes: b"hi\r".to_vec(),
            },
            GuiAction::Resize {
                seat,
                generation: 7,
                cols: 80,
                rows: 24,
            },
        ] {
            let mut value = serde_json::to_value(action).unwrap();
            assert_eq!(value["body"]["generation"], 7);
            assert!(serde_json::from_value::<GuiAction>(value.clone()).is_ok());
            value["body"].as_object_mut().unwrap().remove("generation");
            assert!(serde_json::from_value::<GuiAction>(value).is_err());
        }
    }
}
#[cfg(test)]
mod ui_tests {
    use super::*;
    // keeps: spec ctl-cli §1
    #[test]
    fn scroll_rows_stay_unsigned_and_live_positions_can_be_unindexed() {
        let seat = Ulid::new();
        for motion in [
            UiScroll::Top,
            UiScroll::Bottom,
            UiScroll::PageUp,
            UiScroll::PageDown,
            UiScroll::Row(u64::MAX),
        ] {
            let request = UiRequest::Scroll { seat, motion };
            assert!(request.validate().is_ok());
            let value = serde_json::to_value(&request).unwrap();
            assert_eq!(serde_json::from_value::<UiRequest>(value).unwrap(), request);
        }
        assert!(
            UiRequest::Scroll {
                seat: Ulid::nil(),
                motion: UiScroll::Top
            }
            .validate()
            .is_err()
        );
        let legacy: UiPane = serde_json::from_value(serde_json::json!({"seat":seat,"rect":[0.,0.,80.,24.],"cols":80,"rows":24,"focused":true})).unwrap();
        assert_eq!(legacy.scroll, UiScrollPosition::default());
        assert_eq!(
            serde_json::to_value(UiScrollPosition::default()).unwrap(),
            serde_json::json!({"top_row":null,"live":true})
        );
        assert!(
            serde_json::from_value::<UiScroll>(serde_json::json!({"kind":"row","body":-1}))
                .is_err()
        );
    }
    // keeps: spec ctl-cli §1
    #[test]
    fn ui_requests_keep_their_scope_and_reject_unbounded_input() {
        let request = UiRequest::Key(vec!["cmd-t".into(), "esc".into()]);
        assert_eq!(
            serde_json::from_value::<UiRequest>(serde_json::to_value(&request).unwrap()).unwrap(),
            request
        );
        assert!(request.validate().is_ok());
        for request in [
            UiRequest::Key(vec![]),
            UiRequest::Key(vec!["cmd-t".into(); 33]),
            UiRequest::Key(vec!["text into terminal".into()]),
            UiRequest::Focus { seat: Ulid::nil() },
            UiRequest::Open(UiTarget::Docs("x\n".into())),
        ] {
            assert!(request.validate().is_err());
        }
    }
}
#[cfg(test)]
mod spawn_pick_tests {
    use super::*;
    // keeps: spec ctl-cli §1
    #[test]
    fn spawn_pick_wire_is_bounded_and_old_layouts_keep_revision_unknown() {
        let pick = UiSpawn {
            harness: "zsh".into(),
            name: None,
            role: None,
            cwd: "/fixture".into(),
            swarm: None,
            place: SpawnPlace::Right,
        };
        let request = UiRequest::Spawn(Box::new(pick.clone()));
        request.validate().unwrap();
        assert_eq!(
            serde_json::from_value::<UiRequest>(serde_json::to_value(&request).unwrap()).unwrap(),
            request
        );
        let mut oversized = pick.clone();
        oversized.cwd = "x".repeat(4097);
        assert!(UiRequest::Spawn(Box::new(oversized)).validate().is_err());
        let mut invalid = pick;
        invalid.swarm = Some(Ulid::nil());
        assert!(UiRequest::Spawn(Box::new(invalid)).validate().is_err());
        let old:UiLayout=serde_json::from_value(serde_json::json!({"frame_seq":4,"size":[100,100],"swarm":null,"tab":null,"focused":null,"launcher_open":false,"page":null,"document":null,"panes":[],"tree":[]})).unwrap();
        assert_eq!(old.layout_revision, None);
        assert_eq!(old.spawned, None);
    }
}
