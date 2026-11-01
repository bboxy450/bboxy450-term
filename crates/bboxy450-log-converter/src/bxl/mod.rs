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

//! Append-only bxl v0.1 transport; lc-bxl #4961. Filesystem effects are per log.
//! Format inspiration: https://github.com/facebook/zstd/tree/v1.5.7/contrib/seekable_format
//! Independently implemented; policy https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "synchronous bxl IO belongs to converter processes and per-log workers, never a UI turn"
)]
use crate::ir::{
    self,
    cbor::{self, Decoded},
};
use ciborium::value::Value;
use std::{fmt, path::PathBuf, sync::Arc, time::Instant};
mod frame;
mod reader;
mod wire;
mod writer;
pub use reader::{Reader, Tail};
pub use writer::{Recovery, Writer, recover};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
pub const MAX_EVENTS: usize = 256;
pub const ROTATE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_AGE_MS: u64 = 250;
pub type FrameSeq = u64;
/// Opaque canonical CBOR source-cursor record, committed with its batch. P7 owns its mapping.
pub type SourceCursor = Value;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Format(String),
    WriterBusy,
    Version { found: u64, supported: u64 },
    DictionaryMissing { id: u32 },
    Oversize,
    Closed,
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Format(e) => write!(f, "bxl_format: {e}"),
            Self::WriterBusy => f.write_str("writer_busy"),
            Self::Version { found, supported } => {
                write!(f, "bxl_version: found {found}, supported {supported}")
            }
            Self::DictionaryMissing { id } => write!(f, "dictionary_missing: {id}"),
            Self::Oversize => f.write_str("bxl_frame_oversize"),
            Self::Closed => f.write_str("bxl_writer_closed"),
        }
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<cbor::Error> for Error {
    fn from(e: cbor::Error) -> Self {
        Self::Format(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn invalid(message: impl Into<String>) -> Error {
    Error::Format(message.into())
}

#[derive(Clone, Debug)]
pub struct Blob {
    pub blake3: [u8; 32],
    pub bytes: Vec<u8>,
    pub unknown: Vec<(Value, Value)>,
}
impl Blob {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            blake3: *blake3::hash(&bytes).as_bytes(),
            bytes,
            unknown: vec![],
        }
    }
}
#[derive(Clone, Debug)]
pub enum Record {
    Event(Decoded<ir::Event>),
    Session(Decoded<ir::Session>),
    Blob(Blob),
    Unknown(Value),
}
impl Record {
    pub fn event(event: &ir::Event) -> Result<Self> {
        Ok(Self::Event(cbor::decode(&cbor::encode(event)?)?))
    }
    pub fn session(session: &ir::Session) -> Result<Self> {
        Ok(Self::Session(cbor::decode(&cbor::encode(session)?)?))
    }
    pub fn seq(&self) -> Option<u64> {
        match self {
            Self::Event(e) => Some(e.value().seq),
            _ => None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Frame {
    pub seq: FrameSeq,
    pub first_event_seq: u64,
    pub source_cursor: Option<SourceCursor>,
    pub records: Vec<Record>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub segment_no: u64,
    pub offset: u64,
    pub frame_seq: u64,
    pub index_in_frame: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub major: u64,
    pub minor: u64,
    pub session_id: ir::SessionId,
    pub segment_no: u64,
    pub dict_id: u32,
    pub writer_id: ir::SessionId,
    pub created: Option<ir::Timestamp>,
    pub unknown: Vec<(Value, Value)>,
}

pub trait Clock: Send + Sync + 'static {
    fn now_ms(&self) -> u64;
}
pub struct MonotonicClock(Instant);
impl Default for MonotonicClock {
    fn default() -> Self {
        Self(Instant::now())
    }
}
impl Clock for MonotonicClock {
    fn now_ms(&self) -> u64 {
        self.0.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
    }
}
#[derive(Clone)]
pub struct Options {
    pub dictionary: Option<Vec<u8>>,
    pub clock: Arc<dyn Clock>,
    pub segment_bytes: u64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            dictionary: None,
            clock: Arc::new(MonotonicClock::default()),
            segment_bytes: ROTATE_BYTES,
        }
    }
}
pub(crate) fn segment(root: &std::path::Path, n: u64) -> PathBuf {
    root.join(format!("{n:06}.bxl"))
}
pub(crate) fn index(root: &std::path::Path, n: u64) -> PathBuf {
    root.join(format!("{n:06}.idx"))
}
