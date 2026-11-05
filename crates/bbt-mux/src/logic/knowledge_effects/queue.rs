// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded per-root FIFO waiting; no writer lease or provider call is retained.
use std::collections::{BTreeMap, VecDeque};
const REQUESTS: usize = 1024;
const BYTES: usize = 16 * 1024 * 1024;
pub(crate) struct EffectQueue<T> {
    roots: BTreeMap<String, VecDeque<(usize, T)>>,
    count: usize,
    bytes: usize,
}
impl<T> Default for EffectQueue<T> {
    fn default() -> Self {
        Self {
            roots: BTreeMap::new(),
            count: 0,
            bytes: 0,
        }
    }
}
impl<T> EffectQueue<T> {
    pub fn contains(&self, root: &str) -> bool {
        self.roots.contains_key(root)
    }
    pub fn roots(&self) -> impl Iterator<Item = &String> {
        self.roots.keys()
    }
    pub fn push(&mut self, root: String, bytes: usize, value: T, front: bool) -> Result<(), T> {
        if self.count >= REQUESTS || bytes > BYTES.saturating_sub(self.bytes) {
            return Err(value);
        }
        let queue = self.roots.entry(root).or_default();
        if front {
            queue.push_front((bytes, value));
        } else {
            queue.push_back((bytes, value));
        }
        self.count += 1;
        self.bytes += bytes;
        Ok(())
    }
    pub fn pop(&mut self, root: &str) -> Option<T> {
        let queue = self.roots.get_mut(root)?;
        let (bytes, value) = queue.pop_front()?;
        if queue.is_empty() {
            self.roots.remove(root);
        }
        self.count -= 1;
        self.bytes -= bytes;
        Some(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn thousand_waiting_writers_keep_fifo_and_another_root_can_progress() {
        let mut queue = EffectQueue::default();
        for n in 0..1000 {
            queue.push("busy".into(), 64, n, false).unwrap();
        }
        queue.push("other".into(), 64, 1001, false).unwrap();
        assert_eq!(queue.pop("other"), Some(1001));
        // A temporarily occupied writer keeps its position on requeue.
        let first = queue.pop("busy").unwrap();
        queue.push("busy".into(), 64, first, true).unwrap();
        for n in 0..1000 {
            assert_eq!(queue.pop("busy"), Some(n));
        }
        assert!(!queue.contains("busy"));
        assert_eq!((queue.count, queue.bytes), (0, 0));
    }
    #[test]
    fn capacity_refuses_before_admission_and_released_bytes_are_reusable() {
        let mut queue = EffectQueue::default();
        queue.push("root".into(), BYTES, 1, false).unwrap();
        assert_eq!(queue.push("root".into(), 1, 2, false), Err(2));
        assert_eq!(queue.pop("root"), Some(1));
        for n in 0..REQUESTS {
            queue.push("root".into(), 0, n, false).unwrap();
        }
        assert_eq!(queue.push("root".into(), 0, REQUESTS, false), Err(REQUESTS));
        assert_eq!(queue.count, REQUESTS);
    }
}
