//! MCP is a client of the admitted seat socket; logic remains the authority.

pub mod bridge;
pub(crate) mod journal;
pub(crate) mod messaging;
pub(crate) mod seats;
pub(crate) mod tools;
pub(crate) mod work_items;

pub(crate) fn mutation(tool: &str) -> bool {
    messaging::mutation(tool) || work_items::mutation(tool) || seats::mutation(tool)
}

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use ulid::Ulid;

use crate::messaging::wire::IssuerKind;
use crate::registry::Role;

pub use bridge::run_cli;

pub const MAX_ARGS: usize = 64 * 1024;
pub const MAX_RESULT: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Call {
    pub op_id: Ulid,
    pub tool: String,
    pub args: Value,
}
impl Call {
    pub(crate) fn validate(&self) -> crate::store::Result<()> {
        if self.op_id == Ulid::nil()
            || self.tool.is_empty()
            || self.tool.len() > 64
            || !self
                .tool
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
            || !self.args.is_object()
            || serde_json::to_vec(&self.args)?.len() > MAX_ARGS
        {
            return Err(crate::store::invalid("malformed or oversized control call"));
        }
        Ok(())
    }
}

/// Resolved by logic from admission and current policy, never from tool input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeatActor {
    pub seat_id: Ulid,
    pub process_generation: u64,
    pub swarm_id: Ulid,
    pub role: Role,
    pub grant_epoch: u64,
    pub issuer_kind: IssuerKind,
}

/// Logic resolves authority; GUI input never supplies an operator identity.
/// Seat authority retains its existing fields and untagged journal encoding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Actor {
    Seat(SeatActor),
    Operator {
        device_id: Ulid,
        /// Which door the operator came through (decision #260): the window or `ctl`.
        /// Omitted for the window, so existing journal records keep their bytes.
        #[serde(default, skip_serializing_if = "Door::is_gui")]
        door: Door,
    },
}
/// The operator's two doors on `gui.sock` (decision #260), told apart on `account/control`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Door {
    /// The app's own window.
    #[default]
    Gui,
    /// `bboxy450-term ctl`, opened by `--ctl`.
    Ctl,
}
impl Door {
    pub fn is_gui(&self) -> bool {
        *self == Self::Gui
    }
}
impl Actor {
    pub(crate) fn class(&self) -> crate::registry::Class {
        match self {
            Self::Seat(actor) => actor.role.class,
            Self::Operator { .. } => crate::registry::Class::Oracle,
        }
    }
    pub(crate) fn seat(&self) -> Option<&SeatActor> {
        match self {
            Self::Seat(actor) => Some(actor),
            Self::Operator { .. } => None,
        }
    }
    /// The same authority for replay: a seat by its whole actor; the operator by device,
    /// whichever door it came through (decision #31: any operator connection on the device).
    pub(crate) fn same_authority(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Operator { device_id: a, .. }, Self::Operator { device_id: b, .. }) => a == b,
            _ => self == other,
        }
    }
    pub(crate) fn reference(&self) -> String {
        match self {
            Self::Seat(actor) => format!("seat:{}", actor.seat_id),
            Self::Operator { device_id, .. } => format!("operator:{device_id}"),
        }
    }
}

pub(crate) type Principal = Actor;

impl From<&SeatActor> for Actor {
    fn from(actor: &SeatActor) -> Self {
        Self::Seat(actor.clone())
    }
}
impl From<&Actor> for Actor {
    fn from(principal: &Actor) -> Self {
        principal.clone()
    }
}

pub(crate) fn success(op_id: Ulid, mut data: Value) -> Value {
    data["op_id"] = json!(op_id);
    json!({"content":[{"type":"text","text":data.to_string()}],"structuredContent":data,"isError":false})
}

pub(crate) fn error(op_id: Ulid, code: &str, message: &str) -> Value {
    let data = json!({"op_id":op_id,"status":if code=="unknown"{"unknown"}else{"rejected"},"error":{"code":code,"message":message}});
    json!({"content":[{"type":"text","text":data.to_string()}],"structuredContent":data,"isError":true})
}
