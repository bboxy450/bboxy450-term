// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A genuine SDK-served hook which attempts ten seconds of work. The host
//! must enforce the declared budget, record the overrun and keep UI responsive.
use crate::Knobs;
use bbt_plugin_sdk::{Contract, Ctx, HookEvent, HookReply, Init, Manifest, Plugin, Refusal};
use serde_json::{Value, json};
use std::{fs::OpenOptions, io::Write, sync::OnceLock, time::Duration};

static KNOBS: OnceLock<Knobs> = OnceLock::new();
struct HookFixture {
    data_dir: std::path::PathBuf,
}

pub fn main(knobs: Knobs) {
    KNOBS.set(knobs).expect("one fixture configuration");
    bbt_plugin_sdk::run::<HookFixture>();
}

impl Plugin for HookFixture {
    fn manifest() -> Manifest {
        if let Some(path) = &KNOBS.get().unwrap().observe {
            let mut marker = OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .unwrap();
            writeln!(marker, "manifest").unwrap();
        }
        let mut manifest = Manifest::new(
            "bboxy450-term-repo_graph-reference",
            "G3 slow hook",
            "SDK fixture: hook attempts 10 s of work",
        )
        .contract(Contract::RepoGraph)
        .trace();
        for hook in &KNOBS.get().unwrap().hooks {
            let hook: bbt_plugin_sdk::Hook = serde_json::from_value(hook.clone()).unwrap();
            manifest = manifest.hook(&hook.event, hook.phase, hook.budget_ms);
        }
        manifest
    }
    fn initialize(_: Init, _: &Ctx) -> Result<Self, Refusal> {
        Ok(Self {
            data_dir: std::env::current_dir()
                .map_err(|e| Refusal::new("fixture_data_dir", e.to_string()))?,
        })
    }
    fn call(
        &mut self,
        contract: Contract,
        method: &str,
        args: Value,
        _: &Ctx,
    ) -> Result<Value, Refusal> {
        match (contract, method) {
            (Contract::RepoGraph, "describe") => Ok(json!({"repos":[],"edges":[]})),
            (Contract::RepoGraph, "changes_since") => {
                Ok(json!({"changed":[],"next":args["cursor"]}))
            }
            _ => Err(Refusal::new("unknown_method", method)),
        }
    }
    fn on_hook(&mut self, event: &HookEvent, ctx: &Ctx) -> Result<HookReply, Refusal> {
        let knobs = KNOBS.get().unwrap();
        let slow = knobs
            .delay_file
            .as_ref()
            .is_none_or(|path| std::path::Path::new(path).exists());
        {
            let path = self.data_dir.join("hooks.jsonl");
            let mut log = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| Refusal::new("fixture_log", e.to_string()))?;
            writeln!(
                log,
                "{}",
                json!({"event":event.event,"attempted_ms":if slow {10_000} else {0}})
            )
            .map_err(|e| Refusal::new("fixture_log", e.to_string()))?;
        }
        let _span = ctx.span("hook work");
        if slow {
            // Deliberately ignore cooperative cancellation: G3 proves the
            // host's deadline against a genuinely over-budget SDK plugin.
            std::thread::sleep(Duration::from_secs(10));
        }
        Ok(HookReply::Allow)
    }
}
