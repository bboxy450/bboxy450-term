// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Routing hints only. Authority and signed grants still use fresh Registry::get.
use super::*;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock, Weak,
        atomic::{AtomicU64, Ordering},
    },
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SeatRoute {
    pub location: Location,
    pub swarm: Ulid,
}

#[derive(Default)]
pub(super) struct Cache {
    epoch: AtomicU64,
    seats: Mutex<BTreeMap<Ulid, (u64, SeatRoute)>>,
}
impl Cache {
    pub fn for_path(path: &Path) -> Arc<Self> {
        static CACHES: OnceLock<Mutex<BTreeMap<PathBuf, Weak<Cache>>>> = OnceLock::new();
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let mut caches = CACHES
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(cache) = caches.get(&path).and_then(Weak::upgrade) {
            return cache;
        }
        caches.retain(|_, cache| cache.strong_count() > 0);
        let cache = Arc::new(Self::default());
        caches.insert(path, Arc::downgrade(&cache));
        cache
    }
    pub fn invalidate(&self) {
        let mut epoch = self.epoch.load(Ordering::Acquire);
        loop {
            let next = epoch
                .checked_add(1)
                .expect("registry routing epoch exhausted");
            match self
                .epoch
                .compare_exchange(epoch, next, Ordering::AcqRel, Ordering::Acquire)
            {
                Ok(_) => return,
                Err(current) => epoch = current,
            }
        }
    }
}
impl Registry {
    pub(crate) fn routing_epoch(&self) -> u64 {
        self.routing.epoch.load(Ordering::Acquire)
    }
    pub(crate) fn route(&self, id: Ulid) -> Result<SeatRoute> {
        // A write invalidates after commit and before returning. A miss is stamped
        // with its starting epoch, so a concurrent invalidation cannot bless old data.
        let epoch = self.routing_epoch();
        let cached = self
            .routing
            .seats
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&id)
            .cloned();
        if let Some((cached_epoch, route)) = cached
            && cached_epoch == epoch
        {
            return Ok(route);
        }
        // No shared cache lock spans storage IO: an unavailable cold seat does
        // not hold another caller's already-cached routing hint.
        let record = self.get(id)?;
        let route = SeatRoute {
            location: record.location,
            swarm: record.swarm,
        };
        self.routing
            .seats
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(id, (epoch, route.clone()));
        Ok(route)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routing_cache_invalidates_across_handles_and_a_b_a() {
        let root = tempfile::tempdir().unwrap();
        let registry = Registry::new(SqliteStore::open(root.path()).unwrap());
        let writer = Registry::new(SqliteStore::open(root.path()).unwrap());
        let seat = SeatRecord::new(Ulid::new(), Ulid::new(), "route", "zsh");
        registry.insert(&seat).unwrap();
        let a = registry.route(seat.id).unwrap();
        let first = registry.routing_epoch();
        let other = Ulid::new();
        writer
            .update(seat.id, |r| {
                r.swarm = other;
                r.location = Location::Ssh {
                    host: "remote".into(),
                };
                Ok(())
            })
            .unwrap();
        assert!(registry.routing_epoch() > first);
        assert_eq!(registry.route(seat.id).unwrap().swarm, other);
        assert_eq!(
            registry.route(seat.id).unwrap().location,
            Location::Ssh {
                host: "remote".into()
            }
        );
        writer
            .update(seat.id, |r| {
                r.swarm = seat.swarm;
                r.location = Location::Local;
                Ok(())
            })
            .unwrap();
        assert_eq!(registry.route(seat.id).unwrap(), a);
        assert!(registry.routing_epoch() >= first + 2);
        assert_eq!(
            registry.routing.seats.lock().unwrap()[&seat.id].0,
            registry.routing_epoch()
        );
        let old = registry.routing_epoch();
        writer.rename_swarm(seat.swarm, "renamed").unwrap();
        assert!(registry.routing_epoch() > old);
        assert_eq!(registry.route(seat.id).unwrap(), a);
    }
    #[test]
    fn routing_cache_never_caches_a_missing_seat_or_authority() {
        let root = tempfile::tempdir().unwrap();
        let registry = Registry::new(SqliteStore::open(root.path()).unwrap());
        let seat = SeatRecord::new(Ulid::new(), Ulid::new(), "route", "zsh");
        assert!(registry.route(seat.id).is_err());
        registry.insert(&seat).unwrap();
        registry.route(seat.id).unwrap();
        registry
            .update(seat.id, |r| {
                r.name = "fresh authority".into();
                Ok(())
            })
            .unwrap();
        assert_eq!(registry.get(seat.id).unwrap().name, "fresh authority");
        assert!(registry.route(seat.id).is_ok());
    }
}
