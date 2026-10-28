// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]
use bboxy450_term::{
    grids::Grids,
    model::{self, Model},
    settings::{self, Io, State, Store},
};
use bbt_ui::modules::settings::controller::Patch;
use std::{sync::Arc, time::Duration};

fn custom_role(prompt: &str) -> bbt_ui::modules::settings::Role {
    bbt_ui::modules::settings::Role {
        prompt: prompt.into(),
        send: "own swarm · operator DM".into(),
        color: "#7397d4".into(),
        mark: "♜".into(),
        cls: "worker".into(),
        custom: true,
    }
}

#[test]
fn destination_availability_persists_independently_and_preserves_rediscovery_and_comments() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, "# retained\n[unmanaged]\nvalue = 17\n[destinations.'ssh:node-a']\navailable = true # availability comment\n[destinations.'cloud:future provider']\navailable = false\n").unwrap();
    let store = Store::new(path.clone());
    let mut state = State::default();
    state.accept(&settings::Snapshot {
        loaded: true,
        preferences: store.load().unwrap(),
        ..Default::default()
    });
    assert!(
        state
            .preferences
            .disabled_destinations
            .contains("cloud:future provider")
    );
    state
        .edit(Patch::DestinationAvailable {
            id: "ssh:node-a".into(),
            available: false,
        })
        .unwrap();
    state
        .edit(Patch::DestinationAvailable {
            id: "cloud:nebula".into(),
            available: false,
        })
        .unwrap();
    let request = state.edit(Patch::Theme("light".into())).unwrap();
    let settings::Request::Save { patches, .. } = request else {
        panic!("save request");
    };
    assert_eq!(
        patches.len(),
        3,
        "destination edits never coalesce over each other or appearance"
    );
    let saved = store.save(&patches).unwrap();
    assert_eq!(
        saved.disabled_destinations,
        std::collections::BTreeSet::from([
            "ssh:node-a".into(),
            "cloud:nebula".into(),
            "cloud:future provider".into()
        ])
    );
    assert_eq!(saved, store.load().unwrap());
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("# retained")
            && text.contains("value = 17")
            && text.contains("# availability comment")
    );
    let restored = store
        .save(&[Patch::DestinationAvailable {
            id: "ssh:node-a".into(),
            available: true,
        }])
        .unwrap();
    assert!(!restored.disabled_destinations.contains("ssh:node-a"));
    assert!(
        restored
            .disabled_destinations
            .contains("cloud:future provider")
    );
    let empty = settings::records(Some(&Model::default()), &restored);
    assert!(
        empty.destination_ids.is_empty(),
        "a stored identifier never creates a provider or host"
    );
    assert!(
        empty
            .disabled_destinations
            .contains("cloud:future provider")
    );
}

#[test]
fn invalid_destination_configuration_and_edits_leave_the_file_untouched() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, "# original\n[other]\nvalue = 7\n").unwrap();
    let store = Store::new(path.clone());
    let original = std::fs::read(&path).unwrap();
    for id in [
        "",
        "ssh:",
        "cloud:",
        "local",
        "ftp:rig",
        "ssh:rig\n",
        "cloud:\0provider",
        "cloud: provider ",
    ] {
        assert!(!settings::destinations::valid_id(id));
        let patch = Patch::DestinationAvailable {
            id: id.into(),
            available: false,
        };
        let mut preferences = settings::Preferences::default();
        assert!(preferences.apply(&patch).is_err());
        assert!(preferences.disabled_destinations.is_empty());
        assert!(store.save(&[patch]).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
    assert!(!settings::destinations::valid_id(&format!(
        "ssh:{}",
        "a".repeat(509)
    )));
    assert!(settings::destinations::valid_id(&format!(
        "ssh:{}",
        "a".repeat(508)
    )));
    for text in [
        "[destinations.'ssh:rig']\navailable='yes'\n",
        "[destinations.'local']\navailable=false\n",
        "destinations=[]\n",
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(&[Patch::Theme("dark".into())]).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), text);
    }
}

#[test]
fn settings_destinations_follow_observed_ids_and_rendered_group_order_not_labels() {
    use bbt_proto::{GuiHost, HostState, Where};
    let host = |location, label: &str| GuiHost {
        location,
        provider: "fixture".into(),
        label: label.into(),
        address: Some("display@address.invalid".into()),
        region: Some("fixture-region".into()),
        state: HostState::Connected { rtt_ms: 60 },
    };
    let hosts = vec![
        host(Where::Local, "Local"),
        host(
            Where::Cloud {
                provider: "zeta".into(),
                host: "one".into(),
            },
            "Shared label",
        ),
        host(
            Where::Ssh {
                host: "node-b".into(),
            },
            "Shared label",
        ),
        host(
            Where::Cloud {
                provider: "alpha".into(),
                host: "one".into(),
            },
            "Shared label",
        ),
        host(
            Where::Ssh {
                host: "node-a".into(),
            },
            "Shared label",
        ),
        host(
            Where::Cloud {
                provider: "alpha".into(),
                host: "two".into(),
            },
            "Other label",
        ),
    ];
    let mut preferences = settings::Preferences::default();
    for id in ["ssh:node-b", "cloud:alpha"] {
        preferences
            .apply(&Patch::DestinationAvailable {
                id: id.into(),
                available: false,
            })
            .unwrap();
    }
    let model = Model {
        hosts: hosts.clone(),
        ..Default::default()
    };
    let records = settings::records(Some(&model), &preferences);
    assert_eq!(
        records.destination_ids,
        std::collections::BTreeMap::from([
            ("machine/0".into(), "ssh:node-b".into()),
            ("machine/1".into(), "ssh:node-a".into()),
            ("cloud/0".into(), "cloud:alpha".into()),
            ("cloud/1".into(), "cloud:zeta".into()),
        ])
    );
    assert_eq!(
        records.disabled_destinations,
        preferences.disabled_destinations
    );
    assert_eq!(records.machines.len(), 2);
    assert_eq!(records.cloud.len(), 2);
    assert_eq!(records.cloud[0].0, "alpha");
    assert_eq!(
        records.cloud[0].4,
        Some(bbt_ui::modules::link::LinkState::Connected { rtt_ms: 60 })
    );
    assert_eq!(
        model.hosts, hosts,
        "availability never changes observed provider connection state"
    );
    assert_eq!(
        settings::destinations::id(&Where::Ssh {
            host: "node-b".into()
        })
        .as_deref(),
        Some("ssh:node-b")
    );
    assert_eq!(
        settings::destinations::id(&Where::Cloud {
            provider: "alpha".into(),
            host: "two".into()
        })
        .as_deref(),
        Some("cloud:alpha")
    );
    assert_eq!(settings::destinations::id(&Where::Local), None);
}

#[test]
fn roles_and_harness_choices_persist_and_batches_preserve_comments_and_other_fields() {
    use std::collections::BTreeSet;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# untouched\n[other]\nx = 7\n[roles.executor]\nprompt = 'old prompt' # role comment\n[harnesses.codex]\ncommand = 'codex' # command comment\n").unwrap();
    let store = Store::new(path.clone());
    let mut state = State::default();
    state.accept(&settings::Snapshot {
        loaded: true,
        preferences: store.load().unwrap(),
        ..Default::default()
    });
    state
        .edit(Patch::RoleUpsert {
            name: "Reviewer Δ".into(),
            role: custom_role("Review the diff.\nReport concrete defects."),
        })
        .unwrap();
    state
        .edit(Patch::HarnessRole {
            harness: "codex".into(),
            role: "Reviewer Δ".into(),
        })
        .unwrap();
    state
        .edit(Patch::HarnessLaunch {
            harness: "codex".into(),
            command: "/tools/codex".into(),
            arguments: vec!["--sandbox".into(), "workspace-write".into()],
        })
        .unwrap();
    let request = state
        .edit(Patch::HarnessAvailableRoles {
            harness: "codex".into(),
            roles: BTreeSet::from(["executor".into(), "Reviewer Δ".into()]),
        })
        .unwrap();
    let settings::Request::Save { patches, .. } = request else {
        panic!("save request");
    };
    assert_eq!(
        patches.len(),
        4,
        "independent changes to one harness must all survive coalescing"
    );
    let saved = store.save(&patches).unwrap();
    assert_eq!(saved, store.load().unwrap());
    assert_eq!(
        saved.roles["Reviewer Δ"].class,
        bbt_proto::registry::Class::Worker
    );
    assert_eq!(saved.harnesses["codex"].role.as_deref(), Some("Reviewer Δ"));
    assert_eq!(
        saved.harnesses["codex"].command.as_deref(),
        Some("/tools/codex")
    );
    assert_eq!(
        saved.harnesses["codex"].arguments.as_deref(),
        Some(["--sandbox".to_owned(), "workspace-write".to_owned()].as_slice())
    );
    let mut executor = settings::roles::view("executor", &saved.roles["executor"]);
    executor.prompt = "Implement the scoped task and report evidence.".into();
    store
        .save(&[Patch::RoleUpsert {
            name: "executor".into(),
            role: executor,
        }])
        .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("# untouched")
            && text.contains("x = 7")
            && text.contains("# role comment")
            && text.contains("# command comment")
    );
    assert!(text.contains("Reviewer Δ"));
    assert_eq!(
        store.load().unwrap().roles["Reviewer Δ"].prompt,
        "Review the diff.\nReport concrete defects."
    );
}

#[test]
fn unavailable_roles_invalid_authority_and_duplicate_configuration_never_damage_the_file() {
    use std::collections::BTreeSet;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let store = Store::new(path.clone());
    store
        .save(&[
            Patch::RoleUpsert {
                name: "reviewer".into(),
                role: custom_role("Review"),
            },
            Patch::HarnessAvailableRoles {
                harness: "codex".into(),
                roles: BTreeSet::from(["executor".into(), "reviewer".into()]),
            },
            Patch::HarnessRole {
                harness: "codex".into(),
                role: "reviewer".into(),
            },
        ])
        .unwrap();
    let original = std::fs::read(&path).unwrap();
    for patches in [
        vec![Patch::RoleDelete("executor".into())],
        vec![Patch::RoleDelete("reviewer".into())],
        vec![Patch::HarnessRole {
            harness: "codex".into(),
            role: "oracle".into(),
        }],
        vec![Patch::HarnessRole {
            harness: "claude".into(),
            role: "missing".into(),
        }],
        vec![Patch::HarnessAvailableRoles {
            harness: "codex".into(),
            roles: BTreeSet::from(["executor".into()]),
        }],
        vec![Patch::RoleUpsert {
            name: "Oracle".into(),
            role: custom_role("reserved spelling"),
        }],
    ] {
        assert!(store.save(&patches).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
    let mut values = store.load().unwrap();
    let before = values.clone();
    assert!(
        values
            .apply(&Patch::HarnessAvailableRoles {
                harness: "codex".into(),
                roles: BTreeSet::from(["executor".into()])
            })
            .is_err()
    );
    assert_eq!(
        values, before,
        "refused optimistic edits leave the current view intact"
    );
    for invalid in [
        "[roles.reviewer]\nprompt='a'\n[roles.reviewer]\nprompt='b'\n",
        "[harnesses.codex]\nallowed_roles=['executor','executor']\n",
    ] {
        std::fs::write(&path, invalid).unwrap();
        assert!(store.load().is_err());
        assert!(store.save(&[Patch::Theme("light".into())]).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), invalid);
    }
}

#[test]
fn implicit_role_availability_grows_but_explicit_sets_and_typed_classes_stay_fixed() {
    use std::collections::BTreeSet;
    let mut preferences = settings::Preferences::default();
    preferences
        .apply(&Patch::HarnessAvailableRoles {
            harness: "codex".into(),
            roles: BTreeSet::from(["executor".into()]),
        })
        .unwrap();
    preferences
        .apply(&Patch::RoleUpsert {
            name: "reviewer".into(),
            role: custom_role("Review"),
        })
        .unwrap();
    let catalog = preferences.role_catalog();
    assert_eq!(catalog.harnesses["codex"].available_roles, ["executor"]);
    assert!(
        catalog.harnesses["pi"]
            .available_roles
            .iter()
            .any(|name| name == "reviewer")
    );
    assert!(
        catalog.harnesses["claude"]
            .available_roles
            .iter()
            .any(|name| name == "reviewer")
    );
    assert!(
        catalog.harnesses["zsh"]
            .available_roles
            .iter()
            .any(|name| name == "reviewer")
    );
    let mut oracle = custom_role("Plan and supervise");
    oracle.cls = "oracle".into();
    preferences
        .apply(&Patch::RoleUpsert {
            name: "planner".into(),
            role: oracle,
        })
        .unwrap();
    let catalog = preferences.role_catalog();
    assert!(
        catalog.harnesses["claude"]
            .available_roles
            .iter()
            .any(|name| name == "planner")
    );
    for harness in ["claude", "pi", "omp", "zsh"] {
        assert!(
            catalog.harnesses[harness]
                .available_roles
                .iter()
                .any(|name| name == "planner")
        );
    }
    assert_eq!(catalog.harnesses["codex"].available_roles, ["executor"]);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[roles.reviewer]\nprompt='Review'\n").unwrap();
    assert_eq!(
        Store::new(path).load().unwrap().roles["reviewer"].class,
        bbt_proto::registry::Class::Worker
    );
}

#[test]
fn every_harness_can_persist_any_role_and_builtin_role_classes_are_editable() {
    use bbt_proto::registry::Class;
    use std::collections::BTreeSet;
    for harness in ["claude", "codex", "pi", "omp", "zsh", "custom"] {
        for (name, class) in [
            ("oracle", Class::Worker),
            ("executor", Class::Oracle),
            ("shell", Class::Oracle),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::new(dir.path().join("config.toml"));
            let mut role = settings::roles::view(name, &settings::roles::builtin()[name]);
            role.cls = settings::roles::class_name(class).into();
            store
                .save(&[
                    Patch::RoleUpsert {
                        name: name.into(),
                        role,
                    },
                    Patch::HarnessRole {
                        harness: harness.into(),
                        role: name.into(),
                    },
                    Patch::HarnessAvailableRoles {
                        harness: harness.into(),
                        roles: BTreeSet::from([name.into()]),
                    },
                ])
                .unwrap();
            let saved = store.load().unwrap();
            let catalog = saved.role_catalog();
            assert_eq!(catalog.roles[name].class, class);
            assert_eq!(catalog.harnesses[harness].default_role, name);
            assert_eq!(catalog.harnesses[harness].available_roles, [name]);
            assert_eq!(
                settings::roles::eligible(harness, &catalog.roles).len(),
                catalog.roles.len()
            );
            let records = settings::records(None, &saved);
            let shown = &records
                .harness
                .iter()
                .find(|(kind, _)| kind == harness)
                .unwrap()
                .1;
            assert_eq!(shown.role, name);
            assert_eq!(shown.available_roles, [name]);
            assert_eq!(shown.eligible_roles.len(), catalog.roles.len());
        }
    }
}

// keeps: spec ux-prototype §5.6
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn settings_io_coalesces_edits_without_losing_other_fields_and_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "# untouched\n[other]\nx = 7\n").unwrap();
    let mut io = Io::start(path.clone(), &tokio::runtime::Handle::current(), |_| {});
    tokio::time::timeout(Duration::from_secs(30), io.updates.changed())
        .await
        .unwrap()
        .unwrap();
    let mut state = State::default();
    state.accept(&io.updates.borrow());
    assert!(state.loaded);
    for i in 0..12 {
        io.submit(
            state
                .edit(Patch::Theme(
                    if i % 2 == 0 { "light" } else { "dark" }.into(),
                ))
                .unwrap(),
        )
        .unwrap();
    }
    io.submit(state.edit(Patch::NotificationsEnabled(false)).unwrap())
        .unwrap();
    let expected = state.revision;
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let snapshot = io.updates.borrow_and_update().clone();
            state.accept(&snapshot);
            if snapshot.revision == expected {
                break;
            }
            io.updates.changed().await.unwrap();
        }
    })
    .await
    .unwrap();
    assert!(state.saved());
    drop(io);
    let reloaded = Store::new(path.clone()).load().unwrap();
    assert_eq!(reloaded, state.preferences);
    assert!(!reloaded.notifications.enabled);
    let contents = std::fs::read_to_string(path).unwrap();
    assert!(contents.contains("# untouched"));
    assert!(contents.contains("x = 7"));
}

// keeps: spec ux-prototype §5.6
#[test]
fn settings_metadata_rebuild_preserves_context_and_the_preferences_snapshot() {
    let mut previous = Model::default();
    previous.ui.window.theme = bbt_ui::Theme::Light;
    previous.ui.window.density = bbt_ui::Density::Compact;
    let snapshot = Arc::new(settings::Snapshot {
        loaded: true,
        ..Default::default()
    });
    previous.settings = Some(snapshot.clone());
    let next = model::build(&[], &Grids::new(), &previous);
    assert_eq!(next.ui.window.theme, bbt_ui::Theme::Light);
    assert_eq!(next.ui.window.density, bbt_ui::Density::Compact);
    assert!(Arc::ptr_eq(next.settings.as_ref().unwrap(), &snapshot));
}

// keeps: spec ux-prototype §5.6
#[test]
fn settings_projects_missing_identity_honestly_and_uses_the_backend_role_defaults() {
    let records = settings::records(Some(&Model::default()), &Default::default());
    assert_eq!(records.account.key_state, "unavailable");
    assert!(records.account.fingerprint.is_empty());
    assert!(records.account.devices.is_empty());
    for (harness, expected) in [
        ("claude", "oracle"),
        ("codex", "executor"),
        ("zsh", "shell"),
    ] {
        assert_eq!(
            records
                .harness
                .iter()
                .find(|(name, _)| name == harness)
                .unwrap()
                .1
                .role,
            expected
        );
    }
    assert!(records.roles.iter().any(|(name, _)| name == "shell"));
    assert_eq!(
        records
            .harness
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["claude", "codex", "pi", "omp", "zsh"]
    );
    for (kind, wake, source, hooks) in [
        ("claude", "socket", "reported", true),
        ("codex", "app-server", "reported", true),
        ("pi", "pty", "output cadence", false),
        ("omp", "pty", "output cadence", false),
    ] {
        let harness = &records
            .harness
            .iter()
            .find(|(name, _)| name == kind)
            .unwrap()
            .1;
        assert_eq!(
            (
                harness.wake.as_str(),
                harness.state_from.as_str(),
                harness.hooks
            ),
            (wake, source, hooks)
        );
        assert!(harness.available_roles.contains(&harness.role));
        assert!(
            harness
                .available_roles
                .iter()
                .all(|name| harness.eligible_roles.contains(name))
        );
    }
    assert!(
        records
            .roles
            .iter()
            .filter(|(name, _)| name != "shell")
            .all(|(_, role)| !role.prompt.is_empty()
                && !role.color.is_empty()
                && !role.mark.is_empty())
    );
}

// keeps: spec ux-prototype §5.6
#[test]
fn settings_fixtures_stay_labelled_and_live_knowledge_requires_observed_root_state() {
    use bbt_proto::gui::{KnowledgeProvider, KnowledgeRoot, KnowledgeStatus};
    let model = Model::default();
    let records = settings::records(Some(&model), &Default::default());
    let examples = settings::records(None, &Default::default());
    assert!(
        examples
            .plugin_notice
            .as_deref()
            .unwrap()
            .contains("Reference fixtures")
    );
    assert_eq!(examples.plugins.len(), 6);
    assert!(examples.plugins.iter().all(|p| {
        p.health
            .say
            .as_deref()
            .unwrap()
            .starts_with("reference fixture")
    }));
    assert!(examples.plugins.iter().any(|p| {
        p.alerts
            .iter()
            .any(|a| a == "this plugin takes 10 s on every hook")
    }));
    assert!(records.plugins.is_empty());
    assert!(!records.plugin_controls);
    assert!(
        records
            .plugin_notice
            .as_deref()
            .unwrap()
            .contains("snapshot pending")
    );
    assert!(records.knowledge.offered.is_empty());
    assert!(records.knowledge.roots.is_empty());
    assert!(!records.knowledge_controls);
    let mut ready = model;
    ready.e2e = Some(Arc::new(Default::default()));
    assert!(
        settings::records(Some(&ready), &Default::default())
            .knowledge
            .roots
            .is_empty(),
        "an acceptance-record snapshot is not root/provider authority"
    );
    Arc::make_mut(ready.e2e.as_mut().unwrap())
        .faults
        .push("provider refused the retained read".into());
    assert!(
        settings::records(Some(&ready), &Default::default())
            .knowledge
            .roots
            .is_empty()
    );
    let bundled = KnowledgeProvider {
        id: "bboxy450-term-data_source-bboxy".into(),
        name: "bboxy450-term-data_source-bboxy".into(),
        sha256: None,
    };
    ready.knowledge_roots = Some(Arc::new(KnowledgeStatus {
        roots: vec![KnowledgeRoot {
            id: "host-root".into(),
            path: "/private/root".into(),
            provider: bundled.clone(),
            revision: 3,
            records: Some(418),
            process: None,
            refusal: None,
        }],
        offered: vec![bundled],
    }));
    let observed = settings::records(Some(&ready), &Default::default());
    assert!(observed.knowledge_controls);
    let root = &observed.knowledge.roots[0];
    assert_eq!(root.provider, observed.knowledge.offered[0]);
    assert_eq!(root.records, Some(418));
    assert_eq!(root.answering, "logic · in process");
    assert_eq!(root.digests, None);
    assert_eq!(root.refused, None);
    let external = KnowledgeProvider {
        id: "checked-pin".into(),
        name: "bboxy450-term-data_source-root-fixture".into(),
        sha256: Some("a".repeat(64)),
    };
    let status = Arc::make_mut(ready.knowledge_roots.as_mut().unwrap());
    status.offered.push(external.clone());
    status.roots[0].provider = external.clone();
    status.roots[0].records = None;
    status.roots[0].process = Some(77);
    status.roots[0].refusal = Some("provider_busy: root connection occupied".into());
    let refused = settings::records(Some(&ready), &Default::default());
    let root = &refused.knowledge.roots[0];
    assert_eq!(
        root.provider, external.name,
        "never invent a bundled fallback"
    );
    assert_eq!(root.records, None);
    assert_eq!(root.digests, None);
    assert_eq!(root.answering, "provider read unavailable");
    assert_eq!(
        root.refused.as_deref(),
        Some("provider_busy: root connection occupied")
    );
}

// keeps: spec ux-prototype §5.6
#[test]
fn settings_mute_refusal_is_visible_and_success_waits_for_authoritative_metadata() {
    use bboxy450_term::{cockpit::UiMsg, messaging::Reply};
    use bbt_proto::messaging::{Request, Response};
    let id = bbt_ui_model::SeatId::from_u128(1);
    let swarm = bbt_ui_model::SwarmId::from_u128(2);
    let op = bbt_mux::Ulid::new();
    let mut model = Model::default();
    model.ui.seats.insert(
        id,
        bbt_ui_model::Seat::new(id, swarm, [0; 32], "real", "codex"),
    );
    let message = UiMsg::Messaging {
        id: op,
        swarm_id: bbt_proto::SwarmId::from_u128(2),
        request: Request::Mute {
            seat: bbt_mux::Ulid(1),
            muted: true,
        },
    };
    let mut mute = settings::Mute::default();
    mute.requested(&message);
    Arc::make_mut(&mut model.operation_results).push(op, Reply::Refused("held identity".into()));
    mute.observe(&model);
    assert!(!mute.pending());
    assert!(mute.notice.as_ref().unwrap().text.contains("held identity"));
    assert!(!model.ui.seats[&id].muted);
    mute.requested(&message);
    Arc::make_mut(&mut model.operation_results)
        .push(op, Reply::Messaging(Response::Muted { notice: None }));
    mute.observe(&model);
    assert!(mute.pending());
    assert!(!model.ui.seats[&id].muted);
    model.ui.seats.get_mut(&id).unwrap().muted = true;
    mute.observe(&model);
    assert!(!mute.pending());
    assert_eq!(mute.notice.as_ref().unwrap().text, "Seat muted");
}

// keeps: spec ux-prototype §5.6
#[test]
fn settings_persisted_policy_drives_attention_and_keeps_focus_and_mute_suppression() {
    use bbt_proto::{Activity, Grade, GuiSeat, ProcessStatus, SeatState, WaitKind};
    use std::{collections::BTreeSet, time::Instant};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "# keep policy comments\n[notifications]\nenabled = true\nonly_when_not_frontmost = true\ndisabled_harnesses = [\"codex\"]\n",
    )
    .unwrap();
    let store = Store::new(path.clone());
    let load = || {
        let mut state = State::default();
        state.accept(&settings::Snapshot {
            preferences: store.load().unwrap(),
            loaded: true,
            ..Default::default()
        });
        state
    };
    let mut model = Model::default();
    let mut seats: Vec<_> = [(1, "codex"), (2, "claude")]
        .into_iter()
        .map(|(id, harness)| GuiSeat {
            attention: None,
            record: bbt_proto::registry::SeatRecord::new(
                bbt_mux::Ulid(id),
                bbt_mux::Ulid(3),
                "waiting fixture",
                harness,
            ),
            state: SeatState {
                generation: 1,
                process: ProcessStatus::Running { pid: 9 },
                activity: Activity::Waiting(WaitKind::Question),
                grade: Grade::Shim,
                title: String::new(),
                detail: None,
                turn_open: true,
                observed_ms: 10,
                subagents: vec![],
            },
        })
        .collect();
    for seat in &seats {
        model.observe_attention(seat, Instant::now());
    }
    let codex = bbt_ui_model::SeatId::from_u128(1);
    let claude = bbt_ui_model::SeatId::from_u128(2);
    let unloaded = State::default();
    assert!(
        model
            .notifications(&unloaded.notification_policy(), false, None)
            .is_empty(),
        "startup must not post before the saved policy is known"
    );
    let state = load();
    let desired = model.notifications(&state.notification_policy(), false, None);
    assert_eq!(desired.len(), 1);
    assert!(
        desired
            .values()
            .next()
            .unwrap()
            .notification
            .title
            .ends_with("claude")
    );
    assert!(
        model
            .notifications(&state.notification_policy(), true, None)
            .is_empty()
    );

    store.save(&[Patch::BackgroundOnly(false)]).unwrap();
    let state = load();
    assert_eq!(
        model
            .notifications(&state.notification_policy(), true, Some(codex))
            .len(),
        1
    );
    assert!(
        model
            .notifications(&state.notification_policy(), true, Some(claude))
            .is_empty()
    );

    store.save(&[Patch::NotificationsEnabled(false)]).unwrap();
    let state = load();
    assert_eq!(
        state.preferences.notifications.disabled_harnesses,
        BTreeSet::from(["codex".into()])
    );
    assert!(
        model
            .notifications(&state.notification_policy(), false, None)
            .is_empty()
    );
    store
        .save(&[
            Patch::NotificationsEnabled(true),
            Patch::DisabledHarnesses(BTreeSet::new()),
        ])
        .unwrap();
    let state = load();
    assert_eq!(
        model
            .notifications(&state.notification_policy(), false, None)
            .len(),
        2
    );
    seats[0].record.muted = true;
    model.observe_attention(&seats[0], Instant::now());
    assert_eq!(
        model
            .notifications(&state.notification_policy(), false, None)
            .len(),
        1
    );
    assert!(
        model
            .notifications(&state.notification_policy(), true, Some(claude))
            .is_empty()
    );
    assert!(
        std::fs::read_to_string(path)
            .unwrap()
            .contains("# keep policy comments")
    );
}
