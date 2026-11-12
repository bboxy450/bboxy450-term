// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! JSONL codecs. Store logs use one complete, newline-terminated batch per line:
//! a crash inside a batch can never publish a prefix of its operations.
use crate::{Action, Op};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    pub batch_id: String,
    pub author: String,
    pub at: u64,
    pub expect_seq: Option<u64>,
    pub stale: bool,
    pub ops: Vec<Op>,
}
impl Batch {
    pub(crate) fn same_request(
        &self,
        author: &str,
        expect_seq: Option<u64>,
        ops: &[Action],
    ) -> bool {
        // The log holds the batch in applied order, which may differ from the
        // listed order; a retry compares the set of actions, not the sequence.
        let mut remaining: Vec<&Action> = self.ops.iter().map(|op| &op.action).collect();
        self.author == author
            && self.expect_seq == expect_seq
            && remaining.len() == ops.len()
            && ops.iter().all(|action| {
                remaining
                    .iter()
                    .position(|r| *r == action)
                    .map(|i| remaining.swap_remove(i))
                    .is_some()
            })
    }
}
#[derive(Debug)]
pub struct Loaded {
    pub batches: Vec<Batch>,
    /// Every unterminated tail is dropped, even if its JSON happens to parse.
    pub dropped_tail_bytes: usize,
}
pub fn decode_batches(bytes: &[u8]) -> io::Result<Loaded> {
    let complete_bytes = bytes.iter().rposition(|b| *b == b'\n').map_or(0, |i| i + 1);
    let mut batches = vec![];
    for (index, line) in bytes[..complete_bytes].split(|b| *b == b'\n').enumerate() {
        if line.is_empty() {
            continue;
        }
        batches.push(serde_json::from_slice(line).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("canvas batch line {}: {e}", index + 1),
            )
        })?);
    }
    Ok(Loaded {
        batches,
        dropped_tail_bytes: bytes.len() - complete_bytes,
    })
}
pub fn encode_batch(batch: &Batch) -> io::Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(batch).map_err(io::Error::other)?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// C0 interchange codec retained for existing fixtures. It is not the store's
/// atomic batch format; sequence/reference validation belongs to replay.
pub fn load_log(reader: impl BufRead) -> io::Result<Vec<Op>> {
    let mut ops = vec![];
    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        ops.push(serde_json::from_str(&line).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("line {}: {e}", index + 1),
            )
        })?);
    }
    Ok(ops)
}
pub fn save_log(ops: &[Op], mut writer: impl Write) -> io::Result<()> {
    for op in ops {
        serde_json::to_writer(&mut writer, op).map_err(io::Error::other)?;
        writer.write_all(b"\n")?;
    }
    Ok(())
}
