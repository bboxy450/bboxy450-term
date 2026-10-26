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
}

// keeps: spec ux-prototype §5.6
#[test]
fn settings_registry_is_labelled_reference_and_knowledge_never_invents_a_provider_or_count() {
    let model = Model::default();
    let records = settings::records(Some(&model), &Default::default());
    assert!(
        records
            .plugin_notice
            .as_deref()
            .unwrap()
            .contains("Reference fixtures")
    );
    assert_eq!(records.plugins.len(), 6);
    assert!(records.plugins.iter().all(|p| {
        p.health
            .say
            .as_deref()
            .unwrap()
            .starts_with("reference fixture")
    }));
    assert!(records.plugins.iter().any(|p| {
        p.alerts
            .iter()
            .any(|a| a == "this plugin takes 10 s on every hook")
    }));
    assert_eq!(
        records.knowledge.offered,
        ["bboxy450-term-data_source-bboxy"]
    );
    let root = &records.knowledge.roots[0];
    assert_eq!(root.provider, records.knowledge.offered[0]);
    assert_eq!(root.records, None);
    assert_eq!(root.digests, None);
    assert!(
        root.refused
            .as_deref()
            .unwrap()
            .contains("switching is unavailable")
    );
    assert!(root.answering.contains("pending"));
    let mut ready = model;
    ready.e2e = Some(Arc::new(Default::default()));
    assert_eq!(
        settings::records(Some(&ready), &Default::default())
            .knowledge
            .roots[0]
            .answering,
        "logic · in process"
    );
    Arc::make_mut(ready.e2e.as_mut().unwrap())
        .faults
        .push("provider refused the retained read".into());
    let refused = settings::records(Some(&ready), &Default::default());
    assert_eq!(
        refused.knowledge.roots[0].answering,
        "bundled read unavailable"
    );
    assert!(
        refused.knowledge.roots[0]
            .refused
            .as_deref()
            .unwrap()
            .contains("provider refused the retained read")
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
