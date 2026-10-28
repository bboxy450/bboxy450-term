// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Standalone, synthetic PTY producer compiled only by the ignored crash test.
//! It reports successful writes independently of the supervisor's mmap ledger.
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static STOP: AtomicBool = AtomicBool::new(false);
unsafe extern "C" {
    fn signal(number: i32, handler: usize) -> usize;
    fn write(fd: i32, bytes: *const std::ffi::c_void, length: usize) -> isize;
}
extern "C" fn hangup(_: i32) {
    STOP.store(true, Ordering::Relaxed);
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let final_path = std::path::PathBuf::from(args.next().unwrap());
    let go = std::path::PathBuf::from(args.next().unwrap());
    // SAFETY: the signal handler only stores to a lock-free atomic flag.
    assert_ne!(
        unsafe { signal(1, hangup as *const () as usize) },
        usize::MAX
    );
    while !go.exists() && !STOP.load(Ordering::Relaxed) {
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut written = 0u64;
    let mut frame = [b'x'; 64];
    let mut index = 0u64;
    let mut max_write_ns = 0u128;
    'stream: while !STOP.load(Ordering::Relaxed) {
        frame[..8].copy_from_slice(format!("{index:08x}").as_bytes());
        let mut position = 0;
        while position < frame.len() {
            let start = Instant::now();
            // SAFETY: this slice stays alive for the write to our own PTY.
            let n = unsafe { write(1, frame[position..].as_ptr().cast(), frame.len() - position) };
            max_write_ns = max_write_ns.max(start.elapsed().as_nanos());
            if n < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                break 'stream;
            }
            if n == 0 {
                break 'stream;
            }
            written += n as u64;
            position += n as usize;
        }
        index += 1;
        std::thread::sleep(Duration::from_millis(2));
    }
    let temporary = final_path.with_extension("tmp");
    std::fs::write(
        &temporary,
        format!("{{\"written\":{written},\"max_write_ns\":{max_write_ns}}}"),
    )
    .unwrap();
    std::fs::rename(temporary, final_path).unwrap();
}
