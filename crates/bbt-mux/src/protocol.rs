//! Version 1 of the private supervisor protocol, shared by both transports.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

pub const VERSION: u16 = 1;
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
pub const OUTPUT_CHUNK_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frame<T> {
    pub version: u16,
    pub body: T,
}

impl<T> Frame<T> {
    pub fn new(body: T) -> Self {
        Self { version: VERSION, body }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatSize {
    pub rows: u16,
    pub cols: u16,
    pub pixel_width: u16,
    pub pixel_height: u16,
}

impl Default for SeatSize {
    fn default() -> Self {
        Self { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 }
    }
}

impl SeatSize {
    pub(crate) fn validate(self) -> Result<(), Failure> {
        if self.rows == 0 || self.cols == 0 {
            return Err(Failure::new(FailureCode::Invalid, "PTY dimensions must be nonzero"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchSpec {
    pub seat: Ulid,
    pub generation: u64,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// Explicit overrides only. The inherited environment is never recorded.
    pub env: BTreeMap<String, String>,
    pub size: SeatSize,
}

impl LaunchSpec {
    pub fn new(program: impl Into<String>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            seat: Ulid::new(), generation: 1, program: program.into(), args: Vec::new(),
            cwd: cwd.into(), env: BTreeMap::new(), size: SeatSize::default(),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), Failure> {
        self.size.validate()?;
        if self.seat == Ulid::nil() || self.generation == 0 || self.program.is_empty()
            || self.program.contains('\0') || !self.cwd.is_absolute()
            || self.args.len() > 1024 || self.env.len() > 256
        {
            return Err(Failure::new(FailureCode::Invalid, "invalid launch identity, cwd or command"));
        }
        let mut bytes = self.program.len() + self.cwd.as_os_str().len();
        for arg in &self.args {
            bytes = bytes.saturating_add(arg.len());
            if arg.contains('\0') {
                return Err(Failure::new(FailureCode::Invalid, "NUL in launch argument"));
            }
        }
        for (key, value) in &self.env {
            bytes = bytes.saturating_add(key.len()).saturating_add(value.len());
            if key.is_empty() || key.contains(['=', '\0']) || value.contains('\0') {
                return Err(Failure::new(FailureCode::Invalid, "invalid environment override"));
            }
        }
        if bytes > MAX_INPUT_BYTES {
            return Err(Failure::new(FailureCode::Limit, "launch configuration exceeds 64 KiB"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub request_id: Ulid,
    pub logic_lease: Option<Ulid>,
    pub action: Action,
}

impl Request {
    pub fn new(logic_lease: Option<Ulid>, action: Action) -> Self {
        Self { request_id: Ulid::new(), logic_lease, action }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Action {
    /// Admission capability comes from the owner, never from a seat name/path.
    AcquireLogic { capability: [u8; 32] },
    ListOwnedSeats,
    Spawn { spec: LaunchSpec },
    Attach { seat: Ulid, generation: u64 },
    Input { attachment: Attachment, input_id: Ulid, bytes: Vec<u8> },
    InputStatus { seat: Ulid, generation: u64, input_id: Ulid },
    Resize { attachment: Attachment, size: SeatSize },
    Kill { attachment: Attachment },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attachment {
    pub seat: Ulid,
    pub generation: u64,
    pub writer_lease: Ulid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitInfo {
    pub code: u32,
    pub signal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeatInfo {
    pub seat: Ulid,
    pub generation: u64,
    pub pid: u32,
    pub size: SeatSize,
    pub next_sequence: u64,
    pub exited: Option<ExitInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputState {
    Pending,
    PtyWritten,
    /// A prefix may have been written. Query, never blindly paste again.
    PartialUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputReceipt {
    pub input_id: Ulid,
    pub state: InputState,
    pub bytes_written: usize,
    pub bytes_total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Reply {
    Acquired { logic_lease: Ulid, supervisor_boot: Ulid },
    Seats { seats: Vec<SeatInfo> },
    Spawned { info: SeatInfo },
    Attached {
        attachment: Attachment,
        info: SeatInfo,
        /// No historical replay before this cursor in W2.1 (the ring is W1.2).
        first_available_sequence: u64,
    },
    Input { receipt: InputReceipt },
    Resized { size: SeatSize, sequence: u64 },
    Terminating,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Event {
    Output { seat: Ulid, generation: u64, seq: u64, bytes: Vec<u8> },
    Resized { seat: Ulid, generation: u64, seq: u64, size: SeatSize },
    Exited { seat: Ulid, generation: u64, seq: u64, status: ExitInfo },
    InputReceipt { seat: Ulid, generation: u64, receipt: InputReceipt },
}

impl Event {
    pub fn seat(&self) -> Ulid {
        match self {
            Self::Output { seat, .. } | Self::Resized { seat, .. }
            | Self::Exited { seat, .. } | Self::InputReceipt { seat, .. } => *seat,
        }
    }

    pub(crate) fn sequence(&self) -> Option<u64> {
        match self {
            Self::Output { seq, .. } | Self::Resized { seq, .. } | Self::Exited { seq, .. } => Some(*seq),
            Self::InputReceipt { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Message {
    Reply { request_id: Ulid, result: Result<Reply, Failure> },
    Event { event: Event },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureCode {
    Unauthorized, StaleLease, StaleGeneration, StaleWriter, NotFound,
    Invalid, Limit, Busy, Exited, InputConflict, Io, Version,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub code: FailureCode,
    pub message: String,
}

impl Failure {
    pub(crate) fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self { code, message: message.into() }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for Failure {}

impl From<std::io::Error> for Failure {
    fn from(error: std::io::Error) -> Self {
        Self::new(FailureCode::Io, error.to_string())
    }
}
