#![allow(clippy::disallowed_methods)] // Disposable blocking-IO policy fixture.

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
