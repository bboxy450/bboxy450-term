use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use crossbeam_queue::ArrayQueue;

use crate::{DirtySource, ExitStatus, OutputChunk, TaskId};

#[derive(Clone, Debug)]
pub enum UiMsg {
    Output(OutputChunk),
    Exited {
        task: TaskId,
        status: ExitStatus,
    },
    /// App-defined source/seat state lives in a latest-value slot.
    Changed {
        source: usize,
    },
}

#[derive(Clone)]
pub struct UiSender {
    queue: Arc<ArrayQueue<UiMsg>>,
    dirty: DirtySource,
}

pub struct UiReceiver(Arc<ArrayQueue<UiMsg>>, DirtySource);

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
    pub fn try_send(&self, message: UiMsg) -> Result<(), UiMsg> {
        self.queue.push(message)?;
        self.dirty.mark();
        Ok(())
    }
}

impl UiReceiver {
    pub fn try_recv(&self) -> Option<UiMsg> {
        self.0.pop()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
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
    pub fn new(dirty: DirtySource) -> Self {
        Self {
            value: ArrayQueue::new(1),
            dirty,
        }
    }
    pub fn publish(&self, value: T) {
        self.value.force_push(value);
        self.dirty.mark();
    }
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
    pub fn new(capacity: usize, dirty: DirtySource) -> Self {
        Self(Arc::new(Ring {
            queue: ArrayQueue::new(capacity),
            dropped: AtomicU64::new(0),
            dirty,
        }))
    }
    pub fn push(&self, item: T) {
        if self.0.queue.force_push(item).is_some() {
            self.0.dropped.fetch_add(1, Ordering::Relaxed);
        }
        self.0.dirty.mark();
    }
    pub fn pop(&self) -> Option<T> {
        self.0.queue.pop()
    }
    pub fn len(&self) -> usize {
        self.0.queue.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.queue.is_empty()
    }
    pub fn capacity(&self) -> usize {
        self.0.queue.capacity()
    }
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
