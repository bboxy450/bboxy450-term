// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Network-free transport example: carry fields as literal argv via env.
#![allow(clippy::disallowed_methods)] // An isolated fixture plugin process.
use bbt_plugin::transport::{WrapRequest, WrapResult};
use bbt_plugin_sdk::contracts::{AsTransport, Base, Transport};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};
struct Local;
impl Base for Local {
    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-transport-example",
            "Local wrapper",
            "Network-free SDK transport fixture",
        )
        .transport_field("prefix", "Prefix", Some("wrapped"))
        .transport_field("delay_ms", "Delay (ms)", Some("0"))
    }
    fn initialize(_: Init, _: &Ctx) -> Result<Self, Refusal> {
        Ok(Self)
    }
}
impl Transport for Local {
    fn wrap(&mut self, req: WrapRequest, _: &Ctx) -> Result<WrapResult, Refusal> {
        let delay = req
            .fields
            .get("delay_ms")
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0)
            .min(5000);
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        let mut argv = vec![
            "/usr/bin/env".into(),
            format!(
                "FIXTURE_PREFIX={}",
                req.fields.get("prefix").map_or("wrapped", String::as_str)
            ),
        ];
        argv.extend(req.command);
        Ok(WrapResult { argv })
    }
}
fn main() {
    run::<AsTransport<Local>>()
}
