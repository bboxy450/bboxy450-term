// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Transport parameters and wrap(fields, inner argv) -> argv. No shell or IO.
use crate::Refusal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapRequest {
    pub fields: BTreeMap<String, String>,
    pub command: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapResult {
    pub argv: Vec<String>,
}

pub fn validate_fields(fields: &[Field]) -> Result<(), Refusal> {
    let mut names = BTreeSet::new();
    if fields.len() > 32 {
        return Err(Refusal::new(
            "invalid_transport_fields",
            "at most 32 transport fields",
        ));
    }
    for field in fields {
        if field.name.is_empty()
            || field.name.len() > 64
            || !field
                .name
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
            || !names.insert(&field.name)
            || field.label.is_empty()
            || field.label.len() > 128
            || field.label.chars().any(char::is_control)
            || field
                .default
                .as_ref()
                .is_some_and(|s| s.len() > 4096 || s.contains('\0'))
        {
            return Err(Refusal::new(
                "invalid_transport_fields",
                "transport fields need unique bounded names, labels and defaults without NUL",
            ));
        }
    }
    Ok(())
}

pub fn resolve_fields(
    fields: &[Field],
    values: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, Refusal> {
    validate_fields(fields)?;
    if values
        .keys()
        .any(|key| !fields.iter().any(|f| &f.name == key))
    {
        return Err(Refusal::new(
            "unknown_transport_field",
            "field is not declared by the trusted transport",
        ));
    }
    fields
        .iter()
        .map(|field| {
            let value = values
                .get(&field.name)
                .or(field.default.as_ref())
                .ok_or_else(|| Refusal::new("missing_transport_field", &field.name))?;
            if value.len() > 4096 || value.contains('\0') {
                return Err(Refusal::new("invalid_transport_field", &field.name));
            }
            Ok((field.name.clone(), value.clone()))
        })
        .collect()
}
