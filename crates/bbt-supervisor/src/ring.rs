//! Supervisor-owned, CRC32C framed capture with atomic committed boundaries.
//! Logical hard-line indexes are produced by logic, never guessed by capture.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use bbt_core::{Checkpoint, Term, vte};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::durability::{sync_directory, sync_file};
use crate::protocol::{Event, ExitInfo, OUTPUT_CHUNK_BYTES, SeatSize};
use crate::store::{Blobs, Result, blobs::LocalBlobs, invalid};

mod journal;
mod writer;
pub(crate) use writer::{
    CaptureWriter, Handle as CaptureHandle, Query as CaptureQuery, Response as CaptureResponse,
};

const HEADER: usize = 56;
const MAX_RECORD: usize = 1024 * 1024;
const MAX_CHECKPOINT: usize = 128 * 1024;
const MAX_DECODED: u64 = 16 * 1024 * 1024;
const MAX_POINTS: usize = 4096;
const PAGE_BYTES: usize = 64 * 1024;

/// Cumulative capture timings; durations use monotonic nanoseconds.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureStats {
    pub records: u64,
    pub encode_ns: u64,
    pub hot_write_ns: u64,
    pub sync_ns: u64,
    pub marker_ns: u64,
    pub cold_ns: u64,
    pub seals: u64,
    pub max_seal_ns: u64,
    pub queued_bytes: usize,
    pub peak_queue_bytes: usize,
    pub queue_full_count: u64,
    #[serde(default)]
    pub ordered_commits: u64,
    #[serde(default)]
    pub durable_commits: u64,
    /// Shared writer counters, repeated on each seat: do not sum across seats.
    #[serde(default)]
    pub writer_busy_ns: u64,
    #[serde(default)]
    pub commit_rounds: u64,
    /// Cumulative round p99, rounded up to 1 ms (overflow uses the exact max).
    #[serde(default)]
    pub commit_p99_ns: u64,
    #[serde(default)]
    pub commit_max_ns: u64,
    #[serde(default)]
    pub undurable_bytes: usize,
    #[serde(default)]
    pub peak_undurable_bytes: usize,
    #[serde(default)]
    pub durable_pauses: u64,
    #[serde(default)]
    pub durable_rounds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub capacity_bytes: u64,
    pub checkpoint_bytes: u64,
    pub idle_ms: u64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            capacity_bytes: 4 * 1024 * 1024,
            checkpoint_bytes: 64 * 1024,
            idle_ms: 200,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if self.capacity_bytes < 4096
            || self.capacity_bytes > 64 * 1024 * 1024
            || self.checkpoint_bytes == 0
            || self.checkpoint_bytes > self.capacity_bytes / 4
            || self.idle_ms == 0
            || self.idle_ms > 60_000
        {
            return Err(invalid("invalid ring/checkpoint budget"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowPoint {
    pub sequence: u64,
    pub first_row: u64,
    pub last_row: u64,
    pub epoch: u64,
}

/// Absolute *logical hard-line* coordinates, not reflowed physical rows.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct RowTracker {
    parser: vte::Parser,
    pub row: u64,
    pub epoch: u64,
}
struct RowEvents<'a>(&'a mut u64, &'a mut u64);
impl vte::Perform for RowEvents<'_> {
    fn execute(&mut self, byte: u8) {
        if matches!(byte, b'\n' | 0x0b | 0x0c) {
            *self.0 = self.0.saturating_add(1);
        }
    }
    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        ignore: bool,
        action: char,
    ) {
        if ignore {
            return;
        }
        // Invalidate physical-layout references on clear or alternate screen.
        let first = params
            .iter()
            .next()
            .and_then(|p| p.first())
            .copied()
            .unwrap_or(0);
        if (action == 'J' && matches!(first, 2 | 3))
            || (intermediates == b"?"
                && matches!(action, 'h' | 'l')
                && params
                    .iter()
                    .any(|p| p.first().is_some_and(|n| matches!(*n, 47 | 1047 | 1049))))
        {
            *self.1 = self.1.saturating_add(1);
        }
    }
    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        if ignore || !intermediates.is_empty() {
            return;
        }
        if matches!(byte, b'D' | b'E') {
            *self.0 = self.0.saturating_add(1);
        }
        if byte == b'c' {
            *self.1 = self.1.saturating_add(1);
        }
    }
}
impl RowTracker {
    fn advance(&mut self, bytes: &[u8]) {
        self.parser
            .advance(&mut RowEvents(&mut self.row, &mut self.epoch), bytes);
    }
}

/// Versioned state produced on logic's IO worker and stored by the supervisor.
#[derive(Clone, Serialize, Deserialize)]
pub struct Saved {
    pub version: u16,
    pub seat: Ulid,
    pub generation: u64,
    pub sequence: u64,
    pub byte_end: u64,
    pub cols: usize,
    pub rows: usize,
    pub checkpoint: Checkpoint,
    pub title: String,
    pub cwd: Option<String>,
    pub gap: Option<String>,
    pub tracker: RowTracker,
    pub index: Vec<RowPoint>,
}
impl Saved {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let bytes = serde_json::to_vec(self)?;
        if bytes.len() as u64 > MAX_DECODED {
            return Err(invalid("checkpoint decoded budget exceeded"));
        }
        let compressed = zstd::stream::encode_all(bytes.as_slice(), 1)?;
        if compressed.len() > MAX_CHECKPOINT {
            return Err(invalid("checkpoint frame budget exceeded"));
        }
        Ok(compressed)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_CHECKPOINT {
            return Err(invalid("oversized checkpoint"));
        }
        let mut decoded = Vec::new();
        zstd::stream::read::Decoder::new(bytes)?
            .take(MAX_DECODED + 1)
            .read_to_end(&mut decoded)?;
        if decoded.len() as u64 > MAX_DECODED {
            return Err(invalid("checkpoint decompression budget exceeded"));
        }
        let saved: Self = serde_json::from_slice(&decoded)?;
        if saved.version != 1
            || saved.generation == 0
            || saved.seat == Ulid::nil()
            || saved.cols < 2
            || saved.rows == 0
            || saved.cols > 512
            || saved.rows > 256
            || saved.cols * saved.rows > 32768
            || saved.index.len() > MAX_POINTS
            || saved
                .index
                .iter()
                .any(|p| p.sequence > saved.sequence || p.first_row > p.last_row)
            || saved
                .index
                .windows(2)
                .any(|p| p[0].sequence >= p[1].sequence)
        {
            return Err(invalid(
                "invalid checkpoint version/identity/index/dimensions",
            ));
        }
        Ok(saved)
    }
}

/// Isolated replay drains generated effects instead of sending them to a PTY.
pub struct Playback {
    pub term: Term,
    pub seat: Ulid,
    pub generation: u64,
    pub sequence: u64,
    pub byte_end: u64,
    pub title: String,
    pub cwd: Option<String>,
    pub gap: Option<String>,
    pub tracker: RowTracker,
    pub index: VecDeque<RowPoint>,
}
impl Playback {
    pub fn new(seat: Ulid, generation: u64, size: SeatSize) -> Self {
        Self {
            term: Term::new(size.cols as usize, size.rows as usize, 0),
            seat,
            generation,
            sequence: 0,
            byte_end: 0,
            title: String::new(),
            cwd: None,
            gap: None,
            tracker: RowTracker::default(),
            index: VecDeque::new(),
        }
    }
    pub fn restore(saved: Saved) -> Self {
        let mut term = Term::new(saved.cols, saved.rows, 0);
        term.restore(saved.checkpoint);
        Self {
            term,
            seat: saved.seat,
            generation: saved.generation,
            sequence: saved.sequence,
            byte_end: saved.byte_end,
            title: saved.title,
            cwd: saved.cwd,
            gap: saved.gap,
            tracker: saved.tracker,
            index: saved.index.into(),
        }
    }
    pub fn saved(&self) -> Saved {
        use bbt_core::Dimensions;
        Saved {
            version: 1,
            seat: self.seat,
            generation: self.generation,
            sequence: self.sequence,
            byte_end: self.byte_end,
            cols: self.term.grid().columns(),
            rows: self.term.grid().screen_lines(),
            checkpoint: self.term.checkpoint(),
            title: self.title.clone(),
            cwd: self.cwd.clone(),
            gap: self.gap.clone(),
            tracker: self.tracker.clone(),
            index: self.index.iter().cloned().collect(),
        }
    }
    pub fn apply(&mut self, event: &Event) -> Result<()> {
        if event.seat() != self.seat
            || event_generation(event) != self.generation
            || event.sequence() != Some(self.sequence + 1)
        {
            return Err(invalid("replay identity/cursor discontinuity"));
        }
        let first_row = self.tracker.row;
        match event {
            Event::Output { bytes, .. } => {
                self.tracker.advance(bytes);
                self.term.advance(bytes);
                self.byte_end += bytes.len() as u64;
            }
            Event::Resized { size, .. } => {
                self.term.resize(size.cols as usize, size.rows as usize);
                self.tracker.epoch = self.tracker.epoch.saturating_add(1);
            }
            Event::Exited { .. } => (),
        }
        for effect in self.term.take_events() {
            match effect {
                bbt_core::Event::Title(title) => self.title = title,
                bbt_core::Event::ResetTitle => self.title.clear(),
                _ => (),
            }
        }
        self.sequence += 1;
        self.index.push_back(RowPoint {
            sequence: self.sequence,
            first_row,
            last_row: self.tracker.row,
            epoch: self.tracker.epoch,
        });
        if self.index.len() > MAX_POINTS {
            self.index.pop_front();
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Status {
    pub generation: u64,
    pub first_sequence: u64,
    pub committed_sequence: u64,
    pub captured_sequence: u64,
    pub byte_start: u64,
    pub byte_end: u64,
    pub hot_bytes: u64,
    pub paused: bool,
    pub recovery_notice: Option<String>,
    #[serde(default)]
    pub capture: CaptureStats,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedEvent {
    pub event: Event,
    pub row: Option<RowPoint>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct History {
    pub status: Status,
    pub records: Vec<IndexedEvent>,
    pub gap: Option<HistoryGap>,
    pub next: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Boundary {
    #[serde(default)]
    journal_position: Option<journal::Position>,
    version: u16,
    seat: Ulid,
    generation: u64,
    file: String,
    length: u64,
    base: u64,
    sequence: u64,
    byte_start: u64,
    byte_end: u64,
    settings: Settings,
    #[serde(default)]
    sealed_head: Option<String>,
    #[serde(default)]
    origin_base: Option<u64>,
    #[serde(default)]
    origin_byte_start: Option<u64>,
    #[serde(default)]
    checkpoints: Vec<CheckpointRef>,
}
#[derive(Clone, Serialize, Deserialize)]
struct CheckpointRef {
    sequence: u64,
    digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Sealed {
    previous: Option<String>,
    data: String,
    file: String,
    seat: Ulid,
    generation: u64,
    base: u64,
    sequence: u64,
    byte_start: u64,
    byte_end: u64,
    length: u64,
}
struct Hydrated {
    digest: String,
    bytes: Vec<u8>,
    entries: Vec<Entry>,
    rows: std::collections::BTreeMap<u64, RowPoint>,
}
#[derive(Clone)]
struct Entry {
    start: u64,
    end: u64,
    kind: u16,
    sequence: u64,
    byte_start: u64,
}

pub struct Ring {
    root: PathBuf,
    boundary: Boundary,
    entries: Vec<Entry>,
    index: std::collections::BTreeMap<u64, RowPoint>,
    notice: Option<String>,
    sealed: Vec<Sealed>,
    hydrated: RefCell<Option<Hydrated>>,
    durable: Boundary,
    last_commit: std::time::Instant,
    stats: CaptureStats,
    group_bytes: u64,
    append_file: Option<File>,
    grouped: bool,
    strong_dirty: bool,
}
impl Ring {
    pub fn open(
        root: &Path,
        seat: Ulid,
        generation: u64,
        base: u64,
        settings: Settings,
    ) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        settings.validate()?;
        if seat == Ulid::nil() || generation == 0 {
            return Err(invalid("invalid ring identity"));
        }
        std::fs::create_dir_all(root)?;
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let meta = std::fs::symlink_metadata(root)?;
        // SAFETY: geteuid takes no arguments and changes no state.
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } {
            return Err(invalid("ring directory must be owned and not a symlink"));
        }
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))?;
        let mut failures = Vec::new();
        for name in ["committed", "previous"] {
            match load_boundary(&root.join(name)) {
                Ok(boundary) => {
                    if boundary.seat != seat
                        || boundary.generation != generation
                        || boundary.settings != settings
                    {
                        return Err(invalid("ring generation/settings mismatch"));
                    }
                    match scan(root, &boundary) {
                        Ok(entries) => {
                            let actual = std::fs::metadata(root.join(&boundary.file))?.len();
                            let notice = if name == "previous" || actual > boundary.length {
                                Some(format!(
                                    "rejected uncommitted/torn suffix; recovered through {}",
                                    boundary.sequence
                                ))
                            } else {
                                None
                            };
                            let mut ring = Self {
                                root: root.into(),
                                durable: boundary.clone(),
                                last_commit: std::time::Instant::now(),
                                stats: CaptureStats::default(),
                                group_bytes: 64 * 1024,
                                append_file: None,
                                grouped: false,
                                strong_dirty: false,
                                boundary,
                                entries,
                                index: Default::default(),
                                notice,
                                sealed: Vec::new(),
                                hydrated: RefCell::new(None),
                            };
                            // Never append over a rejected suffix. Retain it in cold blobs first.
                            if actual > ring.boundary.length {
                                ring.archive(&ring.boundary.file)?;
                                let file = OpenOptions::new().write(true).open(ring.data_path())?;
                                file.set_len(ring.boundary.length)?;
                                file.sync_all()?;
                            }
                            if ring.notice.is_some() {
                                // Repair current without rotating an invalid marker over the
                                // independently valid previous boundary.
                                ring.write_marker("committed", &ring.boundary)?;
                            }
                            ring.load_sealed()?;
                            ring.rebuild_index()?;
                            ring.retire_orphans()?;
                            journal::recover(&mut ring)?;
                            return Ok(ring);
                        }
                        Err(e) => failures.push(e.to_string()),
                    }
                }
                Err(e) if !root.join(name).exists() => {
                    let _ = e;
                }
                Err(e) => failures.push(e.to_string()),
            }
        }
        if !failures.is_empty() {
            return Err(invalid(format!(
                "no valid committed ring boundary: {}",
                failures.join("; ")
            )));
        }
        let boundary = Boundary {
            journal_position: None,
            version: 1,
            seat,
            generation,
            file: format!("data-{}.log", Ulid::new()),
            length: 0,
            base,
            sequence: base,
            byte_start: 0,
            byte_end: 0,
            settings,
            sealed_head: None,
            origin_base: Some(base),
            origin_byte_start: Some(0),
            checkpoints: Vec::new(),
        };
        let ring = Self {
            root: root.into(),
            durable: boundary.clone(),
            last_commit: std::time::Instant::now(),
            stats: CaptureStats::default(),
            group_bytes: 64 * 1024,
            append_file: None,
            grouped: false,
            strong_dirty: false,
            boundary,
            entries: Vec::new(),
            index: Default::default(),
            notice: None,
            sealed: Vec::new(),
            hydrated: RefCell::new(None),
        };
        File::create(ring.data_path())?.sync_all()?;
        ring.publish_boundary(&ring.boundary)?;
        Ok(ring)
    }
    fn data_path(&self) -> PathBuf {
        self.root.join(&self.boundary.file)
    }
    fn put_blob(&self, bytes: &[u8]) -> Result<String> {
        let blobs = LocalBlobs::open(&self.root)?;
        blobs.put(bytes)
    }
    pub fn directory(&self) -> &Path {
        &self.root
    }
    pub fn settings(&self) -> &Settings {
        &self.boundary.settings
    }
    pub fn status(&self) -> Status {
        Status {
            generation: self.boundary.generation,
            first_sequence: self.first_base() + 1,
            committed_sequence: self.durable.sequence,
            captured_sequence: self.boundary.sequence,
            byte_start: self
                .boundary
                .origin_byte_start
                .unwrap_or(self.boundary.byte_start),
            byte_end: self.boundary.byte_end,
            hot_bytes: self.boundary.length,
            paused: false,
            recovery_notice: self.notice.clone(),
            capture: self.stats.clone(),
        }
    }
    fn first_base(&self) -> u64 {
        self.boundary.origin_base.unwrap_or(self.boundary.base)
    }
    pub fn can_capture(&self) -> bool {
        true
    }
    pub fn capture_capacity(&self) -> usize {
        OUTPUT_CHUNK_BYTES.min(self.boundary.settings.capacity_bytes as usize - HEADER)
    }
    pub fn can_control(&self) -> bool {
        true
    }
    pub fn prepare_capture(&mut self) -> Result<()> {
        if self.boundary.length + (HEADER + self.capture_capacity()) as u64
            > self.boundary.settings.capacity_bytes
        {
            self.seal_hot()?;
        }
        Ok(())
    }
    fn checkpoint_refs(&self) -> Result<Vec<CheckpointRef>> {
        let mut refs = self.boundary.checkpoints.clone();
        for entry in self.entries.iter().filter(|e| e.kind == 4).rev().take(2) {
            refs.push(CheckpointRef {
                sequence: entry.sequence,
                digest: self.put_blob(&self.payload(entry)?)?,
            });
        }
        refs.sort_by_key(|r| r.sequence);
        refs.dedup_by_key(|r| r.sequence);
        if refs.len() > 2 {
            refs.drain(..refs.len() - 2);
        }
        Ok(refs)
    }
    pub fn latest_checkpoint(&self) -> Result<Option<Vec<u8>>> {
        if let Some(entry) = self.entries.iter().rev().find(|e| e.kind == 4) {
            if self
                .boundary
                .checkpoints
                .last()
                .is_none_or(|r| entry.sequence >= r.sequence)
            {
                return self.payload(entry).map(Some);
            }
        }
        if let Some(reference) = self.boundary.checkpoints.last() {
            let bytes = LocalBlobs::open(&self.root)?.get(&reference.digest)?;
            let saved = Saved::decode(&bytes)?;
            if saved.sequence != reference.sequence
                || saved.seat != self.boundary.seat
                || saved.generation != self.boundary.generation
            {
                return Err(invalid("checkpoint reference identity mismatch"));
            }
            return Ok(Some(bytes));
        }
        self.entries
            .iter()
            .rev()
            .find(|e| e.kind == 4)
            .map(|e| self.payload(e))
            .transpose()
    }
    fn load_sealed(&mut self) -> Result<()> {
        let blobs = LocalBlobs::open(&self.root)?;
        let mut head = self.boundary.sealed_head.clone();
        let mut expected = self.boundary.base;
        let mut byte_end = self.boundary.byte_start;
        self.sealed.clear();
        let mut seen = std::collections::BTreeSet::new();
        while let Some(digest) = head {
            if !seen.insert(digest.clone()) {
                return Err(invalid("cyclic sealed segment chain"));
            }
            let bytes = blobs.get(&digest)?;
            if bytes.len() > 4096 {
                return Err(invalid("oversized sealed descriptor"));
            }
            let segment: Sealed = serde_json::from_slice(&bytes)?;
            if segment.seat != self.boundary.seat
                || segment.generation != self.boundary.generation
                || segment.sequence != expected
                || segment.byte_end != byte_end
                || segment.base > segment.sequence
                || segment.length > self.boundary.settings.capacity_bytes
                || segment.byte_start > segment.byte_end
            {
                return Err(invalid("sealed segment continuity mismatch"));
            }
            head = segment.previous.clone();
            expected = segment.base;
            byte_end = segment.byte_start;
            self.sealed.push(segment);
        }
        if expected != self.first_base() {
            return Err(invalid("sealed origin mismatch"));
        }
        self.sealed.reverse();
        Ok(())
    }
    fn seal_hot(&mut self) -> Result<()> {
        if self.boundary.length == 0 {
            return Ok(());
        }
        let seal_started = std::time::Instant::now();
        self.flush()?;
        let cold_started = std::time::Instant::now();
        let raw = std::fs::read(self.data_path())?;
        if raw.len() as u64 != self.boundary.length {
            return Err(invalid("cannot seal an uncommitted tail"));
        }
        let data = self.put_blob(&raw)?;
        let segment = Sealed {
            previous: self.boundary.sealed_head.clone(),
            data,
            file: self.boundary.file.clone(),
            seat: self.boundary.seat,
            generation: self.boundary.generation,
            base: self.boundary.base,
            sequence: self.boundary.sequence,
            byte_start: self.boundary.byte_start,
            byte_end: self.boundary.byte_end,
            length: self.boundary.length,
        };
        let digest = self.put_blob(&serde_json::to_vec(&segment)?)?;
        self.stats.cold_ns += cold_started.elapsed().as_nanos() as u64;
        let mut next = self.boundary.clone();
        next.origin_base = Some(self.first_base());
        next.origin_byte_start = Some(
            self.boundary
                .origin_byte_start
                .unwrap_or(self.boundary.byte_start),
        );
        next.sealed_head = Some(digest);
        next.checkpoints = self.checkpoint_refs()?;
        next.file = format!("data-{}.log", Ulid::new());
        next.length = 0;
        next.base = next.sequence;
        next.byte_start = next.byte_end;
        let next_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(self.root.join(&next.file))?;
        sync_file(&next_file, true)?;
        self.publish_boundary_tier(&next, true)?;
        self.durable = next.clone();
        self.last_commit = std::time::Instant::now();
        self.boundary = next;
        self.append_file = None;
        self.entries.clear();
        self.sealed.push(segment);
        self.rebuild_index()?;
        self.retire_orphans()?;
        self.stats.seals += 1;
        self.stats.max_seal_ns = self
            .stats
            .max_seal_ns
            .max(seal_started.elapsed().as_nanos() as u64);
        Ok(())
    }
    fn hydrate(&self, segment: &Sealed) -> Result<()> {
        if self
            .hydrated
            .borrow()
            .as_ref()
            .is_some_and(|h| h.digest == segment.data)
        {
            return Ok(());
        }
        let bytes = LocalBlobs::open(&self.root)?.get(&segment.data)?;
        if bytes.len() as u64 != segment.length {
            return Err(invalid("sealed segment length mismatch"));
        }
        let mut b = self.boundary.clone();
        b.length = segment.length;
        b.base = segment.base;
        b.sequence = segment.sequence;
        b.byte_start = segment.byte_start;
        b.byte_end = segment.byte_end;
        let entries = scan_reader(&mut std::io::Cursor::new(&bytes), &b)?;
        let mut rows = std::collections::BTreeMap::new();
        for e in entries.iter().filter(|e| e.kind == 4) {
            for point in Saved::decode(&bytes[e.start as usize + HEADER..e.end as usize])?.index {
                rows.insert(point.sequence, point);
            }
        }
        *self.hydrated.borrow_mut() = Some(Hydrated {
            digest: segment.data.clone(),
            bytes,
            entries,
            rows,
        });
        Ok(())
    }
    pub fn checkpoint(&mut self, bytes: Vec<u8>) -> Result<()> {
        let saved = Saved::decode(&bytes)?;
        if saved.seat != self.boundary.seat
            || saved.generation != self.boundary.generation
            || saved.sequence > self.boundary.sequence
            || saved.sequence < self.first_base()
            || saved.byte_end != self.byte_end_at(saved.sequence)?
        {
            return Err(invalid("checkpoint does not cover a captured cursor"));
        }
        if let Some(old) = self.latest_checkpoint()? {
            let old_sequence = Saved::decode(&old)?.sequence;
            if saved.sequence < old_sequence {
                return Err(invalid("stale checkpoint"));
            }
            if saved.sequence == old_sequence {
                if old == bytes {
                    return Ok(());
                }
                return Err(invalid("checkpoint cursor reused with different state"));
            }
        }
        self.append(4, saved.sequence, saved.byte_end, &bytes)?;
        for point in saved.index {
            self.index.insert(point.sequence, point);
        }
        Ok(())
    }
    pub fn append_event(&mut self, event: &Event) -> Result<()> {
        if event.seat() != self.boundary.seat
            || event_generation(event) != self.boundary.generation
            || event.sequence() != Some(self.boundary.sequence + 1)
        {
            return Err(invalid("ring event discontinuity"));
        }
        let (kind, payload) = encode_event(event)?;
        self.append(
            kind,
            event.sequence().unwrap(),
            self.boundary.byte_end,
            &payload,
        )
    }
    fn append(&mut self, kind: u16, sequence: u64, byte_start: u64, payload: &[u8]) -> Result<()> {
        bbt_rt::assert_not_ui_thread();
        let encode_started = std::time::Instant::now();
        let bytes = encode(
            kind,
            self.boundary.seat,
            self.boundary.generation,
            sequence,
            byte_start,
            payload,
        )?;
        self.stats.encode_ns += encode_started.elapsed().as_nanos() as u64;
        self.stats.records += 1;
        if bytes.len() as u64 > self.boundary.settings.capacity_bytes {
            return Err(invalid("record exceeds ring capacity"));
        }
        if self.boundary.length + bytes.len() as u64 > self.boundary.settings.capacity_bytes {
            self.seal_hot()?;
        }
        let start = self.boundary.length;
        let write_started = std::time::Instant::now();
        if self.append_file.is_none() {
            self.append_file = Some(OpenOptions::new().append(true).open(self.data_path())?);
        }
        let file = self.append_file.as_mut().unwrap();
        file.write_all(&bytes)?;
        self.stats.hot_write_ns += write_started.elapsed().as_nanos() as u64;
        let mut next = self.boundary.clone();
        next.length += bytes.len() as u64;
        if kind == 4 && !self.grouped {
            next.checkpoints = self.checkpoint_refs()?;
            next.checkpoints.push(CheckpointRef {
                sequence,
                digest: self.put_blob(payload)?,
            });
            if next.checkpoints.len() > 2 {
                next.checkpoints.remove(0);
            }
        }
        if kind != 4 {
            next.sequence = sequence;
        }
        if kind == 1 {
            next.byte_end += payload.len() as u64;
        }
        self.boundary = next;
        self.entries.push(Entry {
            start,
            end: self.boundary.length,
            kind,
            sequence,
            byte_start,
        });
        if !self.grouped
            && (kind != 1
                || self.boundary.length.saturating_sub(self.durable.length) >= self.group_bytes)
        {
            self.flush()?;
        }
        Ok(())
    }
    /// Direct callers and segment seals require a durable derived boundary.
    pub fn flush(&mut self) -> Result<()> {
        self.commit(true)
    }
    fn commit(&mut self, strong: bool) -> Result<()> {
        bbt_rt::assert_not_ui_thread();
        if self.boundary.file == self.durable.file
            && self.boundary.length == self.durable.length
            && (!strong || !self.strong_dirty)
        {
            return Ok(());
        }
        let sync_started = std::time::Instant::now();
        let file = OpenOptions::new().write(true).open(self.data_path())?;
        if file.metadata()?.len() != self.boundary.length {
            return Err(invalid("untracked/torn capture suffix before commit"));
        }
        sync_file(&file, strong)?;
        self.stats.sync_ns += sync_started.elapsed().as_nanos() as u64;
        let marker_started = std::time::Instant::now();
        self.publish_boundary_tier(&self.boundary, strong)?;
        self.durable = self.boundary.clone();
        self.last_commit = std::time::Instant::now();
        self.strong_dirty = !strong;
        if strong {
            self.stats.durable_commits += 1;
            self.retire_orphans()?;
        } else {
            self.stats.ordered_commits += 1;
        }
        self.stats.marker_ns += marker_started.elapsed().as_nanos() as u64;
        Ok(())
    }
    pub fn flush_due(&mut self) -> Result<()> {
        if self.last_commit.elapsed() >= std::time::Duration::from_millis(20) {
            self.flush()?;
        }
        Ok(())
    }
    fn write_marker(&self, name: &str, boundary: &Boundary) -> Result<()> {
        let bytes = serde_json::to_vec(boundary)?;
        let mut wrapped = bytes.clone();
        wrapped.extend(crc32c(&bytes).to_le_bytes());
        let temp = self.root.join(format!(".commit-{}", Ulid::new()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(&wrapped)?;
        file.sync_all()?;
        std::fs::rename(temp, self.root.join(name))?;
        File::open(&self.root)?.sync_all()?;
        Ok(())
    }
    fn publish_boundary(&self, boundary: &Boundary) -> Result<()> {
        self.publish_boundary_tier(boundary, true)
    }
    fn publish_boundary_tier(&self, boundary: &Boundary, strong: bool) -> Result<()> {
        let bytes = serde_json::to_vec(boundary)?;
        let mut wrapped = bytes.clone();
        wrapped.extend(crc32c(&bytes).to_le_bytes());
        let temp = self.root.join(format!(".commit-{}", Ulid::new()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(&wrapped)?;
        sync_file(&file, strong)?;
        if self.root.join("committed").exists() {
            std::fs::rename(self.root.join("committed"), self.root.join("previous"))?;
        }
        std::fs::rename(temp, self.root.join("committed"))?;
        if strong {
            sync_directory(&File::open(&self.root)?)?;
            sync_file(&file, true)?;
        }
        Ok(())
    }
    fn archive(&self, name: &str) -> Result<()> {
        if std::fs::metadata(self.root.join(name))?.len()
            > self.boundary.settings.capacity_bytes + MAX_RECORD as u64
        {
            return Err(invalid("oversized corrupt suffix retained in place"));
        }
        let bytes = std::fs::read(self.root.join(name))?;
        let hash = LocalBlobs::open(&self.root)?.put(&bytes)?;
        let mut index = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join("cold.jsonl"))?;
        serde_json::to_writer(
            &mut index,
            &serde_json::json!({"file":name,"digest":hash,"generation":self.boundary.generation}),
        )?;
        index.write_all(b"\n")?;
        index.sync_all()?;
        Ok(())
    }
    fn retire_orphans(&self) -> Result<()> {
        let previous = load_boundary(&self.root.join("previous")).ok();
        for entry in std::fs::read_dir(&self.root)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with("data-")
                && name.ends_with(".log")
                && name != self.boundary.file
                && previous.as_ref().is_none_or(|p| p.file != name)
            {
                if !self.sealed.iter().any(|s| s.file == name) {
                    self.archive(&name)?;
                }
                std::fs::remove_file(entry.path())?;
            }
        }
        Ok(())
    }
    fn raw(&self, entry: &Entry) -> Result<Vec<u8>> {
        bbt_rt::assert_not_ui_thread();
        let mut file = File::open(self.data_path())?;
        file.seek(SeekFrom::Start(entry.start))?;
        let mut bytes = vec![0; (entry.end - entry.start) as usize];
        file.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    fn payload(&self, entry: &Entry) -> Result<Vec<u8>> {
        Ok(self.raw(entry)?[HEADER..].to_vec())
    }
    fn byte_end_at(&self, seq: u64) -> Result<u64> {
        if seq == self.first_base() {
            return Ok(self
                .boundary
                .origin_byte_start
                .unwrap_or(self.boundary.byte_start));
        }
        if seq > self.boundary.base {
            let e = self
                .entries
                .iter()
                .find(|e| e.kind != 4 && e.sequence == seq)
                .ok_or_else(|| invalid("checkpoint cursor missing"))?;
            return Ok(e.byte_start
                + if e.kind == 1 {
                    e.end - e.start - HEADER as u64
                } else {
                    0
                });
        }
        let segment = self
            .sealed
            .iter()
            .find(|s| seq > s.base && seq <= s.sequence)
            .ok_or_else(|| invalid("checkpoint cold cursor missing"))?;
        self.hydrate(segment)?;
        let cached = self.hydrated.borrow();
        let cached = cached.as_ref().unwrap();
        let e = cached
            .entries
            .iter()
            .find(|e| e.kind != 4 && e.sequence == seq)
            .ok_or_else(|| invalid("sealed checkpoint cursor missing"))?;
        Ok(e.byte_start
            + if e.kind == 1 {
                e.end - e.start - HEADER as u64
            } else {
                0
            })
    }
    fn rebuild_index(&mut self) -> Result<()> {
        self.index.clear();
        if let Some(bytes) = self.latest_checkpoint()? {
            for point in Saved::decode(&bytes)?.index {
                self.index.insert(point.sequence, point);
            }
        }
        for entry in self.entries.iter().filter(|e| e.kind == 4) {
            for point in Saved::decode(&self.payload(entry)?)?.index {
                if point.sequence > self.first_base() {
                    self.index.insert(point.sequence, point);
                }
            }
        }
        Ok(())
    }
    pub fn history(&self, from: u64, to: u64) -> Result<History> {
        if from > to {
            return Err(invalid("invalid history range"));
        }
        let first = self.first_base() + 1;
        let last = self.boundary.sequence;
        let gap = (from < first || to > last).then(|| HistoryGap {
            requested_from: from,
            requested_to: to,
            retained_from: (first <= last).then_some(first),
            retained_to: (first <= last).then_some(last),
            reason: "requested capture range outside committed hot+cold history".into(),
        });
        let mut records = Vec::new();
        let mut size = 0;
        let mut next = None;
        for segment in self
            .sealed
            .iter()
            .filter(|s| s.sequence >= from && s.base < to)
        {
            self.hydrate(segment)?;
            let cache = self.hydrated.borrow();
            let cache = cache.as_ref().unwrap();
            for entry in cache
                .entries
                .iter()
                .filter(|e| e.kind != 4 && e.sequence >= from && e.sequence <= to)
            {
                let payload = &cache.bytes[entry.start as usize + HEADER..entry.end as usize];
                let record = IndexedEvent {
                    event: decode_event(
                        entry.kind,
                        self.boundary.seat,
                        self.boundary.generation,
                        entry.sequence,
                        payload,
                    )?,
                    row: self
                        .index
                        .get(&entry.sequence)
                        .or_else(|| cache.rows.get(&entry.sequence))
                        .cloned(),
                };
                let bytes = serde_json::to_vec(&record)?.len();
                if !records.is_empty() && size + bytes > PAGE_BYTES {
                    next = Some(entry.sequence);
                    break;
                }
                records.push(record);
                size += bytes;
            }
            if next.is_some() {
                break;
            }
        }
        if next.is_none() {
            for entry in self
                .entries
                .iter()
                .filter(|e| e.kind != 4 && e.sequence >= from && e.sequence <= to)
            {
                let record = IndexedEvent {
                    event: decode_event(
                        entry.kind,
                        self.boundary.seat,
                        self.boundary.generation,
                        entry.sequence,
                        &self.payload(entry)?,
                    )?,
                    row: self.index.get(&entry.sequence).cloned(),
                };
                let bytes = serde_json::to_vec(&record)?.len();
                if !records.is_empty() && size + bytes > PAGE_BYTES {
                    next = Some(entry.sequence);
                    break;
                }
                records.push(record);
                size += bytes;
            }
        }
        Ok(History {
            status: self.status(),
            records,
            gap,
            next,
        })
    }
    pub fn replay(&self) -> Result<Playback> {
        let bytes = self
            .latest_checkpoint()?
            .ok_or_else(|| invalid("ring has no committed checkpoint"))?;
        let mut playback = Playback::restore(Saved::decode(&bytes)?);
        let mut from = playback.sequence + 1;
        while from <= self.boundary.sequence {
            let page = self.history(from, self.boundary.sequence)?;
            if page.gap.is_some() || page.records.is_empty() {
                return Err(invalid("ring replay gap"));
            }
            for record in page.records {
                playback.apply(&record.event)?;
            }
            from = playback.sequence + 1;
        }
        Ok(playback)
    }
}

fn event_generation(event: &Event) -> u64 {
    match event {
        Event::Output { generation, .. }
        | Event::Resized { generation, .. }
        | Event::Exited { generation, .. } => *generation,
    }
}
fn encode_event(event: &Event) -> Result<(u16, Vec<u8>)> {
    Ok(match event {
        Event::Output { bytes, .. } => (1, bytes.clone()),
        Event::Resized { size, .. } => (2, serde_json::to_vec(size)?),
        Event::Exited { status, .. } => (3, serde_json::to_vec(status)?),
    })
}
fn decode_event(kind: u16, seat: Ulid, generation: u64, seq: u64, payload: &[u8]) -> Result<Event> {
    Ok(match kind {
        1 => Event::Output {
            seat,
            generation,
            seq,
            bytes: payload.to_vec(),
        },
        2 => Event::Resized {
            seat,
            generation,
            seq,
            size: serde_json::from_slice::<SeatSize>(payload)?,
        },
        3 => Event::Exited {
            seat,
            generation,
            seq,
            status: serde_json::from_slice::<ExitInfo>(payload)?,
        },
        _ => return Err(invalid("unknown capture kind")),
    })
}
fn encode(
    kind: u16,
    seat: Ulid,
    generation: u64,
    seq: u64,
    byte_start: u64,
    payload: &[u8],
) -> Result<Vec<u8>> {
    if payload.len() > MAX_RECORD {
        return Err(invalid("record length exceeds bound"));
    }
    let mut bytes = Vec::with_capacity(HEADER + payload.len());
    bytes.extend(b"BBR1");
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(kind.to_le_bytes());
    bytes.extend((payload.len() as u32).to_le_bytes());
    bytes.extend(seq.to_le_bytes());
    bytes.extend(generation.to_le_bytes());
    bytes.extend(seat.to_bytes());
    bytes.extend(byte_start.to_le_bytes());
    bytes.extend([0; 4]);
    bytes.extend(payload);
    let crc = crc32c_parts(&bytes[..HEADER - 4], payload);
    bytes[HEADER - 4..HEADER].copy_from_slice(&crc.to_le_bytes());
    Ok(bytes)
}
fn scan(root: &Path, b: &Boundary) -> Result<Vec<Entry>> {
    if b.version != 1
        || b.length > b.settings.capacity_bytes
        || !b.file.starts_with("data-")
        || b.file.contains('/')
        || b.base > b.sequence
    {
        return Err(invalid("invalid committed boundary"));
    }
    let mut file = File::open(root.join(&b.file))?;
    if file.metadata()?.len() < b.length {
        return Err(invalid("torn committed suffix"));
    }
    scan_reader(&mut file, b)
}
fn scan_reader(file: &mut impl Read, b: &Boundary) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
    let (mut offset, mut seq, mut bytes) = (0, b.base, b.byte_start);
    while offset < b.length {
        let mut h = [0; HEADER];
        file.read_exact(&mut h)?;
        let kind = u16::from_le_bytes(h[6..8].try_into()?);
        let len = u32::from_le_bytes(h[8..12].try_into()?) as usize;
        let cursor = u64::from_le_bytes(h[12..20].try_into()?);
        let generation = u64::from_le_bytes(h[20..28].try_into()?);
        let start = u64::from_le_bytes(h[44..52].try_into()?);
        if &h[..4] != b"BBR1"
            || u16::from_le_bytes(h[4..6].try_into()?) != 1
            || !(1..=4).contains(&kind)
            || len > MAX_RECORD
            || offset + HEADER as u64 + len as u64 > b.length
            || generation != b.generation
            || h[28..44] != b.seat.to_bytes()
            || (kind != 4 && (cursor != seq + 1 || start != bytes))
            || (kind == 4 && cursor > seq)
        {
            return Err(invalid("invalid ring header/sequence/generation/length"));
        }
        let mut payload = vec![0; len];
        file.read_exact(&mut payload)?;
        if u32::from_le_bytes(h[52..56].try_into()?) != crc32c_parts(&h[..52], &payload) {
            return Err(invalid("ring CRC32C mismatch"));
        }
        if kind == 4 {
            let saved = Saved::decode(&payload)?;
            if saved.sequence != cursor
                || saved.byte_end != start
                || saved.seat != b.seat
                || saved.generation != generation
            {
                return Err(invalid("checkpoint record mismatch"));
            }
        } else {
            decode_event(kind, b.seat, generation, cursor, &payload)?;
            seq = cursor;
            if kind == 1 {
                bytes += len as u64;
            }
        }
        let end = offset + HEADER as u64 + len as u64;
        entries.push(Entry {
            start: offset,
            end,
            kind,
            sequence: cursor,
            byte_start: start,
        });
        offset = end;
    }
    if seq != b.sequence || bytes != b.byte_end {
        return Err(invalid("committed tail mismatch"));
    }
    Ok(entries)
}
fn load_boundary(path: &Path) -> Result<Boundary> {
    let mut bytes = Vec::new();
    File::open(path)?.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() < 4 || bytes.len() > 4096 {
        return Err(invalid("invalid boundary size"));
    }
    let end = bytes.len() - 4;
    if crc32c(&bytes[..end]) != u32::from_le_bytes(bytes[end..].try_into()?) {
        return Err(invalid("boundary CRC32C mismatch"));
    }
    Ok(serde_json::from_slice(&bytes[..end])?)
}
fn crc32c_parts(header: &[u8], payload: &[u8]) -> u32 {
    let mut crc = !0u32;
    for byte in header.iter().chain(payload) {
        crc = CRC32C[((crc ^ *byte as u32) & 255) as usize] ^ (crc >> 8);
    }
    !crc
}
const CRC32C: [u32; 256] = {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut bit = 0;
        while bit < 8 {
            crc = (crc >> 1) ^ (0x82f63b78 & (0u32.wrapping_sub(crc & 1)));
            bit += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
};
pub fn crc32c(bytes: &[u8]) -> u32 {
    crc32c_parts(bytes, &[])
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryGap {
    pub requested_from: u64,
    pub requested_to: u64,
    pub retained_from: Option<u64>,
    pub retained_to: Option<u64>,
    pub reason: String,
}
