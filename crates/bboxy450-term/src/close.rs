//! Window close intents. The registry owns lifecycle; these snapshots retain
//! only the window's exact restoration handle and never perform IO.
use std::time::{Duration, Instant};

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
