// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Example `session` plugin: describes the external harness sessions listed
//! in `sessions.json` (an array of `{id, name, harness, cwd?}`) in its data
//! directory; none when the file is absent.
#![allow(clippy::disallowed_methods)] // A plugin process: no UI thread.

use bbt_plugin::session as ss;
use bbt_plugin_sdk::contracts::{AsSession, Base, Session};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};

struct Listed;

impl Base for Listed {
    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-session-example",
            "Example session list",
            "External sessions listed in a local file",
        )
    }
    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        Ok(Self)
    }
}

impl Session for Listed {
    fn describe(
        &mut self,
        _req: ss::DescribeRequest,
        _ctx: &Ctx,
    ) -> Result<ss::DescribeResult, Refusal> {
        let sessions = match std::fs::read("sessions.json") {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| Refusal::new("invalid_sessions", e.to_string()))?,
            Err(_) => Vec::new(),
        };
        Ok(ss::DescribeResult { sessions })
    }
}

fn main() {
    run::<AsSession<Listed>>();
}

// keeps: plugin doctor — this example has no gaps (`testkit::doctor`).
#[cfg(test)]
mod doctor {
    #[test]
    fn the_plugin_doctor_finds_no_gaps() {
        let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let binary = bbt_plugin_sdk::testkit::example(package, "bboxy450-term-session-example");
        bbt_plugin_sdk::testkit::doctor(
            &binary,
            &bbt_plugin_sdk::doctor::Options::package(package),
        );
    }
}
