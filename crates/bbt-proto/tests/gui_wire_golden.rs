// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! #320 cut 0: the GUI wire (ctl-cli §2) pinned byte for byte before dispatch moves to
//! per-noun modules. Every line of `golden/gui_wire.tsv` must decode and re-encode unchanged,
//! and every variant either enum declares must have a line, so a new kind without one fails.
use std::collections::BTreeSet;

use bbt_proto::{GuiAction, GuiReply};
use serde::{Serialize, de::DeserializeOwned};

const GOLDEN: &str = include_str!("golden/gui_wire.tsv");

fn lines(dir: &str) -> Vec<&'static str> {
    GOLDEN
        .lines()
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('\t'))
        .filter(|(d, _)| *d == dir)
        .map(|(_, json)| json)
        .collect()
}

/// The variants serde itself would accept: its unknown-variant error names all of them.
fn declared<T: DeserializeOwned>() -> BTreeSet<String> {
    let error = serde_json::from_str::<T>(r#"{"kind":"__no_such_kind__","body":null}"#)
        .err()
        .expect("an unknown kind must be refused")
        .to_string();
    let (_, list) = error
        .split_once("expected one of ")
        .unwrap_or_else(|| panic!("unknown-kind error no longer lists the kinds: {error}"));
    list.split(", ")
        .map(|kind| kind.split('`').nth(1).unwrap_or(kind).to_owned())
        .collect()
}

fn pinned<T: Serialize + DeserializeOwned>(dir: &str) -> BTreeSet<String> {
    let mut kinds = BTreeSet::new();
    for json in lines(dir) {
        let value: T = serde_json::from_str(json)
            .unwrap_or_else(|error| panic!("{dir} no longer decodes: {error}\n{json}"));
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            json,
            "{dir} re-encodes differently"
        );
        let kind: serde_json::Value = serde_json::from_str(json).unwrap();
        kinds.insert(kind["kind"].as_str().unwrap().to_owned());
    }
    kinds
}

// keeps: decision #320
#[test]
fn every_gui_action_kind_keeps_its_exact_bytes() {
    assert_eq!(pinned::<GuiAction>("action"), declared::<GuiAction>());
}

// keeps: decision #320
#[test]
fn every_gui_reply_kind_keeps_its_exact_bytes() {
    assert_eq!(pinned::<GuiReply>("reply"), declared::<GuiReply>());
}

#[test]
fn seat_controls_require_generation_and_scoped_assignment_without_caller_roots() {
    for json in lines("action")
        .into_iter()
        .filter(|line| line.contains("\"kind\":\"SeatControl\""))
    {
        let mut value: serde_json::Value = serde_json::from_str(json).unwrap();
        value["body"].as_object_mut().unwrap().remove("generation");
        assert!(serde_json::from_value::<GuiAction>(value).is_err());
    }
    let json = lines("action")
        .into_iter()
        .find(|line| line.contains("\"kind\":\"assign_task\""))
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    let mut missing = value.clone();
    missing["body"]["command"]
        .as_object_mut()
        .unwrap()
        .remove("scope");
    assert!(serde_json::from_value::<GuiAction>(missing).is_err());
    for field in ["root_id", "path"] {
        let mut injected = value.clone();
        injected["body"]["command"]["scope"][field] = serde_json::json!("caller root");
        assert!(serde_json::from_value::<GuiAction>(injected).is_err());
    }
}

#[test]
fn optional_swarm_scope_preserves_legacy_bytes_and_accepts_only_host_context_handles() {
    use bbt_proto::gui::SwarmScope;
    let scope = SwarmScope {
        swarm: ulid::Ulid::from(7_u128),
        revision: 0,
    };
    for family in [
        "Knowledge",
        "Task",
        "RecordWrite",
        "KnowledgeCommit",
        "Plugin",
        "KnowledgeControl",
    ] {
        let legacy = lines("action")
            .into_iter()
            .find(|line| serde_json::from_str::<serde_json::Value>(line).unwrap()["kind"] == family)
            .unwrap();
        let parsed: GuiAction = serde_json::from_str(legacy).unwrap();
        assert_eq!(serde_json::to_string(&parsed).unwrap(), legacy);
        let mut scoped: serde_json::Value = serde_json::from_str(legacy).unwrap();
        scoped["body"]["scope"] = serde_json::to_value(scope).unwrap();
        let parsed: GuiAction = serde_json::from_value(scoped.clone()).unwrap();
        match parsed {
            GuiAction::Knowledge {
                scope: Some(value), ..
            }
            | GuiAction::Task {
                scope: Some(value), ..
            }
            | GuiAction::RecordWrite {
                scope: Some(value), ..
            }
            | GuiAction::KnowledgeCommit {
                scope: Some(value), ..
            }
            | GuiAction::Plugin {
                scope: Some(value), ..
            }
            | GuiAction::KnowledgeControl {
                scope: Some(value), ..
            } => assert_eq!(value, scope),
            _ => panic!("{family} did not retain its captured scope"),
        }
        for injected in ["path", "root_id"] {
            let mut invalid = scoped.clone();
            invalid["body"]["scope"][injected] = serde_json::json!("caller-controlled");
            assert!(
                serde_json::from_value::<GuiAction>(invalid).is_err(),
                "{family} accepted {injected}"
            );
        }
    }
}
