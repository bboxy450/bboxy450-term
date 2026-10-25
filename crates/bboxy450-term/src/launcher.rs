//! App placement capture. Reusable launcher control lives with its UI module.

pub use bbt_ui::modules::launcher::controller::{CaretMove, Command, Controller, Outcome};
use bbt_ui_model::input::Modifiers;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedSpawn {
    pub spec: bbt_proto::SpawnSpec,
    pub layout_revision: u64,
}

/// Resolve an explicit operator pick once, on the GUI owner. A missing split
/// anchor is an error, never a fallback to another pane or a new tab.
pub fn operator_placement(
    pick: bbt_proto::gui::UiSpawn,
    model: &crate::model::Model,
    stage: &bbt_ui_model::Stage,
    layout_revision: u64,
) -> Result<CapturedSpawn, String> {
    use bbt_proto::gui::SpawnPlace;
    use bbt_proto::{
        SwarmId, TabId,
        seat_state::{Placement, Side},
    };
    let selected = pick.swarm.map(|id| SwarmId::from_u128(id.0));
    let (swarm, placement) = match pick.place {
        SpawnPlace::Space => {
            if selected.is_some() {
                return Err(
                    "invalid_placement: a new space cannot select an existing --swarm".into(),
                );
            }
            (
                SwarmId::from_u128(bbt_mux::Ulid::new().0),
                Placement::NewTab {
                    tab: TabId::from_u128(bbt_mux::Ulid::new().0),
                },
            )
        }
        SpawnPlace::Tab => (
            selected
                .or(stage.swarm)
                .unwrap_or_else(|| SwarmId::from_u128(bbt_mux::Ulid::new().0)),
            Placement::NewTab {
                tab: TabId::from_u128(bbt_mux::Ulid::new().0),
            },
        ),
        SpawnPlace::Right | SpawnPlace::Down => {
            let target = stage
                .focused
                .ok_or("placement_anchor_missing: no focused pane")?;
            let tab = stage
                .tab
                .ok_or("placement_anchor_missing: no selected tab")?;
            let row = model
                .ui
                .seats
                .get(&target)
                .ok_or("placement_anchor_missing: focused seat is absent")?;
            let (si, ti) = model
                .ui
                .tab_of(target)
                .ok_or("placement_anchor_missing: focused seat has no tab")?;
            let owner = &model.ui.window.sidebar.swarms[si];
            if Some(owner.id) != stage.swarm
                || owner.tabs[ti].id != tab
                || row.lifecycle != bbt_ui_model::Lifecycle::Open
            {
                return Err(
                    "placement_anchor_missing: focused pane is no longer open in the selected tab"
                        .into(),
                );
            }
            if selected.is_some_and(|swarm| swarm != owner.id) {
                return Err(
                    "placement_swarm_changed: split target belongs to a different swarm".into(),
                );
            }
            (
                owner.id,
                Placement::Split {
                    tab,
                    target,
                    target_generation: row.process_generation,
                    side: if pick.place == SpawnPlace::Right {
                        Side::Right
                    } else {
                        Side::Bottom
                    },
                },
            )
        }
    };
    Ok(CapturedSpawn {
        spec: bbt_proto::SpawnSpec {
            swarm,
            harness_kind: pick.harness,
            name: pick.name,
            role: pick.role,
            cwd: pick.cwd,
            location: bbt_proto::Where::Local,
            resume: None,
            placement: Some(placement),
        },
        layout_revision,
    })
}

/// Resolve only a placement suffix, never a harness name or shell command.
/// The caller still admits the returned root against its actual item catalogue.
pub fn captured_placement<'a>(
    id: &'a str,
    modifiers: Modifiers,
    model: Option<&crate::model::Model>,
    stage: Option<&bbt_ui_model::Stage>,
) -> (
    &'a str,
    Option<bbt_proto::SwarmId>,
    bbt_proto::seat_state::Placement,
) {
    use bbt_proto::seat_state::{Placement, Side};
    let (root, explicit) = match id.rsplit_once(':') {
        Some((root, "tab")) => (root, Some(None)),
        Some((root, "right")) => (root, Some(Some(Side::Right))),
        Some((root, "bottom")) => (root, Some(Some(Side::Bottom))),
        _ => (id, None),
    };
    let stage = stage.or_else(|| model.map(|model| &model.ui.window.stage));
    let swarm = stage.and_then(|stage| stage.swarm);
    let side = explicit.unwrap_or_else(|| {
        modifiers.command.then_some(if modifiers.shift {
            Side::Bottom
        } else {
            Side::Right
        })
    });
    let anchor = model.zip(stage).and_then(|(model, stage)| {
        let target = stage.focused?;
        let tab = stage.tab?;
        let seat = model.ui.seats.get(&target)?;
        let (si, ti) = model.ui.tab_of(target)?;
        let owner = &model.ui.window.sidebar.swarms[si];
        (Some(owner.id) == swarm && owner.tabs[ti].id == tab).then_some((
            target,
            tab,
            seat.process_generation,
        ))
    });
    let placement = match side.zip(anchor) {
        Some((side, (target, tab, target_generation))) => Placement::Split {
            tab,
            target,
            target_generation,
            side,
        },
        None => Placement::NewTab {
            tab: bbt_proto::TabId::from_u128(bbt_mux::Ulid::new().0),
        },
    };
    (root, swarm, placement)
}

#[cfg(test)]
mod operator_tests {
    use super::*;
    use bbt_proto::{
        gui::{SpawnPlace, UiSpawn},
        seat_state::{Placement, Side},
    };
    use bbt_ui_model::{
        Axis, Model as UiModel, Seat, SeatId, SplitNode, Swarm, SwarmId, Tab, TabId,
    };

    fn model() -> crate::model::Model {
        let swarm = SwarmId::from_u128(1);
        let mut a = Seat::new(SeatId::from_u128(2), swarm, [0; 32], "a", "zsh");
        a.process_generation = 7;
        let b = Seat::new(SeatId::from_u128(3), swarm, [1; 32], "b", "zsh");
        crate::model::Model {
            ui: UiModel::new(
                vec![Swarm::new(
                    swarm,
                    "swarm",
                    vec![Tab::new(
                        TabId::from_u128(4),
                        "tab",
                        SplitNode::split(
                            Axis::Row,
                            0.37,
                            SplitNode::Leaf(a.id),
                            SplitNode::Leaf(b.id),
                        ),
                    )],
                )],
                vec![a, b],
            )
            .unwrap(),
            ..Default::default()
        }
    }
    fn pick(place: SpawnPlace) -> UiSpawn {
        UiSpawn {
            harness: "zsh".into(),
            name: Some("chosen name".into()),
            role: Some("shell".into()),
            cwd: "/chosen cwd".into(),
            swarm: None,
            place,
        }
    }
    #[test]
    fn captured_operator_anchor_survives_later_focus_and_revision_changes() {
        let mut model = model();
        let mut stage = model.ui.window.stage.clone();
        let plan = operator_placement(pick(SpawnPlace::Down), &model, &stage, 19).unwrap();
        stage.focused = Some(SeatId::from_u128(3));
        let later = operator_placement(pick(SpawnPlace::Right), &model, &stage, 24).unwrap();
        assert_eq!(plan.layout_revision, 19);
        assert!(
            matches!(plan.spec.placement.clone(),Some(Placement::Split {target,target_generation:7,tab,side:Side::Bottom}) if target==SeatId::from_u128(2)&&tab==TabId::from_u128(4))
        );
        assert!(
            matches!(later.spec.placement,Some(Placement::Split {target,side:Side::Right,..}) if target==SeatId::from_u128(3))
        );
        assert_eq!(plan.spec.cwd, "/chosen cwd");
        assert_eq!(plan.spec.name.as_deref(), Some("chosen name"));
        assert_eq!(plan.spec.role.as_deref(), Some("shell"));
        // Resolve the pending request after focus moved. The published model
        // must still split the original anchor and retain its sibling ratio.
        let op = bbt_mux::Ulid(100);
        model.request_spawn(op, plan.spec).unwrap();
        model.ui.window.stage = stage;
        let records: Vec<_> = [2, 3, 5]
            .into_iter()
            .map(|id| {
                let mut record = bbt_proto::SeatRecord::new(
                    bbt_mux::Ulid(id),
                    bbt_mux::Ulid(1),
                    format!("seat {id}"),
                    "zsh",
                );
                if let Some(old) = model.ui.seats.get(&SeatId::from_u128(id)) {
                    record.generation = old.process_generation;
                }
                record
            })
            .collect();
        model.spawned(op, SeatId::from_u128(5));
        let model = crate::model::build(&records, &crate::model::Grids::new(), &model);
        let leaf = |id| SplitNode::Leaf(SeatId::from_u128(id));
        assert_eq!(
            model.ui.window.sidebar.swarms[0].tabs[0].tree,
            SplitNode::split(
                Axis::Row,
                0.37,
                SplitNode::split(Axis::Column, 0.5, leaf(2), leaf(5)),
                leaf(3),
            )
        );
        assert_eq!(
            model.spawns[&op].outcome,
            crate::model::SpawnOutcome::Placed(SeatId::from_u128(5))
        );
    }
    #[test]
    fn explicit_split_never_falls_back_when_the_anchor_is_missing_or_cross_swarm() {
        let model = model();
        let mut stage = model.ui.window.stage.clone();
        stage.focused = None;
        assert!(
            operator_placement(pick(SpawnPlace::Right), &model, &stage, 0)
                .unwrap_err()
                .starts_with("placement_anchor_missing")
        );
        stage = model.ui.window.stage.clone();
        let mut wrong = pick(SpawnPlace::Down);
        wrong.swarm = Some(bbt_mux::Ulid(88));
        assert!(
            operator_placement(wrong, &model, &stage, 0)
                .unwrap_err()
                .starts_with("placement_swarm_changed")
        );
    }
    #[test]
    fn a_new_space_has_a_fresh_swarm_and_first_tab_without_retargeting_the_old_one() {
        let model = model();
        let stage = &model.ui.window.stage;
        let tab = operator_placement(pick(SpawnPlace::Tab), &model, stage, 1).unwrap();
        let space = operator_placement(pick(SpawnPlace::Space), &model, stage, 1).unwrap();
        assert_eq!(tab.spec.swarm, SwarmId::from_u128(1));
        assert_ne!(space.spec.swarm, tab.spec.swarm);
        assert!(matches!(
            space.spec.placement,
            Some(Placement::NewTab { .. })
        ));
    }
}
