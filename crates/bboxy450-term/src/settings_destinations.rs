// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Launcher availability never changes the provider's observed connection state.
//! Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_proto::GuiHost;
use bbt_ui::modules::launcher::Item;
use std::collections::BTreeSet;

pub(super) fn filter(items: &mut Vec<Item>, disabled: &BTreeSet<String>, hosts: &[GuiHost]) {
    let blocked: BTreeSet<_> = hosts
        .iter()
        .filter(|host| {
            bboxy450_term::settings::destinations::id(&host.location)
                .is_some_and(|id| disabled.contains(&id))
        })
        .map(|host| bboxy450_term::hosts::key(&host.location))
        .collect();
    fn prune(items: &mut Vec<Item>, blocked: &BTreeSet<String>) {
        items.retain(|item| !blocked.contains(&item.id));
        for item in items.iter_mut() {
            if let Some(children) = &mut item.children {
                prune(children, blocked);
            }
        }
        items.retain(|item| {
            !matches!(item.id.as_str(), "ssh" | "cloud")
                || item
                    .children
                    .as_ref()
                    .is_none_or(|children| !children.is_empty())
        });
    }
    prune(items, &blocked);
}
