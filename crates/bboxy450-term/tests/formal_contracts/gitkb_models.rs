// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The GitKB plugin's models (index freshness, paging, attribution). Their
//! Rust source ships in the plugin package (`plugins/gitkb/models.rs`, beside
//! its exported `models/*.tla`/`*.cfg`); this module registers them with the
//! gate and fails when a shipped file drifts from that source.
use super::*;

mod gitkb {
    include!("../../../../plugins/gitkb/models.rs");
}

pub(super) fn cases() -> Vec<(Model, &'static str)> {
    gitkb::families()
        .into_iter()
        .flat_map(bboxy450_tlc::build::family)
        .collect()
}

// keeps: plugin doctor models.present — plugins/gitkb/models holds exactly
// what its Rust source exports now (BBOXY_UPDATE_MODELS=1 rewrites it).
#[test]
fn shipped_gitkb_models_match_their_rust_source() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugins/gitkb/models");
    let update = std::env::var_os("BBOXY_UPDATE_MODELS").is_some();
    let mut expected = std::collections::BTreeSet::new();
    for (model, _) in cases() {
        for (name, text) in bboxy450_tlc::build::artifacts(&model).unwrap() {
            let path = dir.join(&name);
            if update {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(&path, &text).unwrap();
            }
            assert!(
                std::fs::read_to_string(&path).unwrap_or_default() == text,
                "{} drifted from plugins/gitkb/models.rs; run BBOXY_UPDATE_MODELS=1 cargo test -p bboxy450-term --test it formal_contracts::shipped_gitkb",
                path.display()
            );
            expected.insert(name);
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
