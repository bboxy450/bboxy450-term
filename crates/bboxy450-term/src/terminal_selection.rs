// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! GUI selection over the exact displayed cells. No parser, IO or clipboard effects.
use std::sync::Arc;

use bbt_core::term::cell::Flags;

pub fn web_link(uri: &str) -> bool {
    (uri.starts_with("https://")
        || uri.starts_with("http://")
        || uri.starts_with("mailto:")
        || uri.starts_with("file:///")
        || uri.starts_with("file://localhost/"))
        && !uri.chars().any(|c| c.is_control() || c.is_whitespace())
}
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
    pub historical_row: Option<u64>,
}
impl Display {
    /// Resolve Markdown/wiki tokens from the exact displayed logical line,
    /// including soft wraps. The host resolves the locator in its captured workspace.
    pub fn document_at(&self, at: [f32; 2]) -> Option<String> {
        if !self.contains(at) {
            return None;
        }
        let row = ((at[1] - self.rect.y) / self.cell[1]).floor() as usize;
        let col = ((at[0] - self.rect.x) / self.cell[0]).floor() as usize;
        if let Some(link) = self.screen.cells.get(row)?.get(col)?.hyperlink() {
            return markdown_target(link.uri());
        }
        let wraps = |r: usize| {
            self.screen
                .cells
                .get(r)
                .and_then(|r| r.last())
                .is_some_and(|c| c.flags.contains(Flags::WRAPLINE))
        };
        let mut first = row;
        while first > 0 && wraps(first - 1) {
            first -= 1;
        }
        let mut last = row;
        while last + 1 < self.screen.rows && wraps(last) {
            last += 1;
        }
        let mut line = String::new();
        let mut hit = 0;
        for r in first..=last {
            for (c, cell) in self.screen.cells.get(r)?.iter().enumerate() {
                if r == row && c == col {
                    hit = line.len();
                }
                if !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    line.push(cell.c);
                }
            }
        }
        let mut from = 0;
        while let Some(open) = line[from..].find("[[") {
            let start = from + open;
            let Some(close) = line[start + 2..].find("]]") else {
                break;
            };
            let end = start + 2 + close;
            if (start..end + 2).contains(&hit) {
                let target = line[start + 2..end].split('|').next()?.trim();
                return (!target.is_empty() && !target.chars().any(char::is_control))
                    .then(|| target.into());
            }
            from = end + 2;
        }
        let boundary = |c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '`' | '<' | '>');
        let start = line[..hit].char_indices().rfind(|(_,c)| boundary(*c)).map_or(0, |(i,c)| i+c.len_utf8());
        let end = line[hit..].find(boundary).map_or(line.len(), |i| hit + i);
        markdown_target(&line[start..end])
    }
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
            && self.screen.rows == other.screen.rows
            && self.screen.cols == other.screen.cols
            && self.screen.gap == other.screen.gap
            && self.rect == other.rect
            && self.cell == other.cell
            && self.historical_row == other.historical_row
    }
    pub fn allows_selection(&self, _shift: bool) -> bool {
        // The host currently owns pointer gestures; it does not forward mouse reports.
        // Enabling a TUI mouse mode must not leave that gesture with no owner.
        true
    }
    /// OSC 8 labels and ordinary web URLs use exactly the last displayed cells.
    pub fn link_at(&self, at: [f32; 2]) -> Option<String> {
        if !self.contains(at) {
            return None;
        }
        let row = ((at[1] - self.rect.y) / self.cell[1]).floor() as usize;
        let col = ((at[0] - self.rect.x) / self.cell[0]).floor() as usize;
        let cells = self.screen.cells.get(row)?;
        if let Some(link) = cells.get(col)?.hyperlink() {
            return web_link(link.uri()).then(|| link.uri().to_owned());
        }
        let index = row * self.screen.cols + col;
        let cell = |n: usize| {
            self.screen
                .cells
                .get(n / self.screen.cols)?
                .get(n % self.screen.cols)
        };
        let joins = |left: usize| cell(left).is_some_and(|c| c.flags.contains(Flags::WRAPLINE));
        let valid = |c: char| !c.is_whitespace() && !matches!(c, '<' | '>' | '"' | '\'' | '`');
        let mut a = index;
        let mut b = index + 1;
        while a > 0
            && (!a.is_multiple_of(self.screen.cols) || joins(a - 1))
            && cell(a - 1).is_some_and(|c| valid(c.c))
        {
            a -= 1;
        }
        while b < self.screen.rows * self.screen.cols
            && (!b.is_multiple_of(self.screen.cols) || joins(b - 1))
            && cell(b).is_some_and(|c| valid(c.c))
        {
            b += 1;
        }
        let token: String = (a..b)
            .filter_map(cell)
            .filter(|c| !c.flags.contains(Flags::WIDE_CHAR_SPACER))
            .map(|c| c.c)
            .collect();
        let target = token
            .trim_start_matches(['(', '[', '{'])
            .trim_end_matches(['.', ',', ';', ':', '!', '?', ')', ']', '}']);
        web_link(target).then(|| target.to_owned())
    }
}

fn markdown_target(token: &str) -> Option<String> {
    let token = token
        .trim_start_matches(['(', '[', '{'])
        .trim_end_matches(['.', ',', ';', '!', '?', ')', ']', '}']);
    if token.starts_with("file://") && !token.starts_with("file:///") && !token.starts_with("file://localhost/") { return None; }
    let token = token
        .strip_prefix("file://localhost")
        .or_else(|| token.strip_prefix("file://"))
        .unwrap_or(token);
    if token.contains("://") || token.chars().any(char::is_control) {
        return None;
    }
    let end = token.rfind(".md")? + 3;
    if token[..end].contains(':') { return None; }
    let suffix = &token[end..];
    if !suffix.is_empty()
        && (!suffix.starts_with(':')
            || !suffix[1..]
                .split(':')
                .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit())))
    {
        return None;
    }
    Some(token[..end].into())
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
    /// Reject changed identity/geometry or overwritten selected cells; cursor, metadata and
    /// unrelated output remain live. Only selected cells are compared, never retained history.
    pub fn valid(&self, display: &Display) -> bool {
        self.selected.as_ref().is_some_and(|s| {
            s.display.same(display)
                && (Arc::ptr_eq(&s.display.screen, &display.screen)
                    || Self::selected_ranges(s).iter().all(|&(row, a, b)| {
                        display
                            .screen
                            .cells
                            .get(row)
                            .and_then(|cells| cells.get(a..b))
                            == s.display
                                .screen
                                .cells
                                .get(row)
                                .and_then(|cells| cells.get(a..b))
                    }))
        })
    }
    /// Accept unchanged selected cells once per new displayed source. Subsequent idle paint
    /// and copy validation use Arc identity; incompatible sources are discarded immediately.
    pub fn refresh(&mut self, display: &Display) -> bool {
        if !self.valid(display) {
            self.clear();
            return false;
        }
        self.selected.as_mut().unwrap().display = display.clone();
        true
    }
    pub fn extend(&mut self, display: &Display, at: [f32; 2]) -> bool {
        self.extend_changed(display, at);
        self.dragging
    }
    /// Report a changed endpoint or invalidated selection without allocating ranges.
    /// Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    pub fn extend_changed(&mut self, display: &Display, at: [f32; 2]) -> bool {
        if !self.dragging {
            return false;
        }
        if !self.refresh(display) {
            return true;
        }
        let selection = self.selected.as_mut().unwrap();
        let end = display.point(at);
        let changed = selection.end != end;
        selection.end = end;
        changed
    }
    pub fn release(&mut self) {
        self.dragging = false;
    }
    /// Row ranges are half-open and expand a boundary inside a wide glyph.
    pub fn ranges(&self, display: &Display) -> Vec<(usize, usize, usize)> {
        if !self.valid(display) {
            return vec![];
        }
        Self::selected_ranges(self.selected.as_ref().unwrap())
    }
    fn selected_ranges(s: &Selection) -> Vec<(usize, usize, usize)> {
        let display = &s.display;
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
    use bbt_core::term::TermMode;
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
            historical_row: None,
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
    fn markdown_paths_wikilinks_aliases_and_wrapped_locators_use_displayed_cells() {
        let d = display(b"see (e2e/target.md:12:3), [[target|an alias]]", 60, 2);
        assert_eq!(d.document_at([100., 1.]).as_deref(), Some("e2e/target.md"));
        assert_eq!(d.document_at([370., 1.]).as_deref(), Some("target"));
        assert_eq!(d.document_at([10., 1.]), None);
        let d = display(b"[[target|a long alias]]", 12, 3);
        assert_eq!(d.document_at([20., 21.]).as_deref(), Some("target"));
        for token in [
            "https://example.com/a.md",
            "a.md.bak",
            "a.md:no",
            "[[unfinished",
        ] {
            assert_eq!(
                display(token.as_bytes(), 50, 2).document_at([10., 1.]),
                None,
                "{token}"
            );
        }
    }
    #[test]
    fn terminal_links_osc8_plain_wrapped_urls_and_inert_schemes() {
        let d = display(b"see https://example.com/a then", 40, 2);
        assert_eq!(
            d.link_at([70., 1.]).as_deref(),
            Some("https://example.com/a")
        );
        assert_eq!(d.link_at([0., 1.]), None);
        let d = display(b"https://example.com/path", 12, 3);
        assert_eq!(
            d.link_at([10., 21.]).as_deref(),
            Some("https://example.com/path")
        );
        let d = display(
            b"\x1b]8;;https://example.com/hidden\x1b\\label\x1b]8;;\x1b\\",
            40,
            2,
        );
        assert_eq!(
            d.link_at([10., 1.]).as_deref(),
            Some("https://example.com/hidden")
        );
        for uri in [
            "javascript:alert(1)",
            "data:text/html,hi",
            "file://remote/tmp/no",
            "https://evil.example/\nrun",
        ] {
            assert!(!web_link(uri));
        }
        let d = display(
            b"\x1b]8;;javascript:alert(1)\x1b\\label\x1b]8;;\x1b\\",
            40,
            2,
        );
        assert!(d.link_at([10., 1.]).is_none());
        assert!(d.link_at([-10., 1.]).is_none());
    }
    #[test]
    fn repeated_drag_inside_a_cell_or_at_a_clamped_edge_has_no_changed_work() {
        let d = display(b"abcdef", 8, 2);
        let mut selection = SelectionState::default();
        assert!(selection.begin(d.clone(), [0., 1.], false));
        for _ in 0..1000 {
            assert!(!selection.extend_changed(&d, [1., 2.]));
            assert!(selection.dragging);
        }
        assert!(selection.extend_changed(&d, [20., 2.]));
        assert_eq!(selection.text(&d).as_deref(), Some("ab"));
        assert!(selection.extend_changed(&d, [1000., 1000.]));
        for _ in 0..1000 {
            assert!(!selection.extend_changed(&d, [2000., 2000.]));
        }
        let mut replaced = d.clone();
        replaced.view.generation += 1;
        assert!(selection.extend_changed(&replaced, [20., 2.]));
        assert!(!selection.dragging);
        assert!(selection.text(&replaced).is_none());
        assert!(!selection.extend_changed(&replaced, [20., 2.]));
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
        assert_eq!(
            select(&d, [0., 1.], [0., 21.]).text(&d).as_deref(),
            Some("ab\n")
        );
        let d = display(b"abcdefgh", 5, 3);
        assert_eq!(
            select(&d, [0., 1.], [30., 21.]).text(&d).as_deref(),
            Some("abcdefgh")
        );
    }
    #[test]
    fn selection_survives_cursor_metadata_and_unselected_output_but_not_overwritten_cells() {
        let d = display(b"abcdef\r\nother", 10, 2);
        let s = select(&d, [0., 1.], [30., 1.]);
        let mut next = d.clone();
        let screen = Arc::make_mut(&mut next.screen);
        screen.cursor = (1, 2);
        screen.title = "new title".into();
        screen.cells[1][0].c = 'X';
        screen.cells[0][8].c = 'Y';
        assert_eq!(s.text(&next).as_deref(), Some("abc"));
        Arc::make_mut(&mut next.screen).cells[0][1].c = 'X';
        assert_eq!(s.text(&next), None);
    }
    #[test]
    fn selection_refresh_rebases_accepted_arc_and_discards_overwritten_source() {
        let d = display(b"abcdef\r\nother", 10, 2);
        let mut s = select(&d, [0., 1.], [30., 1.]);
        let mut next = d.clone();
        Arc::make_mut(&mut next.screen).cursor = (1, 2);
        assert!(!Arc::ptr_eq(
            &s.selected.as_ref().unwrap().display.screen,
            &next.screen
        ));
        assert!(s.refresh(&next));
        assert!(Arc::ptr_eq(
            &s.selected.as_ref().unwrap().display.screen,
            &next.screen
        ));
        assert_eq!(s.text(&next).as_deref(), Some("abc"));
        Arc::make_mut(&mut next.screen).cells[0][1].c = 'X';
        assert!(!s.refresh(&next));
        assert!(s.selected.is_none());
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
        assert_eq!(s.text(&other).as_deref(), Some("abc"));
        let mut other = d.clone();
        other.historical_row = Some(42);
        assert_eq!(s.text(&other), None);
        let mut other = d.clone();
        other.rect.x += 1.;
        assert_eq!(s.text(&other), None);
        let mut other = d.clone();
        Arc::make_mut(&mut other.screen).modes = TermMode::MOUSE_REPORT_CLICK.bits() as u64;
        let mut s = SelectionState::default();
        assert!(s.begin(other.clone(), [0., 1.], false));
        assert!(s.begin(other, [0., 1.], true));
    }
}
