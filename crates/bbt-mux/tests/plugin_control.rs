// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "owned headless socket and file-account fixtures"
)]
mod common;
use bbt_mux::{
    Supervisor, Ulid,
    ctl_door::{self, Peers},
    identity::TestAccount,
    logic::{Logic, LogicConfig},
};
use bbt_proto::{GuiAction, GuiReply, GuiRequest, gui::PluginRequest};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
};

// keeps: plugin controls use the admitted operator door and the selected private account.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admitted_plugin_inventory_never_executes_or_trusts_an_unapproved_file() {
    let root = tempfile::tempdir().unwrap();
    let run = root.path().join("run");
    let repo = root.path().join("repo");
    let plugins = repo.join(".bboxy/plugins");
    std::fs::create_dir_all(&plugins).unwrap();
    let marker = root.path().join("must-not-run");
    let source = plugins.join("bboxy450-term-repo_graph-private");
    std::fs::write(
        &source,
        format!("#!/bin/sh\ntouch '{}'\nexit 97\n", marker.display()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o700)).unwrap();
    let account = TestAccount::open(&root.path().join("custody"), &run).unwrap();
    let supervisor = Supervisor::start(common::capture_config(root.path())).unwrap();
    let logic = Logic::start_custodied_with_store(
        supervisor.connect().unwrap(),
        LogicConfig {
            run_dir: run.clone(),
            repository: repo,
            ring: Default::default(),
        },
        account.key_store(),
        &tokio::runtime::Handle::current(),
    )
    .unwrap();
    let _door = ctl_door::open(
        &logic.client,
        &run,
        Peers {
            supervisor: u32::MAX - 1,
            gui: u32::MAX,
        },
    )
    .await
    .unwrap();
    let endpoint = logic.client.gui_endpoint().await.unwrap();
    let (read, mut write) = UnixStream::connect(endpoint.path)
        .await
        .unwrap()
        .into_split();
    let mut read = BufReader::new(read);
    let op = Ulid::new();
    for (id, request) in [
        (op, PluginRequest::List),
        (
            Ulid::new(),
            PluginRequest::Enable {
                candidate: "missing".into(),
            },
        ),
    ] {
        let mut bytes = serde_json::to_vec(&GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: endpoint.capability,
            action: GuiAction::Plugin {
                scope: None,
                id,
                request,
            },
        })
        .unwrap();
        bytes.push(b'\n');
        write.write_all(&bytes).await.unwrap();
        let reply = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            loop {
                let line = bbt_mux::seat_socket::read_line(&mut read)
                    .await
                    .unwrap()
                    .unwrap();
                let reply: GuiReply = serde_json::from_slice(&line).unwrap();
                if matches!(&reply,GuiReply::Plugin {op,..}|GuiReply::Error {op,..} if *op==id) {
                    break reply;
                }
            }
        })
        .await
        .unwrap();
        if id == op {
            let GuiReply::Plugin { entries, .. } = reply else {
                panic!("{reply:?}");
            };
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].state(), bbt_plugin::registry::State::Untrusted);
            assert!(entries[0].trust.is_none());
        } else {
            assert!(
                matches!(reply,GuiReply::Error {ref reason,..} if reason.contains("plugin_not_discovered"))
            );
        }
    }
    assert!(!marker.exists());
    drop(write);
    drop(read);
    drop(_door);
    logic.shutdown().unwrap();
    supervisor.shutdown().unwrap();
}
