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
fn store_for(source: &UiCanvasSource) -> bboxy450_canvas::store::Store {
    bboxy450_canvas::store::Store::new(
        Path::new(&normalize_root(&source.root))
            .join(".bboxy")
            .join("canvases"),
    )
}
fn record_of(source: &UiCanvasSource, closed: bool) -> Result<bboxy450_canvas::record::Record, String> {
    let store = store_for(source);
    let records = if closed { store.closed_records() } else { store.records() }.map_err(|e| e.to_string())?;
    records
        .into_iter()
        .find(|r| r.id == source.id && r.swarm == source.swarm.to_string())
        .ok_or_else(|| "canvas record not found".to_owned())
}
/// Move the canvas record and log under `closed/`; the drawing is kept.
pub fn archive(source: &UiCanvasSource) -> Result<(), String> {
    match record_of(source, false) {
        Ok(record) => store_for(source).close(&record).map_err(|e| e.to_string()),
        // Already archived: nothing to do.
        Err(_) if record_of(source, true).is_ok() => Ok(()),
        Err(e) => Err(e),
    }
}
/// Bring an archived canvas back.
pub fn restore(source: &UiCanvasSource) -> Result<(), String> {
    match record_of(source, true) {
        Ok(record) => store_for(source).restore(&record).map_err(|e| e.to_string()),
        Err(_) if record_of(source, false).is_ok() => Ok(()),
        Err(e) => Err(e),
    }
}
/// Drop canvas panes whose close grace has passed: their archive stays.
pub fn expire_closed(model: &mut crate::model::Model, now: std::time::Instant) {
    let expired: Vec<bbt_proto::SeatId> = model
        .ui
        .window
        .sidebar
        .swarms
        .iter()
        .flat_map(|s| &s.closed)
        .filter(|c| now >= c.until && model.canvas_sources.contains_key(&c.seat))
        .map(|c| c.seat)
        .collect();
    for pane in expired {
        model.canvas_sources.remove(&pane);
        model.ui.seats.remove(&pane);
        for swarm in &mut model.ui.window.sidebar.swarms {
            swarm.closed.retain(|c| c.seat != pane);
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
