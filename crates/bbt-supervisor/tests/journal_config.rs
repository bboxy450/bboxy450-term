#![allow(clippy::disallowed_methods)] // Disposable owned PTY fixture.
use bbt_supervisor::{Action, Config, FailureCode, LaunchSpec, Supervisor};
#[test]
fn capture_requires_an_explicit_supervisor_journal_directory() {
    let root = tempfile::tempdir().unwrap();
    let supervisor = Supervisor::start(Config::default()).unwrap();
    let client = supervisor.connect().unwrap();
    let mut launch = LaunchSpec::new("/bin/sleep", root.path());
    launch.args = vec!["30".into()];
    let seat = client.spawn(launch).unwrap();
    let result = client.request(Action::AttachRing {
        seat: seat.seat,
        generation: 1,
        directory: root.path().join("seat"),
        settings: Default::default(),
    });
    let error = result.unwrap_err();
    assert_eq!(error.code, FailureCode::Invalid);
    assert!(error.message.contains("JournalDirectoryRequired"));
    assert!(!root.path().join("seat").exists());
}
