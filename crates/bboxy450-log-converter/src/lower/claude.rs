// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

//! Streaming verbatim replay of retained Claude records in dense seq order.
//! Inferred boundaries/metadata and block projections share a retained native
//! record. Unsupported non-raw events fail, never silently disappear.
use crate::ir::*;
use std::{
    borrow::Borrow,
    io::{self, Write},
};
pub struct Writer<W> {
    output: W,
    next_seq: u64,
    current_record: Option<RecordId>,
}
impl<W: Write> Writer<W> {
    pub fn new(session: &Session, output: W) -> io::Result<Self> {
        if session.source.provider != Provider::Claude {
            return Err(invalid("not a Claude source"));
        }
        Ok(Self {
            output,
            next_seq: 0,
            current_record: None,
        })
    }
    pub fn push(&mut self, event: &Event) -> io::Result<()> {
        if event.seq != self.next_seq {
            return Err(invalid("events must be dense from zero"));
        }
        if let Some(raw) = &event.raw {
            if raw.provider != Provider::Claude || raw.bytes.contains(&b'\n') {
                return Err(invalid("not one retained Claude line"));
            }
            self.current_record = Some(event.record);
            self.output.write_all(&raw.bytes)?;
            self.output.write_all(b"\n")?;
        } else if self.current_record != Some(event.record)
            && !(event.fidelity == Fidelity::Inferred
                && matches!(
                    event.kind,
                    EventKind::TurnStart { .. } | EventKind::TurnEnd { .. }
                ))
        {
            return Err(invalid("non-raw event needs cross-provider synthesis"));
        }
        self.next_seq += 1;
        Ok(())
    }
    pub fn finish(self) -> W {
        self.output
    }
}
pub fn write<W, I, E>(session: &Session, events: I, output: W) -> io::Result<()>
where
    W: Write,
    I: IntoIterator<Item = E>,
    E: Borrow<Event>,
{
    let mut writer = Writer::new(session, output)?;
    for event in events {
        writer.push(event.borrow())?;
    }
    Ok(())
}
fn invalid(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, s)
}
