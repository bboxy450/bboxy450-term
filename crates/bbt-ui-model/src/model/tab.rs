// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::{SeatId, SplitNode, TabId};

#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    pub id: TabId,
    pub name: String,
    pub shortcut: Option<u8>,
    pub tree: SplitNode,
    /// Sidebar insertion order, independent of the spatial tree's leaf order.
    pub seats: Vec<SeatId>,
    pub expanded: bool,
}
impl Tab {
    pub fn new(id: TabId, name: impl Into<String>, tree: SplitNode) -> Self {
        Self {
            id,
            name: name.into(),
            shortcut: None,
            seats: tree.leaves(),
            tree,
            expanded: true,
        }
    }
}
