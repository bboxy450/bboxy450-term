// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Test-only shared-memory admission counter. Publishing the head is the
//! observed admission point, while the queue mutex still excludes the writer.
use std::fs::OpenOptions;
use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};

const LENGTH: usize = 8192;
const SLOTS: usize = 256;

pub(super) struct Probe(NonNull<AtomicU64>);
// SAFETY: the mapping is owned by Probe and lives until Drop. The queue mutex
// serializes this process's writer; all shared words are atomic across processes.
unsafe impl Send for Probe {}

impl Probe {
    pub(super) fn create(root: &Path) -> io::Result<()> {
        std::fs::create_dir_all(root)?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("admission.probe"))?
            .set_len(LENGTH as u64)
    }
    pub(super) fn open(root: &Path) -> io::Result<Option<Self>> {
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join("admission.probe"))
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        if file.metadata()?.len() != LENGTH as u64 {
            return Err(io::Error::other("invalid admission probe length"));
        }
        // SAFETY: a page-aligned, read/write shared mapping of an owned fixed
        // length file. Every accessed AtomicU64 is within the initialized mapping.
        let pointer = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                LENGTH,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED,
                file.as_raw_fd(),
                0,
            )
        };
        if pointer == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }
        Ok(Some(Self(NonNull::new(pointer.cast()).unwrap())))
    }
    fn word(&self, index: usize) -> &AtomicU64 {
        assert!(index < LENGTH / 8);
        // SAFETY: mmap is aligned for AtomicU64, the bounds above cover every
        // access, and Drop cannot run while this shared borrow exists.
        unsafe { &*self.0.as_ptr().add(index) }
    }
    fn entry(&self, head: u64) -> (u64, u64, u64) {
        let i = 1 + (head as usize % SLOTS) * 3;
        (
            self.word(i).load(Ordering::Relaxed),
            self.word(i + 1).load(Ordering::Relaxed),
            self.word(i + 2).load(Ordering::Relaxed),
        )
    }
    pub(super) fn record(&self, bytes: usize) {
        let head = self.word(0).load(Ordering::Acquire);
        let end = if head == 0 { 0 } else { self.entry(head).0 } + bytes as u64;
        let mut now = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: clock_gettime writes one valid timespec and has no side effects.
        assert_eq!(
            unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut now) },
            0
        );
        let ns = now.tv_sec as u64 * 1_000_000_000 + now.tv_nsec as u64;
        let next = head + 1;
        let i = 1 + (next as usize % SLOTS) * 3;
        self.word(i).store(end, Ordering::Relaxed);
        self.word(i + 1).store(ns, Ordering::Relaxed);
        self.word(i + 2).store(next, Ordering::Relaxed);
        self.word(0).store(next, Ordering::Release);
    }
    pub(super) fn snapshot(&self) -> (u64, u64) {
        let head = self.word(0).load(Ordering::Acquire);
        if head == 0 {
            return (0, 0);
        }
        let (end, now, _) = self.entry(head);
        let before = (head.saturating_sub(SLOTS as u64 - 1).max(1)..=head)
            .filter_map(|seq| {
                let (bytes, time, generation) = self.entry(seq);
                (generation == seq && bytes <= end && time <= now.saturating_sub(20_000_000))
                    .then_some(bytes)
            })
            .max()
            .unwrap_or(0);
        (end, end - before)
    }
}
impl Drop for Probe {
    fn drop(&mut self) {
        // SAFETY: this is exactly the mapping owned by this Probe, after all
        // borrows have ended. Other processes own independent mappings.
        unsafe {
            libc::munmap(self.0.as_ptr().cast(), LENGTH);
        }
    }
}
