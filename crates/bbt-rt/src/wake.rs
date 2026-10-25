use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

/// An app's non-blocking event-loop wake (for example EventLoopProxy).
pub trait Waker: Send + Sync + 'static {
    fn wake(&self);
    /// Service non-visual work on the app's bounded deadline. Consumers without
    /// a display loop may service it immediately.
    fn wake_deferred(&self) {
        self.wake();
    }
}

struct Word {
    bits: AtomicU64,
    retained: AtomicU64,
    deferred: AtomicU64,
}

struct Shared {
    words: Box<[Word]>,
    sources: usize,
    pending: AtomicBool,
    waker: Arc<dyn Waker>,
}

#[derive(Clone)]
pub struct DirtySet(Arc<Shared>);

#[derive(Clone)]
pub struct DirtySource {
    shared: Arc<Shared>,
    index: usize,
    mode: Mode,
}

#[derive(Clone, Copy)]
enum Mode {
    Immediate,
    Retained,
    Deferred,
}

impl Shared {
    fn post(&self) {
        if !self.pending.swap(true, Ordering::AcqRel) {
            self.waker.wake();
        }
    }
}

impl DirtySet {
    pub fn new(sources: usize, waker: Arc<dyn Waker>) -> Self {
        assert!(sources > 0);
        Self(Arc::new(Shared {
            words: (0..sources.div_ceil(64))
                .map(|_| Word {
                    bits: AtomicU64::new(0),
                    retained: AtomicU64::new(0),
                    deferred: AtomicU64::new(0),
                })
                .collect(),
            sources,
            pending: AtomicBool::new(false),
            waker,
        }))
    }

    pub fn source(&self, index: usize) -> DirtySource {
        self.with_mode(index, Mode::Immediate)
    }

    /// Coalesce publications until their consumer completes a real draw.
    pub fn paint_source(&self, index: usize) -> DirtySource {
        self.with_mode(index, Mode::Retained)
    }

    /// Coalesce non-visual publications until a bounded consumer drain.
    pub fn deferred_source(&self, index: usize) -> DirtySource {
        assert!(index < self.0.sources);
        self.0.words[index / 64]
            .deferred
            .fetch_or(1 << (index % 64), Ordering::Release);
        self.with_mode(index, Mode::Deferred)
    }

    fn with_mode(&self, index: usize, mode: Mode) -> DirtySource {
        assert!(index < self.0.sources);
        DirtySource {
            shared: self.0.clone(),
            index,
            mode,
        }
    }

    /// Take posted source bits without retiring paint/queue consumption.
    /// Retire event admission after the take, then re-check immediate work so
    /// a publication racing the take cannot lose its only wake. Deferred work
    /// already has a deadline; it must not escape that deadline in this re-check.
    pub fn drain(&self) -> Vec<usize> {
        let mut sources = Vec::new();
        for (word, row) in self.0.words.iter().enumerate() {
            let mut value =
                row.bits.swap(0, Ordering::AcqRel) | row.retained.load(Ordering::Acquire);
            while value != 0 {
                sources.push(word * 64 + value.trailing_zeros() as usize);
                value &= value - 1;
            }
        }
        self.0.pending.swap(false, Ordering::AcqRel);
        if self.0.words.iter().any(|row| {
            row.bits.load(Ordering::Acquire) & !row.deferred.load(Ordering::Acquire) != 0
        }) {
            self.0.post();
        }
        sources
    }
}

impl DirtySource {
    pub fn mark(&self) {
        let row = &self.shared.words[self.index / 64];
        let bit = 1 << (self.index % 64);
        if !matches!(self.mode, Mode::Immediate)
            && row.retained.fetch_or(bit, Ordering::AcqRel) & bit != 0
        {
            return;
        }
        row.bits.fetch_or(bit, Ordering::Release);
        if matches!(self.mode, Mode::Deferred) {
            self.shared.waker.wake_deferred();
        } else {
            self.shared.post();
        }
    }

    pub(crate) fn complete_drain(&self) {
        self.shared.words[self.index / 64]
            .retained
            .fetch_and(!(1 << (self.index % 64)), Ordering::AcqRel);
    }
}
