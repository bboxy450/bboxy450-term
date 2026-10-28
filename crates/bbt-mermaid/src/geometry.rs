// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Output geometry, text measurement and layout style.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    pub w: f32,
    pub h: f32,
}

/// Axis-aligned rectangle; `x`, `y` is the top-left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn centered(c: Point, s: Size) -> Self {
        Rect {
            x: c.x - s.w / 2.0,
            y: c.y - s.h / 2.0,
            w: s.w,
            h: s.h,
        }
    }
    pub fn center(&self) -> Point {
        Point {
            x: self.x + self.w / 2.0,
            y: self.y + self.h / 2.0,
        }
    }
    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
    /// True when the interiors intersect (touching edges do not count).
    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }
    pub fn contains_rect(&self, o: &Rect) -> bool {
        o.x >= self.x && o.y >= self.y && o.right() <= self.right() && o.bottom() <= self.bottom()
    }
}

/// Which type step a piece of text is set in; the caller maps it to its type scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextRole {
    Node,
    EdgeLabel,
    ClusterTitle,
    Actor,
    Message,
    Note,
    FrameLabel,
}

/// Caller-provided text measurement: the advance width and line height of one line.
pub trait Measure {
    fn measure(&self, line: &str, role: TextRole) -> Size;
}

/// Monospace estimate for tests and debug dumps: `char_w` per char, `line_h` per line.
#[derive(Clone, Copy, Debug)]
pub struct MonoMeasure {
    pub char_w: f32,
    pub line_h: f32,
}

impl Default for MonoMeasure {
    fn default() -> Self {
        Self {
            char_w: 7.0,
            line_h: 16.0,
        }
    }
}

impl Measure for MonoMeasure {
    fn measure(&self, line: &str, role: TextRole) -> Size {
        let k = match role {
            TextRole::EdgeLabel | TextRole::FrameLabel | TextRole::Note => 0.9,
            _ => 1.0,
        };
        Size {
            w: line.chars().count() as f32 * self.char_w * k,
            h: self.line_h * k,
        }
    }
}

/// Spacing used by the layouts, in the caller's units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub node_pad_x: f32,
    pub node_pad_y: f32,
    /// Gap between adjacent nodes in a rank.
    pub node_sep: f32,
    /// Gap next to edge waypoints in a rank.
    pub edge_sep: f32,
    /// Gap between ranks.
    pub rank_sep: f32,
    pub cluster_pad: f32,
    pub label_pad: f32,
    /// Bend radius the renderer should use at interior polyline points.
    pub corner_radius: f32,
    pub margin: f32,
    /// Sequence: gap between participant boxes.
    pub actor_gap: f32,
    /// Sequence: vertical step between rows.
    pub row_gap: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            node_pad_x: 14.0,
            node_pad_y: 9.0,
            node_sep: 36.0,
            edge_sep: 14.0,
            rank_sep: 48.0,
            cluster_pad: 14.0,
            label_pad: 4.0,
            corner_radius: 6.0,
            margin: 8.0,
            actor_gap: 40.0,
            row_gap: 12.0,
        }
    }
}

/// Text block: `lines` stack top to bottom, each centred in `rect`.
#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub lines: Vec<String>,
    pub role: TextRole,
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Shape {
    Rect,
    Rounded,
    Stadium,
    /// `[[ ]]`; with `Node::link` set it is our live document link.
    Subroutine,
    Cylinder,
    Circle,
    DoubleCircle,
    Asymmetric,
    Diamond,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
    /// Sequence participant box (`participant`).
    Participant,
    /// Sequence stick figure (`actor`); the rect bounds figure and name.
    Actor,
    Note,
    Activation,
    /// State diagram `[*]` as a source: a small filled circle.
    Start,
    /// State diagram `[*]` as a target: a ringed filled circle.
    End,
    /// `<<choice>>`: a small diamond.
    Choice,
    /// `<<fork>>` / `<<join>>`: a solid bar across the flow.
    Bar,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: String,
    pub shape: Shape,
    pub rect: Rect,
    pub label: Option<Label>,
    /// `A[[slug]]`: the slug of the document this node opens.
    pub link: Option<String>,
    pub classes: Vec<String>,
    /// Raw `style id ...` declaration, if any.
    pub style: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClusterKind {
    Subgraph,
    Loop,
    Alt,
    Opt,
    Par,
    Critical,
    Break,
    Rect,
}

/// A horizontal divider inside a frame (`else`, `and`, `option`).
#[derive(Clone, Debug, PartialEq)]
pub struct Divider {
    pub y: f32,
    pub label: Option<Label>,
}

/// A subgraph box, or a sequence frame (loop, alt, ...). Outer clusters come first.
#[derive(Clone, Debug, PartialEq)]
pub struct Cluster {
    pub id: String,
    pub kind: ClusterKind,
    pub rect: Rect,
    pub title: Option<Label>,
    pub dividers: Vec<Divider>,
    /// Nesting depth, 0 for top level.
    pub depth: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Stroke {
    Solid,
    Dotted,
    Thick,
    /// `~~~`: affects layout only; renderers skip it.
    Invisible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Arrow {
    None,
    /// Filled triangle.
    Filled,
    /// Open two-stroke head (sequence async `-)`).
    Open,
    Cross,
    Circle,
}

/// An edge or a message. `points` is a polyline: the first point lies on the
/// source's border, the last on the target's; arrowhead tips sit exactly on
/// the end points, so a renderer shortens the stroke by the head length.
/// Interior points are bends to round with `Geometry::corner_radius`.
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub points: Vec<Point>,
    pub stroke: Stroke,
    pub start: Arrow,
    pub end: Arrow,
    pub label: Option<Label>,
}

/// A plain line segment (sequence lifelines).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Line {
    pub a: Point,
    pub b: Point,
    pub stroke: Stroke,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Flowchart,
    Sequence,
    State,
}

/// `classDef name style...`, raw.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassDef {
    pub name: String,
    pub style: String,
}

/// A laid-out diagram at true size. Paint order: clusters (outer first),
/// lines, nodes, edges with their labels.
#[derive(Clone, Debug, PartialEq)]
pub struct Geometry {
    pub kind: Kind,
    pub size: Size,
    pub clusters: Vec<Cluster>,
    pub lines: Vec<Line>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub class_defs: Vec<ClassDef>,
    pub corner_radius: f32,
}
