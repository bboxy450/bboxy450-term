// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_plugin::{
    contracts::Contract,
    policy::Tier,
    registry::{Activation, Approval, Candidate, Entry, Scope, State},
    shape::PluginName,
};

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn approval_uses_the_fixed_primary_contract_tier_and_grants_nothing_extra() {
    for system in Contract::ALL {
        let name = PluginName {
            system,
            provider: "reference".into(),
        };
        let approval = Approval::minimum(&name);
        assert_eq!(approval.tier, system.tier());
        assert!(approval.grants.payload_events.is_empty());
        assert!(approval.grants.block.is_none());
    }
    assert_eq!(
        Approval::minimum(&PluginName {
            system: Contract::RepoGraph,
            provider: "reference".into()
        })
        .tier,
        Tier::Read
    );
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn discovery_and_a_desired_enable_flag_do_not_imply_account_trust() {
    let name = PluginName {
        system: Contract::DataSource,
        provider: "reference".into(),
    };
    let candidate = Candidate {
        id: "candidate".into(),
        source: "/plugins/reference".into(),
        scope: Scope::Repository,
        name: Some(name.clone()),
        sha256: Some("a".repeat(64)),
        refusal: None,
    };
    let mut entry = Entry {
        candidate: candidate.clone(),
        activation: None,
        trust: None,
        conformance: None,
        measurements: Vec::new(),
        setup: None,
    };
    assert_eq!(entry.state(), State::Untrusted);
    entry.activation = Some(Activation {
        candidate: candidate.id,
        source: candidate.source,
        scope: candidate.scope,
        name,
        sha256: candidate.sha256.unwrap(),
        generation: 1,
        desired_enabled: true,
        needs_retrust: false,
        inspection_started: false,
        inspection: None,
    });
    assert_eq!(entry.state(), State::Untrusted);
    entry.activation.as_mut().unwrap().needs_retrust = true;
    assert_eq!(entry.state(), State::NeedsRetrust);
    let serialized = serde_json::to_vec(&entry).unwrap();
    assert!(!String::from_utf8_lossy(&serialized).contains("conformance"));
    assert_eq!(serde_json::from_slice::<Entry>(&serialized).unwrap(), entry);
}
