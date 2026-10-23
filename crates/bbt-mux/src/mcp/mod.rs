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
pub struct Actor {
    pub seat_id: Ulid,
    pub process_generation: u64,
    pub swarm_id: Ulid,
    pub role: Role,
    pub grant_epoch: u64,
    pub issuer_kind: IssuerKind,
}

/// Logic resolves authority; GUI input never supplies an operator identity.
/// Untagged seat encoding retains the existing journal's on-disk Actor shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum Principal {
    Seat(Actor),
    Operator { device_id: Ulid },
}
impl From<&Actor> for Principal {
    fn from(actor: &Actor) -> Self {
        Self::Seat(actor.clone())
    }
}
impl From<&Principal> for Principal {
    fn from(principal: &Principal) -> Self {
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
