// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Example `decision_model` plugin: picks the criterion whose description
//! shares the most words with the instructions and state. Deterministic and
//! offline; a hosted model (for example Jev) would answer the same request.
#![allow(clippy::disallowed_methods)] // A plugin process: no UI thread.

use std::collections::{BTreeMap, BTreeSet};

use bbt_plugin::decision_model as dm;
use bbt_plugin_sdk::contracts::{AsDecisionModel, Base, DecisionModel};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};

struct Overlap;

fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

impl Base for Overlap {
    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-decision_model-example",
            "Example word-overlap chooser",
            "Chooses the criterion that shares the most words with the question",
        )
    }
    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        Ok(Self)
    }
}

impl DecisionModel for Overlap {
    fn choose(&mut self, req: dm::ChooseRequest, _ctx: &Ctx) -> Result<dm::ChooseResult, Refusal> {
        let question = words(&format!("{} {}", req.instructions, req.state));
        // One point for being named, so no criterion has probability zero.
        let scores: BTreeMap<String, f64> = req
            .criteria
            .iter()
            .map(|(name, text)| {
                let shared = words(text).intersection(&question).count();
                (name.clone(), 1. + shared as f64)
            })
            .collect();
        let total: f64 = scores.values().sum();
        let probabilities: BTreeMap<String, f64> = scores
            .iter()
            .map(|(name, score)| (name.clone(), score / total))
            .collect();
        let (choice, confidence) = probabilities
            .iter()
            .max_by(|a, b| a.1.total_cmp(b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(name, p)| (name.clone(), *p))
            .ok_or_else(|| Refusal::new("invalid_arguments", "criteria are empty"))?;
        Ok(dm::ChooseResult {
            choice,
            confidence,
            probabilities,
            model: "example-overlap-1".into(),
        })
    }
}

fn main() {
    run::<AsDecisionModel<Overlap>>();
}
