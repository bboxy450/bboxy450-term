// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Window-owned launch presentation. Operations are never provisional seats.
use crate::model::{Model, PENDING_SPAWNS, SpawnOutcome};
use bbt_mux::Ulid;
use bbt_proto::{
    SeatId, SwarmId, TabId,
    gui::{UiStartingFrame, UiStartingState},
    seat_state::{Placement, Side},
};
use bbt_ui_model::{SplitNode, Stage};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Slot {
    Seat(SeatId),
    Launch(Ulid),
}

#[derive(Debug, Clone)]
pub struct Launch {
    pub op: Ulid,
    pub name: String,
    pub harness: String,
    pub swarm: SwarmId,
    pub placement: Placement,
    pub layout_revision: u64,
    pub admitted_frame: u64,
    pub first: Option<UiStartingFrame>,
    pub refused: Option<String>,
    pub dismissed: bool,
    result: Option<SeatId>,
    ambiguous: bool,
    ordinal: u64,
}
impl Launch {
    pub fn seat(&self) -> Option<SeatId> {
        self.result
    }
    pub fn tab(&self) -> TabId {
        match self.placement {
            Placement::NewTab { tab } | Placement::Split { tab, .. } => tab,
        }
    }
    pub fn state(&self) -> UiStartingState {
        self.refused
            .as_ref()
            .map_or(UiStartingState::Starting, |reason| {
                UiStartingState::Refused {
                    reason: reason.clone(),
                }
            })
    }
}

#[derive(Default)]
pub struct Starting {
    entries: BTreeMap<Ulid, Launch>,
    completed: VecDeque<Finished>,
    ordinal: u64,
    focused: Option<Ulid>,
}
struct Finished {
    op: Ulid,
    seat: SeatId,
    first: Option<UiStartingFrame>,
    focus_allowed: bool,
    ordinal: u64,
}

pub struct Projection {
    pub swarm: Option<SwarmId>,
    pub tab: Option<TabId>,
    pub focused: Option<Slot>,
    pub tree: Option<SplitNode<Slot>>,
}

pub struct Intent {
    pub op: Ulid,
    pub harness: String,
    pub name: Option<String>,
    pub swarm: Option<SwarmId>,
    pub placement: Placement,
    pub layout_revision: u64,
    pub admitted_frame: u64,
}

fn names<'a>(occupied: impl Iterator<Item = &'a str>) -> String {
    use bbt_proto::seat_names::POOL;
    let occupied: BTreeSet<_> = occupied.collect();
    for tier in 0..=occupied.len() / POOL.len() {
        for base in POOL {
            let name = if tier == 0 {
                base.to_owned()
            } else {
                format!("{base} {}", tier + 1)
            };
            if !occupied.contains(name.as_str()) {
                return name;
            }
        }
    }
    unreachable!("one more name tier than the finite occupied set can fill")
}

impl Starting {
    pub fn prepare(
        &self,
        intent: Intent,
        model: &Model,
        stage: &Stage,
    ) -> Result<Launch, &'static str> {
        let Intent {
            op,
            harness,
            name,
            swarm,
            placement,
            layout_revision,
            admitted_frame,
        } = intent;
        if self.entries.contains_key(&op) || model.spawns.contains_key(&op) {
            return Err("spawn_id_conflict");
        }
        let retained: BTreeSet<_> = self
            .entries
            .keys()
            .chain(model.spawns.keys())
            .copied()
            .collect();
        if retained.len() >= PENDING_SPAWNS {
            return Err("spawn_queue_full");
        }
        let swarm = swarm
            .or(stage.swarm)
            .or_else(|| model.ui.window.sidebar.swarms.first().map(|swarm| swarm.id))
            .unwrap_or_else(|| SwarmId::from_u128(Ulid::new().0));
        if let Placement::Split {
            tab,
            target,
            target_generation,
            ..
        } = placement
        {
            let seat = model
                .ui
                .seats
                .get(&target)
                .ok_or("placement_anchor_missing")?;
            if seat.process_generation != target_generation {
                return Err("placement_anchor_changed");
            }
            let (si, ti) = model.ui.tab_of(target).ok_or("placement_anchor_missing")?;
            let owner = &model.ui.window.sidebar.swarms[si];
            if owner.id != swarm || owner.tabs[ti].id != tab {
                return Err("placement_anchor_missing");
            }
        }
        let name = name.unwrap_or_else(|| {
            names(
                model
                    .ui
                    .seats
                    .values()
                    .filter(|seat| seat.swarm == swarm)
                    .map(|seat| seat.name.as_str())
                    .chain(
                        self.entries
                            .values()
                            .filter(|entry| entry.swarm == swarm)
                            .map(|entry| entry.name.as_str()),
                    ),
            )
        });
        Ok(Launch {
            op,
            name,
            harness,
            swarm,
            placement,
            layout_revision,
            admitted_frame,
            first: None,
            refused: None,
            dismissed: false,
            result: None,
            ambiguous: true,
            ordinal: self.ordinal,
        })
    }

    /// Call only after the UI's existing bounded command queue accepts this intent.
    pub fn admit(&mut self, launch: Launch) {
        self.ordinal = self.ordinal.saturating_add(1);
        self.focused = Some(launch.op);
        self.entries.insert(launch.op, launch);
    }
    pub fn get(&self, op: Ulid) -> Option<&Launch> {
        self.entries.get(&op)
    }
    pub fn focused(&self) -> Option<Ulid> {
        self.focused
            .filter(|op| self.entries.get(op).is_some_and(|entry| !entry.dismissed))
    }
    pub fn visible(&self) -> impl Iterator<Item = &Launch> {
        self.entries.values().filter(|entry| !entry.dismissed)
    }
    pub fn first(&self, op: Ulid) -> Option<&UiStartingFrame> {
        self.entries
            .get(&op)
            .and_then(|entry| entry.first.as_ref())
            .or_else(|| {
                self.completed
                    .iter()
                    .find(|done| done.op == op)
                    .and_then(|done| done.first.as_ref())
            })
    }
    pub fn completed_seat(&self, op: Ulid) -> Option<SeatId> {
        self.completed
            .iter()
            .find(|done| done.op == op)
            .map(|done| done.seat)
    }
    pub fn should_focus(&self, op: Ulid) -> bool {
        if self.entries.contains_key(&op) {
            return self.focused == Some(op);
        }
        self.completed
            .iter()
            .find(|done| done.op == op)
            .is_none_or(|done| done.focus_allowed && done.ordinal.saturating_add(1) == self.ordinal)
    }
    pub fn release_focus(&mut self) {
        self.focused = None;
    }
    pub fn expire(&mut self, op: Ulid) -> bool {
        let Some(entry) = self.entries.get_mut(&op) else {
            return false;
        };
        if entry.result.is_some() || entry.refused.is_some() {
            return false;
        }
        entry.refused = Some(
            "spawn_timeout: no reply before the launch deadline; outcome remains unknown".into(),
        );
        true
    }
    pub fn dismiss(&mut self, op: Ulid) -> bool {
        let Some(entry) = self.entries.get_mut(&op) else {
            return false;
        };
        if entry.refused.is_none() {
            return false;
        }
        entry.dismissed = true;
        let remove = !entry.ambiguous;
        if self.focused == Some(op) {
            self.focused = None;
        }
        if remove {
            self.entries.remove(&op);
        }
        true
    }

    /// Reconcile only correlated outcomes. No absence or unrelated metadata is success.
    pub fn observe(&mut self, model: &Model) -> bool {
        let mut changed = false;
        for (op, entry) in &mut self.entries {
            let Some(request) = model.spawns.get(op) else {
                continue;
            };
            match &request.outcome {
                SpawnOutcome::Placed(seat) if model.ui.tab_of(*seat).is_some() => {
                    changed |= entry.result != Some(*seat);
                    entry.result = Some(*seat);
                    entry.refused = None;
                }
                SpawnOutcome::Rejected(reason) | SpawnOutcome::Partial { reason, .. } => {
                    changed |= entry.refused.as_ref() != Some(reason);
                    entry.refused = Some(reason.clone());
                    entry.ambiguous = false;
                }
                SpawnOutcome::Held(reason) | SpawnOutcome::Unknown(reason) => {
                    changed |= entry.refused.as_ref() != Some(reason);
                    entry.refused = Some(reason.clone());
                    entry.ambiguous = true;
                }
                _ => {}
            }
        }
        let ready: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(op, entry)| {
                entry
                    .result
                    .filter(|_| {
                        entry.first.is_some() || entry.dismissed || self.focused != Some(*op)
                    })
                    .map(|seat| (*op, seat))
            })
            .collect();
        for (op, seat) in ready {
            let entry = self.entries.remove(&op).expect("collected launch");
            let focus_allowed = self.focused == Some(op) && !entry.dismissed;
            if self.focused == Some(op) {
                self.focused = None;
            }
            self.completed.push_back(Finished {
                op,
                seat,
                first: entry.first,
                focus_allowed,
                ordinal: entry.ordinal,
            });
            while self.completed.len() > PENDING_SPAWNS {
                self.completed.pop_front();
            }
            changed = true;
        }
        changed
    }

    /// These bounds belong to an actually completed frame, not the next-frame target.
    pub fn presented(&mut self, frame: u64, panes: &[(Ulid, [f32; 4])]) -> bool {
        let mut changed = false;
        for (op, rect) in panes {
            let Some(entry) = self.entries.get_mut(op) else {
                continue;
            };
            if !entry.dismissed && entry.first.is_none() && frame > entry.admitted_frame {
                entry.first = Some(UiStartingFrame {
                    admitted_frame_seq: entry.admitted_frame,
                    frame_seq: frame,
                    rect: *rect,
                    name: entry.name.clone(),
                });
                changed = true;
            }
        }
        changed
    }

    pub fn projection(&self, model: &Model, stage: &Stage) -> Projection {
        fn convert(tree: &SplitNode) -> SplitNode<Slot> {
            match tree {
                SplitNode::Leaf(seat) => SplitNode::Leaf(Slot::Seat(*seat)),
                SplitNode::Split { dir, ratio, a, b } => {
                    SplitNode::split(*dir, *ratio, convert(a), convert(b))
                }
            }
        }
        fn replace(tree: &mut SplitNode<Slot>, target: Slot, next: &SplitNode<Slot>) -> bool {
            match tree {
                SplitNode::Leaf(found) if *found == target => {
                    *tree = next.clone();
                    true
                }
                SplitNode::Split { a, b, .. } => {
                    replace(a, target, next) || replace(b, target, next)
                }
                _ => false,
            }
        }
        let selected = self
            .focused
            .and_then(|op| self.entries.get(&op))
            .filter(|entry| !entry.dismissed);
        let swarm = selected.map(|entry| entry.swarm).or(stage.swarm);
        let tab = selected.map(Launch::tab).or(stage.tab);
        let mut tree = model
            .ui
            .window
            .sidebar
            .swarms
            .iter()
            .filter(|owner| Some(owner.id) == swarm)
            .flat_map(|owner| &owner.tabs)
            .find(|row| Some(row.id) == tab)
            .map(|row| convert(&row.tree));
        let mut entries: Vec<_> = self
            .visible()
            .filter(|entry| Some(entry.swarm) == swarm && Some(entry.tab()) == tab)
            .collect();
        entries.sort_by_key(|entry| entry.ordinal);
        for entry in entries {
            let leaf = SplitNode::Leaf(Slot::Launch(entry.op));
            if let (Some(tree), Some(seat)) = (&mut tree, entry.result)
                && replace(tree, Slot::Seat(seat), &leaf)
            {
                continue;
            }
            match entry.placement {
                Placement::NewTab { .. } => tree = Some(leaf),
                Placement::Split { target, side, .. } => {
                    if let Some(tree) = &mut tree {
                        let next = SplitNode::split(
                            match side {
                                Side::Right => bbt_ui_model::Axis::Row,
                                Side::Bottom => bbt_ui_model::Axis::Column,
                            },
                            0.5,
                            SplitNode::Leaf(Slot::Seat(target)),
                            leaf,
                        );
                        replace(tree, Slot::Seat(target), &next);
                    }
                }
            }
        }
        Projection {
            swarm,
            tab,
            focused: selected
                .map(|entry| Slot::Launch(entry.op))
                .or_else(|| stage.focused.map(Slot::Seat)),
            tree,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_ui_model as ui;

    fn fixture() -> Model {
        let swarm = SwarmId::from_u128(1);
        Model {
            ui: ui::Model::new(
                vec![ui::Swarm::new(
                    swarm,
                    "fixture",
                    vec![ui::Tab::new(
                        TabId::from_u128(2),
                        "seats",
                        SplitNode::split(
                            ui::Axis::Row,
                            0.7,
                            SplitNode::Leaf(SeatId::from_u128(10)),
                            SplitNode::Leaf(SeatId::from_u128(20)),
                        ),
                    )],
                )],
                vec![
                    ui::Seat::new(
                        SeatId::from_u128(10),
                        swarm,
                        [1; 32],
                        "Vesper Greywind",
                        "zsh",
                    ),
                    ui::Seat::new(SeatId::from_u128(20), swarm, [2; 32], "other", "zsh"),
                ],
            )
            .unwrap(),
            ..Default::default()
        }
    }
    fn intent(op: u128) -> Intent {
        Intent {
            op: Ulid(op),
            harness: "claude".into(),
            name: None,
            swarm: Some(SwarmId::from_u128(1)),
            placement: Placement::Split {
                tab: TabId::from_u128(2),
                target: SeatId::from_u128(10),
                target_generation: 1,
                side: Side::Right,
            },
            layout_revision: 4,
            admitted_frame: 42,
        }
    }
    fn bounds(projection: &Projection) -> BTreeMap<Slot, [f32; 4]> {
        ui::layout::stage_split(
            projection.tree.as_ref().unwrap(),
            ui::layout::Rect::new(300., 40., 900., 600.),
            4.,
        )
        .panes
        .into_iter()
        .map(|(id, rect)| (id, [rect.x, rect.y, rect.w, rect.h]))
        .collect()
    }

    // keeps: decision #340 (first-frame operation presentation, same-slot real-seat transition)
    #[test]
    fn an_immediate_backend_reply_still_renders_the_starting_operation_before_swapping_in_place() {
        let mut model = fixture();
        let before = model.ui.clone();
        let stage = Stage {
            swarm: Some(SwarmId::from_u128(1)),
            tab: Some(TabId::from_u128(2)),
            focused: Some(SeatId::from_u128(10)),
        };
        let mut state = Starting::default();
        let launch = state.prepare(intent(100), &model, &stage).unwrap();
        let name = launch.name.clone();
        let placement = launch.placement.clone();
        state.admit(launch);
        assert_eq!(
            model.ui, before,
            "admission must create no seat or authoritative layout"
        );
        let starting = bounds(&state.projection(&model, &stage));
        assert!(starting.contains_key(&Slot::Launch(Ulid(100))));
        model.ui.seats.insert(
            SeatId::from_u128(30),
            ui::Seat::new(
                SeatId::from_u128(30),
                SwarmId::from_u128(1),
                [3; 32],
                name,
                "claude",
            ),
        );
        model.ui = ui::place(
            &model.ui,
            SeatId::from_u128(30),
            ui::Placement::Split {
                target: SeatId::from_u128(10),
                side: ui::Side::Right,
            },
        )
        .unwrap();
        model
            .request_spawn(
                Ulid(100),
                bbt_proto::SpawnSpec {
                    via: None,
                    args: Vec::new(),
                    env: Vec::new(),
                    command: None,
                    swarm: SwarmId::from_u128(1),
                    harness_kind: "claude".into(),
                    name: None,
                    role: None,
                    cwd: "/fixture".into(),
                    location: ui::Where::Local,
                    resume: None,
                    placement: Some(placement),
                },
            )
            .unwrap();
        model.spawns.get_mut(&Ulid(100)).unwrap().outcome =
            SpawnOutcome::Placed(SeatId::from_u128(30));
        state.observe(&model);
        assert_eq!(
            bounds(&state.projection(&model, &stage)),
            starting,
            "a fast backend cannot erase the required first starting frame"
        );
        assert!(
            !state.presented(42, &[(Ulid(100), starting[&Slot::Launch(Ulid(100))])]),
            "an older in-flight frame cannot witness this operation"
        );
        assert!(state.presented(43, &[(Ulid(100), starting[&Slot::Launch(Ulid(100))])]));
        state.observe(&model);
        let mut real_stage = stage.clone();
        real_stage.focused = Some(SeatId::from_u128(30));
        let actual = bounds(&state.projection(&model, &real_stage));
        assert_eq!(
            actual[&Slot::Seat(SeatId::from_u128(30))],
            starting[&Slot::Launch(Ulid(100))]
        );
        assert_eq!(state.first(Ulid(100)).unwrap().frame_seq, 43);
        assert_eq!(state.completed_seat(Ulid(100)), Some(SeatId::from_u128(30)));
        assert_eq!(state.visible().count(), 0);
    }

    // keeps: decision #340 (named timeout, dismissal, no uncorrelated success)
    #[test]
    fn missing_and_unrelated_outcomes_never_resolve_the_operation_and_timeout_is_dismissible() {
        let model = fixture();
        let stage = model.ui.window.stage.clone();
        let mut state = Starting::default();
        state.admit(state.prepare(intent(100), &model, &stage).unwrap());
        assert!(!state.observe(&model));
        assert!(!state.presented(43, &[(Ulid(200), [0., 0., 1., 1.])]));
        assert!(state.first(Ulid(100)).is_none());
        assert!(state.expire(Ulid(100)));
        assert!(matches!(state.get(Ulid(100)).unwrap().state(),
            UiStartingState::Refused { reason } if reason.starts_with("spawn_timeout:")));
        assert_eq!(state.visible().count(), 1);
        assert!(state.dismiss(Ulid(100)));
        assert_eq!(state.visible().count(), 0);
        assert!(model.spawns.is_empty());
        assert_eq!(model.ui.seats.len(), 2);
    }

    // keeps: decision #340 (shared name pool, bounded presentation admission)
    #[test]
    fn admitted_names_are_distinct_and_a_full_window_refuses_before_any_backend_work() {
        let model = fixture();
        let stage = model.ui.window.stage.clone();
        let mut state = Starting::default();
        let mut names = BTreeSet::new();
        for n in 0..PENDING_SPAWNS {
            let launch = state
                .prepare(intent(100 + n as u128), &model, &stage)
                .unwrap();
            assert!(names.insert(launch.name.clone()));
            assert_ne!(launch.name, "Vesper Greywind");
            state.admit(launch);
        }
        assert_eq!(
            state.prepare(intent(1000), &model, &stage).unwrap_err(),
            "spawn_queue_full"
        );
        assert!(model.spawns.is_empty());
    }
}
