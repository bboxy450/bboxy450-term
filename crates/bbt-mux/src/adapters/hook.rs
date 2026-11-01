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
        let (path, bytes) = frame(seat, crate::seat_socket::Action::Hook { event })?;
        send_socket(std::path::Path::new(&path), &bytes, until)
    }
    fn decide(
        &self,
        seat: Ulid,
        event: ShimEvent,
        decide_by: std::time::Duration,
        until: Instant,
    ) -> Result<Decision> {
        let action = crate::seat_socket::Action::HookDecide {
            event,
            wait_ms: decide_by.as_millis() as u64,
        };
        let (path, bytes) = frame(seat, action)?;
        let reply = exchange_socket(std::path::Path::new(&path), &bytes, until)?;
        decision(&serde_json::from_slice(&reply)?)
    }
}
/// The seat socket's answer to `HookDecide`.
pub fn decision(reply: &serde_json::Value) -> Result<Decision> {
    match (reply["kind"].as_str(), reply["decision"].as_str()) {
        (Some("HookDecided"), Some("allow")) => Ok(Decision::Allow),
        (Some("HookDecided"), Some("deny")) => Ok(Decision::Deny(
            reply["reason"]
                .as_str()
                .unwrap_or("denied by a plugin hook")
                .to_owned(),
        )),
        _ => Err(invalid(format!(
            "hook decision refused: {}",
            reply["reason"].as_str().unwrap_or("malformed reply")
        ))),
    }
}
fn frame(seat: Ulid, action: crate::seat_socket::Action) -> Result<(String, Vec<u8>)> {
    let path = std::env::var("BBOXY_SOCK")?;
    let capability = bboxy450_harness_wake_up::capability::from_env()?;
    let generation = std::env::var("BBOXY_GENERATION")?.parse()?;
    let request = crate::seat_socket::Request {
        version: 1,
        seat,
        generation,
        capability,
        action,
    };
    let mut bytes = serde_json::to_vec(&request)?;
    bytes.push(b'\n');
    Ok((path, bytes))
}
/// Private binary dispatch; parsing, OSC and the total deadline are the kit's.
/// Returns the hook's exit code (2 is a plugin veto).
pub fn run_cli(args: &[String]) -> i32 {
    // Validate admission at every event start, even when the kit can emit OSC
    // without falling back to the socket. Listing static hooks needs no seat.
    if !args.first().is_some_and(|arg| arg == "--list") {
        let started = Instant::now();
        if let Err(error) = bboxy450_harness_wake_up::capability::from_env() {
            SeatSocketSink.failed(args, &error.to_string(), started.elapsed());
            return 0; // Keep the documented hook fail-open policy.
        }
    }
    bboxy450_harness_wake_up::hook::run_cli(args, &SeatSocketSink)
}
