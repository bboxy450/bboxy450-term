// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Link framing (ICD §3): u32 big-endian length, then one UTF-8 JSON object.
//! Frames are additive-only: an unknown `type` decodes to [Frame::Unknown] so
//! an older peer skips it. IR envelopes travel as the author's exact bytes
//! (Log IR v1 law E6: only the author serializes), as a JSON string holding
//! the envelope's exact text; the link never parses or re-serializes it.
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};

/// Link protocol major version. A peer refuses a different major.
pub const VERSION: u16 = 1;
/// Written by `serve` before any frame. A remote login may print banners or
/// MOTD text on stdout first; the client skips every byte before this line.
/// Inspired by herdr-core's banner-tolerant `hc connect` handshake
/// (Apache-2.0, https://github.com/herdrdev/herdr-core, machines.rs).
pub const MAGIC: &[u8] = b"bboxy450-remote 1\n";
/// Bytes a client tolerates before MAGIC.
pub const MAX_BANNER: usize = 64 * 1024;

/// Skip banner bytes until MAGIC; error if it never appears within MAX_BANNER.
pub fn expect_magic(r: &mut impl Read) -> io::Result<()> {
    let mut window: Vec<u8> = Vec::with_capacity(MAGIC.len());
    let mut byte = [0u8; 1];
    for _ in 0..MAX_BANNER + MAGIC.len() {
        r.read_exact(&mut byte)?;
        window.push(byte[0]);
        if window.len() > MAGIC.len() {
            window.remove(0);
        }
        if window == MAGIC {
            return Ok(());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "no bboxy450-remote handshake within the banner limit",
    ))
}

/// Largest frame body; the IR caps one envelope at 256 KiB.
pub const MAX_FRAME: usize = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Frame {
    /// Local → remote, first frame on every connection.
    Hello {
        version: u16,
        swarm_id: String,
        host: String,
    },
    /// Remote → local reply to Hello.
    Welcome { version: u16, supervisor: String },
    /// Local → remote: start (or adopt) one harness ACP session.
    Spawn {
        session: String,
        profile: Option<String>,
        cwd: String,
    },
    /// Local → remote: one IR OpEnvelope for a session, keyed by message_id.
    Op {
        session: String,
        message_id: String,
        envelope: String,
    },
    /// Remote → local: message_id is durably applied (sent again for duplicates).
    Applied { message_id: String },
    /// Remote → local: one IR Envelope with its session epoch and seq.
    Event {
        session: String,
        epoch: u64,
        seq: u64,
        envelope: String,
    },
    /// Local → remote: replay a session's events after (epoch, seq).
    Resume {
        session: String,
        epoch: u64,
        after_seq: u64,
    },
    /// Local → remote: close one session.
    Close { session: String },
    /// Remote → local: the epoch changed or the tail is gone; clear the view,
    /// then accept events of `epoch` from `from_seq`.
    Reset {
        session: String,
        epoch: u64,
        from_seq: u64,
    },
    /// Remote → local: an op was not journalled or applied (unknown session).
    Refused { message_id: String, reason: String },
    /// Remote → local keepalive; a link that carries no bytes for the
    /// dead-line timeout is treated as gone (time-based liveness counts any
    /// bytes, so a big frame on a slow link is never mistaken for a dead one).
    Ping,
    /// A frame type this build does not know; skipped, never an error.
    #[serde(other)]
    Unknown,
}

pub fn write(w: &mut impl Write, frame: &Frame) -> io::Result<()> {
    let body = serde_json::to_vec(frame)?;
    if body.len() > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "frame exceeds MAX_FRAME",
        ));
    }
    w.write_all(&(body.len() as u32).to_be_bytes())?;
    w.write_all(&body)?;
    w.flush()
}

/// Read one frame; `Ok(None)` on a clean end of stream between frames.
pub fn read(r: &mut impl Read) -> io::Result<Option<Frame>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame exceeds MAX_FRAME",
        ));
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_round_trip_and_keep_ir_bytes_exact() {
        let raw = r#"{"v":1, "op_id":"x","op":{"kind":"user_turn"}}"#;
        let frame = Frame::Op {
            session: "s".into(),
            message_id: "m1".into(),
            envelope: raw.into(),
        };
        let mut wire = Vec::new();
        write(&mut wire, &frame).unwrap();
        let back = read(&mut wire.as_slice()).unwrap().unwrap();
        let Frame::Op {
            envelope,
            message_id,
            ..
        } = back
        else {
            panic!("{back:?}")
        };
        assert_eq!(envelope, raw, "author bytes, spacing included");
        assert_eq!(message_id, "m1");
        assert!(read(&mut [].as_slice()).unwrap().is_none());
    }
    #[test]
    fn unknown_frames_are_skipped_and_oversize_is_refused() {
        let body = br#"{"type":"from_the_future","x":1}"#;
        let mut wire = (body.len() as u32).to_be_bytes().to_vec();
        wire.extend_from_slice(body);
        assert!(matches!(
            read(&mut wire.as_slice()).unwrap(),
            Some(Frame::Unknown)
        ));
        let huge = ((MAX_FRAME + 1) as u32).to_be_bytes();
        assert!(read(&mut huge.as_slice()).is_err());
    }
}
