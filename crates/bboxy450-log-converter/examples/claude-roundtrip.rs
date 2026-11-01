// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

//! Private native-probe adapter: parse/lower without touching client state.
use bboxy450_log_converter::{ir::SessionId, lower::claude::Writer, parse::claude::Parser};
use std::{
    io::{self, BufReader, Write},
    os::unix::fs::OpenOptionsExt,
};
#[allow(clippy::disallowed_methods)] // Standalone converter process.
fn main() -> io::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let source = args
        .next()
        .ok_or_else(|| io::Error::other("source required"))?;
    let target = args
        .next()
        .ok_or_else(|| io::Error::other("target required"))?;
    let mut parser = Parser::new(
        BufReader::new(std::fs::File::open(source)?),
        SessionId(ulid::Ulid::from(1u128)),
    )?;
    let first = parser.next_record()?;
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(target)?;
    let mut writer = Writer::new(parser.session(), std::io::BufWriter::new(output))?;
    if let Some(events) = first {
        for event in events {
            writer.push(&event)?;
        }
    }
    while let Some(events) = parser.next_record()? {
        for event in events {
            writer.push(&event)?;
        }
    }
    while let Some(events) = parser.finish_tail()? {
        for event in events {
            writer.push(&event)?;
        }
    }
    writer.finish().flush()
}
