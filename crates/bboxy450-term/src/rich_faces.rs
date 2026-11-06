// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::*;
use bboxy450_term::rich_pane::{Input, Pane};
use bbt_proto::gui::PaneFace;
use bbt_ui_model::draw::{Color, DrawList, TextStyle};
impl App {
    pub(super) fn rich_observation(
        &self,
    ) -> std::collections::BTreeMap<bbt_mux::Ulid, bbt_proto::gui::UiRichObservation> {
        if !self.test_account {
            return Default::default();
        }
        self.pane_bounds
            .iter()
            .filter_map(|(id, _)| {
                let pane = self.rich_panes.get(id)?;
                Some((
                    bbt_mux::Ulid(id.0.0),
                    bbt_proto::gui::UiRichObservation {
                        face: PaneFace::HarnessRich,
                        ready: pane.snapshot.ready,
                        cursor: self.rich_cursors.get(&bbt_mux::Ulid(id.0.0)).copied(),
                        retained_items: pane.snapshot.retained_items,
                        paint_count: pane.painted().len(),
                        error: pane.snapshot.error.clone(),
                        focused: pane.snapshot.focused,
                        composer_len: pane.snapshot.composer_len,
                        scroll: pane.snapshot.scroll,
                        approval_choices: pane.snapshot.approval_choices.clone(),
                        ime_allowed: self.text_ime_allowed(),
                        ime_caret: pane.snapshot.ime_caret.map(|r| [r.x, r.y, r.w, r.h]),
                    },
                ))
            })
            .collect()
    }
    pub(super) fn rich_face(&self, seat: SeatId) -> bool {
        self.pane_faces
            .get(&bbt_mux::Ulid(seat.0.0))
            .copied()
            .unwrap_or_else(|| {
                if self.live_state.as_ref().is_some_and(|m| {
                    m.gui_seats.iter().any(|r| {
                        r.record.id.0 == seat.0.0
                            && r.record.harness_kind == "bboxy450-harness"
                            && r.record.harness_session_id.is_some()
                    })
                }) {
                    PaneFace::HarnessRich
                } else {
                    PaneFace::Terminal
                }
            })
            == PaneFace::HarnessRich
    }
    pub(super) fn rich_header_views(&self) -> BTreeMap<SeatId, bbt_ui::modules::strip::ViewTarget> {
        use bbt_ui::modules::strip::ViewTarget;
        self.live_state
            .iter()
            .flat_map(|model| model.gui_seats.iter())
            .filter(|row| row.record.harness_kind == "bboxy450-harness")
            .map(|row| {
                let id = SeatId::from_u128(row.record.id.0);
                let target = if self.rich_face(id) {
                    ViewTarget::Terminal
                } else {
                    ViewTarget::Harness
                };
                (id, target)
            })
            .collect()
    }
    pub(super) fn rich_toggle_face(&mut self, seat: SeatId) {
        // Retain the old control's live harness eligibility, including before
        // announcement: Terminal remains reachable while the rich face starts.
        if !self.rich_header_views().contains_key(&seat) {
            return;
        }
        let face = if self.rich_face(seat) {
            PaneFace::Terminal
        } else {
            PaneFace::HarnessRich
        };
        self.pane_faces.insert(bbt_mux::Ulid(seat.0.0), face);
        self.rich_sync();
        self.sync_text_ime();
        self.request_redraw();
    }
    pub(super) fn rich_theme_changed(&mut self) {
        let theme = self.scene.ui_context().theme;
        for pane in self.rich_panes.values() {
            pane.set_theme(theme);
        }
    }
    pub(super) fn rich_sync(&mut self) {
        if self.occluded || self.document.open || self.local_tab.is_some() {
            self.rich_panes.clear();
            self.rich_focused.clear();
            return;
        }
        let visible: Vec<_> = self
            .pane_bounds
            .iter()
            .filter_map(|(id, r)| {
                if !self.rich_face(*id) {
                    return None;
                }
                let row = self
                    .live_state
                    .as_ref()?
                    .gui_seats
                    .iter()
                    .find(|s| s.record.id.0 == id.0.0)?;
                Some((*id, *r, row.state.generation, row.rich_socket.clone()?))
            })
            .collect();
        self.rich_panes
            .retain(|id, _| visible.iter().any(|(s, _, _, _)| s == id));
        let theme = self.scene.ui_context().theme;
        let focused = self.rich_target();
        let mut focus_changed = false;
        for (id, rect, generation, path) in visible {
            if self
                .rich_panes
                .get(&id)
                .is_some_and(|p| p.generation != generation || p.path != path)
            {
                self.rich_panes.remove(&id);
            }
            if !self.rich_panes.contains_key(&id) {
                let pane = Pane::connect_retained(
                    path,
                    generation,
                    rect,
                    self.rich_cursors
                        .get(&bbt_mux::Ulid(id.0.0))
                        .filter(|c| {
                            c.generation == generation
                                && self.live_state.as_ref().is_some_and(|m| {
                                    m.gui_seats.iter().any(|r| {
                                        r.record.id.0 == id.0.0
                                            && r.record.harness_session_id.as_deref()
                                                == Some(&c.session.to_string())
                                    })
                                })
                        })
                        .map(|c| {
                            (
                                bboxy450_harness::protocol::Epoch(c.epoch),
                                bboxy450_harness::protocol::Seq(c.seq),
                            )
                        }),
                    Arc::new(Wake::new(self.proxy.clone(), bbt_rt::handle().clone())),
                    self.rich_drafts.entry(id).or_default(),
                );
                self.rich_panes.insert(id, pane);
            }
            let pane = self.rich_panes.get_mut(&id).unwrap();
            pane.set_theme(theme);
            pane.resize(rect);
            if self.rich_focused.get(&id) != Some(&(focused == Some(id)))
                && pane.send(Input::Focus(focused == Some(id)))
            {
                self.rich_focused.insert(id, focused == Some(id));
                focus_changed = true;
            }
        }
        self.rich_focused
            .retain(|id, _| self.rich_panes.contains_key(id));
        if focus_changed {
            self.sync_text_ime();
        }
    }
    pub(super) fn rich_drain(&mut self) {
        let start = Instant::now();
        self.rich_drain_with_clock(|| start.elapsed());
    }
    fn rich_drain_with_clock(&mut self, mut elapsed: impl FnMut() -> Duration) {
        let ids: Vec<_> = self.rich_panes.keys().copied().collect();
        for i in 0..ids.len() {
            if elapsed() >= Duration::from_micros(300) {
                self.request_redraw(); // Deferred dirty views need another frame even while idle.
                break;
            }
            let id = ids[(self.rich_round + i) % ids.len()];
            if self.rich_panes.get_mut(&id).unwrap().drain() {
                if self.rich_panes[&id].snapshot.ready
                    && let Some((session, epoch, seq)) = self.rich_panes[&id].snapshot.cursor
                {
                    self.rich_cursors.insert(
                        bbt_mux::Ulid(id.0.0),
                        bbt_proto::gui::RichCursor {
                            session: session.0,
                            generation: self.rich_panes[&id].generation,
                            epoch: epoch.0,
                            seq: seq.0,
                        },
                    );
                }
                self.request_redraw();
            }
        }
        if !ids.is_empty() {
            self.rich_round = (self.rich_round + 1) % ids.len();
        }
    }
    pub(super) fn rich_paints(&self) -> Vec<bbt_ui::components::Paint> {
        let mut ops = Vec::new();
        for (id, rect) in &self.pane_bounds {
            if !self.rich_face(*id) {
                continue;
            }
            if let Some(pane) = self.rich_panes.get(id)
                && !pane.painted().is_empty()
            {
                let paints = pane.painted().to_vec();
                if self.scene.focused() == Some(*id) {
                    ops.extend(paints);
                } else {
                    let palette = self.scene.ui_context().theme.palette();
                    let dim = bbt_ui_model::TOKENS.dim;
                    let mut wash = DrawList::default();
                    wash.rect(
                        *rect,
                        0.,
                        Color::hex(palette.bg).mix(Color::hex(palette.fg_dim), dim.wash as f32),
                    );
                    ops.push(bbt_ui::components::Paint::Draw(wash));
                    ops.push(bbt_ui::components::Paint::Group {
                        ops: paints,
                        opacity: dim.opacity as f32,
                        saturation: dim.saturate as f32,
                        mask: None,
                    });
                }
                continue;
            }
            let mut list = DrawList::default();
            list.rect(
                *rect,
                0.,
                Color::hex(self.scene.ui_context().theme.palette().bg),
            );
            list.text(
                *rect,
                "Harness starting / unavailable · Terminal remains available",
                TextStyle::default(),
                Color::hex(self.scene.ui_context().theme.palette().fg),
            );
            ops.push(bbt_ui::components::Paint::Draw(list));
        }
        ops
    }
    pub(super) fn rich_target(&self) -> Option<SeatId> {
        if self.app_overlays.open()
            || self.document.open
            || self.launcher.is_some()
            || self.local_tab.is_some()
            || self.composer.view.focus
            || self.reject_focus
        {
            return None;
        }
        self.focus
            .stage
            .as_ref()?
            .focused
            .filter(|id| self.rich_face(*id))
    }
    pub(super) fn rich_key(&mut self, input: &keyboard::Input<'_>) -> bool {
        let Some(id) = self.rich_target() else {
            return false;
        };
        match keyboard::rich_key(input) {
            keyboard::RichKey::Global => return false,
            keyboard::RichKey::Paste => {
                if let Ok(Some(content)) = self.clipboard_content()
                    && let Some(pane) = self.rich_panes.get(&id)
                {
                    pane.send(Input::clipboard(content));
                }
            }
            keyboard::RichKey::Pane(action) => {
                if let Some(pane) = self.rich_panes.get(&id) {
                    pane.send(action);
                }
            }
            keyboard::RichKey::Consumed => {}
        }
        true
    }
    pub(super) fn rich_ime(&self, event: &Ime) -> bool {
        let Some(id) = self.rich_target() else {
            return false;
        };
        let edit = match event {
            Ime::Commit(t) => Some(EditCommand::Commit(t.clone())),
            Ime::Preedit(t, _) => Some(EditCommand::Preedit {
                text: t.clone(),
                caret: None,
            }),
            Ime::Disabled => Some(EditCommand::CancelPreedit),
            _ => None,
        };
        if let (Some(pane), Some(edit)) = (self.rich_panes.get(&id), edit) {
            pane.send(Input::Edit(edit));
        }
        true
    }
    pub(super) fn rich_pointer(&mut self, at: [f32; 2]) -> bool {
        if self.app_overlays.open()
            || self.document.open
            || self.launcher.is_some()
            || self.local_tab.is_some()
        {
            return false;
        }
        for (id, rect) in self.pane_bounds.clone() {
            if self.rich_face(id) && hit(rect, at) {
                self.focus_action(FocusAction::Activate(id, FocusOrigin::Pane));
                if let Some(pane) = self.rich_panes.get(&id) {
                    if pane.painted().is_empty() {
                        return true;
                    }
                    let local = [at[0] - rect.x, at[1] - rect.y];
                    let control = pane.snapshot.controls.iter().any(|(r, _)| hit(*r, local));
                    if !control {
                        if let Some(bbt_ui::modules::document_modal::controller::Action::ExpandDiagram(key)) = pane.snapshot.links.hit(local) {
                            if let Some(canvas) = pane.snapshot.links.diagram(&key) { self.open_diagram(canvas); }
                            return true;
                        }
                        if let Some(bbt_ui::modules::document_modal::controller::Action::Open(target)) =
                            pane.snapshot.links.hit(local)
                        {
                            if ["https://", "http://", "mailto:"]
                                .iter()
                                .any(|scheme| target.starts_with(scheme))
                            {
                                self.open_external_link(target);
                            } else {
                                self.open_document(target);
                            }
                            return true;
                        }
                        if self.ui_selection.begin(at, self.scene.text_metrics()) {
                            self.request_redraw();
                            return true;
                        }
                    }
                    pane.send(Input::Click(local));
                }
                return true;
            }
        }
        false
    }
    pub(super) fn rich_wheel(&self, delta: &MouseScrollDelta) -> bool {
        if self.app_overlays.open()
            || self.document.open
            || self.launcher.is_some()
            || self.local_tab.is_some()
        {
            return false;
        }
        for (id, rect) in &self.pane_bounds {
            if self.rich_face(*id) && hit(*rect, self.pointer) {
                if let Some(pane) = self.rich_panes.get(id) {
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(x, y) => [-*x * 28., -*y * 28.],
                        MouseScrollDelta::PixelDelta(p) => {
                            let scale = self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
                            [-p.x as f32 / scale, -p.y as f32 / scale]
                        }
                    };
                    pane.send(Input::Wheel2D(
                        [self.pointer[0] - rect.x, self.pointer[1] - rect.y],
                        delta,
                    ));
                }
                return true;
            }
        }
        false
    }
}

fn hit(r: Rect, at: [f32; 2]) -> bool {
    at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom()
}

#[cfg(test)]
mod verification {
    #![allow(
        clippy::disallowed_methods,
        reason = "private unattended offscreen fixture"
    )]
    use super::*;
    use bboxy450_harness::frontends::gui::{
        bridge::{FrameState, read_replay},
        smoke::capture_frame,
        view::HarnessView,
    };
    use bbt_render::chrome::{ChromeRenderer, materials as m};
    fn png(path: &std::path::Path, size: [u32; 2], rgba: &[u8]) {
        let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), size[0], size[1]);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(rgba)
            .unwrap();
    }
    fn pixels(path: &std::path::Path) -> Vec<u8> {
        let mut r = png::Decoder::new(std::io::BufReader::new(std::fs::File::open(path).unwrap()))
            .read_info()
            .unwrap();
        let mut b = vec![0; r.output_buffer_size().unwrap()];
        let n = r.next_frame(&mut b).unwrap().buffer_size();
        b.truncate(n);
        b
    }
    #[test]
    fn harness_theme_parent_preferences_fan_out_without_model_update() {
        let name = "rich_faces::verification::harness_theme_parent_preferences_fan_out_without_model_update";
        if std::env::var_os("BBOXY_THEME_APP_PRIVATE").is_none() {
            let home = tempfile::Builder::new()
                .prefix("theme-app-home-")
                .tempdir_in("/tmp")
                .unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", name, "--nocapture", "--test-threads=1"])
                .env_clear()
                .env("HOME", home.path())
                .env("PATH", "/usr/bin:/bin")
                .env("BBOXY_THEME_APP_PRIVATE", "1")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        struct Signal;
        impl bbt_rt::Waker for Signal {
            fn wake(&self) {}
        }
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        let root = tempfile::Builder::new()
            .prefix("theme-parent-")
            .tempdir_in("/tmp")
            .unwrap();
        let seat = SeatId::from_u128(42);
        let rect = Rect::new(0., 0., 640., 400.);
        app.pane_faces
            .insert(bbt_mux::Ulid(seat.0.0), PaneFace::HarnessRich);
        app.pane_bounds = vec![(seat, rect)];
        app.rich_panes.insert(
            seat,
            Pane::connect(
                root.path().join("unavailable.sock"),
                17,
                rect,
                None,
                Arc::new(Signal),
            ),
        );
        for theme in [Theme::Light, Theme::Dark, Theme::Light] {
            app.system_theme = theme;
            // This is the production handler called by WindowEvent::ThemeChanged
            // and explicit appearance preferences, without any model publication.
            app.apply_preferences();
            let deadline = Instant::now() + Duration::from_secs(8);
            loop {
                let pane = app.rich_panes.get_mut(&seat).unwrap();
                pane.drain();
                if pane.snapshot.theme == theme && !pane.painted().is_empty() {
                    break;
                }
                assert!(
                    Instant::now() < deadline,
                    "appearance did not reach loading pane"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(app.rich_panes[&seat].generation, 17);
            assert!(app.rich_panes[&seat].snapshot.cursor.is_none());
            assert_eq!(
                app.rich_panes[&seat].snapshot.completed_inputs, 0,
                "appearance bypasses input/agent command queues"
            );
        }
        app.rich_panes.clear();
        let fallback = app.rich_paints();
        assert!(
            matches!(&fallback[0],bbt_ui::components::Paint::Draw(list) if list.primitives.iter().any(|p|matches!(p,bbt_ui_model::draw::Prim::Rect{fill,..} if *fill==Color::hex(Theme::Light.palette().bg))))
        );
    }
    #[test]
    fn rich_keys_and_face_preference_do_not_reach_the_pty() {
        let name = "rich_faces::verification::rich_keys_and_face_preference_do_not_reach_the_pty";
        if crate::lazy_projection_tests::isolated_ui_case(name) {
            return;
        }
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        let seat = SeatId::from_u128(42);
        app.focus.stage = Some(Stage {
            focused: Some(seat),
            ..Default::default()
        });
        app.pane_faces
            .insert(bbt_mux::Ulid(seat.0.0), PaneFace::HarnessRich);
        let mut adapter = keyboard::State::default();
        for event in ["press", "repeat", "release"] {
            let key:bbt_proto::gui::UiInput=serde_json::from_value(serde_json::json!({"kind":"key","logical":"x","physical":"KeyX","unmodified":"x","text":"x","event":event})).unwrap();
            let key = adapter.structured(&key).unwrap();
            assert!(
                app.key_input(&key).is_none(),
                "rich input cannot produce RoutedKey::Terminal"
            );
        }
        assert!(
            app.pending_commands.is_empty(),
            "rich key events queued zero PTY commands while unavailable"
        );
        assert!(app.rich_ime(&Ime::Commit("日本語".into())));
        let mut saved = bboxy450_term::window_state::capture(
            &bboxy450_term::model::Model::default(),
            &Stage::default(),
            [800., 600.],
            None,
        );
        saved.pane_faces = app.pane_faces.clone();
        saved.rich_cursors.insert(
            bbt_mux::Ulid(seat.0.0),
            bbt_proto::gui::RichCursor {
                session: bbt_mux::Ulid(99),
                generation: 1,
                epoch: 2,
                seq: 7,
            },
        );
        let restored: bbt_proto::gui::WindowState =
            serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap();
        assert_eq!(restored, saved);
        app.pane_faces
            .insert(bbt_mux::Ulid(seat.0.0), PaneFace::Terminal);
        assert!(!app.rich_face(seat));
    }
    #[test]
    fn view_switch_titlebar_pointer_action_and_saved_preference() {
        use bbt_ui::modules::strip::ViewTarget;
        let name =
            "rich_faces::verification::view_switch_titlebar_pointer_action_and_saved_preference";
        if crate::lazy_projection_tests::isolated_ui_case(name) {
            return;
        }
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        let seat = SeatId::from_u128(42);
        let ordinary = SeatId::from_u128(43);
        let record = bbt_proto::registry::SeatRecord::new(
            bbt_mux::Ulid(42),
            bbt_mux::Ulid(1),
            "Harness pane",
            "bboxy450-harness",
        );
        let terminal = bbt_proto::registry::SeatRecord::new(
            bbt_mux::Ulid(43),
            bbt_mux::Ulid(1),
            "Shell pane",
            "zsh",
        );
        let mut model = bboxy450_term::model::build(
            &[record.clone(), terminal.clone()],
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
        );
        let rows = [record, terminal].map(|record| bbt_proto::GuiSeat {
            state: bbt_proto::SeatState {
                generation: record.generation,
                process: bbt_proto::ProcessStatus::Running { pid: 0 },
                activity: bbt_proto::Activity::Idle,
                grade: bbt_proto::Grade::Reported,
                title: String::new(),
                detail: None,
                stale: false,
                turn_open: false,
                observed_ms: 1,
                subagents: vec![],
            },
            record,
            rich_socket: None,
            attention: None,
            notification_episode: None,
        });
        model.apply_seat_states(&rows);
        let mut stage = model.ui.window.stage.clone();
        stage.focused = Some(seat);
        app.focus.stage = Some(stage.clone());
        app.scene = scene::Scene::live();
        app.scene.set_model(&model, &stage);
        app.live_state = Some(Arc::new(model));
        assert_eq!(
            app.rich_header_views().get(&seat),
            Some(&ViewTarget::Harness)
        );
        assert!(!app.rich_header_views().contains_key(&ordinary));
        app.navigate_to(&format!("pane-view:{ordinary}"));
        assert!(!app.pane_faces.contains_key(&bbt_mux::Ulid(43)));
        for target in [ViewTarget::Harness, ViewTarget::Terminal] {
            app.solve_ui([1200., 800.]).unwrap();
            let rect = app
                .navigation_regions
                .iter()
                .find(|(key, _)| key == &format!("pane-view:{seat}"))
                .unwrap()
                .1;
            assert_eq!(app.rich_header_views()[&seat], target);
            let before = app.pending_commands.len();
            app.pointer_pressed([rect.x + rect.w / 2., rect.y + rect.h / 2.]);
            app.pointer_released();
            assert_eq!(
                app.pending_commands.len(),
                before,
                "view action queued a PTY/backend command"
            );
            assert_eq!(app.rich_face(seat), target == ViewTarget::Harness);
            assert_eq!(
                app.pane_faces[&bbt_mux::Ulid(42)],
                if app.rich_face(seat) {
                    PaneFace::HarnessRich
                } else {
                    PaneFace::Terminal
                }
            );
            if app.rich_face(seat) {
                assert!(app.rich_ime(&Ime::Commit("私的な入力".into())));
                assert!(
                    !app.rich_paints().is_empty(),
                    "unavailable rich view keeps its fallback"
                );
            } else {
                assert!(
                    app.rich_paints().is_empty(),
                    "Terminal has no floating overlay button"
                );
            }
        }
        // The existing WindowState serialization continues to carry the explicit choice.
        app.solve_ui([1200., 800.]).unwrap();
        let saved = app.window_last.as_ref().unwrap();
        assert_eq!(
            saved.pane_faces.get(&bbt_mux::Ulid(42)),
            Some(&PaneFace::Terminal)
        );
        assert_eq!(
            app.live_state.as_ref().unwrap().gui_seats[0]
                .state
                .generation,
            rows[0].state.generation
        );
    }
    #[test]
    #[ignore = "private offscreen Metal capture; make test-rich-pane"]
    fn rich_native_parity_and_composition_microbench() {
        bbt_rt::handle().block_on(bboxy450_harness::runtime::blocking(parity_capture));
    }
    fn parity_capture() {
        let output = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/rich-pane-parity");
        std::fs::create_dir_all(&output).unwrap();
        let fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../bboxy450-harness/tests/fixtures/ir-v1/v1.1-all-items.jsonl");
        let transcript = bbt_rt::handle().block_on(read_replay(fixture)).unwrap();
        let mut state = FrameState {
            transcript: Arc::new(transcript),
            ready: true,
            replay: true,
            ..Default::default()
        };
        let mut view = HarnessView::default();
        let size = [1120, 820];
        let frame = capture_frame(&mut view, &mut state, &{
            let mut options =
                bboxy450_harness::frontends::gui::options::Options::parse(Vec::<String>::new())
                    .unwrap();
            options.capture = Some(size);
            options.no_chrome = true;
            options
        })
        .unwrap();
        let mut native =
            ChromeRenderer::new(bboxy450_harness::frontends::gui::assets::icons()).unwrap();
        let standalone = m::render_composed_to_rgba(
            &mut native,
            m::Scene {
                backdrop: m::Backdrop::DrawList(&frame.list),
                items: &frame.rims,
                width: size[0],
                height: size[1],
                scale: 1.,
                time_ms: 0.,
                theme: Theme::Dark,
            },
            &frame.over,
        )
        .unwrap();
        png(&output.join("standalone.png"), size, &standalone);
        let ops = bboxy450_term::rich_pane::paints(
            frame,
            Rect::new(0., 0., size[0] as f32, size[1] as f32),
        );
        let mut gpu = bbt_rt::handle()
            .block_on(crate::gpu::VerificationFrame::new())
            .unwrap();
        gpu.resize(size);
        let (sent, received) = std::sync::mpsc::sync_channel(1);
        let model = bboxy450_term::model::Model::default();
        gpu.draw_layers_notifying(
            ops,
            crate::gpu::Panes {
                model: &model,
                bounds: &[],
                focused: None,
                overlay: vec![],
                retained: None,
                overlay_after: vec![],
            },
            1,
            0.,
            sent,
            || {},
        )
        .unwrap();
        received.recv_timeout(Duration::from_secs(10)).unwrap();
        let (sent, received) = std::sync::mpsc::sync_channel(1);
        gpu.capture(
            output.join("pane.png"),
            None,
            crate::log::Logger::new(),
            move |r| {
                let _ = sent.send(r);
            },
        );
        received
            .recv_timeout(Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let pane = pixels(&output.join("pane.png"));
        assert_eq!(pane.len(), standalone.len());
        // Adapted from bboxy450-harness scripts/parity-web.sh row profile.
        // https://github.com/bboxy450/bboxy450-harness/blob/main/ATTRIBUTION.md
        let row_ink = |bytes: &[u8]| -> Vec<bool> {
            bytes
                .chunks_exact(size[0] as usize * 4)
                .map(|row| {
                    let gray = |p: &[u8]| {
                        0.299 * f64::from(p[0]) + 0.587 * f64::from(p[1]) + 0.114 * f64::from(p[2])
                    };
                    let ground = gray(&row[..4]);
                    row.as_chunks::<4>()
                        .0
                        .iter()
                        .step_by(4)
                        .filter(|p| (gray(*p) - ground).abs() > 24.)
                        .count()
                        > 2
                })
                .collect()
        };
        let a = row_ink(&standalone);
        let b = row_ink(&pane);
        assert!(
            a.iter().any(|v| *v) && a.iter().any(|v| !*v),
            "non-vacuous row profile"
        );
        let mismatch = a.iter().zip(&b).filter(|(a, b)| a != b).count() as f64 / a.len() as f64;
        assert!(mismatch <= 0.01, "row_structure_mismatch={mismatch}");
        let mut samples = Vec::new();
        for _ in 0..200 {
            let at = Instant::now();
            for _ in 0..4 {
                let _ = bboxy450_term::rich_pane::paints(frame, Rect::new(0., 0., 1120., 820.));
            }
            samples.push(at.elapsed().as_secs_f64() * 1000.);
        }
        samples.sort_by(f64::total_cmp);
        let p99 = samples[197];
        let report = serde_json::json!({"measurement":"four-copy Frame-to-Paint conversion microbenchmark","whole_frame_budget_verdict":"UNPROVEN","row_structure_mismatch":mismatch,"four_visible_views_composition_p99_ms":p99,"ui_drain_allocation_ms":0.3,"ui_projection_allocation_ms":0.6,"memory_allocation_mib":32,"memory_measured":false,"under_load":"UNMEASURED","harness":std::env::var("BBOXY_HARNESS_CANDIDATE").ok()});
        std::fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        eprintln!("rich-pane: {report}");
    }
}

#[cfg(test)]
#[path = "rich_faces/budgets.rs"]
mod budget_verification;
