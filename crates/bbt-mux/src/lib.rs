// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Restartable logic above the local supervisor process boundary.
//!
//! Logic owns durable objects, the bus, repository index, harness adapters,
//! messaging and viewport streams. `bbt-supervisor` owns PTYs, leases and ring
//! capture without Tokio; its existing public paths remain re-exported here.

// This is the IO owner. Blocking entry points assert they are off the UI thread.
#![allow(clippy::disallowed_methods)]

#[cfg(not(unix))]
compile_error!("bbt-mux currently requires Unix PTYs and Unix sockets");

pub mod adapters;
mod attention_observer;
pub mod bus;
mod checkpoint;
pub mod ctl_door;
pub mod debug;
mod gui_capture;
mod gui_events;
mod history_fault;
pub mod identity;
pub mod index;
pub mod knowledge;
pub mod logic;
pub mod mcp;
pub mod messaging;
#[cfg(feature = "model")]
pub mod models;
pub mod plugins;
pub mod provenance;
pub mod provider;
pub mod registry;
pub mod seat_socket;
pub mod store;
pub mod streams;

// Preserve the existing client paths while the supervisor is a lower crate.
pub use bbt_supervisor::{protocol, ring, supervisor, transport};

pub use protocol::{
    Action, Attachment, Event, Failure, FailureCode, Frame, InputReceipt, InputState, LaunchSpec,
    Message, Reply, Request, SeatInfo, SeatSize,
};
pub use supervisor::{Client, Config, Supervisor};
pub use ulid::Ulid;

mod window_journal;

mod rich;

#[cfg(test)]
mod qos_probe {
    // Copyright 2026 Boxy (bboxy450) and contributors.
    // SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
    // License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
    // Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
    // Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
    // LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    // Adapted from this project's crates/bbt-mux/tests/mcp.rs private Fixture.
    // Own temporary identity, repository, supervisor and seat socket; never installed.
    #[test]
    #[ignore = "stdio fixture launched only by scripts/qos-probe.py"]
    fn serve() {
        use crate::mcp::bridge::{Bridge, Config};
        use crate::{
            LaunchSpec, Supervisor, Ulid,
            logic::{Logic, LogicConfig},
            registry::SeatRecord,
        };
        use ed25519_dalek::SigningKey;
        bbt_rt::qos::initiated_thread();
        let temp = tempfile::Builder::new()
            .prefix("qos-")
            .tempdir_in("/tmp")
            .unwrap();
        let repository = temp.path().join("repository");
        std::fs::create_dir_all(repository.join(".bboxy/tasks")).unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .on_thread_start(bbt_rt::qos::initiated_thread)
            .enable_all()
            .build()
            .unwrap();
        let supervisor = Supervisor::start(crate::Config {
            journal_directory: Some(temp.path().join("journal")),
            ..Default::default()
        })
        .unwrap();
        let logic = Logic::start(
            supervisor.connect().unwrap(),
            LogicConfig {
                run_dir: temp.path().join("run"),
                repository,
                ring: Default::default(),
            },
            SigningKey::from_bytes(&[42; 32]),
            runtime.handle(),
        )
        .unwrap();
        let record = SeatRecord::new(Ulid::new(), Ulid::new(), "QoS probe", "zsh");
        let seat = record.id;
        let mut spec = LaunchSpec::new("/bin/cat", temp.path());
        spec.seat = seat;
        runtime.block_on(logic.client.spawn(record, spec)).unwrap();
        let endpoint = runtime.block_on(logic.client.endpoint(seat)).unwrap();
        println!(
            "QOS_READY {}",
            serde_json::json!({"requested_class": bbt_rt::qos::requested_class(), "seat": seat})
        );
        Bridge::new(Config {
            socket: endpoint.path,
            seat,
            generation: endpoint.generation,
            capability: endpoint.capability,
        })
        .run(
            &mut std::io::BufReader::new(std::io::stdin().lock()),
            &mut std::io::stdout().lock(),
        )
        .unwrap();
        logic.shutdown().unwrap();
        drop(supervisor);
    }
}
