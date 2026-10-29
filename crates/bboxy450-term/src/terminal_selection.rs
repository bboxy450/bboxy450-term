// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! GUI selection over the exact displayed cells. No parser, IO or clipboard effects.
use std::sync::Arc;

use bbt_core::term::{TermMode, cell::Flags};
use bbt_proto::{Screen, SeatId, View};
use bbt_ui_model::layout::Rect;

/// A displayed screen and its actual (unscaled, clipped) cell geometry.
#[derive(Clone)]
pub struct Display {
    pub seat: SeatId,
    pub view: View,
    pub screen: Arc<Screen>,
    pub rect: Rect,
    pub cell: [f32; 2],
}
impl Display {
    pub fn contains(&self, at: [f32; 2]) -> bool {
        at[0] >= self.rect.x
            && at[0] < self.rect.right()
            && at[1] >= self.rect.y
            && at[1] < self.rect.bottom()
    }
    fn point(&self, at: [f32; 2]) -> Point {
        Point {
            row: (((at[1] - self.rect.y) / self.cell[1]).floor().max(0.) as usize)
                .min(self.screen.rows.saturating_sub(1)),
            col: (((at[0] - self.rect.x) / self.cell[0]).round().max(0.) as usize)
                .min(self.screen.cols),
        }
    }
    fn same(&self, other: &Self) -> bool {
        self.seat == other.seat
            && self.view == other.view
            && Arc::ptr_eq(&self.screen, &other.screen)
            && self.rect == other.rect
            && self.cell == other.cell
    }
    pub fn allows_selection(&self, shift: bool) -> bool {
        shift || self.screen.modes & TermMode::MOUSE_MODE.bits() as u64 == 0
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Point {
    row: usize,
    col: usize,
}
struct Selection {
    display: Display,
    anchor: Point,
    end: Point,
}
#[derive(Default)]
pub struct SelectionState {
    selected: Option<Selection>,
    pub dragging: bool,
}
impl SelectionState {
    pub fn clear(&mut self) {
        self.selected = None;
        self.dragging = false;
    }
    pub fn begin(&mut self, display: Display, at: [f32; 2], shift: bool) -> bool {
        self.clear();
        if display.screen.rows == 0
            || display.screen.cols == 0
            || display.cell.iter().any(|n| !n.is_finite() || *n <= 0.)
            || !display.contains(at)
            || !display.allows_selection(shift)
        {
            return false;
        }
        let anchor = display.point(at);
        self.selected = Some(Selection {
            display,
            anchor,
            end: anchor,
        });
        self.dragging = true;
        true
    }
    pub fn seat(&self) -> Option<SeatId> {
        self.selected.as_ref().map(|s| s.display.seat)
    }
    /// A changed viewport, focus, process/view generation or screen invalidates the selection.
    pub fn valid(&self, display: &Display) -> bool {
        self.selected
            .as_ref()
            .is_some_and(|s| s.display.same(display))
    }
    pub fn extend(&mut self, display: &Display, at: [f32; 2]) -> bool {
        if !self.dragging {
            return false;
        }
        if !self.valid(display) {
            self.clear();
            return false;
        }
        self.selected.as_mut().unwrap().end = display.point(at);
        true
    }
    pub fn release(&mut self) {
        self.dragging = false;
    }
    /// Row ranges are half-open and expand a boundary inside a wide glyph.
    pub fn ranges(&self, display: &Display) -> Vec<(usize, usize, usize)> {
        if !self.valid(display) {
            return vec![];
        }
        let s = self.selected.as_ref().unwrap();
        let (start, end) = if s.anchor <= s.end {
            (s.anchor, s.end)
        } else {
            (s.end, s.anchor)
        };
        if start == end {
            return vec![];
        }
        (start.row..=end.row)
            .filter_map(|row| {
                let cells = display.screen.cells.get(row)?;
                let mut a = if row == start.row { start.col } else { 0 }.min(cells.len());
                let mut b = if row == end.row {
                    end.col
                } else {
                    display.screen.cols
                }
                .min(cells.len());
                if a < b {
                    if a > 0
                        && cells
                            .get(a)
                            .is_some_and(|c| c.flags.contains(Flags::WIDE_CHAR_SPACER))
                    {
                        a -= 1;
                    }
                    if b < cells.len() && cells[b].flags.contains(Flags::WIDE_CHAR_SPACER) {
                        b += 1;
                    }
                    Some((row, a, b))
                } else if row < end.row {
                    Some((row, a, b))
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn text(&self, display: &Display) -> Option<String> {
        let ranges = self.ranges(display);
        if ranges.is_empty() {
            return None;
        }
        let mut text = String::new();
        let selected = self.selected.as_ref().unwrap();
        let end_row = selected.anchor.row.max(selected.end.row);
        for &(row, start, end) in &ranges {
            let cells = &display.screen.cells[row];
            let mut line = String::new();
            for c in &cells[start..end] {
                if c.flags
                    .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
                {
                    continue;
                }
                line.push(c.c);
                line.extend(c.zerowidth().unwrap_or_default());
            }
            let wrapped = end == display.screen.cols
                && cells
                    .last()
                    .is_some_and(|c| c.flags.contains(Flags::WRAPLINE));
            // Blank padding at a hard line end is not copied. Explicit partial spaces remain.
            if end == display.screen.cols && !wrapped {
                line.truncate(line.trim_end_matches(' ').len());
            }
            text.push_str(&line);
            if row < end_row && !wrapped {
                text.push('\n');
            }
        }
        (!text.is_empty()).then_some(text)
    }
    pub fn rectangles(&self, display: &Display) -> Vec<Rect> {
        self.ranges(display)
            .into_iter()
            .filter_map(|(row, a, b)| {
                let x = display.rect.x + a as f32 * display.cell[0];
                let y = display.rect.y + row as f32 * display.cell[1];
                let w = (b - a) as f32 * display.cell[0];
                let w = w.min(display.rect.right() - x);
                let h = display.cell[1].min(display.rect.bottom() - y);
                (w > 0. && h > 0.).then_some(Rect::new(x, y, w, h))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn display(bytes: &[u8], cols: usize, rows: usize) -> Display {
        let mut term = bbt_core::Term::new(cols, rows, 0);
        term.advance(bytes);
        Display {
            seat: SeatId::from_u128(1),
            view: View {
                seat: bbt_mux::Ulid(1),
                generation: 2,
                view_generation: bbt_mux::Ulid(3),
            },
            screen: Arc::new(Screen::from_term(&term, String::new(), None, None)),
            rect: Rect::new(0., 0., cols as f32 * 10., rows as f32 * 20.),
            cell: [10., 20.],
        }
    }
    fn select(d: &Display, a: [f32; 2], b: [f32; 2]) -> SelectionState {
        let mut s = SelectionState::default();
        assert!(s.begin(d.clone(), a, false));
        assert!(s.extend(d, b));
        s.release();
        s
    }
    #[test]
    fn selection_unicode_wide_combining_reverse_and_spaces() {
        let d = display("a界e\u{301}  z".as_bytes(), 10, 2);
        let s = select(&d, [10., 1.], [40., 1.]);
        assert_eq!(s.text(&d).as_deref(), Some("界e\u{301}"));
        assert_eq!(select(&d, [40., 1.], [20., 1.]).text(&d), s.text(&d));
        assert_eq!(
            select(&d, [40., 1.], [60., 1.]).text(&d).as_deref(),
            Some("  ")
        );
        assert_eq!(select(&d, [0., 1.], [0., 1.]).text(&d), None);
    }
    #[test]
    fn selection_hard_lines_trim_padding_and_soft_wraps_join() {
        let d = display(b"ab\r\ncd", 5, 3);
        assert_eq!(
            select(&d, [0., 1.], [50., 21.]).text(&d).as_deref(),
            Some("ab\ncd")
        );
        let d = display(b"abcdefgh", 5, 3);
        assert_eq!(
            select(&d, [0., 1.], [30., 21.]).text(&d).as_deref(),
            Some("abcdefgh")
        );
    }
    #[test]
    fn selection_fences_seat_generation_view_screen_and_geometry_and_mouse_override() {
        let d = display(b"abcdef", 10, 2);
        let s = select(&d, [0., 1.], [30., 1.]);
        let mut other = d.clone();
        other.view.generation = 3;
        assert_eq!(s.text(&other), None);
        let mut other = d.clone();
        other.seat = SeatId::from_u128(9);
        assert_eq!(s.text(&other), None);
        let mut other = d.clone();
        other.view.view_generation = bbt_mux::Ulid(9);
        assert_eq!(s.text(&other), None);
        let mut other = d.clone();
        other.screen = Arc::new((*d.screen).clone());
        assert_eq!(s.text(&other), None);
        let mut other = d.clone();
        other.rect.x += 1.;
        assert_eq!(s.text(&other), None);
        let mut other = d.clone();
        Arc::make_mut(&mut other.screen).modes = TermMode::MOUSE_REPORT_CLICK.bits() as u64;
        let mut s = SelectionState::default();
        assert!(!s.begin(other.clone(), [0., 1.], false));
        assert!(s.begin(other, [0., 1.], true));
    }
}
