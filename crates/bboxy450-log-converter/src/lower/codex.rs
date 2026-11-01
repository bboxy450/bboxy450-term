// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-log-converter: https://github.com/bboxy450/bboxy450-log-converter/blob/17ef2a173d7873c1c881544fd69f40245e0b2fa9/src/lower/codex.rs
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Streaming same-provider replay. Cross-provider synthesis belongs to P6.
use crate::ir::{Event, Provider, RecordId, Session};
use std::{
    borrow::Borrow,
    io::{self, Write},
};

/// Stream consecutive record groups in dense seq order. Each group must own
/// exactly one raw line. Projections never cause the native line to be repeated.
/// No client file or database is opened here; P6 owns non-raw synthesis.
pub fn write<W, I, E>(session: &Session, events: I, mut output: W) -> io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = E>,
    E: Borrow<Event>,
{
    if session.source.provider != Provider::Codex {
        return Err(invalid("not a Codex source session"));
    }
    let mut first = true;
    let mut record: Option<RecordId> = None;
    let mut has_raw = false;
    for (seq, event) in events.into_iter().enumerate() {
        let event = event.borrow();
        if event.seq != seq as u64 {
            return Err(invalid("events must be in dense seq order from zero"));
        }
        if record != Some(event.record) {
            if record.is_some() && !has_raw {
                return Err(invalid("record needs the P6 synthesis path"));
            }
            record = Some(event.record);
            has_raw = false;
        }
        let Some(raw) = &event.raw else {
            continue;
        };
        if has_raw {
            return Err(invalid("duplicate raw ownership within one record"));
        }
        if raw.provider != Provider::Codex || raw.bytes.contains(&b'\n') {
            return Err(invalid("not one retained Codex native line"));
        }
        if first {
            let value: serde_json::Value = serde_json::from_slice(&raw.bytes)
                .map_err(|_| invalid("first record is not session_meta"))?;
            if value["type"] != "session_meta"
                || value["payload"]["id"]
                    .as_str()
                    .or_else(|| value["payload"]["session_id"].as_str())
                    != Some(&session.source.native_id)
            {
                return Err(invalid("first record must be this session's session_meta"));
            }
            first = false;
        }
        output.write_all(&raw.bytes)?;
        output.write_all(b"\n")?;
        has_raw = true;
    }
    if first || !has_raw {
        return Err(invalid(
            "every record needs retained raw and session_meta first",
        ));
    }
    Ok(())
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
