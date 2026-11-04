// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Example `task_system` plugin: an in-memory task board. A task path the
//! board has not seen opens a new task. Every transition stores the host's
//! `signed_record` and returns it byte for byte; the host, not the plugin,
//! decides who may make a transition.
//!
//! Build: `cargo build -p bboxy450-term-plugin-sdk --example bboxy450-term-task_system-example`.
#![allow(clippy::disallowed_methods)] // A plugin process: no UI thread.

use std::collections::BTreeMap;

use bbt_plugin::task_system as ts;
use bbt_plugin_sdk::contracts::{AsTaskSystem, Base, TaskSystem};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};
use serde_json::{Map, json};

struct Board {
    tasks: BTreeMap<String, ts::TaskRecord>,
    revision: u64,
}

// A ULID-shaped ID derived from the path (Crockford base32, first symbol 0-7).
fn task_id(path: &str) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut hash: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for byte in path.bytes() {
        hash ^= u128::from(byte);
        hash = hash.wrapping_mul(0x0000_0000_0100_0000_0000_0000_0000_013b);
    }
    (0..26)
        .rev()
        .map(|i| ALPHABET[((hash >> (5 * i)) & if i == 25 { 7 } else { 31 }) as usize] as char)
        .collect()
}

impl Board {
    fn task(&mut self, path: &str) -> &mut ts::TaskRecord {
        let number = self.tasks.len() as u64 + 1;
        self.tasks
            .entry(path.to_owned())
            .or_insert_with(|| ts::TaskRecord {
                id: task_id(path),
                number: Some(number),
                external_id: None,
                path: path.to_owned(),
                metadata: Map::from_iter([("status".into(), json!("open"))]),
                body: String::new(),
                body_truncated: None,
                etag: "0".into(),
                signed_record: None,
            })
    }

    // Apply one host-signed transition and return the stored task.
    fn transition(
        &mut self,
        path: &str,
        signed_record: &str,
        change: impl FnOnce(&mut Map<String, serde_json::Value>),
    ) -> ts::TaskWriteResult {
        self.revision += 1;
        let revision = self.revision;
        let task = self.task(path);
        change(&mut task.metadata);
        task.etag = revision.to_string();
        task.signed_record = Some(signed_record.to_owned());
        task.clone()
    }
}

impl Base for Board {
    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-task_system-example",
            "Example task board",
            "In-memory tasks for the SDK guide",
        )
    }
    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        Ok(Self {
            tasks: BTreeMap::new(),
            revision: 0,
        })
    }
}

impl TaskSystem for Board {
    fn list(&mut self, req: ts::ListRequest, _ctx: &Ctx) -> Result<ts::TaskPage, Refusal> {
        let limit = req.limit.unwrap_or(32).clamp(1, 128) as usize;
        let mut items: Vec<_> = self
            .tasks
            .values()
            .filter(|task| req.after.as_ref().is_none_or(|after| &task.id > after))
            .filter(|task| {
                req.status
                    .as_ref()
                    .is_none_or(|status| task.metadata.get("status") == Some(&json!(status)))
            })
            .cloned()
            .collect();
        items.sort_by(|a, b| a.id.cmp(&b.id));
        let next = (items.len() > limit).then(|| items[limit - 1].id.clone());
        items.truncate(limit);
        Ok(ts::TaskPage { items, next })
    }
    fn status(&mut self, req: ts::StatusRequest, _ctx: &Ctx) -> Result<ts::TaskRecord, Refusal> {
        match req {
            ts::StatusRequest::Path { task_path } => Ok(self.task(&task_path).clone()),
            ts::StatusRequest::Id { task_id } => self
                .tasks
                .values()
                .find(|task| task.id == task_id)
                .cloned()
                .ok_or_else(|| Refusal::new("not_found", "no task has that ID")),
        }
    }
    fn changes_since(
        &mut self,
        req: ts::ChangesRequest,
        _ctx: &Ctx,
    ) -> Result<ts::Changes, Refusal> {
        let since: u64 = req.cursor.parse().unwrap_or(0);
        Ok(ts::Changes {
            changed: self
                .tasks
                .values()
                .filter(|task| task.etag.parse::<u64>().unwrap_or(0) > since)
                .map(|task| task.id.clone())
                .collect(),
            next: self.revision.to_string(),
        })
    }
    fn delegate(
        &mut self,
        req: ts::DelegateRequest,
        _ctx: &Ctx,
    ) -> Result<ts::TaskWriteResult, Refusal> {
        Ok(self.transition(&req.task_path, &req.signed_record, |m| {
            m.insert("status".into(), json!("delegated"));
            m.insert("assignee".into(), json!(format!("seat:{}", req.seat_id)));
        }))
    }
    fn reassign(
        &mut self,
        req: ts::ReassignRequest,
        _ctx: &Ctx,
    ) -> Result<ts::TaskWriteResult, Refusal> {
        Ok(self.transition(&req.task_path, &req.signed_record, |m| {
            m.insert("assignee".into(), json!(format!("seat:{}", req.seat_id)));
        }))
    }
    fn report(
        &mut self,
        req: ts::ReportRequest,
        _ctx: &Ctx,
    ) -> Result<ts::TaskWriteResult, Refusal> {
        Ok(self.transition(&req.task_path, &req.signed_record, |m| {
            m.insert("status".into(), json!("reported"));
            m.insert("report".into(), json!({"summary": req.summary}));
        }))
    }
    fn accept(
        &mut self,
        req: ts::AcceptRequest,
        _ctx: &Ctx,
    ) -> Result<ts::TaskWriteResult, Refusal> {
        Ok(self.transition(&req.task_path, &req.signed_record, |m| {
            m.insert("status".into(), json!("accepted"));
        }))
    }
    fn reject(
        &mut self,
        req: ts::RejectRequest,
        _ctx: &Ctx,
    ) -> Result<ts::TaskWriteResult, Refusal> {
        Ok(self.transition(&req.task_path, &req.signed_record, |m| {
            m.insert("status".into(), json!("rejected"));
            m.insert("reason".into(), json!(req.reason));
        }))
    }
}

fn main() {
    run::<AsTaskSystem<Board>>();
}

// keeps: plugin doctor — this example has no gaps (`testkit::doctor`).
#[cfg(test)]
mod doctor {
    #[test]
    fn the_plugin_doctor_finds_no_gaps() {
        let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let binary = bbt_plugin_sdk::testkit::example(package, "bboxy450-term-task_system-example");
        bbt_plugin_sdk::testkit::doctor(
            &binary,
            &bbt_plugin_sdk::doctor::Options::package(package),
        );
    }
}
