// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md

use super::{Action, Attachment, Command, Duration, Lifecycle, Result, SeatRecord, Ulid, Worker};
use crate::adapters::native;
use crate::registry::CloseRecord;
use crate::store::now_ms;

pub(super) struct ArchiveTimer {
    operation: Ulid,
    timer: Option<tokio::task::JoinHandle<()>>,
}

impl ArchiveTimer {
    pub(super) fn started(&self, operation: Ulid) -> bool {
        self.operation == operation && self.timer.is_none()
    }
}

impl Drop for ArchiveTimer {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
    }
}

impl Worker {
    pub(super) fn schedule_seat_close(
        &mut self,
        attachment: Attachment,
        close: &CloseRecord,
    ) -> Result<()> {
        let id = attachment.seat;
        let generation = attachment.generation;
        let view = &self.views[&id];
        let shared_codex = view.shared_codex_runtime.is_some();
        if view
            .codex_close
            .as_ref()
            .is_some_and(|archive| archive.operation == close.operation)
        {
            return Ok(());
        }
        if shared_codex && view.info.exited.is_some() {
            let record = self.registry.get(id)?;
            self.archive_on_exit(&record);
            return self.registry.exited(id, generation);
        }
        // The original deadline still ends undo. Give the bounded native RPC
        // a head start while the supervisor retains an independent stop deadline.
        let deadline_ms = if shared_codex {
            close
                .deadline_ms
                .saturating_add(native::DEADLINE.as_millis() as u64 + 100)
                .min(now_ms().saturating_add(86_400_000))
        } else {
            close.deadline_ms
        };
        self.request_for_seat(
            id,
            Action::ScheduleClose {
                attachment,
                close_id: close.operation,
                deadline_ms,
            },
        )?;
        if shared_codex {
            self.arm_archive(id, generation, close);
        }
        Ok(())
    }

    fn arm_archive(&mut self, id: Ulid, generation: u64, close: &CloseRecord) {
        let tx = self.client.tx.clone();
        let operation = close.operation;
        let sleep = Duration::from_millis(close.deadline_ms.saturating_sub(now_ms()));
        let timer = self.handle.spawn(async move {
            tokio::time::sleep(sleep).await;
            let _ = tx
                .send(Command::CodexCloseDue(id, generation, operation))
                .await;
        });
        self.views.get_mut(&id).unwrap().codex_close = Some(ArchiveTimer {
            operation,
            timer: Some(timer),
        });
    }

    pub(super) fn archive_due(&mut self, id: Ulid, generation: u64, operation: Ulid) {
        let Ok(record) = self.registry.get(id) else {
            return;
        };
        if record.generation != generation || record.lifecycle != Lifecycle::ClosedGrace {
            return;
        }
        let Some(close) = record
            .close
            .as_ref()
            .filter(|close| close.operation == operation)
        else {
            return;
        };
        if !self.views.get(&id).is_some_and(|view| {
            view.generation == generation
                && view.codex_close.as_ref().is_some_and(|archive| {
                    archive.operation == operation && archive.timer.is_some()
                })
        }) {
            return;
        }
        if now_ms() < close.deadline_ms {
            self.arm_archive(id, generation, close);
        } else {
            self.archive_codex(&record, operation);
        }
    }

    pub(super) fn archive_on_exit(&mut self, record: &SeatRecord) {
        // Recovery and an early TUI exit can reach the close before its timer.
        // The effect uses a direct daemon connection, independent of the relay.
        if record.lifecycle == Lifecycle::ClosedGrace
            && let Some(close) = &record.close
        {
            self.archive_codex(record, close.operation);
        }
    }

    fn archive_codex(&mut self, record: &SeatRecord, operation: Ulid) {
        let id = record.id;
        let Some(view) = self.views.get_mut(&id) else {
            return;
        };
        if view.generation != record.generation
            || view
                .codex_close
                .as_ref()
                .is_some_and(|archive| archive.started(operation))
        {
            return;
        }
        let Some(path) = view.shared_codex_runtime.clone() else {
            return;
        };
        let binding = view.adapter.binding.clone();
        if record.harness_session_id.as_ref() != Some(&binding.session) {
            return;
        }
        view.codex_close = Some(ArchiveTimer {
            operation,
            timer: None,
        });
        let diagnostics = self.config.run_dir.clone();
        self.handle.spawn(async move {
            if let Err(error) = native::archive_shared_codex_thread(&binding, &path).await {
                bbt_rt::error_journal::report_to(
                    &diagnostics,
                    bbt_rt::error_journal::Failure::new("native", "codex_close_archive", error)
                        .seat(id)
                        .harness("codex"),
                );
            }
        });
    }
}
