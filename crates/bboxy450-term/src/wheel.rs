// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native and headless wheel dispatch share this exact hit-test and admission path.
use crate::*;
use bbt_core::term::TermMode;

// One native event emits at most one bounded input command, including a full-page fling.
const MAX_WHEEL_REPORTS: usize = 256;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Target {
    History,
    Mouse(u64),
    Cursor(bool),
}
impl Target {
    fn name(self) -> &'static str {
        match self {
            Self::History => "pane_history",
            Self::Mouse(_) => "program_mouse",
            Self::Cursor(_) => "program_cursor",
        }
    }
}
pub(super) struct Intent {
    view: View,
    target: Target,
    remainder: f64,
}

fn target(modes: u64, shift: bool, historical: bool) -> Target {
    let has = |mode: TermMode| modes & u64::from(mode.bits()) != 0;
    if shift || historical {
        Target::History
    } else if has(TermMode::MOUSE_MODE) {
        Target::Mouse(
            modes
                & u64::from(
                    (TermMode::MOUSE_MODE | TermMode::SGR_MOUSE | TermMode::UTF8_MOUSE).bits(),
                ),
        )
    } else if has(TermMode::ALT_SCREEN) && has(TermMode::ALTERNATE_SCROLL) {
        Target::Cursor(has(TermMode::APP_CURSOR))
    } else {
        Target::History
    }
}

// Independently implemented from the XTerm control-sequence protocol (wheel buttons,
// normal/UTF-8/SGR coordinates): https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
// Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
fn mouse_report(
    modes: u64,
    up: bool,
    position: [usize; 2],
    modifiers: ModifiersState,
) -> Option<Vec<u8>> {
    if position.contains(&0) {
        return None;
    }
    let code = (if up { 64 } else { 65 })
        + u8::from(modifiers.alt_key()) * 8
        + u8::from(modifiers.control_key()) * 16;
    if modes & u64::from(TermMode::SGR_MOUSE.bits()) != 0 {
        return Some(format!("\x1b[<{code};{};{}M", position[0], position[1]).into_bytes());
    }
    let utf8 = modes & u64::from(TermMode::UTF8_MOUSE.bits()) != 0;
    let limit = if utf8 { 2015 } else { 223 };
    if position.iter().any(|p| *p == 0 || *p > limit) {
        return None;
    }
    let mut bytes = b"\x1b[M".to_vec();
    for value in [usize::from(code), position[0], position[1]] {
        if utf8 {
            bytes.extend_from_slice(
                char::from_u32((value + 32) as u32)?
                    .encode_utf8(&mut [0; 4])
                    .as_bytes(),
            );
        } else {
            bytes.push((value + 32) as u8);
        }
    }
    Some(bytes)
}

impl App {
    pub(super) fn verify_wheel(&mut self) {
        if !cfg!(debug_assertions)
            || !self.test_account
            || self
                .live_state
                .as_ref()
                .and_then(|m| m.identity.as_ref())
                .and_then(|i| i.private_test_run)
                .is_none()
        {
            self.logger
                .write("wheel verification refused: private debug test account required");
            return;
        }
        let saved = self.pointer;
        let centers: Vec<_> = self
            .focus_bounds
            .iter()
            .map(|(_, r)| [r.x + r.w / 2., r.y + r.h / 2.])
            .collect();
        for center in centers {
            self.pointer = center;
            self.mouse_wheel(MouseScrollDelta::LineDelta(0., 3.));
        }
        self.pointer = saved;
    }

    pub(super) fn mouse_wheel(&mut self, delta: MouseScrollDelta) {
        if self.rich_wheel(&delta) { return; }
        let previous = self.scroll_wheel.take();
        match () {
            () if self.app_overlays.open() => {
                self.wheel_consumed("overlay");
                if let Some(diagram) = &mut self.app_overlays.diagram {
                    let delta = match delta {
                        MouseScrollDelta::LineDelta(x, y) => [-x * 28., -y * 28.],
                        MouseScrollDelta::PixelDelta(p) => {
                            let scale = self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
                            [-p.x as f32 / scale, -p.y as f32 / scale]
                        }
                    };
                    if (self.modifiers.super_key() || self.modifiers.control_key()) && delta[1] != 0. {
                        diagram.act(if delta[1] < 0. { bbt_ui::modules::document_modal::viewer::Action::In } else { bbt_ui::modules::document_modal::viewer::Action::Out });
                    } else { diagram.pan(delta); }
                    self.app_overlays.layout = None;
                    self.request_redraw();
                }
            }
            () if self.launcher.is_some() => {
                self.wheel_consumed("launcher");
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 28.,
                    MouseScrollDelta::PixelDelta(p) => {
                        -p.y as f32 / self.window.as_ref().map_or(1., |w| w.scale_factor() as f32)
                    }
                };
                self.scroll_launcher(dy);
            }
            () if self.document.open => {
                self.wheel_consumed("document");
                let [mut dx, mut dy] = match delta {
                    MouseScrollDelta::LineDelta(x, y) => [-x * 28., -y * 28.],
                    MouseScrollDelta::PixelDelta(p) => {
                        let scale = self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
                        [-p.x as f32 / scale, -p.y as f32 / scale]
                    }
                };
                if self.modifiers.shift_key() && dx == 0. {
                    dx = dy;
                    dy = 0.;
                }
                if let Some(layout) = &self.document_layout {
                    let horizontal = layout.regions.horizontal_at(self.pointer).cloned();
                    let extent = layout.regions.extent;
                    let root_dx = if horizontal.as_ref().is_some_and(|r| r.root) {
                        dx
                    } else {
                        0.
                    };
                    let mut changed = self.document.scroll.scroll([root_dx, dy], extent);
                    if let Some(region) = horizontal.filter(|r| !r.root) {
                        changed |= self.document.scroll.pan(&region.key, dx, region.max);
                    }
                    if changed {
                        self.document_layout = None;
                        self.request_redraw();
                    }
                }
            }
            () if self.launcher.is_none()
                && self.sidebar_viewport.is_some_and(|(r, _)| {
                    self.pointer[0] >= r.x
                        && self.pointer[0] < r.right()
                        && self.pointer[1] >= r.y
                        && self.pointer[1] < r.bottom()
                }) =>
            {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 28.,
                    MouseScrollDelta::PixelDelta(p) => {
                        -p.y as f32 / self.window.as_ref().map_or(1., |w| w.scale_factor() as f32)
                    }
                };
                self.wheel_consumed("sidebar");
                self.scroll_sidebar(dy);
            }
            () if self.launcher.is_none()
                && self
                    .local_page_layout
                    .as_ref()
                    .is_some_and(|p| p.contains(self.pointer)) =>
            {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 28.,
                    MouseScrollDelta::PixelDelta(p) => {
                        -p.y as f32 / self.window.as_ref().map_or(1., |w| w.scale_factor() as f32)
                    }
                };
                self.wheel_consumed("local_scroll");
                self.local_scroll(delta);
            }
            () if self.launcher.is_none()
                && self
                    .local_page_layout
                    .as_ref()
                    .is_some_and(|p| p.focuses(self.pointer)) =>
            {
                self.wheel_consumed("local_focus");
            }
            () if self.launcher.is_none()
                && self.conversation_layout.as_ref().is_some_and(|layout| {
                    let r = layout.regions.body;
                    self.pointer[0] >= r.x
                        && self.pointer[0] < r.right()
                        && self.pointer[1] >= r.y
                        && self.pointer[1] < r.bottom()
                }) =>
            {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * 28.,
                    MouseScrollDelta::PixelDelta(p) => {
                        -p.y as f32 / self.window.as_ref().map_or(1., |w| w.scale_factor() as f32)
                    }
                };
                self.wheel_consumed("conversation_body");
                self.scroll_composer_body(dy);
            }
            () if self.launcher.is_none()
                && self.conversation_layout.as_ref().is_some_and(|layout| {
                    layout.actions.contains("conversation/rows", self.pointer)
                        || layout.actions.contains("conversation/tasks", self.pointer)
                }) =>
            {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y * 28.,
                    MouseScrollDelta::PixelDelta(p) => {
                        p.y as f32 / self.window.as_ref().map_or(1., |w| w.scale_factor() as f32)
                    }
                };
                let tasks = self.conversation_layout.as_ref().is_some_and(|layout| {
                    layout.actions.contains("conversation/tasks", self.pointer)
                });
                self.wheel_consumed(if tasks {
                    "conversation_tasks"
                } else {
                    "conversation_rows"
                });
                let _ = self.scroll_sidebar_section(tasks, -dy);
            }
            () if self.launcher.is_none() => {
                if let Some((seat, rect)) = self.focus_bounds.iter().find(|(_, rect)| {
                    self.pointer[0] >= rect.x
                        && self.pointer[0] < rect.right()
                        && self.pointer[1] >= rect.y
                        && self.pointer[1] < rect.bottom()
                }) {
                    let seat = *seat;
                    let rect = *rect;
                    let Some((view, screen, historical, cell)) =
                        self.live_state.as_ref().and_then(|model| {
                            let generation = model.ui.seats.get(&seat)?.process_generation;
                            let (view, _) = *model.screen_baselines.get(&seat)?;
                            if view.generation != generation {
                                return None;
                            }
                            let screen = model.screens.get(&seat)?.clone();
                            let scale =
                                self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
                            let cell = model
                                .terminal_cell_pixels
                                .map(|cell| cell.map(|v| v as f32 / scale));
                            Some((view, screen, model.history.contains_key(&seat), cell))
                        })
                    else {
                        self.wheel_consumed("pane_unavailable");
                        return;
                    };
                    let lines = match delta {
                        MouseScrollDelta::LineDelta(_, y) => f64::from(y),
                        MouseScrollDelta::PixelDelta(position) => {
                            let scale = self.window.as_ref().map_or(1., |w| w.scale_factor());
                            position.y
                                / (f64::from(rect.h) / screen.rows.max(1) as f64 * scale).max(1.)
                        }
                    };
                    if !lines.is_finite() {
                        self.wheel_consumed("nonfinite");
                        return;
                    }
                    let body = self
                        .pane_bounds
                        .iter()
                        .find(|(id, _)| *id == seat)
                        .map(|(_, r)| *r)
                        .filter(|r| {
                            self.pointer[0] >= r.x
                                && self.pointer[0] < r.right()
                                && self.pointer[1] >= r.y
                                && self.pointer[1] < r.bottom()
                        });
                    let target = if body.is_some() {
                        target(screen.modes, self.modifiers.shift_key(), historical)
                    } else {
                        Target::History
                    };
                    let mut intent = previous
                        .filter(|p| p.view == view && p.target == target)
                        .unwrap_or(Intent {
                            view,
                            target,
                            remainder: 0.,
                        });
                    intent.remainder += lines;
                    let whole = intent.remainder.trunc() as i64;
                    intent.remainder = intent.remainder.fract();
                    self.scroll_wheel = Some(intent);
                    if whole == 0 {
                        self.wheel_consumed(target.name());
                        return;
                    }
                    let report = match target {
                        Target::History => {
                            self.wheel_consumed(target.name());
                            self.scroll_pane(seat, ScrollRequest::lines(whole));
                            return;
                        }
                        Target::Cursor(application) => {
                            keyboard::alternate_scroll(whole > 0, application)
                        }
                        Target::Mouse(modes) => {
                            let body = body.expect("program wheel is inside the terminal body");
                            let cell = cell.unwrap_or([
                                body.w / screen.cols.max(1) as f32,
                                body.h / screen.rows.max(1) as f32,
                            ]);
                            let position = [
                                (((self.pointer[0] - body.x) / cell[0].max(1.)).floor() as usize
                                    + 1)
                                .min(screen.cols.max(1)),
                                (((self.pointer[1] - body.y) / cell[1].max(1.)).floor() as usize
                                    + 1)
                                .min(screen.rows.max(1)),
                            ];
                            let Some(report) =
                                mouse_report(modes, whole > 0, position, self.modifiers)
                            else {
                                self.wheel_consumed("program_mouse_coordinate_bound");
                                return;
                            };
                            report
                        }
                    };
                    self.wheel_consumed(target.name());
                    self.terminal_selection.clear();
                    self.queue(live::UiMsg::Input {
                        seat,
                        generation: view.generation,
                        id: bbt_mux::Ulid::new(),
                        bytes: report
                            .repeat(whole.unsigned_abs().min(MAX_WHEEL_REPORTS as u64) as usize),
                    });
                } else {
                    self.wheel_consumed("no_pane");
                }
            }
            _ => {}
        }
    }

    fn wheel_consumed(&mut self, route: &'static str) {
        if let Some(line) = self.wheel_note(route, Instant::now()) {
            self.logger.write(line);
        }
    }

    fn wheel_note(&mut self, route: &'static str, now: Instant) -> Option<String> {
        if self
            .wheel_log_at
            .is_some_and(|at| now.duration_since(at) < Duration::from_secs(1))
        {
            return None;
        }
        self.wheel_log_at = Some(now);
        let view = self.scroll_wheel.as_ref().map(|intent| intent.view);
        let seat = view.map(|v| SeatId::from_u128(v.seat.0));
        let modes = seat.and_then(|seat| {
            self.live_state
                .as_ref()?
                .screens
                .get(&seat)
                .map(|s| s.modes)
        });
        Some(format!(
            "wheel route={route} pointer={:.0},{:.0} seat={} generation={} modes={:#x} panes={} model_seats={}",
            self.pointer[0],
            self.pointer[1],
            seat.map_or_else(|| "none".into(), |seat| seat.to_string()),
            view.map_or(0, |view| view.generation),
            modes.unwrap_or(0),
            self.focus_bounds.len(),
            self.live_state.as_ref().map_or(0, |m| m.ui.seats.len())
        ))
    }
}

#[cfg(test)]
mod scroll_input_tests {
    use super::*;
    use bboxy450_term::{grids::Grids, model, scrollback::History};
    use bbt_proto::{GuiAction, Screen, SeatRecord, gui::HistoryRecord};

    fn mouse_app() -> App {
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        app.live_commands = Some(live::disconnected_commands());
        app.backend_lost = true;
        let seat = SeatId::from_u128(1);
        let mut term = bbt_core::Term::new(80, 24, 1000);
        term.advance(b"line 1\r\nline 2\r\n\x1b[?1049h\x1b[?1000h\x1b[?1006h");
        let screen = Arc::new(Screen::from_term(&term, String::new(), None, None));
        let record = SeatRecord::new(bbt_mux::Ulid(1), bbt_mux::Ulid(90), "mouse", "zsh");
        let mut current = model::build_at(
            &[record],
            &Grids::new(),
            &Default::default(),
            Instant::now(),
            1000,
        );
        let view = View {
            seat: bbt_mux::Ulid(1),
            generation: current.ui.seats[&seat].process_generation,
            view_generation: bbt_mux::Ulid::new(),
        };
        current.screens.insert(seat, screen);
        current.screen_baselines.insert(seat, (view, 1));
        app.live_state = Some(Arc::new(current));
        let pane = (seat, Rect::new(300., 0., 800., 480.));
        app.focus_bounds = vec![pane];
        app.pane_bounds = vec![pane];
        app.pointer = [700., 240.];
        app
    }

    #[test]
    fn wheel_reaches_the_mouse_reporting_program_after_alternate_screen_entry() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "wheel::scroll_input_tests::wheel_reaches_the_mouse_reporting_program_after_alternate_screen_entry",
        ) {
            return;
        }
        let mut app = mouse_app();
        app.mouse_wheel(MouseScrollDelta::LineDelta(0., 3.));
        match app
            .pending_commands
            .pop_front()
            .expect("wheel must have an owner")
        {
            live::UiMsg::Input { seat, bytes, .. } => {
                assert_eq!(seat, SeatId::from_u128(1));
                assert_eq!(bytes, b"\x1b[<64;41;13M".repeat(3));
            }
            _ => panic!(
                "mouse-reporting TUI received no wheel input; alternate-screen replay cannot scroll its application history"
            ),
        }
    }

    fn set_modes(app: &mut App, modes: u64) {
        let model = Arc::make_mut(app.live_state.as_mut().unwrap());
        Arc::make_mut(model.screens.get_mut(&SeatId::from_u128(1)).unwrap()).modes = modes;
    }

    #[test]
    fn real_wheel_modes_shift_history_and_generation_changes_keep_one_owner() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "wheel::scroll_input_tests::real_wheel_modes_shift_history_and_generation_changes_keep_one_owner",
        ) {
            return;
        }
        let mut app = mouse_app();
        let bits = |m: TermMode| u64::from(m.bits());
        let mouse = bits(TermMode::MOUSE_REPORT_CLICK | TermMode::SGR_MOUSE | TermMode::ALT_SCREEN);
        for (modes, shift, expected) in [
            (0, false, None),
            (mouse, true, None),
            (bits(TermMode::SGR_MOUSE), false, None),
            (bits(TermMode::ALT_SCREEN), false, None),
            (
                bits(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL),
                false,
                Some(b"\x1b[A".to_vec()),
            ),
            (
                bits(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR),
                false,
                Some(b"\x1bOA".to_vec()),
            ),
            (mouse, false, Some(b"\x1b[<64;41;13M".to_vec())),
        ] {
            set_modes(&mut app, modes);
            app.modifiers.set(ModifiersState::SHIFT, shift);
            app.mouse_wheel(MouseScrollDelta::LineDelta(0., 1.));
            let message = app.pending_commands.pop_front().unwrap();
            if let Some(expected) = expected {
                assert!(matches!(message,live::UiMsg::Input {bytes,..} if bytes==expected));
            } else {
                assert!(
                    matches!(message,live::UiMsg::Scroll {request,..} if request==ScrollRequest::lines(1))
                );
            }
            assert!(app.pending_commands.is_empty());
        }
        app.modifiers = ModifiersState::empty();
        set_modes(&mut app, mouse);
        app.mouse_wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0., 8.),
        ));
        set_modes(&mut app, mouse | bits(TermMode::SHOW_CURSOR));
        app.mouse_wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0., 12.),
        ));
        assert!(
            matches!(
                app.pending_commands.pop_front(),
                Some(live::UiMsg::Input { .. })
            ),
            "cursor visibility cannot reset a smooth wheel gesture"
        );
        {
            let model = Arc::make_mut(app.live_state.as_mut().unwrap());
            model.history.insert(
                SeatId::from_u128(1),
                Arc::new(bboxy450_term::scrollback::Historical {
                    screen: model.screens[&SeatId::from_u128(1)].clone(),
                    gaps: vec![],
                    top_row: 0,
                }),
            );
        }
        app.mouse_wheel(MouseScrollDelta::LineDelta(0., 1.));
        assert!(matches!(
            app.pending_commands.pop_front(),
            Some(live::UiMsg::Scroll { .. })
        ));
        Arc::make_mut(app.live_state.as_mut().unwrap())
            .history
            .clear();
        // A -> B -> A ownership and view changes cannot combine fractional deltas.
        app.mouse_wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0., 8.),
        ));
        app.modifiers = ModifiersState::SHIFT;
        app.mouse_wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0., 8.),
        ));
        app.modifiers = ModifiersState::empty();
        app.mouse_wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0., 14.),
        ));
        assert!(app.pending_commands.is_empty());
        app.mouse_wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(0., 8.),
        ));
        assert!(
            matches!(app.pending_commands.pop_front(),Some(live::UiMsg::Input {bytes,..}) if bytes==b"\x1b[<64;41;13M")
        );
        let seat = SeatId::from_u128(1);
        let model = Arc::make_mut(app.live_state.as_mut().unwrap());
        model.ui.seats.get_mut(&seat).unwrap().process_generation += 1;
        app.mouse_wheel(MouseScrollDelta::LineDelta(0., 3.));
        assert!(
            app.pending_commands.is_empty(),
            "an old screen cannot select input for a new generation"
        );
        assert!(app.scroll_wheel.is_none());
    }

    #[test]
    fn extreme_and_nonfinite_wheel_deltas_never_make_unbounded_input_or_remainder_debt() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "wheel::scroll_input_tests::extreme_and_nonfinite_wheel_deltas_never_make_unbounded_input_or_remainder_debt",
        ) {
            return;
        }
        let mut app = mouse_app();
        app.mouse_wheel(MouseScrollDelta::LineDelta(0., f32::MAX));
        assert!(
            matches!(app.pending_commands.pop_front(),Some(live::UiMsg::Input {bytes,..}) if bytes==b"\x1b[<64;41;13M".repeat(MAX_WHEEL_REPORTS))
        );
        app.mouse_wheel(MouseScrollDelta::LineDelta(0., -1.));
        assert!(
            matches!(app.pending_commands.pop_front(),Some(live::UiMsg::Input {bytes,..}) if bytes==b"\x1b[<65;41;13M")
        );
        app.mouse_wheel(MouseScrollDelta::LineDelta(0., f32::NAN));
        assert!(app.pending_commands.is_empty());
        assert!(app.scroll_wheel.is_none());
    }

    #[test]
    fn invisible_overlay_consumption_is_named_bounded_and_rate_limited() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "wheel::scroll_input_tests::invisible_overlay_consumption_is_named_bounded_and_rate_limited",
        ) {
            return;
        }
        let mut app = mouse_app();
        app.open_context(
            app_overlays::Target::Seat(SeatId::from_u128(1)),
            [700., 240.],
        );
        assert!(app.app_overlays.open());
        assert!(app.app_overlays.layout.is_none());
        app.mouse_wheel(MouseScrollDelta::LineDelta(0., 3.));
        assert!(app.pending_commands.is_empty());
        let at = app
            .wheel_log_at
            .expect("even an unsolved covering surface is diagnosed");
        assert!(
            app.wheel_note("overlay", at + Duration::from_millis(999))
                .is_none()
        );
        let line = app
            .wheel_note("overlay", at + Duration::from_secs(1))
            .unwrap();
        assert!(line.starts_with("wheel route=overlay "));
        assert!(line.len() < 512);
        assert!(
            app.wheel_note("no_pane", at + Duration::from_secs(1))
                .is_none()
        );
    }

    #[test]
    fn mouse_protocols_encode_coordinates_modifiers_and_bounds() {
        let bits = |m: TermMode| u64::from(m.bits());
        assert_eq!(
            mouse_report(0, true, [41, 13], ModifiersState::empty()).unwrap(),
            [27, 91, 77, 96, 73, 45]
        );
        assert!(mouse_report(0, true, [224, 1], ModifiersState::empty()).is_none());
        assert!(
            mouse_report(
                bits(TermMode::SGR_MOUSE),
                true,
                [0, 1],
                ModifiersState::empty()
            )
            .is_none()
        );
        assert_eq!(
            mouse_report(
                bits(TermMode::UTF8_MOUSE),
                true,
                [200, 240],
                ModifiersState::empty()
            )
            .unwrap(),
            "\x1b[M`\u{e8}\u{110}".as_bytes()
        );
        assert_eq!(
            mouse_report(
                bits(TermMode::SGR_MOUSE),
                false,
                [512, 256],
                ModifiersState::ALT | ModifiersState::CONTROL
            )
            .unwrap(),
            b"\x1b[<89;512;256M"
        );
    }

    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "isolated headless witness joins its private history worker; production UI only admits the wheel intent"
    )]
    fn wheel_replays_aged_history_through_native_dispatch() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "wheel::scroll_input_tests::wheel_replays_aged_history_through_native_dispatch",
        ) {
            return;
        }
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        app.live_commands = Some(live::disconnected_commands());
        app.backend_lost = true;
        let seat = SeatId::from_u128(1);
        let bytes: Vec<u8> = (1..=100_000)
            .flat_map(|n| format!("line {n:06}\r\n").into_bytes())
            .collect();
        let mut term = bbt_core::Term::new(80, 24, 0);
        term.advance(&bytes);
        let screen = Arc::new(Screen::from_term(&term, String::new(), None, None));
        let record = SeatRecord::new(bbt_mux::Ulid(1), bbt_mux::Ulid(90), "wheel", "zsh");
        let mut current = model::build_at(
            &[record],
            &Grids::new(),
            &Default::default(),
            Instant::now(),
            1000,
        );
        current.screens.insert(seat, screen.clone());
        let generation = current.ui.seats[&seat].process_generation;
        current.screen_baselines.insert(
            seat,
            (
                View {
                    seat: bbt_mux::Ulid(1),
                    generation,
                    view_generation: bbt_mux::Ulid::new(),
                },
                1,
            ),
        );
        app.live_state = Some(Arc::new(current));
        app.focus_bounds = vec![(seat, Rect::new(300., 0., 800., 480.))];
        app.pointer = [700., 240.];
        let view = View {
            seat: bbt_mux::Ulid(1),
            generation,
            view_generation: bbt_mux::Ulid::new(),
        };
        let records: Vec<_> = bytes
            .chunks(16_000)
            .enumerate()
            .map(|(n, bytes)| HistoryRecord::Bytes {
                sequence: n as u64 + 1,
                bytes: bytes.to_vec(),
            })
            .collect();
        let started = Instant::now();
        app.mouse_wheel(MouseScrollDelta::LineDelta(0., 3.));
        let Some(live::UiMsg::Scroll { request, .. }) = app.pending_commands.pop_front() else {
            panic!("wheel never reached pane");
        };
        let row = std::thread::spawn(move || {
            let mut history = History::default();
            history
                .request(seat, view, records.len() as u64, screen, request)
                .unwrap();
            for _ in 0..100_000 {
                if let Some(GuiAction::GetHistory { from, to, .. }) =
                    history.next_action(Instant::now())
                {
                    let end = (from + 3).min(to);
                    history
                        .accept(
                            seat,
                            records[(from - 1) as usize..end as usize].to_vec(),
                            (end < to).then_some(end + 1),
                        )
                        .unwrap();
                }
                if let Some((_, result)) = history.step()
                    && let Some(display) = result.unwrap()
                {
                    eprintln!(
                        "wheel history first display after {:?}: row={} line={}",
                        started.elapsed(),
                        display.top_row,
                        display.screen.cells[0]
                            .iter()
                            .map(|c| c.c)
                            .collect::<String>()
                    );
                    return display.top_row;
                }
            }
            panic!(
                "wheel never changed the scroll offset: {}",
                history.metrics(&Default::default())
            );
        })
        .join()
        .expect("private history worker");
        assert_eq!(row, 100_001 - 24 - 3);
    }
}
