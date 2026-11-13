// Project integration: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Existing Apache-2.0 crate terms and retained upstream notices apply.
// Credit bboxy450 integration accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![doc = include_str!("../README.md")]
#![warn(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]

use std::mem;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub mod keyboard;

mod vendor;

pub use vendor::alacritty::{event, grid, index, sync, term};
pub use vendor::vte;
// Preserve the vendored parser's original crate-relative references.
pub use ansi::{Color, NamedColor, Rgb};
pub use event::Event;
pub use grid::{Dimensions, Grid};
pub use index::{Column, Line, Point};
pub use term::cell::Cell;
pub use vte::{Params, ParamsIter, Parser, Perform, ansi};

use event::EventListener;
use term::Term as Model;

#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<Event>>>);

impl EventListener for Events {
    fn send_event(&self, event: Event) {
        self.0.lock().push(event);
    }
}

type OscHook = Box<dyn Fn(&[&[u8]]) + Send + Sync>;

/// A complete model/parser snapshot, including incomplete input and both cursors.
///
/// Runtime listeners, callbacks and queued output events are excluded. The embedding host must
/// version and validate its enclosing checkpoint record before restoring it.
#[derive(Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    model: Model<Events>,
    processor: ansi::Processor,
}

/// One terminal model. Its owner provides bytes and drains generated events.
pub struct Term {
    model: Model<Events>,
    processor: ansi::Processor,
    sink: Events,
    events: Vec<Event>,
    osc_hook: Option<OscHook>,
}

impl Term {
    /// Construct a grid with an explicit bounded scrollback limit.
    ///
    /// # Panics
    /// Panics if there are fewer than two columns or fewer than one visible row.
    pub fn new(cols: usize, rows: usize, history_size: usize) -> Self {
        assert!(
            cols >= term::MIN_COLUMNS,
            "terminal needs at least two columns"
        );
        assert!(
            rows >= term::MIN_SCREEN_LINES,
            "terminal needs at least one row"
        );
        let sink = Events::default();
        let size = Size { cols, rows };
        let config = term::Config {
            scrolling_history: history_size,
            kitty_keyboard: true,
            ..Default::default()
        };
        Self {
            model: Model::new(config, &size, sink.clone()),
            processor: ansi::Processor::new(),
            sink,
            events: Vec::new(),
            osc_hook: None,
        }
    }

    /// Construct a replay terminal without the upstream 1000-row allocation
    /// cache. A zero spare-row budget allocates only actual retained rows.
    /// Ordinary live terminals keep the unchanged [`Term::new`] policy.
    ///
    /// # Panics
    /// Panics for dimensions rejected by `Term::new`.
    pub fn new_with_spare_rows(
        cols: usize,
        rows: usize,
        history_size: usize,
        spare_rows: usize,
    ) -> Self {
        let mut term = Self::new(cols, rows, history_size);
        term.model.set_spare_rows(spare_rows);
        term.processor.release_spare_capacity();
        term
    }

    /// Conservative retained allocation size for replay budget accounting.
    /// Includes both grids, spare capacity, parser/sync buffers and cell extras;
    /// shared cell extras are counted per reference. Drain [`Term::take_events`]
    /// first. Returns None while effects or an opaque user callback are retained.
    /// The inline Term size is included for owners that keep it on their heap.
    pub fn retained_heap_bytes(&self) -> Option<usize> {
        let queued = self.sink.0.lock();
        if self.osc_hook.is_some() || !self.events.is_empty() || !queued.is_empty() {
            return None;
        }
        Some(
            std::mem::size_of::<Self>()
                + std::mem::size_of::<Mutex<Vec<Event>>>()
                + 2 * std::mem::size_of::<usize>()
                + (queued.capacity() + self.events.capacity()) * std::mem::size_of::<Event>()
                + self.model.retained_heap_bytes()
                + self.processor.retained_heap_bytes(),
        )
    }

    /// Stable half-open row range resident in the active history and viewport.
    /// Rows evicted by the history limit remain addressable by replaying bytes
    /// into a fresh terminal; this method does not fetch or keep those bytes.
    pub fn history_bounds(&self) -> std::ops::Range<u64> {
        let grid = self.grid();
        let origin = grid.rows_scrolled();
        origin.saturating_sub(grid.history_size() as u64)
            ..origin.saturating_add(grid.screen_lines() as u64)
    }

    /// Copy a bounded row window, including history above the live viewport.
    /// Return None for a nonresident range, so callers seek instead of silently
    /// showing a different row. The UI toolkit and scroll policy stay outside.
    pub fn history_window(&self, first: u64, rows: usize) -> Option<Vec<Vec<Cell>>> {
        let bounds = self.history_bounds();
        let end = first.checked_add(rows as u64)?;
        if first < bounds.start || end > bounds.end {
            return None;
        }
        let grid = self.grid();
        let origin = grid.rows_scrolled();
        Some(
            (first..end)
                .map(|row| {
                    let line = Line((i128::from(row) - i128::from(origin)) as i32);
                    (0..grid.columns())
                        .map(|col| grid[line][Column(col)].clone())
                        .collect()
                })
                .collect(),
        )
    }

    /// Parse a chunk without requiring escape or UTF-8 boundaries.
    pub fn advance(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.model, bytes);
        self.collect_events();
    }

    /// Kitty graphics: the images and placements this terminal holds.
    pub fn graphics(&self) -> &bboxy450_kitty_graphics::Graphics {
        &self.model.graphics
    }
    /// Rows that have scrolled off the top since the first screen; the
    /// absolute row of screen line `l` is this plus `l`.
    pub fn rows_scrolled(&self) -> u64 {
        self.model.grid().rows_scrolled()
    }
    /// Borrow the active grid, including its bounded retained history.
    pub fn grid(&self) -> &Grid<Cell> {
        self.model.grid()
    }

    /// Apply an ordered viewport resize while preserving parser and model state.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        self.model.resize(Size {
            cols: cols.max(term::MIN_COLUMNS),
            rows: rows.max(term::MIN_SCREEN_LINES),
        });
        self.collect_events();
    }

    /// Drop retained history and keep both screens at history size zero.
    pub fn viewport_only(&mut self) {
        self.model.viewport_only();
    }

    /// Borrow the active terminal-mode bitset.
    pub fn mode(&self) -> &term::TermMode {
        self.model.mode()
    }

    /// Current terminal palette overrides.
    pub fn colors(&self) -> &term::color::Colors {
        self.model.colors()
    }

    /// Active cursor style, including the configured default.
    pub fn cursor_style(&self) -> ansi::CursorStyle {
        self.model.cursor_style()
    }

    /// Clone model and parser state without live callbacks or queued output effects.
    pub fn checkpoint(&self) -> Checkpoint {
        let mut model = self.model.clone();
        model.compact_checkpoint();
        model.replace_event_proxy(Events::default());
        Checkpoint {
            model,
            processor: self.processor.clone(),
        }
    }

    /// Restore data without replaying old output effects or replacing the live hook.
    pub fn restore(&mut self, mut checkpoint: Checkpoint) {
        self.events.clear();
        self.sink.0.lock().clear();
        checkpoint.model.replace_event_proxy(self.sink.clone());
        checkpoint.model.mark_fully_damaged();
        self.model = checkpoint.model;
        self.processor = checkpoint.processor;
    }

    /// Observe raw, semicolon-separated parameters of unrecognized OSC commands.
    pub fn on_unknown_osc<F>(&mut self, hook: F)
    where
        F: Fn(&[&[u8]]) + Send + Sync + 'static,
    {
        self.osc_hook = Some(Box::new(hook));
    }

    /// Drain generated events, including PTY responses; the model never writes a PTY.
    pub fn take_events(&mut self) -> Vec<Event> {
        mem::take(&mut self.events)
    }

    /// Return whether synchronized-output parsing retains an unfinished batch.
    pub fn sync_pending(&self) -> bool {
        self.processor.sync_pending()
    }

    /// Deadline for the owner's synchronized-output timer (not a persistent clock).
    pub fn sync_timeout(&self) -> Option<Instant> {
        self.processor.sync_timeout().sync_timeout()
    }

    /// Finish buffered synchronized output and collect its newly generated events.
    pub fn stop_sync(&mut self) {
        self.processor.stop_sync(&mut self.model);
        self.collect_events();
    }

    fn collect_events(&mut self) {
        let events = mem::take(&mut *self.sink.0.lock());
        for event in &events {
            if let (Event::Osc(params), Some(hook)) = (event, &self.osc_hook) {
                let params: Vec<&[u8]> = params.iter().map(Vec::as_slice).collect();
                hook(&params);
            }
        }
        self.events.extend(events);
    }
}

struct Size {
    cols: usize,
    rows: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

#[cfg(test)]
// keeps: invariant kitty-keyboard
#[test]
fn kitty_keyboard_invariant() {
    keyboard::assert_invariant();
}

#[cfg(test)]
mod kitty_tests {
    use super::*;
    // State: an empty 20x5 terminal with its cursor at row 1. Safety: a chunked
    // PNG transmit-and-display keeps one image, one placement at the cursor,
    // moves the cursor past the picture, answers nothing when quiet, and a
    // checkpoint round-trips it; scrolling the picture off the top forgets it.
    #[test]
    fn kitty_graphics_place_reply_scroll_and_checkpoint() {
        let mut term = Term::new(20, 5, 100);
        term.advance(b"\n");
        let image = bboxy450_kitty_graphics::Image::new(0, 6, 4, vec![200; 6 * 4 * 4]);
        let png = image.png();
        term.advance(&bboxy450_kitty_graphics::encode_png(&png, 9, 3, 2));
        let placements = term.graphics().placements().to_vec();
        assert_eq!(placements.len(), 1);
        assert_eq!((placements[0].row, placements[0].col, placements[0].cols, placements[0].rows), (1, 0, 3, 2));
        assert_eq!(term.grid().cursor.point, Point::new(Line(2), Column(3)));
        assert!(term.take_events().iter().all(|e| !matches!(e, Event::PtyWrite(_))), "quiet=1 answers nothing");
        // A loud query answers OK on the PTY.
        term.advance(b"\x1b_Ga=q,i=4,f=32,s=1,v=1;AAAAAA==\x1b\\");
        let replies: Vec<String> = term
            .take_events()
            .into_iter()
            .filter_map(|e| match e {
                Event::PtyWrite(s) => Some(s),
                _ => None,
            })
            .collect();
        assert_eq!(replies, vec!["\x1b_Gi=4;OK\x1b\\".to_owned()]);
        let checkpoint = term.checkpoint();
        let mut restored = Term::new(20, 5, 100);
        restored.restore(checkpoint);
        assert_eq!(restored.graphics().placements(), term.graphics().placements());
        assert_eq!(restored.graphics().image(9).unwrap().rgba, image.rgba);
        // Six linefeeds scroll the two picture rows past the top.
        for _ in 0..8 {
            term.advance(b"\n");
        }
        assert!(term.graphics().placements().is_empty());
        assert!(term.rows_scrolled() >= 6);
        // An oversized APC string is dropped whole, never half-applied.
        let mut huge = b"\x1b_Ga=T,f=32,s=1,v=1,i=5;".to_vec();
        huge.extend(std::iter::repeat_n(b'A', 9000));
        huge.extend(b"\x1b\\");
        term.advance(&huge);
        assert!(term.graphics().image(5).is_none());
        term.advance(b"\rafter");
        assert_eq!(term.grid()[Line(4)][Column(0)].c, 'a');
    }
}
