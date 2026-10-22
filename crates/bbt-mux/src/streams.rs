//! A bounded acknowledged baseline and one latest pending viewport per viewer.

use bbt_core::{Cell, Column, Dimensions, Line, Term};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::store::{Result, invalid};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct View {
    pub seat: Ulid,
    pub generation: u64,
    pub view_generation: Ulid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Screen {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Vec<Cell>>,
    pub cursor: (i32, usize),
    pub modes: u64,
    pub palette: serde_json::Value,
    pub cursor_style: serde_json::Value,
    pub title: String,
    pub cwd: Option<String>,
    pub gap: Option<String>,
}
impl Screen {
    pub fn from_term(term: &Term, title: String, cwd: Option<String>, gap: Option<String>) -> Self {
        let grid = term.grid();
        // Version 1 uses our fork's checkpoint schema for otherwise-private
        // palette/cursor metadata, without changing the core's public API.
        let checkpoint = serde_json::to_value(term.checkpoint())
            .expect("terminal checkpoint is JSON serializable");
        let model = &checkpoint["model"];
        Self {
            cols: grid.columns(),
            rows: grid.screen_lines(),
            cells: (0..grid.screen_lines())
                .map(|row| {
                    (0..grid.columns())
                        .map(|col| grid[Line(row as i32)][Column(col)].clone())
                        .collect()
                })
                .collect(),
            cursor: (grid.cursor.point.line.0, grid.cursor.point.column.0),
            modes: term.mode().bits() as u64,
            palette: model["colors"].clone(),
            cursor_style: if model["cursor_style"].is_null() {
                model["config"]["default_cursor_style"].clone()
            } else {
                model["cursor_style"].clone()
            },
            title,
            cwd,
            gap,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub view: View,
    pub sequence: u64,
    pub screen: Screen,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Delta {
    pub view: View,
    pub base: u64,
    pub sequence: u64,
    pub rows: Vec<(usize, Vec<Cell>)>,
    pub cursor: (i32, usize),
    pub modes: u64,
    pub palette: serde_json::Value,
    pub cursor_style: serde_json::Value,
    pub title: String,
    pub cwd: Option<String>,
    pub gap: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Update {
    Snapshot {
        snapshot: Box<Snapshot>,
    },
    Delta {
        #[serde(flatten)]
        delta: Box<Delta>,
    },
}

#[derive(Default)]
pub struct ViewStream {
    baseline: Option<Snapshot>,
    sent: Option<Snapshot>,
    pending: Option<Snapshot>,
}
impl ViewStream {
    pub fn offer(&mut self, snapshot: Snapshot) -> Result<()> {
        let latest = self
            .pending
            .as_ref()
            .or(self.sent.as_ref())
            .or(self.baseline.as_ref());
        if let Some(latest) = latest {
            if latest.view.seat != snapshot.view.seat
                || latest.view.generation > snapshot.view.generation
            {
                return Err(invalid("stale or wrong-seat viewport"));
            }
            if latest.view == snapshot.view && snapshot.sequence < latest.sequence {
                return Err(invalid("stale viewport sequence"));
            }
        }
        self.pending = Some(snapshot);
        Ok(())
    }
    pub fn next_update(&mut self) -> Option<Update> {
        if self.sent.is_some() {
            return None;
        }
        let next = self.pending.take()?;
        let update = if let Some(base) = &self.baseline
            && base.view == next.view
            && base.screen.cols == next.screen.cols
            && base.screen.rows == next.screen.rows
        {
            Update::Delta {
                delta: Box::new(Delta {
                    view: next.view,
                    base: base.sequence,
                    sequence: next.sequence,
                    rows: next
                        .screen
                        .cells
                        .iter()
                        .enumerate()
                        .filter(|(i, row)| base.screen.cells[*i] != **row)
                        .map(|(i, row)| (i, row.clone()))
                        .collect(),
                    cursor: next.screen.cursor,
                    modes: next.screen.modes,
                    palette: next.screen.palette.clone(),
                    cursor_style: next.screen.cursor_style.clone(),
                    title: next.screen.title.clone(),
                    cwd: next.screen.cwd.clone(),
                    gap: next.screen.gap.clone(),
                }),
            }
        } else {
            Update::Snapshot {
                snapshot: Box::new(next.clone()),
            }
        };
        self.sent = Some(next);
        Some(update)
    }
    pub fn applied(&mut self, view: View, sequence: u64) -> Result<()> {
        if self
            .baseline
            .as_ref()
            .is_some_and(|b| b.view == view && b.sequence == sequence)
        {
            return Ok(());
        }
        if self
            .sent
            .as_ref()
            .is_none_or(|s| s.view != view || s.sequence != sequence)
        {
            return Err(invalid("ack does not match sent viewport"));
        }
        self.baseline = self.sent.take();
        Ok(())
    }
    pub fn reset(&mut self) {
        self.baseline = None;
        self.sent = None;
    }
}

/// Minimal GUI-side reference: deltas are applied to an exact, atomic baseline.
#[derive(Default)]
pub struct Replica {
    pub snapshot: Option<Snapshot>,
}
impl Replica {
    pub fn apply(&mut self, update: Update) -> Result<()> {
        match update {
            Update::Snapshot { snapshot } => {
                if self.snapshot.as_ref().is_some_and(|old| {
                    old.view.seat == snapshot.view.seat
                        && (old.view.generation > snapshot.view.generation
                            || (old.view == snapshot.view && old.sequence > snapshot.sequence))
                }) {
                    return Err(invalid("stale snapshot"));
                }
                if snapshot.screen.cells.len() != snapshot.screen.rows
                    || snapshot
                        .screen
                        .cells
                        .iter()
                        .any(|r| r.len() != snapshot.screen.cols)
                {
                    return Err(invalid("invalid snapshot dimensions"));
                }
                self.snapshot = Some(*snapshot);
            }
            Update::Delta { delta } => {
                let Delta {
                    view,
                    base,
                    sequence,
                    rows,
                    cursor,
                    modes,
                    palette,
                    cursor_style,
                    title,
                    cwd,
                    gap,
                } = *delta;
                let old = self
                    .snapshot
                    .as_ref()
                    .ok_or_else(|| invalid("snapshot required"))?;
                if old.view != view || old.sequence != base || sequence < base {
                    return Err(invalid("baseline mismatch; snapshot required"));
                }
                if rows
                    .iter()
                    .any(|(i, row)| *i >= old.screen.rows || row.len() != old.screen.cols)
                {
                    return Err(invalid("invalid delta rows"));
                }
                let mut next = old.clone();
                for (i, row) in rows {
                    next.screen.cells[i] = row;
                }
                next.sequence = sequence;
                next.screen.cursor = cursor;
                next.screen.modes = modes;
                next.screen.palette = palette;
                next.screen.cursor_style = cursor_style;
                next.screen.title = title;
                next.screen.cwd = cwd;
                next.screen.gap = gap;
                self.snapshot = Some(next);
            }
        }
        Ok(())
    }
}

pub use bbt_supervisor::ring::HistoryGap;
