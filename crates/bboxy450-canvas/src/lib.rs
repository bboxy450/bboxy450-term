// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The canonical log supplies contiguous operation sequence numbers. Multiple
//! authors replay in append order; removed stable ids never resurrect.
mod camera;
pub mod log;
pub mod record;
pub mod store;
pub use camera::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

pub type ElementId = String;
pub const MAX_ELEMENTS: usize = 10_000;
/// A free path or a connector's waypoints are bounded so one element cannot
/// become the whole log.
pub const MAX_POINTS: usize = 4_096;
/// Logical pixels kept clear around the content when fitting it to a view.
pub const FIT_PADDING: f64 = 32.;

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
    /// The smallest bounds holding every given one; `None` when there are none.
    pub fn union(all: impl IntoIterator<Item = Bounds>) -> Option<Bounds> {
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        let mut any = false;
        for b in all {
            any = true;
            for i in 0..2 {
                lo[i] = lo[i].min(b.origin[i]);
                hi[i] = hi[i].max(b.origin[i] + b.size[i]);
            }
        }
        any.then(|| Bounds {
            origin: lo,
            size: [hi[0] - lo[0], hi[1] - lo[1]],
        })
    }
    fn of_points(points: &[[f64; 2]]) -> Option<Bounds> {
        let mut b = Bounds::union(points.iter().map(|p| Bounds {
            origin: *p,
            size: [0.; 2],
        }))?;
        // A horizontal or vertical stroke still occupies a positive extent.
        b.size = b.size.map(|n| n.max(1.));
        Some(b)
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
    Hexagon,
    Triangle,
    Parallelogram,
    Star,
    Cloud,
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
/// How a connector travels between its endpoints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    #[default]
    Straight,
    /// Axis-aligned elbows; the elbow crossing fewer other elements is taken.
    Orthogonal,
    /// One smooth curve leaving and entering along the port normals.
    Curved,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Align {
    Left,
    #[default]
    Center,
    Right,
}
/// Named colours resolve through the theme; `#rgb`, `#rrggbb` and `#rrggbbaa`
/// are literal.
pub const NAMED_COLORS: &[&str] = &[
    "accent", "fg", "dim", "surface", "bg", "waiting", "error", "done", "red", "orange", "yellow",
    "green", "teal", "blue", "indigo", "violet", "pink", "white", "black",
];
pub fn valid_color(value: &str) -> bool {
    if NAMED_COLORS.contains(&value) {
        return true;
    }
    let Some(hex) = value.strip_prefix('#') else {
        return false;
    };
    matches!(hex.len(), 3 | 6 | 8) && hex.chars().all(|c| c.is_ascii_hexdigit())
}
/// Explicit look. Every field is optional; an absent field keeps the house
/// style derived from kind, emphasis and status. Sizes are logical pixels at
/// zoom one and scale with the drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Style {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke_width: Option<f64>,
    /// Whole-element opacity, 0 to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,
    /// A soft halo in this colour around the element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glow: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub shadow: bool,
    /// Corner radius for box-like shapes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub radius: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,
    /// 100 to 900.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<u16>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub italic: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub mono: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<Align>,
}
impl Style {
    fn valid(&self) -> bool {
        let unit = |n: Option<f64>| n.is_none_or(|n| n.is_finite() && (0. ..=1.).contains(&n));
        let positive =
            |n: Option<f64>, max: f64| n.is_none_or(|n| n.is_finite() && n >= 0. && n <= max);
        [&self.fill, &self.stroke, &self.glow, &self.text]
            .into_iter()
            .flatten()
            .all(|c| valid_color(c))
            && unit(self.opacity)
            && positive(self.stroke_width, 64.)
            && positive(self.radius, 1_000.)
            && positive(self.font_size, 400.)
            && self.weight.is_none_or(|w| (100..=900).contains(&w))
    }
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
        #[serde(default, skip_serializing_if = "is_default")]
        route: Route,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from_side: Option<Side>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        to_side: Option<Side>,
        /// World points the connector passes through, in order.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        waypoints: Vec<[f64; 2]>,
        /// Where along the route the label sits, 0 at `from` to 1 at `to`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label_at: Option<f64>,
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
    /// A free polyline or polygon in world points: a hand-drawn mark, an
    /// outline, a region. Its bounds are the points' box.
    Path {
        points: Vec<[f64; 2]>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        closed: bool,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        smooth: bool,
        #[serde(default, skip_serializing_if = "is_default")]
        arrowheads: [bool; 2],
    },
}
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}
impl Content {
    pub fn bounds(&self) -> Option<Bounds> {
        match self {
            Self::Shape { bounds, .. } | Self::Text { bounds, .. } | Self::Frame { bounds, .. } => {
                Some(*bounds)
            }
            Self::Path { points, .. } => Bounds::of_points(points),
            Self::Connector { .. } => None,
        }
    }
    /// The text a harness reads or patches: a shape's label, a text's body, a
    /// frame's name or a connector's label.
    pub fn label(&self) -> Option<&str> {
        match self {
            Self::Shape { label, .. } => Some(label),
            Self::Text { text, .. } => Some(text),
            Self::Frame { name, .. } => Some(name),
            Self::Connector { label, .. } => label.as_deref(),
            Self::Path { .. } => None,
        }
    }
    pub(crate) fn references(&self) -> Vec<&str> {
        match self {
            Self::Connector { from, to, .. } => vec![from, to],
            Self::Frame { members, .. } => members.iter().map(String::as_str).collect(),
            _ => vec![],
        }
    }
    fn valid(&self) -> bool {
        let finite = |points: &[[f64; 2]]| {
            points.len() <= MAX_POINTS && points.iter().flatten().all(|n| n.is_finite())
        };
        match self {
            Self::Connector {
                waypoints,
                label_at,
                ..
            } => {
                finite(waypoints)
                    && label_at.is_none_or(|t| t.is_finite() && (0. ..=1.).contains(&t))
            }
            Self::Path { points, .. } => points.len() >= 2 && finite(points),
            _ => self.bounds().is_some_and(Bounds::valid),
        }
    }
    /// Moves and resizes the element; a path's points scale with its box.
    fn set_bounds(&mut self, next: Bounds) -> bool {
        match self {
            Self::Shape { bounds, .. } | Self::Text { bounds, .. } | Self::Frame { bounds, .. } => {
                *bounds = next;
                true
            }
            Self::Path { points, .. } => {
                let Some(old) = Bounds::of_points(points) else {
                    return false;
                };
                for (i, v) in points.iter_mut().flat_map(|p| p.iter_mut().enumerate()) {
                    *v = next.origin[i] + (*v - old.origin[i]) / old.size[i] * next.size[i];
                }
                true
            }
            Self::Connector { .. } => false,
        }
    }
    fn set_label(&mut self, value: String) -> bool {
        match self {
            Self::Shape { label, .. } => *label = value,
            Self::Text { text, .. } => *text = value,
            Self::Frame { name, .. } => *name = value,
            Self::Connector { label, .. } => *label = Some(value),
            Self::Path { .. } => return false,
        }
        true
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
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
    /// Changes only the given fields of an existing element. A `style` here
    /// replaces the whole style; an empty object clears it.
    Patch {
        id: ElementId,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bounds: Option<Bounds>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        style: Option<Style>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status: Option<Status>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        emphasis: Option<bool>,
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
    removed: BTreeSet<ElementId>,
}
impl Canvas {
    pub fn elements(&self) -> impl DoubleEndedIterator<Item = &Element> {
        self.elements.values()
    }
    pub fn get(&self, id: &str) -> Option<&Element> {
        self.elements.get(id)
    }
    /// The box around everything with bounds. Fit-to-content in the GUI and
    /// the headless image both fit exactly this.
    pub fn extent(&self) -> Option<Bounds> {
        Bounds::union(self.elements().filter_map(|e| e.content.bounds()))
    }
    /// The box around the named elements, for a frame's members.
    pub fn extent_of<'a>(&self, ids: impl IntoIterator<Item = &'a str>) -> Option<Bounds> {
        Bounds::union(
            ids.into_iter()
                .filter_map(|id| self.get(id).and_then(|e| e.content.bounds())),
        )
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
            if op.author.is_empty() {
                return Err(ReplayError::Author);
            }
            canvas.apply(&op.action)?;
        }
        Ok(canvas)
    }
    /// Applies one batch as a set: an action whose references are not present
    /// yet waits for the rest of the batch, so a frame may list members added
    /// after it. A reference nothing in the batch adds still fails. Returns
    /// the order in which the actions applied.
    pub fn apply_set(&mut self, actions: &[Action]) -> Result<Vec<usize>, ReplayError> {
        let mut pending: Vec<usize> = (0..actions.len()).collect();
        let mut order = Vec::with_capacity(actions.len());
        while !pending.is_empty() {
            let ready = pending.iter().position(|&i| match &actions[i] {
                Action::Add { element } | Action::Change { element } => element
                    .content
                    .references()
                    .iter()
                    .all(|id| self.get(id).is_some()),
                _ => true,
            });
            let i = pending.remove(ready.unwrap_or(0));
            self.apply(&actions[i])?;
            order.push(i);
        }
        Ok(order)
    }
    /// The store validates a whole candidate batch before making any append.
    pub(crate) fn apply(&mut self, action: &Action) -> Result<(), ReplayError> {
        match action {
            Action::Add { element } | Action::Change { element } => {
                if self.removed.contains(&element.id) {
                    return Ok(());
                }
                let exists = self.elements.contains_key(&element.id);
                if matches!(action, Action::Add { .. }) {
                    if exists {
                        return Err(ReplayError::DuplicateId(element.id.clone()));
                    }
                    if self.elements.len() == MAX_ELEMENTS {
                        return Err(ReplayError::ElementCap { cap: MAX_ELEMENTS });
                    }
                } else if !exists {
                    return Err(ReplayError::UnknownId(element.id.clone()));
                }
                self.validate(element)?;
                // An endpoint cannot be replaced by a connector: references
                // always resolve directly to an element with world bounds.
                if element.content.bounds().is_none() && self.elements.values().any(|e| {
                        matches!(&e.content, Content::Connector { from, to, .. } if from == &element.id || to == &element.id)
                    }) { return Err(ReplayError::ReferencedId(element.id.clone())); }
                let mut element = element.clone();
                element.tags.sort();
                element.tags.dedup();
                self.elements.insert(element.id.clone(), element);
            }
            Action::Patch {
                id,
                bounds,
                label,
                style,
                status,
                emphasis,
            } => {
                if self.removed.contains(id) {
                    return Ok(());
                }
                let mut element = self
                    .elements
                    .get(id)
                    .cloned()
                    .ok_or_else(|| ReplayError::UnknownId(id.clone()))?;
                if let Some(bounds) = bounds
                    && !element.content.set_bounds(*bounds)
                {
                    return Err(ReplayError::InvalidElement(id.clone()));
                }
                if let Some(label) = label
                    && !element.content.set_label(label.clone())
                {
                    return Err(ReplayError::InvalidElement(id.clone()));
                }
                if let Some(style) = style {
                    element.style = (*style != Style::default()).then(|| style.clone());
                }
                if let Some(status) = status {
                    element.status = *status;
                }
                if let Some(emphasis) = emphasis {
                    element.emphasis = *emphasis;
                }
                self.validate(&element)?;
                self.elements.insert(id.clone(), element);
            }
            Action::Remove { id } => {
                if self.removed.contains(id) {
                    return Ok(());
                }
                if !self.elements.contains_key(id) {
                    return Err(ReplayError::UnknownId(id.clone()));
                }
                if self
                    .elements
                    .values()
                    .any(|e| e.content.references().contains(&id.as_str()))
                {
                    return Err(ReplayError::ReferencedId(id.clone()));
                }
                self.elements.remove(id);
                self.removed.insert(id.clone());
            }
            Action::Tag { id, tag } => {
                if self.removed.contains(id) {
                    return Ok(());
                }
                if tag.is_empty() {
                    return Err(ReplayError::InvalidTag);
                }
                let element = self
                    .elements
                    .get_mut(id)
                    .ok_or_else(|| ReplayError::UnknownId(id.clone()))?;
                if let Err(at) = element.tags.binary_search(tag) {
                    element.tags.insert(at, tag.clone());
                }
            }
        }
        Ok(())
    }
    fn validate(&self, element: &Element) -> Result<(), ReplayError> {
        if element.id.is_empty()
            || !element.content.valid()
            || element.style.as_ref().is_some_and(|s| !s.valid())
        {
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

pub use log::{load_log, save_log};
