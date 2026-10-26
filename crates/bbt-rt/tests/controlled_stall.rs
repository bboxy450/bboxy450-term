#![cfg(feature = "runtime")]
use std::time::Duration;

struct Wake;
impl bbt_rt::Waker for Wake {
    fn wake(&self) {}
}

// keeps: spec runtime §1 and §6 — memory-only UI handoff progresses while IO is held
#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "test driver completion guards, not application UI scheduling"
)]
fn held_file_read_allows_exact_ui_messages_before_release() {
    let inbox = bbt_rt::init_ui(Wake);
    let (ready, started) = std::sync::mpsc::sync_channel(1);
    let (release, held) = tokio::sync::oneshot::channel();
    let (done, receive) = std::sync::mpsc::sync_channel(1);
    bbt_rt::handle().spawn(async move {
        let result = bbt_rt::fault::stalled_file_read_until(async {
            ready.send(()).unwrap();
            held.await.unwrap();
        })
        .await;
        let _ = done.send(result);
    });
    started
        .recv_timeout(Duration::from_secs(120))
        .expect("IO-ready hang guard");
    assert!(matches!(
        receive.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    for source in 0..100 {
        inbox
            .sender
            .try_send(bbt_rt::UiMsg::Changed { source })
            .unwrap();
        assert!(
            matches!(inbox.messages.try_recv(), Some(bbt_rt::UiMsg::Changed { source: actual }) if actual == source)
        );
        inbox.messages.complete_drain();
    }
    assert!(matches!(
        receive.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    release.send(()).unwrap();
    let read = receive
        .recv_timeout(Duration::from_secs(120))
        .expect("released read hang guard")
        .unwrap();
    assert_eq!(read.bytes, b"R5 stalled file read released\n");
    assert_eq!(bbt_rt::ui_blocking_violations(), 0);
}
