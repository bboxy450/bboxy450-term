#![cfg(feature = "runtime")]
use std::time::{Duration, Instant};

// keeps: spec runtime §6 — real two-second file-read boundary, measured in the night
#[test]
#[ignore = "#327 B4: real two-second native measurement; scripts/night-test-boundaries.sh"]
#[allow(
    clippy::disallowed_methods,
    reason = "native night measurement completion guard, not the application event loop"
)]
fn real_two_second_file_stall_reports_native_measurements() {
    bbt_rt::register_ui_thread();
    let cpu = bbt_rt::fault::thread_cpu_time().unwrap();
    let start = Instant::now();
    let (done, receive) = std::sync::mpsc::sync_channel(1);
    bbt_rt::handle().spawn(async move {
        let result = bbt_rt::fault::stalled_file_read(Duration::from_secs(2)).await;
        let _ = done.try_send(result);
    });
    let read = receive
        .recv_timeout(Duration::from_secs(120))
        .expect("native read hang guard")
        .unwrap();
    assert_eq!(read.bytes, b"R5 stalled file read released\n");
    assert_eq!(bbt_rt::ui_blocking_violations(), 0);
    eprintln!(
        "native file-stall measurement: read={:?}, driver={:?}, driver_cpu={:?}",
        read.elapsed,
        start.elapsed(),
        bbt_rt::fault::thread_cpu_time()
            .unwrap()
            .saturating_sub(cpu)
    );
}
