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
    collections::BTreeSet,
    io::{self, Write},
};
pub struct Writer<W> {
    output: W,
    next_seq: u64,
    native_id: String,
    derived: BTreeSet<EventId>,
}
impl<W: Write> Writer<W> {
    pub fn new(session: &Session, output: W) -> io::Result<Self> {
        if session.source.provider != Provider::Claude {
            return Err(invalid("not a Claude source"));
        }
        Ok(Self {
            output,
            next_seq: 0,
            native_id: session.source.native_id.clone(),
            derived: BTreeSet::new(),
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
            self.derived.clear();
            if let Ok(row) = serde_json::from_slice::<serde_json::Value>(&raw.bytes) {
                let key = row["uuid"].as_str().map(str::to_owned).unwrap_or_default();
                if row["message"]["content"].is_string() {
                    self.derived.insert(EventId::inferred(
                        Provider::Claude,
                        &self.native_id,
                        &key,
                        "block-0",
                    ));
                }
                if let Some(blocks) = row["message"]["content"].as_array() {
                    for index in 0..blocks.len() {
                        self.derived.insert(EventId::inferred(
                            Provider::Claude,
                            &self.native_id,
                            &key,
                            &format!("block-{index}"),
                        ));
                    }
                }
            }
            self.output.write_all(&raw.bytes)?;
            self.output.write_all(b"\n")?;
        } else if event.fidelity != Fidelity::Inferred && !self.derived.contains(&event.id) {
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
