// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! All three admitted doors produce usable, generation-bound Codex MCP wiring.
use super::*;
use bbt_mux::mcp::Door;
use bbt_proto::{SpawnSpec, SwarmId, Where, seat_state::Placement};

// keeps: spec harness-adapters, CodexMcpReadiness
#[test]
fn codex_launch_doors_share_required_catalog_identity_resume_and_replay() {
    let f = Fixture::new();
    f.prepare_operator();
    let alias = f._temp.path().join("work alias");
    std::os::unix::fs::symlink(&f.config.repository, &alias).unwrap();
    let cwd = f.config.repository.canonicalize().unwrap();
    let baseline = f.registry().list().unwrap().len();
    let mut launches = 0;
    for resume in [
        None,
        Some("3dac58c8-f6a4-4fe6-9863-8c72a5216d5e".to_owned()),
    ] {
        for door in ["ui", "ctl", "mcp", "mcp-ctl"] {
            let op = Ulid::new();
            let tab = Ulid::new();
            let seat = if matches!(door, "mcp" | "mcp-ctl") {
                let args = json!({"swarm_id":f.own.swarm,"approved_profile":"codex",
                    "cwd":alias,"role":"executor","name":"Unified launch","tab_id":tab,"resume":resume});
                let (tool, args) = if door == "mcp-ctl" {
                    ("ctl", json!({"command":"seat_spawn","arguments":args}))
                } else {
                    ("seat_spawn", args)
                };
                let first = f.mutate(f.oracle.id, op, tool, args.clone());
                assert_eq!(f.mutate(f.oracle.id, op, tool, args), first);
                serde_json::from_value(data(first)["seat_id"].clone()).unwrap()
            } else {
                let spec = SpawnSpec {
                    swarm: SwarmId::from_u128(f.own.swarm.0),
                    harness_kind: "codex".into(),
                    name: Some("Unified launch".into()),
                    role: Some("executor".into()),
                    cwd: alias.to_string_lossy().into_owned(),
                    location: Where::Local,
                    resume: resume.clone(),
                    placement: Some(Placement::NewTab {
                        tab: bbt_proto::TabId::from_u128(tab.0),
                    }),
                    via: None,
                    command: None,
                    args: vec![],
                    env: vec![],
                };
                let door = if door == "ui" { Door::Gui } else { Door::Ctl };
                let first = f
                    .runtime
                    .block_on(f.client().gui_spawn(op, spec.clone(), door))
                    .unwrap();
                assert_eq!(
                    f.runtime
                        .block_on(f.client().gui_spawn(op, spec, door))
                        .unwrap(),
                    first
                );
                Ulid::from(first.0.0)
            };
            launches += 1;
            assert_eq!(f.registry().list().unwrap().len(), baseline + launches);
            let record = f.registry().get(seat).unwrap();
            assert_eq!(record.role.name, "executor");
            assert_eq!(record.harness_session_id, resume);
            let spec = f.record_link.as_ref().unwrap().launch_spec(seat);
            assert_eq!(spec.cwd, cwd, "{door}: directory must resolve on the owner");
            assert_eq!(spec.env["BBOXY_SEAT_ID"], seat.to_string());
            assert_eq!(spec.env["BBOXY_GENERATION"], "1");
            let admission = PathBuf::from(&spec.env["BBOXY_CAPABILITY_FILE"]);
            let overlay = admission.parent().unwrap();
            let launch: bboxy450_harness_wake_up::launch::CodexLaunch =
                serde_json::from_slice(&std::fs::read(overlay.join("codex-launch.json")).unwrap())
                    .unwrap();
            assert_eq!(launch.cwd, cwd);
            assert_eq!(launch.resume, resume);
            let server = &launch.config["mcp_servers"]["bboxy"];
            assert_eq!(server["command"], json!(overlay.join("bboxy-seat")));
            assert_eq!(server["args"], json!(["mcp"]));
            assert_eq!(server["enabled"], true);
            assert_eq!(
                server["required"], true,
                "{door}: optional startup can lose the catalog"
            );
            assert_eq!(server["default_tools_approval_mode"], "approve");
            assert!(
                launch.config["developer_instructions"]
                    .as_str()
                    .unwrap()
                    .contains("skill_list")
            );
            assert!(launch.config["hooks"].get("SessionStart").is_some());
            assert!(spec.companion.is_some(), "{door}: missing seat proxy");
            let mut bridge = Bridge::new(BridgeConfig {
                socket: spec.env["BBOXY_SOCK"].clone().into(),
                seat,
                generation: 1,
                capability: bboxy450_harness_wake_up::capability::read(&admission).unwrap(),
            });
            initialize(&mut bridge);
            let catalog = rpc(&mut bridge, 1, "tools/list", json!({}));
            let names: Vec<_> = catalog["result"]["tools"]
                .as_array()
                .unwrap()
                .iter()
                .map(|tool| tool["name"].as_str().unwrap())
                .collect();
            for required in [
                "orient",
                "skill_list",
                "skill_read",
                "task_report",
                "message_send",
            ] {
                assert!(names.contains(&required), "{door}: missing {required}");
            }
            let oriented = rpc(
                &mut bridge,
                2,
                "tools/call",
                json!({"name":"orient","arguments":{}}),
            );
            assert_eq!(oriented["result"]["isError"], false, "{oriented}");
            assert_eq!(
                oriented["result"]["structuredContent"]["actor"]["seat_id"],
                json!(seat)
            );
        }
    }
}
