// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Example `event_sink` plugin: appends each signed envelope the host fans
//! out, unchanged, as one JSON line to `events.jsonl` in the plugin's data
//! directory (its working directory), once per envelope `id`. A real sink
//! would forward it to NATS, a webhook or a queue the same way.
#![allow(clippy::disallowed_methods)] // A plugin process: no UI thread.

use std::collections::BTreeSet;
use std::io::Write;

use bbt_plugin::event_sink as es;
use bbt_plugin_sdk::contracts::{AsEventSink, Base, EventSink};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};

struct Sink {
    seen: BTreeSet<String>,
}

impl Base for Sink {
    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-event_sink-example",
            "Example event log",
            "Appends delivered envelopes to a local JSON-lines file",
        )
    }
    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        Ok(Self {
            seen: BTreeSet::new(),
        })
    }
}

impl EventSink for Sink {
    fn deliver(
        &mut self,
        req: es::DeliverRequest,
        _ctx: &Ctx,
    ) -> Result<es::DeliverResult, Refusal> {
        let id = req.envelope.id.clone();
        if !self.seen.insert(id.clone()) {
            return Ok(es::DeliverResult {
                id,
                accepted: true,
                reason: Some("duplicate".into()),
            });
        }
        let line = serde_json::json!({"subject": req.subject, "envelope": req.envelope});
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("events.jsonl")
            .and_then(|mut file| writeln!(file, "{line}"))
            .map_err(|e| Refusal::new("sink_unavailable", e.to_string()))?;
        Ok(es::DeliverResult {
            id,
            accepted: true,
            reason: None,
        })
    }
}

fn main() {
    run::<AsEventSink<Sink>>();
}

// keeps: plugin doctor — this example has no gaps (`testkit::doctor`).
#[cfg(test)]
mod doctor {
    #[test]
    fn the_plugin_doctor_finds_no_gaps() {
        let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let binary = bbt_plugin_sdk::testkit::example(package, "bboxy450-term-event_sink-example");
        bbt_plugin_sdk::testkit::doctor(
            &binary,
            &bbt_plugin_sdk::doctor::Options::package(package),
        );
    }
}
