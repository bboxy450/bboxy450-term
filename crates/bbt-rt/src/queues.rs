// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use crossbeam_queue::ArrayQueue;

use crate::{DirtySource, ExitStatus, OutputChunk, TaskId};

/// An individual output, completion or state-change notice for the UI owner.
#[derive(Clone, Debug)]
pub enum UiMsg {
    /// A bounded chunk from one command pipe.
    Output(OutputChunk),
    /// Final command status; also retained independently in the task handle.
    Exited {
        /// Identity whose completion this notice reports.
        task: TaskId,
        /// Final process and output-drain result.
        status: ExitStatus,
    },
    /// Application-defined replaceable state lives in a latest-value slot.
    Changed {
        /// Application-defined source index whose latest state can be read.
        source: usize,
    },
}

/// Cloneable producer for a bounded, non-waiting UI message channel.
#[derive(Clone)]
pub struct UiSender {
    queue: Arc<ArrayQueue<UiMsg>>,
    dirty: DirtySource,
}

/// Consumer of individual messages; finish each bounded drain with `complete_drain`.
pub struct UiReceiver(Arc<ArrayQueue<UiMsg>>, DirtySource);

/// Create a bounded channel that marks the supplied source after successful admission.
///
/// # Panics
/// Panics when `capacity` is zero.
pub fn ui_channel(capacity: usize, dirty: DirtySource) -> (UiSender, UiReceiver) {
    let queue = Arc::new(ArrayQueue::new(capacity));
    (
        UiSender {
            queue: queue.clone(),
            dirty: dirty.clone(),
        },
        UiReceiver(queue, dirty),
    )
}

impl UiSender {
    /// Returns ownership on saturation. Never waits for the UI to consume.
    ///
    /// # Errors
    /// Returns the original message when the bounded queue is full.
    pub fn try_send(&self, message: UiMsg) -> Result<(), UiMsg> {
        self.queue.push(message)?;
        self.dirty.mark();
        Ok(())
    }
}

impl UiReceiver {
    /// Take one admitted message, or return `None` without waiting.
    pub fn try_recv(&self) -> Option<UiMsg> {
        self.0.pop()
    }
    /// Observe the number of currently queued items; concurrent producers may change it.
    pub fn len(&self) -> usize {
        self.0.len()
    }
    /// Observe whether the queue is empty without waiting for producers.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// Retire the consumed burst, then re-check for a racing publication.
    pub fn complete_drain(&self) {
        self.1.complete_drain();
        if !self.is_empty() {
            self.1.mark();
        }
    }
}

/// A single pending value. The UI retains its previous value when `take` is
/// empty. Replacing an unread value never queues obsolete intermediate states.
pub struct Latest<T> {
    value: ArrayQueue<T>,
    dirty: DirtySource,
}

impl<T> Latest<T> {
    /// Create an empty single-value slot bound to the supplied wake source.
    pub fn new(dirty: DirtySource) -> Self {
        Self {
            value: ArrayQueue::new(1),
            dirty,
        }
    }
    /// Replace any unread value and mark the source; evicted values drop on this thread.
    pub fn publish(&self, value: T) {
        self.value.force_push(value);
        self.dirty.mark();
    }
    /// Take the latest pending value, or return `None` while retaining no history.
    pub fn take(&self) -> Option<T> {
        self.value.pop()
    }
    /// A draw consumed the retained value. A newer unread value gets its own
    /// wake even if its publication raced the just-completed draw.
    pub fn complete_drain(&self) {
        self.dirty.complete_drain();
        if !self.value.is_empty() {
            self.dirty.mark();
        }
    }
}

struct Ring<T> {
    queue: ArrayQueue<T>,
    dropped: AtomicU64,
    dirty: DirtySource,
}

/// Bounded by item count. Command items are at most 16 KiB each. This is a
/// disposable view, never the durable byte ring or the store's retention policy.
pub struct ViewRing<T>(Arc<Ring<T>>);

impl<T> Clone for ViewRing<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> ViewRing<T> {
    /// Create a disposable ring bounded by item count, with no bytes-size enforcement.
    ///
    /// # Panics
    /// Panics when `capacity` is zero.
    pub fn new(capacity: usize, dirty: DirtySource) -> Self {
        Self(Arc::new(Ring {
            queue: ArrayQueue::new(capacity),
            dropped: AtomicU64::new(0),
            dirty,
        }))
    }
    /// Append an item, dropping the oldest on saturation and counting the eviction.
    pub fn push(&self, item: T) {
        if self.0.queue.force_push(item).is_some() {
            self.0.dropped.fetch_add(1, Ordering::Relaxed);
        }
        self.0.dirty.mark();
    }
    /// Take the oldest retained item without waiting.
    pub fn pop(&self) -> Option<T> {
        self.0.queue.pop()
    }
    /// Observe the number of currently queued items; concurrent producers may change it.
    pub fn len(&self) -> usize {
        self.0.queue.len()
    }
    /// Observe whether the queue is empty without waiting for producers.
    pub fn is_empty(&self) -> bool {
        self.0.queue.is_empty()
    }
    /// Return the fixed maximum retained item count.
    pub fn capacity(&self) -> usize {
        self.0.queue.capacity()
    }
    /// Return the cumulative number of items evicted by saturated pushes.
    pub fn dropped(&self) -> u64 {
        self.0.dropped.load(Ordering::Relaxed)
    }
    /// Retire a budgeted drain without stranding its unconsumed tail.
    pub fn complete_drain(&self) {
        self.0.dirty.complete_drain();
        if !self.is_empty() {
            self.0.dirty.mark();
        }
    }
    pub(crate) fn mark_dirty(&self) {
        self.0.dirty.mark();
    }
}
