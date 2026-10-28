// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Lifecycle targets for the shared UI frame receipt owner; no scheduler or IO.
use crate::{close::Intent, model::Model};
use bbt_mux::Ulid;
use bbt_proto::SeatId;
use bbt_ui_model::Lifecycle;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Target {
    seat: SeatId,
    generation: u64,
    close_id: Ulid,
    undo: bool,
}
impl Target {
    pub fn close(intent: &Intent) -> Self {
        Self {
            seat: intent.seat,
            generation: intent.view.undo.process_generation,
            close_id: intent.operation,
            undo: false,
        }
    }
    pub fn undo(model: &Model, seat: SeatId, now: Instant) -> Result<Self, String> {
        let record = model
            .ui
            .seats
            .get(&seat)
            .ok_or("no such seat in the window")?;
        let (close_id, _) = model.closes.undo(seat, now).map_err(str::to_owned)?;
        if !matches!(record.lifecycle, Lifecycle::ClosedGrace { .. }) {
            return Err("the window has no grace state to undo".into());
        }
        Ok(Self {
            seat,
            generation: record.process_generation,
            close_id,
            undo: true,
        })
    }
    /// A queued intent or a layout-only edit is not an authoritative close.
    pub fn ready(&self, model: &Model) -> Result<bool, String> {
        let record = model
            .ui
            .seats
            .get(&self.seat)
            .filter(|record| record.process_generation == self.generation)
            .ok_or("close_superseded: the requested seat generation changed")?;
        Ok(if self.undo {
            record.lifecycle == Lifecycle::Open
                && model.ui.tab_of(self.seat).is_some()
                && model.closes.confirmed_close(self.seat).is_none()
        } else {
            matches!(record.lifecycle, Lifecycle::ClosedGrace { .. })
                && model.ui.tab_of(self.seat).is_none()
                && model.closes.confirmed_close(self.seat) == Some((self.close_id, self.generation))
        })
    }
    /// Checked against the produced layout by C3's existing receipt path.
    pub fn shown(&self, focused: Option<SeatId>, visible: impl Fn(SeatId) -> bool) -> bool {
        if self.undo {
            focused == Some(self.seat) && visible(self.seat)
        } else {
            !visible(self.seat)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_proto::registry::{CloseRecord, Lifecycle as RegistryLifecycle, SeatRecord};
    use bbt_ui_model as ui;

    fn fixture() -> (Model, Model, Model, Intent, Instant) {
        let now = Instant::now();
        let seat = SeatId::from_u128(2);
        let op = Ulid(9);
        let tree = ui::SplitNode::split(
            ui::Axis::Row,
            0.6,
            ui::SplitNode::Leaf(SeatId::from_u128(1)),
            ui::SplitNode::Leaf(seat),
        );
        let seats = [1, 2]
            .map(|id| {
                let mut s = ui::Seat::new(
                    SeatId::from_u128(id),
                    ui::SwarmId::from_u128(4),
                    [0; 32],
                    "fixture",
                    "zsh",
                );
                s.process_generation = 7;
                s
            })
            .into();
        let model = ui::Model::new(
            vec![ui::Swarm::new(
                ui::SwarmId::from_u128(4),
                "fixture",
                vec![ui::Tab::new(ui::TabId::from_u128(5), "tab", tree.clone())],
            )],
            seats,
        )
        .unwrap();
        let mut open = Model {
            ui: model,
            metadata_initialized: true,
            ..Default::default()
        };
        let intent =
            Intent::prepare(&open.ui, ui::TabId::from_u128(5), &tree, seat, 11, op, now).unwrap();
        open.closes.register(intent.clone()).unwrap();
        let records: Vec<_> = [1, 2]
            .map(|id| {
                let mut r = SeatRecord::new(Ulid(id), Ulid(4), "fixture", "zsh");
                r.generation = 7;
                r
            })
            .into();
        let mut closed = records.clone();
        closed[1].lifecycle = RegistryLifecycle::ClosedGrace;
        closed[1].close = Some(CloseRecord {
            operation: op,
            revision: 12,
            deadline_ms: 61_000,
            position: intent.position.clone(),
        });
        let grids = crate::grids::Grids::new();
        let hidden = crate::model::build_at(&closed, &grids, &open, now, 1000);
        let restored = crate::model::build_at(&records, &grids, &hidden, now, 1000);
        (open, hidden, restored, intent, now)
    }
    // keeps: spec ctl-cli §1a (produced-frame close receipt)
    #[test]
    fn close_needs_the_confirmed_operation_not_just_a_model_or_layout_edit() {
        let (open, hidden, _, intent, now) = fixture();
        let target = Target::close(&intent);
        assert!(!target.ready(&open).unwrap());
        let mut layout_only = open.clone();
        layout_only.ui = ui::close(&open.ui, intent.seat, now, crate::close::GRACE).unwrap();
        assert!(!target.ready(&layout_only).unwrap());
        assert!(target.ready(&hidden).unwrap());
        assert!(!target.shown(None, |_| true));
        assert!(target.shown(None, |_| false));
        let mut wrong = intent.clone();
        wrong.operation = Ulid(99);
        assert!(!Target::close(&wrong).ready(&hidden).unwrap());
    }
    // keeps: spec ctl-cli §1a (produced-frame close receipt)
    #[test]
    fn undo_needs_the_same_generation_open_and_shown_in_its_restored_pane() {
        let (open, hidden, restored, intent, now) = fixture();
        assert!(Target::undo(&open, intent.seat, now).is_err());
        let target = Target::undo(&hidden, intent.seat, now).unwrap();
        assert!(!target.ready(&hidden).unwrap());
        assert!(target.ready(&restored).unwrap());
        assert!(!target.shown(None, |_| true));
        assert!(!target.shown(Some(intent.seat), |_| false));
        assert!(target.shown(Some(intent.seat), |_| true));
        let mut replaced = restored.clone();
        replaced
            .ui
            .seats
            .get_mut(&intent.seat)
            .unwrap()
            .process_generation += 1;
        assert!(
            target
                .ready(&replaced)
                .unwrap_err()
                .starts_with("close_superseded:")
        );
    }
}
