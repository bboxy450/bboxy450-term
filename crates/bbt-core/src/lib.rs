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
