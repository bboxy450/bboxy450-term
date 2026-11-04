// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded predecessor replay. Workers never publish views or change ownership.
use super::*;
use std::sync::{Mutex, mpsc as sync};

fn lanes() -> usize {
    std::thread::available_parallelism().map_or(1, |cores| (cores.get() / 2).clamp(1, 2))
}

pub(super) struct Preparations {
    receiver: sync::Receiver<(SeatRecord, Result<Playback>)>,
    cancelled: Arc<AtomicBool>,
}
impl Preparations {
    pub fn start(
        records: Vec<SeatRecord>,
        prepare: impl Fn(&SeatRecord) -> Result<Playback> + Send + Sync + 'static,
    ) -> Result<Self> {
        let pending = Arc::new(Mutex::new(VecDeque::from(records)));
        let cancelled = Arc::new(AtomicBool::new(false));
        let prepare = Arc::new(prepare);
        let (sender, receiver) = sync::sync_channel(1);
        let batch = Self {
            receiver,
            cancelled: cancelled.clone(),
        };
        for index in 0..lanes() {
            let (pending, cancelled, prepare, sender) = (
                pending.clone(),
                cancelled.clone(),
                prepare.clone(),
                sender.clone(),
            );
            std::thread::Builder::new()
                .name(format!("bbt-recover-{index}"))
                .spawn(move || {
                    let _scans = bbt_supervisor::ring::RecoveryScanCache::default();
                    while !cancelled.load(Ordering::Acquire) {
                        let Some(record) = pending.lock().unwrap().pop_front() else {
                            break;
                        };
                        if cancelled.load(Ordering::Acquire) {
                            break;
                        }
                        let result = prepare(&record);
                        if cancelled.load(Ordering::Acquire)
                            || sender.send((record, result)).is_err()
                        {
                            break;
                        }
                    }
                })?;
        }
        Ok(batch)
    }
    pub fn recv(
        &self,
        timeout: Duration,
    ) -> std::result::Result<(SeatRecord, Result<Playback>), sync::RecvTimeoutError> {
        self.receiver.recv_timeout(timeout)
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
impl Drop for Preparations {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_workers_bound_replay_and_cancel_discards_late_completions() {
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let started = Arc::new(AtomicU64::new(0));
        let (entered, entries) = sync::sync_channel(2);
        let (finished, finishes) = sync::sync_channel(2);
        let records = (1..=4)
            .map(|id| SeatRecord::new(Ulid::from(id), Ulid::from(9), "fixture", "zsh"))
            .collect();
        let ready = gate.clone();
        let count = started.clone();
        let batch = Preparations::start(records, move |record| {
            count.fetch_add(1, Ordering::AcqRel);
            entered.send(()).unwrap();
            let (lock, wake) = &*ready;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
            let state = Playback::new(
                record.id,
                record.generation,
                SeatSize::default(),
            );
            finished.send(()).unwrap();
            Ok(state)
        })
        .unwrap();
        for _ in 0..lanes() {
            entries.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert_eq!(started.load(Ordering::Acquire), lanes() as u64);
        assert!(batch.recv(Duration::ZERO).is_err());
        drop(batch); // No receiver remains that could publish a stale generation.
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        for _ in 0..lanes() {
            finishes.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert_eq!(started.load(Ordering::Acquire), lanes() as u64);
    }
}
