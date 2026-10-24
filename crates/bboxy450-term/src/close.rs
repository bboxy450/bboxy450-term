//! Window close intents. The registry owns lifecycle; these snapshots retain
//! only the window's exact restoration handle and never perform IO.
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use bbt_mux::Ulid;
use bbt_proto::registry::{Lifecycle, Position, SeatRecord};
use bbt_ui_model::{self as ui, SplitNode, TabId};

pub const GRACE: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
pub struct Intent {
    pub seat: ui::SeatId,
    pub operation: Ulid,
    pub position: Position,
    pub view: ui::ClosedStrip,
    tree: SplitNode,
    seat_order: Vec<ui::SeatId>,
}

impl Intent {
    /// `tree` is the actual visible Stage's full split, including the tiled
    /// tree retained beneath spotlight. No rectangle or path is guessed.
    pub fn prepare(
        model: &ui::Model,
        tab: TabId,
        tree: &SplitNode,
        seat: ui::SeatId,
        revision: u64,
        operation: Ulid,
        now: Instant,
    ) -> Result<Self, ui::EditError> {
        let (si, ti) = model.tab_of(seat).ok_or(ui::EditError::MissingTab)?;
        let swarm = &model.window.sidebar.swarms[si];
        if swarm.tabs[ti].id != tab {
            return Err(ui::EditError::LayoutConflict);
        }
        let mut path = Vec::new();
        let mut ratios = Vec::new();
        if !locate(tree, seat, &mut path, &mut ratios) {
            return Err(ui::EditError::MissingSeat);
        }
        let mut original = swarm.tabs[ti].tree.leaves();
        let mut visible = tree.leaves();
        original.sort();
        visible.sort();
        if original != visible {
            return Err(ui::EditError::LayoutConflict);
        }
        let position = Position {
            swarm: Ulid(swarm.id.0.0),
            tab: Ulid(tab.0.0),
            split_path: path,
            ratios,
            layout_revision: revision,
        };
        // The local focus controller can hold a more recent split ratio than
        // the IO publication. Save that solved tree in the existing undo port.
        let mut solved = model.clone();
        solved.window.sidebar.swarms[si].tabs[ti].tree = tree.clone();
        let next = ui::close(&solved, seat, now, GRACE)?;
        let view = next.window.sidebar.swarms[si]
            .closed
            .iter()
            .find(|closed| closed.seat == seat)
            .cloned()
            .ok_or(ui::EditError::NotClosed)?;
        Ok(Self {
            seat,
            operation,
            position,
            view,
            tree: tree.clone(),
            seat_order: swarm.tabs[ti].seats.clone(),
        })
    }

    /// A queued intent is not a closed seat. Only matching authoritative
    /// metadata supplies the grace deadline, including time spent in a retry.
    pub fn confirm(
        &mut self,
        record: &SeatRecord,
        now: Instant,
        now_ms: u64,
    ) -> Option<ui::ClosedStrip> {
        if record.id.0 != self.seat.0.0
            || record.generation != self.view.undo.process_generation
            || record.lifecycle != Lifecycle::ClosedGrace
        {
            return None;
        }
        let close = record.close.as_ref()?;
        if close.operation != self.operation || close.position != self.position {
            return None;
        }
        let remaining = close.deadline_ms.checked_sub(now_ms)?;
        if remaining == 0 {
            return None;
        }
        self.view.until = now.checked_add(Duration::from_millis(remaining))?;
        Some(self.view.clone())
    }
}

/// IO-owned restoration handles, retained across coalesced model publications.
/// Registry metadata, not a reply's arrival or an animation, owns each transition.
#[derive(Debug, Clone, Default)]
pub struct State(BTreeMap<ui::SeatId, Entry>);
#[derive(Debug, Clone)]
struct Entry {
    intent: Intent,
    confirmed: bool,
}
impl State {
    pub fn needs_reconcile(&self, records: &[SeatRecord]) -> bool {
        self.0.iter().any(|(id, e)| {
            records.iter().find(|r| r.id.0 == id.0.0).is_some_and(|r| {
                r.generation != e.intent.view.undo.process_generation
                    || (!e.confirmed && r.lifecycle != Lifecycle::Open)
                    || (e.confirmed && r.lifecycle != Lifecycle::ClosedGrace)
            })
        })
    }
    pub fn register(&mut self, intent: Intent) -> Result<bool, &'static str> {
        if let Some(old) = self.0.get(&intent.seat) {
            return if old.intent.operation == intent.operation {
                Ok(false)
            } else {
                Err("a close is already pending or in grace")
            };
        }
        if self.0.len() >= 1024 {
            return Err("close restoration limit reached");
        }
        self.0.insert(
            intent.seat,
            Entry {
                intent,
                confirmed: false,
            },
        );
        Ok(true)
    }
    pub fn undo(&self, seat: ui::SeatId, now: Instant) -> Result<(Ulid, u64), &'static str> {
        let entry = self
            .0
            .get(&seat)
            .ok_or("there is no window close to undo")?;
        if !entry.confirmed {
            return Err("close has not been confirmed");
        }
        if now >= entry.intent.view.until {
            return Err("the 60 second undo period has ended");
        }
        Ok((
            entry.intent.operation,
            entry.intent.position.layout_revision,
        ))
    }
    pub fn undo_in(
        &self,
        model: &ui::Model,
        seat: ui::SeatId,
        now: Instant,
    ) -> Result<(Ulid, u64), String> {
        let request = self.undo(seat, now).map_err(str::to_owned)?;
        ui::undo(model, seat, now).map_err(|e| e.to_string())?;
        Ok(request)
    }
    pub fn latest(&self, now: Instant) -> Option<ui::SeatId> {
        self.0
            .iter()
            .filter(|(_, e)| e.confirmed && now < e.intent.view.until)
            .max_by_key(|(_, e)| e.intent.view.until)
            .map(|(id, _)| *id)
    }
    pub fn refused(&mut self, operation: Ulid) {
        self.0
            .retain(|_, e| e.confirmed || e.intent.operation != operation);
    }
    pub fn reconcile(
        &mut self,
        records: &[SeatRecord],
        model: &mut ui::Model,
        now: Instant,
        now_ms: u64,
    ) {
        self.0.retain(|id, entry| {
            let Some(record) = records.iter().find(|record| record.id.0 == id.0.0) else {
                return true;
            };
            if record.generation != entry.intent.view.undo.process_generation {
                for swarm in &mut model.window.sidebar.swarms {
                    swarm.closed.retain(|c| c.seat != *id);
                }
                return false;
            }
            match record.lifecycle {
                Lifecycle::ClosedGrace => {
                    if let Some(view) = entry.intent.confirm(record, now, now_ms) {
                        if !entry.confirmed {
                            // Install the solved split captured by this window, then use
                            // the same close edit as the pure kit and headless host.
                            if let Some((si, ti)) = model.tab_of(*id) {
                                let tab = &mut model.window.sidebar.swarms[si].tabs[ti];
                                let mut current = tab.tree.leaves();
                                let mut captured = entry.intent.tree.leaves();
                                current.sort();
                                captured.sort();
                                if current == captured {
                                    tab.tree = entry.intent.tree.clone();
                                }
                            }
                            if let Ok(closed) = ui::close(
                                model,
                                *id,
                                now,
                                view.until.saturating_duration_since(now),
                            ) {
                                *model = closed;
                            }
                        }
                        if let Some(swarm) = model
                            .window
                            .sidebar
                            .swarms
                            .iter_mut()
                            .find(|s| s.id.0.0 == entry.intent.position.swarm.0)
                        {
                            swarm.closed.retain(|c| c.seat != *id);
                            swarm.closed.push(view);
                        }
                        entry.confirmed = true;
                    } else if entry.confirmed {
                        for swarm in &mut model.window.sidebar.swarms {
                            swarm.closed.retain(|c| c.seat != *id);
                        }
                    }
                    true
                }
                Lifecycle::Open if entry.confirmed => {
                    // The daemon already validated the deadline and generation. The
                    // publication may have been delayed beyond the UI's local clock.
                    for swarm in &mut model.window.sidebar.swarms {
                        if let Some(closed) = swarm.closed.iter_mut().find(|c| c.seat == *id) {
                            closed.until = now + Duration::from_secs(1);
                        }
                    }
                    if let Ok(mut restored) = ui::undo(model, *id, now) {
                        // The shared edit restores the split, but appends the seat to
                        // the list. Keep its original sidebar position as well, while
                        // leaving any other rows added/reordered during grace intact.
                        if let Some((si, ti)) = restored.tab_of(*id) {
                            let seats = &mut restored.window.sidebar.swarms[si].tabs[ti].seats;
                            seats.retain(|seat| seat != id);
                            let old = entry
                                .intent
                                .seat_order
                                .iter()
                                .position(|seat| seat == id)
                                .unwrap_or(seats.len());
                            let at = entry
                                .intent
                                .seat_order
                                .iter()
                                .skip(old + 1)
                                .find_map(|next| seats.iter().position(|seat| seat == next))
                                .unwrap_or(old.min(seats.len()));
                            seats.insert(at, *id);
                        }
                        *model = restored;
                    }
                    false
                }
                Lifecycle::ClosedRetained => {
                    if !record.close.as_ref().is_some_and(|close| {
                        close.operation == entry.intent.operation
                            && close.position == entry.intent.position
                    }) {
                        return false;
                    }
                    if let Some(swarm) = model
                        .window
                        .sidebar
                        .swarms
                        .iter_mut()
                        .find(|s| s.id.0.0 == entry.intent.position.swarm.0)
                    {
                        swarm.closed.retain(|c| c.seat != *id);
                        swarm.stream.push(ui::StreamItem::Presence {
                            kind: ui::PresenceKind::Presence,
                            text: format!("{} closed; its history is kept", record.name),
                            time: format!(
                                "{:02}:{:02}",
                                (now_ms / 3_600_000) % 24,
                                (now_ms / 60_000) % 60
                            ),
                        });
                        if swarm.stream.len() > 512 {
                            swarm.stream.remove(0);
                        }
                    }
                    false
                }
                _ => true,
            }
        });
    }
}

/// Script readback of the exact retained UI projection used by either host.
/// It performs no IO and reports omissions instead of returning an incomplete
/// list as if it were the whole window. Frame capture remains the pixel proof.
pub fn status(model: &crate::model::Model, now: Instant) -> serde_json::Value {
    use serde_json::json;
    let mut slots = vec![];
    let mut grace = vec![];
    let mut notices = vec![];
    let mut omitted = [0usize; 3];
    let mut truncated = 0usize;
    for swarm in &model.ui.window.sidebar.swarms {
        for tab in &swarm.tabs {
            for seat in &tab.seats {
                if slots.len() < 2048 {
                    slots.push(json!({"seat":seat.to_string(),"swarm":swarm.id.to_string(),"tab":tab.id.to_string()}));
                } else {
                    omitted[0] += 1;
                }
            }
        }
        for row in &swarm.closed {
            if now >= row.until {
                continue;
            }
            if grace.len() < 512 {
                let undo = model.ui.seats.get(&row.seat).is_some_and(|s| {
                    matches!(s.lifecycle, ui::Lifecycle::ClosedGrace { .. })
                        && s.process_generation == row.undo.process_generation
                });
                grace.push(json!({"seat":row.seat.to_string(),"swarm":swarm.id.to_string(),"remaining_ms":row.until.saturating_duration_since(now).as_millis() as u64,"undo":undo}));
            } else {
                omitted[1] += 1;
            }
        }
        for item in &swarm.stream {
            if let ui::StreamItem::Presence { text, time, .. } = item {
                if notices.len() < 128 {
                    let clipped = text.chars().count() > 256;
                    truncated += usize::from(clipped);
                    notices.push(json!({"swarm":swarm.id.to_string(),"text":text.chars().take(256).collect::<String>(),"time":time,"truncated":clipped}));
                } else {
                    omitted[2] += 1;
                }
            }
        }
    }
    json!({"slots":slots,"grace":grace,"notices":notices,"omitted":{"slots":omitted[0],"grace":omitted[1],"notices":omitted[2],"truncated_notices":truncated},"complete":omitted==[0;3] && truncated==0})
}

fn locate(
    tree: &SplitNode,
    seat: ui::SeatId,
    path: &mut Vec<usize>,
    ratios: &mut Vec<f64>,
) -> bool {
    match tree {
        SplitNode::Leaf(id) => *id == seat,
        SplitNode::Split { ratio, a, b, .. } => {
            ratios.push(f64::from(*ratio));
            for (index, child) in [(0, a), (1, b)] {
                path.push(index);
                if locate(child, seat, path, ratios) {
                    return true;
                }
                path.pop();
            }
            ratios.pop();
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (ui::Model, SplitNode, ui::SeatId) {
        let swarm = ui::SwarmId::from_u128(4);
        let ids = [1, 2, 3].map(ui::SeatId::from_u128);
        let tree = SplitNode::split(
            ui::Axis::Row,
            0.6,
            SplitNode::Leaf(ids[0]),
            SplitNode::split(
                ui::Axis::Column,
                0.4,
                SplitNode::Leaf(ids[1]),
                SplitNode::Leaf(ids[2]),
            ),
        );
        let seats = ids
            .into_iter()
            .map(|id| {
                let mut seat = ui::Seat::new(id, swarm, [0; 32], "fixture", "codex");
                seat.process_generation = 7;
                seat
            })
            .collect();
        let model = ui::Model::new(
            vec![ui::Swarm::new(
                swarm,
                "fixture",
                vec![ui::Tab::new(ui::TabId::from_u128(5), "main", tree.clone())],
            )],
            seats,
        )
        .unwrap();
        (model, tree, ids[2])
    }

    #[test]
    fn t7_position_and_undo_snapshot_use_the_actual_split_without_optimistic_close() {
        let (model, mut tree, seat) = fixture();
        if let SplitNode::Split { ratio, .. } = &mut tree {
            *ratio = 0.7;
        }
        let intent = Intent::prepare(
            &model,
            ui::TabId::from_u128(5),
            &tree,
            seat,
            11,
            Ulid(6),
            Instant::now(),
        )
        .unwrap();
        assert_eq!(intent.position.swarm, Ulid(4));
        assert_eq!(intent.position.tab, Ulid(5));
        assert_eq!(intent.position.split_path, [1, 1]);
        assert_eq!(
            intent.position.ratios,
            [f64::from(0.7_f32), f64::from(0.4_f32)]
        );
        assert_eq!(intent.position.layout_revision, 11);
        let parent = intent.view.undo.parent.as_ref().unwrap();
        assert_eq!(parent.dir, ui::Axis::Column);
        assert_eq!(parent.side, ui::Branch::B);
        assert_eq!(intent.view.undo.process_generation, 7);
        assert_eq!(model.seats[&seat].lifecycle, ui::Lifecycle::Open);
        assert!(model.window.sidebar.swarms[0].closed.is_empty());
    }

    #[test]
    fn t7_metadata_rebuild_restores_the_same_generation_tree_and_focus() {
        let (ui, tree, seat) = fixture();
        let now = Instant::now();
        let records: Vec<_> = [1, 2, 3]
            .map(|id| {
                let mut r = SeatRecord::new(Ulid(id), Ulid(4), "fixture", "zsh");
                r.generation = 7;
                r
            })
            .into();
        let mut model = crate::model::Model {
            ui,
            metadata_initialized: true,
            ..Default::default()
        };
        let intent = Intent::prepare(
            &model.ui,
            ui::TabId::from_u128(5),
            &tree,
            seat,
            11,
            Ulid(6),
            now,
        )
        .unwrap();
        model.closes.register(intent.clone()).unwrap();
        let grids = crate::grids::Grids::new();
        let open = crate::model::build(&records, &grids, &model);
        assert_eq!(open.ui.window.sidebar.swarms[0].tabs[0].tree, tree);
        let mut closed = records.clone();
        closed[2].lifecycle = Lifecycle::ClosedGrace;
        closed[2].close = Some(bbt_proto::registry::CloseRecord {
            operation: intent.operation,
            revision: 12,
            deadline_ms: bbt_mux::store::now_ms() + 60_000,
            position: intent.position.clone(),
        });
        let hidden = crate::model::build(&closed, &grids, &open);
        assert_eq!(hidden.ui.window.sidebar.swarms[0].closed.len(), 1);
        assert_eq!(
            hidden.ui.window.sidebar.swarms[0].tabs[0]
                .tree
                .leaves()
                .len(),
            2
        );
        assert!(
            !hidden.ui.window.sidebar.swarms[0].tabs[0]
                .tree
                .leaves()
                .contains(&seat)
        );
        assert_eq!(hidden.closes.latest(now), Some(seat));
        let read = status(&hidden, Instant::now());
        assert_eq!(read["complete"], true);
        assert_eq!(read["grace"].as_array().unwrap().len(), 1);
        assert_eq!(read["grace"][0]["seat"], seat.to_string());
        assert_eq!(read["grace"][0]["undo"], true);
        assert!(
            !read["slots"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["seat"] == seat.to_string())
        );
        let refresh = crate::model::build(&closed, &grids, &hidden);
        assert_eq!(refresh.ui.window.sidebar.swarms[0].closed.len(), 1);
        let restored = crate::model::build(&records, &grids, &refresh);
        assert_eq!(restored.ui.window.sidebar.swarms[0].tabs[0].tree, tree);
        assert_eq!(restored.ui.window.stage.focused, Some(seat));
        assert_eq!(restored.ui.seats[&seat].process_generation, 7);
        assert!(restored.ui.window.sidebar.swarms[0].closed.is_empty());
        assert!(restored.closes.latest(now).is_none());
        let read = status(&restored, Instant::now());
        assert!(read["grace"].as_array().unwrap().is_empty());
        assert!(
            read["slots"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["seat"] == seat.to_string())
        );
    }
    #[test]
    fn t7_expiry_removes_only_the_row_and_emits_one_exact_retained_notice() {
        let (mut ui, tree, seat) = fixture();
        let now = Instant::now();
        let intent =
            Intent::prepare(&ui, ui::TabId::from_u128(5), &tree, seat, 11, Ulid(6), now).unwrap();
        let mut record = SeatRecord::new(Ulid(3), Ulid(4), "Tamsin Voss", "codex");
        record.generation = 7;
        record.lifecycle = Lifecycle::ClosedGrace;
        record.close = Some(bbt_proto::registry::CloseRecord {
            operation: intent.operation,
            revision: 12,
            deadline_ms: 61_000,
            position: intent.position.clone(),
        });
        let mut state = State::default();
        state.register(intent).unwrap();
        state.reconcile(&[record.clone()], &mut ui, now, 1000);
        assert!(state.undo(seat, now + GRACE).is_err());
        record.lifecycle = Lifecycle::ClosedRetained;
        state.reconcile(&[record.clone()], &mut ui, now + GRACE, 61_000);
        state.reconcile(&[record], &mut ui, now + GRACE, 61_000);
        assert!(ui.window.sidebar.swarms[0].closed.is_empty());
        assert_eq!(ui.window.sidebar.swarms[0].stream.len(), 1);
        assert!(
            matches!(&ui.window.sidebar.swarms[0].stream[0],ui::StreamItem::Presence {text,..} if text=="Tamsin Voss closed; its history is kept")
        );
        assert!(ui.seats.contains_key(&seat));
        assert!(state.undo(seat, now + GRACE).is_err());
    }

    #[test]
    fn t7_a_delayed_close_publication_still_announces_retention_once() {
        let (ui, tree, seat) = fixture();
        let now = Instant::now();
        let intent =
            Intent::prepare(&ui, ui::TabId::from_u128(5), &tree, seat, 1, Ulid(9), now).unwrap();
        let mut model = crate::model::Model {
            ui,
            metadata_initialized: true,
            ..Default::default()
        };
        model.closes.register(intent.clone()).unwrap();
        let mut records: Vec<_> = (1..=3)
            .map(|id| {
                let mut r = SeatRecord::new(Ulid(id), Ulid(4), "fixture", "zsh");
                r.generation = 7;
                r
            })
            .collect();
        records[2].lifecycle = Lifecycle::ClosedRetained;
        records[2].close = Some(bbt_proto::registry::CloseRecord {
            operation: intent.operation,
            revision: 1,
            deadline_ms: 1,
            position: intent.position,
        });
        let grids = crate::grids::Grids::new();
        let mut unrelated = records.clone();
        unrelated[2].close.as_mut().unwrap().operation = Ulid(10);
        let foreign = crate::model::build(&unrelated, &grids, &model);
        assert!(
            status(&foreign, now)["notices"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let closed = crate::model::build(&records, &grids, &model);
        let again = crate::model::build(&records, &grids, &closed);
        assert!(again.ui.tab_of(seat).is_none());
        assert!(again.closes.latest(now).is_none());
        let read = status(&again, now);
        assert!(read["grace"].as_array().unwrap().is_empty());
        assert_eq!(read["notices"].as_array().unwrap().len(), 1);
        assert_eq!(
            read["notices"][0]["text"],
            "fixture closed; its history is kept"
        );
    }

    #[test]
    fn t7_grace_uses_only_the_matching_registry_deadline_and_generation() {
        let (model, tree, seat) = fixture();
        let now = Instant::now();
        let mut intent = Intent::prepare(
            &model,
            ui::TabId::from_u128(5),
            &tree,
            seat,
            11,
            Ulid(6),
            now,
        )
        .unwrap();
        let mut record = SeatRecord::new(Ulid(3), Ulid(4), "fixture", "codex");
        record.generation = 7;
        assert!(intent.confirm(&record, now, 1000).is_none());
        record.lifecycle = Lifecycle::ClosedGrace;
        record.close = Some(bbt_proto::registry::CloseRecord {
            operation: intent.operation,
            revision: 12,
            deadline_ms: 61_000,
            position: intent.position.clone(),
        });
        let later = now + Duration::from_secs(5);
        assert_eq!(
            intent.confirm(&record, later, 6000).unwrap().until,
            now + GRACE
        );
        record.generation += 1;
        assert!(intent.confirm(&record, later, 6000).is_none());
        record.generation -= 1;
        assert!(intent.confirm(&record, now + GRACE, 61_000).is_none());
    }
}
