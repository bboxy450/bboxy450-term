// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Our own Mermaid engine: parse a diagram and lay it out to plain geometry.
//!
//! Pure library, no rendering and no workspace dependencies. The caller measures
//! text through [`Measure`]; the output is true size (never scaled) in the
//! caller's units. Unsupported syntax is an [`Error`]: the caller shows the
//! block's source instead of a broken diagram.
//!
//! ```
//! use bbt_mermaid::{layout, parse, MonoMeasure, Style};
//! let d = parse("flowchart LR\n  A[start] --> B{ok?}").unwrap();
//! let g = layout(&d, &MonoMeasure::default(), &Style::default()).unwrap();
//! assert_eq!(g.nodes.len(), 2);
//! ```

mod flowchart;
mod geometry;
mod layered;
mod sequence;
mod state;
mod text;

pub use flowchart::{Dir, Endpoint, FEdge, FNode, Flowchart, Subgraph};
pub use geometry::*;
pub use sequence::{Actor, Event, FragmentKind, NotePlace, Sequence};

/// A parsed diagram.
#[derive(Clone, Debug, PartialEq)]
pub enum Diagram {
    Flowchart(Flowchart),
    Sequence(Sequence),
    /// A state diagram, parsed into the flowchart model (composites are subgraphs).
    State(Flowchart),
}

/// Parse or layout failure. `line` is 1-based in the block source (0 when not tied to a line).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

impl Error {
    pub(crate) fn new(line: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.line > 0 {
            write!(f, "line {}: {}", self.line, self.message)
        } else {
            f.write_str(&self.message)
        }
    }
}

impl std::error::Error for Error {}

/// Parse a Mermaid block (the text inside the fence).
pub fn parse(src: &str) -> Result<Diagram, Error> {
    let lines = text::strip_frontmatter(src);
    let Some((n, first)) = lines.iter().find(|(_, l)| {
        let t = l.trim();
        !t.is_empty() && !t.starts_with("%%")
    }) else {
        return Err(Error::new(0, "empty diagram"));
    };
    let kw = first.split_whitespace().next().unwrap_or("");
    match kw {
        "flowchart" | "graph" => flowchart::parse(&lines).map(Diagram::Flowchart),
        "sequenceDiagram" => sequence::parse(&lines).map(Diagram::Sequence),
        "stateDiagram" | "stateDiagram-v2" => state::parse(&lines).map(Diagram::State),
        other => Err(Error::new(
            *n,
            format!("unsupported diagram type '{other}'"),
        )),
    }
}

/// Lay a parsed diagram out to geometry.
pub fn layout(d: &Diagram, m: &dyn Measure, s: &Style) -> Result<Geometry, Error> {
    match d {
        Diagram::Flowchart(f) => layered::layout(f, Kind::Flowchart, m, s),
        Diagram::State(f) => layered::layout(f, Kind::State, m, s),
        Diagram::Sequence(q) => sequence::layout(q, m, s),
    }
}

/// Parse and lay out in one step.
pub fn render(src: &str, m: &dyn Measure, s: &Style) -> Result<Geometry, Error> {
    layout(&parse(src)?, m, s)
}
