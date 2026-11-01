// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Table-driven native presentation and launch-key contracts; no provider/network.
use super::*;
use bbt_ui::{
    self as ui, components as c,
    modules::{harness, seat_row::SeatRow, settings::Settings},
};
use bbt_ui_model::layout::Rect;

fn solve(el: c::Element) -> c::Layout {
    el.layout(
        Rect::new(0., 0., 1400., 1800.),
        ui::Context::new(ui::Theme::Dark, ui::Density::Cozy),
        &scene::Metrics::default(),
    )
    .unwrap()
}
fn nodes(node: &c::Layout) -> Vec<&c::Layout> {
    std::iter::once(node)
        .chain(node.children.iter().flat_map(nodes))
        .collect()
}
fn has_mark(layout: &c::Layout, kind: &str) -> bool {
    nodes(layout).iter().any(|n| match &n.kind {
        c::LayoutKind::Image { asset, .. } => harness::icon(kind) == asset,
        c::LayoutKind::Icon { name, .. } => harness::icon(kind) == name,
        _ => false,
    })
}
fn has_text(layout: &c::Layout, text: &str) -> bool {
    nodes(layout).iter().any(|n| matches!(&n.kind, c::LayoutKind::Text { runs, .. } if runs.iter().map(|r| r.text.as_str()).collect::<String>() == text))
}
fn assert_display_names(layout: &c::Layout, surface: &str) {
    for node in nodes(layout) {
        if let c::LayoutKind::Text { runs, .. } = &node.kind {
            let text = runs.iter().map(|r| r.text.as_str()).collect::<String>();
            assert!(
                !text.to_lowercase().contains("bboxy harness"),
                "{surface}: legacy label {text:?}"
            );
            assert!(
                !text
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|word| word == "Bboxy"),
                "{surface}: interim label {text:?}"
            );
        }
    }
}
fn complete(surface: &str, ids: &[&str]) -> Result<(), String> {
    for kind in bboxy450_term::settings::roles::HARNESSES {
        if !ids.contains(&kind) {
            return Err(format!("{surface} omits {kind}"));
        }
    }
    Ok(())
}
#[test]
fn harness_branding_all_menu_surfaces_and_launch_keys() {
    let items = launcher_items(None);
    let ids: Vec<_> = items.iter().map(|i| i.id.as_str()).collect();
    complete("launcher", &ids).unwrap();
    for side in [
        bbt_proto::seat_state::Side::Right,
        bbt_proto::seat_state::Side::Bottom,
    ] {
        let filtered = preset_launcher_items(items.clone(), Some(side));
        complete(
            "split picker",
            &filtered.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
        )
        .unwrap();
    }
    for kind in bboxy450_term::settings::roles::HARNESSES {
        let item = items.iter().find(|i| i.id == kind).unwrap();
        let (_, label, _, color, _) = harness::launchables().find(|(id, ..)| *id == kind).unwrap();
        assert_eq!(item.label, label);
        if kind == "bboxy450-harness" {
            assert_eq!(item.label, "bboxy450-harness");
            assert_eq!(
                bbt_mux::adapters::HarnessRecord::builtin(kind)
                    .unwrap()
                    .name,
                "bboxy450-harness"
            );
        }
        assert_eq!(item.tile.as_deref(), Some(color));
        let fields =
            bboxy450_term::spawn_params::resolve(kind, &Default::default(), None, "/fixture");
        let resolved =
            bboxy450_term::spawn_params::Resolved::from_fields(&fields, "/fixture").unwrap();
        let builtin = bbt_mux::adapters::HarnessRecord::builtin(kind).unwrap();
        assert_eq!(
            resolved.command, builtin.command[0],
            "menu executable {kind}"
        );
        assert_eq!(
            resolved.args,
            builtin.command[1..],
            "menu must preserve builtin subcommand {kind}"
        );

        let mut controller = new_launcher(None, 0);
        controller.handle(LauncherCommand::Query(label.into()));
        assert!(
            matches!(controller.handle(LauncherCommand::Activate(Default::default())), bbt_ui::modules::launcher::controller::Outcome::Pick { id, .. } if id == kind)
        );
        let mut picker = new_launcher(None, 0).launcher;
        picker.items = vec![item.clone()];
        let layout = solve(picker.element(ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)));
        assert!(has_mark(&layout, kind), "launcher {kind}");
        assert!(has_text(&layout, label), "launcher label {kind}");
        assert_display_names(&layout, "launcher");
        for leaf in item.children.as_ref().unwrap() {
            assert!(
                matches!(spawn_pick(&leaf.id, Default::default(), None, None), Some(live::UiMsg::Spawn {harness_kind, ..}) if harness_kind == kind)
            );
        }
        assert!(
            matches!(default_spawn(kind, Default::default(), None), Some(live::UiMsg::Spawn {harness_kind, ..}) if harness_kind == kind)
        );
    }
    let mut remote = items.clone();
    configure_machine_items(
        &mut remote,
        &std::collections::BTreeMap::from([(
            "ssh:fixture".into(),
            bbt_ui::modules::settings::Machine::default(),
        )]),
    );
    let host = &remote
        .iter()
        .find(|i| i.id == "ssh")
        .unwrap()
        .children
        .as_ref()
        .unwrap()[0];
    complete(
        "remote picker",
        &host
            .children
            .as_ref()
            .unwrap()
            .iter()
            .map(|i| i.harness.as_str())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let machines = std::collections::BTreeMap::from([(
        "ssh:fixture".into(),
        bbt_ui::modules::settings::Machine::default(),
    )]);
    for kind in bboxy450_term::settings::roles::HARNESSES {
        for suffix in ["", ":right", ":bottom", ":tab"] {
            assert_eq!(
                bboxy450_term::spawn_params::machine_target(
                    &format!("ssh:fixture:{kind}{suffix}"),
                    &machines
                ),
                Some((kind.into(), "ssh:fixture".into()))
            );
        }
    }
    let proto = bbt_proto::gui::RoleCatalog::builtin();
    complete(
        "role catalogue",
        &proto
            .harnesses
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
    )
    .unwrap();
}
#[test]
fn harness_branding_omission_control_is_rejected() {
    let mut ids = bboxy450_term::settings::roles::HARNESSES.to_vec();
    ids.retain(|id| *id != "bboxy450-harness");
    assert_eq!(
        complete("broken picker", &ids),
        Err("broken picker omits bboxy450-harness".into())
    );
}
#[test]
fn harness_branding_settings_and_seat_surfaces() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let metrics = scene::Metrics::default();
    let values = bboxy450_term::settings::records(None, &Default::default());
    complete(
        "Settings",
        &values
            .harness
            .iter()
            .map(|(id, _)| id.as_str())
            .collect::<Vec<_>>(),
    )
    .unwrap();
    let mut view = Settings {
        values: values.clone(),
        defaults: values,
        launch: harness::launchables()
            .map(|(id, label, _, tile, _)| ui::modules::settings::Launch {
                harness: id.into(),
                label: label.into(),
                tile: tile.into(),
            })
            .collect(),
        ..Default::default()
    };
    for section in ["Harnesses", "Notifications"] {
        view.section = section.into();
        let layout = solve(view.element(ctx, &metrics, 1400., 1800.));
        assert_display_names(&layout, section);
        assert!(
            has_text(&layout, "bboxy450-harness"),
            "{section} must use the canonical name"
        );
        for kind in bboxy450_term::settings::roles::HARNESSES {
            assert!(has_mark(&layout, kind), "{section} {kind}");
            let label = if section == "Notifications" {
                kind
            } else {
                view.launch
                    .iter()
                    .find(|i| i.harness == kind)
                    .unwrap()
                    .label
                    .as_str()
            };
            assert!(has_text(&layout, label), "{section} {label}");
        }
    }
    for kind in bboxy450_term::settings::roles::HARNESSES {
        let row = SeatRow::new("fixture", "Fixture Executor", kind);
        let closed = ui::modules::closed_strip::ClosedStrip {
            key: "fixture".into(),
            name: "Fixture".into(),
            harness: kind.into(),
            remaining_ms: 1000,
            grace_ms: 2000,
        };
        let pending = ui::modules::swarm_card::PendingSpawn {
            name: "Fixture".into(),
            harness: kind.into(),
            status: "starting".into(),
            caption: "starting".into(),
            dismiss: false,
        };
        for (surface, element) in [
            ("sidebar seat row", row.element(ctx)),
            (
                "pane strip",
                ui::modules::strip::Strip::new(row.clone()).element(ctx),
            ),
            ("closed seat", closed.element(ctx, &metrics, 600.)),
            (
                "launch placeholder",
                ui::modules::swarm_card::pending_row(&pending, ctx),
            ),
        ] {
            let layout = solve(element);
            assert!(has_mark(&layout, kind), "{surface} {kind}");
            assert_display_names(&layout, surface);
        }
        let graph = ui::modules::graph_tab::GraphTab {
            seats: vec![ui::modules::graph_tab::Seat {
                key: "fixture".into(),
                name: "Fixture".into(),
                harness: kind.into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let layout = solve(graph.element(ctx, &metrics));
        assert!(has_mark(&layout, kind), "graph {kind}");
        assert_display_names(&layout, "graph");
        let patch = ui::modules::settings::controller::Patch::HarnessLaunch {
            harness: kind.into(),
            command: kind.into(),
            arguments: vec![],
        };
        patch.validate().unwrap();
    }
}
#[test]
#[allow(
    clippy::disallowed_methods,
    reason = "private windowless native rendering witness"
)]
fn harness_branding_headless_menu() {
    let scene = scene::Scene::new();
    let launcher = new_launcher(None, 0).launcher;
    let menu = scene.launcher_layout([1280., 1000.], &launcher).unwrap();
    assert!(has_mark(menu.solved(), "bboxy450-harness"));
    assert!(has_text(menu.solved(), "bboxy450-harness"));
    assert_display_names(menu.solved(), "headless menu");
    let mut gpu = bbt_rt::handle()
        .block_on(gpu::VerificationFrame::new())
        .unwrap();
    gpu.resize([1280, 1000]);
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    gpu.draw(
        scene.paint(&[], [1280., 1000.], None, Some(&menu)),
        &Default::default(),
        &[],
        1,
        send,
    )
    .unwrap();
    receive.recv_timeout(Duration::from_secs(30)).unwrap();
    let private = tempfile::tempdir().unwrap();
    let path = std::env::var_os("BBOXY_BRANDING_CAPTURE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| private.path().join("menu.png"));
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    gpu.capture(path.clone(), None, log::Logger::new(), move |result| {
        send.send(result).unwrap();
    });
    receive
        .recv_timeout(Duration::from_secs(30))
        .unwrap()
        .unwrap();
    assert!(std::fs::metadata(&path).unwrap().len() > 1000);
    let fox = nodes(menu.solved())
        .into_iter()
        .find(|n| matches!(&n.kind, c::LayoutKind::Image { asset, .. } if asset == "bboxy"))
        .unwrap()
        .rect;
    let mut reader =
        png::Decoder::new(std::io::BufReader::new(std::fs::File::open(&path).unwrap()))
            .read_info()
            .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    let mut colors = std::collections::BTreeSet::new();
    let mut blues = 0;
    for y in fox.y.ceil() as usize..fox.bottom().floor() as usize {
        for x in fox.x.ceil() as usize..fox.right().floor() as usize {
            let at = (y * info.width as usize + x) * 4;
            let rgb = [pixels[at], pixels[at + 1], pixels[at + 2]];
            colors.insert(rgb);
            blues += usize::from(u16::from(rgb[2]) > u16::from(rgb[0]) + 25);
        }
    }
    assert!(
        colors.len() > 16 && blues > 20,
        "fox must render as a multicolour blue bitmap, not a generic glyph: {} colours, {blues} blue pixels",
        colors.len()
    );
    println!(
        "native menu: {}, fox={fox:?}, colours={}, blue_pixels={blues}",
        path.display(),
        colors.len()
    );
}

/// Deliberately red when explicitly selected, so the omission detector cannot pass silently.
#[test]
#[ignore = "broken control; make test-harness-branding-control expects exit 101"]
fn harness_branding_broken_picker() {
    let mut items = launcher_items(None);
    items.retain(|i| i.id != "bboxy450-harness");
    complete(
        "broken picker",
        &items.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
    )
    .unwrap();
}
