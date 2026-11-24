// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Chromium for bboxy450: our defaults ([`defaults`]), a Chrome DevTools
//! Protocol client ([`cdp`]) and, behind the `engine` feature, the CEF glue
//! that applies one and carries the other.
//!
//! Agents drive the browser over CDP only; nothing here opens a
//! remote-debugging port.
pub mod cdp;
pub mod defaults;
#[cfg(feature = "engine")]
pub mod engine;
