// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Example `provider` plugin: describes the SSH hosts named in `hosts.txt`
//! (one `name address` per line) in its data directory; none when absent.
#![allow(clippy::disallowed_methods)] // A plugin process: no UI thread.

use bbt_plugin::provider as pv;
use bbt_plugin_sdk::contracts::{AsProvider, Base, Provider};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};

struct Hosts;

impl Base for Hosts {
    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-provider-example",
            "Example host list",
            "SSH hosts listed in a local file",
        )
    }
    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        Ok(Self)
    }
}

impl Provider for Hosts {
    fn describe(
        &mut self,
        _req: pv::DescribeRequest,
        _ctx: &Ctx,
    ) -> Result<pv::DescribeResult, Refusal> {
        let text = std::fs::read_to_string("hosts.txt").unwrap_or_default();
        let hosts = text
            .lines()
            .filter_map(|line| line.split_once(' '))
            .map(|(name, address)| pv::Host {
                id: format!("ssh:{name}"),
                name: name.into(),
                kind: pv::HostKind::Ssh,
                address: address.trim().into(),
            })
            .collect();
        Ok(pv::DescribeResult { hosts })
    }
}

fn main() {
    run::<AsProvider<Hosts>>();
}
