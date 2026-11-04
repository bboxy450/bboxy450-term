// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Headless worker + OS sampler cost; no providers, credentials, Keychain or network.
#![allow(
    clippy::disallowed_methods,
    reason = "standalone headless measurement harness owns its test runtime"
)]
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
fn cpu() -> f64 {
    unsafe {
        let mut usage: libc::rusage = std::mem::zeroed();
        assert_eq!(libc::getrusage(libc::RUSAGE_SELF, &mut usage), 0);
        (usage.ru_utime.tv_sec + usage.ru_stime.tv_sec) as f64
            + (usage.ru_utime.tv_usec + usage.ru_stime.tv_usec) as f64 / 1_000_000.
    }
}
fn main() {
    let home = std::path::PathBuf::from(std::env::var_os("HOME").expect("HOME"));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let samples = Arc::new(AtomicUsize::new(0));
    let healthy = Arc::new(AtomicBool::new(true));
    let count = samples.clone();
    let health = healthy.clone();
    let start = std::time::Instant::now();
    let before = cpu();
    runtime.block_on(async {
        let worker = bboxy450_usage::poller::Worker::start(
            runtime.handle(),
            home.clone(),
            vec![],
            Arc::new(bboxy450_usage::io::LiveReader { home }),
            Arc::new(bboxy450_usage::io::LiveHttp::default()),
            Arc::new(move |update| {
                if let bboxy450_usage::poller::Update::Machine(m) = update {
                    let previous = count.fetch_add(1, Ordering::SeqCst);
                    if m.memory.is_none() || m.disk.is_none() || (previous > 0 && m.cpu.is_none()) {
                        health.store(false, Ordering::SeqCst);
                    }
                }
            }),
            Default::default(),
        );
        tokio::time::sleep(std::time::Duration::from_secs(32)).await;
        drop(worker);
    });
    let elapsed = start.elapsed().as_secs_f64();
    let cpu = cpu() - before;
    let percent = cpu / elapsed * 100.;
    let samples = samples.load(Ordering::SeqCst);
    println!(
        "{{\"samples\":{samples},\"elapsed_seconds\":{elapsed:.6},\"cpu_seconds\":{cpu:.6},\"one_core_percent\":{percent:.6}}}"
    );
    assert!(
        healthy.load(Ordering::SeqCst) && samples >= 16,
        "invalid samples"
    );
    assert!(percent < 0.5, "worker exceeded sampler CPU budget");
}
