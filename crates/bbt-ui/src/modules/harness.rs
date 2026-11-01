// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Built-in harness presentation. IDs remain launch keys; labels never become argv.
use crate::{self as ui, components as c};

pub const HARNESSES: [&str; 6] = ["claude", "codex", "pi", "omp", "bboxy450-harness", "zsh"];
pub type Launchable = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

/// Every menu is projected from HARNESSES, including remote machine children.
pub fn launchables() -> impl Iterator<Item = Launchable> {
    HARNESSES.into_iter().map(|id| {
        let (label, sub, tile, section) = match id {
            "claude" => ("Claude", "oracle by default", "#D97757", "Suggestions"),
            "codex" => ("Codex", "executor by default", "#1F2937", "Suggestions"),
            "pi" => ("pi", "executor by default", "#7C3AED", "Harnesses"),
            "omp" => ("omp", "executor by default", "#0EA5E9", "Harnesses"),
            "zsh" => ("zsh", "shell by default", "#111827", "Shells"),
            "bboxy450-harness" => (
                "bboxy harness",
                "executor by default",
                "#168BFF",
                "Harnesses",
            ),
            _ => (id, "executor by default", "#111827", "Harnesses"),
        };
        (id, label, sub, tile, section)
    })
}
pub fn label(kind: &str) -> &str {
    if kind == "bboxy450-harness" {
        "bboxy harness"
    } else {
        kind
    }
}
pub fn icon(kind: &str) -> &str {
    match kind {
        "bboxy450-harness" => "bboxy",
        "plugin-login" => "key",
        _ => kind,
    }
}
pub fn tile(kind: &str) -> Option<&'static str> {
    launchables()
        .find(|(id, ..)| *id == kind)
        .map(|(_, _, _, tile, _)| tile)
}
/// The existing identity PNG keeps its colours; SVG marks retain their tint.
/// Host assets own decoding. No UI filesystem access or substitute artwork.
pub fn mark(kind: &str, size: f32, paint: ui::Paint) -> c::Element {
    match icon(kind) {
        "bboxy" => c::Element::Image {
            asset: "bboxy".into(),
            size: ui::Length::Px(size),
            radius: ui::Length::Px(2.),
        },
        name => c::Element::Icon {
            name: name.into(),
            size: ui::Length::Px(size),
            paint,
        },
    }
}
