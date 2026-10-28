// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods)] // Disposable blocking-IO policy fixture.

// keeps: invariant ui-never-waits
#[test]
#[cfg(debug_assertions)]
fn ui_thread_is_rejected_before_creating_a_database_directory() {
    let fixture = tempfile::tempdir().unwrap();
    let object = fixture.path().join("not-created");
    bbt_rt::register_ui_thread();
    assert!(
        std::panic::catch_unwind(|| bbt_mux::store::sqlite::SqliteStore::open(&object)).is_err()
    );
    assert!(!object.exists());
}
