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
        subject: None,
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
                subject: None,
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

// keeps: G3 hooks — bus hooks name a public subject pattern, observe after
// publication and never see inboxes or plugin activity.
#[test]
fn bus_hooks_need_a_public_after_subject_and_harness_hooks_take_none() {
    use bbt_plugin::policy::{subject_matches, validate_subject};
    let bus = |subject: Option<&str>, phase| Hook {
        event: "bus".into(),
        phase,
        budget_ms: 1000,
        subject: subject.map(str::to_owned),
    };
    let ok = bus(Some("task/*/status"), Phase::After);
    ok.validate().unwrap();
    assert!(ok.is_bus() && !ok.carries_payload().unwrap());
    assert_eq!(ok.ceiling_ms().unwrap(), 5000);
    bus(Some("swarms/>"), Phase::After).validate().unwrap();
    for (hook, code) in [
        (bus(None, Phase::After), "invalid_hook"),
        (bus(Some("task/*/status"), Phase::Before), "invalid_hook"),
        (bus(Some("inbox/>"), Phase::After), "invalid_subject"),
        (bus(Some("plugin/>"), Phase::After), "invalid_subject"),
        (bus(Some(">"), Phase::After), "invalid_subject"),
        (bus(Some("*/x"), Phase::After), "invalid_subject"),
        (bus(Some("task/>/x"), Phase::After), "invalid_subject"),
        (bus(Some("task//x"), Phase::After), "invalid_subject"),
        (
            Hook {
                event: "PreToolUse".into(),
                phase: Phase::Before,
                budget_ms: 1000,
                subject: Some("task/>".into()),
            },
            "invalid_hook",
        ),
    ] {
        assert_eq!(hook.validate().unwrap_err().code, code, "{hook:?}");
    }
    let pre = Hook {
        event: "pre_tool_use".into(),
        phase: Phase::Before,
        budget_ms: 2000,
        subject: None,
    };
    assert!(pre.matches_event("PreToolUse") && !pre.matches_event("PostToolUse"));
    assert!(
        !ok.matches_event("bus"),
        "bus hooks match subjects, not events"
    );
    assert!(subject_matches("task/*/status", "task/7/status"));
    assert!(!subject_matches("task/*/status", "task/7/status/x"));
    assert!(subject_matches("swarms/>", "swarms/a/b"));
    assert!(!subject_matches("swarms/>", "swarms"));
    validate_subject("plugin/abc/hook/delivered", false).unwrap();
    assert!(validate_subject("task/*", false).is_err());
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
