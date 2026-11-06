// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Plugin-system contract models. They ship with the crates that own each
//! contract (feature `model`): bboxy450-plugin (hook policy, host API),
//! bboxy450-term-plugin-sdk (the plugin runtime) and bboxy450-term-mux
//! (activation, connections, hook gate, setup), each with its exported
//! `models/*.tla`/`*.cfg`. This module re-exports them for the project gate
//! and fails when a shipped file drifts from its Rust source.
use super::*;

pub use bbt_mux::models::plugin_activation;

/// A model family: the model, or its broken control.
type Family = fn(bool) -> Model;

/// The crate that ships each family, and its `models/` directory.
fn shipped() -> Vec<(&'static str, Vec<Family>)> {
    vec![
        (
            "crates/bbt-plugin/models",
            bbt_plugin::models::families().to_vec(),
        ),
        (
            "crates/bbt-plugin-sdk/models",
            bbt_plugin_sdk::models::families().to_vec(),
        ),
        // PluginActivation is registered in the shared family list.
        (
            "crates/bbt-mux/models",
            bbt_mux::models::families().to_vec(),
        ),
    ]
}

pub fn cases() -> Vec<(Model, &'static str)> {
    shipped()
        .into_iter()
        .flat_map(|(_, families)| families)
        .filter(|family| family(false).name != "PluginActivation")
        .flat_map(bboxy450_tlc::build::family)
        .map(|(model, expected)| {
            // The positive fence has a fair progress property, but its broken
            // control removes completion authority and violates safety. Keep
            // both properties; the generic temporal-family default is not its
            // expected counterexample kind (verification/corpus.toml).
            let expected = if model.name == "BrokenManagedWorkspaceFence" {
                "safety"
            } else {
                expected
            };
            (model, expected)
        })
        .collect()
}

// keeps: 9e shipped models — every shipped .tla/.cfg equals what its Rust
// builder exports now (BBOXY_UPDATE_MODELS=1 rewrites them).
#[test]
fn shipped_plugin_models_match_their_rust_sources() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let update = std::env::var_os("BBOXY_UPDATE_MODELS").is_some();
    for (dir, families) in shipped() {
        let dir = root.join(dir);
        let mut expected = std::collections::BTreeSet::new();
        for family in families {
            for (model, _) in bboxy450_tlc::build::family(family) {
                for (name, text) in bboxy450_tlc::build::artifacts(&model).unwrap() {
                    let path = dir.join(&name);
                    if update {
                        std::fs::create_dir_all(&dir).unwrap();
                        std::fs::write(&path, &text).unwrap();
                    }
                    let shipped = std::fs::read_to_string(&path).unwrap_or_default();
                    assert!(
                        shipped == text,
                        "{} drifted from its Rust model; run BBOXY_UPDATE_MODELS=1 cargo test -p bboxy450-term --test formal_contracts",
                        path.display()
                    );
                    expected.insert(name);
                }
            }
        }
        let actual: std::collections::BTreeSet<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            actual,
            expected,
            "{} holds exactly the shipped models",
            dir.display()
        );
    }
}
