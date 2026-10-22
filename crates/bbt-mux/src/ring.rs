//! Supervisor-owned, CRC32C framed capture with atomic committed boundaries.
//! Logical hard-line indexes are produced by logic, never guessed by capture.

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use bbt_core::{Checkpoint, Term, vte};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::protocol::{Event, ExitInfo, OUTPUT_CHUNK_BYTES, SeatSize};
use crate::store::{Blobs, Result, blobs::LocalBlobs, invalid};
use crate::streams::HistoryGap;

const HEADER: usize = 56;
const MAX_RECORD: usize = 1024 * 1024;
const MAX_CHECKPOINT: usize = 128 * 1024;
const MAX_DECODED: u64 = 16 * 1024 * 1024;
const MAX_POINTS: usize = 4096;
const PAGE_BYTES: usize = 64 * 1024;

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
    pub byte_start: u64,
    pub byte_end: u64,
    pub hot_bytes: u64,
    pub paused: bool,
    pub recovery_notice: Option<String>,
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
                                boundary,
                                entries,
                                index: Default::default(),
                                notice,
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
                            ring.rebuild_index()?;
                            ring.retire_orphans()?;
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
        };
        let ring = Self {
            root: root.into(),
            boundary,
            entries: Vec::new(),
            index: Default::default(),
            notice: None,
        };
        File::create(ring.data_path())?.sync_all()?;
        ring.publish_boundary(&ring.boundary)?;
        Ok(ring)
    }
    fn data_path(&self) -> PathBuf {
        self.root.join(&self.boundary.file)
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
            first_sequence: self.boundary.base + 1,
            committed_sequence: self.boundary.sequence,
            byte_start: self.boundary.byte_start,
            byte_end: self.boundary.byte_end,
            hot_bytes: self.boundary.length,
            paused: !self.can_capture(),
            recovery_notice: self.notice.clone(),
        }
    }
    pub fn can_capture(&self) -> bool {
        self.capture_capacity() > 0
    }
    pub fn capture_capacity(&self) -> usize {
        if !self.entries.iter().any(|e| e.kind == 4) {
            return 0;
        }
        self.boundary
            .settings
            .capacity_bytes
            .saturating_sub(self.boundary.length + HEADER as u64 + self.checkpoint_reserve())
            .min(OUTPUT_CHUNK_BYTES as u64) as usize
    }
    pub fn can_control(&self) -> bool {
        self.boundary.length + 512 <= self.boundary.settings.capacity_bytes
    }
    fn checkpoint_reserve(&self) -> u64 {
        (MAX_CHECKPOINT as u64 + HEADER as u64 + 512).min(self.boundary.settings.capacity_bytes / 4)
    }
    pub fn prepare_capture(&mut self) -> Result<()> {
        let reserve = OUTPUT_CHUNK_BYTES as u64 + HEADER as u64 + self.checkpoint_reserve();
        if self.boundary.length + reserve > self.boundary.settings.capacity_bytes {
            self.compact_for(
                &[],
                0,
                self.boundary.sequence,
                self.boundary.byte_end,
                reserve,
            )?;
        }
        Ok(())
    }
    pub fn latest_checkpoint(&self) -> Result<Option<Vec<u8>>> {
        self.entries
            .iter()
            .rev()
            .find(|e| e.kind == 4)
            .map(|e| self.payload(e))
            .transpose()
    }
    pub fn checkpoint(&mut self, bytes: Vec<u8>) -> Result<()> {
        let saved = Saved::decode(&bytes)?;
        if saved.seat != self.boundary.seat
            || saved.generation != self.boundary.generation
            || saved.sequence > self.boundary.sequence
            || saved.sequence < self.boundary.base
            || saved.byte_end != self.byte_end_at(saved.sequence)?
        {
            return Err(invalid("checkpoint does not cover a captured cursor"));
        }
        if let Some(old) = self.entries.iter().rev().find(|e| e.kind == 4) {
            if saved.sequence < old.sequence {
                return Err(invalid("stale checkpoint"));
            }
            if saved.sequence == old.sequence {
                if self.payload(old)? == bytes {
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
        let bytes = encode(
            kind,
            self.boundary.seat,
            self.boundary.generation,
            sequence,
            byte_start,
            payload,
        )?;
        if bytes.len() as u64 > self.boundary.settings.capacity_bytes {
            return Err(invalid("record exceeds ring capacity"));
        }
        if self.boundary.length + bytes.len() as u64 > self.boundary.settings.capacity_bytes {
            if !self.compact_for(&bytes, kind, sequence, byte_start, 0)? {
                return Err(invalid("ring full: awaiting a newer committed checkpoint"));
            }
            return Ok(());
        }
        let start = self.boundary.length;
        let mut file = OpenOptions::new().append(true).open(self.data_path())?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        let mut next = self.boundary.clone();
        next.length += bytes.len() as u64;
        if kind != 4 {
            next.sequence = sequence;
        }
        if kind == 1 {
            next.byte_end += payload.len() as u64;
        }
        self.publish_boundary(&next)?;
        self.boundary = next;
        self.entries.push(Entry {
            start,
            end: self.boundary.length,
            kind,
            sequence,
            byte_start,
        });
        self.retire_orphans()?;
        Ok(())
    }
    fn compact_for(
        &mut self,
        new: &[u8],
        kind: u16,
        sequence: u64,
        byte_start: u64,
        reserve: u64,
    ) -> Result<bool> {
        let mut records = self
            .entries
            .iter()
            .map(|e| Ok((e.clone(), self.raw(e)?)))
            .collect::<Result<Vec<_>>>()?;
        if !new.is_empty() {
            records.push((
                Entry {
                    start: 0,
                    end: new.len() as u64,
                    kind,
                    sequence,
                    byte_start,
                },
                new.to_vec(),
            ));
        }
        let checkpoints: Vec<_> = records
            .iter()
            .filter(|(e, _)| e.kind == 4)
            .map(|(e, _)| e.sequence)
            .collect();
        if checkpoints.len() < 2 {
            return Ok(false);
        }
        for &base in &checkpoints[..checkpoints.len() - 1] {
            let mut selected = Vec::new();
            let Some(first) = records
                .iter()
                .find(|(e, _)| e.kind == 4 && e.sequence == base)
            else {
                continue;
            };
            selected.push(first.clone());
            selected.extend(
                records
                    .iter()
                    .filter(|(e, _)| {
                        (e.kind != 4 && e.sequence > base) || (e.kind == 4 && e.sequence > base)
                    })
                    .cloned(),
            );
            let size: usize = selected.iter().map(|(_, raw)| raw.len()).sum();
            if size as u64 + reserve > self.boundary.settings.capacity_bytes {
                continue;
            }
            let saved = Saved::decode(&selected[0].1[HEADER..])?;
            let mut next = self.boundary.clone();
            next.file = format!("data-{}.log", Ulid::new());
            next.base = base;
            next.byte_start = saved.byte_end;
            next.length = size as u64;
            if kind != 4 {
                next.sequence = sequence;
            }
            if kind == 1 {
                next.byte_end += (new.len() - HEADER) as u64;
            }
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(self.root.join(&next.file))?;
            for (_, raw) in &selected {
                file.write_all(raw)?;
            }
            file.sync_all()?;
            self.publish_boundary(&next)?;
            self.boundary = next;
            self.entries = scan(&self.root, &self.boundary)?;
            self.rebuild_index()?;
            self.retire_orphans()?;
            return Ok(true);
        }
        Ok(false)
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
        if self.root.join("committed").exists() {
            std::fs::rename(self.root.join("committed"), self.root.join("previous"))?;
        }
        std::fs::rename(temp, self.root.join("committed"))?;
        File::open(&self.root)?.sync_all()?;
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
                self.archive(&name)?;
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
        if seq == self.boundary.base {
            return Ok(self.boundary.byte_start);
        }
        let entry = self
            .entries
            .iter()
            .find(|e| e.kind != 4 && e.sequence == seq)
            .ok_or_else(|| invalid("checkpoint cursor not retained"))?;
        Ok(entry.byte_start
            + if entry.kind == 1 {
                entry.end - entry.start - HEADER as u64
            } else {
                0
            })
    }
    fn rebuild_index(&mut self) -> Result<()> {
        self.index.clear();
        for entry in self.entries.iter().filter(|e| e.kind == 4) {
            for point in Saved::decode(&self.payload(entry)?)?.index {
                if point.sequence > self.boundary.base {
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
        let first = self.boundary.base + 1;
        let last = self.boundary.sequence;
        let gap = (from < first || to > last).then(||HistoryGap { requested_from: from,requested_to:to,
            retained_from: (first<=last).then_some(first),retained_to:(first<=last).then_some(last),
            reason:"requested capture range outside the hot checkpoint window (older bytes retained in cold blobs)".into() });
        let mut records = Vec::new();
        let mut bytes = 0;
        let mut next = None;
        for entry in self
            .entries
            .iter()
            .filter(|e| e.kind != 4 && e.sequence >= from && e.sequence <= to)
        {
            let event = decode_event(
                entry.kind,
                self.boundary.seat,
                self.boundary.generation,
                entry.sequence,
                &self.payload(entry)?,
            )?;
            let record = IndexedEvent {
                event,
                row: self.index.get(&entry.sequence).cloned(),
            };
            let size = serde_json::to_vec(&record)?.len();
            if !records.is_empty() && bytes + size > PAGE_BYTES {
                next = Some(entry.sequence);
                break;
            }
            records.push(record);
            bytes += size;
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
            || (kind == 4 && (cursor < b.base || cursor > seq))
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
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0x82f63b78 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}
pub fn crc32c(bytes: &[u8]) -> u32 {
    crc32c_parts(bytes, &[])
}
