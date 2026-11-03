// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Scheduling class for the terminal path. The threads that carry PTY bytes to
//! the screen run user-interactive, so a parallel build cannot starve them.
//! A process inherits the class of the thread that spawns it, so every seat,
//! companion, daemon and plugin is spawned at default class: their builds run
//! below the terminal path, never beside it.

/// Mark the calling thread user-interactive (macOS; a no-op elsewhere).
pub fn interactive_thread() {
    #[cfg(target_os = "macos")]
    // SAFETY: changes only the calling thread's own QoS class; no pointers.
    unsafe {
        libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_USER_INTERACTIVE, 0);
    }
}

/// Run `spawn` at default class and restore the caller's class afterwards, so
/// a process started inside it does not inherit an interactive class.
pub fn with_default_qos<T>(spawn: impl FnOnce() -> T) -> T {
    #[cfg(target_os = "macos")]
    {
        let mut class = libc::qos_class_t::QOS_CLASS_UNSPECIFIED;
        let mut relative = 0;
        // SAFETY: both out-pointers are valid locals; the calls touch only the
        // calling thread's own QoS.
        unsafe {
            libc::pthread_get_qos_class_np(libc::pthread_self(), &mut class, &mut relative);
            libc::pthread_set_qos_class_self_np(libc::qos_class_t::QOS_CLASS_DEFAULT, 0);
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
    #[cfg(not(target_os = "macos"))]
    spawn()
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
