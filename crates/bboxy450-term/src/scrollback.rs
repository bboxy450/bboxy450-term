//! IO-owned historical pane views, separate from the live terminal replica.

use std::{
    collections::BTreeMap,
    mem,
    sync::Arc,
    time::{Duration, Instant},
};

use bbt_core::{Cell, Term};
use bbt_mux::Ulid;
use bbt_proto::{GuiAction, Screen, SeatId, View, gui::HistoryRecord};

pub const HEAP_BUDGET: usize = 4_000_000;
const MAX_LINES: usize = 10_000;
// The GUI body is at most 1 MiB. Reserve space for decoded records/byte vectors,
// not just their wire bytes. Actual retained capacities are checked below too.
const PAGE_RESERVE: usize = 2_000_000;
const QUANTUM: usize = 4096;
const HISTORY_RETRY: Duration = Duration::from_millis(100);

/// Where a scroll intent starts. Positive line deltas move toward earlier output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Current,
    Top,
    Live,
    Row(u64),
}

/// One coalesced intent per pane, even while its IO queue is full or disconnected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub anchor: Anchor,
    pub lines: i64,
    /// Local window-operation correlation, never a history/bus cursor.
    pub op: Option<Ulid>,
}

impl Request {
    pub const fn lines(lines: i64) -> Self {
        Self {
            anchor: Anchor::Current,
            lines,
            op: None,
        }
    }

    pub const fn top() -> Self {
        Self {
            anchor: Anchor::Top,
            lines: 0,
            op: None,
        }
    }

    pub const fn live() -> Self {
        Self {
            anchor: Anchor::Live,
            lines: 0,
            op: None,
        }
    }

    pub const fn row(row: u64) -> Self {
        Self {
            anchor: Anchor::Row(row),
            lines: 0,
            op: None,
        }
    }
    pub fn correlated(mut self, op: Ulid) -> Self {
        self.op = Some(op);
        self
    }

    /// Relative motions accumulate; a new absolute intent replaces the old one.
    pub fn merge(&mut self, next: Self) {
        if next.anchor == Anchor::Current {
            self.lines = self.lines.saturating_add(next.lines);
            self.op = next.op;
        } else {
            *self = next;
        }
    }
}

#[derive(Clone, Copy)]
enum Goal {
    Row(u64),
    BehindLive(u64),
}

struct Page {
    records: std::vec::IntoIter<HistoryRecord>,
    capacity: usize,
    current: Option<(u64, Vec<u8>, usize)>,
    next: Option<u64>,
}
impl Page {
    fn heap_bytes(&self) -> usize {
        self.capacity * mem::size_of::<HistoryRecord>()
            + self
                .records
                .as_slice()
                .iter()
                .map(|record| match record {
                    HistoryRecord::Bytes { bytes, .. } => bytes.capacity(),
                    HistoryRecord::Gap { reason, .. } => reason.capacity(),
                })
                .sum::<usize>()
            + self
                .current
                .as_ref()
                .map_or(0, |(_, bytes, _)| bytes.capacity())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GapMarker {
    pub row: usize,
    pub from: u64,
    pub to: u64,
}

/// Cells and marker coordinates from the same replay step are published together.
#[derive(Clone)]
pub struct Historical {
    pub screen: Arc<Screen>,
    pub gaps: Vec<GapMarker>,
    pub top_row: u64,
}

pub use bbt_proto::gui::UiScrollPosition as Position;
#[derive(Clone, Debug)]
pub struct Reply {
    pub id: Ulid,
    pub seat: SeatId,
    pub generation: u64,
    pub result: Result<Position, String>,
}
#[derive(Clone, Copy)]
struct Pending {
    id: Ulid,
    seat: SeatId,
    generation: u64,
}
#[derive(Clone, Default)]
pub struct Readback {
    pub reply: Option<Reply>,
    live: BTreeMap<SeatId, (View, u64, u64)>,
}
impl Readback {
    pub fn position(&self, seat: SeatId, historical: Option<&Historical>) -> Position {
        historical.map_or_else(
            || Position {
                top_row: self.live.get(&seat).map(|(_, _, row)| *row),
                live: true,
            },
            |history| Position {
                top_row: Some(history.top_row),
                live: false,
            },
        )
    }
    pub fn observe(&mut self, seat: SeatId, baseline: Option<(View, u64)>) -> bool {
        if self
            .live
            .get(&seat)
            .is_some_and(|(view, sequence, _)| baseline != Some((*view, *sequence)))
        {
            self.live.remove(&seat);
            true
        } else {
            false
        }
    }
    pub fn observed(&self, seat: SeatId, baseline: Option<(View, u64)>) -> bool {
        self.live
            .get(&seat)
            .is_some_and(|(view, sequence, _)| baseline != Some((*view, *sequence)))
    }
    pub fn displayed(
        &mut self,
        seat: SeatId,
        current: Option<(View, u64)>,
        baseline: (View, u64),
        position: Position,
    ) {
        if position.live
            && current == Some(baseline)
            && let Some(row) = position.top_row
        {
            self.live.insert(seat, (baseline.0, baseline.1, row));
        } else {
            self.live.remove(&seat);
        }
    }
    pub fn forget(&mut self, seat: SeatId) {
        self.live.remove(&seat);
    }
    pub fn retain(&mut self, valid: impl Fn(SeatId) -> bool) {
        self.live.retain(|seat, _| valid(*seat));
    }
}

pub struct DisplayPosition {
    pub seat: SeatId,
    pub baseline: (View, u64),
    pub position: Position,
}

pub type DisplayUpdate = (SeatId, Result<Option<Arc<Historical>>, &'static str>);

struct Gap {
    row: u64,
    from: u64,
    to: u64,
}

/// A bounded terminal replay. It is constructed and advanced on IO only;
/// publication exposes just a viewport-sized immutable Screen to the UI.
pub struct Playback {
    term: Term,
    template: Arc<Screen>,
    view: View,
    epoch: Ulid,
    through: u64,
    sequence: u64,
    next: u64,
    page: Option<Page>,
    complete: bool,
    live_row: Option<u64>,
    goal: Goal,
    displayed: Option<u64>,
    history_lines: usize,
    gaps: Vec<Gap>,
    display_heap: usize,
    checked_heap: Option<usize>,
    pages_fetched: u64,
    pages_evicted: u64,
}

enum Step {
    Pending,
    Display(Arc<Historical>),
    Live,
}

fn json_heap(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::String(value) => value.capacity(),
        serde_json::Value::Array(values) => {
            values.capacity() * mem::size_of::<serde_json::Value>()
                + values.iter().map(json_heap).sum::<usize>()
        }
        // Wire-decoded palette RGB/cursor objects fit in one map node. Larger
        // objects reserve a full node per entry instead of guessing utilization.
        serde_json::Value::Object(values) => {
            (if values.is_empty() {
                0
            } else if values.len() <= 11 {
                1024
            } else {
                values.len() * 1024
            }) + values
                .iter()
                .map(|(key, value)| key.capacity() + json_heap(value))
                .sum::<usize>()
        }
        _ => 0,
    }
}

fn screen_heap(screen: &Screen) -> usize {
    mem::size_of::<Screen>()
        + screen.cells.capacity() * mem::size_of::<Vec<Cell>>()
        + screen
            .cells
            .iter()
            .map(|row| {
                row.capacity() * mem::size_of::<Cell>()
                    + row.iter().map(Cell::retained_extra_bytes).sum::<usize>()
            })
            .sum::<usize>()
        + screen.title.capacity()
        + screen.cwd.as_ref().map_or(0, String::capacity)
        + screen.gap.as_ref().map_or(0, String::capacity)
        + json_heap(&screen.palette)
        + json_heap(&screen.cursor_style)
}

impl Playback {
    pub fn new(view: View, through: u64, screen: Arc<Screen>) -> Result<Self, &'static str> {
        bbt_rt::assert_not_ui_thread();
        // These are the core Term's minimum dimensions, not a heap limit.
        if screen.cols < 2 || screen.rows == 0 {
            return Err("history_dimensions_unsupported");
        }
        let row_bytes = screen
            .cols
            .checked_mul(mem::size_of::<Cell>())
            .and_then(|bytes| bytes.checked_add(mem::size_of::<Vec<Cell>>()))
            .ok_or("history_viewport_exceeds_budget")?;
        let view_bytes = row_bytes
            .checked_mul(screen.rows)
            .ok_or("history_viewport_exceeds_budget")?;
        if view_bytes.saturating_mul(4).saturating_add(PAGE_RESERVE) >= HEAP_BUDGET {
            return Err("history_viewport_exceeds_budget");
        }
        let base = Term::new_with_spare_rows(screen.cols, screen.rows, 0, 0);
        let display_heap = screen_heap(&screen)
            + mem::size_of::<Historical>()
            + 2 * screen.rows * mem::size_of::<GapMarker>();
        let mandatory = base
            .retained_heap_bytes()
            .ok_or("history_heap_accounting_incomplete")?
            + screen_heap(&screen)
            + 2 * display_heap;
        drop(base);
        let lines = HEAP_BUDGET.saturating_sub(mandatory + PAGE_RESERVE) / row_bytes;
        let minimum = screen.rows.saturating_mul(3);
        if lines < minimum {
            return Err("history_viewport_exceeds_budget");
        }
        let history_lines = lines.min(MAX_LINES);
        Ok(Self {
            term: Term::new_with_spare_rows(screen.cols, screen.rows, history_lines, 0),
            template: screen,
            view,
            epoch: Ulid::new(),
            through,
            sequence: 0,
            next: 1,
            page: None,
            complete: through == 0,
            live_row: None,
            goal: Goal::BehindLive(1),
            displayed: None,
            history_lines,
            gaps: Vec::new(),
            display_heap,
            checked_heap: None,
            pages_fetched: 0,
            pages_evicted: 0,
        })
    }

    /// Includes the held baseline and room for the published historical view.
    /// The core counts shared cell extras conservatively before these clones.
    pub fn retained_heap_bytes(&self) -> Option<usize> {
        Some(
            self.term.retained_heap_bytes()?
                + screen_heap(&self.template)
                + 2 * self.display_heap
                + self.gaps.capacity() * mem::size_of::<Gap>()
                + self.page.as_ref().map_or(0, Page::heap_bytes)
                + mem::size_of::<Self>(),
        )
    }

    fn check_heap(&mut self) -> Option<usize> {
        let bytes = self.retained_heap_bytes();
        self.checked_heap = bytes;
        bytes
    }

    fn evict_page(&mut self) {
        if self.page.take().is_some() {
            self.pages_evicted = self.pages_evicted.saturating_add(1);
        }
    }

    fn row(&self) -> Option<u64> {
        match self.goal {
            Goal::Row(row) => Some(row),
            Goal::BehindLive(lines) => self.live_row.map(|row| row.saturating_sub(lines)),
        }
    }

    fn resident(&self, row: u64) -> bool {
        let bounds = self.term.history_bounds();
        row >= bounds.start
            && row.saturating_add(self.template.rows as u64) <= bounds.end
            && (self.complete || bounds.end - self.template.rows as u64 > row)
    }

    fn needs_work(&self) -> bool {
        self.row().is_none_or(|row| !self.resident(row))
    }

    fn runnable(&self) -> bool {
        (self.page.is_some() && self.needs_work())
            || self.row().is_some_and(|row| {
                row < self.term.history_bounds().start
                    || (self.resident(row) && self.displayed != Some(row))
                    || (self.complete && self.live_row.is_some_and(|end| row >= end))
            })
    }

    fn restart(&mut self) {
        self.term = Term::new_with_spare_rows(
            self.template.cols,
            self.template.rows,
            self.history_lines,
            0,
        );
        self.sequence = 0;
        self.next = 1;
        self.complete = self.through == 0;
        self.evict_page();
        self.checked_heap = None;
        self.displayed = None;
        self.gaps.clear();
        self.gaps.shrink_to_fit();
        self.epoch = Ulid::new();
    }

    fn request(&mut self, request: Request, through: u64, live: &Arc<Screen>) -> bool {
        self.goal = match request.anchor {
            Anchor::Top => Goal::Row(0u64.saturating_add_signed(request.lines.saturating_neg())),
            Anchor::Live => Goal::BehindLive(request.lines.max(0) as u64),
            Anchor::Row(row) => {
                Goal::Row(row.saturating_add_signed(request.lines.saturating_neg()))
            }
            Anchor::Current => match self.goal {
                Goal::Row(row) => {
                    Goal::Row(row.saturating_add_signed(request.lines.saturating_neg()))
                }
                Goal::BehindLive(lines) => {
                    Goal::BehindLive(lines.saturating_add_signed(request.lines))
                }
            },
        };
        if matches!(self.goal, Goal::BehindLive(0)) {
            return false;
        }
        if let Some(row) = self.row() {
            if self.complete && self.live_row.is_some_and(|end| row >= end) {
                if through <= self.through {
                    return false;
                }
                // New output is fetched only when navigation reaches that end.
                self.through = through;
                self.template = live.clone();
                self.complete = false;
                self.live_row = None;
            }
            if row < self.term.history_bounds().start {
                self.restart();
            }
        }
        true
    }

    fn accept(
        &mut self,
        records: Vec<HistoryRecord>,
        next: Option<u64>,
    ) -> Result<(), &'static str> {
        let _sample = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::HistoryPage).work(1, 0);
        if self.page.is_some() {
            return Err("history_page_already_retained");
        }
        self.page = Some(Page {
            capacity: records.capacity(),
            records: records.into_iter(),
            current: None,
            next,
        });
        self.pages_fetched = self.pages_fetched.saturating_add(1);
        if self.check_heap().is_none_or(|bytes| bytes > HEAP_BUDGET) {
            self.evict_page();
            return Err("history_heap_budget_exceeded");
        }
        Ok(())
    }

    fn display(&mut self) -> Option<Arc<Historical>> {
        let row = self.row()?;
        if self.displayed == Some(row) || !self.resident(row) {
            return None;
        }
        let cells = self.term.history_window(row, self.template.rows)?;
        let mut screen = self.template.as_ref().clone();
        screen.cells = cells;
        screen.cursor = (-1, 0);
        let gaps: Vec<_> = self
            .gaps
            .iter()
            .filter_map(|gap| {
                (gap.row >= row && gap.row < row + self.template.rows as u64).then_some(GapMarker {
                    row: gap.row.saturating_sub(row) as usize,
                    from: gap.from,
                    to: gap.to,
                })
            })
            .collect();
        // Historical cells can hold hyperlinks absent from the live viewport.
        // Reserve the actual largest published heap, including marker capacity,
        // for both the UI's previous model and the newly published model.
        self.display_heap = self.display_heap.max(
            screen_heap(&screen)
                + mem::size_of::<Historical>()
                + gaps.capacity() * mem::size_of::<GapMarker>(),
        );
        self.displayed = Some(row);
        Some(Arc::new(Historical {
            screen: Arc::new(screen),
            gaps,
            top_row: row,
        }))
    }

    fn step(&mut self) -> Result<Step, &'static str> {
        let _sample = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::HistoryStep).work(1, 0);
        if let Some(row) = self.row() {
            if row < self.term.history_bounds().start {
                self.restart();
                return Ok(Step::Pending);
            }
            if self.complete && self.live_row.is_some_and(|end| row >= end) {
                return Ok(Step::Live);
            }
        }
        let mut consumed = 0;
        let mut records_processed = 0;
        while self.needs_work() && consumed < QUANTUM && records_processed < 64 {
            let Some(page) = self.page.as_mut() else {
                break;
            };
            if let Some((sequence, bytes, offset)) = &mut page.current {
                let amount = (bytes.len() - *offset)
                    .min(self.template.rows.max(1))
                    .min(QUANTUM - consumed);
                self.term.advance(&bytes[*offset..*offset + amount]);
                // History has no presentation boundary to defer. Flush parsed
                // synchronized output inside this bounded unit, so a long BSU
                // cannot hide the requested first rows until after eviction.
                if self.term.sync_pending() {
                    self.term.stop_sync();
                }
                *offset += amount;
                consumed += amount;
                drop(self.term.take_events());
                if *offset == bytes.len() {
                    self.sequence = *sequence;
                    page.current = None;
                }
            } else if let Some(record) = page.records.next() {
                records_processed += 1;
                match record {
                    HistoryRecord::Bytes { sequence, bytes } => {
                        if sequence != self.sequence.saturating_add(1) || sequence > self.through {
                            return Err("history_capture_sequence_mismatch");
                        }
                        if bytes.is_empty() {
                            self.sequence = sequence;
                        } else {
                            page.current = Some((sequence, bytes, 0));
                        }
                    }
                    HistoryRecord::Gap { from, to, .. } => {
                        if from != self.sequence.saturating_add(1) || to < from || to > self.through
                        {
                            return Err("history_gap_sequence_mismatch");
                        }
                        self.term.advance(b"\x18");
                        if self.term.grid().cursor.point.column.0 != 0 {
                            self.term.advance(b"\r\n");
                        }
                        let row = self.term.history_bounds().end - self.template.rows as u64
                            + self.term.grid().cursor.point.line.0.max(0) as u64;
                        self.term.advance(b"\x1b[2K\r\n");
                        self.gaps.push(Gap { row, from, to });
                        drop(self.term.take_events());
                        self.sequence = to;
                    }
                }
            } else {
                let next = page.next;
                self.page = None;
                self.pages_evicted = self.pages_evicted.saturating_add(1);
                if next.is_some_and(|next| next <= self.sequence) {
                    return Err("history_page_did_not_advance");
                }
                self.next = next.unwrap_or_else(|| self.sequence.saturating_add(1));
                self.complete = self.sequence >= self.through;
                if self.complete {
                    if self.term.sync_pending() {
                        self.term.stop_sync();
                        drop(self.term.take_events());
                    }
                    self.live_row =
                        Some(self.term.history_bounds().end - self.template.rows as u64);
                }
                break;
            }
        }
        let first = self.term.history_bounds().start;
        if self.gaps.first().is_some_and(|gap| gap.row < first) {
            self.gaps.retain(|gap| gap.row >= first);
            self.gaps.shrink_to_fit();
        }
        if self.check_heap().is_none_or(|bytes| bytes > HEAP_BUDGET) {
            return Err("history_heap_budget_exceeded");
        }
        if self.complete
            && self
                .row()
                .is_some_and(|row| self.live_row.is_some_and(|end| row >= end))
        {
            return Ok(Step::Live);
        }
        let display = self.display();
        if self.check_heap().is_none_or(|bytes| bytes > HEAP_BUDGET) {
            return Err("history_heap_budget_exceeded");
        }
        Ok(display.map_or(Step::Pending, Step::Display))
    }
}

struct Flight {
    seat: SeatId,
    epoch: Ulid,
    view: View,
    from: u64,
    through: u64,
}

/// One request in flight on the multiplexed connection, one retained page per
/// scrolled seat. Parsing yields between bounded quanta; live ACKs keep flowing.
#[derive(Default)]
pub struct History {
    seats: BTreeMap<SeatId, Playback>,
    flight: Option<Flight>,
    last_request: Option<SeatId>,
    last_work: Option<SeatId>,
    retry_at: Option<Instant>,
    pending: Option<Pending>,
    completion: Option<Reply>,
    position: Option<DisplayPosition>,
}

impl History {
    /// Bounded scalar sample; heap values reuse the existing budget checks.
    /// No cell walk, model build, layout solve or history fetch is done here.
    pub fn metrics(&self, displays: &BTreeMap<SeatId, Arc<Historical>>) -> serde_json::Value {
        const LIMIT: usize = 128;
        let panes: Vec<_> = self
            .seats
            .iter()
            .take(LIMIT)
            .map(|(&seat, playback)| {
                let display = displays.get(&seat);
                let gaps: Vec<_> = display
                    .into_iter()
                    .flat_map(|d| d.gaps.iter())
                    .take(8)
                    .map(|gap| serde_json::json!({"row":gap.row,"from":gap.from,"to":gap.to}))
                    .collect();
                serde_json::json!({"seat":Ulid(seat.0.0),"generation":playback.view.generation,
                "retained_history_bytes":playback.checked_heap,"budget_bytes":HEAP_BUDGET,
                "pages_resident":usize::from(playback.page.is_some()),
                "pages_fetched":playback.pages_fetched,"pages_evicted":playback.pages_evicted,
                "top_row":display.map(|d|d.top_row),"gap_markers":gaps,
                "omitted_gap_markers":display.map_or(0,|d|d.gaps.len().saturating_sub(8))})
            })
            .collect();
        let complete = self.seats.len() <= LIMIT;
        let total = complete
            .then(|| {
                self.seats
                    .values()
                    .try_fold(mem::size_of::<Self>(), |total, p| {
                        total.checked_add(p.checked_heap?)
                    })
            })
            .flatten();
        serde_json::json!({"available":true,"basis":"GUI IO owner; conservative retained bytes from latest existing budget check, null before first check; page eviction means payload released after consumption/reset; counters live with Playback",
            "complete":complete,"omitted_panes":self.seats.len().saturating_sub(LIMIT),
            "retained_history_bytes":total,"per_pane_budget_bytes":HEAP_BUDGET,"panes":panes})
    }

    pub fn request(
        &mut self,
        seat: SeatId,
        view: View,
        sequence: u64,
        screen: Arc<Screen>,
        request: Request,
    ) -> Result<(), &'static str> {
        self.position = None;
        if let Some(id) = request.op {
            self.pending = Some(Pending {
                id,
                seat,
                generation: view.generation,
            });
            self.completion = None;
        } else {
            self.finish(seat, Err("history_superseded"));
        }
        let result = self.request_inner(seat, view, sequence, screen, request);
        match result {
            Ok(Some(position)) => {
                self.position = Some(DisplayPosition {
                    seat,
                    baseline: (view, sequence),
                    position,
                });
                self.finish(seat, Ok(position));
            }
            Err(reason) => self.finish(seat, Err(reason)),
            Ok(None) => (),
        }
        result.map(|_| ())
    }
    fn request_inner(
        &mut self,
        seat: SeatId,
        view: View,
        sequence: u64,
        screen: Arc<Screen>,
        request: Request,
    ) -> Result<Option<Position>, &'static str> {
        if let std::collections::btree_map::Entry::Vacant(entry) = self.seats.entry(seat) {
            if matches!(request.anchor, Anchor::Current | Anchor::Live) && request.lines <= 0 {
                return Ok(Some(Position::default()));
            }
            let mut playback = Playback::new(view, sequence, screen.clone())?;
            playback.goal = Goal::BehindLive(0);
            entry.insert(playback);
        }
        let playback = self.seats.get_mut(&seat).unwrap();
        if playback.view != view {
            return Err("history_view_changed");
        }
        if !playback.request(request, sequence, &screen) {
            let position = Position {
                top_row: (playback.complete && playback.through == sequence)
                    .then_some(playback.live_row)
                    .flatten(),
                live: true,
            };
            self.seats.remove(&seat);
            return Ok(Some(position));
        }
        if let Some(row) = playback.displayed
            && playback.row() == Some(row)
        {
            return Ok(Some(Position {
                top_row: Some(row),
                live: false,
            }));
        }
        Ok(None)
    }
    fn finish(&mut self, seat: SeatId, result: Result<Position, &str>) {
        if self.pending.is_some_and(|pending| pending.seat == seat) {
            let pending = self.pending.take().unwrap();
            self.completion = Some(Reply {
                id: pending.id,
                seat,
                generation: pending.generation,
                result: result.map_err(|reason| reason.chars().take(256).collect()),
            });
        }
    }
    pub fn failed(&mut self, seat: SeatId, reason: &str) {
        self.finish(seat, Err(reason));
        self.forget(seat);
    }
    pub fn take_completion(&mut self) -> Option<Reply> {
        self.completion.take()
    }
    pub fn take_position(&mut self) -> Option<DisplayPosition> {
        self.position.take()
    }

    pub fn forget(&mut self, seat: SeatId) {
        self.finish(seat, Err("history_superseded"));
        self.seats.remove(&seat);
        if self.seats.is_empty() {
            self.retry_at = None;
        }
    }

    pub fn retain(&mut self, valid: impl Fn(SeatId, View, &Screen) -> bool) {
        self.seats
            .retain(|&seat, playback| valid(seat, playback.view, &playback.template));
        if let Some(pending) = self.pending
            && !self.seats.contains_key(&pending.seat)
        {
            self.finish(pending.seat, Err("history_view_unavailable"));
        }
    }

    pub fn disconnected(&mut self) {
        if let Some(pending) = self.pending {
            self.finish(pending.seat, Err("history_disconnected"));
        }
        self.flight = None;
        self.retry_at = None;
        // A later connection cannot answer an old request. Keep the displayed
        // view, but allow its exact read-only page range to be requested again.
    }

    pub fn next_action(&mut self, now: Instant) -> Option<GuiAction> {
        if self.flight.is_some() || self.retry_at.is_some_and(|at| now < at) {
            return None;
        }
        self.retry_at = None;
        let seat = self
            .seats
            .iter()
            .filter(|(_, playback)| {
                playback.page.is_none() && !playback.complete && playback.needs_work()
            })
            .map(|(&seat, _)| seat)
            .find(|seat| self.last_request.is_none_or(|last| *seat > last))
            .or_else(|| {
                self.seats
                    .iter()
                    .find(|(_, p)| p.page.is_none() && !p.complete && p.needs_work())
                    .map(|(&s, _)| s)
            })?;
        let playback = &self.seats[&seat];
        let (from, through) = (playback.next, playback.through);
        self.flight = Some(Flight {
            seat,
            epoch: playback.epoch,
            view: playback.view,
            from,
            through,
        });
        self.last_request = Some(seat);
        Some(GuiAction::GetHistory {
            seat,
            from,
            to: through,
        })
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.retry_at
    }

    /// Only the correlated terminal result may release this request's fence.
    /// A timeout is ambiguous and never authorizes an overlapping retry.
    pub fn rejected(&mut self, retry: bool, now: Instant, reason: &str) -> Option<SeatId> {
        let flight = self.flight.take()?;
        if self
            .seats
            .get(&flight.seat)
            .is_none_or(|playback| playback.epoch != flight.epoch || playback.view != flight.view)
        {
            return None;
        }
        if retry {
            self.retry_at = Some(now + HISTORY_RETRY);
            None
        } else {
            self.failed(flight.seat, reason);
            Some(flight.seat)
        }
    }

    pub fn accept(
        &mut self,
        seat: SeatId,
        records: Vec<HistoryRecord>,
        next: Option<u64>,
    ) -> Result<(), &'static str> {
        let Some(flight) = &self.flight else {
            return Ok(());
        };
        if flight.seat != seat {
            return Ok(());
        }
        let end = records.last().map(|record| match record {
            HistoryRecord::Bytes { sequence, .. } => *sequence,
            HistoryRecord::Gap { to, .. } => *to,
        });
        // An idempotent retry can leave an old reply in the reader. It never
        // consumes the next page's fence or rewinds the parser.
        if end.is_some_and(|end| end < flight.from) {
            return Ok(());
        }
        let flight = self.flight.take().unwrap();
        if end.is_some_and(|end| end > flight.through) {
            return Err("history_reply_exceeds_requested_cursor");
        }
        let Some(playback) = self.seats.get_mut(&seat) else {
            return Ok(());
        };
        if playback.epoch != flight.epoch || playback.view != flight.view {
            return Ok(());
        }
        if records.is_empty() {
            return Err("history_empty_page");
        }
        playback.accept(records, next)
    }

    pub fn has_work(&self) -> bool {
        self.seats.values().any(Playback::runnable)
    }

    /// Return at most one changed display per IO turn.
    pub fn step(&mut self) -> Option<DisplayUpdate> {
        let seat = self
            .seats
            .iter()
            .filter(|(_, p)| p.runnable())
            .map(|(&seat, _)| seat)
            .find(|seat| self.last_work.is_none_or(|last| *seat > last))
            .or_else(|| {
                self.seats
                    .iter()
                    .find(|(_, p)| p.runnable())
                    .map(|(&s, _)| s)
            })?;
        self.last_work = Some(seat);
        self.position = None;
        let playback = self.seats.get_mut(&seat).unwrap();
        let baseline = (playback.view, playback.through);
        let result = playback.step();
        match result {
            Ok(Step::Pending) => None,
            Ok(Step::Display(screen)) => {
                let position = Position {
                    top_row: Some(screen.top_row),
                    live: false,
                };
                self.position = Some(DisplayPosition {
                    seat,
                    baseline,
                    position,
                });
                self.finish(seat, Ok(position));
                Some((seat, Ok(Some(screen))))
            }
            Ok(Step::Live) => {
                let position = Position {
                    top_row: self.seats[&seat].live_row,
                    live: true,
                };
                self.position = Some(DisplayPosition {
                    seat,
                    baseline,
                    position,
                });
                self.finish(seat, Ok(position));
                self.seats.remove(&seat);
                Some((seat, Ok(None)))
            }
            Err(reason) => {
                self.finish(seat, Err(reason));
                self.seats.remove(&seat);
                Some((seat, Err(reason)))
            }
        }
    }

    pub fn contains(&self, seat: SeatId) -> bool {
        self.seats.contains_key(&seat)
    }

    /// Conservative aggregate for diagnostics and the 50-watched-seat gate.
    pub fn retained_heap_bytes(&self) -> Option<usize> {
        self.seats
            .values()
            .try_fold(mem::size_of::<Self>(), |total, playback| {
                Some(total + playback.retained_heap_bytes()?)
            })
    }
}
