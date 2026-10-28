// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxy450_term::{
    grids::Grids,
    hosts,
    model::{self, Model},
};
use bbt_proto::{GuiHost, HostState, Where};
use bbt_ui::modules::link::{LinkState, MachineState};

fn host(location: Where, state: HostState) -> GuiHost {
    GuiHost {
        location,
        state,
        provider: "fixture".into(),
        label: "Build box".into(),
        address: Some("agent@build-box.invalid".into()),
        region: Some("east-test-1".into()),
    }
}
// keeps: spec d1-providers §3
#[test]
fn provider_data_binds_settings_launcher_and_survives_model_rebuilds() {
    let hosts = vec![
        host(
            Where::Ssh {
                host: "build-box".into(),
            },
            HostState::Disconnected {
                reason: "link dropped".into(),
            },
        ),
        host(
            Where::Cloud {
                provider: "nebula".into(),
                host: "east".into(),
            },
            HostState::Connected { rtt_ms: 60 },
        ),
        host(
            Where::Cloud {
                provider: "nebula".into(),
                host: "west".into(),
            },
            HostState::Degraded { rtt_ms: 380 },
        ),
    ];
    let records = hosts::settings(&hosts);
    assert!(
        matches!(&records.machines[0].4,MachineState::Link(LinkState::Disconnected{reason}) if reason=="link dropped")
    );
    assert_eq!(records.machines[0].1, "agent@build-box.invalid");
    assert_eq!(records.cloud.len(), 1);
    assert_eq!(records.cloud[0].0, "nebula");
    assert!(matches!(
        records.cloud[0].4,
        Some(LinkState::Degraded { rtt_ms: 380 })
    ));
    let items = hosts::launcher_items(&hosts);
    let remote = &items[0].children.as_ref().unwrap()[0];
    assert!(remote.down && remote.children.is_none());
    assert_eq!(remote.link.as_ref().unwrap().label(), "link dropped");
    let previous = Model {
        hosts: hosts.clone(),
        ..Default::default()
    };
    let next = model::build(&[], &Grids::new(), &previous);
    assert_eq!(next.hosts, hosts);
    assert_eq!(hosts::sidebar_cards(&hosts).len(), 2);
}
