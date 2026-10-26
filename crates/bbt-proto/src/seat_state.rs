//! Portable sidebar state. The mux maps observations into these values; clients
//! never infer process lifetime from harness activity.
use serde::{Deserialize, Serialize};

use crate::registry::SeatRecord;
pub use bbt_ui_model::{
    Activity, Grade, Lifecycle, Role, RoleClass, SeatId, SwarmId, WaitKind, Where,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GuiSeat {
    pub record: SeatRecord,
    pub state: SeatState,
    /// Full policy on an explicit read; small digest observations on ordinary state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attention: Option<serde_json::Value>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatState {
    pub generation: u64,
    pub process: ProcessStatus,
    #[serde(with = "activity")]
    pub activity: Activity,
    #[serde(with = "grade")]
    pub grade: Grade,
    pub title: String,
    pub detail: Option<String>,
    pub turn_open: bool,
    pub observed_ms: u64,
    pub subagents: Vec<SubagentRow>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubagentRow {
    pub id: String,
    pub title: String,
    #[serde(with = "activity")]
    pub activity: Activity,
    #[serde(with = "grade")]
    pub grade: Grade,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProcessStatus {
    Running { pid: u32 },
    Exited { status: i32 },
    ClosedGrace { until_ms: u64 },
    Retained,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnSpec {
    #[serde(with = "text_id")]
    pub swarm: SwarmId,
    pub harness_kind: String,
    pub name: Option<String>,
    pub role: Option<String>,
    pub cwd: String,
    #[serde(with = "location")]
    pub location: Where,
    pub resume: Option<String>,
    /// Captured at the pick. Never resolve an anchor from later GUI focus.
    #[serde(default)]
    pub placement: Option<Placement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Placement {
    NewTab {
        #[serde(with = "text_id")]
        tab: crate::TabId,
    },
    Split {
        #[serde(with = "text_id")]
        tab: crate::TabId,
        #[serde(with = "text_id")]
        target: SeatId,
        target_generation: u64,
        side: Side,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Right,
    Bottom,
}

#[cfg(test)]
mod placement_tests {
    use super::*;

    fn spec() -> SpawnSpec {
        SpawnSpec {
            swarm: SwarmId::from_u128(1),
            harness_kind: "zsh".into(),
            name: None,
            role: None,
            cwd: "/fixture".into(),
            location: Where::Local,
            resume: None,
            placement: None,
        }
    }

    // keeps: spec ctl-cli §1
    #[test]
    fn absent_placement_keeps_the_default_lane() {
        let expected = spec();
        let mut value = serde_json::to_value(&expected).unwrap();
        value.as_object_mut().unwrap().remove("placement");
        assert_eq!(
            serde_json::from_value::<SpawnSpec>(value).unwrap(),
            expected
        );
    }

    // keeps: spec ctl-cli §1
    #[test]
    fn placement_roundtrip_keeps_the_captured_tab_anchor_and_generation() {
        let mut expected = spec();
        expected.placement = Some(Placement::Split {
            tab: crate::TabId::from_u128(2),
            target: SeatId::from_u128(3),
            target_generation: 7,
            side: Side::Bottom,
        });
        let value = serde_json::to_value(&expected).unwrap();
        assert_eq!(
            value["placement"]["tab"],
            crate::TabId::from_u128(2).to_string()
        );
        assert_eq!(
            value["placement"]["target"],
            SeatId::from_u128(3).to_string()
        );
        assert_eq!(
            serde_json::from_value::<SpawnSpec>(value.clone()).unwrap(),
            expected
        );
        let mut invalid = value;
        invalid["placement"]["side"] = serde_json::json!("diagonal");
        assert!(serde_json::from_value::<SpawnSpec>(invalid).is_err());
    }

    // keeps: spec ctl-cli §1
    #[test]
    fn spawn_and_reply_keep_the_same_operation_id() {
        let op = ulid::Ulid::new();
        let action = crate::GuiAction::Spawn {
            id: op,
            spec: spec(),
        };
        let action: crate::GuiAction =
            serde_json::from_slice(&serde_json::to_vec(&action).unwrap()).unwrap();
        assert!(matches!(action, crate::GuiAction::Spawn { id, .. } if id == op));
        let reply = crate::GuiReply::Spawned {
            op,
            seat: SeatId::from_u128(4),
        };
        let reply: crate::GuiReply =
            serde_json::from_slice(&serde_json::to_vec(&reply).unwrap()).unwrap();
        assert!(
            matches!(reply, crate::GuiReply::Spawned { op: actual, seat } if actual == op && seat == SeatId::from_u128(4))
        );
    }
}

// UI-model deliberately has no serde dependency. Keep its wire adapters here,
// including canonical ULID text, rather than changing the lower model layer.
pub(crate) mod text_id {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::{fmt::Display, str::FromStr};
    pub fn serialize<T: Display, S: Serializer>(value: &T, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(value)
    }
    pub fn deserialize<'de, T, D>(d: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        String::deserialize(d)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
pub(crate) mod activity {
    use super::*;
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind", content = "waiting", rename_all = "snake_case")]
    enum Wire {
        Active,
        Idle,
        Waiting(Wait),
        Errored,
    }
    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    enum Wait {
        Permission,
        Question,
        Auth,
    }
    pub fn serialize<S: serde::Serializer>(value: &Activity, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Activity::Active => Wire::Active,
            Activity::Idle => Wire::Idle,
            Activity::Errored => Wire::Errored,
            Activity::Waiting(kind) => Wire::Waiting(match kind {
                WaitKind::Permission => Wait::Permission,
                WaitKind::Question => Wait::Question,
                WaitKind::Auth => Wait::Auth,
            }),
        }
        .serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Activity, D::Error> {
        Ok(match Wire::deserialize(d)? {
            Wire::Active => Activity::Active,
            Wire::Idle => Activity::Idle,
            Wire::Errored => Activity::Errored,
            Wire::Waiting(kind) => Activity::Waiting(match kind {
                Wait::Permission => WaitKind::Permission,
                Wait::Question => WaitKind::Question,
                Wait::Auth => WaitKind::Auth,
            }),
        })
    }
}
pub(crate) mod grade {
    use super::*;
    pub fn serialize<S: serde::Serializer>(value: &Grade, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(match value {
            Grade::Reported => "reported",
            Grade::Shim => "shim",
            Grade::Inferred => "inferred",
        })
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Grade, D::Error> {
        match String::deserialize(d)?.as_str() {
            "reported" => Ok(Grade::Reported),
            "shim" => Ok(Grade::Shim),
            "inferred" => Ok(Grade::Inferred),
            _ => Err(serde::de::Error::custom("unknown observation grade")),
        }
    }
}
pub(crate) mod location {
    use super::*;
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "kind", rename_all = "snake_case")]
    enum Wire {
        Local,
        Ssh { host: String },
        Cloud { provider: String, host: String },
    }
    pub fn serialize<S: serde::Serializer>(value: &Where, s: S) -> Result<S::Ok, S::Error> {
        match value {
            Where::Local => Wire::Local,
            Where::Ssh { host } => Wire::Ssh { host: host.clone() },
            Where::Cloud { provider, host } => Wire::Cloud {
                provider: provider.clone(),
                host: host.clone(),
            },
        }
        .serialize(s)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Where, D::Error> {
        Ok(match Wire::deserialize(d)? {
            Wire::Local => Where::Local,
            Wire::Ssh { host } => Where::Ssh { host },
            Wire::Cloud { provider, host } => Where::Cloud { provider, host },
        })
    }
}
