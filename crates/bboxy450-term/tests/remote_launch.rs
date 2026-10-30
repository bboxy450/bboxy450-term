// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private transport executables print argv; no real host or credential is used.
#![allow(
    clippy::disallowed_methods,
    reason = "private scripted transport witnesses, never UI or network"
)]
use bboxy450_term::{settings, spawn_params};
use bbt_ui::modules::settings::{
    Machine,
    controller::{Action, Controller, Field, Patch},
};
use std::{
    collections::BTreeMap,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
};

fn fields(machine: &Machine) -> Vec<bbt_ui_model::SpawnField> {
    let mut fields = spawn_params::with_machine(
        spawn_params::resolve("codex", &BTreeMap::new(), None, "/local-only"),
        machine,
    );
    for field in &mut fields {
        match field.key {
            bbt_ui_model::FieldKey::Arguments => {
                field.value = "--literal '$(must-stay-literal)' ''".into()
            }
            bbt_ui_model::FieldKey::Environment => field.value = "A='two words' B=''".into(),
            _ => {}
        }
    }
    fields
}

#[test]
fn compose_machine_fields_equals_spawned_argv_for_every_transport() {
    let root = tempfile::tempdir().unwrap();
    for kind in ["ssh", "mosh", "et"] {
        let executable = root.path().join(kind);
        std::fs::write(
            &executable,
            "#!/bin/sh\nprintf '%s\\000' \"${0##*/}\" \"$@\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        for custom in [false, true] {
            let machine = Machine {
                via: kind.into(),
                connect: "user@fixture.invalid".into(),
                port: if custom { "2222" } else { "" }.into(),
                key: if custom { "~/.ssh/a key" } else { "" }.into(),
                directory: "~/project with spaces".into(),
                routes: vec!["user@other.invalid".into()],
                ..Default::default()
            };
            let resolved = spawn_params::Resolved::from_fields(
                &fields(&machine),
                root.path().to_str().unwrap(),
            )
            .unwrap();
            let mut harness = bbt_mux::adapters::HarnessRecord::builtin("codex").unwrap();
            harness.command = std::iter::once(resolved.command.clone())
                .chain(resolved.args.clone())
                .collect();
            let mut seat = bbt_mux::registry::SeatRecord::new(
                bbt_mux::Ulid::new(),
                bbt_mux::Ulid::new(),
                "fixture",
                "codex",
            );
            let options = bbt_mux::adapters::launch::LaunchOptions {
                via: resolved.via.clone(),
                env: resolved.env.clone(),
                cwd: resolved.cwd.clone().into(),
                role_prompt: String::new(),
                executable: std::env::current_exe().unwrap(),
                user_home: root.path().into(),
                user_zdotdir: None,
                resume: false,
            };
            let spec =
                bbt_mux::adapters::launch::transport_spec(&seat, &harness, &options).unwrap();
            assert_eq!(spec.cwd, root.path());
            assert!(
                !spec.env.contains_key("A"),
                "remote environment must not leak into the local client"
            );
            let expected = resolved.argv();
            assert_eq!(
                std::iter::once(spec.program.clone())
                    .chain(spec.args.clone())
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(shell_words::split(&resolved.runs()).unwrap(), expected);
            let output = Command::new(&executable)
                .args(&spec.args)
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", root.path())
                .current_dir(&spec.cwd)
                .stdin(Stdio::null())
                .output()
                .unwrap();
            assert!(output.status.success());
            let actual = output
                .stdout
                .split(|byte| *byte == 0)
                .map(|s| String::from_utf8(s.to_vec()).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(&actual[..actual.len() - 1], expected);
            seat.generation += 1;
            let restart =
                bbt_mux::adapters::launch::transport_spec(&seat, &harness, &options).unwrap();
            assert_eq!((restart.program, restart.args), (spec.program, spec.args));
            assert_eq!(restart.generation, seat.generation);
        }
    }
}

#[test]
fn machine_choices_persist_without_changing_local_directory_or_role_defaults() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("settings.toml");
    std::fs::write(
        &path,
        "# keep this comment\n[unrelated]\nvalue = 'keep me'\n",
    )
    .unwrap();
    let store = settings::Store::new(path.clone());
    let machine = Machine {
        connect: "user@one.invalid".into(),
        routes: vec!["user@two.invalid".into()],
        directory: "~/remote".into(),
        ..Default::default()
    };
    let patch = |machine| Patch::MachineLaunch {
        id: "ssh:fixture".into(),
        machine,
    };
    store.save(&[patch(machine.clone())]).unwrap();
    assert_eq!(store.load().unwrap().machines["ssh:fixture"], machine);
    let resolved =
        spawn_params::Resolved::from_fields(&fields(&machine), root.path().to_str().unwrap())
            .unwrap();
    let captured = resolved.argv();
    for route in ["user@two.invalid", "user@one.invalid"] {
        let mut edited = machine.clone();
        edited.connect = route.into();
        store.save(&[patch(edited)]).unwrap();
        assert_eq!(
            resolved.argv(),
            captured,
            "Settings changes cannot re-resolve a captured spawn"
        );
    }
    let remembered = resolved.remember_machine(machine);
    store.save(&[patch(remembered.clone())]).unwrap();
    assert_eq!(store.load().unwrap().machines["ssh:fixture"], remembered);
    assert_eq!(remembered.recent_directories, ["~/remote"]);
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("# keep this comment")
    );
    assert!(std::fs::read_to_string(&path).unwrap().contains("keep me"));
    let mut swarm =
        bbt_ui_model::Swarm::new(bbt_ui_model::SwarmId::from_u128(1), "fixture", vec![]);
    swarm.workdir = Some("/local-only".into());
    let mut harnesses = BTreeMap::new();
    resolved.remember("codex", &mut swarm, &mut harnesses);
    assert_eq!(swarm.workdir.as_deref(), Some("/local-only"));
    assert!(swarm.recent_directories.is_empty());
    assert_eq!(harnesses["codex"].role, None);
}

#[test]
fn native_machine_text_control_emits_a_persistable_patch() {
    use bbt_ui::text_edit::Command as Edit;
    let mut values = bbt_ui::modules::settings::Records { machine_controls: true, ..Default::default() };
    values.machine_defaults.insert(
        "ssh:fixture".into(),
        Machine {
            connect: "user@fixture.invalid".into(),
            ..Default::default()
        },
    );
    let defaults = values.clone();
    let mut controller = Controller::new("Machines");
    controller.handle(
        Action::Toggle(Field::Machine("ssh:fixture".into(), "port".into())),
        &mut values,
        &defaults,
    );
    assert!(controller.editing());
    controller.handle(Action::Search(Edit::SelectAll), &mut values, &defaults);
    controller.handle(
        Action::Search(Edit::Insert("2222".into())),
        &mut values,
        &defaults,
    );
    let patch = controller
        .handle(Action::Commit, &mut values, &defaults)
        .unwrap();
    patch.validate().unwrap();
    assert!(matches!(patch, Patch::MachineLaunch { machine, .. } if machine.port == "2222"));
    assert_eq!(values.machine_defaults["ssh:fixture"].port, "2222");
}

#[test]
fn add_machine_is_available_without_a_remote_provider_snapshot() {
    use bbt_ui::text_edit::Command as Edit;
    let mut gallery = bbt_ui::modules::settings::Records::default();
    let gallery_defaults = gallery.clone();
    let mut unhosted = Controller::new("Machines");
    unhosted.handle(Action::Toggle(Field::MachineNew), &mut gallery, &gallery_defaults);
    assert!(!unhosted.editing(), "a gallery never enables host persistence controls");
    let mut values = bbt_ui::modules::settings::Records { machine_controls: true, ..Default::default() };
    let defaults = values.clone();
    let mut controller = Controller::new("Machines");
    controller.handle(Action::Toggle(Field::MachineNew), &mut values, &defaults);
    assert!(controller.editing());
    controller.handle(
        Action::Search(Edit::Insert("user@new.invalid".into())),
        &mut values,
        &defaults,
    );
    let patch = controller
        .handle(Action::Commit, &mut values, &defaults)
        .unwrap();
    let mut prefs = settings::Preferences::default();
    prefs.apply(&patch).unwrap();
    let projected = settings::records(None, &prefs);
    assert_eq!(projected.machines.len(), 1);
    assert_eq!(projected.machines[0].1, "user@new.invalid");
    assert_eq!(
        projected.destination_ids["machine/0"],
        "ssh:user@new.invalid"
    );
    assert_eq!(
        spawn_params::machine_target("ssh:user@new.invalid:codex:right", &prefs.machines),
        Some(("codex".into(), "ssh:user@new.invalid".into()))
    );
    assert!(
        spawn_params::machine_target("ssh:user@new.invalid:unknown", &prefs.machines).is_none()
    );
}

#[test]
fn route_changes_keep_prior_network_choices_in_settings_and_launcher() {
    let mut machine = Machine { connect: "user@one.invalid".into(), ..Default::default() };
    machine.set("connect", "user@two.invalid".into());
    assert!(machine.routes.contains(&"user@one.invalid".into()));
    assert!(machine.routes.contains(&"user@two.invalid".into()));
    let mut launch = fields(&machine);
    launch.iter_mut().find(|field| field.key == bbt_ui_model::FieldKey::Connect).unwrap().value = "user@three.invalid".into();
    let captured = spawn_params::Resolved::from_fields(&launch, "/private/home").unwrap();
    let remembered = captured.remember_machine(machine);
    assert_eq!(remembered.connect, "user@three.invalid");
    assert_eq!(remembered.routes, ["user@three.invalid", "user@two.invalid", "user@one.invalid"]);
}
