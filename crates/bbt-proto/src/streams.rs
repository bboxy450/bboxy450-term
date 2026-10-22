//! Serializable terminal view snapshots and deltas.
use bbt_core::{Cell, Column, Dimensions, Line, Term};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

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
