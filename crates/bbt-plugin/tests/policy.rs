// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_plugin::{
    contracts::Contract,
    policy::{FailurePolicy, Hook, Phase, RoleClass, Tier, allowed_env_name, filter_environment},
};

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn contract_authority_is_fixed_and_hooks_raise_exposure() {
    for c in Contract::ALL {
        assert_eq!(
            c.tier(),
            match c {
                Contract::RepoGraph => Tier::Read,
                Contract::Session | Contract::Provider => Tier::Transport,
                _ => Tier::Export,
            }
        );
    }
    assert_eq!(
        Contract::TaskSystem.role("accept").unwrap(),
        RoleClass::Oracle
    );
    assert_eq!(
        Contract::TaskSystem.role("report").unwrap(),
        RoleClass::Worker
    );
    assert_eq!(
        Contract::DataSource.wire_name("fetch").unwrap(),
        "data_source_fetch"
    );
    assert!(Contract::TaskSystem.role("get").is_err());
    assert_eq!(
        serde_json::to_string(&FailurePolicy::FailClosed).unwrap(),
        "\"fail-closed\""
    );
    let prompt = Hook {
        event: "UserPromptSubmit".into(),
        phase: Phase::Before,
        budget_ms: 5000,
    };
    assert!(prompt.carries_payload().unwrap());
    prompt.validate().unwrap();
    for (event, before, after) in [
        ("PreToolUse", 2000, 5000),
        ("PostToolUse", 2000, 5000),
        ("PermissionRequest", 2000, 5000),
        ("SessionEnd", 1000, 1000),
        ("Interrupt", 1000, 1000),
        ("SessionStart", 5000, 5000),
        ("SeatState", 5000, 5000),
    ] {
        for (phase, ceiling) in [(Phase::Before, before), (Phase::After, after)] {
            let hook = Hook {
                event: event.into(),
                phase,
                budget_ms: ceiling,
            };
            hook.validate().unwrap();
            assert!(
                Hook {
                    budget_ms: ceiling + 1,
                    ..hook
                }
                .validate()
                .is_err()
            );
        }
    }
}

// keeps: decision #278 — concrete contracts, additive wire and fixed host approval
#[test]
fn credential_names_lose_even_inside_the_environment_allowlist() {
    let pairs = [
        ("PATH", "bin"),
        ("HOME", "fixture"),
        ("LANG", "C"),
        ("LC_ALL", "C"),
        ("TMPDIR", "tmp"),
        ("TERM", "xterm"),
        ("LC_API_KEY", "secret"),
        ("LC_token", "secret"),
        ("LC_SECRET", "secret"),
        ("ICQ_SOCK", "socket"),
        ("BBOXY_CAPABILITY", "capability"),
        ("SSH_AUTH_SOCK", "socket"),
        ("AWS_ACCESS_KEY_ID", "secret"),
        ("RANDOM_SETTING", "ignored"),
    ];
    let filtered = filter_environment(pairs);
    assert_eq!(filtered.len(), 6);
    assert!(filtered.keys().all(|key| allowed_env_name(key)));
    for bad in [
        "LC_API_KEY",
        "LC_token",
        "LC_SECRET",
        "ICQ_SOCK",
        "BBOXY_CAPABILITY",
        "SSH_AUTH_SOCK",
        "AWS_ACCESS_KEY_ID",
        "RANDOM_SETTING",
    ] {
        assert!(!filtered.contains_key(bad));
    }
}
