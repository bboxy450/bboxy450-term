// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Runtime of [crate::resume_model::RemoteResume]: the remote [SessionLog]
//! (bounded, current epoch only) and the local [Cursor]. Produce = push,
//! Deliver = Cursor::accept (Apply shows the event), RemoteRestart = a new
//! server epoch, ApplyReset = Cursor::reset. The model abstracts sequence
//! numbers to the first two events of an epoch; the runtime uses u64 seqs and
//! answers a gap or an unknown epoch with `reset` plus the retained tail.
use crate::frame::Frame;
use std::collections::VecDeque;

/// Events retained per session for replay.
pub const RETAINED: usize = 4096;

pub struct SessionLog {
    session: String,
    epoch: u64,
    next: u64,
    events: VecDeque<(u64, String)>,
}

impl SessionLog {
    pub fn new(session: &str, epoch: u64) -> Self {
        Self { session: session.to_owned(), epoch, next: 1, events: VecDeque::new() }
    }
    /// Append one event; returns its frame.
    pub fn push(&mut self, envelope: String) -> Frame {
        let seq = self.next;
        self.next += 1;
        if self.events.len() == RETAINED {
            self.events.pop_front();
        }
        self.events.push_back((seq, envelope.clone()));
        Frame::Event { session: self.session.clone(), epoch: self.epoch, seq, envelope }
    }
    /// Frames that bring a cursor at (epoch, after_seq) up to date.
    pub fn replay(&self, epoch: u64, after_seq: u64) -> Vec<Frame> {
        let first = self.events.front().map_or(self.next, |(seq, _)| *seq);
        let mut frames = Vec::new();
        let from = if epoch == self.epoch && after_seq + 1 >= first && after_seq < self.next {
            after_seq + 1
        } else {
            frames.push(Frame::Reset { session: self.session.clone(), epoch: self.epoch, from_seq: first });
            first
        };
        frames.extend(self.events.iter().filter(|(seq, _)| *seq >= from).map(|(seq, envelope)| Frame::Event {
            session: self.session.clone(),
            epoch: self.epoch,
            seq: *seq,
            envelope: envelope.clone(),
        }));
        frames
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Accept {
    /// Show this event; the cursor advanced.
    Apply,
    /// Already shown; drop it.
    Duplicate,
    /// Events are missing: send `resume` from the cursor.
    Gap,
    /// From another epoch: drop it and wait for `reset`.
    Stale,
}

/// Local view position for one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub epoch: u64,
    /// Last shown seq (0 before the first event).
    pub seq: u64,
    reset_pending: bool,
}

impl Cursor {
    /// A view that has not seen this session yet; the first frame must be a reset.
    pub fn new() -> Self {
        Self { epoch: 0, seq: 0, reset_pending: true }
    }
    /// The view was cleared for `epoch`; accept from `from_seq`.
    pub fn reset(&mut self, epoch: u64, from_seq: u64) {
        self.epoch = epoch;
        self.seq = from_seq.saturating_sub(1);
        self.reset_pending = false;
    }
    pub fn accept(&mut self, epoch: u64, seq: u64) -> Accept {
        if self.reset_pending || epoch != self.epoch {
            return Accept::Stale;
        }
        match seq.cmp(&(self.seq + 1)) {
            std::cmp::Ordering::Less => Accept::Duplicate,
            std::cmp::Ordering::Greater => Accept::Gap,
            std::cmp::Ordering::Equal => {
                self.seq = seq;
                Accept::Apply
            }
        }
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}
