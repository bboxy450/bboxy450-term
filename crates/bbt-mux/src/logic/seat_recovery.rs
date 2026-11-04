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
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
impl Drop for Preparations {
    fn drop(&mut self) {
        self.cancel();
    }
}

pub(super) struct Batch {
    pub pending: BTreeMap<Ulid, SeatRecord>,
    pub preparations: Option<Preparations>,
    pub deadline: Instant,
    pub total: u64,
    pub completed: u64,
    pub progress: Option<mpsc::UnboundedSender<(u64, u64)>>,
    pub reply: Option<oneshot::Sender<Result<()>>>,
}
impl Batch {
    // This runs on the command owner: never wait here. One launch per turn
    // leaves MCP initialization and other seats' requests serviceable.
    pub fn next(&mut self, now: Instant) -> Option<(SeatRecord, Option<Playback>, bool)> {
        let result = self
            .preparations
            .as_ref()
            .map_or(Err(sync::TryRecvError::Disconnected), |pool| {
                pool.receiver.try_recv()
            });
        match result {
            Ok((record, result)) => {
                if !self
                    .pending
                    .get(&record.id)
                    .is_some_and(|saved| saved.generation == record.generation)
                {
                    return None;
                }
                self.pending.remove(&record.id);
                match result {
                    Ok(state) => Some((record, Some(state), false)),
                    Err(error) => {
                        eprintln!("SeatRecoveryDeferred [{}]: {error}", record.id);
                        Some((record, None, true))
                    }
                }
            }
            Err(sync::TryRecvError::Empty) if now < self.deadline => None,
            Err(_) => {
                if let Some(pool) = &self.preparations {
                    pool.cancel();
                }
                self.pending
                    .pop_first()
                    .map(|(_, record)| (record, None, true))
            }
        }
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
        let records: Vec<_> = (1..=4)
            .map(|id| SeatRecord::new(Ulid::from(id), Ulid::from(9), "fixture", "zsh"))
            .collect();
        let pending = records
            .iter()
            .map(|record| (record.id, record.clone()))
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
            let state = Playback::new(record.id, record.generation, SeatSize::default());
            finished.send(()).unwrap();
            Ok(state)
        })
        .unwrap();
        for _ in 0..lanes() {
            entries.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert_eq!(started.load(Ordering::Acquire), lanes() as u64);
        let before = Instant::now();
        let mut batch = Batch {
            pending,
            preparations: Some(batch),
            deadline: before + Duration::from_secs(60),
            total: 4,
            completed: 0,
            progress: None,
            reply: None,
        };
        assert!(batch.next(before).is_none());
        assert!(
            before.elapsed() < Duration::from_secs(1),
            "replay blocked the command owner"
        );
        drop(batch); // No receiver remains that could publish a stale generation.
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        for _ in 0..lanes() {
            finishes.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        assert_eq!(started.load(Ordering::Acquire), lanes() as u64);
    }
}
