// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
            palette: serde_json::to_value(term.colors())
                .expect("terminal palette is JSON serializable"),
            cursor_style: serde_json::to_value(term.cursor_style())
                .expect("terminal cursor style is JSON serializable"),
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

#[cfg(test)]
mod tests {
    use super::*;

    // The pre-optimization implementation, independent of the new getters.
    fn checkpoint_screen(term: &Term) -> Screen {
        let grid = term.grid();
        let checkpoint = serde_json::to_value(term.checkpoint()).unwrap();
        let model = &checkpoint["model"];
        Screen {
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
            title: "fixture".into(),
            cwd: Some("/fixture".into()),
            gap: Some("fixture gap".into()),
        }
    }

    fn assert_same(term: &Term) {
        assert_eq!(
            Screen::from_term(
                term,
                "fixture".into(),
                Some("/fixture".into()),
                Some("fixture gap".into())
            ),
            checkpoint_screen(term),
        );
    }

    // keeps: spec terminal-core §D7
    #[test]
    fn direct_metadata_matches_checkpoint_for_all_firehose_parser_scenes() {
        let fixtures: &[&[u8]] = &[
            include_bytes!("../../bboxy-firehose/corpus/codex.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/claude.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/palette.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/unicode.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/progress.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/alternate.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/scroll-links-sync.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/osc7501-claude.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/osc7501-preflight.ansi"),
            include_bytes!("../../bboxy-firehose/corpus/osc7501-codex.ansi"),
        ];
        for bytes in fixtures {
            let mut term = Term::new(80, 24, 0);
            assert_same(&term);
            for part in bytes.chunks(97) {
                term.advance(part);
            }
            assert_same(&term);
        }
    }

    // keeps: spec terminal-core §D7
    #[test]
    fn direct_metadata_preserves_custom_palette_cursor_and_resets() {
        let mut term = Term::new(38, 57, 0);
        let states: &[&[u8]] = &[
            b"\x1b]4;1;rgb:12/34/56;42;#abcdef\x07",
            b"\x1b]10;#102030\x07\x1b]11;#405060\x07\x1b]12;#708090\x07",
            b"\x1b[1 q",
            b"\x1b[2 q",
            b"\x1b[3 q",
            b"\x1b[4 q",
            b"\x1b[5 q",
            b"\x1b[6 q",
            b"\x1b[?1049hwide:\xe4\xb8\xad\xcc\x81",
            b"\x1b[?1049l\x1b]104;1;42\x07\x1b]110\x07\x1b]111\x07\x1b]112\x07",
            b"\x1b[0 q\x1b[!p",
            b"\x1bc",
        ];
        for bytes in states {
            term.advance(bytes);
            assert_same(&term);
        }
        term.resize(77, 57);
        assert_same(&term);
        let checkpoint = term.checkpoint();
        term.advance(b"\x1b[6 q\x1b]10;#abcdef\x07");
        term.restore(checkpoint);
        assert_same(&term);
    }
}
