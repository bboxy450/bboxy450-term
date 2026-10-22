//! Stable serializable seat metadata; independent of registry storage.
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub class: Class,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Oracle,
    Worker,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Location {
    Local,
    Ssh { host: String },
    Cloud { provider: String, host: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub swarm: Ulid,
    pub tab: Ulid,
    pub split_path: Vec<usize>,
    pub ratios: Vec<f64>,
    pub layout_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CloseRecord {
    pub operation: Ulid,
    pub revision: u64,
    pub deadline_ms: u64,
    pub position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lifecycle {
    Open,
    ClosedGrace,
    ClosedRetained,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeatRecord {
    pub id: Ulid,
    pub swarm: Ulid,
    pub name: String,
    pub harness_kind: String,
    pub harness_session_id: Option<String>,
    pub role: Role,
    #[serde(rename = "where")]
    pub location: Location,
    pub muted: bool,
    pub generation: u64,
    pub lifecycle: Lifecycle,
    pub lifecycle_revision: u64,
    pub close: Option<CloseRecord>,
    pub tasks: Vec<Ulid>,
}

impl SeatRecord {
    pub fn new(
        id: Ulid,
        swarm: Ulid,
        name: impl Into<String>,
        harness_kind: impl Into<String>,
    ) -> Self {
        Self {
            id,
            swarm,
            name: name.into(),
            harness_kind: harness_kind.into(),
            harness_session_id: None,
            role: Role {
                name: "executor".into(),
                class: Class::Worker,
            },
            location: Location::Local,
            muted: false,
            generation: 1,
            lifecycle: Lifecycle::Open,
            lifecycle_revision: 0,
            close: None,
            tasks: Vec::new(),
        }
    }
}
