//! Shared local composer/seat API. Identity and class come from the admitted
//! connection, never from fields supplied by a peer.
use super::{
    ledger::{Page, Receipt, StoredMessage},
    tree::{Filter, Node, Rate, Right},
    wire::SendRequest,
};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Send {
        message: SendRequest,
    },
    List {
        thread: Option<Ulid>,
        channel: Option<String>,
        after: u64,
        limit: usize,
    },
    Receipts {
        message: Ulid,
        after: u64,
        limit: usize,
    },
    Acknowledge {
        message: Ulid,
        agent: Option<String>,
    },
    Ls {
        path: String,
        after: Option<String>,
        limit: usize,
    },
    Subscribe {
        folder: String,
        name: String,
        filter: Filter,
        rate: Rate,
    },
    Unsubscribe {
        path: String,
    },
    Link {
        node: Node,
    },
    Pause {
        path: String,
        paused: bool,
    },
    Grant {
        path: String,
        seat: Ulid,
        rights: Vec<Right>,
    },
    Mute {
        seat: Ulid,
        muted: bool,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Response {
    Stored {
        message: Box<StoredMessage>,
    },
    Messages {
        page: Page<StoredMessage>,
    },
    Receipts {
        page: Page<Receipt>,
    },
    Tree {
        nodes: Vec<Node>,
        next: Option<String>,
    },
    Muted {
        notice: Option<Ulid>,
    },
    Applied,
}
