//! Starting-pane effects stay on the GUI owner; the existing IO queue still launches seats.
use crate::*;
use bboxy450_term::starting::{Intent, Launch};

impl App {
    pub(super) fn prepare_starting(
        &self,
        message: &mut live::UiMsg,
    ) -> Result<Option<Launch>, String> {
        let Some(model) = &self.live_state else {
            return Ok(None);
        };
        if matches!(message, live::UiMsg::SpawnZsh) {
            *message = live::UiMsg::Spawn {
                id: bbt_mux::Ulid::new(),
                swarm: None,
                placement: None,
                harness_kind: "zsh".into(),
                name: None,
                role: None,
                location: bbt_proto::Where::Local,
            };
        }
        let (op, harness, name, swarm, placement) = match message {
            live::UiMsg::Spawn {
                id,
                harness_kind,
                name,
                swarm,
                placement,
                ..
            } => (
                *id,
                harness_kind.clone(),
                name.clone(),
                *swarm,
                placement.clone(),
            ),
            live::UiMsg::SpawnSpec { id, spec } => (
                *id,
                spec.harness_kind.clone(),
                spec.name.clone(),
                Some(spec.swarm),
                spec.placement.clone(),
            ),
            _ => return Ok(None),
        };
        let stage = self.focus.stage.as_ref().unwrap_or(&model.ui.window.stage);
        let launch = self
            .starting
            .prepare(
                Intent {
                    op,
                    harness,
                    name,
                    swarm,
                    placement: placement.unwrap_or_else(|| {
                        bbt_proto::seat_state::Placement::NewTab {
                            tab: bbt_proto::TabId::from_u128(bbt_mux::Ulid::new().0),
                        }
                    }),
                    layout_revision: self.scene.layout_revision(),
                    admitted_frame: self.submitted_frame,
                },
                model,
                stage,
            )
            .map_err(str::to_owned)?;
        match message {
            live::UiMsg::Spawn {
                name,
                swarm,
                placement,
                ..
            } => {
                *name = Some(launch.name.clone());
                *swarm = Some(launch.swarm);
                *placement = Some(launch.placement.clone());
            }
            live::UiMsg::SpawnSpec { spec, .. } => {
                spec.name = Some(launch.name.clone());
                spec.swarm = launch.swarm;
                spec.placement = Some(launch.placement.clone());
            }
            _ => unreachable!("spawn intent selected"),
        }
        Ok(Some(launch))
    }
    pub(super) fn admit_starting(&mut self, launch: Launch) {
        let op = launch.op;
        self.starting.admit(launch);
        let proxy = self.proxy.clone();
        let timer = bbt_rt::handle().spawn(async move {
            tokio::time::sleep(bboxy450_term::focus_ack::DEADLINE).await;
            let _ = proxy.send_event(UiEvent::StartingDeadline(op));
        });
        self.starting_timers.insert(op, timer.abort_handle());
        self.scene_dirty = true;
        self.request_redraw();
    }
    pub(super) fn starting_changed(&mut self) {
        let done: Vec<_> = self
            .starting_timers
            .keys()
            .filter(|op| {
                self.starting
                    .get(**op)
                    .is_none_or(|launch| launch.seat().is_some() || launch.refused.is_some())
            })
            .copied()
            .collect();
        for op in done {
            if let Some(timer) = self.starting_timers.remove(&op) {
                timer.abort();
            }
        }
        self.scene_dirty = true;
        self.request_redraw();
    }
    pub(super) fn starting_deadline(&mut self, op: bbt_mux::Ulid) {
        self.starting_timers.remove(&op);
        if self.starting.expire(op) {
            self.starting_changed();
        }
    }
    pub(super) fn dismiss_starting(&mut self, op: bbt_mux::Ulid) -> bool {
        if !self.starting.dismiss(op) {
            return false;
        }
        self.starting_changed();
        true
    }
    /// Called only after the renderer successfully produced this numbered frame.
    pub(super) fn starting_produced(&mut self, frame: u64, layouts: &[bbt_ui::components::Layout]) {
        self.submitted_frame = frame;
        if self.starting.visible().next().is_none() {
            return;
        }
        let bounds = self
            .scene
            .starting_bounds(layouts)
            .into_iter()
            .map(|(op, rect)| (op, rect_array(rect)))
            .collect::<Vec<_>>();
        if self.starting.presented(frame, &bounds) {
            let changed = self
                .live_state
                .as_ref()
                .is_some_and(|model| self.starting.observe(model));
            if changed {
                self.starting_changed();
            }
        }
    }
    pub(super) fn starting_layout(
        &self,
        layouts: &[bbt_ui::components::Layout],
    ) -> Vec<bbt_proto::gui::UiStartingPane> {
        self.scene
            .starting_bounds(layouts)
            .into_iter()
            .filter_map(|(op, rect)| {
                let launch = self.starting.get(op)?;
                Some(bbt_proto::gui::UiStartingPane {
                    op,
                    name: launch.name.clone(),
                    harness_kind: launch.harness.clone(),
                    swarm: bbt_mux::Ulid(launch.swarm.0.0),
                    placement: launch.placement.clone(),
                    state: launch.state(),
                    rect: rect_array(rect),
                    focused: self.scene.starting_stage().is_some_and(|(_, _, focused)| {
                        focused == Some(bboxy450_term::starting::Slot::Launch(op))
                    }),
                    admitted_frame_seq: launch.admitted_frame,
                    first_frame_seq: launch.first.as_ref().map(|frame| frame.frame_seq),
                })
            })
            .collect()
    }
    pub(super) fn starting_capture(&self, op: bbt_mux::Ulid) -> Option<UiSpawnCapture> {
        let launch = self.starting.get(op)?;
        Some(UiSpawnCapture {
            op,
            swarm: launch.swarm,
            placement: launch.placement.clone(),
            layout_revision: launch.layout_revision,
            seat: None,
            seat_row: None,
        })
    }
    /// A metadata read, not a UI mutation or a produced-layout request. It must
    /// work before observation is enabled and while a backend launch is pending.
    pub(super) fn answer_seats(&mut self, id: bbt_mux::Ulid) {
        let Some(model) = &self.live_state else {
            self.answer_ui(id, Err("ui_not_ready: no live model".into()));
            return;
        };
        let hidden: std::collections::BTreeSet<_> = self
            .starting
            .visible()
            .filter_map(|launch| launch.seat())
            .collect();
        let seats: Vec<_> = model
            .gui_seats
            .iter()
            .filter(|row| !hidden.contains(&SeatId::from_u128(row.record.id.0)))
            .cloned()
            .collect();
        if seats.len() + hidden.len() != model.gui_seats.len() {
            self.answer_ui(
                id,
                Err("ui_not_ready: retained seat metadata is incomplete".into()),
            );
            return;
        }
        let starting = self
            .starting
            .visible()
            .map(|launch| {
                bbt_proto::gui::UiStartingPane {
                    op: launch.op,
                    name: launch.name.clone(),
                    harness_kind: launch.harness.clone(),
                    swarm: bbt_mux::Ulid(launch.swarm.0.0),
                    placement: launch.placement.clone(),
                    state: launch.state(),
                    // Listing consumers use state/op, not geometry. None first_frame_seq
                    // explicitly means no produced starting frame has been observed yet.
                    rect: launch.first.as_ref().map_or([0.; 4], |frame| frame.rect),
                    focused: false,
                    admitted_frame_seq: launch.admitted_frame,
                    first_frame_seq: launch.first.as_ref().map(|frame| frame.frame_seq),
                }
            })
            .collect();
        self.answer_ui(
            id,
            Ok(bbt_proto::gui::UiLayout {
                frame_seq: self.submitted_frame,
                layout_revision: None,
                clock: None,
                size: self.solved_size,
                swarm: None,
                tab: None,
                focused: None,
                launcher_open: false,
                page: None,
                document: None,
                panes: Vec::new(),
                tree: Vec::new(),
                spawned: None,
                composed: None,
                starting,
                seats: Some(seats),
            }),
        );
    }
}
