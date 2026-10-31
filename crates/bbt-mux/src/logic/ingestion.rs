// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One bounded parser completion per seat, independent of ring/history waits.
use super::*;

pub(super) struct Parsed {
    pub state: Playback,
    pub tap: OscTap,
    pub replies: Vec<Vec<u8>>,
    pub events: u64,
    pub bytes: u64,
    pub boundary: bool,
}

struct Job {
    attachment: Attachment,
    incarnation: Ulid,
    from: u64,
    through: u64,
    live: bool,
    result: oneshot::Receiver<Result<Parsed>>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Default)]
pub(super) struct Parsers(BTreeMap<Ulid, Job>, Vec<Arc<tokio::sync::Semaphore>>);
impl Parsers {
    pub fn contains(&self, id: Ulid) -> bool {
        self.0.contains_key(&id)
    }
    pub fn retire(&mut self, id: Ulid) {
        self.0.remove(&id);
    }

    pub fn start(worker: &mut Worker, id: Ulid, events: Vec<Event>, live: bool) {
        assert!(!worker.parsers.contains(id), "one parser owner per seat");
        if worker.parsers.1.is_empty() {
            // Independent CPU lanes, with at least one runtime worker left for
            // control/bus IO. A lane never owns a ring or history wait.
            let lanes = worker
                .handle
                .metrics()
                .num_workers()
                .saturating_sub(1)
                .clamp(1, 4);
            worker.parsers.1 = (0..lanes)
                .map(|_| Arc::new(tokio::sync::Semaphore::new(1)))
                .collect();
        }
        let lane = worker.parsers.1[(id.0 % worker.parsers.1.len() as u128) as usize].clone();
        let view = &worker.views[&id];
        let saved = view.saved();
        let from = view.sequence + 1;
        let through = events.last().unwrap().sequence().unwrap();
        let attachment = worker.attachments[&id].clone();
        let incarnation = view.view_generation;
        let recorder = worker.client.capture_diagnostics.clone();
        let ready = worker.ingestion_ready.clone();
        let (tx, result) = oneshot::channel();
        let task = worker.handle.spawn(async move {
            // CPU work is bounded by one existing ring page. There is no ring,
            // history, checkpoint, bus or filesystem wait on this seat task.
            let parse = recorder.start(Phase::Parse, id, saved.generation, from, through);
            let value: Result<Parsed> = async {
                let _cpu = lane
                    .acquire_owned()
                    .await
                    .map_err(|_| invalid("parser CPU lane stopped"))?;
                let mut state = Playback::restore(saved);
                let tap = OscTap::install(&mut state.term);
                let mut replies = Vec::new();
                let mut bytes = 0;
                let mut boundary = false;
                let mut yielded_bytes = 0;
                for (index, event) in events.iter().enumerate() {
                    if let Event::Output { bytes: output, .. } = event {
                        // The first record was observed before checkpoint admission.
                        // Observe later records in parser order, never pre-scan a
                        // future probe past the preceding records in this page.
                        if !live && index > 0 {
                            bbt_rt::diagnostics::probe_ring_output(
                                id.0,
                                state.generation,
                                event.sequence().unwrap(),
                                output,
                            );
                        }
                        bytes += output.len() as u64;
                    } else {
                        boundary = true;
                    }
                    let decode = bbt_rt::diagnostics::span(
                        bbt_rt::diagnostics::Phase::ReadRingTerminalDecode,
                    );
                    state.apply_with_effects(event, |effect| {
                        if live
                            && let bbt_core::Event::PtyWrite(bytes) = effect
                            && replies.len() < 32
                        {
                            replies.push(bytes.into_bytes());
                        }
                    })?;
                    decode.finish(1, 0);
                    if !live {
                        let _ = tap.drain();
                    }
                    if bytes - yielded_bytes >= 8 * 1024 || index % 32 == 31 {
                        tokio::task::yield_now().await;
                        yielded_bytes = bytes;
                    }
                }
                Ok(Parsed {
                    state,
                    tap,
                    replies,
                    events: events.len() as u64,
                    bytes,
                    boundary,
                })
            }
            .await;
            let (count, bytes) = value
                .as_ref()
                .map_or((0, 0), |p: &Parsed| (p.events, p.bytes));
            parse.finish(capture_outcome(&value), count, bytes);
            let _ = tx.send(value);
            ready.notify_one();
        });
        worker.parsers.0.insert(
            id,
            Job {
                attachment,
                incarnation,
                from,
                through,
                live,
                result,
                task,
            },
        );
    }

    pub fn take(
        &mut self,
        view: &ViewState,
        attachment: &Attachment,
        through: u64,
        live: bool,
    ) -> Result<Option<Parsed>> {
        let id = attachment.seat;
        let Some(job) = self.0.get_mut(&id) else {
            return Ok(None);
        };
        if job.attachment != *attachment
            || job.incarnation != view.view_generation
            || job.from != view.sequence + 1
            || job.through > through
            || job.live != live
        {
            self.retire(id);
            return Err(invalid("stale seat parser completion"));
        }
        let result = match job.result.try_recv() {
            Ok(result) => result,
            Err(oneshot::error::TryRecvError::Empty) => return Ok(None),
            Err(oneshot::error::TryRecvError::Closed) => Err(invalid("seat parser stopped")),
        };
        self.retire(id);
        result.map(Some)
    }
}
