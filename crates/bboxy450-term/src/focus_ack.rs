//! A focus command is complete only when its seat is in a completed frame.
//! Older accepted placements settle first; the UI thread never waits for IO.
use crate::model::{Model, SpawnOutcome};
use bbt_mux::Ulid;
use bbt_proto::{SeatId, gui::FocusFrame};
use std::time::Duration;

pub const DEADLINE: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub struct Ask {
    pub id: Ulid,
    pub seat: SeatId,
    /// IO snapshots the requests already accepted before this focus notice.
    pub prior_spawns: Vec<Ulid>,
}
pub struct Answer {
    pub id: Ulid,
    pub result: Result<FocusFrame, String>,
}
pub enum Effect {
    Apply(SeatId),
    Answer(Answer),
}
#[derive(Default)]
pub struct Pending {
    ask: Option<Ask>,
    applied: Option<(u64, u64)>,
    timer: Option<tokio::task::AbortHandle>,
}
impl Drop for Pending {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
    }
}
impl Pending {
    pub fn set(&mut self, ask: Ask) -> Option<Answer> {
        let old = self.fail("focus_superseded: a newer focus request replaced this one");
        self.ask = Some(ask);
        old
    }
    pub fn applied(&mut self, after_frame: u64, generation: u64) {
        self.applied = Some((after_frame, generation));
    }
    pub fn arm(&mut self, timer: tokio::task::AbortHandle) {
        if let Some(old) = self.timer.replace(timer) {
            old.abort();
        }
    }
    pub fn fail(&mut self, reason: &str) -> Option<Answer> {
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
        self.applied = None;
        self.ask.take().map(|ask| Answer {
            id: ask.id,
            result: Err(reason.into()),
        })
    }
    pub fn expire(&mut self, id: Ulid) -> Option<Answer> {
        if self.ask.as_ref().is_some_and(|ask| ask.id == id) {
            self.fail("focus_timeout: no completed frame applied the requested focus")
        } else {
            None
        }
    }
    /// Call after consuming the latest model, and again after a completed frame.
    pub fn poll(
        &mut self,
        model: &Model,
        focused: Option<SeatId>,
        visible: impl Fn(SeatId) -> bool,
        completed_frame: u64,
    ) -> Option<Effect> {
        let ask = self.ask.as_ref()?;
        if model
            .spawns
            .iter()
            .any(|(op, request)| !ask.prior_spawns.contains(op) && !request.outcome.is_result())
        {
            return self
                .fail("focus_superseded: a newer placement was accepted")
                .map(Effect::Answer);
        }
        if ask.prior_spawns.iter().any(|op| {
            model.spawns.get(op).is_some_and(|request| {
                matches!(
                    request.outcome,
                    SpawnOutcome::Pending | SpawnOutcome::AwaitingSeat(_)
                )
            })
        }) {
            return None;
        }
        let seat = model.ui.seats.get(&ask.seat)?;
        if seat.lifecycle != bbt_ui_model::Lifecycle::Open || model.ui.tab_of(ask.seat).is_none() {
            return self
                .fail("not_on_screen: the requested seat has no open pane")
                .map(Effect::Answer);
        }
        let Some((after, generation)) = self.applied else {
            return Some(Effect::Apply(ask.seat));
        };
        if completed_frame < after {
            return None;
        }
        if focused != Some(ask.seat) || !visible(ask.seat) || generation != seat.process_generation
        {
            return self
                .fail("focus_superseded: the requested pane changed before its frame completed")
                .map(Effect::Answer);
        }
        let frame = FocusFrame {
            seat: ask.seat,
            generation,
            frame_seq: completed_frame,
        };
        let id = ask.id;
        self.ask = None;
        self.applied = None;
        if let Some(timer) = self.timer.take() {
            timer.abort();
        }
        Some(Effect::Answer(Answer {
            id,
            result: Ok(frame),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_ui_model as ui;

    fn fixture() -> (Model, SeatId, SeatId) {
        let swarm = ui::SwarmId::from_u128(4);
        let ids = [1, 2].map(SeatId::from_u128);
        let seats = ids
            .iter()
            .map(|id| {
                let mut seat = ui::Seat::new(*id, swarm, [0; 32], "fixture", "zsh");
                seat.process_generation = 7;
                seat
            })
            .collect();
        let ui = ui::Model::new(
            vec![ui::Swarm::new(
                swarm,
                "fixture",
                vec![
                    ui::Tab::new(
                        ui::TabId::from_u128(5),
                        "first",
                        ui::SplitNode::Leaf(ids[0]),
                    ),
                    ui::Tab::new(
                        ui::TabId::from_u128(6),
                        "second",
                        ui::SplitNode::Leaf(ids[1]),
                    ),
                ],
            )],
            seats,
        )
        .unwrap();
        (
            Model {
                ui,
                ..Default::default()
            },
            ids[0],
            ids[1],
        )
    }
    fn accept(model: &mut Model, op: Ulid) {
        let spec = bbt_proto::SpawnSpec {
            swarm: ui::SwarmId::from_u128(4),
            harness_kind: "zsh".into(),
            name: None,
            role: None,
            cwd: "/fixture".into(),
            location: ui::Where::Local,
            resume: None,
            placement: None,
        };
        model.request_spawn(op, spec).unwrap();
    }
    // keeps: spec ctl-cli §1a (produced-frame focus receipt)
    #[test]
    fn an_earlier_placement_settles_before_focus_and_only_a_completed_frame_acknowledges_it() {
        let (mut model, first, second) = fixture();
        let op = Ulid(10);
        accept(&mut model, op);
        model.spawned(op, second);
        let mut pending = Pending::default();
        pending.set(Ask {
            id: Ulid(11),
            seat: first,
            prior_spawns: vec![op],
        });
        assert!(pending.poll(&model, Some(first), |_| true, 99).is_none());
        model.spawns.get_mut(&op).unwrap().outcome = SpawnOutcome::Placed(second);
        model.ui.window.stage.focused = Some(second);
        assert!(
            matches!(pending.poll(&model,Some(second),|_|true,99),Some(Effect::Apply(id)) if id==first)
        );
        pending.applied(100, 7);
        assert!(pending.poll(&model, Some(first), |_| true, 99).is_none());
        let Some(Effect::Answer(answer)) = pending.poll(&model, Some(first), |_| true, 100) else {
            panic!("completed frame must acknowledge focus")
        };
        let frame = answer.result.unwrap();
        assert_eq!(
            (frame.seat, frame.generation, frame.frame_seq),
            (first, 7, 100)
        );
    }
    // keeps: spec ctl-cli §1a (produced-frame focus receipt)
    #[test]
    fn a_newer_placement_or_the_wrong_pane_never_gets_a_success_acknowledgment() {
        let (mut model, first, second) = fixture();
        let mut pending = Pending::default();
        pending.set(Ask {
            id: Ulid(11),
            seat: first,
            prior_spawns: vec![],
        });
        pending.applied(10, 7);
        let Some(Effect::Answer(answer)) = pending.poll(&model, Some(second), |_| false, 10) else {
            panic!()
        };
        assert!(answer.result.unwrap_err().starts_with("focus_superseded:"));
        pending.set(Ask {
            id: Ulid(12),
            seat: first,
            prior_spawns: vec![],
        });
        accept(&mut model, Ulid(13));
        let Some(Effect::Answer(answer)) = pending.poll(&model, Some(first), |_| true, 10) else {
            panic!()
        };
        assert!(answer.result.unwrap_err().starts_with("focus_superseded:"));
    }
    // keeps: spec ctl-cli §1a (produced-frame focus receipt)
    #[test]
    fn an_already_consumed_result_in_an_older_ui_snapshot_does_not_supersede_focus() {
        let (mut model, first, second) = fixture();
        let op = Ulid(10);
        accept(&mut model, op);
        model.spawns.get_mut(&op).unwrap().outcome = SpawnOutcome::Placed(second);
        let mut pending = Pending::default();
        pending.set(Ask {
            id: Ulid(11),
            seat: first,
            prior_spawns: vec![],
        });
        assert!(
            matches!(pending.poll(&model,Some(second),|_|true,9),Some(Effect::Apply(id)) if id==first)
        );
    }
    // keeps: spec ctl-cli §1a (produced-frame focus receipt)
    #[test]
    fn replacing_and_expiring_focus_are_bounded_and_do_not_complete_another_operation() {
        let (_, first, second) = fixture();
        let mut pending = Pending::default();
        pending.set(Ask {
            id: Ulid(11),
            seat: first,
            prior_spawns: vec![],
        });
        let answer = pending
            .set(Ask {
                id: Ulid(12),
                seat: second,
                prior_spawns: vec![],
            })
            .unwrap();
        assert_eq!(answer.id, Ulid(11));
        assert!(answer.result.is_err());
        assert!(pending.expire(Ulid(11)).is_none());
        let answer = pending.expire(Ulid(12)).unwrap();
        assert!(answer.result.unwrap_err().starts_with("focus_timeout:"));
        assert!(pending.expire(Ulid(12)).is_none());
    }
}
