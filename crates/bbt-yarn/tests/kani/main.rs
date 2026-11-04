// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Inspired by Kani's inductive harnesses and automatic private-struct generation:
// https://model-checking.github.io/kani/reference/experimental/autoharness.html
#![cfg(all(feature = "macros", feature = "model"))]
#![allow(unexpected_cfgs)]

// Reuse the actual good/broken declarations and their native witnesses verbatim.
#[path = "../protocol.rs"]
mod declarations;

#[cfg(kani)]
mod proofs;
