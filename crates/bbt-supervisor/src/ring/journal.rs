//! Supervisor-wide write-ahead capture. One CRC'd commit frame and one barrier
//! publish a tick; per-seat files are rebuildable projections, never tick markers.
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{FileExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use super::{HEADER, MAX_RECORD, Ring, Saved, crc32c_parts, decode_event, encode, invalid};
use crate::durability::{sync_directory, sync_file};
use crate::store::Result;

type Key = (Ulid, u64);
type Coverage = HashMap<Key, (Position, u64)>;
const COMMIT: u16 = 0xffff;
const CHECKPOINT_PREFIX: usize = 28;
const CHECKPOINT_MAGIC: &[u8; 4] = b"JCP2";
const ROTATE: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct Position {
    pub file: String,
    pub offset: u64,
}
#[derive(Clone)]
pub(super) struct Record {
    pub position: Position,
    pub start: u64,
    pub kind: u16,
    pub key: Key,
    pub sequence: u64,
    pub byte_start: u64,
    pub length: usize,
}
pub(super) struct Journal {
    root: PathBuf,
    floor: Position,
    file: File,
    name: String,
    length: u64,
    committed: u64,
    tick: u64,
    dirty: bool,
    buffer: Vec<u8>,
    last_full: std::time::Instant,
    files: HashMap<String, File>,
    required: Coverage,
    retired: Vec<(String, Coverage)>,
    projected: Coverage,
}
impl Journal {
    pub fn open(root: &Path) -> Result<Self> {
        if !root.is_absolute() {
            return Err(invalid("journal directory must be absolute"));
        }
        std::fs::create_dir_all(root)?;
        let meta = std::fs::symlink_metadata(root)?;
        // SAFETY: geteuid reads the process identity and changes no state.
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } {
            return Err(invalid("journal directory must be owned and not a symlink"));
        }
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))?;
        let existing = journal_files(root)?;
        let previous = existing
            .last()
            .and_then(|p| p.file_name())
            .and_then(|p| p.to_str());
        let name = next_name(previous)?;
        let floor = Position {
            file: existing
                .first()
                .unwrap_or(&root.join(&name))
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            offset: 0,
        };
        let file = OpenOptions::new()
            .read(true)
            .append(true)
            .create_new(true)
            .open(root.join(&name))?;
        sync_directory(&File::open(root)?)?;
        sync_file(&file, true)?;
        Ok(Self {
            root: root.into(),
            floor,
            file,
            name,
            length: 0,
            committed: 0,
            tick: 0,
            dirty: false,
            buffer: Vec::with_capacity(1024 * 1024),
            last_full: std::time::Instant::now(),
            files: HashMap::new(),
            required: HashMap::new(),
            retired: Vec::new(),
            projected: HashMap::new(),
        })
    }
    pub fn bind(&self, ring: &Ring) -> Result<()> {
        let temp = ring.root.join(format!(".journal-{}", Ulid::new()));
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        serde_json::to_writer(&mut file, &self.root)?;
        sync_file(&file, true)?;
        std::fs::rename(temp, ring.root.join("journal.json"))?;
        sync_directory(&File::open(&ring.root)?)?;
        Ok(())
    }
    pub fn append(
        &mut self,
        key: Key,
        kind: u16,
        sequence: u64,
        byte_start: u64,
        payload: &[u8],
    ) -> Result<Record> {
        if kind == 4 {
            return Err(invalid("checkpoint requires a coverage cursor"));
        }
        self.append_at(key, kind, sequence, byte_start, payload, None)
    }
    pub fn floor(&self) -> Position {
        self.floor.clone()
    }
    pub fn checkpoint(
        &mut self,
        key: Key,
        sequence: u64,
        byte_end: u64,
        payload: &[u8],
        covered: &Position,
    ) -> Result<Record> {
        self.append_at(key, 4, sequence, byte_end, payload, Some(covered))
    }
    fn append_at(
        &mut self,
        key: Key,
        kind: u16,
        sequence: u64,
        byte_start: u64,
        payload: &[u8],
        covered: Option<&Position>,
    ) -> Result<Record> {
        let checkpoint;
        let payload = if let Some(covered) = covered {
            let id = Ulid::from_string(&covered.file[8..covered.file.len() - 4])?;
            checkpoint = [
                CHECKPOINT_MAGIC.as_slice(),
                id.to_bytes().as_slice(),
                covered.offset.to_le_bytes().as_slice(),
                payload,
            ]
            .concat();
            checkpoint.as_slice()
        } else {
            payload
        };
        let bytes = encode(kind, key.0, key.1, sequence, byte_start, payload)?;
        let start = self.length;
        self.buffer_frame(&bytes)?;
        self.length += bytes.len() as u64;
        self.dirty = true;
        let position = Position {
            file: self.name.clone(),
            offset: self.length,
        };
        let max_sequence = self
            .required
            .get(&key)
            .map_or(sequence, |(_, seq)| (*seq).max(sequence));
        self.required.insert(key, (position.clone(), max_sequence));
        Ok(Record {
            position,
            start,
            kind,
            key,
            sequence,
            byte_start,
            length: payload.len(),
        })
    }
    fn buffer_frame(&mut self, bytes: &[u8]) -> Result<()> {
        if self.buffer.len() + bytes.len() > 1024 * 1024 {
            self.flush_buffer()?;
        }
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }
    fn flush_buffer(&mut self) -> Result<()> {
        if !self.buffer.is_empty() {
            self.file.write_all(&self.buffer)?;
            self.buffer.clear();
        }
        Ok(())
    }
    pub fn durable_due(&self) -> bool {
        self.dirty && self.last_full.elapsed() >= std::time::Duration::from_secs(5)
    }
    pub fn payload(&mut self, record: &Record) -> Result<Vec<u8>> {
        self.flush_buffer()?;
        let file = if record.position.file == self.name {
            &self.file
        } else {
            if !self.files.contains_key(&record.position.file) {
                self.files.insert(
                    record.position.file.clone(),
                    File::open(self.root.join(&record.position.file))?,
                );
            }
            &self.files[&record.position.file]
        };
        let mut bytes = vec![0; record.length];
        file.read_exact_at(&mut bytes, record.start + HEADER as u64)?;
        if record.kind == 4 {
            Ok(checkpoint_payload(&bytes)?.to_vec())
        } else {
            Ok(bytes)
        }
    }
    /// Returns whether this round submitted one ordering/full barrier.
    pub fn commit(&mut self, strong: bool) -> Result<bool> {
        if !self.dirty || (!strong && self.length == self.committed) {
            return Ok(false);
        }
        if self.length != self.committed {
            self.tick += 1;
            let frame = encode(
                COMMIT,
                Ulid::nil(),
                0,
                self.tick,
                self.length,
                &self.length.to_le_bytes(),
            )?;
            self.buffer_frame(&frame)?;
            self.length += frame.len() as u64;
        }
        self.flush_buffer()?;
        sync_file(&self.file, strong)?;
        self.committed = self.length;
        if strong {
            self.dirty = false;
            self.last_full = std::time::Instant::now();
        }
        Ok(true)
    }
    pub fn rotate_due(&self) -> bool {
        self.length >= ROTATE
    }
    pub fn rotate(&mut self) -> Result<()> {
        self.commit(true)?;
        self.files.insert(self.name.clone(), self.file.try_clone()?);
        self.retired
            .push((self.name.clone(), std::mem::take(&mut self.required)));
        self.name = next_name(Some(&self.name))?;
        self.file = OpenOptions::new()
            .read(true)
            .append(true)
            .create_new(true)
            .open(self.root.join(&self.name))?;
        sync_directory(&File::open(&self.root)?)?;
        sync_file(&self.file, true)?;
        self.length = 0;
        self.committed = 0;
        self.tick = 0;
        Ok(())
    }
    /// Only strong sealed projections may retire old journal files. Each seat
    /// must cover both the last record's offset and its parser checkpoint.
    pub fn projected(&mut self, key: Key, position: Position, checkpoint: u64) -> Result<()> {
        self.projected.insert(key, (position, checkpoint));
        let mut keep = Vec::new();
        let mut removed = false;
        for (name, required) in self.retired.drain(..) {
            let safe = required.iter().all(|(key, (end, seq))| {
                self.projected.get(key).is_some_and(|(p, cp)| {
                    *cp >= *seq
                        && (p.file > end.file || (p.file == end.file && p.offset >= end.offset))
                })
            });
            if safe {
                self.files.remove(&name);
                std::fs::remove_file(self.root.join(&name))?;
                removed = true;
            } else {
                keep.push((name, required));
            }
        }
        self.retired = keep;
        if removed {
            sync_directory(&File::open(&self.root)?)?;
        }
        Ok(())
    }
}

fn checkpoint_payload(bytes: &[u8]) -> Result<&[u8]> {
    let prefix = if bytes.starts_with(CHECKPOINT_MAGIC) {
        CHECKPOINT_PREFIX
    } else {
        8
    };
    bytes
        .get(prefix..)
        .ok_or_else(|| invalid("truncated journal checkpoint prefix"))
}

fn next_name(previous: Option<&str>) -> Result<String> {
    let mut id = Ulid::new();
    if let Some(previous) = previous {
        let prior = Ulid::from_string(&previous[8..previous.len() - 4])?;
        if id <= prior {
            id = Ulid::from(
                u128::from(prior)
                    .checked_add(1)
                    .ok_or_else(|| invalid("journal id exhausted"))?,
            );
        }
    }
    Ok(format!("journal-{id}.log"))
}

pub(super) fn journal_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with("journal-")
            && name.ends_with(".log")
            && Ulid::from_string(&name[8..name.len() - 4]).is_ok()
        {
            if !entry.file_type()?.is_file() {
                return Err(invalid("journal segment must be a regular file"));
            }
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}
/// Scan bounded frames, retain only whole ticks. A corrupt/torn suffix cannot
/// advance the boundary even when later bytes happen to resemble a commit.
pub(super) fn scan(path: &Path) -> Result<(Vec<Record>, u64, bool)> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let mut records = Vec::new();
    let (mut offset, mut committed, mut count, mut tick) = (0, 0, 0, 0);
    while offset < length {
        let mut h = [0; HEADER];
        if file.read_exact(&mut h).is_err() {
            break;
        }
        let kind = u16::from_le_bytes(h[6..8].try_into()?);
        let len = u32::from_le_bytes(h[8..12].try_into()?) as usize;
        let seq = u64::from_le_bytes(h[12..20].try_into()?);
        let generation = u64::from_le_bytes(h[20..28].try_into()?);
        let seat = Ulid::from_bytes(h[28..44].try_into()?);
        let byte_start = u64::from_le_bytes(h[44..52].try_into()?);
        if &h[..4] != b"BBR1"
            || u16::from_le_bytes(h[4..6].try_into()?) != 1
            || len > MAX_RECORD
            || offset + HEADER as u64 + len as u64 > length
        {
            break;
        }
        let mut payload = vec![0; len];
        file.read_exact(&mut payload)?;
        if u32::from_le_bytes(h[52..56].try_into()?) != crc32c_parts(&h[..52], &payload) {
            break;
        }
        let end = offset + HEADER as u64 + len as u64;
        if kind == COMMIT {
            if seat != Ulid::nil()
                || generation != 0
                || seq != tick + 1
                || payload != offset.to_le_bytes()
                || byte_start != offset
            {
                break;
            }
            tick = seq;
            committed = end;
            count = records.len();
        } else {
            if seat == Ulid::nil() || generation == 0 || !(1..=4).contains(&kind) {
                break;
            }
            if kind == 4 {
                if payload.starts_with(CHECKPOINT_MAGIC) {
                    if len < CHECKPOINT_PREFIX {
                        break;
                    }
                    let covered_file = format!(
                        "journal-{}.log",
                        Ulid::from_bytes(payload[4..20].try_into()?)
                    );
                    let covered_offset = u64::from_le_bytes(payload[20..28].try_into()?);
                    if covered_file > name || (covered_file == name && covered_offset > offset) {
                        break;
                    }
                } else if len < 8 || u64::from_le_bytes(payload[..8].try_into()?) != offset {
                    break;
                }
                let saved = Saved::decode(checkpoint_payload(&payload)?)?;
                if saved.seat != seat
                    || saved.generation != generation
                    || saved.sequence != seq
                    || saved.byte_end != byte_start
                {
                    break;
                }
            } else if decode_event(kind, seat, generation, seq, &payload).is_err() {
                break;
            }
            records.push(Record {
                position: Position {
                    file: name.clone(),
                    offset: end,
                },
                start: offset,
                kind,
                key: (seat, generation),
                sequence: seq,
                byte_start,
                length: len,
            });
        }
        offset = end;
    }
    records.truncate(count);
    Ok((records, committed, committed != length))
}

/// Called once when reopening a derived ring. The ring's cursor is the last
/// compacted journal position, so an already retained record is never reapplied.
pub(super) fn recover(ring: &mut Ring) -> Result<()> {
    let link = ring.root.join("journal.json");
    if !link.exists() {
        return Ok(());
    }
    let mut bytes = Vec::new();
    File::open(link)?.take(4097).read_to_end(&mut bytes)?;
    if bytes.len() > 4096 {
        return Err(invalid("oversized journal pointer"));
    }
    let root: PathBuf = serde_json::from_slice(&bytes)?;
    if !root.is_absolute() {
        return Err(invalid("invalid journal pointer"));
    }
    let key = (ring.boundary.seat, ring.boundary.generation);
    let cursor = ring.boundary.journal_position.clone();
    let mut changed = false;
    ring.grouped = true;
    for path in journal_files(&root)? {
        let name = path.file_name().unwrap().to_string_lossy();
        if cursor
            .as_ref()
            .is_some_and(|p| name.as_ref() < p.file.as_str())
        {
            continue;
        }
        let (records, _, torn) = scan(&path)?;
        if torn {
            ring.notice = Some("rejected uncommitted/torn journal suffix".into());
        }
        let mut file = File::open(&path)?;
        for r in records.into_iter().filter(|r| r.key == key) {
            if cursor
                .as_ref()
                .is_some_and(|p| p.file == r.position.file && r.position.offset <= p.offset)
            {
                continue;
            }
            file.seek(SeekFrom::Start(r.start + HEADER as u64))?;
            let mut bytes = vec![0; r.length];
            file.read_exact(&mut bytes)?;
            if r.kind == 4 {
                let latest = ring
                    .latest_checkpoint()?
                    .map(|b| Saved::decode(&b).map(|s| s.sequence))
                    .transpose()?;
                if latest.is_none_or(|seq| r.sequence >= seq) {
                    ring.checkpoint(checkpoint_payload(&bytes)?.to_vec())?;
                }
            } else if r.sequence > ring.boundary.sequence {
                if r.byte_start != ring.boundary.byte_end {
                    return Err(invalid("journal byte cursor discontinuity"));
                }
                ring.append_event(&decode_event(r.kind, key.0, key.1, r.sequence, &bytes)?)?;
            }
            ring.boundary.journal_position = Some(r.position);
            changed = true;
        }
    }
    if changed {
        ring.flush()?;
    }
    ring.grouped = false;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Event;
    use crate::ring::{Playback, Settings};

    #[test]
    fn crash_rebuilds_two_seats_only_through_the_last_tick_and_rejects_torn_suffix() {
        let temp = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(&temp.path().join("journal")).unwrap();
        let seats = [Ulid::new(), Ulid::new()];
        for (i, seat) in seats.iter().enumerate() {
            let ring = Ring::open(
                &temp.path().join(i.to_string()),
                *seat,
                1,
                0,
                Settings::default(),
            )
            .unwrap();
            journal.bind(&ring).unwrap();
            journal
                .append((*seat, 1), 1, 1, 0, &[b'A' + i as u8; 64])
                .unwrap();
        }
        journal.commit(false).unwrap();
        let path = journal.root.join(&journal.name);
        let boundary = journal.committed;
        journal
            .append((seats[0], 1), 1, 2, 64, b"not committed")
            .unwrap();
        journal.flush_buffer().unwrap();
        journal.file.write_all(b"BBR1\x01").unwrap();
        let (records, end, torn) = scan(&path).unwrap();
        assert_eq!(end, boundary);
        assert!(torn);
        assert_eq!(records.len(), 2);
        drop(journal);
        for (i, seat) in seats.iter().enumerate() {
            let ring = Ring::open(
                &temp.path().join(i.to_string()),
                *seat,
                1,
                0,
                Settings::default(),
            )
            .unwrap();
            assert_eq!(ring.status().committed_sequence, 1);
            assert!(
                ring.status()
                    .recovery_notice
                    .unwrap()
                    .contains("journal suffix")
            );
            let page = ring.history(1, 1).unwrap();
            assert_eq!(page.records.len(), 1);
            assert!(
                matches!(&page.records[0].event,Event::Output {bytes,..} if bytes==&vec![b'A'+i as u8;64])
            );
        }
    }
    #[test]
    fn old_segment_waits_for_every_sealed_projection_and_checkpoint() {
        let temp = tempfile::tempdir().unwrap();
        let mut j = Journal::open(temp.path()).unwrap();
        let a = (Ulid::new(), 1);
        let b = (Ulid::new(), 1);
        let ra = j.append(a, 1, 1, 0, b"a").unwrap();
        let rb = j.append(b, 1, 1, 0, b"b").unwrap();
        let old = j.root.join(&j.name);
        j.rotate().unwrap();
        j.projected(a, ra.position, 1).unwrap();
        assert!(old.exists());
        j.projected(b, rb.position.clone(), 0).unwrap();
        assert!(old.exists());
        j.projected(b, rb.position, 1).unwrap();
        assert!(!old.exists());
    }
    #[test]
    fn metadata_only_debt_keeps_the_five_second_full_flush_deadline() {
        let temp = tempfile::tempdir().unwrap();
        let mut j = Journal::open(temp.path()).unwrap();
        let size = serde_json::to_vec(&crate::SeatSize::default()).unwrap();
        j.append((Ulid::new(), 1), 2, 1, 0, &size).unwrap();
        j.commit(false).unwrap();
        j.last_full = std::time::Instant::now() - std::time::Duration::from_secs(6);
        assert!(j.durable_due());
        j.commit(true).unwrap();
        assert!(!j.durable_due());
    }
    #[test]
    fn legacy_checkpoint_publication_offsets_remain_readable() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("seat");
        let seat = Ulid::new();
        let ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        let mut j = Journal::open(&temp.path().join("journal")).unwrap();
        j.bind(&ring).unwrap();
        let mut state = Playback::new(seat, 1, crate::SeatSize::default());
        let e = Event::Output {
            seat,
            generation: 1,
            seq: 1,
            bytes: b"legacy".to_vec(),
        };
        state.apply(&e).unwrap();
        j.append((seat, 1), 1, 1, 0, b"legacy").unwrap();
        let legacy = [
            j.length.to_le_bytes().as_slice(),
            state.saved().encode().unwrap().as_slice(),
        ]
        .concat();
        j.append_at((seat, 1), 4, 1, state.byte_end, &legacy, None)
            .unwrap();
        j.commit(true).unwrap();
        assert_eq!(
            Ring::open(&root, seat, 1, 0, Settings::default())
                .unwrap()
                .replay()
                .unwrap()
                .byte_end,
            state.byte_end
        );
    }
    #[test]
    fn bad_crc_and_oversized_header_never_commit_a_later_tick() {
        let temp = tempfile::tempdir().unwrap();
        let mut j = Journal::open(temp.path()).unwrap();
        let key = (Ulid::new(), 1);
        j.append(key, 1, 1, 0, b"first").unwrap();
        j.commit(false).unwrap();
        let first = j.committed;
        let r = j.append(key, 1, 2, 5, b"second").unwrap();
        j.commit(false).unwrap();
        let path = j.root.join(&j.name);
        let f = OpenOptions::new().write(true).open(&path).unwrap();
        f.write_all_at(b"x", r.start + HEADER as u64).unwrap();
        let (records, end, torn) = scan(&path).unwrap();
        assert_eq!(end, first);
        assert_eq!(records.len(), 1);
        assert!(torn);
        f.write_all_at(&u32::MAX.to_le_bytes(), r.start + 8)
            .unwrap();
        assert_eq!(scan(&path).unwrap().1, first);
    }
    #[test]
    fn checkpoint_carries_offset_and_rotated_tail_replays_without_effects() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("seat");
        let seat = Ulid::new();
        let ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        let mut j = Journal::open(&temp.path().join("journal")).unwrap();
        j.bind(&ring).unwrap();
        let mut playback = Playback::new(seat, 1, crate::SeatSize::default());
        let event = Event::Output {
            seat,
            generation: 1,
            seq: 1,
            bytes: b"\x1b[31mred".to_vec(),
        };
        playback.apply(&event).unwrap();
        let covered = j
            .append((seat, 1), 1, 1, 0, b"\x1b[31mred")
            .unwrap()
            .position;
        j.append((seat, 1), 1, 2, playback.byte_end, b"tail")
            .unwrap();
        let offset = covered.offset;
        let cp = j
            .checkpoint(
                (seat, 1),
                1,
                playback.byte_end,
                &playback.saved().encode().unwrap(),
                &covered,
            )
            .unwrap();
        let mut prefix = [0; CHECKPOINT_PREFIX];
        j.flush_buffer().unwrap();
        j.file
            .read_exact_at(&mut prefix, cp.start + HEADER as u64)
            .unwrap();
        assert_eq!(
            u64::from_le_bytes(prefix[20..28].try_into().unwrap()),
            offset
        );
        assert_eq!(
            format!(
                "journal-{}.log",
                Ulid::from_bytes(prefix[4..20].try_into().unwrap())
            ),
            covered.file
        );
        j.rotate().unwrap();
        j.append((seat, 1), 1, 3, playback.byte_end + 4, b"end")
            .unwrap();
        j.commit(true).unwrap();
        drop(j);
        let ring = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        let mut restored = ring.replay().unwrap();
        assert_eq!(restored.sequence, 3);
        assert_eq!(restored.byte_end, playback.byte_end + 7);
        assert!(restored.term.take_events().is_empty());
        // A second open uses the compacted cursor and does not duplicate bytes.
        let again = Ring::open(&root, seat, 1, 0, Settings::default()).unwrap();
        assert_eq!(again.status().byte_end, restored.byte_end);
    }
}
