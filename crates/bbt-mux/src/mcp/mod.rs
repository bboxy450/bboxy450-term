//! MCP is a client of the admitted seat socket; logic remains the authority.

pub mod bridge;
pub(crate) mod journal;
pub(crate) mod tools;

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

pub(crate) fn success(op_id: Ulid, mut data: Value) -> Value {
    data["op_id"] = json!(op_id);
    json!({"content":[{"type":"text","text":data.to_string()}],"structuredContent":data,"isError":false})
}

pub(crate) fn error(op_id: Ulid, code: &str, message: &str) -> Value {
    let data = json!({"op_id":op_id,"error":{"code":code,"message":message}});
    json!({"content":[{"type":"text","text":data.to_string()}],"structuredContent":data,"isError":true})
}
