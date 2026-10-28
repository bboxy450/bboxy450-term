// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Small, bounded GUI-state IO jobs; no job runs on the logic Worker or UI thread.
use crate::store::{Result, Store, invalid, object_dir, sqlite::SqliteStore};
use bbt_proto::gui::{WindowSnapshot, WindowState, WindowTree};
use rusqlite::{OptionalExtension, params};
use std::{
    collections::BTreeSet,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::Semaphore;
use ulid::Ulid;
const STATE: &str = "gui/window/state";
const EPOCH: &str = "gui/window/epoch";
const MAX_BYTES: usize = 768 * 1024;
#[derive(Clone)]
pub(crate) struct Journal {
    store: SqliteStore,
    jobs: Arc<Semaphore>,
    latest: Arc<Mutex<(u64, Option<Arc<WindowState>>)>>,
    cache_revision: Arc<AtomicU64>,
}
impl Journal {
    pub(crate) fn open(run: &std::path::Path) -> Result<Self> {
        let store = SqliteStore::open(&object_dir(run, "account")?)?;
        // A damaged window snapshot must not prevent logic from owning PTYs.
        // claim() still returns the precise validation error to the window.
        let state = store
            .get(STATE)?
            .and_then(|bytes| serde_json::from_slice::<WindowState>(&bytes).ok())
            .filter(|state| validate(state).is_ok());
        Ok(Self {
            store,
            latest: Arc::new(Mutex::new((0, state.map(Arc::new)))),
            cache_revision: Arc::new(AtomicU64::new(0)),
            jobs: Arc::new(Semaphore::new(2)),
        })
    }
    pub(crate) fn latest(&self) -> Option<Arc<WindowState>> {
        self.latest
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .1
            .clone()
    }
    pub(crate) async fn claim(&self) -> Result<WindowSnapshot> {
        let permit = self
            .jobs
            .clone()
            .try_acquire_owned()
            .map_err(|_| invalid("window_state_busy"))?;
        let store = self.store.clone();
        let latest = self.latest.clone();
        let revision = self.cache_revision.clone();
        tokio::task::spawn_blocking(move || {
            let _permit=permit;
            let mut connection=store.connection()?;
            let tx=connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let bytes:Option<Vec<u8>>=tx.query_row("SELECT value FROM kv WHERE key=?1",[STATE],|r|r.get(0)).optional()?;
            let state=bytes.as_deref().map(serde_json::from_slice::<WindowState>).transpose()?;
            if let Some(state)=&state {validate(state)?;}
            let epoch=Ulid::new();
            tx.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![EPOCH,epoch.to_string().as_bytes()])?;
            let revision = revision.fetch_add(1, Ordering::Relaxed) + 1;
            tx.commit()?;
            let mut cached = latest.lock().unwrap_or_else(|e| e.into_inner());
            if revision > cached.0 { *cached = (revision, state.clone().map(Arc::new)); }
            Ok(WindowSnapshot {epoch,state})
        }).await.map_err(|e|invalid(format!("window state IO stopped: {e}")))?
    }
    pub(crate) async fn save(&self, epoch: Ulid, state: WindowState) -> Result<()> {
        validate(&state)?;
        let bytes = serde_json::to_vec(&state)?;
        if bytes.len() > MAX_BYTES {
            return Err(invalid("window_state_too_large"));
        }
        let permit = self
            .jobs
            .clone()
            .try_acquire_owned()
            .map_err(|_| invalid("window_state_busy"))?;
        let store = self.store.clone();
        let latest = self.latest.clone();
        let revision = self.cache_revision.clone();
        tokio::task::spawn_blocking(move || {
            let _permit=permit;
            let mut connection=store.connection()?;
            let tx=connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
            let current:Option<Vec<u8>>=tx.query_row("SELECT value FROM kv WHERE key=?1",[EPOCH],|r|r.get(0)).optional()?;
            if current.as_deref()!=Some(epoch.to_string().as_bytes()) {return Err(invalid("window_state_stale_gui"));}
            tx.execute("INSERT INTO kv(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",params![STATE,bytes])?;
            let revision = revision.fetch_add(1, Ordering::Relaxed) + 1;
            tx.commit()?;
            let mut cached = latest.lock().unwrap_or_else(|e| e.into_inner());
            if revision > cached.0 { *cached = (revision, Some(Arc::new(state))); }
            Ok(())
        }).await.map_err(|e|invalid(format!("window state IO stopped: {e}")))?
    }
}
pub(crate) fn validate(state: &WindowState) -> Result<()> {
    if !matches!(
        state.version,
        1 | 2 | 3 | bbt_proto::gui::WINDOW_STATE_VERSION
    ) || state
        .size
        .iter()
        .any(|n| !n.is_finite() || !(1.0..=16384.0).contains(n))
        || !state.sidebar_width.is_finite()
        || state.sidebar_width < 0.0
        || !state.sidebar_scroll.is_finite()
        || state.swarms.len() > 4096
    {
        return Err(invalid("window_state_invalid"));
    }
    let mut sections = BTreeSet::new();
    if state.sidebar_sections_closed.len() > 2
        || state.sidebar_sections_closed.iter().any(|section| {
            !matches!(section.as_str(), "providers/Hosts" | "providers/Cloud")
                || !sections.insert(section)
        })
    {
        return Err(invalid("window_state_invalid_sidebar_sections"));
    }
    let mut swarms = BTreeSet::new();
    let mut tabs = BTreeSet::new();
    let mut seats = BTreeSet::new();
    fn walk(tree: &WindowTree, depth: usize, seen: &mut BTreeSet<Ulid>) -> Result<()> {
        if depth > 64 || seen.len() > 4096 {
            return Err(invalid("window_state_too_large"));
        }
        match tree {
            WindowTree::Seat { id } if seen.insert(*id) => Ok(()),
            WindowTree::Seat { .. } => Err(invalid("window_state_duplicate_seat")),
            WindowTree::Split { ratio, a, b, .. } => {
                if !ratio.is_finite() || !(0.1..=0.9).contains(ratio) {
                    return Err(invalid("window_state_invalid_ratio"));
                }
                walk(a, depth + 1, seen)?;
                walk(b, depth + 1, seen)
            }
        }
    }
    for swarm in &state.swarms {
        if swarm
            .workdir
            .as_deref()
            .is_some_and(|path| !bbt_proto::gui::valid_workdir(path))
        {
            return Err(invalid("window_state_invalid_workdir"));
        }
        if swarm.recent_directories.len() > 6
            || swarm
                .recent_directories
                .iter()
                .any(|p| !bbt_proto::gui::valid_workdir(p))
            || swarm.launch_environment.as_ref().is_some_and(|env| {
                env.len() > 256
                    || env.iter().any(|(k, v)| {
                        k.is_empty()
                            || k.len() > 256
                            || v.len() > 16 * 1024
                            || k.contains('\0')
                            || v.contains('\0')
                    })
            })
        {
            return Err(invalid("window_state_invalid_launch_defaults"));
        }
        if !swarms.insert(swarm.id) {
            return Err(invalid("window_state_duplicate_swarm"));
        }
        for tab in &swarm.tabs {
            if !tabs.insert(tab.id) || tab.name.len() > 4096 {
                return Err(invalid("window_state_invalid_tab"));
            }
            let mut leaves = BTreeSet::new();
            walk(&tab.tree, 0, &mut leaves)?;
            let order: BTreeSet<_> = tab.seats.iter().copied().collect();
            if order.len() != tab.seats.len()
                || order != leaves
                || leaves.iter().any(|id| !seats.insert(*id))
            {
                return Err(invalid("window_state_invalid_seat_order"));
            }
        }
    }
    if seats.len() > 4096 || state.scroll.len() > 4096 {
        return Err(invalid("window_state_too_large"));
    }
    let mut scroll = BTreeSet::new();
    if state
        .scroll
        .iter()
        .any(|s| !seats.contains(&s.seat) || !scroll.insert(s.seat))
    {
        return Err(invalid("window_state_invalid_scroll"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_proto::gui::WindowStage;
    fn empty() -> WindowState {
        WindowState {
            version: 1,
            size: [1280., 720.],
            sidebar_width: 300.,
            sidebar_scroll: 0.,
            sidebar_shown: true,
            open_swarms: vec![],
            sidebar_sections_closed: vec![],
            swarms: vec![],
            stage: WindowStage {
                swarm: None,
                tab: None,
                focused: None,
            },
            scroll: vec![],
        }
    }
    #[test]
    fn journal_accepts_legacy_and_current_schemas_but_rejects_invalid_sections() {
        for version in [1, 2, 3, bbt_proto::gui::WINDOW_STATE_VERSION] {
            let mut state = empty();
            state.version = version;
            validate(&state).unwrap();
        }
        let mut state = empty();
        state.version = bbt_proto::gui::WINDOW_STATE_VERSION;
        for sections in [
            vec!["providers/Hosts"],
            vec!["providers/Cloud"],
            vec!["providers/Cloud", "providers/Hosts"],
        ] {
            state.sidebar_sections_closed = sections.into_iter().map(String::from).collect();
            validate(&state).unwrap();
        }
        for sections in [
            vec!["unknown"],
            vec!["providers/Hosts", "providers/Hosts"],
            vec!["providers/Hosts", "providers/Cloud", "providers/Hosts"],
        ] {
            state.sidebar_sections_closed = sections.into_iter().map(String::from).collect();
            assert!(
                validate(&state)
                    .unwrap_err()
                    .to_string()
                    .contains("window_state_invalid_sidebar_sections")
            );
        }
        state.sidebar_sections_closed.clear();
        state.version = bbt_proto::gui::WINDOW_STATE_VERSION + 1;
        assert!(validate(&state).is_err());
    }
    // keeps: decision #344: a new GUI reads the last committed frame, old queued IO cannot overwrite it
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn reopening_restores_the_snapshot_and_fences_old_gui_jobs() {
        let root = tempfile::tempdir().unwrap();
        let journal = Journal::open(root.path()).unwrap();
        let first = journal.claim().await.unwrap();
        assert!(first.state.is_none());
        let mut saved = empty();
        saved.version = bbt_proto::gui::WINDOW_STATE_VERSION;
        saved.sidebar_sections_closed = vec!["providers/Hosts".into(), "providers/Cloud".into()];
        journal.save(first.epoch, saved.clone()).await.unwrap();
        assert_eq!(journal.latest().as_deref(), Some(&saved));
        let reopened = Journal::open(root.path()).unwrap();
        let next = reopened.claim().await.unwrap();
        assert_eq!(next.state, Some(saved.clone()));
        assert_ne!(first.epoch, next.epoch);
        let mut stale = saved.clone();
        stale.size = [1600., 900.];
        assert!(
            journal
                .save(first.epoch, stale)
                .await
                .unwrap_err()
                .to_string()
                .contains("stale_gui")
        );
        assert_eq!(journal.latest().as_deref(), Some(&saved));
        assert_eq!(reopened.latest().as_deref(), Some(&saved));
        assert_eq!(reopened.claim().await.unwrap().state, Some(saved));
    }
    // keeps: decision #344: malformed UI data cannot replace the last good snapshot
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn invalid_geometry_cannot_replace_the_last_good_frame() {
        let root = tempfile::tempdir().unwrap();
        let journal = Journal::open(root.path()).unwrap();
        let first = journal.claim().await.unwrap();
        let valid = empty();
        journal.save(first.epoch, valid.clone()).await.unwrap();
        let mut invalid = valid.clone();
        invalid.size[0] = f32::NAN;
        assert!(journal.save(first.epoch, invalid).await.is_err());
        assert_eq!(journal.claim().await.unwrap().state, Some(valid));
    }
}
