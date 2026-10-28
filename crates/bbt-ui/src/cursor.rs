// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Semantic pointer feedback from retained interaction regions. Hosts map these
//! values to native cursors without putting a window or renderer in the UI crate.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Cursor {
    #[default]
    Default,
    Pointer,
    Text,
    EwResize,
    NsResize,
}
