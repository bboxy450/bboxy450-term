// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Independent, single-flight workers. UI receives owned values and never locks or reads IO.
use crate::*;
use std::{path::PathBuf, time::Duration};
use tokio::{runtime::Handle, sync::watch, task::JoinHandle};

#[derive(Debug, Clone)]
pub enum Update {
    Machine(machine::Machine),
    Provider {
        id: &'static str,
        reading: Option<Reading>,
    },
}
#[derive(Clone, Copy)]
pub struct Intervals {
    pub machine: Duration,
    pub ai: Duration,
    pub timeout: Duration,
}
impl Default for Intervals {
    fn default() -> Self {
        Self {
            machine: MACHINE_INTERVAL,
            ai: AI_INTERVAL,
            timeout: READ_TIMEOUT,
        }
    }
}

/// Atomic owner state mapped to UsagePoller. Epochs identify attempts, not account names:
/// an A -> B -> A switch cannot resurrect the first A's completion.
#[derive(Default)]
pub struct PollState {
    epoch: u64,
    pending: bool,
    identity: Option<[u8; 32]>,
    pub reading: Option<Reading>,
}
impl PollState {
    pub fn begin(&mut self, identity: Option<[u8; 32]>) -> Option<u64> {
        let Some(epoch) = self.epoch.checked_add(1) else {
            self.pending = false;
            self.identity = None;
            self.reading = Some(Reading::error("Usage generation exhausted"));
            return None;
        };
        self.epoch = epoch;
        self.pending = true;
        self.identity = identity;
        self.reading = Some(Reading::error("Refreshing usage"));
        Some(self.epoch)
    }
    pub fn complete(&mut self, epoch: u64, identity: Option<[u8; 32]>, reading: Reading) -> bool {
        if !self.pending || self.epoch != epoch || self.identity != identity {
            return false;
        }
        self.pending = false;
        self.reading = Some(reading);
        true
    }
    pub fn expire(&mut self) {
        self.pending = false;
        if let Some(reading) = &mut self.reading {
            reading.stale("Refresh due");
        }
    }
}

/// A timed-out blocking OS read is kept, never detached and replaced repeatedly.
/// At most one OS call per owner remains outstanding, even for a permanently hung filesystem.
struct ReadSlot<T> {
    pending: Option<JoinHandle<T>>,
}
impl<T: Send + 'static> ReadSlot<T> {
    fn new() -> Self {
        Self { pending: None }
    }
    async fn read(
        &mut self,
        timeout: Duration,
        work: impl FnOnce() -> T + Send + 'static,
    ) -> Result<T, Failure> {
        if let Some(pending) = &self.pending {
            if !pending.is_finished() {
                return Err(Failure::Timeout);
            }
            // A late completion belongs to an expired attempt. Discard, then read afresh.
            drop(self.pending.take());
        }
        self.pending = Some(tokio::task::spawn_blocking(work));
        let result =
            tokio::time::timeout(timeout, self.pending.as_mut().expect("installed read")).await;
        match result {
            Ok(result) => {
                self.pending.take();
                result.map_err(|_| Failure::Credentials)
            }
            Err(_) => Err(Failure::Timeout),
        }
    }
}
struct Prepared {
    presence: Result<Presence, Failure>,
    fallback: Option<Reading>,
}
async fn prepare(
    slot: &mut ReadSlot<Prepared>,
    p: Arc<dyn Provider>,
    r: Arc<dyn CredentialReader>,
    timeout: Duration,
) -> Result<Prepared, Failure> {
    slot.read(timeout, move || {
        let presence = p.credential(r.as_ref());
        let fallback = if matches!(presence, Ok(Presence::SignedOut)) {
            p.fallback(r.as_ref())
        } else {
            None
        };
        Prepared { presence, fallback }
    })
    .await
}

pub struct Worker {
    refresh: watch::Sender<u64>,
    tasks: Vec<JoinHandle<()>>,
}
impl Worker {
    pub fn start(
        runtime: &Handle,
        home: PathBuf,
        providers: Vec<Arc<dyn Provider>>,
        reader: Arc<dyn CredentialReader>,
        fetch: Arc<dyn Fetch>,
        publish: Arc<dyn Fn(Update) + Send + Sync>,
        intervals: Intervals,
    ) -> Self {
        let (refresh, rx) = watch::channel(0);
        let mut tasks = Vec::new();
        for provider in providers {
            tasks.push(runtime.spawn(provider_loop(
                provider,
                reader.clone(),
                fetch.clone(),
                publish.clone(),
                rx.clone(),
                intervals,
            )));
        }
        tasks.push(runtime.spawn(async move {
            let mut sampler = machine::Sampler::default();
            let mut slot = ReadSlot::new();
            let mut tick = tokio::time::interval(intervals.machine);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                let home = home.clone();
                match slot
                    .read(intervals.timeout, move || {
                        let snapshot = sampler.sample(&home);
                        (sampler, snapshot)
                    })
                    .await
                {
                    Ok((returned, snapshot)) => {
                        sampler = returned;
                        publish(Update::Machine(snapshot));
                    }
                    Err(e) => {
                        sampler = machine::Sampler::default();
                        publish(Update::Machine(machine::Machine {
                            reason: Some(e.reason().into()),
                            ..Default::default()
                        }));
                    }
                }
            }
        }));
        Self { refresh, tasks }
    }
    /// Coalesced, nonblocking signal: it does not bypass the single-flight or timeout bounds.
    pub fn refresh(&self) {
        self.refresh.send_modify(|n| *n = n.wrapping_add(1));
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

async fn provider_loop(
    p: Arc<dyn Provider>,
    r: Arc<dyn CredentialReader>,
    fetch: Arc<dyn Fetch>,
    publish: Arc<dyn Fn(Update) + Send + Sync>,
    mut refresh: watch::Receiver<u64>,
    intervals: Intervals,
) {
    let mut slot = ReadSlot::new();
    let mut state = PollState::default();
    let mut visible = false;
    loop {
        state.expire();
        if visible {
            // Credentials are about to be re-observed: withhold previous account numbers now.
            publish(Update::Provider {
                id: p.id(),
                reading: Some(Reading::error("Refreshing sign-in")),
            });
        }
        let prepared = prepare(&mut slot, p.clone(), r.clone(), intervals.timeout).await;
        let reading = match prepared {
            Ok(Prepared {
                presence: Ok(Presence::Missing),
                ..
            }) => None,
            Ok(Prepared {
                presence: Ok(Presence::SignedOut),
                fallback,
            }) => Some(fallback.unwrap_or_else(|| Reading::error(p.login_hint()))),
            Ok(Prepared {
                presence: Ok(Presence::Ready(c)),
                ..
            }) => {
                let identity = Some(c.identity());
                let Some(epoch) = state.begin(identity) else {
                    return;
                };
                let result = fetch_reading(p.as_ref(), fetch.as_ref(), &c, intervals.timeout).await;
                let check = prepare(&mut slot, p.clone(), r.clone(), intervals.timeout).await;
                let reading = match check {
                    Ok(Prepared {
                        presence: Ok(Presence::Ready(current)),
                        ..
                    }) if current == c => result,
                    _ => Reading::error(Failure::AccountChanged.reason()),
                };
                if !state.complete(epoch, identity, reading) {
                    continue;
                }
                state.reading.clone()
            }
            Ok(Prepared {
                presence: Err(e), ..
            })
            | Err(e) => Some(error_reading(e, p.as_ref())),
        };
        visible = reading.is_some();
        publish(Update::Provider {
            id: p.id(),
            reading,
        });
        tokio::select! {
            _ = tokio::time::sleep(intervals.ai) => {},
            result = refresh.changed() => if result.is_err() { return; },
        }
    }
}
fn error_reading(e: Failure, p: &dyn Provider) -> Reading {
    Reading::error(if e == Failure::Auth {
        p.login_hint()
    } else {
        e.reason()
    })
}
pub async fn fetch_reading(
    p: &dyn Provider,
    fetch: &dyn Fetch,
    c: &Credential,
    timeout: Duration,
) -> Reading {
    let result = tokio::time::timeout(timeout, async {
        let response = fetch.get(p.request(c)).await?;
        match response.status {
            401 | 403 => Err(Failure::Auth),
            200..300 => p.decode(&response.body),
            status => Err(Failure::Http(status)),
        }
    })
    .await
    .unwrap_or(Err(Failure::Timeout));
    match result {
        Ok(left) => Reading::current(left),
        Err(e) => error_reading(e, p),
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_methods,
    reason = "private unattended runtime and latch witnesses, never a UI turn"
)]
mod tests {
    use super::*;
    #[test]
    fn refresh_timeout_duplicate_and_aba_cannot_publish_old_numbers() {
        let mut s = PollState::default();
        let a = Some([1; 32]);
        let b = Some([2; 32]);
        let first = s.begin(a).unwrap();
        let second = s.begin(b).unwrap();
        assert!(!s.complete(first, a, Reading::current(88)));
        assert_eq!(s.reading.as_ref().unwrap().left, None);
        assert!(s.complete(second, b, Reading::current(12)));
        assert!(!s.complete(second, b, Reading::current(99)));
        s.expire();
        assert!(s.reading.as_ref().unwrap().stale);
        let third = s.begin(a).unwrap();
        assert!(!s.complete(first, a, Reading::current(88)));
        s.expire();
        assert!(!s.complete(third, a, Reading::current(88)));
        assert_eq!(s.reading.as_ref().unwrap().left, None);
    }

    #[tokio::test]
    async fn hung_read_has_a_deadline_and_cannot_spawn_replacements() {
        use std::sync::{
            Condvar, Mutex,
            atomic::{AtomicUsize, Ordering},
        };
        let release = Arc::new((Mutex::new(false), Condvar::new()));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut slot = ReadSlot::<u8>::new();
        let latch = release.clone();
        let count = calls.clone();
        let first = slot
            .read(Duration::from_millis(10), move || {
                count.fetch_add(1, Ordering::SeqCst);
                let (lock, cv) = &*latch;
                let guard = lock.lock().unwrap();
                drop(cv.wait_while(guard, |ready| !*ready).unwrap());
                99
            })
            .await;
        // Always release the owned blocking task before asserting, even on a failed timeout.
        let second = slot.read(Duration::from_millis(10), || 12).await;
        let (lock, cv) = &*release;
        *lock.lock().unwrap() = true;
        cv.notify_all();
        assert_eq!(first, Err(Failure::Timeout));
        assert_eq!(second, Err(Failure::Timeout));
        for _ in 0..100 {
            if slot.pending.as_ref().unwrap().is_finished() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(slot.read(Duration::from_secs(1), || 12).await, Ok(12));
    }
}
