// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Markdown parsing and retained document composition. Files, navigation,
//! signature verification and renderer execution belong to the host.
pub mod links;
pub mod markdown;
pub mod mermaid;
pub use markdown::{Block, BlockKind, Frontmatter, Inline, Markdown, TableAlign, parse};
