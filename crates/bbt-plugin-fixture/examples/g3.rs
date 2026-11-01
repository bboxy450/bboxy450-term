// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Build private SDK fixtures for make test-settings-plugins; no account IO.
#![allow(clippy::disallowed_methods)]
use bbt_plugin_fixture::{Kind, Knobs};
use serde_json::json;
fn main() {
    let out = std::path::absolute(
        std::env::args_os()
            .nth(1)
            .expect("fixture output directory"),
    )
    .unwrap();
    std::fs::create_dir_all(&out).unwrap();
    let knobs = Knobs {
        hooks: vec![json!({"event":"SessionStart","phase":"after","budget_ms":800})],
        ..Knobs::of(Kind::SdkHook)
    };
    knobs.install(&out.join("bboxy450-term-repo_graph-reference"));
    Knobs {
        observe: Some("untrusted-executed".into()),
        ..knobs.nonce("untrusted")
    }
    .install(&out.join("bboxy450-term-repo_graph-untrusted"));
}
