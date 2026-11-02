// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(clippy::disallowed_methods, reason = "private journal fixture IO")]

use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

use bbt_mux::provider::{Connections, HostId, LocalProvider};
use bbt_mux::{Action, Attachment, Event, Failure, FailureCode, Reply, Ulid};
use bbt_rt::error_journal;
use bbt_supervisor::SupervisorConnection;

struct RefuseInput(FailureCode);
impl SupervisorConnection for RefuseInput {
    fn request(&mut self, _: Action) -> bbt_mux::store::Result<Reply> {
        Err(Failure {
            code: self.0,
            message: "input refused by fixture".into(),
        }
        .into())
    }

    fn receive(&mut self, _: Duration) -> bbt_mux::store::Result<Option<Event>> {
        Ok(None)
    }
}

#[test]
fn provider_input_refusals_are_journalled_with_the_exact_seat() {
    // This integration executable owns its one-time journal configuration.
    let root = tempfile::tempdir().unwrap();
    std::fs::set_permissions(root.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    error_journal::configure(root.path());
    let (tx, rx) = std::sync::mpsc::channel();
    let server = error_journal::Server::start(root.path().to_path_buf(), move |_| {
        tx.send(()).unwrap();
    })
    .unwrap();
    server
        .ready()
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    let attachment = Attachment {
        seat: Ulid::new(),
        generation: 1,
        writer_lease: Ulid::new(),
    };
    for code in [
        FailureCode::Limit,
        FailureCode::InputConflict,
        FailureCode::Busy,
        FailureCode::StaleGeneration,
        FailureCode::Invalid,
    ] {
        let mut connections =
            Connections::open(vec![Box::new(LocalProvider::new(RefuseInput(code)))]).unwrap();
        let actions = [
            Action::Input {
                attachment: attachment.clone(),
                input_id: Ulid::new(),
                bytes: b"INPUT_PAYLOAD".to_vec(),
            },
            Action::GuardedInput {
                attachment: attachment.clone(),
                input_id: Ulid::new(),
                bytes: b"INPUT_PAYLOAD".to_vec(),
                expected: bbt_supervisor::protocol::InputContext {
                    epoch: 0,
                    last_input_ms: 0,
                    sequence: 0,
                    draft: false,
                    foreground_pid: None,
                    canonical: true,
                    echo: true,
                    settled: true,
                },
            },
            Action::InputContext {
                attachment: attachment.clone(),
            },
            Action::InputStatus {
                seat: attachment.seat,
                generation: 1,
                input_id: Ulid::new(),
            },
        ];
        for action in actions {
            let error = connections.request(&HostId::Local, action).unwrap_err();
            assert_eq!(error.downcast_ref::<Failure>().unwrap().code, code);
            rx.recv_timeout(Duration::from_secs(5)).unwrap();
        }
    }
    let page = error_journal::read(root.path(), None, None, false).unwrap();
    let rows = page["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 20, "{page}");
    for row in rows {
        assert_eq!(row["source"], "provider");
        assert_eq!(row["operation"], "request");
        assert_eq!(row["seat"], attachment.seat.to_string());
        assert!(
            !row["message"].as_str().unwrap().contains("INPUT_PAYLOAD"),
            "input content must not enter diagnostics"
        );
    }
}
