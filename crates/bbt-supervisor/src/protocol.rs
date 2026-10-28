// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Version 1 of the private supervisor protocol, shared by both transports.

use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Current supervisor wire-protocol version.
pub const VERSION: u16 = 1;
/// Maximum accepted PTY input or aggregate launch-configuration bytes.
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
/// Maximum original PTY bytes in one output chunk.
pub const OUTPUT_CHUNK_BYTES: usize = 16 * 1024;
/// One replay prefetch stays below the private socket's single-frame reserve.
pub const MAX_RING_BATCH_MEMBERS: usize = 8;
/// Maximum accepted aggregate byte budget for a batched ring read.
pub const MAX_RING_BATCH_BYTES: usize = 768 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Versioned supervisor transport envelope with one typed payload.
pub struct Frame<T> {
    /// Protocol version carried by this envelope.
    pub version: u16,
    /// Typed envelope payload.
    pub body: T,
}

impl<T> Frame<T> {
    /// Wrap a payload using the current wire version.
    pub fn new(body: T) -> Self {
        Self {
            version: VERSION,
            body,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// PTY cell dimensions and optional pixel dimensions.
pub struct SeatSize {
    /// Visible PTY row count; must be nonzero for admission.
    pub rows: u16,
    /// Visible PTY column count; must be nonzero for admission.
    pub cols: u16,
    /// PTY pixel width, or zero when unspecified.
    pub pixel_width: u16,
    /// PTY pixel height, or zero when unspecified.
    pub pixel_height: u16,
}

impl Default for SeatSize {
    fn default() -> Self {
        Self {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl SeatSize {
    pub(crate) fn validate(self) -> Result<(), Failure> {
        if self.rows == 0 || self.cols == 0 {
            return Err(Failure::new(
                FailureCode::Invalid,
                "PTY dimensions must be nonzero",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Explicit process-generation launch command, directory, environment overrides and PTY size.
pub struct LaunchSpec {
    /// Opaque process identity owned by this supervisor.
    pub seat: Ulid,
    /// Nonzero process-generation fence.
    pub generation: u64,
    /// Executable selected explicitly by the caller.
    pub program: String,
    /// Ordered argument strings, without shell interpolation by this library.
    pub args: Vec<String>,
    /// Absolute working directory selected by the caller.
    pub cwd: PathBuf,
    /// Explicit overrides only. The inherited environment is never recorded.
    /// Explicit environment overrides for this process.
    pub env: BTreeMap<String, String>,
    /// PTY cell and pixel dimensions.
    pub size: SeatSize,
    /// An optional process owned for this seat's lifetime, outside its PTY.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub companion: Option<Box<CompanionSpec>>,
}

/// A seat-local service started alongside its PTY. The client command handles
/// its own socket readiness; the host never waits on an external service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanionSpec {
    /// Executable selected explicitly by the caller.
    pub program: String,
    /// Ordered argument strings, without shell interpolation by this library.
    pub args: Vec<String>,
    /// Explicit environment overrides for this process.
    pub env: BTreeMap<String, String>,
    /// Caller-selected socket path advertised by the owned companion.
    pub ready_socket: PathBuf,
}

/// The actual owned process, retained across client/logic reconnections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanionInfo {
    /// Operating-system process identifier; use generation metadata to disambiguate reuse.
    pub pid: u32,
    /// Caller-selected socket path advertised by the owned companion.
    pub ready_socket: PathBuf,
}

impl LaunchSpec {
    /// Create a fresh identity at generation one with default PTY dimensions; validate before admission.
    pub fn new(program: impl Into<String>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            seat: Ulid::new(),
            generation: 1,
            program: program.into(),
            args: Vec::new(),
            cwd: cwd.into(),
            env: BTreeMap::new(),
            size: SeatSize::default(),
            companion: None,
        }
    }

    /// Validate identity, dimensions, command, environment and companion bounds.
    ///
    /// # Errors
    /// Returns Invalid for malformed fields or Limit when the aggregate configuration exceeds 64 KiB.
    pub fn validate(&self) -> Result<(), Failure> {
        self.size.validate()?;
        if self.seat == Ulid::nil()
            || self.generation == 0
            || self.program.is_empty()
            || self.program.contains('\0')
            || !self.cwd.is_absolute()
            || self.args.len() > 1024
            || self.env.len() > 256
        {
            return Err(Failure::new(
                FailureCode::Invalid,
                "invalid launch identity, cwd or command",
            ));
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
                return Err(Failure::new(
                    FailureCode::Invalid,
                    "invalid environment override",
                ));
            }
        }
        if let Some(companion) = &self.companion {
            let mut service = Self::new(&companion.program, &self.cwd);
            service.args = companion.args.clone();
            service.env = companion.env.clone();
            service.validate()?;
            if !companion.ready_socket.is_absolute()
                || companion.ready_socket.as_os_str().len() >= 104
                || companion
                    .ready_socket
                    .as_os_str()
                    .as_encoded_bytes()
                    .contains(&0)
            {
                return Err(Failure::new(
                    FailureCode::Invalid,
                    "invalid companion socket path",
                ));
            }
            bytes = bytes
                .saturating_add(companion.program.len())
                .saturating_add(companion.args.iter().map(String::len).sum::<usize>())
                .saturating_add(
                    companion
                        .env
                        .iter()
                        .map(|(k, v)| k.len() + v.len())
                        .sum::<usize>(),
                )
                .saturating_add(companion.ready_socket.as_os_str().len());
        }
        if bytes > MAX_INPUT_BYTES {
            return Err(Failure::new(
                FailureCode::Limit,
                "launch configuration exceeds 64 KiB",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Correlated control request under an optional admitted logic lease.
pub struct Request {
    /// Correlation identifier of one control request.
    pub request_id: Ulid,
    /// Current admitted logic lease, absent only for operations that do not require one.
    pub logic_lease: Option<Ulid>,
    /// Typed operation to admit and execute.
    pub action: Action,
}

impl Request {
    /// Assign a fresh request correlation identifier under the supplied optional lease.
    pub fn new(logic_lease: Option<Ulid>, action: Action) -> Self {
        Self {
            request_id: Ulid::new(),
            logic_lease,
            action,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
/// Explicit supervisor operations admitted under capabilities, generation and writer leases.
pub enum Action {
    /// Admission capability comes from the owner, never from a seat name/path.
    AcquireLogic {
        /// Owner-supplied admission capability; never derived from a name or path.
        capability: [u8; 32],
    },
    /// List processes currently retained by this supervisor.
    ListOwnedSeats,
    /// Start the explicit launch specification.
    Spawn {
        /// Explicit process launch specification.
        spec: LaunchSpec,
    },
    /// Acquire a current-generation writer attachment.
    Attach {
        /// Opaque process identity owned by this supervisor.
        seat: Ulid,
        /// Nonzero process-generation fence.
        generation: u64,
    },
    /// Enable capture for a current process generation using the selected directory and budgets.
    AttachRing {
        /// Opaque process identity owned by this supervisor.
        seat: Ulid,
        /// Nonzero process-generation fence.
        generation: u64,
        /// Caller-selected retained-capture directory.
        directory: PathBuf,
        /// Capture capacity, checkpoint and idle-flush budgets.
        settings: crate::ring::Settings,
    },
    /// Read committed capture state and its latest checkpoint.
    RingRecovery {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
    },
    /// Read an inclusive capture-sequence interval.
    ReadRing {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
        /// Inclusive first requested capture sequence.
        from: u64,
        /// Inclusive last requested capture sequence.
        to: u64,
    },
    /// All members are admitted before waiting; the reply is one bounded group.
    ReadRingBatch {
        /// Independent range requests admitted as one bounded batch.
        reads: Vec<RingRead>,
        /// Upper budget for the batched history reply.
        max_bytes: u32,
    },
    /// Commit opaque checkpoint bytes for the attached generation.
    PublishCheckpoint {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
        /// Opaque serialized model/parser checkpoint bytes.
        checkpoint: Vec<u8>,
    },
    /// Submit idempotently identified PTY bytes.
    Input {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
        /// Stable caller-selected identity used to deduplicate PTY input.
        input_id: Ulid,
        /// Original PTY input or output bytes, according to this operation.
        bytes: Vec<u8>,
    },
    /// Read supervisor facts used to guard input admission.
    InputContext {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
    },
    /// Admit bytes only when the supervisor still observes the supplied expected context.
    GuardedInput {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
        /// Stable caller-selected identity used to deduplicate PTY input.
        input_id: Ulid,
        /// Original PTY input or output bytes, according to this operation.
        bytes: Vec<u8>,
        /// Supervisor context that must still match at guarded admission.
        expected: InputContext,
    },
    /// Read the latest receipt for an existing input identifier.
    InputStatus {
        /// Opaque process identity owned by this supervisor.
        seat: Ulid,
        /// Nonzero process-generation fence.
        generation: u64,
        /// Stable caller-selected identity used to deduplicate PTY input.
        input_id: Ulid,
    },
    /// Order a PTY resize under the current attachment.
    Resize {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
        /// PTY cell and pixel dimensions.
        size: SeatSize,
    },
    /// Request termination under the current attachment.
    Kill {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
    },
    /// Schedule a correlated process closure at an explicit wall-clock deadline.
    ScheduleClose {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
        /// Correlation identifier of the scheduled close operation.
        close_id: Ulid,
        /// Wall-clock close deadline in Unix-epoch milliseconds.
        deadline_ms: u64,
    },
    /// Cancel the matching scheduled closure when it is still reversible.
    UndoClose {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
        /// Correlation identifier of the scheduled close operation.
        close_id: Ulid,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Generation-bound writer attachment; possession does not bypass host lease checks.
pub struct Attachment {
    /// Opaque process identity owned by this supervisor.
    pub seat: Ulid,
    /// Nonzero process-generation fence.
    pub generation: u64,
    /// Current writer lease for the attached generation.
    pub writer_lease: Ulid,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// One generation-bound inclusive capture-sequence read.
pub struct RingRead {
    /// Current generation-bound writer attachment.
    pub attachment: Attachment,
    /// Inclusive first requested capture sequence.
    pub from: u64,
    /// Inclusive last requested capture sequence.
    pub to: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Independent success or failure for one batched capture-sequence range.
pub struct RingReadResult {
    /// Opaque process identity owned by this supervisor.
    pub seat: Ulid,
    /// Nonzero process-generation fence.
    pub generation: u64,
    /// Inclusive first requested capture sequence.
    pub from: u64,
    /// Inclusive last requested capture sequence.
    pub to: u64,
    /// Named operation success or failure; no automatic retry is implied.
    pub result: Result<Box<crate::ring::History>, Failure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Portable process exit code and optional signal description.
pub struct ExitInfo {
    /// Portable process exit code or named refusal code, according to the containing type.
    pub code: u32,
    /// Signal description reported for signal-driven termination.
    pub signal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Supervisor-owned process identity, dimensions, capture head and exit state.
pub struct SeatInfo {
    /// Opaque process identity owned by this supervisor.
    pub seat: Ulid,
    /// Nonzero process-generation fence.
    pub generation: u64,
    /// Operating-system process identifier; use generation metadata to disambiguate reuse.
    pub pid: u32,
    /// PTY cell and pixel dimensions.
    pub size: SeatSize,
    /// Next process-event sequence to be assigned.
    pub next_sequence: u64,
    /// Observed final process status, absent while exit is not recorded.
    pub exited: Option<ExitInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Optional lifetime-owned companion metadata or launch specification.
    pub companion: Option<CompanionInfo>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// PTY write evidence; none of these states proves application consumption.
pub enum InputState {
    /// The operation has no final PTY-write disposition yet.
    Pending,
    /// All bytes were accepted by PTY writes; application consumption is not established.
    PtyWritten,
    /// A prefix may have been written. Query, never blindly paste again.
    PartialUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// Observed disposition of one idempotently identified PTY input operation.
pub struct InputReceipt {
    /// Stable caller-selected identity used to deduplicate PTY input.
    pub input_id: Ulid,
    /// Latest PTY-write evidence for this input operation.
    pub state: InputState,
    /// Number of input bytes known to have reached PTY writes.
    pub bytes_written: usize,
    /// Total input bytes in the original operation.
    pub bytes_total: usize,
}

/// Supervisor-owned facts, sampled again immediately before guarded admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputContext {
    /// Supervisor-sampled context freshness epoch.
    pub epoch: u64,
    /// Timestamp of the latest observed input in wall-clock milliseconds.
    pub last_input_ms: u64,
    /// Capture/event sequence in this process generation.
    pub sequence: u64,
    /// Whether supervisor observations indicate a pending input draft.
    pub draft: bool,
    /// Observed foreground process-group identity, when available.
    pub foreground_pid: Option<u32>,
    /// Whether canonical terminal input processing was observed.
    pub canonical: bool,
    /// Whether terminal echo was observed.
    pub echo: bool,
    /// Whether the supervisor context met its settled-input observation rule.
    pub settled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
/// Typed result of one supervisor action.
pub enum Reply {
    /// A logic lease and supervisor boot identity were admitted.
    Acquired {
        /// Current admitted logic lease, absent only for operations that do not require one.
        logic_lease: Ulid,
        /// Identity of this supervisor instance.
        supervisor_boot: Ulid,
    },
    /// Current retained process metadata.
    Seats {
        /// Current retained process metadata rows.
        seats: Vec<SeatInfo>,
    },
    /// The requested process generation was created.
    Spawned {
        /// Current metadata for the requested process generation.
        info: SeatInfo,
    },
    /// Writer attachment and process metadata for the requested generation.
    Attached {
        /// Current generation-bound writer attachment.
        attachment: Attachment,
        /// Current metadata for the requested process generation.
        info: SeatInfo,
        /// No historical replay before this cursor in W2.1 (the ring is W1.2).
        first_available_sequence: u64,
    },
    /// Latest receipt for the submitted input operation.
    Input {
        /// Latest PTY input receipt.
        receipt: InputReceipt,
    },
    /// Supervisor-sampled context for guarded input.
    InputContext {
        /// Supervisor-sampled input context.
        context: InputContext,
    },
    /// Committed capture state and its latest checkpoint, when available.
    RingRecovery {
        /// Committed capture or final process status, according to the containing variant.
        status: Box<crate::ring::Status>,
        /// Opaque serialized model/parser checkpoint bytes.
        checkpoint: Option<Vec<u8>>,
    },
    /// A bounded committed-history page, possibly with an explicit gap.
    RingHistory {
        /// Bounded committed capture history.
        history: Box<crate::ring::History>,
    },
    /// Independent per-member results for a bounded batch.
    RingHistoryBatch {
        /// Per-member batch results in the request order.
        results: Vec<RingReadResult>,
    },
    /// The checkpoint was committed through the returned capture sequence.
    CheckpointCommitted {
        /// Capture/event sequence in this process generation.
        sequence: u64,
    },
    /// The resize was applied at the returned capture sequence.
    Resized {
        /// PTY cell and pixel dimensions.
        size: SeatSize,
        /// Capture/event sequence in this process generation.
        sequence: u64,
    },
    /// Termination was requested; this is not an exit event.
    Terminating,
    /// The matching close operation has a scheduled deadline.
    CloseScheduled {
        /// Correlation identifier of the scheduled close operation.
        close_id: Ulid,
        /// Wall-clock close deadline in Unix-epoch milliseconds.
        deadline_ms: u64,
    },
    /// The matching scheduled close was cancelled.
    CloseUndone {
        /// Correlation identifier of the scheduled close operation.
        close_id: Ulid,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
/// Observed process output, resize or exit, scoped by generation and sequence.
pub enum Event {
    /// Original observed PTY bytes at a capture sequence.
    Output {
        /// Opaque process identity owned by this supervisor.
        seat: Ulid,
        /// Nonzero process-generation fence.
        generation: u64,
        /// Capture sequence for this observed process event.
        seq: u64,
        /// Original PTY input or output bytes, according to this operation.
        bytes: Vec<u8>,
    },
    /// The resize was applied at the returned capture sequence.
    Resized {
        /// Opaque process identity owned by this supervisor.
        seat: Ulid,
        /// Nonzero process-generation fence.
        generation: u64,
        /// Capture sequence for this observed process event.
        seq: u64,
        /// PTY cell and pixel dimensions.
        size: SeatSize,
    },
    /// Observed process exit at a capture sequence.
    Exited {
        /// Opaque process identity owned by this supervisor.
        seat: Ulid,
        /// Nonzero process-generation fence.
        generation: u64,
        /// Capture sequence for this observed process event.
        seq: u64,
        /// Committed capture or final process status, according to the containing variant.
        status: ExitInfo,
    },
}

impl Event {
    /// Return the process identity associated with this observed event.
    pub fn seat(&self) -> Ulid {
        match self {
            Self::Output { seat, .. } | Self::Resized { seat, .. } | Self::Exited { seat, .. } => {
                *seat
            }
        }
    }

    /// Return this generation-scoped capture sequence; every current event variant has one.
    pub fn sequence(&self) -> Option<u64> {
        match self {
            Self::Output { seq, .. } | Self::Resized { seq, .. } | Self::Exited { seq, .. } => {
                Some(*seq)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
/// A correlated reply or an asynchronous observed event.
pub enum Message {
    /// Result of the correlated request identifier.
    Reply {
        /// Correlation identifier of one control request.
        request_id: Ulid,
        /// Named operation success or failure; no automatic retry is implied.
        result: Result<Reply, Failure>,
    },
    /// An asynchronous observed process event.
    Event {
        /// Observed asynchronous process event.
        event: Event,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// Named supervisor refusal or transport-failure category.
pub enum FailureCode {
    /// Admission capability or authorization was refused.
    Unauthorized,
    /// The supplied logic lease is no longer current.
    StaleLease,
    /// The requested process generation is no longer current.
    StaleGeneration,
    /// The supplied writer attachment is no longer current.
    StaleWriter,
    /// The requested process or operation is not retained.
    NotFound,
    /// Request shape or configuration is invalid.
    Invalid,
    /// A documented size or count bound was exceeded.
    Limit,
    /// Admission is temporarily unavailable; inspect uncertain input outcomes before retrying.
    Busy,
    /// The requested process generation has already exited.
    Exited,
    /// An input identifier was reused with conflicting content.
    InputConflict,
    /// A host IO operation failed.
    Io,
    /// The protocol version is unsupported.
    Version,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
/// A named failure code and diagnostic message without an implicit retry policy.
pub struct Failure {
    /// Portable process exit code or named refusal code, according to the containing type.
    pub code: FailureCode,
    /// Human-readable failure detail; consumers should match the named code.
    pub message: String,
}

impl Failure {
    pub(crate) fn new(code: FailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
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
