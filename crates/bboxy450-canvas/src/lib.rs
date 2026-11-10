// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! C0 is a single writer, contiguous sequence starting at one. Replay rejects
//! reordered/duplicate operations and author changes; no convergence is claimed.
mod camera;
pub use camera::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt,
    io::{self, BufRead, Write},
};

pub type ElementId = String;
pub const MAX_ELEMENTS: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanvasRecord {
    pub title: String,
    pub swarm: String,
    pub tags: Vec<String>,
    pub creator: String,
    pub task: Option<String>,
}

/// Top-left and extent in world units. Extents must be positive and finite.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bounds {
    pub origin: [f64; 2],
    pub size: [f64; 2],
}
impl Bounds {
    pub fn center(self) -> [f64; 2] {
        std::array::from_fn(|i| self.origin[i] + self.size[i] / 2.)
    }
    fn valid(self) -> bool {
        self.origin.iter().all(|n| n.is_finite())
            && self.size.iter().all(|n| n.is_finite() && *n > 0.)
            && (0..2).all(|i| (self.origin[i] + self.size[i]).is_finite())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shape {
    Box,
    Rounded,
    Ellipse,
    Diamond,
    Cylinder,
    Actor,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextRole {
    Heading,
    Paragraph,
    Note,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Service,
    Store,
    Queue,
    Person,
    External,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Normal,
    Waiting,
    Error,
    Done,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Content {
    Shape {
        bounds: Bounds,
        shape: Shape,
        label: String,
    },
    Connector {
        from: ElementId,
        to: ElementId,
        label: Option<String>,
        arrowheads: [bool; 2],
        dashed: bool,
    },
    Text {
        bounds: Bounds,
        text: String,
        role: TextRole,
    },
    Frame {
        bounds: Bounds,
        name: String,
        members: Vec<ElementId>,
    },
}
impl Content {
    pub fn bounds(&self) -> Option<Bounds> {
        match self {
            Self::Shape { bounds, .. } | Self::Text { bounds, .. } | Self::Frame { bounds, .. } => {
                Some(*bounds)
            }
            Self::Connector { .. } => None,
        }
    }
    fn references(&self) -> Vec<&str> {
        match self {
            Self::Connector { from, to, .. } => vec![from, to],
            Self::Frame { members, .. } => members.iter().map(String::as_str).collect(),
            _ => vec![],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Element {
    pub id: ElementId,
    pub tags: Vec<String>,
    pub kind: Option<Kind>,
    #[serde(default)]
    pub emphasis: bool,
    #[serde(default)]
    pub status: Status,
    pub content: Content,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Add {
        element: Element,
    },
    /// Replaces the named element, preserving its identity.
    Change {
        element: Element,
    },
    Remove {
        id: ElementId,
    },
    /// Adds one tag, idempotently. Change replaces the complete tag set.
    Tag {
        id: ElementId,
        tag: String,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Op {
    pub author: String,
    pub seq: u64,
    pub at: u64,
    pub action: Action,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplayError {
    Sequence { expected: u64, actual: u64 },
    Author,
    UnknownId(ElementId),
    DuplicateId(ElementId),
    ReferencedId(ElementId),
    InvalidElement(ElementId),
    InvalidTag,
    ElementCap { cap: usize },
}
impl fmt::Display for ReplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for ReplayError {}

/// Only replay can construct a nonempty canvas; callers cannot mutate its state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Canvas {
    elements: BTreeMap<ElementId, Element>,
}
impl Canvas {
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = &Element> {
        self.elements.values()
    }
    pub fn get(&self, id: &str) -> Option<&Element> {
        self.elements.get(id)
    }
    pub fn replay(ops: &[Op]) -> Result<Self, ReplayError> {
        let mut canvas = Self::default();
        for (index, op) in ops.iter().enumerate() {
            let expected = index as u64 + 1;
            if op.seq != expected {
                return Err(ReplayError::Sequence {
                    expected,
                    actual: op.seq,
                });
            }
            if op.author.is_empty() || op.author != ops[0].author {
                return Err(ReplayError::Author);
            }
            match &op.action {
                Action::Add { element } | Action::Change { element } => {
                    let exists = canvas.elements.contains_key(&element.id);
                    if matches!(op.action, Action::Add { .. }) {
                        if exists {
                            return Err(ReplayError::DuplicateId(element.id.clone()));
                        }
                        if canvas.elements.len() == MAX_ELEMENTS {
                            return Err(ReplayError::ElementCap { cap: MAX_ELEMENTS });
                        }
                    } else if !exists {
                        return Err(ReplayError::UnknownId(element.id.clone()));
                    }
                    canvas.validate(element)?;
                    // An endpoint cannot be replaced by a connector: references
                    // always resolve directly to an element with world bounds.
                    if element.content.bounds().is_none() && canvas.elements.values().any(|e| {
                        matches!(&e.content, Content::Connector { from, to, .. } if from == &element.id || to == &element.id)
                    }) { return Err(ReplayError::ReferencedId(element.id.clone())); }
                    let mut element = element.clone();
                    element.tags.sort();
                    element.tags.dedup();
                    canvas.elements.insert(element.id.clone(), element);
                }
                Action::Remove { id } => {
                    if !canvas.elements.contains_key(id) {
                        return Err(ReplayError::UnknownId(id.clone()));
                    }
                    if canvas
                        .elements
                        .values()
                        .any(|e| e.content.references().contains(&id.as_str()))
                    {
                        return Err(ReplayError::ReferencedId(id.clone()));
                    }
                    canvas.elements.remove(id);
                }
                Action::Tag { id, tag } => {
                    if tag.is_empty() {
                        return Err(ReplayError::InvalidTag);
                    }
                    let element = canvas
                        .elements
                        .get_mut(id)
                        .ok_or_else(|| ReplayError::UnknownId(id.clone()))?;
                    if let Err(at) = element.tags.binary_search(tag) {
                        element.tags.insert(at, tag.clone());
                    }
                }
            }
        }
        Ok(canvas)
    }
    fn validate(&self, element: &Element) -> Result<(), ReplayError> {
        if element.id.is_empty() || element.content.bounds().is_some_and(|b| !b.valid()) {
            return Err(ReplayError::InvalidElement(element.id.clone()));
        }
        if element.tags.iter().any(String::is_empty) {
            return Err(ReplayError::InvalidTag);
        }
        for id in element.content.references() {
            let other = self
                .get(id)
                .ok_or_else(|| ReplayError::UnknownId(id.into()))?;
            if id == element.id
                || (matches!(element.content, Content::Connector { .. })
                    && other.content.bounds().is_none())
            {
                return Err(ReplayError::InvalidElement(element.id.clone()));
            }
        }
        Ok(())
    }
    /// SHA-256 of JSON elements in lexical id order, with sorted unique tags.
    /// Audit metadata belongs to the log, not the picture's digest.
    pub fn digest(&self) -> String {
        format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&self.elements).expect("validated canvas serializes")
            )
        )
    }
}

/// JSONL is serialization only: replay performs sequence/reference validation.
/// Empty lines are ignored; malformed records report their one-based line.
pub fn load_log(reader: impl BufRead) -> io::Result<Vec<Op>> {
    let mut ops = vec![];
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        ops.push(serde_json::from_str(&line).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("line {}: {e}", index + 1),
            )
        })?);
    }
    Ok(ops)
}
pub fn save_log(ops: &[Op], mut writer: impl Write) -> io::Result<()> {
    for op in ops {
        serde_json::to_writer(&mut writer, op).map_err(io::Error::other)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}
