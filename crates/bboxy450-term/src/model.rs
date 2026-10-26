//! Back-model assembly and terminal pane preparation. Socket ownership is C1a's.
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use bbt_core::{
    Color, NamedColor,
    term::{TermMode, cell::Flags},
};
use bbt_mux::Ulid;
use bbt_proto::events::{EventGap, GuiEventCursor, GuiEventPage, GuiEventPayload, GuiEventRecord};
use bbt_proto::{GuiSeat, Screen, SeatId, SeatRecord, SeatState, View, registry};
use bbt_render::{
    cells::terminal::{
        self, Cell, Cursor, CursorShape, Grid, PreparedGrid, TerminalRenderer, Underline,
    },
    device::DeviceContext,
};
use bbt_ui_model::{self as ui, layout::Rect, tokens::Theme};

use crate::grids::Grids;

pub const PENDING_SPAWNS: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnOutcome {
    Pending,
    AwaitingSeat(SeatId),
    Placed(SeatId),
    Partial { seat: SeatId, reason: String },
    Rejected(String),
    Held(String),
    Unknown(String),
}
impl SpawnOutcome {
    pub fn is_result(&self) -> bool {
        !matches!(self, Self::Pending | Self::AwaitingSeat(_))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnRequest {
    pub spec: bbt_proto::SpawnSpec,
    pub outcome: SpawnOutcome,
    pub consumed: bool,
    pub ordinal: u64,
}

const EVENT_ROWS: usize = 512;
const EVENT_BYTES: usize = 1024 * 1024;
const EVENT_STREAMS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EventLaneState {
    #[default]
    Detached,
    Attaching,
    Active,
    Held(String),
}

/// Counts within the retained event window, not a global artifact inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowArtifactCounts {
    pub created: usize,
    pub modified: usize,
    pub claimed: usize,
    pub deleted: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSnapshot {
    pub revision: u64,
    pub high: u64,
    pub next: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageError {
    pub revision: u64,
    pub reason: String,
}

/// IO-owned, immutable once published. The serialized row window is bounded
/// by both count and bytes; per-stream positions have a separate finite cap.
/// SQLite, not this cache, owns delivery and operator-read state.
#[derive(Clone, Default)]
pub struct EventLane {
    records: VecDeque<Arc<GuiEventRecord>>,
    sizes: VecDeque<usize>,
    bytes: usize,
    positions: BTreeMap<(String, u64), (Ulid, String)>,
    streams: BTreeSet<String>,
    omitted: BTreeMap<String, EventGap>,
    cursors: BTreeMap<String, GuiEventCursor>,
    page_next: BTreeMap<String, Option<u64>>,
    page_snapshots: BTreeMap<String, PageSnapshot>,
    page_errors: BTreeMap<String, PageError>,
    read_page: Vec<Arc<GuiEventRecord>>,
    read_sizes: Vec<usize>,
    read_bytes: usize,
    read_gap: Option<EventGap>,
    state: EventLaneState,
    attach: Option<Ulid>,
    artifacts: WindowArtifactCounts,
}

impl EventLane {
    pub fn records(&self) -> &VecDeque<Arc<GuiEventRecord>> {
        &self.records
    }
    pub fn omitted(&self) -> &BTreeMap<String, EventGap> {
        &self.omitted
    }
    pub fn cursors(&self) -> &BTreeMap<String, GuiEventCursor> {
        &self.cursors
    }
    /// Read-through pagination: absent means no page yet; None means its end.
    /// A sparse page's next position need not equal its last displayed row.
    pub fn pages(&self) -> &BTreeMap<String, Option<u64>> {
        &self.page_next
    }
    pub fn page_snapshots(&self) -> &BTreeMap<String, PageSnapshot> {
        &self.page_snapshots
    }
    pub fn page_errors(&self) -> &BTreeMap<String, PageError> {
        &self.page_errors
    }
    /// The latest explicit read remains separate from the rolling live window.
    /// These are the server's shared projections, including their original proof.
    pub fn read_page(&self) -> &[Arc<GuiEventRecord>] {
        &self.read_page
    }
    pub fn read_gap(&self) -> Option<&EventGap> {
        self.read_gap.as_ref()
    }
    pub fn state(&self) -> &EventLaneState {
        &self.state
    }
    pub fn artifacts(&self) -> WindowArtifactCounts {
        self.artifacts
    }
    pub fn bytes(&self) -> usize {
        self.bytes + self.read_bytes
    }

    fn register(&mut self, stream: &str) -> Result<(), &'static str> {
        if stream.is_empty()
            || stream.len() > 128
            || !stream
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        {
            return Err("invalid event stream");
        }
        if !self.streams.contains(stream) && self.streams.len() == EVENT_STREAMS {
            return Err("event stream limit reached; original retained");
        }
        self.streams.insert(stream.into());
        Ok(())
    }

    pub fn begin_attach(&mut self, id: Ulid) {
        bbt_rt::assert_not_ui_thread();
        self.attach = Some(id);
        self.state = EventLaneState::Attaching;
    }

    pub fn attach_result(&mut self, id: Ulid, error: Option<&str>) -> bool {
        bbt_rt::assert_not_ui_thread();
        if self.attach != Some(id) {
            return false;
        }
        self.status(error)
    }

    pub fn status(&mut self, error: Option<&str>) -> bool {
        bbt_rt::assert_not_ui_thread();
        let next = error.map_or(EventLaneState::Active, |reason| {
            EventLaneState::Held(reason.chars().take(256).collect())
        });
        if self.state == next {
            return false;
        }
        self.state = next;
        true
    }

    pub fn disconnected(&mut self, reason: &str) -> bool {
        bbt_rt::assert_not_ui_thread();
        self.attach = None;
        let next = EventLaneState::Held(reason.chars().take(256).collect());
        if self.state == next {
            return false;
        }
        self.state = next;
        true
    }

    pub fn apply_cursor(&mut self, mut cursor: GuiEventCursor) -> Result<bool, &'static str> {
        bbt_rt::assert_not_ui_thread();
        if !cursor_consistent(&cursor) {
            return Err("inconsistent event cursor");
        }
        self.register(&cursor.stream)?;
        if let Some(old) = self.cursors.get(&cursor.stream) {
            // An in-flight projection can predate a later read-mark reply.
            // It cannot move operator-read state backwards.
            if cursor.read < old.read || cursor.high < old.high || old == &cursor {
                return Ok(false);
            }
            cursor.admitted = cursor.admitted.max(old.admitted);
            cursor.acknowledged = cursor.acknowledged.max(old.acknowledged);
            if old == &cursor {
                return Ok(false);
            }
        }
        self.cursors.insert(cursor.stream.clone(), cursor);
        Ok(true)
    }

    pub fn apply_page(&mut self, page: GuiEventPage) -> Result<bool, &'static str> {
        bbt_rt::assert_not_ui_thread();
        if page.records.len() > 128
            || serde_json::to_vec(&page)
                .map_err(|_| "invalid event page")?
                .len()
                > 256 * 1024
            || page.next.is_some_and(|next| next > page.cursor.high)
            || page
                .records
                .iter()
                .any(|row| record_start(row).0 != page.cursor.stream)
        {
            return Err("event history page exceeds bounds or crosses streams");
        }
        if !cursor_consistent(&page.cursor) {
            return Err("inconsistent event cursor");
        }
        for row in &page.records {
            self.validate_record(row)?;
        }
        let stream = page.cursor.stream.clone();
        let high = page.cursor.high;
        self.apply_cursor(page.cursor)?;
        self.read_page.clear();
        self.read_sizes.clear();
        self.read_bytes = 0;
        self.read_gap = None;
        for row in page.records {
            let size = serde_json::to_vec(&row)
                .map_err(|_| "invalid event page record")?
                .len();
            self.read_bytes += size;
            self.read_sizes.push(size);
            self.read_page.push(Arc::new(row));
        }
        self.trim_window();
        let revision = self.next_page_revision(&stream)?;
        self.page_next.insert(stream.clone(), page.next);
        self.page_errors.remove(&stream);
        self.page_snapshots.insert(
            stream,
            PageSnapshot {
                revision,
                high,
                next: page.next,
            },
        );
        // Even an identical or empty page completes a particular read.
        Ok(true)
    }

    fn next_page_revision(&self, stream: &str) -> Result<u64, &'static str> {
        self.page_snapshots
            .get(stream)
            .map_or(0, |page| page.revision)
            .max(
                self.page_errors
                    .get(stream)
                    .map_or(0, |error| error.revision),
            )
            .checked_add(1)
            .ok_or("event page revision exhausted")
    }

    pub fn page_refused(&mut self, stream: &str, reason: &str) -> Result<bool, &'static str> {
        bbt_rt::assert_not_ui_thread();
        self.register(stream)?;
        let revision = self.next_page_revision(stream)?;
        self.page_errors.insert(
            stream.into(),
            PageError {
                revision,
                reason: reason.chars().take(256).collect(),
            },
        );
        Ok(true)
    }

    fn validate_record(&self, record: &GuiEventRecord) -> Result<(String, u64, u64), &'static str> {
        let (stream, first, last) = match &record {
            GuiEventRecord::Event(row) => {
                let reference = &row.metadata.reference;
                if reference.position.sequence == 0
                    || reference.hash.len() != 64
                    || !reference.hash.bytes().all(|byte| byte.is_ascii_hexdigit())
                {
                    return Err("invalid retained event reference");
                }
                if let Some(cursor) = &row.cursor
                    && (cursor.stream != reference.position.stream || !cursor_consistent(cursor))
                {
                    return Err("event cursor and row streams disagree");
                }
                (
                    &reference.position.stream,
                    reference.position.sequence,
                    reference.position.sequence,
                )
            }
            GuiEventRecord::Gap(gap) => {
                if gap.first > gap.last {
                    return Err("invalid event gap");
                }
                (&gap.stream, gap.first, gap.last)
            }
        };
        let stream = stream.clone();
        if let GuiEventRecord::Event(row) = &record {
            let reference = &row.metadata.reference;
            if let Some((id, hash)) = self.positions.get(&(stream.clone(), last))
                && (*id != row.metadata.id || hash != &reference.hash)
            {
                return Err("event sequence conflicts with the published canonical row");
            }
            for old in &self.read_page {
                if let GuiEventRecord::Event(old) = old.as_ref()
                    && old.metadata.reference.position == reference.position
                    && (old.metadata.id != row.metadata.id
                        || old.metadata.reference.hash != reference.hash)
                {
                    return Err("event sequence conflicts with the requested canonical row");
                }
            }
        }
        Ok((stream, first, last))
    }

    pub fn apply(&mut self, record: GuiEventRecord) -> Result<bool, &'static str> {
        bbt_rt::assert_not_ui_thread();
        let (stream, first, last) = self.validate_record(&record)?;
        self.register(&stream)?;
        let mut changed = false;
        if let GuiEventRecord::Event(row) = &record {
            if let Some(cursor) = &row.cursor {
                changed |= self.apply_cursor(cursor.clone())?;
            }
            if self.positions.contains_key(&(stream.clone(), last))
                || self
                    .omitted
                    .get(&stream)
                    .is_some_and(|gap| first >= gap.first && last <= gap.last)
            {
                return Ok(changed);
            }
        } else if self.records.iter().any(|old| old.as_ref() == &record) {
            return Ok(false);
        }
        let size = serde_json::to_vec(&record)
            .map_err(|_| "event projection cannot be encoded")?
            .len();
        if size > EVENT_BYTES {
            return self
                .apply(GuiEventRecord::Gap(EventGap {
                    stream,
                    first,
                    last,
                    reason: "app_record_exceeds_byte_bound; original retained".into(),
                }))
                .map(|inserted| inserted || changed);
        }
        if let GuiEventRecord::Event(row) = &record {
            self.positions.insert(
                (stream.clone(), last),
                (row.metadata.id, row.metadata.reference.hash.clone()),
            );
        }
        // Each stream is ordered. Cross-stream arrival order is only a display
        // order and never becomes a fabricated global sequence.
        let at = self
            .records
            .iter()
            .position(|old| {
                let (old_stream, old_first) = record_start(old);
                old_stream == stream && old_first > first
            })
            .unwrap_or(self.records.len());
        self.records.insert(at, Arc::new(record));
        self.sizes.insert(at, size);
        self.bytes += size;
        self.trim_window();
        Ok(true)
    }

    fn trim_window(&mut self) {
        // Ordinary reads reserve their bounded share of the window. A large
        // newest live record always wins if the two cannot coexist at all.
        let newest = self.sizes.back().copied().unwrap_or(0);
        while newest + self.read_bytes > EVENT_BYTES && !self.read_page.is_empty() {
            let old = self.read_page.remove(0);
            self.read_bytes -= self.read_sizes.remove(0);
            let (stream, first) = record_start(&old);
            let last = match old.as_ref() {
                GuiEventRecord::Gap(gap) => gap.last,
                _ => first,
            };
            let gap = self.read_gap.get_or_insert_with(|| EventGap {
                stream: stream.into(),
                first,
                last,
                reason: "app_read_page_byte_bound; live window takes precedence; original retained"
                    .into(),
            });
            gap.first = gap.first.min(first);
            gap.last = gap.last.max(last);
        }
        while self.records.len() + self.read_page.len() > EVENT_ROWS || self.bytes() > EVENT_BYTES {
            let Some(old) = self.records.pop_front() else {
                break;
            };
            self.bytes -= self.sizes.pop_front().unwrap();
            let (stream, first, last) = match old.as_ref() {
                GuiEventRecord::Event(row) => {
                    let position = &row.metadata.reference.position;
                    self.positions
                        .remove(&(position.stream.clone(), position.sequence));
                    (&position.stream, position.sequence, position.sequence)
                }
                GuiEventRecord::Gap(gap) => (&gap.stream, gap.first, gap.last),
            };
            self.omitted
                .entry(stream.clone())
                .and_modify(|gap| {
                    gap.first = gap.first.min(first);
                    gap.last = gap.last.max(last);
                })
                .or_insert_with(|| EventGap {
                    stream: stream.clone(),
                    first,
                    last,
                    reason: "app_window_evicted; original retained".into(),
                });
        }
        self.artifacts = WindowArtifactCounts::default();
        for record in &self.records {
            if let GuiEventRecord::Event(row) = record.as_ref()
                && let GuiEventPayload::Activity { summary } = &row.payload
                && (row.metadata.subject.starts_with("artifact/")
                    || (row.metadata.subject.starts_with("doc/")
                        && row.metadata.subject.ends_with("/written")
                        && summary.action == "claimed"))
            {
                match summary.action.as_str() {
                    "created" => self.artifacts.created += 1,
                    "modified" => self.artifacts.modified += 1,
                    "claimed" => self.artifacts.claimed += 1,
                    "deleted" => self.artifacts.deleted += 1,
                    _ => (),
                }
            }
        }
    }
}

fn record_start(record: &GuiEventRecord) -> (&str, u64) {
    match record {
        GuiEventRecord::Event(row) => (
            &row.metadata.reference.position.stream,
            row.metadata.reference.position.sequence,
        ),
        GuiEventRecord::Gap(gap) => (&gap.stream, gap.first),
    }
}

fn cursor_consistent(cursor: &GuiEventCursor) -> bool {
    cursor.acknowledged <= cursor.admitted
        && cursor.admitted <= cursor.high
        && cursor.read <= cursor.high
        && cursor.unread <= cursor.high - cursor.read
}

/// A complete immutable UI frame. The IO owner publishes one Arc<Model>; no
/// terminal rows are cloned or locked by the UI while taking that Arc.
#[derive(Clone)]
pub struct Model {
    /// Exact public rows retained for GUI-owned listings, without a logic round trip.
    pub gui_seats: Arc<[GuiSeat]>,
    pub messaging_deferred: BTreeMap<SeatId, bbt_ui::modules::seat_row::Deferred>,
    pub hosts: Vec<bbt_proto::GuiHost>,
    pub ui: ui::Model,
    pub screens: BTreeMap<SeatId, Arc<Screen>>,
    pub screen_baselines: BTreeMap<SeatId, (View, u64)>,
    pub subtitles: BTreeMap<SeatId, String>,
    /// Historical display only. Live modes, replicas and acknowledgements keep
    /// using screens; the UI never owns a history parser or a fetched page.
    pub history: BTreeMap<SeatId, Arc<crate::scrollback::Historical>>,
    pub scroll: Arc<crate::scrollback::Readback>,
    pub frames: BTreeMap<SeatId, Arc<PreparedGrid>>,
    /// Provenance from the same screen used to prepare each cached grid.
    pub prepared_screens: BTreeMap<SeatId, PreparedScreen>,
    pub states: BTreeMap<SeatId, SeatState>,
    /// Signing holds affect messaging, never whether terminal panes stay live.
    pub identity: Option<bbt_proto::GuiIdentity>,
    pub custody_reply: Option<Arc<crate::settings::IdentityReply>>,
    /// Latest explicitly selected policy, bounded to one seat; ordinary rows keep small observations.
    pub attention_read: Option<Arc<GuiSeat>>,
    /// Correlation survives Latest replacing an intermediate publication.
    pub spawns: BTreeMap<Ulid, SpawnRequest>,
    pub operation_results: Arc<crate::messaging::Results>,
    pub metadata_initialized: bool,
    pub events: Arc<EventLane>,
    pub e2e: Option<Arc<bbt_ui::modules::e2e_checklist::Checklist>>,
    /// G5 paint state only; neither map changes layout or selects a seat.
    pub attention: BTreeMap<SeatId, crate::attention::Attention>,
    pub focus_swell: Option<(SeatId, Instant)>,
    /// IO bookkeeping; episode internals are private to the attention module.
    #[doc(hidden)]
    pub attention_episodes: crate::attention::Episodes,
    pub settings: Option<Arc<crate::settings::Snapshot>>,
    pub document_reply: Option<Arc<crate::documents::Reply>>,
    pub closes: crate::close::State,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            gui_seats: Arc::default(),
            messaging_deferred: BTreeMap::new(),
            ui: ui::Model::new(vec![], vec![]).expect("empty model"),
            screens: BTreeMap::new(),
            screen_baselines: BTreeMap::new(),
            subtitles: BTreeMap::new(),
            history: BTreeMap::new(),
            scroll: Arc::default(),
            frames: BTreeMap::new(),
            prepared_screens: BTreeMap::new(),
            states: BTreeMap::new(),
            identity: None,
            custody_reply: None,
            attention_read: None,
            hosts: vec![],
            spawns: BTreeMap::new(),
            operation_results: Arc::default(),
            metadata_initialized: false,
            events: Arc::new(EventLane::default()),
            e2e: None,
            attention: BTreeMap::new(),
            focus_swell: None,
            attention_episodes: crate::attention::Episodes::default(),
            settings: None,
            document_reply: None,
            closes: crate::close::State::default(),
        }
    }
}

impl Model {
    /// Consume current state only, before publishing its live lane delivery.
    pub fn observe_attention(&mut self, seat: &GuiSeat, now: Instant) {
        let id = SeatId::from_u128(seat.record.id.0);
        if self
            .ui
            .seats
            .get(&id)
            .is_some_and(|old| old.process_generation > seat.state.generation)
            || self.states.get(&id).is_some_and(|old| {
                old.generation == seat.state.generation && old.observed_ms > seat.state.observed_ms
            })
        {
            return;
        }
        self.attention_episodes.observe(seat, now);
        self.attention = self.attention_episodes.paint();
    }

    pub fn attention_key(&self, seat: SeatId) -> Option<crate::attention::EpisodeKey> {
        self.attention_episodes.key(seat)
    }

    pub fn notifications(
        &self,
        settings: &crate::attention::Notifications,
        frontmost: bool,
        focused: Option<SeatId>,
    ) -> crate::attention::Desired {
        self.attention_episodes
            .notifications(settings, frontmost, focused)
    }

    /// The explicit UI intent is bound to its generation and exact episode.
    /// The landing timestamp is supplied after a tab switch has a real layout.
    pub fn attention_focus(
        &mut self,
        seat: SeatId,
        generation: u64,
        episode: Option<crate::attention::EpisodeKey>,
        landed: Option<Instant>,
    ) -> bool {
        if !self.ui.seats.get(&seat).is_some_and(|row| {
            row.process_generation == generation && row.lifecycle == ui::Lifecycle::Open
        }) {
            return false;
        }
        let mut changed = episode.is_some_and(|key| {
            key.generation == generation && self.attention_episodes.handle(seat, key)
        });
        if let Some(at) = landed {
            self.focus_swell = Some((seat, at));
            changed = true;
        }
        if changed {
            self.attention = self.attention_episodes.paint();
        }
        changed
    }

    pub fn request_spawn(
        &mut self,
        op: Ulid,
        spec: bbt_proto::SpawnSpec,
    ) -> Result<bool, &'static str> {
        if let Some(old) = self.spawns.get(&op) {
            return if old.spec == spec {
                Ok(false)
            } else {
                Err("spawn_id_conflict")
            };
        }
        if self.spawns.len() >= PENDING_SPAWNS {
            return Err("spawn_queue_full");
        }
        let ordinal = self
            .spawns
            .values()
            .map(|request| request.ordinal)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        self.spawns.insert(
            op,
            SpawnRequest {
                spec,
                outcome: SpawnOutcome::Pending,
                consumed: false,
                ordinal,
            },
        );
        Ok(true)
    }

    pub fn spawned(&mut self, op: Ulid, seat: SeatId) -> bool {
        let Some(request) = self.spawns.get_mut(&op) else {
            return false;
        };
        if !matches!(
            request.outcome,
            SpawnOutcome::Pending | SpawnOutcome::Unknown(_)
        ) {
            return false;
        }
        request.outcome = SpawnOutcome::AwaitingSeat(seat);
        request.consumed = false;
        true
    }

    pub fn spawn_rejected(&mut self, op: Ulid, reason: String) -> bool {
        let Some(request) = self.spawns.get_mut(&op) else {
            return false;
        };
        if !matches!(
            request.outcome,
            SpawnOutcome::Pending | SpawnOutcome::Unknown(_)
        ) {
            return false;
        }
        request.outcome = if reason.starts_with("held:") {
            SpawnOutcome::Held(reason)
        } else if reason.starts_with("unknown:") {
            SpawnOutcome::Unknown(reason)
        } else {
            SpawnOutcome::Rejected(reason)
        };
        request.consumed = false;
        true
    }

    pub fn spawns_disconnected(&mut self, reason: &str) -> bool {
        let mut changed = false;
        for request in self.spawns.values_mut() {
            if request.outcome == SpawnOutcome::Pending {
                request.outcome = SpawnOutcome::Unknown(format!("spawn_outcome_unknown: {reason}"));
                request.consumed = false;
                changed = true;
            }
        }
        changed
    }

    pub fn consume_spawn(&mut self, op: Ulid) {
        let Some(request) = self.spawns.get_mut(&op) else {
            return;
        };
        if matches!(
            request.outcome,
            SpawnOutcome::Unknown(_) | SpawnOutcome::Held(_)
        ) {
            // Keep its captured request available for an explicit same-id query.
            request.consumed = true;
        } else if request.outcome.is_result() {
            self.spawns.remove(&op);
        }
    }

    pub fn resume_held_spawns(&mut self) -> Vec<(Ulid, bbt_proto::SpawnSpec)> {
        let mut ready: Vec<_> = self
            .spawns
            .iter_mut()
            .filter_map(|(&op, request)| {
                if !matches!(request.outcome, SpawnOutcome::Held(_)) {
                    return None;
                }
                request.outcome = SpawnOutcome::Pending;
                request.consumed = false;
                Some((request.ordinal, op, request.spec.clone()))
            })
            .collect();
        ready.sort_by_key(|(ordinal, ..)| *ordinal);
        ready.into_iter().map(|(_, op, spec)| (op, spec)).collect()
    }

    fn place_ready_spawns(&mut self) {
        let ready: Vec<_> = self
            .spawns
            .iter()
            .filter_map(|(&op, request)| {
                if let SpawnOutcome::AwaitingSeat(seat) = request.outcome
                    && self.ui.seats.contains_key(&seat)
                {
                    Some((op, seat, request.spec.clone()))
                } else {
                    None
                }
            })
            .collect();
        for (op, seat, spec) in ready {
            let result = self.captured_placement(seat, &spec).and_then(|placement| {
                ui::place(&self.ui, seat, placement)
                    .map_err(|error| format!("placement_failed: {error}"))
            });
            let outcome = match result {
                Ok(next) => {
                    self.ui = next;
                    SpawnOutcome::Placed(seat)
                }
                Err(reason) => SpawnOutcome::Partial { seat, reason },
            };
            let request = self.spawns.get_mut(&op).expect("collected request");
            request.outcome = outcome;
            request.consumed = false;
        }
    }

    fn captured_placement(
        &self,
        seat: SeatId,
        spec: &bbt_proto::SpawnSpec,
    ) -> Result<ui::Placement, String> {
        use bbt_proto::seat_state::{Placement, Side};
        if self.ui.seats[&seat].swarm != spec.swarm {
            return Err("placement_swarm_changed".into());
        }
        match &spec.placement {
            Some(Placement::NewTab { tab }) => Ok(ui::Placement::Tab {
                swarm: spec.swarm,
                tab: *tab,
            }),
            Some(Placement::Split {
                tab,
                target,
                target_generation,
                side,
            }) => {
                let anchor = self
                    .ui
                    .seats
                    .get(target)
                    .ok_or("placement_anchor_missing")?;
                if anchor.process_generation != *target_generation {
                    return Err("placement_anchor_changed".into());
                }
                let (si, ti) = self.ui.tab_of(*target).ok_or("placement_anchor_missing")?;
                let owner = &self.ui.window.sidebar.swarms[si];
                if owner.id != spec.swarm || owner.tabs[ti].id != *tab {
                    return Err("placement_anchor_missing".into());
                }
                Ok(ui::Placement::Split {
                    target: *target,
                    side: match side {
                        Side::Right => ui::Side::Right,
                        Side::Bottom => ui::Side::Bottom,
                    },
                })
            }
            None => {
                // Compatibility lane for Verify/default-shell callers: extend
                // the existing swarm tree without resolving later GUI focus.
                let target = self
                    .ui
                    .window
                    .sidebar
                    .swarms
                    .iter()
                    .find(|s| s.id == spec.swarm)
                    .and_then(|s| s.tabs.first())
                    .and_then(|t| t.tree.leaves().last().copied());
                Ok(target.map_or(
                    ui::Placement::Tab {
                        swarm: spec.swarm,
                        tab: ui::TabId::from_u128(spec.swarm.0.0),
                    },
                    |target| ui::Placement::Split {
                        target,
                        side: ui::Side::Right,
                    },
                ))
            }
        }
    }

    pub fn provider_settings(&self) -> bbt_ui::modules::settings::Records {
        crate::hosts::settings(&self.hosts)
    }
    pub fn provider_launcher_items(&self) -> Vec<bbt_ui::modules::launcher::Item> {
        crate::hosts::launcher_items(&self.hosts)
    }
    /// Project the portable state after build, before preparation/publication.
    /// State comes from logic; this does not infer activity from terminal text.
    pub fn apply_seat_states(&mut self, seats: &[GuiSeat]) {
        if self.gui_seats.as_ref() != seats {
            self.gui_seats = Arc::from(seats);
        }
        self.messaging_deferred = seats
            .iter()
            .filter_map(|seat| {
                crate::settings::messaging::deferred(seat.attention.as_ref())
                    .map(|deferred| (SeatId::from_u128(seat.record.id.0), deferred))
            })
            .collect();
        self.states.retain(|id, _| self.ui.seats.contains_key(id));
        for seat in seats {
            self.observe_attention(seat, Instant::now());
            let id = SeatId::from_u128(seat.record.id.0);
            let Some(row) = self.ui.seats.get_mut(&id) else {
                continue;
            };
            if row.process_generation != seat.state.generation {
                continue;
            }
            row.activity = seat.state.activity;
            row.grade = seat.state.grade;
            row.title = seat.state.title.clone();
            self.states.insert(id, seat.state.clone());
        }
    }
}

/// Pure assembly on IO. Keep the current layout when its membership is unchanged
/// and keep every unchanged screen Arc; no screen content is copied here.
pub fn build(seats: &[SeatRecord], grids: &Grids, previous: &Model) -> Model {
    build_with_clock(seats, grids, previous, || {
        (Instant::now(), bbt_mux::store::now_ms())
    })
}

/// Build against explicit monotonic and wall coordinates for deterministic hosts/tests.
#[doc(hidden)]
pub fn build_at(
    seats: &[SeatRecord],
    grids: &Grids,
    previous: &Model,
    now: Instant,
    wall_ms: u64,
) -> Model {
    build_with_clock(seats, grids, previous, || (now, wall_ms))
}

fn build_with_clock(
    seats: &[SeatRecord],
    grids: &Grids,
    previous: &Model,
    clock: impl FnOnce() -> (Instant, u64),
) -> Model {
    let measurement = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::ModelBuild);
    let bootstrap = !previous.metadata_initialized
        && previous.ui.seats.is_empty()
        && previous.spawns.is_empty();
    let screens: BTreeMap<_, _> = seats
        .iter()
        .filter_map(|record| {
            let id = SeatId::from_u128(record.id.0);
            grids.screen(id).map(|screen| (id, screen))
        })
        .collect();
    let (now, now_ms) = clock();
    let mut layout_source = std::borrow::Cow::Borrowed(&previous.ui);
    let mut closes = previous.closes.clone();
    if closes.needs_reconcile(seats) {
        closes.reconcile(seats, layout_source.to_mut(), now, now_ms);
    }
    let metadata: BTreeMap<_, _> = seats
        .iter()
        .map(|record| {
            let id = SeatId::from_u128(record.id.0);
            let swarm = ui::SwarmId::from_u128(record.swarm.0);
            let mut seat = previous.ui.seats.get(&id).cloned().unwrap_or_else(|| {
                ui::Seat::new(id, swarm, [0; 32], &record.name, &record.harness_kind)
            });
            seat.swarm = swarm;
            seat.name = record.name.clone();
            seat.harness = ui::HarnessId(record.harness_kind.clone());
            seat.process_generation = record.generation;
            seat.role = Some(ui::Role {
                name: record.role.name.clone(),
                class: match record.role.class {
                    registry::Class::Oracle => ui::RoleClass::Oracle,
                    registry::Class::Worker => ui::RoleClass::Worker,
                },
            });
            seat.where_ = match &record.location {
                registry::Location::Local => ui::Where::Local,
                registry::Location::Ssh { host } => ui::Where::Ssh { host: host.clone() },
                registry::Location::Cloud { provider, host } => ui::Where::Cloud {
                    provider: provider.clone(),
                    host: host.clone(),
                },
            };
            seat.muted = record.muted;
            seat.tasks = record
                .tasks
                .iter()
                .map(|id| ui::TaskId::from_u128(id.0))
                .collect();
            seat.lifecycle = match record.lifecycle {
                registry::Lifecycle::Open => ui::Lifecycle::Open,
                registry::Lifecycle::ClosedRetained => ui::Lifecycle::ClosedRetained,
                registry::Lifecycle::ClosedGrace => record
                    .close
                    .as_ref()
                    .and_then(|close| {
                        now.checked_add(Duration::from_millis(
                            close.deadline_ms.saturating_sub(now_ms),
                        ))
                    })
                    .map_or(ui::Lifecycle::ClosedRetained, |until| {
                        ui::Lifecycle::ClosedGrace { until }
                    }),
            };
            if let Some(screen) = screens.get(&id) {
                seat.title = screen.title.clone();
            }
            (id, seat)
        })
        .collect();
    let mut groups: BTreeMap<ui::SwarmId, Vec<SeatId>> = BTreeMap::new();
    for swarm in &layout_source.window.sidebar.swarms {
        if !swarm.closed.is_empty() || !swarm.stream.is_empty() {
            groups.entry(swarm.id).or_default();
        }
    }
    for seat in metadata
        .values()
        .filter(|s| matches!(s.lifecycle, ui::Lifecycle::Open))
    {
        groups.entry(seat.swarm).or_default().push(seat.id);
    }
    let swarms = groups
        .into_iter()
        .map(|(id, ids)| {
            if let Some(old) = layout_source
                .window
                .sidebar
                .swarms
                .iter()
                .find(|s| s.id == id)
            {
                let mut old_ids: Vec<_> = old
                    .tabs
                    .iter()
                    .flat_map(|tab| tab.seats.iter().copied())
                    .collect();
                old_ids.sort();
                if old_ids == ids {
                    return old.clone();
                }
                let mut next = old.clone();
                next.tabs = old
                    .tabs
                    .iter()
                    .filter_map(|tab| {
                        let mut tree = Some(tab.tree.clone());
                        for removed in tab
                            .tree
                            .leaves()
                            .into_iter()
                            .filter(|seat| !ids.contains(seat))
                        {
                            tree = tree.and_then(|tree| ui::prune(&tree, removed));
                        }
                        let mut tab = tab.clone();
                        tab.tree = tree?;
                        tab.seats.retain(|seat| ids.contains(seat));
                        Some(tab)
                    })
                    .collect();
                return next;
            }
            ui::Swarm::new(
                id,
                "swarm",
                if bootstrap {
                    vec![ui::Tab::new(
                        ui::TabId::from_u128(id.0.0),
                        "seats",
                        split(&ids),
                    )]
                } else {
                    vec![]
                },
            )
        })
        .collect();
    let mut model = ui::Model::new(swarms, metadata.into_values().collect())
        .expect("unique records and valid seat trees");
    let fallback_stage = model.window.stage.clone();
    let next_swarms = std::mem::take(&mut model.window.sidebar.swarms);
    model.window = layout_source.window.clone();
    model
        .window
        .sidebar
        .open
        .retain(|id| next_swarms.iter().any(|s| s.id == *id));
    for swarm in &next_swarms {
        if !layout_source
            .window
            .sidebar
            .swarms
            .iter()
            .any(|old| old.id == swarm.id)
        {
            model.window.sidebar.open.insert(swarm.id);
        }
    }
    model.window.sidebar.swarms = next_swarms;
    model.window.stage = fallback_stage;
    let stage = &layout_source.window.stage;
    if model
        .window
        .sidebar
        .swarms
        .iter()
        .any(|s| Some(s.id) == stage.swarm && s.tabs.iter().any(|t| Some(t.id) == stage.tab))
    {
        model.window.stage = stage.clone();
        if stage.focused.is_none_or(|id| {
            !model
                .seats
                .get(&id)
                .is_some_and(|s| matches!(s.lifecycle, ui::Lifecycle::Open))
        }) {
            model.window.stage.focused = model
                .window
                .sidebar
                .swarms
                .iter()
                .flat_map(|s| &s.tabs)
                .find(|t| Some(t.id) == stage.tab)
                .and_then(|t| t.seats.first().copied());
        }
    }
    model.window.overlay = layout_source.window.overlay.clone();
    let frames = previous
        .frames
        .iter()
        .filter_map(
            |(seat, frame)| match (screens.get(seat), previous.screens.get(seat)) {
                (Some(next), Some(old)) if Arc::ptr_eq(next, old) => Some((*seat, frame.clone())),
                _ => None,
            },
        )
        .collect();
    let states = previous
        .states
        .iter()
        .filter(|(id, _)| model.seats.contains_key(id))
        .map(|(id, state)| (*id, state.clone()))
        .collect();
    let history =
        previous
            .history
            .iter()
            .filter(|(id, historical)| {
                screens.get(id).is_some_and(|live| {
                    (live.cols, live.rows) == (historical.screen.cols, historical.screen.rows)
                        && model.seats.get(id).is_some_and(|seat| {
                            previous.ui.seats.get(id).is_some_and(|old| {
                                old.process_generation == seat.process_generation
                            })
                        })
                })
            })
            .map(|(id, screen)| (*id, screen.clone()))
            .collect();
    model.window.theme = previous.ui.window.theme;
    model.window.density = previous.ui.window.density;
    let subtitles = screens
        .iter()
        .filter_map(|(id, screen)| {
            let text = if previous
                .screens
                .get(id)
                .is_some_and(|old| Arc::ptr_eq(old, screen))
            {
                previous.subtitles.get(id).cloned()
            } else {
                screen.cells.iter().rev().find_map(|row| {
                    let value: String = row
                        .iter()
                        .filter(|c| {
                            !c.flags
                                .contains(bbt_core::term::cell::Flags::WIDE_CHAR_SPACER)
                        })
                        .map(|c| c.c)
                        .collect();
                    let value = value.trim();
                    (!value.is_empty()).then(|| value.chars().take(160).collect::<String>())
                })
            };
            text.map(|text| (*id, text))
        })
        .collect();
    let mut next = Model {
        gui_seats: previous.gui_seats.clone(),
        messaging_deferred: previous.messaging_deferred.clone(),
        hosts: previous.hosts.clone(),
        ui: model,
        screen_baselines: screens
            .keys()
            .filter_map(|seat| grids.baseline(*seat).map(|baseline| (*seat, baseline)))
            .collect(),
        screens,
        subtitles,
        history,
        scroll: previous.scroll.clone(),
        frames,
        prepared_screens: BTreeMap::new(),
        states,
        identity: previous.identity.clone(),
        custody_reply: previous.custody_reply.clone(),
        attention_read: previous.attention_read.clone(),
        spawns: previous.spawns.clone(),
        operation_results: previous.operation_results.clone(),
        metadata_initialized: true,
        events: previous.events.clone(),
        e2e: previous.e2e.clone(),
        attention: previous.attention.clone(),
        focus_swell: previous.focus_swell,
        attention_episodes: previous.attention_episodes.clone(),
        settings: previous.settings.clone(),
        document_reply: previous.document_reply.clone(),
        closes,
    };
    next.attention_episodes.retain(
        next.ui
            .seats
            .iter()
            .filter(|(_, seat)| seat.lifecycle == ui::Lifecycle::Open)
            .map(|(id, _)| *id),
    );
    next.attention = next.attention_episodes.paint();
    next.place_ready_spawns();
    measurement.finish(seats.len() as u64, 0);
    next
}

fn split(ids: &[SeatId]) -> ui::SplitNode {
    if ids.len() == 1 {
        return ui::SplitNode::Leaf(ids[0]);
    }
    let mid = ids.len() / 2;
    ui::SplitNode::split(
        ui::Axis::Row,
        mid as f32 / ids.len() as f32,
        split(&ids[..mid]),
        split(&ids[mid..]),
    )
}

#[derive(Clone)]
pub struct PreparedScreen {
    pub screen: Arc<Screen>,
    /// Historical screens have no live capture cursor.
    pub baseline: Option<(View, u64)>,
}

struct CachedPane {
    screen: Arc<Screen>,
    focused: bool,
    theme: Theme,
    frame: Arc<PreparedGrid>,
}
/// Retained worker-side glyph cache, sharing the app's one GPU device. The IO
/// loop calls prepare after build and before publishing the complete Model.
pub struct PanePreparer {
    context: Arc<DeviceContext>,
    renderer: TerminalRenderer,
    cached: BTreeMap<SeatId, CachedPane>,
}
impl PanePreparer {
    pub fn new(context: Arc<DeviceContext>) -> bbt_render::Result<Self> {
        Self::with_options(context, terminal::Options::default())
    }

    pub fn with_options(
        context: Arc<DeviceContext>,
        options: terminal::Options,
    ) -> bbt_render::Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let renderer = TerminalRenderer::new(&context, options)?;
        Ok(Self {
            context,
            renderer,
            cached: BTreeMap::new(),
        })
    }
    pub fn prepare(&mut self, model: &mut Model) -> bbt_render::Result<()> {
        bbt_rt::assert_not_ui_thread();
        let measurement = bbt_rt::diagnostics::span(bbt_rt::diagnostics::Phase::PanePrepare);
        self.cached
            .retain(|seat, _| model.screens.contains_key(seat));
        let theme = model.ui.window.theme;
        let mut frames = BTreeMap::new();
        let mut prepared_screens = BTreeMap::new();
        for (&seat, screen) in &model.screens {
            let screen = model
                .history
                .get(&seat)
                .map_or(screen, |historical| &historical.screen);
            let focused = model.ui.window.stage.focused == Some(seat);
            let reuse = self.cached.get(&seat).is_some_and(|old| {
                Arc::ptr_eq(&old.screen, screen) && old.focused == focused && old.theme == theme
            });
            if !reuse {
                let grid = screen_grid(screen, theme, focused)?;
                let frame = Arc::new(self.renderer.prepare(&self.context, &grid)?);
                self.cached.insert(
                    seat,
                    CachedPane {
                        screen: screen.clone(),
                        focused,
                        theme,
                        frame,
                    },
                );
            }
            frames.insert(seat, self.cached[&seat].frame.clone());
            prepared_screens.insert(
                seat,
                PreparedScreen {
                    screen: self.cached[&seat].screen.clone(),
                    baseline: (!model.history.contains_key(&seat))
                        .then(|| model.screen_baselines.get(&seat).copied())
                        .flatten(),
                },
            );
        }
        model.frames = frames;
        model.prepared_screens = prepared_screens;
        measurement.finish(model.screens.len() as u64, 0);
        Ok(())
    }
}

pub struct DrawCtx<'a> {
    pub frame: &'a PreparedGrid,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub target: &'a wgpu::TextureView,
    pub scale: f32,
}
/// UI-only encoding of a frame already prepared on IO. No rasterization,
/// allocation of a glyph atlas, device poll, readback or lock occurs here.
pub fn draw_pane(screen: &Screen, rect: Rect, ctx: &mut DrawCtx<'_>) {
    if screen.cols == 0 || screen.rows == 0 || !ctx.scale.is_finite() || ctx.scale <= 0. {
        return;
    }
    let rect = rect.normalized();
    let x = (rect.x * ctx.scale).round().max(0.) as u32;
    let y = (rect.y * ctx.scale).round().max(0.) as u32;
    let w = (rect.w * ctx.scale).floor().max(0.) as u32;
    let h = (rect.h * ctx.scale).floor().max(0.) as u32;
    ctx.frame
        .encode(ctx.encoder, ctx.target, [x, y], [x, y, w, h]);
}

fn rgb(value: u32) -> [u8; 4] {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]
}
fn palette(screen: &Screen, index: usize, theme: Theme) -> [u8; 4] {
    if let Some(v) = screen.palette.get(index)
        && let (Some(r), Some(g), Some(b)) = (v["r"].as_u64(), v["g"].as_u64(), v["b"].as_u64())
    {
        return [r as u8, g as u8, b as u8, 255];
    }
    const ANSI: [u32; 16] = [
        0x2e3436, 0xcc0000, 0x4e9a06, 0xc4a000, 0x3465a4, 0x75507b, 0x06989a, 0xd3d7cf, 0x555753,
        0xef2929, 0x8ae234, 0xfce94f, 0x729fcf, 0xad7fa8, 0x34e2e2, 0xeeeeec,
    ];
    match index {
        0..=15 => rgb(ANSI[index]),
        16..=231 => {
            let n = index - 16;
            let c = [0, 95, 135, 175, 215, 255];
            [c[n / 36], c[n / 6 % 6], c[n % 6], 255]
        }
        232..=255 => {
            let v = (8 + (index - 232) * 10) as u8;
            [v, v, v, 255]
        }
        257 => {
            if theme == Theme::Light {
                rgb(0xf2f4f8)
            } else {
                rgb(0x0f141d)
            }
        }
        259..=266 => {
            let mut c = rgb(ANSI[index - 259]);
            for v in &mut c[..3] {
                *v /= 2;
            }
            c
        }
        _ => {
            if theme == Theme::Light {
                rgb(0x171f2b)
            } else {
                rgb(0xe8edf7)
            }
        }
    }
}
fn color(screen: &Screen, value: Color, theme: Theme, bold: bool) -> [u8; 4] {
    match value {
        Color::Spec(v) => [v.r, v.g, v.b, 255],
        Color::Indexed(i) => palette(
            screen,
            usize::from(if bold && i < 8 { i + 8 } else { i }),
            theme,
        ),
        Color::Named(n) => {
            let i = n as usize;
            palette(screen, if bold && i < 8 { i + 8 } else { i }, theme)
        }
    }
}
/// Convert rows only on IO; the cells renderer has no parser/protocol dependency.
pub fn screen_grid(screen: &Screen, theme: Theme, focused: bool) -> bbt_render::Result<Grid> {
    if screen.cols == 0
        || screen.rows == 0
        || screen.cols > 512
        || screen.rows > 256
        || screen.cells.len() != screen.rows
        || screen.cells.iter().any(|r| r.len() != screen.cols)
    {
        return Err("invalid terminal screen dimensions".into());
    }
    let cells = screen
        .cells
        .iter()
        .flatten()
        .map(|c| {
            let f = c.flags;
            let width = if f.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                0
            } else if f.contains(Flags::WIDE_CHAR) {
                2
            } else {
                1
            };
            let bold = f.contains(Flags::BOLD);
            let mut foreground = color(screen, c.fg, theme, bold);
            let mut background = color(screen, c.bg, theme, false);
            if f.contains(Flags::INVERSE) {
                std::mem::swap(&mut foreground, &mut background);
            }
            Cell {
                text: if width == 0 {
                    String::new()
                } else {
                    std::iter::once(c.c)
                        .chain(c.zerowidth().unwrap_or_default().iter().copied())
                        .collect()
                },
                width,
                foreground,
                background,
                underline_color: c
                    .underline_color()
                    .map_or(foreground, |v| color(screen, v, theme, false)),
                bold,
                italic: f.contains(Flags::ITALIC),
                dim: f.contains(Flags::DIM),
                hidden: f.contains(Flags::HIDDEN),
                underline: if f.contains(Flags::UNDERCURL) {
                    Underline::Curly
                } else if f.contains(Flags::DOUBLE_UNDERLINE) {
                    Underline::Double
                } else if f.contains(Flags::DOTTED_UNDERLINE) {
                    Underline::Dotted
                } else if f.contains(Flags::DASHED_UNDERLINE) {
                    Underline::Dashed
                } else if f.contains(Flags::UNDERLINE) {
                    Underline::Single
                } else {
                    Underline::None
                },
                strike: f.contains(Flags::STRIKEOUT),
            }
        })
        .collect();
    let shape = screen.cursor_style["shape"].as_str().unwrap_or("Block");
    let cursor = (screen.modes & TermMode::SHOW_CURSOR.bits() as u64 != 0
        && shape != "Hidden"
        && screen.cursor.0 >= 0
        && (screen.cursor.0 as usize) < screen.rows
        && screen.cursor.1 < screen.cols)
        .then(|| Cursor {
            column: screen.cursor.1 as u16,
            row: screen.cursor.0 as u16,
            shape: match shape {
                "Beam" => CursorShape::Bar,
                "Underline" => CursorShape::Underline,
                _ => CursorShape::Block,
            },
            focused: focused && shape != "HollowBlock",
            color: color(screen, Color::Named(NamedColor::Cursor), theme, false),
        });
    Ok(Grid {
        columns: screen.cols as u16,
        rows: screen.rows as u16,
        cells,
        cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_mux::Ulid;
    use bbt_proto::{Snapshot, Update, View};

    fn spawn_record(id: u128) -> SeatRecord {
        SeatRecord::new(Ulid(id), Ulid(1), format!("seat {id}"), "zsh")
    }
    fn spawn_spec(placement: Option<bbt_proto::seat_state::Placement>) -> bbt_proto::SpawnSpec {
        bbt_proto::SpawnSpec {
            swarm: ui::SwarmId::from_u128(1),
            harness_kind: "zsh".into(),
            name: None,
            role: None,
            cwd: "/fixture".into(),
            location: ui::Where::Local,
            resume: None,
            placement,
        }
    }
    fn split_spec(target: u128, side: bbt_proto::seat_state::Side) -> bbt_proto::SpawnSpec {
        spawn_spec(Some(bbt_proto::seat_state::Placement::Split {
            tab: ui::TabId::from_u128(1),
            target: SeatId::from_u128(target),
            target_generation: spawn_record(target).generation,
            side,
        }))
    }
    fn spawn_tree(model: &Model) -> ui::SplitNode {
        model.ui.window.sidebar.swarms[0].tabs[0].tree.clone()
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn reversed_spawn_replies_use_their_captured_anchors_and_duplicates_are_inert() {
        use bbt_proto::seat_state::Side;
        let grids = Grids::new();
        let mut records = vec![spawn_record(10), spawn_record(20)];
        let mut model = build(&records, &grids, &Model::default());
        let original = spawn_tree(&model);
        model
            .request_spawn(Ulid(100), split_spec(10, Side::Right))
            .unwrap();
        model
            .request_spawn(Ulid(200), split_spec(20, Side::Bottom))
            .unwrap();
        records.extend([spawn_record(30), spawn_record(40), spawn_record(50)]);
        model = build(&records, &grids, &model);
        assert_eq!(spawn_tree(&model), original); // metadata alone places none
        assert!(model.spawned(Ulid(200), SeatId::from_u128(50)));
        model = build(&records, &grids, &model);
        assert!(model.ui.tab_of(SeatId::from_u128(40)).is_none());
        assert!(model.spawned(Ulid(100), SeatId::from_u128(40)));
        model = build(&records, &grids, &model);
        let leaf = |id| ui::SplitNode::Leaf(SeatId::from_u128(id));
        let expected = ui::SplitNode::split(
            ui::Axis::Row,
            0.5,
            ui::SplitNode::split(ui::Axis::Row, 0.5, leaf(10), leaf(40)),
            ui::SplitNode::split(ui::Axis::Column, 0.5, leaf(20), leaf(50)),
        );
        assert_eq!(spawn_tree(&model), expected);
        assert!(model.ui.tab_of(SeatId::from_u128(30)).is_none());
        assert!(!model.spawned(Ulid(100), SeatId::from_u128(40)));
        records[0].name = "renamed metadata".into();
        model = build(&records, &grids, &model);
        assert_eq!(spawn_tree(&model), expected);
        // A later publication retains both results until the UI consumes them.
        model = build(&records, &grids, &model);
        assert_eq!(
            model.spawns[&Ulid(100)].outcome,
            SpawnOutcome::Placed(SeatId::from_u128(40))
        );
        model.consume_spawn(Ulid(100));
        assert!(!model.spawned(Ulid(100), SeatId::from_u128(40)));
        assert_eq!(spawn_tree(&model), expected);
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn removed_or_replaced_anchor_keeps_the_new_seat_and_names_the_partial_outcome() {
        for replaced in [false, true] {
            let grids = Grids::new();
            let mut records = vec![spawn_record(10), spawn_record(20)];
            let mut model = build(&records, &grids, &Model::default());
            model
                .request_spawn(
                    Ulid(100),
                    split_spec(10, bbt_proto::seat_state::Side::Right),
                )
                .unwrap();
            if replaced {
                records[0].generation += 1;
            } else {
                records.remove(0);
            }
            records.push(spawn_record(30));
            assert!(model.spawned(Ulid(100), SeatId::from_u128(30)));
            model = build(&records, &grids, &model);
            let expected = if replaced {
                "placement_anchor_changed"
            } else {
                "placement_anchor_missing"
            };
            assert_eq!(
                model.spawns[&Ulid(100)].outcome,
                SpawnOutcome::Partial {
                    seat: SeatId::from_u128(30),
                    reason: expected.into(),
                }
            );
            assert!(model.ui.seats.contains_key(&SeatId::from_u128(30)));
            assert!(model.ui.tab_of(SeatId::from_u128(30)).is_none());
        }
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn unknown_spawn_and_unrelated_metadata_never_place_or_lose_the_request() {
        let grids = Grids::new();
        let mut records = vec![spawn_record(10)];
        let mut model = build(&records, &grids, &Model::default());
        let original = spawn_tree(&model);
        let stage = model.ui.window.stage.clone();
        let spec = split_spec(10, bbt_proto::seat_state::Side::Right);
        assert!(model.request_spawn(Ulid(100), spec.clone()).unwrap());
        model.spawns_disconnected("logic disconnected");
        model.consume_spawn(Ulid(100));
        assert!(matches!(
            model.spawns[&Ulid(100)].outcome,
            SpawnOutcome::Unknown(_)
        ));
        assert_eq!(model.spawns[&Ulid(100)].spec, spec);
        records.push(spawn_record(20));
        model = build(&records, &grids, &model);
        assert!(!model.spawned(Ulid(999), SeatId::from_u128(20)));
        assert_eq!(spawn_tree(&model), original);
        assert_eq!(model.ui.window.stage, stage);
        assert!(model.spawned(Ulid(100), SeatId::from_u128(20)));
        model = build(&records, &grids, &model);
        assert_eq!(
            model.spawns[&Ulid(100)].outcome,
            SpawnOutcome::Placed(SeatId::from_u128(20))
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn a_second_spawn_and_refresh_preserve_the_first_explicit_split() {
        let grids = Grids::new();
        let mut records = vec![spawn_record(10), spawn_record(20)];
        let mut model = build(&records, &grids, &Model::default());
        model
            .request_spawn(
                Ulid(100),
                split_spec(10, bbt_proto::seat_state::Side::Right),
            )
            .unwrap();
        records.push(spawn_record(30));
        model.spawned(Ulid(100), SeatId::from_u128(30));
        model = build(&records, &grids, &model);
        let first_tree = spawn_tree(&model);
        model.consume_spawn(Ulid(100));
        let tab = ui::TabId::from_u128(99);
        model
            .request_spawn(
                Ulid(200),
                spawn_spec(Some(bbt_proto::seat_state::Placement::NewTab { tab })),
            )
            .unwrap();
        records.push(spawn_record(40));
        model.spawned(Ulid(200), SeatId::from_u128(40));
        model = build(&records, &grids, &model);
        records[0].name = "metadata refresh".into();
        model = build(&records, &grids, &model);
        assert_eq!(spawn_tree(&model), first_tree);
        assert_eq!(model.ui.window.sidebar.swarms[0].tabs[1].id, tab);
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn pending_spawns_are_bounded_and_a_reused_id_never_requeues_an_effect() {
        let mut model = Model::default();
        let spec = spawn_spec(None);
        for id in 1..=PENDING_SPAWNS as u128 {
            model.request_spawn(Ulid(id), spec.clone()).unwrap();
        }
        assert!(!model.request_spawn(Ulid(1), spec.clone()).unwrap());
        assert_eq!(
            model.request_spawn(Ulid(999), spec.clone()),
            Err("spawn_queue_full")
        );
        let mut conflict = spec;
        conflict.harness_kind = "codex".into();
        assert_eq!(
            model.request_spawn(Ulid(1), conflict),
            Err("spawn_id_conflict")
        );
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn custody_held_spawns_release_in_admission_order_with_the_same_captured_requests() {
        let mut model = Model::default();
        let first = split_spec(10, bbt_proto::seat_state::Side::Right);
        let second = spawn_spec(None);
        model.request_spawn(Ulid(200), first.clone()).unwrap();
        model.request_spawn(Ulid(100), second.clone()).unwrap();
        model.spawn_rejected(Ulid(200), "held: custody unavailable".into());
        model.spawn_rejected(Ulid(100), "held: custody unavailable".into());
        model.consume_spawn(Ulid(200));
        assert_eq!(
            model.resume_held_spawns(),
            vec![(Ulid(200), first), (Ulid(100), second)]
        );
        assert!(model.resume_held_spawns().is_empty());
        model.spawns_disconnected("uncertain send");
        assert!(model.resume_held_spawns().is_empty());
    }

    fn install(grids: &mut Grids, record: &SeatRecord, term: &bbt_core::Term) -> SeatId {
        let id = SeatId::from_u128(record.id.0);
        let view = View {
            seat: record.id,
            generation: record.generation,
            view_generation: Ulid::new(),
        };
        grids.set_watched(&[(id, view)]);
        grids
            .apply(
                id,
                &Update::Snapshot {
                    snapshot: Box::new(Snapshot {
                        view,
                        sequence: 1,
                        screen: Screen::from_term(term, "terminal title".into(), None, None),
                    }),
                },
            )
            .unwrap();
        id
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn metadata_build_keeps_screen_identity_and_canonical_state_ignores_stale_generations() {
        let mut grids = Grids::new();
        let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "first", "zsh");
        let id = install(&mut grids, &record, &bbt_core::Term::new(8, 2, 0));
        let mut first = build(std::slice::from_ref(&record), &grids, &Model::default());
        first.ui.window.sidebar.open.clear();
        record.name = "renamed".into();
        let mut next = build(std::slice::from_ref(&record), &grids, &first);
        assert!(Arc::ptr_eq(&first.screens[&id], &next.screens[&id]));
        assert_eq!(first.ui.window.stage, next.ui.window.stage);
        assert!(next.ui.window.sidebar.open.is_empty());
        assert_eq!(next.ui.seats[&id].name, "renamed");
        let mut seat = GuiSeat {
            attention: None,
            record: record.clone(),
            state: SeatState {
                generation: record.generation,
                process: bbt_proto::ProcessStatus::Running { pid: 123 },
                activity: ui::Activity::Waiting(ui::WaitKind::Permission),
                grade: ui::Grade::Reported,
                title: "permission".into(),
                detail: None,
                turn_open: true,
                observed_ms: 100,
                subagents: vec![],
            },
        };
        next.apply_seat_states(&[seat.clone()]);
        assert_eq!(
            next.ui.seats[&id].activity,
            ui::Activity::Waiting(ui::WaitKind::Permission)
        );
        assert_eq!(next.ui.seats[&id].title, "permission");
        seat.state.generation += 1;
        seat.state.title = "stale process".into();
        next.apply_seat_states(&[seat]);
        assert_eq!(next.ui.seats[&id].title, "permission");
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    fn screen_rows_preserve_palette_styles_width_combining_and_cursor() {
        let mut term = bbt_core::Term::new(12, 2, 0);
        term.advance(b"\x1b]4;1;rgb:aa/bb/cc\x07\x1b[31mX\x1b[0m");
        term.advance("\x1b[1;3;4;9;38;2;10;20;30m中e\u{301}\x1b[0m\x1b[6 q".as_bytes());
        let mut screen = Screen::from_term(&term, String::new(), None, None);
        let grid = screen_grid(&screen, Theme::Dark, true).unwrap();
        assert_eq!(grid.cells[0].foreground, [0xaa, 0xbb, 0xcc, 255]);
        assert_eq!(grid.cells[1].text, "中");
        assert_eq!(grid.cells[1].width, 2);
        assert_eq!(grid.cells[2].width, 0);
        assert!(grid.cells[2].text.is_empty());
        assert_eq!(grid.cells[3].text, "e\u{301}");
        assert_eq!(grid.cells[1].foreground, [10, 20, 30, 255]);
        assert!(grid.cells[1].bold && grid.cells[1].italic && grid.cells[1].strike);
        assert_eq!(grid.cells[1].underline, Underline::Single);
        assert_eq!(grid.cursor.unwrap().shape, CursorShape::Bar);
        screen.cells[0].pop();
        assert!(screen_grid(&screen, Theme::Dark, false).is_err());
    }

    // keeps: spec c1-live-seats §2a
    #[test]
    #[cfg(target_os = "macos")]
    #[allow(
        clippy::disallowed_methods,
        reason = "offline Metal regression: initialize a headless device on the test worker"
    )]
    fn worker_preparation_reuses_unchanged_frames_and_updates_focus_without_grid_copy() {
        fn send<T: Send>() {}
        send::<PanePreparer>();
        let context = Arc::new(bbt_rt::handle().block_on(DeviceContext::new()).unwrap());
        let mut preparer = PanePreparer::new(context).unwrap();
        let record = SeatRecord::new(Ulid::new(), Ulid::new(), "zsh", "zsh");
        let mut grids = Grids::new();
        let id = install(&mut grids, &record, &bbt_core::Term::new(8, 2, 0));
        let mut first = build(std::slice::from_ref(&record), &grids, &Model::default());
        preparer.prepare(&mut first).unwrap();
        let mut next = build(&[record], &grids, &first);
        preparer.prepare(&mut next).unwrap();
        assert!(Arc::ptr_eq(&first.screens[&id], &next.screens[&id]));
        assert!(Arc::ptr_eq(&first.frames[&id], &next.frames[&id]));
        next.ui.window.stage.focused = None;
        preparer.prepare(&mut next).unwrap();
        assert!(Arc::ptr_eq(&first.screens[&id], &next.screens[&id]));
        assert!(!Arc::ptr_eq(&first.frames[&id], &next.frames[&id]));
    }
}

#[cfg(test)]
mod event_lane_tests {
    use super::*;
    use bbt_proto::events::{
        ActivitySummary, EventMetadata, EventPosition, EventProof, EventReference,
        EventSignatureScope, GuiEventEnvelope,
    };

    fn row(stream: &str, sequence: u64) -> GuiEventRecord {
        GuiEventRecord::Event(Box::new(GuiEventEnvelope {
            metadata: EventMetadata {
                reference: EventReference {
                    position: EventPosition {
                        stream: stream.into(),
                        sequence,
                    },
                    hash: format!("{sequence:064x}"),
                },
                id: Ulid::from(u128::from(sequence)),
                ts: sequence,
                subject: "artifact/fixture/created".into(),
                source: "fixture".into(),
                seat: None,
                generation: None,
                proof: EventProof {
                    scope: EventSignatureScope::OriginalRetainedEnvelope,
                    public_key: "opaque fixture proof".into(),
                    signature: "not independently verified by the display cache".into(),
                    prev: String::new(),
                },
            },
            projected_at: sequence,
            cursor: None,
            payload: GuiEventPayload::Activity {
                summary: ActivitySummary {
                    action: "created".into(),
                    id: Some(Ulid::from(u128::from(sequence))),
                    path: None,
                    path_redacted: false,
                    digest: None,
                    before_digest: None,
                    grade: Some("inferred".into()),
                    producer: None,
                    candidates: vec![],
                    status: None,
                    previous_status: None,
                    outcome: None,
                    reason: None,
                    position: None,
                    payload_digest: "f".repeat(64),
                },
            },
        }))
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn rows_dedupe_by_canonical_identity_and_keep_per_stream_order() {
        let mut lane = EventLane::default();
        lane.apply(row("index", 3)).unwrap();
        lane.apply(row("account", 1)).unwrap();
        lane.apply(row("index", 2)).unwrap();
        assert!(!lane.apply(row("index", 2)).unwrap());
        let order: Vec<_> = lane.records().iter().map(|row| record_start(row)).collect();
        assert_eq!(order, [("index", 2), ("index", 3), ("account", 1)]);
        let mut conflict = row("index", 2);
        if let GuiEventRecord::Event(value) = &mut conflict {
            value.metadata.reference.hash = "a".repeat(64);
        }
        assert!(lane.apply(conflict).is_err());
        assert_eq!(lane.records().len(), 3);
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn window_eviction_is_visible_and_old_replays_do_not_repopulate_it() {
        let mut lane = EventLane::default();
        for sequence in 1..=520 {
            lane.apply(row("index", sequence)).unwrap();
        }
        assert_eq!(lane.records().len(), 512);
        assert!(lane.bytes() <= EVENT_BYTES);
        assert_eq!(
            (lane.omitted()["index"].first, lane.omitted()["index"].last),
            (1, 8)
        );
        assert!(lane.omitted()["index"].reason.contains("original retained"));
        assert!(!lane.apply(row("index", 4)).unwrap());
        assert_eq!(lane.artifacts().created, 512);
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn byte_budget_is_independent_of_row_count() {
        let mut lane = EventLane::default();
        for sequence in 1..=500 {
            let mut event = row("index", sequence);
            if let GuiEventRecord::Event(event) = &mut event {
                event.metadata.source = "s".repeat(512);
                if let GuiEventPayload::Activity { summary } = &mut event.payload {
                    summary.path = Some("p".repeat(512));
                    summary.reason = Some("r".repeat(256));
                    summary.candidates = (0..16_u128).map(Ulid::from).collect();
                }
            }
            lane.apply(event).unwrap();
        }
        assert!(lane.records().len() < 500);
        assert!(lane.bytes() <= EVENT_BYTES);
        assert!(lane.omitted().contains_key("index"));
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn stale_transport_snapshots_cannot_roll_back_operator_read_state() {
        let mut lane = EventLane::default();
        let cursor = GuiEventCursor {
            stream: "index".into(),
            admitted: 20,
            acknowledged: 15,
            read: 10,
            high: 20,
            unread: 2,
        };
        lane.apply_cursor(cursor.clone()).unwrap();
        let mut old = cursor.clone();
        old.read = 0;
        old.high = 21;
        old.unread = 12;
        assert!(!lane.apply_cursor(old).unwrap());
        assert_eq!(lane.cursors()["index"].read, 10);
        let mut delayed = cursor.clone();
        delayed.admitted = 18;
        delayed.acknowledged = 12;
        assert!(!lane.apply_cursor(delayed).unwrap());
        assert_eq!(lane.cursors()["index"].acknowledged, 15);
        let mut invalid = cursor;
        invalid.read = 30;
        assert!(lane.apply_cursor(invalid).is_err());
        assert_eq!(lane.cursors()["index"].read, 10);
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn sparse_history_pages_keep_next_without_acknowledging_or_marking_read() {
        let mut lane = EventLane::default();
        let cursor = GuiEventCursor {
            stream: "index".into(),
            admitted: 20,
            acknowledged: 10,
            read: 3,
            high: 100,
            unread: 7,
        };
        assert!(
            lane.apply_page(GuiEventPage {
                records: vec![row("index", 50)],
                next: Some(80),
                cursor: cursor.clone()
            })
            .unwrap()
        );
        assert_eq!(lane.pages()["index"], Some(80));
        assert_eq!(
            lane.page_snapshots()["index"],
            PageSnapshot {
                revision: 1,
                high: 100,
                next: Some(80)
            }
        );
        assert_eq!(lane.read_page().len(), 1);
        assert_eq!(lane.cursors()["index"], cursor);
        assert!(
            lane.apply_page(GuiEventPage {
                records: vec![],
                next: None,
                cursor: cursor.clone()
            })
            .unwrap()
        );
        assert_eq!(lane.pages()["index"], None);
        assert!(lane.records().is_empty());
        assert!(lane.read_page().is_empty());
        assert_eq!(lane.page_snapshots()["index"].revision, 2);
        assert!(
            lane.apply_page(GuiEventPage {
                records: vec![row("account", 2)],
                next: None,
                cursor
            })
            .is_err()
        );
        assert!(!lane.cursors().contains_key("account"));
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn requested_old_page_survives_a_full_live_window_until_replaced() {
        let mut lane = EventLane::default();
        for sequence in 1..=600 {
            lane.apply(row("index", sequence)).unwrap();
        }
        let cursor = GuiEventCursor {
            stream: "index".into(),
            admitted: 600,
            acknowledged: 599,
            read: 2,
            high: 600,
            unread: 0,
        };
        let original = row("index", 4);
        lane.apply_page(GuiEventPage {
            records: vec![original.clone()],
            next: Some(80),
            cursor: cursor.clone(),
        })
        .unwrap();
        assert_eq!(lane.read_page()[0].as_ref(), &original);
        assert_eq!(lane.records().len() + lane.read_page().len(), EVENT_ROWS);
        lane.apply(row("index", 601)).unwrap();
        assert_eq!(lane.read_page()[0].as_ref(), &original);
        assert!(lane.bytes() <= EVENT_BYTES);
        lane.apply_page(GuiEventPage {
            records: vec![row("index", 81)],
            next: Some(150),
            cursor,
        })
        .unwrap();
        assert_eq!(record_start(&lane.read_page()[0]), ("index", 81));
        assert_eq!(lane.page_snapshots()["index"].revision, 2);
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn empty_pages_and_refusals_have_distinct_monotonic_completion_revisions() {
        let mut lane = EventLane::default();
        let page = GuiEventPage {
            records: vec![],
            next: None,
            cursor: GuiEventCursor {
                stream: "index".into(),
                admitted: 0,
                acknowledged: 0,
                read: 0,
                high: 0,
                unread: 0,
            },
        };
        assert!(lane.apply_page(page.clone()).unwrap());
        assert!(lane.apply_page(page).unwrap());
        assert_eq!(lane.page_snapshots()["index"].revision, 2);
        assert!(
            lane.page_refused("account", "refused another stream")
                .unwrap()
        );
        assert_eq!(lane.page_errors()["account"].revision, 1);
        lane.page_refused("index", &"x".repeat(1000)).unwrap();
        assert_eq!(lane.page_errors()["index"].revision, 3);
        assert_eq!(lane.page_errors()["index"].reason.len(), 256);
        assert_eq!(lane.page_snapshots()["index"].revision, 2);
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn a_large_newest_live_record_takes_precedence_over_read_page_bytes() {
        let mut lane = EventLane::default();
        let mut page_row = row("index", 1);
        if let GuiEventRecord::Event(row) = &mut page_row {
            row.metadata.source = "p".repeat(100_000);
        }
        let cursor = GuiEventCursor {
            stream: "index".into(),
            admitted: 2,
            acknowledged: 1,
            read: 0,
            high: 2,
            unread: 0,
        };
        lane.apply_page(GuiEventPage {
            records: vec![page_row],
            next: None,
            cursor,
        })
        .unwrap();
        let mut live = row("index", 2);
        if let GuiEventRecord::Event(row) = &mut live {
            row.metadata.source = "l".repeat(980_000);
        }
        lane.apply(live).unwrap();
        assert_eq!(lane.records().len(), 1);
        assert!(lane.read_page().is_empty());
        assert!(
            lane.read_gap()
                .unwrap()
                .reason
                .contains("live window takes precedence")
        );
        assert!(lane.bytes() <= EVENT_BYTES);
    }

    // keeps: spec l9-messaging-tasks §3
    #[test]
    fn metadata_rebuild_and_reconnect_preserve_the_published_window() {
        let mut model = Model::default();
        let id = Ulid::new();
        let lane = Arc::make_mut(&mut model.events);
        lane.begin_attach(id);
        lane.attach_result(id, None);
        lane.apply(row("index", 1)).unwrap();
        let next = build(&[], &Grids::new(), &model);
        assert!(Arc::ptr_eq(&next.events, &model.events));
        let mut next = next;
        let lane = Arc::make_mut(&mut next.events);
        lane.disconnected("logic restarted");
        assert_eq!(lane.records().len(), 1);
        assert!(!lane.attach_result(Ulid::new(), None));
        let retry = Ulid::new();
        lane.begin_attach(retry);
        assert!(lane.attach_result(retry, None));
        assert!(!lane.apply(row("index", 1)).unwrap());
        assert_eq!(lane.records().len(), 1);
    }
}
