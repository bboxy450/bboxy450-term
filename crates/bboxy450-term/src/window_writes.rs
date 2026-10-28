// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! IO-owned coalescing and durability fences; neither the render thread nor inputs wait.
use bbt_mux::Ulid;
use bbt_proto::{
    GuiAction, GuiReply,
    gui::{WindowSnapshot, WindowState},
};
use std::collections::VecDeque;

pub struct Writes {
    epoch: Option<Ulid>,
    desired: Option<(u64, WindowState)>,
    saved: u64,
    flight: Option<(Ulid, u64)>,
    refused: Option<(u64, String)>,
    replies: VecDeque<(u64, GuiAction)>,
}
impl Writes {
    pub fn new(snapshot: Option<&WindowSnapshot>) -> Self {
        Self {
            epoch: snapshot.map(|s| s.epoch),
            desired: snapshot.and_then(|s| s.state.clone()).map(|s| (0, s)),
            saved: 0,
            flight: None,
            refused: None,
            replies: VecDeque::new(),
        }
    }
    pub fn observe(&mut self, state: WindowState) {
        if self.desired.as_ref().is_some_and(|(_, old)| *old == state) {
            return;
        }
        let version = self.desired.as_ref().map_or(1, |(v, _)| v + 1);
        self.desired = Some((version, state));
        self.refused = None;
    }
    pub fn next(&mut self, snapshot: Option<&WindowSnapshot>) -> Option<GuiAction> {
        let epoch = snapshot?.epoch;
        if self.epoch != Some(epoch) {
            self.epoch = Some(epoch);
            self.saved = 0;
            self.flight = None;
            self.refused = None;
        }
        let (version, state) = self.desired.as_ref()?;
        if self.flight.is_some() || *version <= self.saved || self.refused.is_some() {
            return None;
        }
        let id = Ulid::new();
        self.flight = Some((id, *version));
        Some(GuiAction::WindowSave {
            id,
            epoch,
            state: state.clone(),
        })
    }
    pub fn fence(&mut self, action: GuiAction) -> Option<GuiAction> {
        let version = self.desired.as_ref().map_or(0, |(v, _)| *v);
        if self.epoch.is_none() || version <= self.saved {
            return Some(action);
        }
        if let Some((_, reason)) = &self.refused {
            return Some(refuse(action, reason));
        }
        if self.replies.len() == 64 {
            return Some(refuse(action, "window_state_reply_capacity"));
        }
        self.replies.push_back((version, action));
        None
    }
    pub fn completion(&mut self, reply: &GuiReply) -> Option<Vec<GuiAction>> {
        let GuiReply::WindowSaved { op, error } = reply else {
            return None;
        };
        let Some((id, version)) = self.flight else {
            return Some(vec![]);
        };
        if id != *op {
            return Some(vec![]);
        }
        self.flight = None;
        let mut ready = Vec::new();
        if let Some(reason) = error {
            self.refused = Some((version, reason.clone()));
            while let Some((_, action)) = self.replies.pop_front() {
                ready.push(refuse(action, reason));
            }
        } else {
            self.saved = version;
            while self.replies.front().is_some_and(|(v, _)| *v <= self.saved) {
                ready.push(self.replies.pop_front().unwrap().1);
            }
        }
        Some(ready)
    }
}
fn refuse(action: GuiAction, reason: &str) -> GuiAction {
    let reason = format!("window_state_not_durable: {reason}");
    match action {
        GuiAction::UiReply { id, .. } => GuiAction::UiReply {
            id,
            result: Box::new(Err(reason)),
        },
        GuiAction::Focused { id, .. } => GuiAction::Focused {
            id,
            result: Err(reason),
        },
        _ => action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state(width: f32) -> WindowState {
        serde_json::from_value(serde_json::json!({"version":1,"size":[width,720.0],"sidebar_width":300.0,"sidebar_scroll":0.0,"sidebar_shown":true,"open_swarms":[],"swarms":[],"stage":{"swarm":null,"tab":null,"focused":null},"scroll":[]})).unwrap()
    }
    // keeps: decision #344; a frame reply cannot precede its durable state
    #[test]
    fn frame_replies_wait_for_their_save_and_newer_frames_coalesce() {
        let snapshot = WindowSnapshot {
            epoch: Ulid::new(),
            state: None,
        };
        let mut writes = Writes::new(Some(&snapshot));
        writes.observe(state(1280.));
        let GuiAction::WindowSave { id, .. } = writes.next(Some(&snapshot)).unwrap() else {
            panic!("save missing")
        };
        let reply = Ulid::new();
        assert!(
            writes
                .fence(GuiAction::Focused {
                    id: reply,
                    result: Err("fixture".into())
                })
                .is_none()
        );
        writes.observe(state(1400.));
        writes.observe(state(1600.));
        assert!(writes.next(Some(&snapshot)).is_none());
        let ready = writes
            .completion(&GuiReply::WindowSaved {
                op: id,
                error: None,
            })
            .unwrap();
        assert!(matches!(&ready[..],[GuiAction::Focused {id,..}] if *id==reply));
        let GuiAction::WindowSave { state, .. } = writes.next(Some(&snapshot)).unwrap() else {
            panic!("newest save missing")
        };
        assert_eq!(state.size[0], 1600.);
    }
    // keeps: decision #344; old completion cannot acknowledge the new GUI's frame
    #[test]
    fn an_old_epoch_completion_cannot_release_a_new_gui_reply() {
        let old = WindowSnapshot {
            epoch: Ulid::new(),
            state: None,
        };
        let mut writes = Writes::new(Some(&old));
        writes.observe(state(1280.));
        let GuiAction::WindowSave { id: first, .. } = writes.next(Some(&old)).unwrap() else {
            panic!()
        };
        let new = WindowSnapshot {
            epoch: Ulid::new(),
            state: None,
        };
        let GuiAction::WindowSave { id: second, .. } = writes.next(Some(&new)).unwrap() else {
            panic!()
        };
        assert!(
            writes
                .fence(GuiAction::Focused {
                    id: Ulid::new(),
                    result: Err("fixture".into())
                })
                .is_none()
        );
        assert!(
            writes
                .completion(&GuiReply::WindowSaved {
                    op: first,
                    error: None
                })
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            writes
                .completion(&GuiReply::WindowSaved {
                    op: second,
                    error: None
                })
                .unwrap()
                .len(),
            1
        );
    }
}
