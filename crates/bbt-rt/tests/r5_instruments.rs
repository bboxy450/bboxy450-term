#![cfg(feature = "runtime")]
use std::time::{Duration, Instant};

#[test]
fn real_file_read_stalls_off_ui_while_thread_cpu_and_ui_progress_remain_observable() {
    let (done, receive) = std::sync::mpsc::sync_channel(1);
    bbt_rt::handle().spawn(async move {
        let result = bbt_rt::fault::stalled_file_read(Duration::from_secs(2)).await;
        let _ = done.try_send(result);
    });
    bbt_rt::register_ui_thread();
    let cpu = bbt_rt::fault::thread_cpu_time().unwrap();
    let start = Instant::now();
    let mut frames = 0;
    let read = loop {
        match receive.try_recv() {
            Ok(result) => break result.unwrap(),
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
            Err(error) => panic!("read task stopped: {error}"),
        }
        let slice = Instant::now();
        while slice.elapsed() < Duration::from_millis(1) {
            std::hint::spin_loop();
        }
        frames += 1;
        assert!(start.elapsed() < Duration::from_secs(5));
    };
    assert_eq!(read.bytes, b"R5 stalled file read released\n");
    assert!(read.elapsed >= Duration::from_secs(2));
    assert!(
        frames >= 100,
        "UI frame surrogate stopped while file read was pending"
    );
    assert!(bbt_rt::fault::thread_cpu_time().unwrap() > cpu);
    assert_eq!(bbt_rt::ui_blocking_violations(), 0);
}
