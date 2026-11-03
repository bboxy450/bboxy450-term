// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Public startup observations; production bbt-ui composition is the native side.
use crate::gallery::{self, Case};
pub fn cases() -> Vec<Case> {
    let mut out = vec![];
    for state in [
        "starting",
        "existing supervisor",
        "starting runtime",
        "verifying history",
        "empty history",
        "attaching",
        "reconnecting",
        "refreshing runtime",
        "failed",
        "long failure",
    ] {
        for density in gallery::DENSITIES {
            for theme in gallery::THEMES {
                out.push(Case {
                    component: "startup".into(),
                    state: state.into(),
                    slug: state.replace(' ', "-"),
                    density: gallery::density_name(density).into(),
                    theme: gallery::theme_name(theme).into(),
                    page: "startup.html".into(),
                    selector: ".card > .body".into(),
                    width: 1246,
                    height: 800,
                    checker: false,
                    fixture: Some(serde_json::json!({"layer":"components"})),
                });
            }
        }
    }
    out
}
