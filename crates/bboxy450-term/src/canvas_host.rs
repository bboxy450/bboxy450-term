// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! IO-owned canvas snapshots. Appends notify; focus/startup re-read external writers.
#![allow(
    clippy::disallowed_methods,
    reason = "canvas filesystem reads execute on the IO blocking pool"
)]
use bboxy450_canvas::store::{Snapshot, Store};
use bbt_proto::gui::UiCanvasSource;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

/// A repository and its built-in knowledge directory name the same canvas store.
/// This is lexical: UI lookups never probe the filesystem.
pub fn normalize_root(root: &str) -> String {
    let path = Path::new(root);
    let path = if path.file_name().is_some_and(|name| name == ".bboxy") {
        path.parent().unwrap_or(path)
    } else {
        path
    };
    path.components()
        .collect::<PathBuf>()
        .to_string_lossy()
        .into_owned()
}
#[derive(Clone, Debug)]
pub struct Row {
    pub source: UiCanvasSource,
    pub snapshot: Arc<Snapshot>,
}
impl Row {
    pub fn change(&self) -> u64 {
        self.snapshot.last_change()
    }
    pub fn sidebar(&self) -> bbt_ui::modules::sidebar::CanvasRow {
        bbt_ui::modules::sidebar::CanvasRow {
            id: self.source.id.clone(),
            title: self.source.title.clone(),
            last_change: crate::local_time::hm(self.change()),
            last_author: self.snapshot.last_author().to_owned(),
            tags: self.snapshot.record.tags.clone(),
        }
    }
}
pub fn load(root: &str, swarm: bbt_mux::Ulid) -> Result<Vec<Row>, String> {
    let root = Path::new(root);
    let directory = if root.join(".bboxy").is_dir() {
        root.join(".bboxy")
    } else {
        root.to_owned()
    }
    .join("canvases");
    let store = Store::new(directory);
    let mut rows = vec![];
    for record in store
        .records()
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|r| r.swarm == swarm.to_string())
    {
        let snapshot = Arc::new(store.load(&record).map_err(|e| e.to_string())?);
        rows.push(Row {
            source: UiCanvasSource {
                root: normalize_root(&root.to_string_lossy()),
                swarm,
                id: record.id,
                title: record.title,
            },
            snapshot,
        });
    }
    rows.sort_by(|a, b| (b.change(), &b.source.id).cmp(&(a.change(), &a.source.id)));
    Ok(rows)
}
pub type Completion = (bbt_mux::Ulid, String, u64, Result<Vec<Row>, String>);

/// Each swarm/root has an independent generation and retained result. Returning
/// to A cannot admit an earlier A completion or cancel a valid B read.
#[derive(Default)]
pub struct Reads(std::collections::BTreeMap<(bbt_mux::Ulid, String), u64>);
impl Reads {
    pub fn begin(&mut self, swarm: bbt_mux::Ulid, root: &str) -> u64 {
        let next = self.0.entry((swarm, normalize_root(root))).or_default();
        *next += 1;
        *next
    }
    pub fn current(&self, swarm: bbt_mux::Ulid, root: &str, generation: u64) -> bool {
        self.0.get(&(swarm, normalize_root(root))) == Some(&generation)
    }
}
