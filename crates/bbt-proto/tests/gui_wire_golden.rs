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
