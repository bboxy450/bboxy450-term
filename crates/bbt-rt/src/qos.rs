// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Scheduling classes. The threads that carry PTY bytes to the screen run
//! user-interactive, so a parallel build cannot starve them. A process inherits
//! the class of the thread that spawns it: seats and the daemons that run their
//! tools are spawned user-initiated (above background work, below the terminal
//! path), and plugins and helper commands at default.
//!
//! macOS has QoS classes. Linux has none; it gets the same order from the
//! thread's nice value, which a forked child inherits: the terminal path asks
//! for nice -5 (granted only with CAP_SYS_NICE or a raised RLIMIT_NICE), and
//! everything spawned runs at nice 0, never inheriting the terminal path's.

/// Mark the calling thread user-interactive.
pub fn interactive_thread() {
    #[cfg(target_os = "macos")]
    // SAFETY: changes only the calling thread's own QoS class; no pointers.
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
    }
    #[cfg(target_os = "linux")]
    linux::set_nice(linux::INTERACTIVE);
}

/// Run `spawn` at the seat class (user-initiated): a seat and every process it
/// starts outrank background work but not the terminal path.
pub fn with_seat_qos<T>(spawn: impl FnOnce() -> T) -> T {
    #[cfg(target_os = "macos")]
    return with_class(libc::qos_class_t::QOS_CLASS_USER_INITIATED, spawn);
    #[cfg(target_os = "linux")]
    return linux::with_nice(0, spawn);
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    spawn()
}

/// Run `spawn` at default class and restore the caller's class afterwards, so
/// a process started inside it does not inherit an interactive class.
pub fn with_default_qos<T>(spawn: impl FnOnce() -> T) -> T {
    #[cfg(target_os = "macos")]
    return with_class(libc::qos_class_t::QOS_CLASS_DEFAULT, spawn);
    #[cfg(target_os = "linux")]
    return linux::with_nice(0, spawn);
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    spawn()
}

/// Run `spawn` at `target`, then restore the calling thread's own class.
#[cfg(target_os = "macos")]
fn with_class<T>(target: libc::qos_class_t, spawn: impl FnOnce() -> T) -> T {
    let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
    let mut relative = 0;
    // SAFETY: both out-pointers are valid locals; the calls touch only the
    // calling thread's own QoS.
    unsafe {
        libc::pthread_get_qos_class_np(libc::pthread_self(), &mut class, &mut relative);
        libc::pthread_set_qos_class_self_np(target, 0);
    }
    let result = spawn();
    if class as u32 != libc::qos_class_t::QOS_CLASS_UNSPECIFIED as u32 {
        // SAFETY: as above.
        unsafe {
            libc::pthread_set_qos_class_self_np(class, relative);
        }
    }
    result
}

/// Linux nice is per thread: `PRIO_PROCESS` with a thread id sets one thread.
#[cfg(target_os = "linux")]
mod linux {
    pub const INTERACTIVE: i32 = -5;

    fn tid() -> libc::id_t {
        // SAFETY: gettid takes no arguments and cannot fail.
        unsafe { libc::syscall(libc::SYS_gettid) as libc::id_t }
    }

    /// Best effort: lowering nice below the current value needs privilege.
    pub fn set_nice(nice: i32) {
        // SAFETY: affects only the calling thread's own priority.
        unsafe {
            libc::setpriority(libc::PRIO_PROCESS, tid(), nice);
        }
    }

    pub fn nice() -> i32 {
        // SAFETY: reads only the calling thread's own priority. -1 is also the
        // error value; a thread this module never lowered reads >= 0 anyway.
        unsafe { libc::getpriority(libc::PRIO_PROCESS, tid()) }
    }

    /// Spawn at `target` when the thread runs below it (only a privileged
    /// interactive thread does), then restore; otherwise spawn as is, so an
    /// unprivileged thread never raises a nice it could not lower again.
    pub fn with_nice<T>(target: i32, spawn: impl FnOnce() -> T) -> T {
        let own = nice();
        if own >= target {
            return spawn();
        }
        set_nice(target);
        let result = spawn();
        set_nice(own);
        result
    }
}

/// The calling thread's class, for tests.
#[cfg(all(test, target_os = "macos"))]
fn current() -> u32 {
    let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
    let mut relative = 0;
    // SAFETY: valid out-pointers; reads only the calling thread's own class.
    unsafe {
        libc::pthread_get_qos_class_np(libc::pthread_self(), &mut class, &mut relative);
    }
    class as u32
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "a test thread blocks on /usr/bin/true to observe inheritance"
    )]
    fn spawns_run_at_default_class_and_the_terminal_thread_keeps_its_own() {
        std::thread::spawn(|| {
            interactive_thread();
            assert_eq!(
                current(),
                libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE as u32
            );
            let child_class = with_default_qos(|| {
                assert_eq!(current(), libc::qos_class_t::QOS_CLASS_DEFAULT as u32);
                // A process started here inherits default, not interactive.
                std::process::Command::new("/usr/bin/true")
                    .status()
                    .unwrap();
                current()
            });
            assert_eq!(child_class, libc::qos_class_t::QOS_CLASS_DEFAULT as u32);
            let seat_class = with_seat_qos(|| {
                std::process::Command::new("/usr/bin/true")
                    .status()
                    .unwrap();
                current()
            });
            assert_eq!(
                seat_class,
                libc::qos_class_t::QOS_CLASS_USER_INITIATED as u32
            );
            assert_eq!(
                current(),
                libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE as u32,
                "the terminal thread keeps its class after a spawn"
            );
        })
        .join()
        .unwrap();
    }
}

#[cfg(all(test, target_os = "linux"))]
mod linux_tests {
    use super::*;

    #[test]
    fn spawns_run_at_nice_zero_and_the_terminal_thread_keeps_its_own() {
        std::thread::spawn(|| {
            interactive_thread();
            let own = linux::nice();
            assert!(own <= 0, "the terminal path is never below nice 0: {own}");
            assert_eq!(with_seat_qos(linux::nice), own.max(0));
            assert_eq!(with_default_qos(linux::nice), own.max(0));
            assert_eq!(linux::nice(), own, "the terminal thread keeps its nice");
        })
        .join()
        .unwrap();
    }
}
