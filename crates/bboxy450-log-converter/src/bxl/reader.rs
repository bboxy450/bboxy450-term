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

//! One validated frame at a time. Partial tails never advance a delivery cursor.
use super::{frame::Physical, *};
use notify::Watcher as _;
use std::{
    collections::VecDeque,
    fs::File,
    io::Read,
    sync::mpsc,
    time::{Duration, Instant},
};

pub struct Reader {
    root: PathBuf,
    file: File,
    pub header: Header,
    dict: Vec<u8>,
    offset: u64,
    next_frame: u64,
    next_event: u64,
    sealed: bool,
    current: VecDeque<Record>,
    frame_start: u64,
    frame_seq: u64,
    index_in_frame: usize,
    pub source_cursor: Option<SourceCursor>,
}
impl Reader {
    pub fn open(root: impl AsRef<std::path::Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let (file, header, dict, offset) = Self::open_segment(&root, 1)?;
        Ok(Self {
            root,
            file,
            header,
            dict,
            offset,
            next_frame: 0,
            next_event: 0,
            sealed: false,
            current: VecDeque::new(),
            frame_start: offset,
            frame_seq: 0,
            index_in_frame: 0,
            source_cursor: None,
        })
    }
    fn open_segment(root: &std::path::Path, n: u64) -> Result<(File, Header, Vec<u8>, u64)> {
        let mut file = File::open(segment(root, n))?;
        let Some(Physical::Skip {
            magic: frame::HEADER_MAGIC,
            payload,
            len,
        }) = frame::read(&mut file, 0, &[])?
        else {
            return Err(invalid("incomplete segment header"));
        };
        let header = Header::decode(&payload)?;
        if header.segment_no != n {
            return Err(invalid("segment number"));
        }
        let dict = load_dictionary(root, header.dict_id)?;
        Ok((file, header, dict, len))
    }
    pub fn next_frame(&mut self) -> Result<Option<Frame>> {
        self.current.clear();
        loop {
            if self.sealed {
                if !segment(&self.root, self.header.segment_no + 1).exists() {
                    return Ok(None);
                }
                let (file, header, dict, offset) =
                    Self::open_segment(&self.root, self.header.segment_no + 1)?;
                if header.session_id != self.header.session_id {
                    return Err(invalid("session changed at rotation"));
                }
                self.file = file;
                self.header = header;
                self.dict = dict;
                self.offset = offset;
                self.sealed = false;
            }
            match frame::read(&mut self.file, self.offset, &self.dict)? {
                None => return Ok(None),
                Some(Physical::Skip { magic, len, .. }) => {
                    self.offset += len;
                    if magic == frame::SEEK_MAGIC {
                        self.sealed = true;
                    }
                }
                Some(Physical::Data { compressed, plain }) => {
                    let data = Frame::decode(&plain)?;
                    if data.seq != self.next_frame || data.first_event_seq != self.next_event {
                        return Err(invalid("nonconsecutive frame/event sequence"));
                    }
                    self.frame_start = self.offset;
                    self.frame_seq = data.seq;
                    self.index_in_frame = 0;
                    self.offset += compressed.len() as u64;
                    self.next_frame += 1;
                    self.next_event +=
                        data.records.iter().filter(|r| r.seq().is_some()).count() as u64;
                    if data.source_cursor.is_some() {
                        self.source_cursor = data.source_cursor.clone();
                    }
                    return Ok(Some(data));
                }
            }
        }
    }
    pub fn next_record(&mut self) -> Result<Option<Record>> {
        loop {
            if let Some(next) = self.current.pop_front() {
                self.index_in_frame += 1;
                return Ok(Some(next));
            }
            let Some(frame) = self.next_frame()? else {
                return Ok(None);
            };
            self.current = frame.records.into();
        }
    }

    pub fn next_event(&mut self) -> Result<Option<Decoded<ir::Event>>> {
        while let Some(record) = self.next_record()? {
            if let Record::Event(event) = record {
                return Ok(Some(event));
            }
        }
        Ok(None)
    }
    pub fn cursor(&self) -> Cursor {
        Cursor {
            segment_no: self.header.segment_no,
            offset: self.frame_start,
            frame_seq: self.frame_seq,
            index_in_frame: self.index_in_frame,
        }
    }
    /// A resume revalidates the saved frame and discards its already-delivered prefix.
    pub fn resume(root: impl AsRef<std::path::Path>, cursor: &Cursor) -> Result<Self> {
        let mut reader = Self::open(root)?;
        loop {
            let Some(frame) = reader.next_frame()? else {
                return Err(invalid("cursor beyond valid prefix"));
            };
            if reader.header.segment_no == cursor.segment_no
                && reader.frame_start == cursor.offset
                && frame.seq == cursor.frame_seq
            {
                if cursor.index_in_frame > frame.records.len() {
                    return Err(invalid("cursor index"));
                }
                reader.current = frame
                    .records
                    .into_iter()
                    .skip(cursor.index_in_frame)
                    .collect();
                reader.index_in_frame = cursor.index_in_frame;
                return Ok(reader);
            }
        }
    }
    pub fn seek_event(&mut self, seq: u64) -> Result<()> {
        *self = Self::open(self.root.clone())?;
        while let Some(frame) = self.next_frame()? {
            if let Some(at) = frame
                .records
                .iter()
                .position(|r| r.seq().is_some_and(|n| n >= seq))
            {
                self.current = frame.records.into_iter().skip(at).collect();
                self.index_in_frame = at;
                return Ok(());
            }
        }
        Ok(())
    }
    /// Cache validation is bounded; oversized/invalid caches fall back to scanning.
    pub fn index_is_valid(&mut self) -> bool {
        let check = || -> Result<bool> {
            let mut bytes = Vec::new();
            File::open(index(&self.root, self.header.segment_no))?
                .take(MAX_FRAME_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_FRAME_BYTES {
                return Ok(false);
            }
            let v = wire::decode(&bytes)?;
            if wire::uint(wire::field(&v, 0)?)? > self.file.metadata()?.len() {
                return Ok(false);
            }
            let Value::Array(entries) = wire::field(&v, 3)? else {
                return Ok(false);
            };
            let Some(Value::Array(last)) = entries.last() else {
                return Ok(entries.is_empty());
            };
            if last.len() != 3 {
                return Ok(false);
            }
            let mut file = File::open(segment(&self.root, self.header.segment_no))?;
            let Some(Physical::Data { compressed, plain }) =
                frame::read(&mut file, wire::uint(&last[0])?, &self.dict)?
            else {
                return Ok(false);
            };
            let frame = Frame::decode(&plain)?;
            Ok(
                wire::uint(&last[0])? + compressed.len() as u64 <= wire::uint(wire::field(&v, 0)?)?
                    && frame::hash(&compressed) == wire::uint(wire::field(&v, 2)?)?
                    && frame.seq == wire::uint(wire::field(&v, 1)?)?
                    && frame.seq == wire::uint(&last[1])?
                    && frame.first_event_seq == wire::uint(&last[2])?,
            )
        };
        check().unwrap_or(false)
    }
}
pub(crate) fn load_dictionary(root: &std::path::Path, id: u32) -> Result<Vec<u8>> {
    if id == 0 {
        return Ok(Vec::new());
    }
    let directory = root.join("dicts");
    if let Ok(entries) = std::fs::read_dir(directory) {
        for entry in entries {
            let path = entry?.path();
            if !path.is_file() {
                continue;
            }
            let mut bytes = Vec::new();
            File::open(&path)?
                .take(MAX_FRAME_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_FRAME_BYTES {
                continue;
            }
            if zstd::zstd_safe::get_dict_id_from_dict(&bytes).map_or(0, |n| n.get()) == id {
                let expected = blake3::hash(&bytes).to_hex().to_string();
                if path.file_stem().and_then(|s| s.to_str()) == Some(expected.as_str()) {
                    return Ok(bytes);
                }
            }
        }
    }
    Err(Error::DictionaryMissing { id })
}
pub struct Tail {
    reader: Reader,
    wake: mpsc::Receiver<()>,
    _watch: Option<notify::RecommendedWatcher>,
}
impl Tail {
    pub fn open(root: impl AsRef<std::path::Path>) -> Result<Self> {
        Self::from_reader(Reader::open(root)?)
    }
    pub fn resume(root: impl AsRef<std::path::Path>, cursor: &Cursor) -> Result<Self> {
        Self::from_reader(Reader::resume(root, cursor)?)
    }
    fn from_reader(reader: Reader) -> Result<Self> {
        let (send, wake) = mpsc::sync_channel(1);
        let watch = notify::recommended_watcher(move |_event: notify::Result<notify::Event>| {
            let _ = send.try_send(());
        })
        .and_then(|mut watcher| {
            watcher.watch(&reader.root, notify::RecursiveMode::NonRecursive)?;
            Ok(watcher)
        })
        .ok();
        Ok(Self {
            reader,
            wake,
            _watch: watch,
        })
    }
    pub fn next_event(&mut self, timeout: Duration) -> Result<Option<Decoded<ir::Event>>> {
        let until = Instant::now() + timeout;
        loop {
            match self.reader.next_event() {
                Ok(Some(event)) => return Ok(Some(event)),
                Ok(None) => {}
                Err(Error::Format(message)) if message == "incomplete segment header" => {}
                Err(e) => return Err(e),
            }
            let now = Instant::now();
            if now >= until {
                return Ok(None);
            }
            let _ = self
                .wake
                .recv_timeout((until - now).min(Duration::from_millis(10)));
        }
    }
    pub fn cursor(&self) -> Cursor {
        self.reader.cursor()
    }
}
