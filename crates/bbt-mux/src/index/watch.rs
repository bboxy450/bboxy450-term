// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded dirty paths, settled into a final-state scan rather than event replay.
// Adapted from bboxy450-term src/index.rs FileWatch, same canonical project above.
// Independently implemented; inspired by Watchman's settled subscriptions:
// https://github.com/facebook/watchman/blob/main/website/docs/cmd/subscribe.md
use super::{Result, knowledge_event};
use notify::{RecursiveMode, Watcher};
use std::{
    collections::BTreeSet,
    path::{Component, Path},
    sync::{
        Arc, Mutex, TryLockError,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const PATHS: usize = 4096;
const QUIET: Duration = Duration::from_millis(200);
const MAX_WAIT: Duration = Duration::from_secs(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ScanBatch {
    Full,
    Paths(BTreeSet<String>),
}
struct Pending {
    batch: Option<ScanBatch>,
    first: Instant,
    last: Instant,
    immediate: bool,
}
impl Pending {
    fn new(now: Instant) -> Self {
        Self {
            batch: Some(ScanBatch::Full),
            first: now,
            last: now,
            immediate: true,
        }
    }
    fn push(&mut self, batch: ScanBatch, now: Instant) {
        if self.batch.is_none() {
            self.first = now;
        }
        self.last = now;
        let batch = match batch {
            ScanBatch::Paths(paths) if paths.len() > PATHS => ScanBatch::Full,
            batch => batch,
        };
        self.batch = Some(match (self.batch.take(), batch) {
            (Some(ScanBatch::Full), _) | (_, ScanBatch::Full) => ScanBatch::Full,
            (Some(ScanBatch::Paths(mut old)), ScanBatch::Paths(new)) => {
                old.extend(new);
                if old.len() > PATHS {
                    ScanBatch::Full
                } else {
                    ScanBatch::Paths(old)
                }
            }
            (None, batch) => batch,
        });
    }
    fn ready(&self, now: Instant) -> bool {
        self.batch.is_some()
            && (self.immediate
                || now.duration_since(self.last) >= QUIET
                || now.duration_since(self.first) >= MAX_WAIT)
    }
    fn take(&mut self, now: Instant, force: bool) -> Option<ScanBatch> {
        if !force && !self.ready(now) {
            return None;
        }
        self.immediate = false;
        self.batch.take()
    }
}

/// Callback work only records bounded dirtiness. Scanning/parsing stays on IO.
pub struct FileWatch {
    _watcher: notify::RecommendedWatcher,
    pending: Arc<Mutex<Pending>>,
    root: std::path::PathBuf,
    failed: Arc<AtomicBool>,
}
impl FileWatch {
    pub fn start(root: &Path) -> Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let root = root.canonicalize()?;
        let observed = root.clone();
        let pending = Arc::new(Mutex::new(Pending::new(Instant::now())));
        let failed = Arc::new(AtomicBool::new(false));
        let (changes, failure) = (pending.clone(), failed.clone());
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                let batch = match event {
                    Ok(event) => batch(&observed, &event),
                    Err(_) => {
                        failure.store(true, Ordering::Release);
                        Some(ScanBatch::Full)
                    }
                };
                if let Some(batch) = batch {
                    changes
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .push(batch, Instant::now());
                }
            })?;
        watcher.watch(&root, RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            pending,
            root,
            failed,
        })
    }
    // Preserve the immediate consuming API for existing callers and fixtures.
    pub fn changed(&self) -> bool {
        self.take(true).is_some()
    }
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }
    pub fn failed(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }
    pub(crate) fn pending(&self) -> bool {
        match self.pending.try_lock() {
            Ok(pending) => pending.batch.is_some(),
            Err(TryLockError::Poisoned(error)) => error.into_inner().batch.is_some(),
            // Admission must not assume a clean snapshot while a callback owns it.
            Err(TryLockError::WouldBlock) => true,
        }
    }
    pub(crate) fn ready(&self) -> bool {
        match self.pending.try_lock() {
            Ok(pending) => pending.ready(Instant::now()),
            Err(TryLockError::Poisoned(error)) => error.into_inner().ready(Instant::now()),
            Err(TryLockError::WouldBlock) => false,
        }
    }
    pub(crate) fn take(&self, force: bool) -> Option<ScanBatch> {
        match self.pending.try_lock() {
            Ok(mut pending) => pending.take(Instant::now(), force),
            Err(TryLockError::Poisoned(error)) => error.into_inner().take(Instant::now(), force),
            Err(TryLockError::WouldBlock) => None,
        }
    }
    pub(crate) fn mark_paths(&self, paths: BTreeSet<String>) {
        if !paths.is_empty() {
            self.pending
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push(ScanBatch::Paths(paths), Instant::now());
        }
    }
}

fn batch(root: &Path, event: &notify::Event) -> Option<ScanBatch> {
    if !knowledge_event(root, event) {
        return None;
    }
    if event.need_rescan()
        || event.paths.is_empty()
        || matches!(
            event.kind,
            notify::EventKind::Create(notify::event::CreateKind::Folder)
                | notify::EventKind::Remove(notify::event::RemoveKind::Folder)
                // Case-only renames on macOS can leave both spellings resolving
                // to one inode. Enumerate final namespace, not hinted spellings.
                | notify::EventKind::Modify(notify::event::ModifyKind::Name(_))
        )
    {
        return Some(ScanBatch::Full);
    }
    let mut paths = BTreeSet::new();
    for path in &event.paths {
        if !super::knowledge_path(root, path, event.kind) {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        if matches!(relative.components().next(), Some(Component::Normal(name)) if matches!(name.to_str(), Some("plugins" | "plugin-notices")))
        {
            continue;
        }
        let Some(relative) = relative.to_str() else {
            return Some(ScanBatch::Full);
        };
        paths.insert(relative.to_owned());
        if paths.len() > PATHS {
            return Some(ScanBatch::Full);
        }
    }
    (!paths.is_empty()).then_some(ScanBatch::Paths(paths))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rename_hints_reconcile_final_namespace_instead_of_duplicate_spellings() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(root.join("Source.md"), "evidence\n").unwrap();
        std::fs::rename(root.join("Source.md"), root.join("source.md")).unwrap();
        let event = notify::Event::new(notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Both,
        )))
        .add_path(root.join("Source.md"))
        .add_path(root.join("source.md"));
        assert_eq!(batch(root, &event), Some(ScanBatch::Full));
        // Native backends can deliver just one unpaired rename hint.
        let event = notify::Event::new(notify::EventKind::Modify(notify::event::ModifyKind::Name(
            notify::event::RenameMode::Any,
        )))
        .add_path(root.join("source.md"));
        assert_eq!(batch(root, &event), Some(ScanBatch::Full));
    }
    fn paths(values: &[&str]) -> ScanBatch {
        ScanBatch::Paths(values.iter().map(|value| (*value).to_owned()).collect())
    }
    #[test]
    fn dirty_paths_settle_once_and_new_hints_survive_an_inflight_batch() {
        let now = Instant::now();
        let mut pending = Pending::new(now);
        assert_eq!(pending.take(now, false), Some(ScanBatch::Full));
        pending.push(paths(&["a.md", "b.md"]), now);
        pending.push(paths(&["a.md"]), now + Duration::from_millis(100));
        assert!(!pending.ready(now + Duration::from_millis(299)));
        let first = pending
            .take(now + Duration::from_millis(300), false)
            .unwrap();
        assert_eq!(first, paths(&["a.md", "b.md"]));
        pending.push(paths(&["a.md", "c.md"]), now + Duration::from_millis(310));
        assert_eq!(
            pending.take(now + Duration::from_millis(510), false),
            Some(paths(&["a.md", "c.md"]))
        );
        assert_eq!(first, paths(&["a.md", "b.md"]));
    }
    #[test]
    fn continuous_changes_reach_the_maximum_wait_and_overflow_collapses_to_full() {
        let now = Instant::now();
        let mut pending = Pending::new(now);
        pending.take(now, true);
        pending.push(paths(&["a.md"]), now);
        for n in 1..=10 {
            pending.push(paths(&["a.md"]), now + Duration::from_millis(n * 100));
        }
        assert!(pending.ready(now + MAX_WAIT));
        assert_eq!(pending.take(now + MAX_WAIT, false), Some(paths(&["a.md"])));
        pending.push(
            ScanBatch::Paths((0..=PATHS).map(|n| format!("{n}.md")).collect()),
            now + MAX_WAIT,
        );
        assert_eq!(pending.take(now + MAX_WAIT, true), Some(ScanBatch::Full));
    }
}
