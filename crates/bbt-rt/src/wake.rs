use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

/// An app's non-blocking event-loop wake (for example EventLoopProxy).
pub trait Waker: Send + Sync + 'static {
    fn wake(&self);
}

struct Shared {
    bits: Box<[AtomicU64]>,
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
}

impl DirtySet {
    pub fn new(sources: usize, waker: Arc<dyn Waker>) -> Self {
        assert!(sources > 0);
        Self(Arc::new(Shared {
            bits: (0..sources.div_ceil(64))
                .map(|_| AtomicU64::new(0))
                .collect(),
            sources,
            pending: AtomicBool::new(false),
            waker,
        }))
    }

    pub fn source(&self, index: usize) -> DirtySource {
        assert!(index < self.0.sources);
        DirtySource {
            shared: self.0.clone(),
            index,
        }
    }

    /// UI-only at frame start. Reset wake admission BEFORE clearing bits: a
    /// racing publication is either drained now or has a wake for the next frame.
    pub fn drain(&self) -> Vec<usize> {
        self.0.pending.store(false, Ordering::Release);
        let mut sources = Vec::new();
        for (word, bits) in self.0.bits.iter().enumerate() {
            let mut value = bits.swap(0, Ordering::AcqRel);
            while value != 0 {
                sources.push(word * 64 + value.trailing_zeros() as usize);
                value &= value - 1;
            }
        }
        sources
    }
}

impl DirtySource {
    pub fn mark(&self) {
        self.shared.bits[self.index / 64].fetch_or(1 << (self.index % 64), Ordering::Release);
        if !self.shared.pending.swap(true, Ordering::AcqRel) {
            self.shared.waker.wake();
        }
    }
}
