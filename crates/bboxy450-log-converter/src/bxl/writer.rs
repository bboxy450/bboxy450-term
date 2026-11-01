// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Origin: P2 task #5084, ported from bboxy450-log-converter into bboxy450-term.
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! One owning worker per log; a bounded command lane and a 250ms flush deadline.
//! Durability order: frame -> file sync -> temp index sync -> rename -> directory sync.
use super::{
    frame::Physical,
    wire::{Entry, Index},
    *,
};
use std::{
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

struct Core {
    root: PathBuf,
    _lease: File,
    file: File,
    header: Header,
    dict: Vec<u8>,
    index: Index,
    physical: Vec<(u32, u32)>,
    next_frame: u64,
    next_event: u64,
    buffer: Vec<Record>,
    after: Option<SourceCursor>,
    started: Option<u64>,
    clock: Arc<dyn Clock>,
    segment_bytes: u64,
}
fn sync_dir(root: &std::path::Path) -> Result<()> {
    File::open(root)?.sync_all()?;
    Ok(())
}
fn publish_index(root: &std::path::Path, n: u64, index: &Index) -> Result<()> {
    let target = index_path(root, n);
    let temp = target.with_extension("idx.tmp");
    match std::fs::remove_file(&temp) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    file.write_all(&index.bytes()?)?;
    file.sync_all()?;
    std::fs::rename(temp, target)?;
    sync_dir(root)
}
fn index_path(root: &std::path::Path, n: u64) -> PathBuf {
    super::index(root, n)
}
impl Core {
    fn open(
        root: PathBuf,
        session: &ir::Session,
        options: Options,
        for_write: bool,
    ) -> Result<Self> {
        if !(128..=ROTATE_BYTES).contains(&options.segment_bytes) {
            return Err(invalid("segment byte budget"));
        }
        std::fs::create_dir_all(&root)?;
        let lease = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(segment(&root, 1))?;
        lease.try_lock().map_err(|e| match e {
            std::fs::TryLockError::WouldBlock => Error::WriterBusy,
            std::fs::TryLockError::Error(e) => Error::Io(e),
        })?;
        let mut dict = options.dictionary.unwrap_or_default();
        if !dict.is_empty() {
            if zstd::zstd_safe::get_dict_id_from_dict(&dict).is_none() {
                return Err(invalid("dictionary has no zstd id"));
            }
            std::fs::create_dir_all(root.join("dicts"))?;
            let path = root
                .join("dicts")
                .join(format!("{}.dict", blake3::hash(&dict).to_hex()));
            if !path.exists() {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?;
                file.write_all(&dict)?;
                file.sync_all()?;
                sync_dir(&root.join("dicts"))?;
            }
        }
        let mut n = 1;
        let mut next_frame = 0;
        let mut next_event = 0;
        loop {
            let mut file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(segment(&root, n))?;
            let (header, mut offset) = match frame::read(&mut file, 0, &[]) {
                Ok(Some(Physical::Skip {
                    magic: frame::HEADER_MAGIC,
                    payload,
                    len,
                })) => (Header::decode(&payload)?, len),
                Ok(None) => {
                    if n > 1 && segment(&root, n + 1).exists() {
                        return Err(invalid("missing middle header"));
                    }
                    file.set_len(0)?;
                    let header = Header {
                        major: 1,
                        minor: 0,
                        session_id: session.id,
                        segment_no: n,
                        dict_id: zstd::zstd_safe::get_dict_id_from_dict(&dict)
                            .map_or(0, |v| v.get()),
                        writer_id: session.id,
                        created: session.created_at.clone(),
                        unknown: vec![],
                    };
                    let bytes = frame::skippable(frame::HEADER_MAGIC, &header.encode()?)?;
                    file.seek(SeekFrom::Start(0))?;
                    file.write_all(&bytes)?;
                    file.sync_all()?;
                    sync_dir(&root)?;
                    (header, bytes.len() as u64)
                }
                Ok(_) => return Err(invalid("segment header")),
                Err(e) => return Err(e),
            };
            if header.session_id != session.id || header.segment_no != n {
                return Err(invalid("session/segment mismatch"));
            }
            if dict.is_empty() {
                dict = reader::load_dictionary(&root, header.dict_id)?;
            }
            if header.dict_id
                != zstd::zstd_safe::get_dict_id_from_dict(&dict).map_or(0, |v| v.get())
            {
                return Err(invalid("dictionary changed"));
            }
            let mut index = Index {
                segment_len: offset,
                ..Index::default()
            };
            let mut physical = vec![(offset as u32, 0)];
            let mut sealed = false;
            loop {
                let parsed =
                    frame::read(&mut file, offset, &dict).and_then(|physical| match physical {
                        Some(Physical::Data { compressed, plain }) => {
                            let data = Frame::decode(&plain)?;
                            if data.seq != next_frame || data.first_event_seq != next_event {
                                return Err(invalid("frame sequence"));
                            }
                            Ok(Some((compressed, plain, Some(data), 0)))
                        }
                        Some(Physical::Skip {
                            magic,
                            payload,
                            len,
                        }) => {
                            if magic == frame::HEADER_MAGIC {
                                return Err(invalid("duplicate header"));
                            }
                            if magic == frame::SEQ_MAGIC {
                                wire::decode(&payload)?;
                            }
                            Ok(Some((vec![], vec![], None, (magic as u64) << 32 | len)))
                        }
                        None => Ok(None),
                    });
                match parsed {
                    Ok(None) => break,
                    Ok(Some((compressed, plain, Some(data), _))) => {
                        index.entries.push(Entry {
                            offset,
                            frame_seq: data.seq,
                            first_event_seq: data.first_event_seq,
                        });
                        index.last_frame_seq = Some(data.seq);
                        index.last_frame_xxh64 = frame::hash(&compressed);
                        offset += compressed.len() as u64;
                        next_frame += 1;
                        next_event +=
                            data.records.iter().filter(|r| r.seq().is_some()).count() as u64;
                        physical.push((compressed.len() as u32, plain.len() as u32));
                    }
                    Ok(Some((_, _, None, packed))) => {
                        let magic = (packed >> 32) as u32;
                        let len = packed & 0xffffffff;
                        offset += len;
                        physical.push((len as u32, 0));
                        if magic == frame::SEEK_MAGIC {
                            sealed = true;
                            break;
                        }
                    }
                    Err(Error::Format(_) | Error::Oversize) => break,
                    Err(e) => return Err(e),
                }
            }
            // Sync surviving complete frames before publishing any rebuilt index.
            if file.metadata()?.len() != offset {
                file.set_len(offset)?;
            }
            file.sync_all()?;
            index.segment_len = offset;
            publish_index(&root, n, &index)?;
            if sealed && (for_write || segment(&root, n + 1).exists()) {
                n += 1;
                continue;
            }
            file.seek(SeekFrom::Start(offset))?;
            let mut core = Self {
                root,
                _lease: lease,
                file,
                header,
                dict,
                index,
                physical,
                next_frame,
                next_event,
                buffer: vec![],
                after: None,
                started: None,
                clock: options.clock,
                segment_bytes: options.segment_bytes,
            };
            if next_frame == 0 && for_write {
                core.append(Record::session(session)?, None)?;
            }
            return Ok(core);
        }
    }
    fn events(&self) -> u64 {
        self.buffer.iter().filter(|r| r.seq().is_some()).count() as u64
    }
    fn append(&mut self, record: Record, after: Option<SourceCursor>) -> Result<u64> {
        if let Some(seq) = record.seq()
            && seq != self.next_event + self.events()
        {
            return Err(invalid("append event sequence"));
        }
        if let Record::Blob(blob) = &record {
            if blake3::hash(&blob.bytes).as_bytes() != &blob.blake3 {
                return Err(invalid("blob hash"));
            }
            if self
                .buffer
                .iter()
                .any(|r| matches!(r,Record::Blob(b) if b.blake3==blob.blake3))
            {
                self.after = after.or(self.after.take());
                return Ok(self.last());
            }
            // Blob lookup streams the durable table; no session-sized in-memory set.
            let mut reader = Reader::open(&self.root)?;
            while let Some(r) = reader.next_record()? {
                if matches!(r,Record::Blob(b) if b.blake3==blob.blake3) {
                    self.after = after.or(self.after.take());
                    return Ok(self.last());
                }
            }
        }
        let end = matches!(&record,Record::Event(e) if matches!(e.value().kind,ir::EventKind::TurnEnd {..}));
        let candidate = Frame {
            seq: self.next_frame,
            first_event_seq: self.next_event,
            source_cursor: after.clone().or(self.after.clone()),
            records: self
                .buffer
                .iter()
                .cloned()
                .chain(std::iter::once(record.clone()))
                .collect(),
        };
        if candidate.encode().is_err() && !self.buffer.is_empty() {
            self.flush()?;
        }
        let frame = Frame {
            seq: self.next_frame,
            first_event_seq: self.next_event,
            source_cursor: after.clone().or(self.after.clone()),
            records: self
                .buffer
                .iter()
                .cloned()
                .chain(std::iter::once(record.clone()))
                .collect(),
        };
        frame.encode()?;
        if self.buffer.is_empty() {
            self.started = Some(self.clock.now_ms());
        }
        self.buffer.push(record);
        self.after = after.or(self.after.take());
        if end || self.events() >= MAX_EVENTS as u64 {
            self.flush()?;
        }
        Ok(self.last())
    }
    fn last(&self) -> u64 {
        self.next_frame.saturating_sub(1)
    }
    fn due(&self) -> bool {
        self.started
            .is_some_and(|at| self.clock.now_ms().saturating_sub(at) >= MAX_AGE_MS)
    }
    fn flush(&mut self) -> Result<u64> {
        if self.buffer.is_empty() && self.after.is_none() {
            return Ok(self.last());
        }
        let data = Frame {
            seq: self.next_frame,
            first_event_seq: self.next_event,
            source_cursor: self.after.clone(),
            records: self.buffer.clone(),
        };
        let plain = data.encode()?;
        let compressed = frame::compress(&plain, &self.dict)?;
        // Reserve the closing metadata/table so a segment never exceeds 64 MiB.
        let reserve = 64 + (self.physical.len() + 3) as u64 * 48;
        if (self.index.segment_len + compressed.len() as u64 + reserve > self.segment_bytes
            || self.index.entries.len() >= MAX_FRAME_BYTES / 64)
            && !self.index.entries.is_empty()
        {
            self.seal(true)?;
        }
        let reserve = 64 + (self.physical.len() + 3) as u64 * 48;
        if self.index.segment_len + compressed.len() as u64 + reserve > self.segment_bytes {
            return Err(Error::Oversize);
        }
        let offset = self.file.stream_position()?;
        self.file.write_all(&compressed)?;
        self.file.sync_all()?;
        self.index.entries.push(Entry {
            offset,
            frame_seq: data.seq,
            first_event_seq: data.first_event_seq,
        });
        self.index.segment_len = offset + compressed.len() as u64;
        self.index.last_frame_seq = Some(data.seq);
        self.index.last_frame_xxh64 = frame::hash(&compressed);
        self.physical
            .push((compressed.len() as u32, plain.len() as u32));
        publish_index(&self.root, self.header.segment_no, &self.index)?;
        self.next_frame += 1;
        self.next_event += self.events();
        self.buffer.clear();
        self.after = None;
        self.started = None;
        Ok(data.seq)
    }
    fn seal(&mut self, rotate: bool) -> Result<u64> {
        // Caller flushes first, except automatic rotation of an already encoded buffer.
        let metadata = Value::Array(
            self.index
                .entries
                .iter()
                .map(|e| Value::Array(vec![e.offset.into(), e.first_event_seq.into()]))
                .collect(),
        );
        let ext = frame::skippable(frame::SEQ_MAGIC, &wire::encode(metadata)?)?;
        self.file.write_all(&ext)?;
        self.file.sync_all()?;
        self.physical.push((ext.len() as u32, 0));
        let table = frame::seek_table(&self.physical)?;
        self.file.write_all(&table)?;
        self.file.sync_all()?;
        self.index.segment_len = self.file.stream_position()?;
        publish_index(&self.root, self.header.segment_no, &self.index)?;
        if rotate {
            self.header.segment_no += 1;
            self.file = OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(segment(&self.root, self.header.segment_no))?;
            let bytes = frame::skippable(frame::HEADER_MAGIC, &self.header.encode()?)?;
            self.file.write_all(&bytes)?;
            self.file.sync_all()?;
            sync_dir(&self.root)?;
            self.index = Index {
                segment_len: bytes.len() as u64,
                ..Index::default()
            };
            self.physical = vec![(bytes.len() as u32, 0)];
            publish_index(&self.root, self.header.segment_no, &self.index)?;
        }
        Ok(self.last())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Recovery {
    pub segment_no: u64,
    pub valid_bytes: u64,
    pub next_frame_seq: u64,
    pub next_event_seq: u64,
}
/// Recover only; unlike opening a new writer, this never rotates a sealed log.
pub fn recover(root: impl AsRef<std::path::Path>, session: &ir::Session) -> Result<Recovery> {
    let core = Core::open(
        root.as_ref().to_path_buf(),
        session,
        Options::default(),
        false,
    )?;
    Ok(Recovery {
        segment_no: core.header.segment_no,
        valid_bytes: core.index.segment_len,
        next_frame_seq: core.next_frame,
        next_event_seq: core.next_event,
    })
}
enum Command {
    Append(Box<Record>, Option<SourceCursor>),
    Flush,
    Cursor(SourceCursor),
    Seal,
    Poll,
    Close,
}
struct Request {
    command: Command,
    reply: mpsc::SyncSender<Result<u64>>,
}
pub struct Writer {
    send: Option<mpsc::SyncSender<Request>>,
    worker: Option<JoinHandle<()>>,
}
impl Writer {
    pub fn open(root: impl AsRef<std::path::Path>, session: &ir::Session) -> Result<Self> {
        Self::with_options(root, session, Options::default())
    }
    pub fn with_options(
        root: impl AsRef<std::path::Path>,
        session: &ir::Session,
        options: Options,
    ) -> Result<Self> {
        let mut core = Core::open(root.as_ref().to_path_buf(), session, options, true)?;
        let (send, receive) = mpsc::sync_channel::<Request>(1);
        let worker = thread::spawn(move || {
            loop {
                let request = if let Some(started) = core.started {
                    let left =
                        MAX_AGE_MS.saturating_sub(core.clock.now_ms().saturating_sub(started));
                    match receive.recv_timeout(Duration::from_millis(left)) {
                        Ok(r) => r,
                        Err(mpsc::RecvTimeoutError::Timeout) => {
                            if core.due() && core.flush().is_err() {
                                break;
                            }
                            continue;
                        }
                        Err(_) => break,
                    }
                } else {
                    match receive.recv() {
                        Ok(r) => r,
                        Err(_) => break,
                    }
                };
                let close = matches!(&request.command, Command::Close);
                let result = (|| {
                    if core.due() {
                        core.flush()?;
                    }
                    match request.command {
                        Command::Append(record, after) => core.append(*record, after),
                        Command::Flush => core.flush(),
                        Command::Cursor(after) => {
                            let frame = Frame {
                                seq: core.next_frame,
                                first_event_seq: core.next_event,
                                source_cursor: Some(after.clone()),
                                records: core.buffer.clone(),
                            };
                            frame.encode()?;
                            core.after = Some(after);
                            if core.started.is_none() {
                                core.started = Some(core.clock.now_ms());
                            }
                            Ok(core.last())
                        }
                        Command::Seal => {
                            core.flush()?;
                            core.seal(true)
                        }
                        Command::Poll => Ok(core.last()),
                        Command::Close => {
                            core.flush()?;
                            core.seal(false)
                        }
                    }
                })();
                let failed = result.is_err();
                let _ = request.reply.send(result);
                if close || failed {
                    break;
                }
            }
        });
        Ok(Self {
            send: Some(send),
            worker: Some(worker),
        })
    }
    fn request(&mut self, command: Command) -> Result<u64> {
        let (reply, receive) = mpsc::sync_channel(1);
        self.send
            .as_ref()
            .ok_or(Error::Closed)?
            .send(Request { command, reply })
            .map_err(|_| Error::Closed)?;
        receive.recv().map_err(|_| Error::Closed)?
    }
    pub fn append(&mut self, records: &[Record], after: Option<SourceCursor>) -> Result<()> {
        for (index, record) in records.iter().enumerate() {
            self.request(Command::Append(
                Box::new(record.clone()),
                if index + 1 == records.len() {
                    after.clone()
                } else {
                    None
                },
            ))?;
        }
        if records.is_empty()
            && let Some(after) = after
        {
            self.request(Command::Cursor(after))?;
        }
        Ok(())
    }
    pub fn flush(&mut self) -> Result<FrameSeq> {
        self.request(Command::Flush)
    }
    pub fn sync(&mut self) -> Result<FrameSeq> {
        self.flush()
    }
    pub fn seal(&mut self) -> Result<FrameSeq> {
        self.request(Command::Seal)
    }
    /// Drives an injected clock without sleeping; production also has a real deadline worker.
    pub fn poll(&mut self) -> Result<FrameSeq> {
        self.request(Command::Poll)
    }
    pub fn close(mut self) -> Result<()> {
        let result = self.request(Command::Close);
        self.send.take();
        if let Some(worker) = self.worker.take() {
            worker.join().map_err(|_| invalid("writer panicked"))?;
        }
        result.map(|_| ())
    }
}
impl Drop for Writer {
    fn drop(&mut self) {
        if self.send.is_some() {
            let _ = self.request(Command::Close);
            self.send.take();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
