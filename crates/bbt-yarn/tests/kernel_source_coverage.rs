#![allow(missing_docs)]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, Wake, Waker};

use bbt_yarn::source::{
    BacklogSource, DrainableSource, FixedQueue, Latched, ReactorSource, TryNext,
};

struct WakeCounter(AtomicUsize);

impl Wake for WakeCounter {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn counting_context() -> (Arc<WakeCounter>, Waker) {
    let counter = Arc::new(WakeCounter(AtomicUsize::new(0)));
    let waker = Waker::from(Arc::clone(&counter));
    (counter, waker)
}

// keeps: spec yarn §§0,4 — public source, arbitration, phase or compiler contract
#[test]
fn latched_preserves_the_first_event_and_dormant_polls_do_not_self_wake() {
    let mut source = Latched::new();
    let (wake_counter, waker) = counting_context();
    let mut context = Context::from_waker(&waker);

    assert!(source.is_dormant());
    assert!(!source.has_backlog());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);

    source.arm(11).expect("a dormant latch accepts an event");
    assert!(!source.is_dormant());
    assert!(source.has_backlog());
    let rejected = source
        .arm(12)
        .expect_err("an armed latch rejects replacement");
    assert_eq!(rejected.into_inner(), 12);
    assert_eq!(source.poll_next(&mut context), Poll::Ready(11));
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert!(source.is_dormant());
    assert!(!source.has_backlog());
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);

    source.arm(13).expect("a delivered latch can be rearmed");
    assert_eq!(source.disarm(), Some(13));
    assert!(source.is_dormant());
}

// keeps: spec yarn §§0,4 — public source, arbitration, phase or compiler contract
#[test]
fn fixed_queue_preserves_fifo_order_across_wrap_full_and_empty_transitions() {
    let mut source = FixedQueue::<u8, 3>::default();
    let (wake_counter, waker) = counting_context();
    let mut context = Context::from_waker(&waker);

    assert_eq!(source.len(), 0);
    assert!(source.is_empty());
    assert!(!source.has_backlog());
    assert_eq!(source.poll_next(&mut context), Poll::Pending);

    source.push(10).unwrap();
    source.push(20).unwrap();
    source.push(30).unwrap();
    assert_eq!(source.len(), 3);
    assert!(source.has_backlog());
    assert_eq!(source.push(99).unwrap_err().into_inner(), 99);

    assert_eq!(source.poll_next(&mut context), Poll::Ready(10));
    source
        .push(40)
        .expect("the freed ring slot accepts an item");
    assert_eq!(source.try_next(), TryNext::Item(20));
    assert_eq!(source.try_next(), TryNext::Item(30));
    assert_eq!(source.try_next(), TryNext::Item(40));
    assert_eq!(source.try_next(), TryNext::Empty);
    assert_eq!(source.poll_next(&mut context), Poll::Pending);
    assert_eq!(source.len(), 0);
    assert!(source.is_empty());
    assert!(!source.has_backlog());
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);

    let mut zero = FixedQueue::<u8, 0>::new();
    assert_eq!(zero.push(77).unwrap_err().into_inner(), 77);
    assert_eq!(zero.try_next(), TryNext::Empty);
    assert_eq!(zero.poll_next(&mut context), Poll::Pending);
    assert_eq!(zero.len(), 0);
    assert!(zero.is_empty());
    assert!(!zero.has_backlog());
    assert_eq!(wake_counter.0.load(Ordering::SeqCst), 0);
}
