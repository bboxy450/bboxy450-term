//! Terminal grid and parser, independent of PTYs, rendering and process ownership.
//!
//! ```
//! use bbt_core::{Column, Line, Term};
//! let mut term = Term::new(80, 24, 0);
//! term.advance(b"hello");
//! assert_eq!(term.grid()[Line(0)][Column(0)].c, 'h');
//! ```

use std::mem;
use std::sync::Arc;
use std::time::Instant;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

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
/// Runtime listeners, callbacks and queued output events are excluded. The mux must
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

    /// Parse a chunk without requiring escape or UTF-8 boundaries.
    pub fn advance(&mut self, bytes: &[u8]) {
        self.processor.advance(&mut self.model, bytes);
        self.collect_events();
    }

    pub fn grid(&self) -> &Grid<Cell> {
        self.model.grid()
    }

    /// Apply an ordered viewport resize while preserving parser and model state.
    pub fn resize(&mut self, cols: usize, rows: usize) {
        assert!(cols >= term::MIN_COLUMNS);
        assert!(rows >= term::MIN_SCREEN_LINES);
        self.model.resize(Size { cols, rows });
        self.collect_events();
    }

    /// Drop retained history and keep both screens at history size zero.
    pub fn viewport_only(&mut self) {
        self.model.viewport_only();
    }

    pub fn mode(&self) -> &term::TermMode {
        self.model.mode()
    }

    pub fn checkpoint(&self) -> Checkpoint {
        let mut model = self.model.clone();
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

    pub fn sync_pending(&self) -> bool {
        self.processor.sync_pending()
    }

    /// Deadline for the owner's synchronized-output timer (not a persistent clock).
    pub fn sync_timeout(&self) -> Option<Instant> {
        self.processor.sync_timeout().sync_timeout()
    }

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
