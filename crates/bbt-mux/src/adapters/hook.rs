// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Only the authenticated seat-socket fallback wire belongs to mux.
use crate::{
    Ulid,
    store::{Result, invalid},
};
pub use bboxy450_harness_wake_up::hook::*;
use std::time::Instant;
struct SeatSocketSink;
impl HookSink for SeatSocketSink {
    fn failed(&self, args: &[String], reason: &str, elapsed: std::time::Duration) {
        let harness = args
            .windows(2)
            .find(|p| p[0] == "--agent")
            .map_or("unknown", |p| p[1].as_str());
        let mut row = bbt_rt::error_journal::Failure::new("hook", "bboxy_shim", reason)
            .harness(harness)
            .duration(elapsed);
        if let Ok(seat) = std::env::var("BBOXY_SEAT_ID") {
            row = row.seat(seat);
        }
        bbt_rt::error_journal::report(row);
    }
    fn send(&self, seat: Ulid, event: ShimEvent, until: Instant) -> Result<()> {
        let path = std::env::var("BBOXY_SOCK")?;
        let cap = std::env::var("BBOXY_CAPABILITY")?;
        if cap.len() != 64 || !cap.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid("invalid hook capability"));
        }
        let mut capability = [0; 32];
        for (i, byte) in capability.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&cap[i * 2..i * 2 + 2], 16)?;
        }
        let generation = std::env::var("BBOXY_GENERATION")?.parse()?;
        let request = crate::seat_socket::Request {
            version: 1,
            seat,
            generation,
            capability,
            action: crate::seat_socket::Action::Hook { event },
        };
        let mut bytes = serde_json::to_vec(&request)?;
        bytes.push(b'\n');
        send_socket(std::path::Path::new(&path), &bytes, until)
    }
}
/// Private binary dispatch; parsing, OSC and the total deadline are the kit's.
pub fn run_cli(args: &[String]) {
    bboxy450_harness_wake_up::hook::run_cli(args, &SeatSocketSink);
}
