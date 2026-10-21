use std::time::Instant;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextField { pub value: String, pub selection: std::ops::Range<usize> }

#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    pub id: ShadowId,
    pub name: String,
    pub harness: HarnessId,
    pub role: Option<Role>,
    pub activity: Activity,
    pub grade: Grade,
    pub owner: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceKind { Presence, Audit }
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamItem {
    Message { id: MessageId, from: String, role: Option<Role>, foreign: bool, text: String,
        time: String, unread: bool, reply_to: Option<MessageId> },
    Presence { kind: PresenceKind, text: String, time: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct UndoRecord {
    pub tab: TabId,
    pub tab_index: usize,
    pub tab_name: String,
    pub shortcut: Option<u8>,
    pub expanded: bool,
    pub parent: Option<Parent>,
    pub process_generation: u64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ClosedStrip { pub seat: SeatId, pub until: Instant, pub undo: UndoRecord }

#[derive(Debug, Clone, PartialEq)]
pub struct Swarm {
    pub id: SwarmId,
    pub name: String,
    pub color: SeatColor,
    pub tabs: Vec<Tab>,
    pub closed: Vec<ClosedStrip>,
    pub shadows: Vec<Shadow>,
    pub stream: Vec<StreamItem>,
    pub renaming: Option<TextField>,
}
impl Swarm {
    pub fn new(id: SwarmId, name: impl Into<String>, tabs: Vec<Tab>) -> Self {
        Self { id, name: name.into(), tabs, color: SeatColor::Derived, closed: Vec::new(),
            shadows: Vec::new(), stream: Vec::new(), renaming: None }
    }
}
