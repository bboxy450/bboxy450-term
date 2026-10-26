//! Watched terminal replicas, owned by the app IO task. No socket or UI access.
use std::{collections::BTreeMap, sync::Arc};

use bbt_mux::{
    store::{Result, invalid},
    streams::Replica,
};
use bbt_proto::{GuiAction, Screen, SeatId, Update, View};

#[derive(Default)]
pub struct Grids {
    watched: BTreeMap<SeatId, View>,
    replicas: BTreeMap<SeatId, Replica>,
    screens: BTreeMap<SeatId, Arc<Screen>>,
}

impl Grids {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply on IO. Publish a new Model before acknowledging the update. None
    /// retains the previously published screen (or ignores an unwatched seat's
    /// in-flight update). Errors never mutate the published screen.
    pub fn apply(&mut self, seat: SeatId, update: &Update) -> Result<Option<Arc<Screen>>> {
        let view = match update {
            Update::Snapshot { snapshot } => snapshot.view,
            Update::Delta { delta } => delta.view,
        };
        if view.seat.0 != seat.0.0 {
            return Err(invalid("update belongs to another seat"));
        }
        let Some(replica) = self.replicas.get_mut(&seat) else {
            return Ok(None);
        };
        replica.apply(update.clone())?;
        let next = &replica.snapshot.as_ref().expect("applied snapshot").screen;
        if self
            .screens
            .get(&seat)
            .is_some_and(|old| old.as_ref() == next)
        {
            return Ok(None);
        }
        let screen = Arc::new(next.clone());
        self.screens.insert(seat, screen.clone());
        Ok(Some(screen))
    }

    /// Diff the visible panes. Subscription changes discard their old baseline;
    /// a rewatch requires Snapshot before Delta. Emit Unwatch before Watch.
    pub fn set_watched(&mut self, panes: &[(SeatId, View)]) -> Vec<GuiAction> {
        let next: BTreeMap<_, _> = panes
            .iter()
            .copied()
            .filter(|(seat, view)| seat.0.0 == view.seat.0)
            .collect();
        let removed: Vec<_> = self
            .watched
            .iter()
            .filter(|(seat, view)| next.get(seat) != Some(view))
            .map(|(seat, _)| *seat)
            .collect();
        let mut actions = Vec::new();
        for seat in removed {
            self.forget(seat);
            actions.push(GuiAction::Unwatch { seat });
        }
        for (seat, view) in next {
            if let std::collections::btree_map::Entry::Vacant(entry) = self.watched.entry(seat) {
                entry.insert(view);
                self.replicas.insert(seat, Replica::default());
                actions.push(GuiAction::Watch { seat, view });
            }
        }
        actions
    }

    pub fn screen(&self, seat: SeatId) -> Option<Arc<Screen>> {
        self.screens.get(&seat).cloned()
    }

    /// Applied model sequence is the capture cursor. History freezes its upper
    /// bound here, rather than chasing output that arrives during replay.
    pub fn baseline(&self, seat: SeatId) -> Option<(View, u64)> {
        let snapshot = self.replicas.get(&seat)?.snapshot.as_ref()?;
        Some((snapshot.view, snapshot.sequence))
    }

    pub fn forget(&mut self, seat: SeatId) {
        self.watched.remove(&seat);
        self.replicas.remove(&seat);
        self.screens.remove(&seat);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_mux::Ulid;
    use bbt_proto::{Delta, Snapshot};

    fn view(seat: SeatId) -> View {
        View {
            seat: Ulid(seat.0.0),
            generation: 1,
            view_generation: Ulid::nil(),
        }
    }
    fn snapshot(seat: SeatId, sequence: u64) -> Update {
        Update::Snapshot {
            snapshot: Box::new(Snapshot {
                view: view(seat),
                sequence,
                screen: Screen {
                    cols: 2,
                    rows: 1,
                    cells: vec![vec![Default::default(); 2]],
                    cursor: (0, 0),
                    modes: 0,
                    palette: serde_json::Value::Null,
                    cursor_style: serde_json::Value::Null,
                    title: String::new(),
                    cwd: None,
                    gap: None,
                },
            }),
        }
    }
    // keeps: spec c1-live-seats §2a (watched replicas and shared screens)
    #[test]
    fn subscription_diff_reuses_unchanged_arcs_and_drops_rewatched_baselines() {
        let mut grids = Grids::new();
        let a = SeatId::from_u128(1);
        let b = SeatId::from_u128(2);
        let av = view(a);
        let bv = view(b);
        assert!(
            matches!(grids.set_watched(&[(a,av)]).as_slice(), [GuiAction::Watch { seat, .. }] if *seat == a)
        );
        let first = grids.apply(a, &snapshot(a, 1)).unwrap().unwrap();
        assert!(grids.apply(a, &snapshot(a, 2)).unwrap().is_none());
        assert!(
            matches!(grids.set_watched(&[(a,av),(b,bv)]).as_slice(), [GuiAction::Watch { seat, .. }] if *seat == b)
        );
        grids.apply(b, &snapshot(b, 1)).unwrap();
        assert!(Arc::ptr_eq(&first, &grids.screen(a).unwrap()));
        assert!(grids.apply(a, &snapshot(b, 2)).is_err());
        assert!(Arc::ptr_eq(&first, &grids.screen(a).unwrap()));
        assert!(
            matches!(grids.set_watched(&[(b,bv)]).as_slice(), [GuiAction::Unwatch { seat }] if *seat == a)
        );
        assert!(grids.screen(a).is_none());
        assert!(grids.apply(a, &snapshot(a, 3)).unwrap().is_none());
        grids.set_watched(&[(a, av), (b, bv)]);
        let delta = Update::Delta {
            delta: Box::new(Delta {
                view: av,
                base: 2,
                sequence: 3,
                rows: vec![],
                cursor: (0, 0),
                modes: 0,
                palette: serde_json::Value::Null,
                cursor_style: serde_json::Value::Null,
                title: String::new(),
                cwd: None,
                gap: None,
            }),
        };
        assert!(grids.apply(a, &delta).is_err());
        grids.apply(a, &snapshot(a, 3)).unwrap();
        let mut replacement = av;
        replacement.view_generation = Ulid::new();
        assert!(matches!(
            grids.set_watched(&[(a, replacement), (b, bv)]).as_slice(),
            [GuiAction::Unwatch { .. }, GuiAction::Watch { .. }]
        ));
        assert!(grids.screen(a).is_none());
    }
}
