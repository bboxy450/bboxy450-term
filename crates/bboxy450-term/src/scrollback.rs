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
}

/// One coalesced intent per pane, even while its IO queue is full or disconnected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub anchor: Anchor,
    pub lines: i64,
}

impl Request {
    pub const fn lines(lines: i64) -> Self {
        Self {
            anchor: Anchor::Current,
            lines,
        }
    }

    pub const fn top() -> Self {
        Self {
            anchor: Anchor::Top,
            lines: 0,
        }
    }

    pub const fn live() -> Self {
        Self {
            anchor: Anchor::Live,
            lines: 0,
        }
    }

    /// Relative motions accumulate; a new absolute intent replaces the old one.
    pub fn merge(&mut self, next: Self) {
        if next.anchor == Anchor::Current {
            self.lines = self.lines.saturating_add(next.lines);
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
}

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
        let row_bytes = screen
            .cols
            .checked_mul(mem::size_of::<Cell>())
            .and_then(|bytes| bytes.checked_add(mem::size_of::<Vec<Cell>>()))
            .ok_or("history_viewport_exceeds_budget")?;
        let view_bytes = row_bytes
            .checked_mul(screen.rows)
            .ok_or("history_viewport_exceeds_budget")?;
        if screen.cols < 2
            || screen.rows == 0
            || screen.cols > 512
            || screen.rows > 256
            || view_bytes.saturating_mul(4) + PAGE_RESERVE >= HEAP_BUDGET
        {
            return Err("history_viewport_exceeds_budget");
        }
        let base = Term::new_with_spare_rows(screen.cols, screen.rows, 0, 0);
        let display_heap = screen_heap(&screen) + 2 * screen.rows * mem::size_of::<GapMarker>();
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
        self.page = None;
        self.displayed = None;
        self.gaps.clear();
        self.gaps.shrink_to_fit();
        self.epoch = Ulid::new();
    }

    fn request(&mut self, request: Request, through: u64, live: &Arc<Screen>) -> bool {
        self.goal = match request.anchor {
            Anchor::Top => Goal::Row(0u64.saturating_add_signed(request.lines.saturating_neg())),
            Anchor::Live => Goal::BehindLive(request.lines.max(0) as u64),
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
        if self.page.is_some() {
            return Err("history_page_already_retained");
        }
        self.page = Some(Page {
            capacity: records.capacity(),
            records: records.into_iter(),
            current: None,
            next,
        });
        if self
            .retained_heap_bytes()
            .is_none_or(|bytes| bytes > HEAP_BUDGET)
        {
            self.page = None;
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
        self.display_heap = self
            .display_heap
            .max(screen_heap(&screen) + gaps.capacity() * mem::size_of::<GapMarker>());
        self.displayed = Some(row);
        Some(Arc::new(Historical {
            screen: Arc::new(screen),
            gaps,
        }))
    }

    fn step(&mut self) -> Result<Step, &'static str> {
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
        if self
            .retained_heap_bytes()
            .is_none_or(|bytes| bytes > HEAP_BUDGET)
        {
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
        if self
            .retained_heap_bytes()
            .is_none_or(|bytes| bytes > HEAP_BUDGET)
        {
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
}

impl History {
    pub fn request(
        &mut self,
        seat: SeatId,
        view: View,
        sequence: u64,
        screen: Arc<Screen>,
        request: Request,
    ) -> Result<(), &'static str> {
        if !self.seats.contains_key(&seat) {
            if request.anchor != Anchor::Top && request.lines <= 0 {
                return Ok(());
            }
            let mut playback = Playback::new(view, sequence, screen.clone())?;
            playback.goal = Goal::BehindLive(0);
            self.seats.insert(seat, playback);
        }
        if !self
            .seats
            .get_mut(&seat)
            .unwrap()
            .request(request, sequence, &screen)
        {
            self.seats.remove(&seat);
        }
        Ok(())
    }

    pub fn forget(&mut self, seat: SeatId) {
        self.seats.remove(&seat);
        if self.seats.is_empty() {
            self.retry_at = None;
        }
    }

    pub fn retain(&mut self, valid: impl Fn(SeatId, View, &Screen) -> bool) {
        self.seats
            .retain(|&seat, playback| valid(seat, playback.view, &playback.template));
    }

    pub fn disconnected(&mut self) {
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
    pub fn rejected(&mut self, retry: bool, now: Instant) -> Option<SeatId> {
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
            self.forget(flight.seat);
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
    pub fn step(&mut self) -> Option<(SeatId, Result<Option<Arc<Historical>>, &'static str>)> {
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
        match self.seats.get_mut(&seat).unwrap().step() {
            Ok(Step::Pending) => None,
            Ok(Step::Display(screen)) => Some((seat, Ok(Some(screen)))),
            Ok(Step::Live) => {
                self.seats.remove(&seat);
                Some((seat, Ok(None)))
            }
            Err(reason) => {
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
