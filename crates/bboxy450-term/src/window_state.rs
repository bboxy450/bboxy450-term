// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure journal representation of the visible window, never terminal bytes or grants.
use bbt_mux::Ulid;
use bbt_proto::gui::{WindowScroll, WindowStage, WindowState, WindowSwarm, WindowTab, WindowTree};
use bbt_ui_model as ui;
use std::collections::BTreeSet;

fn tree(source: &ui::SplitNode) -> WindowTree {
    match source {
        ui::SplitNode::Leaf(id) => WindowTree::Seat { id: Ulid(id.0.0) },
        ui::SplitNode::Split { dir, ratio, a, b } => WindowTree::Split {
            row: *dir == ui::Axis::Row,
            ratio: *ratio,
            a: Box::new(tree(a)),
            b: Box::new(tree(b)),
        },
    }
}
fn decoded(source: &WindowTree) -> ui::SplitNode {
    match source {
        WindowTree::Seat { id } => ui::SplitNode::Leaf(ui::SeatId::from_u128(id.0)),
        WindowTree::Split { row, ratio, a, b } => ui::SplitNode::split(
            if *row {
                ui::Axis::Row
            } else {
                ui::Axis::Column
            },
            *ratio,
            decoded(a),
            decoded(b),
        ),
    }
}
/// Called once per produced frame. Unknown/offscreen positions retain the prior known anchor.
pub fn capture(
    model: &crate::model::Model,
    stage: &ui::Stage,
    size: [f32; 2],
    prior: Option<&WindowState>,
) -> WindowState {
    WindowState {
        version: bbt_proto::gui::WINDOW_STATE_VERSION,
        size,
        sidebar_width: model.ui.window.sidebar.width,
        sidebar_scroll: model.ui.window.sidebar.scroll,
        sidebar_shown: model.ui.window.title_bar.sidebar_shown,
        open_swarms: model
            .ui
            .window
            .sidebar
            .open
            .iter()
            .map(|id| Ulid(id.0.0))
            .collect(),
        sidebar_sections_closed: prior
            .or(model.window_restore.as_deref())
            .map_or_else(Vec::new, |saved| saved.sidebar_sections_closed.clone()),
        swarms: model
            .ui
            .window
            .sidebar
            .swarms
            .iter()
            .map(|swarm| WindowSwarm {
                id: Ulid(swarm.id.0.0),
                workdir: swarm.workdir.clone(),
                launch_environment: swarm.launch_environment.clone(),
                recent_directories: swarm.recent_directories.clone(),
                tabs: swarm
                    .tabs
                    .iter()
                    .map(|tab| WindowTab {
                        id: Ulid(tab.id.0.0),
                        name: tab.name.clone(),
                        shortcut: tab.shortcut,
                        expanded: tab.expanded,
                        seats: tab.seats.iter().map(|id| Ulid(id.0.0)).collect(),
                        tree: tree(&tab.tree),
                    })
                    .collect(),
            })
            .collect(),
        stage: WindowStage {
            swarm: stage.swarm.map(|id| Ulid(id.0.0)),
            tab: stage.tab.map(|id| Ulid(id.0.0)),
            focused: stage.focused.map(|id| Ulid(id.0.0)),
        },
        scroll: model
            .ui
            .seats
            .iter()
            .filter(|(_, seat)| seat.lifecycle == ui::Lifecycle::Open)
            .map(|(id, _)| {
                if model.window_waiting.contains(id)
                    && let Some(old) =
                        prior.and_then(|old| old.scroll.iter().find(|s| s.seat.0 == id.0.0))
                {
                    return old.clone();
                }
                let position = model
                    .scroll
                    .position(*id, model.history.get(id).map(std::sync::Arc::as_ref));
                if position.top_row.is_none()
                    && let Some(old) =
                        prior.and_then(|old| old.scroll.iter().find(|s| s.seat.0 == id.0.0))
                {
                    return old.clone();
                }
                WindowScroll {
                    seat: Ulid(id.0.0),
                    top_row: position.top_row,
                    live: position.live,
                }
            })
            .collect(),
    }
}
/// Metadata stays authoritative. Remove gone seats; append seats created while no GUI was attached.
pub fn restore(model: &mut crate::model::Model, saved: &WindowState) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for swarm in &mut model.ui.window.sidebar.swarms {
        let Some(old) = saved.swarms.iter().find(|s| s.id.0 == swarm.id.0.0) else {
            continue;
        };
        let current = swarm.tabs.clone();
        swarm.workdir = old.workdir.clone();
        swarm.launch_environment = old.launch_environment.clone();
        swarm.recent_directories = old.recent_directories.clone();
        let mut tabs = Vec::new();
        for tab in &old.tabs {
            let mut next = Some(decoded(&tab.tree));
            for seat in decoded(&tab.tree).leaves() {
                let valid = model
                    .ui
                    .seats
                    .get(&seat)
                    .is_some_and(|s| s.swarm == swarm.id && s.lifecycle == ui::Lifecycle::Open);
                if !valid {
                    next = next.and_then(|tree| ui::prune(&tree, seat));
                }
            }
            let Some(next) = next else {
                continue;
            };
            let leaves = next.leaves();
            if leaves.iter().any(|seat| !seen.insert(*seat)) {
                return Err("window state repeats a seat".into());
            }
            let mut restored = ui::Tab::new(ui::TabId::from_u128(tab.id.0), tab.name.clone(), next);
            restored.seats = tab
                .seats
                .iter()
                .map(|id| ui::SeatId::from_u128(id.0))
                .filter(|id| leaves.contains(id))
                .collect();
            restored.shortcut = tab.shortcut;
            restored.expanded = tab.expanded;
            tabs.push(restored);
        }
        for mut tab in current {
            let mut next = Some(tab.tree.clone());
            for seat in tab
                .tree
                .leaves()
                .into_iter()
                .filter(|seat| seen.contains(seat))
            {
                next = next.and_then(|tree| ui::prune(&tree, seat));
            }
            let Some(next) = next else {
                continue;
            };
            tab.tree = next;
            tab.seats = tab.tree.leaves();
            seen.extend(&tab.seats);
            if tabs.iter().any(|old| old.id == tab.id) {
                tab.id = ui::TabId::from_u128(Ulid::new().0);
            }
            tabs.push(tab);
        }
        swarm.tabs = tabs;
    }
    model.ui.window.sidebar.width = saved.sidebar_width;
    model.ui.window.sidebar.scroll = saved.sidebar_scroll;
    model.ui.window.sidebar.open = saved
        .open_swarms
        .iter()
        .map(|id| ui::SwarmId::from_u128(id.0))
        .filter(|id| model.ui.window.sidebar.swarms.iter().any(|s| s.id == *id))
        .collect();
    model.ui.window.title_bar.sidebar_shown = saved.sidebar_shown;
    let stage = ui::Stage {
        swarm: saved.stage.swarm.map(|id| ui::SwarmId::from_u128(id.0)),
        tab: saved.stage.tab.map(|id| ui::TabId::from_u128(id.0)),
        focused: saved.stage.focused.map(|id| ui::SeatId::from_u128(id.0)),
    };
    if model.ui.window.sidebar.swarms.iter().any(|s| {
        Some(s.id) == stage.swarm
            && ((s.tabs.is_empty() && stage.tab.is_none() && stage.focused.is_none())
                || s.tabs.iter().any(|t| {
                    Some(t.id) == stage.tab && stage.focused.is_none_or(|id| t.tree.contains(id))
                }))
    }) {
        model.ui.window.stage = stage;
    } else if let Some(swarm) = model
        .ui
        .window
        .sidebar
        .swarms
        .iter()
        .find(|s| !s.tabs.is_empty())
    {
        let tab = &swarm.tabs[0];
        model.ui.window.stage = ui::Stage {
            swarm: Some(swarm.id),
            tab: Some(tab.id),
            focused: tab.tree.leaves().first().copied(),
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_preserves_prior_section_choices_before_restore_fallback() {
        let mut model = crate::model::Model::default();
        let stage = model.ui.window.stage.clone();
        let mut restored = capture(&model, &stage, [1280., 720.], None);
        assert!(restored.sidebar_sections_closed.is_empty());
        restored.sidebar_sections_closed = vec!["providers/Cloud".into()];
        model.window_restore = Some(std::sync::Arc::new(restored));
        assert_eq!(
            capture(&model, &stage, [1280., 720.], None).sidebar_sections_closed,
            ["providers/Cloud"]
        );
        let mut prior = capture(&model, &stage, [1280., 720.], None);
        prior.sidebar_sections_closed = vec!["providers/Hosts".into()];
        assert_eq!(
            capture(&model, &stage, [1280., 720.], Some(&prior)).sidebar_sections_closed,
            ["providers/Hosts"]
        );
        prior.sidebar_sections_closed.clear();
        assert!(
            capture(&model, &stage, [1280., 720.], Some(&prior))
                .sidebar_sections_closed
                .is_empty()
        );
    }
    // keeps: decision #344; UI data roundtrips while current registry metadata wins
    #[test]
    fn layout_and_focus_roundtrip_without_restoring_seat_authority() {
        let a = ui::SeatId::from_u128(1);
        let b = ui::SeatId::from_u128(2);
        let swarm = ui::SwarmId::from_u128(3);
        let mut model = crate::model::Model {
            ui: ui::Model::new(
                vec![ui::Swarm::new(
                    swarm,
                    "fixture",
                    vec![ui::Tab::new(
                        ui::TabId::from_u128(4),
                        "split",
                        ui::SplitNode::split(
                            ui::Axis::Row,
                            0.37,
                            ui::SplitNode::Leaf(a),
                            ui::SplitNode::Leaf(b),
                        ),
                    )],
                )],
                vec![
                    ui::Seat::new(a, swarm, [1; 32], "one", "zsh"),
                    ui::Seat::new(b, swarm, [2; 32], "two", "zsh"),
                ],
            )
            .unwrap(),
            ..Default::default()
        };
        model.ui.window.stage.focused = Some(b);
        model.ui.window.sidebar.swarms[0].workdir = Some("/project with spaces".into());
        model.ui.window.sidebar.swarms[0].launch_environment =
            Some(vec![("TEST".into(), "two words".into())]);
        model.ui.window.sidebar.swarms[0].recent_directories =
            vec!["/project with spaces".into(), "/older".into()];
        let state = capture(&model, &model.ui.window.stage, [1280., 750.], None);
        let bytes = serde_json::to_vec(&state).unwrap();
        let decoded = serde_json::from_slice(&bytes).unwrap();
        let mut next = model.clone();
        next.ui.window.sidebar.swarms[0].tabs[0].tree = ui::SplitNode::split(
            ui::Axis::Column,
            0.7,
            ui::SplitNode::Leaf(b),
            ui::SplitNode::Leaf(a),
        );
        next.ui.window.stage.focused = Some(a);
        next.ui.seats.get_mut(&a).unwrap().name = "current registry".into();
        let metadata = next.ui.seats.clone();
        restore(&mut next, &decoded).unwrap();
        assert_eq!(next.ui.seats, metadata);
        assert_eq!(next.ui.window.stage.focused, Some(b));
        assert_eq!(
            capture(&next, &next.ui.window.stage, [1280., 750.], None),
            state
        );
        let mut legacy = serde_json::to_value(&state).unwrap();
        legacy["version"] = 1.into();
        legacy["swarms"][0]
            .as_object_mut()
            .unwrap()
            .remove("workdir");
        legacy["swarms"][0]
            .as_object_mut()
            .unwrap()
            .remove("launch_environment");
        legacy["swarms"][0]
            .as_object_mut()
            .unwrap()
            .remove("recent_directories");
        let legacy: WindowState = serde_json::from_value(legacy).unwrap();
        restore(&mut next, &legacy).unwrap();
        assert_eq!(next.ui.window.sidebar.swarms[0].workdir, None);
        assert_eq!(next.ui.window.sidebar.swarms[0].launch_environment, None);
        assert!(
            next.ui.window.sidebar.swarms[0]
                .recent_directories
                .is_empty()
        );
    }
}
