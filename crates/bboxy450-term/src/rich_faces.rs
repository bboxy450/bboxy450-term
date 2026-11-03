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
    fn rich_control(&self, seat: SeatId, body: Rect) -> Option<Rect> {
        self.live_state
            .as_ref()?
            .gui_seats
            .iter()
            .any(|r| r.record.id.0 == seat.0.0 && r.record.harness_kind == "bboxy450-harness")
            .then_some(Rect::new(
                (body.right() - 96.).max(body.x),
                body.y - 24.,
                88.,
                22.,
            ))
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
        let focused = self.focus.stage.as_ref().and_then(|s| s.focused);
        for (id, rect, generation, path) in visible {
            if self
                .rich_panes
                .get(&id)
                .is_some_and(|p| p.generation != generation || p.path != path)
            {
                self.rich_panes.remove(&id);
            }
            if !self.rich_panes.contains_key(&id) {
                let pane = Pane::connect(
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
                );
                self.rich_panes.insert(id, pane);
            }
            let pane = self.rich_panes.get_mut(&id).unwrap();
            pane.resize(rect);
            if self.rich_focused.get(&id) != Some(&(focused == Some(id)))
                && pane.send(Input::Focus(focused == Some(id)))
            {
                self.rich_focused.insert(id, focused == Some(id));
            }
        }
        self.rich_focused
            .retain(|id, _| self.rich_panes.contains_key(id));
    }
    pub(super) fn rich_drain(&mut self) {
        let start = Instant::now();
        let ids: Vec<_> = self.rich_panes.keys().copied().collect();
        for i in 0..ids.len() {
            if start.elapsed() >= Duration::from_micros(300) {
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
            if let Some(button) = self.rich_control(*id, *rect) {
                let mut list = DrawList::default();
                list.rect(button, 4., Color::hex(bbt_ui_model::TOKENS.color.dark.bg));
                list.text(
                    button,
                    if self.rich_face(*id) {
                        "Terminal"
                    } else {
                        "Harness"
                    },
                    TextStyle::default(),
                    Color::WHITE,
                );
                ops.push(bbt_ui::components::Paint::Draw(list));
            }
            if !self.rich_face(*id) {
                continue;
            }
            if let Some(pane) = self.rich_panes.get(id) {
                ops.extend(pane.painted().iter().cloned());
                if !pane.painted().is_empty() {
                    continue;
                }
            }
            let mut list = DrawList::default();
            list.rect(*rect, 0., Color::hex("171719"));
            list.text(
                *rect,
                "Harness starting / unavailable · Terminal remains available",
                TextStyle::default(),
                Color::WHITE,
            );
            ops.push(bbt_ui::components::Paint::Draw(list));
        }
        ops
    }
    fn rich_target(&self) -> Option<SeatId> {
        if self.app_overlays.open()
            || self.document.open
            || self.launcher.is_some()
            || self.local_tab.is_some()
            || self.composer.view.focus
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
        let command = input
            .modifiers
            .state
            .contains(winit::keyboard::ModifiersState::SUPER);
        if command
            && matches!(
                keyboard::shortcut_key(input),
                Some('q' | 'w' | 't' | 'n' | '1'..='9')
            )
        {
            return false;
        }
        if command && keyboard::shortcut_key(input) == Some('v') {
            if input.state.is_pressed()
                && let Ok(Some(bboxy450_term_desktop::ClipboardContent::Text(text))) =
                    self.clipboard_content()
                && let Some(pane) = self.rich_panes.get(&id)
            {
                pane.send(Input::Edit(EditCommand::Insert(text)));
            }
            return true;
        }
        if input.state.is_pressed() {
            use bbt_ui::text_edit::{Command as E, Motion};
            let action = match &input.logical_key {
                Key::Named(NamedKey::Enter)
                    if input
                        .modifiers
                        .state
                        .contains(winit::keyboard::ModifiersState::SHIFT) =>
                {
                    Some(Input::Edit(E::Insert("\n".into())))
                }
                Key::Named(NamedKey::Enter) => Some(Input::Enter),
                Key::Named(NamedKey::Escape) => Some(Input::Cancel),
                Key::Named(NamedKey::Backspace) => Some(Input::Edit(E::Delete { backwards: true })),
                Key::Named(NamedKey::Delete) => Some(Input::Edit(E::Delete { backwards: false })),
                Key::Named(
                    k @ (NamedKey::ArrowLeft
                    | NamedKey::ArrowRight
                    | NamedKey::ArrowUp
                    | NamedKey::ArrowDown
                    | NamedKey::Home
                    | NamedKey::End),
                ) => Some(Input::Edit(E::Move {
                    motion: match k {
                        NamedKey::ArrowLeft => Motion::Left,
                        NamedKey::ArrowRight => Motion::Right,
                        NamedKey::ArrowUp => Motion::Up,
                        NamedKey::ArrowDown => Motion::Down,
                        NamedKey::Home => Motion::LineStart,
                        _ => Motion::LineEnd,
                    },
                    extend: input
                        .modifiers
                        .state
                        .contains(winit::keyboard::ModifiersState::SHIFT),
                })),
                _ if command => match keyboard::shortcut_key(input) {
                    Some('a') => Some(Input::Edit(E::SelectAll)),
                    Some('z') => Some(Input::Edit(E::Undo)),
                    _ => None,
                },
                _ => input.text.map(|s| Input::Edit(E::Insert(s.into()))),
            };
            if let (Some(pane), Some(action)) = (self.rich_panes.get(&id), action) {
                pane.send(action);
            }
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
            if self.rich_control(id, rect).is_some_and(|r| hit(r, at)) {
                let face = if self.rich_face(id) {
                    PaneFace::Terminal
                } else {
                    PaneFace::HarnessRich
                };
                self.pane_faces.insert(bbt_mux::Ulid(id.0.0), face);
                self.rich_sync();
                self.request_redraw();
                return true;
            }
            if self.rich_face(id) && hit(rect, at) {
                self.focus_action(FocusAction::Activate(id, FocusOrigin::Pane));
                if let Some(pane) = self.rich_panes.get(&id) {
                    pane.send(Input::Click([at[0] - rect.x, at[1] - rect.y]));
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
                    let dy = match delta {
                        MouseScrollDelta::LineDelta(_, y) => -*y * 28.,
                        MouseScrollDelta::PixelDelta(p) => {
                            -p.y as f32
                                / self.window.as_ref().map_or(1., |w| w.scale_factor() as f32)
                        }
                    };
                    pane.send(Input::Wheel(
                        [self.pointer[0] - rect.x, self.pointer[1] - rect.y],
                        dy,
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
    #[ignore = "private offscreen Metal capture; make test-rich-pane"]
    fn rich_native_parity_and_budget() {
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
        let frame = capture_frame(&mut view, &mut state, size, None, false).unwrap();
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
                    row.as_chunks::<4>().0.iter()
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
        let report = serde_json::json!({"row_structure_mismatch":mismatch,"four_visible_views_composition_p99_ms":p99,"ui_drain_allocation_ms":0.3,"ui_projection_allocation_ms":0.6,"memory_allocation_mib":32,"memory_measured":false,"under_load":"UNMEASURED","harness":std::env::var("BBOXY_HARNESS_CANDIDATE").ok()});
        std::fs::write(
            output.join("report.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        eprintln!("rich-pane: {report}");
    }
}
