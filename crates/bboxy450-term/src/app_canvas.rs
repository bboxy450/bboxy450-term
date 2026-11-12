// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained canvas cameras and ordinary GPU components; no IO in a frame.
use super::*;
use bboxy450_canvas::{Camera, Canvas, FIT_PADDING};
use bbt_ui::modules::document_modal::viewer::TouchPhase;
use bbt_ui_model::layout::Rect;

pub(super) struct View {
    pub camera: Camera,
    rect: Rect,
    fitted: bool,
    smart_return: Option<Camera>,
    drag: Option<[f32; 2]>,
    contacts: [Option<(u64, [f32; 2])>; 2],
    /// The last paint and what produced it: the drawing's digest, the camera,
    /// the pane rect and the theme. A frame that changes none of them reuses
    /// it instead of laying the whole drawing out again.
    cache: Option<(PaintKey, Vec<bbt_ui::components::Paint>)>,
}
#[derive(Clone, PartialEq)]
struct PaintKey {
    digest: String,
    camera: [f64; 3],
    rect: [f32; 4],
    theme: bbt_ui::Theme,
}
impl Default for View {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            rect: Rect::new(0., 0., 1., 1.),
            fitted: false,
            smart_return: None,
            drag: None,
            contacts: [None; 2],
            cache: None,
        }
    }
}
impl View {
    pub(super) fn contains(&self, p: [f32; 2]) -> bool {
        p[0] >= self.rect.x
            && p[0] < self.rect.right()
            && p[1] >= self.rect.y
            && p[1] < self.rect.bottom()
    }
    fn relative(&self, p: [f32; 2]) -> [f64; 2] {
        [
            f64::from(p[0] - self.rect.x - self.rect.w / 2.),
            f64::from(p[1] - self.rect.y - self.rect.h / 2.),
        ]
    }
    pub fn fit(&mut self, canvas: &Canvas) {
        // The same extent and padding as the headless PNG: fit means one thing.
        if let Ok(camera) = Camera::fit(
            canvas.extent(),
            [f64::from(self.rect.w), f64::from(self.rect.h)],
            FIT_PADDING,
        ) {
            self.camera = camera;
            self.smart_return = None;
            self.fitted = true;
        }
    }
    pub fn pan(&mut self, delta: [f32; 2]) {
        self.smart_return = None;
        let _ = self.camera.pan(delta.map(|d| -f64::from(d)));
    }
    pub fn zoom_at(&mut self, factor: f32, at: [f32; 2]) {
        if !self.touching() && self.contains(at) {
            self.smart_return = None;
            let _ = self.camera.zoom_at(f64::from(factor), self.relative(at));
        }
    }
    pub fn smart_zoom(&mut self, at: [f32; 2]) {
        if self.contains(at) && !self.touching() {
            if let Some(camera) = self.smart_return.take() {
                self.camera = camera;
            } else {
                let previous = self.camera;
                let factor = 1. / self.camera.zoom;
                if self.camera.zoom_at(factor, self.relative(at)).is_ok() {
                    self.smart_return = Some(previous);
                }
            }
            self.fitted = true;
        }
    }
    pub fn touching(&self) -> bool {
        self.contacts.iter().any(Option::is_some)
    }
    pub fn begin_pan(&mut self, at: [f32; 2]) {
        if !self.touching() && self.contains(at) {
            self.drag = Some(at);
        }
    }
    pub fn drag(&mut self, at: [f32; 2]) -> bool {
        let Some(old) = self.drag else {
            return false;
        };
        self.drag = Some(at);
        self.pan([old[0] - at[0], old[1] - at[1]]);
        true
    }
    pub fn release(&mut self) {
        self.drag = None;
    }
    pub fn cancel_gestures(&mut self) {
        self.release();
        self.contacts = [None; 2];
    }
    // Adapted from bboxy450-term document_modal/viewer.rs: bounded contact pair.
    // Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    pub fn touch(&mut self, id: u64, phase: TouchPhase, at: [f32; 2]) -> bool {
        let index = self
            .contacts
            .iter()
            .position(|c| c.is_some_and(|(i, _)| i == id));
        match phase {
            TouchPhase::Started => {
                if self.contains(at)
                    && let Some(i) =
                        index.or_else(|| self.contacts.iter().position(Option::is_none))
                {
                    self.contacts[i] = Some((id, at));
                    self.release();
                }
            }
            TouchPhase::Moved => {
                if let Some(i) = index {
                    let old = self.contacts;
                    self.contacts[i] = Some((id, at));
                    if let ([Some((_, a)), Some((_, b))], [Some((_, c)), Some((_, d))]) =
                        (old, self.contacts)
                    {
                        let from = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
                        let to = [(c[0] + d[0]) / 2., (c[1] + d[1]) / 2.];
                        let before = (a[0] - b[0]).hypot(a[1] - b[1]);
                        let after = (c[0] - d[0]).hypot(c[1] - d[1]);
                        if before > 1. && after > 1. {
                            let _ = self
                                .camera
                                .zoom_at(f64::from(after / before), self.relative(from));
                        }
                        let _ = self
                            .camera
                            .pan([f64::from(to[0] - from[0]), f64::from(to[1] - from[1])]);
                    } else if let Some((_, old)) = old[i] {
                        let _ = self
                            .camera
                            .pan([f64::from(at[0] - old[0]), f64::from(at[1] - old[1])]);
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                if let Some(i) = index {
                    self.contacts[i] = None;
                }
            }
        }
        false
    }
}
impl App {
    pub(super) fn canvas_target(&self) -> Option<SeatId> {
        if self.app_overlays.open()
            || self.local_tab.is_some()
            || self.document.open
            || self.launcher.is_some()
        {
            return None;
        }
        let model = self.live_state.as_ref()?;
        let id = self
            .focus
            .stage
            .as_ref()
            .unwrap_or(&model.ui.window.stage)
            .focused?;
        model.canvas_sources.contains_key(&id).then_some(id)
    }
    pub(super) fn canvas_paints(&mut self) -> Vec<bbt_ui::components::Paint> {
        let Some(model) = &self.live_state else {
            return vec![];
        };
        self.canvas_views
            .retain(|id, _| model.canvas_sources.contains_key(id));
        let mut paints = vec![];
        let mut facts = BTreeMap::new();
        for (id, rect) in &self.pane_bounds {
            let Some(source) = model.canvas_sources.get(id) else {
                continue;
            };
            let view = self.canvas_views.entry(*id).or_default();
            view.rect = *rect;
            let row = model
                .canvas_catalog
                .get(&(
                    bbt_ui_model::SwarmId::from_u128(source.swarm.0),
                    bboxy450_term::canvas_host::normalize_root(&source.root),
                ))
                .and_then(|rows| rows.iter().find(|r| r.source.id == source.id));
            if let Some(row) = row {
                if !view.fitted {
                    view.fit(&row.snapshot.canvas);
                }
                let key = PaintKey {
                    digest: row.snapshot.canvas.digest(),
                    camera: [
                        view.camera.center[0],
                        view.camera.center[1],
                        view.camera.zoom,
                    ],
                    rect: rect_array(*rect),
                    theme: self.scene.ui_context().theme,
                };
                if let Some((cached, ops)) = &view.cache
                    && *cached == key
                {
                    paints.extend(ops.iter().cloned());
                } else {
                    match self
                        .scene
                        .canvas_paint(&row.snapshot.canvas, &view.camera, *rect)
                    {
                        Ok(ops) => {
                            paints.extend(ops.iter().cloned());
                            view.cache = Some((key, ops));
                        }
                        Err(reason) => self.logger.write(format!("canvas render: {reason}")),
                    }
                }
                facts.insert(
                    bbt_mux::Ulid(id.0.0),
                    bbt_proto::gui::UiCanvasView {
                        source: source.clone(),
                        rect: rect_array(*rect),
                        camera: [
                            view.camera.center[0],
                            view.camera.center[1],
                            view.camera.zoom,
                        ],
                        seq: row.snapshot.seq,
                        digest: row.snapshot.canvas.digest(),
                    },
                );
            }
        }
        self.canvas_observation = facts;
        paints
    }
    pub(super) fn canvas_key(&mut self, input: &keyboard::Input<'_>) -> bool {
        if self.composer.view.focus || self.reject_focus {
            return false;
        }
        let Some(id) = self.canvas_target() else {
            return false;
        };
        if input.modifiers.state.super_key() {
            return false;
        }
        if input.state.is_pressed() {
            if keyboard::diagram_key(input) == keyboard::DiagramKey::Fit {
                if let Some(source) = self
                    .live_state
                    .as_ref()
                    .and_then(|m| m.canvas_sources.get(&id))
                    && let Some(row) = self
                        .live_state
                        .as_ref()
                        .and_then(|m| {
                            m.canvas_catalog.get(&(
                                bbt_ui_model::SwarmId::from_u128(source.swarm.0),
                                bboxy450_term::canvas_host::normalize_root(&source.root),
                            ))
                        })
                        .and_then(|rows| rows.iter().find(|r| r.source.id == source.id))
                    && let Some(view) = self.canvas_views.get_mut(&id)
                {
                    view.fit(&row.snapshot.canvas);
                }
            } else if let Some(view) = self.canvas_views.get_mut(&id) {
                match keyboard::diagram_key(input) {
                    keyboard::DiagramKey::Pan(delta) => view.pan(delta),
                    keyboard::DiagramKey::ZoomIn => {
                        let _ = view.camera.zoom_at(1.25, [0.; 2]);
                    }
                    keyboard::DiagramKey::ZoomOut => {
                        let _ = view.camera.zoom_at(0.8, [0.; 2]);
                    }
                    keyboard::DiagramKey::Actual => {
                        view.camera.zoom = 1.;
                    }
                    _ => {}
                }
            }
            self.request_redraw();
        }
        true
    }
    pub(super) fn refresh_canvases(&mut self, swarm: Option<bbt_mux::Ulid>) {
        self.queue(live::UiMsg::CanvasRefresh { swarm });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "private child UI registration and canvas store"
    )]
    fn canvas_native_gestures_share_diagram_path_fit_and_never_control_a_pty() {
        const CHILD: &str = "BBOXY_CANVAS_GESTURE_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let home = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "app_canvas::tests::canvas_native_gestures_share_diagram_path_fit_and_never_control_a_pty", "--nocapture", "--test-threads=1"])
                .env(CHILD, "1").env("HOME", home.path()).env("ZDOTDIR", home.path())
                .output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        struct QuietWake;
        impl bbt_rt::Waker for QuietWake {
            fn wake(&self) {}
        }
        let inbox = bbt_rt::init_ui(QuietWake);
        let updates = Arc::new(Latest::new(inbox.dirty.paint_source(5)));
        let mut app = App::owner(
            UiProxy::Headless(Arc::new(|_| {})),
            inbox,
            log::Logger::new(),
            updates,
            &["--headless".into()],
            Verify::default(),
        );
        app.startup.attached();

        let directory = tempfile::tempdir().unwrap();
        let swarm = bbt_mux::Ulid(42);
        let store = bboxy450_canvas::store::Store::new(directory.path().join(".bboxy/canvases"));
        let created = store
            .create(
                &bbt_mux::Ulid::new().to_string(),
                bboxy450_canvas::CanvasRecord {
                    title: "Gesture canvas".into(),
                    swarm: swarm.to_string(),
                    tags: ["one", "two", "three", "four", "five"]
                        .map(str::to_owned)
                        .to_vec(),
                    creator: "seat:test".into(),
                    task: None,
                },
            )
            .unwrap();
        let action = serde_json::from_value(serde_json::json!({"op":"add","element":{"id":"box","tags":[],"content":{"type":"shape","bounds":{"origin":[0,0],"size":[200,100]},"shape":"box","label":"Canvas"}}})).unwrap();
        store
            .append(&created.record, "draw", "seat:test", Some(0), &[action])
            .unwrap();
        let rows =
            bboxy450_term::canvas_host::load(&directory.path().to_string_lossy(), swarm).unwrap();
        let source = rows[0].source.clone();
        let mut model = bboxy450_term::model::Model::default();
        let id = model.open_canvas(source.clone(), None);
        let canvas = rows[0].snapshot.canvas.clone();
        model.canvas_catalog.insert(
            (
                bbt_ui_model::SwarmId::from_u128(swarm.0),
                bboxy450_term::canvas_host::normalize_root(&source.root),
            ),
            Arc::new(rows),
        );
        app.focus.stage = Some(model.ui.window.stage.clone());
        app.live_state = Some(Arc::new(model));
        let rect = Rect::new(100., 100., 800., 600.);
        app.canvas_views.insert(
            id,
            View {
                rect,
                ..Default::default()
            },
        );
        app.pane_bounds = vec![(id, rect)];
        app.pointer = [500., 400.];
        assert_eq!(app.canvas_target(), Some(id));
        app.window_first_frame = true;
        assert!(
            !app.window_restore_waits(),
            "a canvas never waits for a PTY grid on restart"
        );

        assert!(app.key_input(&chrome_input("f").unwrap()).is_none());
        let fitted = Camera::fit(
            canvas.elements().filter_map(|e| e.content.bounds()),
            [800., 600.],
            32.,
        )
        .unwrap();
        assert_eq!(app.canvas_views[&id].camera, fitted);
        use winit::event::TouchPhase as Phase;
        let device_id = winit::event::DeviceId::dummy();
        assert!(app.diagram_gesture(&WindowEvent::PinchGesture {
            device_id,
            delta: 0.5,
            phase: Phase::Moved
        }));
        assert_eq!(app.canvas_views[&id].camera.zoom, fitted.zoom * 1.5);
        let before = app.canvas_views[&id].camera;
        assert!(app.diagram_gesture(&WindowEvent::PanGesture {
            device_id,
            delta: winit::dpi::PhysicalPosition::new(20., -10.),
            phase: Phase::Moved
        }));
        assert_ne!(app.canvas_views[&id].camera.center, before.center);
        assert!(app.diagram_gesture(&WindowEvent::DoubleTapGesture { device_id }));
        assert!((app.canvas_views[&id].camera.zoom - 1.).abs() < 1e-12);
        let smart_zoom = app.canvas_views[&id].camera;
        assert!(!app.canvas_paints().is_empty());
        assert_eq!(
            app.canvas_views[&id].camera, smart_zoom,
            "painting preserves manual smart zoom"
        );

        assert!(app.diagram_gesture(&WindowEvent::MouseInput {
            device_id,
            state: ElementState::Pressed,
            button: MouseButton::Middle
        }));
        let before = app.canvas_views[&id].camera;
        assert!(app.diagram_gesture(&WindowEvent::CursorMoved {
            device_id,
            position: winit::dpi::PhysicalPosition::new(540., 420.)
        }));
        assert_ne!(app.canvas_views[&id].camera.center, before.center);
        assert!(app.diagram_gesture(&WindowEvent::MouseInput {
            device_id,
            state: ElementState::Released,
            button: MouseButton::Middle
        }));
        let stopped = app.canvas_views[&id].camera;
        assert!(!app.diagram_gesture(&WindowEvent::CursorMoved {
            device_id,
            position: winit::dpi::PhysicalPosition::new(600., 460.)
        }));
        assert_eq!(app.canvas_views[&id].camera, stopped);

        let touch = |id, phase, x, y| {
            WindowEvent::Touch(winit::event::Touch {
                device_id,
                id,
                phase,
                location: winit::dpi::PhysicalPosition::new(x, y),
                force: None,
            })
        };
        assert!(app.diagram_gesture(&touch(1, Phase::Started, 300., 300.)));
        assert!(app.diagram_gesture(&touch(2, Phase::Started, 500., 300.)));
        assert!(app.diagram_gesture(&touch(2, Phase::Moved, 700., 300.)));
        assert!((app.canvas_views[&id].camera.zoom - 2.).abs() < 1e-12);
        assert!(
            !app.app_overlays.open(),
            "canvas contacts never open or close a modal"
        );
        assert!(!app.diagram_gesture(&WindowEvent::Focused(false)));
        assert!(!app.canvas_views[&id].touching());
        assert!(app.key_input(&chrome_input("f").unwrap()).is_none());
        assert_eq!(app.canvas_views[&id].camera, fitted);
        let x = chrome_input("a").unwrap();
        assert!(app.key_input(&x).is_none());
        // The existing key owner keeps repeats/releases local after focus moves.
        assert!(
            app.keyboard
                .suppressed(x.physical_key, ElementState::Pressed, true)
        );
        assert!(
            app.keyboard
                .suppressed(x.physical_key, ElementState::Released, false)
        );

        // A native pan over the sidebar must leave the focused canvas intact.
        let before = app.canvas_views[&id].camera;
        app.pointer = [30., 30.];
        assert!(!app.diagram_gesture(&WindowEvent::PanGesture {
            device_id,
            delta: winit::dpi::PhysicalPosition::new(20., -10.),
            phase: Phase::Moved
        }));
        assert_eq!(app.canvas_views[&id].camera, before);
        app.pointer = [500., 400.];

        // A release delivered after focus leaves A must not survive A -> other
        // pane -> A as a phantom drag or a permanently held contact.
        let other = SeatId::from_u128(99);
        assert!(app.diagram_gesture(&WindowEvent::MouseInput {
            device_id,
            state: ElementState::Pressed,
            button: MouseButton::Middle
        }));
        app.focus.stage.as_mut().unwrap().focused = Some(other);
        assert!(!app.diagram_gesture(&WindowEvent::MouseInput {
            device_id,
            state: ElementState::Released,
            button: MouseButton::Middle
        }));
        assert!(app.canvas_views[&id].drag.is_none());
        app.focus.stage.as_mut().unwrap().focused = Some(id);
        assert!(!app.diagram_gesture(&WindowEvent::CursorMoved {
            device_id,
            position: winit::dpi::PhysicalPosition::new(600., 460.)
        }));
        assert_eq!(app.canvas_views[&id].camera, before);
        assert!(app.diagram_gesture(&touch(3, Phase::Started, 300., 300.)));
        assert!(app.canvas_views[&id].touching());
        app.focus.stage.as_mut().unwrap().focused = Some(other);
        assert!(!app.diagram_gesture(&touch(3, Phase::Ended, 300., 300.)));
        assert!(!app.canvas_views[&id].touching());
        app.focus.stage.as_mut().unwrap().focused = Some(id);
        assert!(app.diagram_gesture(&WindowEvent::PinchGesture {
            device_id,
            delta: 0.5,
            phase: Phase::Moved
        }));
        assert_eq!(app.canvas_views[&id].camera.zoom, before.zoom * 1.5);
        assert!(app.key_input(&chrome_input("f").unwrap()).is_none());

        // Opening an expanded diagram also retires canvas-owned contacts.
        assert!(app.diagram_gesture(&touch(4, Phase::Started, 300., 300.)));
        app.open_diagram(Arc::new(bbt_ui::components::Layout {
            rect: Rect::new(0., 0., 800., 600.),
            baseline: 0.,
            kind: bbt_ui::components::LayoutKind::Named("canvas/test-diagram".into()),
            children: vec![],
        }));
        assert!(app.diagram_gesture(&WindowEvent::PinchGesture {
            device_id,
            delta: 0.5,
            phase: Phase::Moved
        }));
        assert!(!app.canvas_views[&id].touching());
        app.close_app_overlay();

        let model = app.live_state.as_ref().unwrap();
        let (watched, sizes) = pane_controls(
            model,
            &app.pane_bounds,
            &[],
            &BTreeMap::new(),
            Some(id),
            1.,
            None,
        );
        assert!(watched.is_empty());
        assert!(sizes.is_empty());
        assert!(!app.pending_commands.iter().any(|m| matches!(
            m,
            live::UiMsg::Input { .. } | live::UiMsg::Resize { .. } | live::UiMsg::Watched(_)
        )));

        // The same five-tag canvas sidebar must fit the existing framed GUI
        // protocol's JSON decoder, not only serialize as a bare layout tree.
        app.scene.set_model(model, &model.ui.window.stage);
        let layouts = app.scene.layout([1280., 900.], false).unwrap();
        let layout = app.ui_layout([1280., 900.], &layouts, None).unwrap();
        let request = bbt_proto::GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: [0; 32], // Serialization witness only, never an admission.
            action: bbt_proto::GuiAction::UiReply {
                id: bbt_mux::Ulid(1),
                result: Box::new(Ok(layout.clone())),
            },
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        serde_json::from_slice::<bbt_proto::GuiRequest>(&bytes).unwrap_or_else(|error| {
            panic!(
                "canvas GUI request roundtrip ({} bytes): {error}",
                bytes.len()
            )
        });
        let reply = bbt_proto::GuiReply::Ui {
            op: bbt_mux::Ulid(1),
            layout,
        };
        let bytes = serde_json::to_vec(&reply).unwrap();
        serde_json::from_slice::<bbt_proto::GuiReply>(&bytes).unwrap_or_else(|error| {
            panic!(
                "canvas GUI reply roundtrip ({} bytes): {error}",
                bytes.len()
            )
        });
        app.document.open = true;
        assert!(!app.diagram_gesture(&WindowEvent::PinchGesture {
            device_id,
            delta: 0.5,
            phase: Phase::Moved
        }));
        assert_eq!(
            app.canvas_views[&id].camera, fitted,
            "a document keeps canvas gestures suspended"
        );
    }
}
