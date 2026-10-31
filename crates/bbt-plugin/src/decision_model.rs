// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Typed requests and results of the `decision_model` 1.0 contract
//! (`schemas/decision_model-1.0.schema.json`): pick one named criterion.
//! A decision model chooses; it never executes, plans or keeps state.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `decision_model` 1.0 method names.
pub const METHODS: [&str; 1] = ["choose"];

/// `choose` one of `criteria` (name → description) for `state`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChooseRequest {
    /// Any JSON value describing the situation.
    pub state: Value,
    pub instructions: String,
    /// At least one named choice.
    pub criteria: BTreeMap<String, String>,
}

/// The chosen criterion. `choice` and every `probabilities` key must name a
/// supplied criterion; values lie in `0..=1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChooseResult {
    pub choice: String,
    pub confidence: f64,
    pub probabilities: BTreeMap<String, f64>,
    /// The model (and version) that decided.
    pub model: String,
}
