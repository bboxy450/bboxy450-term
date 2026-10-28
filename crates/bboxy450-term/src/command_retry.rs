// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One capacity notification for retained GUI commands; never a display wake.
use bboxy450_term::cockpit::UiMsg;
use tokio::{
    sync::mpsc::{OwnedPermit, Sender},
    task::JoinHandle,
};

#[derive(Debug)]
pub struct Ready {
    epoch: u64,
    permit: Result<OwnedPermit<UiMsg>, ()>,
}

#[derive(Default)]
pub struct Retry {
    epoch: u64,
    task: Option<JoinHandle<()>>,
}
impl Retry {
    /// Holding the permit until the UI handles the event prevents a second
    /// producer from taking the capacity that caused this notification.
    pub fn arm(&mut self, sender: &Sender<UiMsg>, notify: impl FnOnce(Ready) + Send + 'static) {
        if self.task.is_some() {
            return;
        }
        self.epoch = self.epoch.wrapping_add(1);
        let epoch = self.epoch;
        let sender = sender.clone();
        self.task = Some(bbt_rt::handle().spawn(async move {
            let permit = sender.reserve_owned().await.map_err(|_| ());
            notify(Ready { epoch, permit });
        }));
    }

    pub fn accept(&mut self, ready: Ready) -> Option<Result<OwnedPermit<UiMsg>, ()>> {
        if ready.epoch != self.epoch || self.task.take().is_none() {
            return None;
        }
        Some(ready.permit)
    }

    pub fn cancel(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
impl Drop for Retry {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // keeps: spec ctl-cli §1a
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // The test macro owns a runtime; UI delivery never blocks.
    async fn capacity_is_reserved_once_and_stale_events_release_it() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let (wake, mut wakes) = tokio::sync::mpsc::unbounded_channel();
        assert!(sender.try_send(UiMsg::Watched(vec![])).is_ok());
        let mut retry = Retry::default();
        for _ in 0..120 {
            let wake = wake.clone();
            retry.arm(&sender, move |ready| {
                let _ = wake.send(ready);
            });
        }
        assert!(wakes.try_recv().is_err());
        receiver.recv().await.unwrap();
        let ready = tokio::time::timeout(Duration::from_secs(30), wakes.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(sender.capacity(), 0, "the event owns the free slot");
        assert!(wakes.try_recv().is_err(), "one event for all retained work");
        retry.cancel();
        assert!(retry.accept(ready).is_none());
        assert_eq!(sender.capacity(), 1, "stale event released its permit");
        let wake = wake.clone();
        retry.arm(&sender, move |ready| {
            let _ = wake.send(ready);
        });
        let ready = tokio::time::timeout(Duration::from_secs(30), wakes.recv())
            .await
            .unwrap()
            .unwrap();
        retry
            .accept(ready)
            .unwrap()
            .unwrap()
            .send(UiMsg::Watched(vec![]));
        assert!(matches!(receiver.recv().await, Some(UiMsg::Watched(_))));
        assert!(wakes.try_recv().is_err());
    }

    // keeps: spec ctl-cli §1a
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // The test macro owns a runtime; UI delivery never blocks.
    async fn a_closed_receiver_wakes_delivery_once_without_polling() {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (wake, mut wakes) = tokio::sync::mpsc::unbounded_channel();
        assert!(sender.try_send(UiMsg::Watched(vec![])).is_ok());
        let mut retry = Retry::default();
        retry.arm(&sender, move |ready| {
            let _ = wake.send(ready);
        });
        drop(receiver);
        let ready = tokio::time::timeout(Duration::from_secs(30), wakes.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(retry.accept(ready).unwrap().is_err());
        assert!(wakes.try_recv().is_err());
    }

    // keeps: spec ctl-cli §1a
    #[tokio::test]
    #[allow(clippy::disallowed_methods)] // The test macro owns a runtime; UI delivery never blocks.
    async fn busy_delivery_resumes_in_fifo_order_without_a_frame_or_poll() {
        use std::collections::VecDeque;
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let (wake, mut wakes) = tokio::sync::mpsc::unbounded_channel();
        assert!(sender.try_send(UiMsg::Watched(vec![])).is_ok());
        let seat = bbt_proto::SeatId::from_u128(42);
        let ids = [bbt_mux::Ulid::new(), bbt_mux::Ulid::new()];
        let mut pending = VecDeque::new();
        for (generation, id, bytes) in [(7, ids[0], b"first"), (8, ids[1], b"later")] {
            crate::queue_pending(
                &mut pending,
                UiMsg::Input {
                    seat,
                    generation,
                    id,
                    bytes: bytes.to_vec(),
                },
            )
            .unwrap();
        }
        crate::flush_pending(&sender, &mut pending, true).unwrap();
        assert_eq!(pending.len(), 2, "paused delivery retains the same ids");
        crate::flush_pending(&sender, &mut pending, false).unwrap();
        assert_eq!(pending.len(), 2, "Busy retains the front message");
        let mut retry = Retry::default();
        let notify = wake.clone();
        retry.arm(&sender, move |ready| {
            let _ = notify.send(ready);
        });
        assert!(wakes.try_recv().is_err());
        assert!(matches!(receiver.recv().await, Some(UiMsg::Watched(_))));

        for (generation, id, expected) in [(7, ids[0], b"first"), (8, ids[1], b"later")] {
            let ready = tokio::time::timeout(Duration::from_secs(30), wakes.recv())
                .await
                .unwrap()
                .unwrap();
            retry
                .accept(ready)
                .unwrap()
                .unwrap()
                .send(pending.pop_front().unwrap());
            crate::flush_pending(&sender, &mut pending, false).unwrap();
            if !pending.is_empty() {
                let wake = wake.clone();
                retry.arm(&sender, move |ready| {
                    let _ = wake.send(ready);
                });
            }
            // The only resumption is the capacity callback above: no frame,
            // delay or display-link tick is involved in delivering either input.
            let Some(UiMsg::Input {
                generation: actual,
                id: actual_id,
                bytes,
                ..
            }) = receiver.recv().await
            else {
                panic!("expected retained input");
            };
            assert_eq!(
                (actual, actual_id, bytes.as_slice()),
                (generation, id, expected.as_slice())
            );
        }
        assert!(pending.is_empty());
        assert!(
            wakes.try_recv().is_err(),
            "empty delivery schedules no idle wake"
        );
    }
}
