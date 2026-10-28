// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Presentation of operation leaves alongside real seats, using one split solver.
use super::*;
use bboxy450_term::starting::{Projection, Slot, Starting};
use bbt_mux::Ulid;

pub(super) struct StartingScene {
    pub projection: Projection,
    pub rows: BTreeMap<Ulid, SeatRow>,
    pub unbound: Vec<(bbt_ui_model::SwarmId, Ulid)>,
}

pub(super) enum PaneKeys {
    Seats(std::vec::IntoIter<SeatId>),
    Operations(std::vec::IntoIter<Slot>),
}
impl Iterator for PaneKeys {
    type Item = Slot;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Seats(rows) => rows.next().map(Slot::Seat),
            Self::Operations(rows) => rows.next(),
        }
    }
}
pub(super) enum PaneRects {
    Seats(std::vec::IntoIter<(SeatId, Rect)>),
    Operations(std::vec::IntoIter<(Slot, Rect)>),
}
impl Iterator for PaneRects {
    type Item = (Slot, Rect);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Seats(rows) => rows.next().map(|(id, rect)| (Slot::Seat(id), rect)),
            Self::Operations(rows) => rows.next(),
        }
    }
}

impl Scene {
    pub fn set_starting(
        &mut self,
        state: &Starting,
        model: &bboxy450_term::model::Model,
        stage: &Stage,
    ) {
        if state.visible().next().is_none() {
            self.starting = None;
            return;
        }
        let projection = state.projection(model, stage);
        if matches!(projection.focused, Some(Slot::Launch(_))) {
            self.focused = None;
            for row in &mut self.seats {
                row.focused = false;
            }
        }
        let mut rows = BTreeMap::new();
        let mut unbound = Vec::new();
        for launch in state.visible() {
            let mut row = SeatRow::new(
                format!("starting/{}", launch.op),
                &launch.name,
                &launch.harness,
            );
            row.title = launch
                .refused
                .as_ref()
                .map_or_else(|| "starting".into(), |reason| format!("refused: {reason}"));
            row.focused = projection.focused == Some(Slot::Launch(launch.op));
            if let Some(index) = launch.seat().and_then(|seat| self.seat_index.get(&seat)) {
                // Only an actually reported seat is aliased here. The row is still
                // keyed by the operation until its first starting frame is produced.
                self.seats[*index] = row.clone();
            } else {
                unbound.push((launch.swarm, launch.op));
            }
            rows.insert(launch.op, row);
        }
        self.starting = Some(StartingScene {
            projection,
            rows,
            unbound,
        });
    }
    pub(super) fn pane_keys(&self) -> PaneKeys {
        if let Some(starting) = &self.starting {
            PaneKeys::Operations(
                starting
                    .projection
                    .tree
                    .as_ref()
                    .map(SplitNode::leaves)
                    .unwrap_or_default()
                    .into_iter(),
            )
        } else {
            PaneKeys::Seats(
                self.tree
                    .as_ref()
                    .map(SplitNode::leaves)
                    .unwrap_or_default()
                    .into_iter(),
            )
        }
    }
    pub(super) fn pane_rects(&self, bounds: Rect, panes: bool) -> PaneRects {
        if let Some(starting) = &self.starting {
            PaneRects::Operations(
                starting
                    .projection
                    .tree
                    .as_ref()
                    .filter(|_| panes)
                    .map(|tree| stage_split(tree, bounds, self.rim).panes)
                    .unwrap_or_default()
                    .into_iter(),
            )
        } else {
            PaneRects::Seats(
                self.tree
                    .as_ref()
                    .filter(|_| panes)
                    .map(|tree| stage_split(tree, bounds, self.rim).panes)
                    .unwrap_or_default()
                    .into_iter(),
            )
        }
    }
    pub(super) fn slot_row(&self, slot: Slot, real: &[SeatRow]) -> SeatRow {
        match slot {
            Slot::Seat(id) => real[self.seat_index[&id]].clone(),
            Slot::Launch(op) => {
                self.starting.as_ref().expect("operation projection").rows[&op].clone()
            }
        }
    }
    pub(super) fn slot_lines(&self, slot: Slot) -> Vec<Vec<ui::modules::pane::Span>> {
        let Slot::Launch(op) = slot else {
            return vec![];
        };
        let row = &self.starting.as_ref().expect("operation projection").rows[&op];
        vec![vec![ui::modules::pane::Span::Text {
            key: format!("starting/{op}/status"),
            value: row.title.clone(),
            foreground: false,
            hover: false,
        }]]
    }
    pub fn starting_bounds(&self, layouts: &[c::Layout]) -> Vec<(Ulid, Rect)> {
        self.pane_keys()
            .zip(layouts.iter().skip(2))
            .filter_map(|(slot, layout)| {
                if let Slot::Launch(op) = slot {
                    pane_body(layout).map(|rect| (op, rect))
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn starting_close_regions(&self, layouts: &[c::Layout]) -> Vec<(Ulid, Rect)> {
        self.pane_keys()
            .zip(layouts.iter().skip(2))
            .filter_map(|(slot, layout)| {
                if let Slot::Launch(op) = slot {
                    named(layout, "pane/strip/close").map(|node| (op, node.rect))
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn starting_stage(
        &self,
    ) -> Option<(
        Option<bbt_ui_model::SwarmId>,
        Option<bbt_ui_model::TabId>,
        Option<Slot>,
    )> {
        self.starting.as_ref().map(|state| {
            (
                state.projection.swarm,
                state.projection.tab,
                state.projection.focused,
            )
        })
    }
    pub(super) fn starting_rows(
        &self,
        swarm: Option<bbt_ui_model::SwarmId>,
        ctx: ui::Context,
    ) -> Vec<c::Element> {
        self.starting.as_ref().map_or_else(Vec::new, |state| {
            state
                .unbound
                .iter()
                .filter(|(id, _)| swarm.is_none_or(|wanted| wanted == *id))
                .map(|(_, op)| state.rows[op].element_in_swarm(ctx))
                .collect()
        })
    }
}
