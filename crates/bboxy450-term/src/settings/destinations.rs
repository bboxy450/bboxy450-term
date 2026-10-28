// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Availability is a launch preference, independent of a provider's observed link.
use super::{Patch, Preferences};
use bbt_proto::Where;
use toml_edit::{DocumentMut, Value};

pub fn valid_id(id: &str) -> bool {
    id.len() <= 512
        && !id.chars().any(char::is_control)
        && id
            .strip_prefix("ssh:")
            .or_else(|| id.strip_prefix("cloud:"))
            .is_some_and(|name| !name.is_empty() && name.trim() == name)
}

pub fn id(location: &Where) -> Option<String> {
    match location {
        Where::Local => None,
        Where::Ssh { host } => Some(format!("ssh:{host}")),
        Where::Cloud { provider, .. } => Some(format!("cloud:{provider}")),
    }
}

pub(super) fn read(doc: &DocumentMut, preferences: &mut Preferences) -> Result<(), String> {
    let Some(destinations) = doc.get("destinations") else {
        return Ok(());
    };
    let destinations = destinations
        .as_table_like()
        .ok_or("Settings [destinations] must be a table")?;
    for (id, item) in destinations.iter() {
        if !valid_id(id) {
            return Err("Invalid destination identifier".into());
        }
        let table = item
            .as_table_like()
            .ok_or("Destination settings must be a table")?;
        if let Some(available) = table.get("available") {
            let available = available
                .as_bool()
                .ok_or("Destination availability must be boolean")?;
            if !available {
                preferences.disabled_destinations.insert(id.into());
            }
        }
    }
    Ok(())
}

pub(super) fn write(doc: &mut DocumentMut, patch: &Patch) -> Result<bool, String> {
    let Patch::DestinationAvailable { id, available } = patch else {
        return Ok(false);
    };
    if !valid_id(id) {
        return Err("Invalid destination identifier".into());
    }
    super::roles::replace(
        &mut doc["destinations"][id]["available"],
        Value::from(*available),
    );
    Ok(true)
}

impl Preferences {
    pub(super) fn apply_destination_patch(&mut self, patch: &Patch) -> Result<(), String> {
        if let Patch::DestinationAvailable { id, available } = patch {
            if !valid_id(id) {
                return Err("Invalid destination identifier".into());
            }
            if *available {
                self.disabled_destinations.remove(id);
            } else {
                self.disabled_destinations.insert(id.clone());
            }
        }
        Ok(())
    }
}
