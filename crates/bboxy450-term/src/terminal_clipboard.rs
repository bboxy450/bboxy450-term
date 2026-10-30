// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native and scripted terminal selection use the same displayed cells and clipboard port.
use crate::*;
use bboxy450_term::terminal_selection::Display;

impl App {
    /// Private accounts never read the operator's pasteboard, including menu actions.
    pub(super) fn clipboard_content(
        &self,
    ) -> Result<Option<bboxy450_term_desktop::ClipboardContent>, bboxy450_term_desktop::Error> {
        if self.test_account {
            Ok(self.private_clipboard.clone().map(bboxy450_term_desktop::ClipboardContent::Text))
        } else {
            bboxy450_term_desktop::read_clipboard()
        }
    }
    /// Paste into the focused live process using the same framing as ctl seat paste.
    /// Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    pub(super) fn terminal_paste(&mut self) {
        let Some(seat) = self.focus.stage.as_ref().and_then(|stage| stage.focused) else {
            return;
        };
        if !self.terminal_context(seat) {
            return;
        }
        let Some((generation, modes)) = self.live_state.as_ref().and_then(|model| {
            Some((model.ui.seats.get(&seat)?.process_generation, model.screens.get(&seat)?.modes))
        }) else {
            return;
        };
        match self.clipboard_content() {
            Ok(Some(bboxy450_term_desktop::ClipboardContent::Text(text))) if !text.is_empty() => {
                self.queue(live::UiMsg::Input {
                    seat,
                    generation,
                    id: bbt_mux::Ulid::new(),
                    bytes: bboxy450_term::ctl::bracketed(text.as_bytes(), modes),
                });
            }
            Err(error) => self.logger.write(format!("terminal paste: {error}")),
            _ => {}
        }
    }
    /// Build the source/geometry that this frame will actually encode, including a retained
    /// old live frame during font resize. Historical fallback without its row is unavailable.
    fn prepared_terminal_display(&self, seat: SeatId) -> Option<Display> {
        let model = self.live_state.as_ref()?;
        let body = self.pane_bounds.iter().find(|(id, _)| *id == seat)?.1;
        let prepared = model.prepared_screens.get(&seat)?;
        let frame = model.frames.get(&seat)?;
        if body.w <= 0. || body.h <= 0. {
            return None;
        }
        let screen = &prepared.screen;
        let (view, historical_row) = match prepared.baseline {
            Some((view, _)) => (view, None),
            None => {
                let history = model.history.get(&seat)?;
                if !Arc::ptr_eq(screen, &history.screen) {
                    return None;
                }
                (model.screen_baselines.get(&seat)?.0, Some(history.top_row))
            }
        };
        let scale = self.window.as_ref().map_or(1., |w| w.scale_factor() as f32);
        let cell = [
            frame.width as f32 / screen.cols as f32 / scale,
            frame.height as f32 / screen.rows as f32 / scale,
        ];
        Some(Display {
            seat,
            view,
            screen: screen.clone(),
            cell,
            historical_row,
            rect: Rect::new(
                body.x,
                body.y,
                body.w.min(frame.width as f32 / scale),
                body.h.min(frame.height as f32 / scale),
            ),
        })
    }
    fn terminal_context(&self, seat: SeatId) -> bool {
        if self.app_overlays.open()
            || self.document.open
            || self.launcher.is_some()
            || self.local_tab.is_some()
            || self.composer.view.focus
            || self.reject_focus
        {
            return false;
        }
        self.live_state.as_ref().is_some_and(|model| {
            self.focus
                .stage
                .as_ref()
                .unwrap_or(&model.ui.window.stage)
                .focused
                == Some(seat)
        })
    }
    /// Input reads the last successful frame, never a newer not-yet-drawn publication.
    pub(super) fn terminal_display(&self, seat: SeatId) -> Option<Display> {
        if !self.terminal_context(seat) {
            return None;
        }
        let model = self.live_state.as_ref()?;
        let drawn = self.frame_observations.displayed(seat)?;
        let view = model.screen_baselines.get(&seat)?.0;
        let body = self.pane_bounds.iter().find(|(id, _)| *id == seat)?.1;
        if drawn.view != view
            || view.generation != model.ui.seats.get(&seat)?.process_generation
            || drawn.historical_row != model.history.get(&seat).map(|h| h.top_row)
            || drawn.rect.x != body.x
            || drawn.rect.y != body.y
            || drawn.rect.w > body.w
            || drawn.rect.h > body.h
        {
            return None;
        }
        Some(drawn.clone())
    }
    /// At most one shared screen Arc per visible terminal body; hidden panes retire on this
    /// successful frame. No history lines or second model snapshot are retained here.
    pub(super) fn remember_terminal_frame(&mut self) {
        let displays: Vec<_> = self
            .pane_bounds
            .iter()
            .filter_map(|(seat, _)| self.prepared_terminal_display(*seat))
            .collect();
        self.frame_observations.record_displayed(displays);
        if let Some(seat) = self.terminal_selection.seat() {
            if let Some(display) = self.terminal_display(seat) {
                self.terminal_selection.refresh(&display);
            } else {
                self.terminal_selection.clear();
            }
        }
    }
    pub(super) fn begin_terminal_selection(&mut self, seat: SeatId, at: [f32; 2]) {
        self.terminal_selection.clear();
        if let Some(display) = self.terminal_display(seat) {
            self.terminal_selection
                .begin(display, at, self.modifiers.shift_key());
        }
        self.request_redraw();
    }
    pub(super) fn extend_terminal_selection(&mut self, at: [f32; 2]) -> bool {
        if !self.terminal_selection.dragging {
            return false;
        }
        let changed = if let Some(display) = self
            .terminal_selection
            .seat()
            .and_then(|seat| self.terminal_display(seat))
        {
            self.terminal_selection.extend_changed(&display, at)
        } else {
            self.terminal_selection.clear();
            true
        };
        if changed {
            self.request_redraw();
        }
        true
    }
    pub(super) fn terminal_copy_text(&self) -> Option<String> {
        let display = self.terminal_display(self.terminal_selection.seat()?)?;
        self.terminal_selection.text(&display)
    }
    pub(super) fn copy_terminal_selection(&mut self) -> bool {
        let Some(text) = self
            .ui_selection
            .text()
            .or_else(|| if self.document.open {self.document.prepared.as_ref().map(|p|p.markdown.source.clone())} else {self.terminal_copy_text()})
        else {
            return false;
        };
        self.copy_text(text)
    }
    pub(super) fn copy_text(&mut self, text: String) -> bool {
        // Test-account windows have a private in-memory clipboard provider. Neither native
        // nor ctl tests can read or mutate the operator's general pasteboard.
        // Origin/credit: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
        let result = if self.test_account {
            self.private_clipboard = Some(text);
            Ok(())
        } else {
            bboxy450_term_desktop::write_clipboard_text(&text)
        };
        if let Err(error) = result {
            self.logger.write(format!("terminal copy: {error}"));
            return false;
        }
        true
    }
    pub(super) fn paint_terminal_selection(&mut self) -> Vec<bbt_ui::components::Paint> {
        let Some(display) = self
            .terminal_selection
            .seat()
            .filter(|seat| self.terminal_context(*seat))
            .and_then(|seat| self.prepared_terminal_display(seat))
        else {
            return vec![];
        };
        if !self.terminal_selection.refresh(&display) {
            return vec![];
        }
        let mut draw = bbt_ui_model::draw::DrawList::default();
        for rect in self.terminal_selection.rectangles(&display) {
            draw.rect(
                rect,
                0.,
                bbt_ui_model::draw::Color([0.25, 0.50, 0.95, 0.35]),
            );
        }
        vec![bbt_ui::components::Paint::Draw(draw)]
    }
}

impl App {
    pub(super) fn open_terminal_link(&mut self, seat: SeatId, at: [f32; 2]) -> bool {
        if let Some(target) = self.terminal_display(seat).and_then(|d| d.document_at(at)) {
            self.open_document(target);
            return true;
        }
        let Some(uri) = self.terminal_display(seat).and_then(|d| d.link_at(at)) else {
            return false;
        };
        self.logger.write(format!("terminal link: {uri}"));
        if !self.test_account {
            let logger = self.logger.clone();
            bbt_rt::handle().spawn(async move {
                #[cfg(target_os = "macos")]
                let mut command = tokio::process::Command::new("/usr/bin/open");
                #[cfg(target_os = "linux")]
                let mut command = tokio::process::Command::new("xdg-open");
                #[cfg(target_os = "windows")]
                let mut command = {
                    let mut c = tokio::process::Command::new("rundll32.exe");
                    c.arg("url.dll,FileProtocolHandler");
                    c
                };
                #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
                {
                    logger.write("opening links is unavailable on this platform");
                    return;
                }
                #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
                {
                    command.arg(uri).kill_on_drop(true);
                    match tokio::time::timeout(Duration::from_secs(10), command.status()).await {
                        Ok(Ok(status)) if status.success() => {}
                        result => logger.write(format!("open terminal link failed: {result:?}")),
                    }
                }
            });
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_pointer_owns_mouse_mode_and_command_link_uses_displayed_frame() {
        if crate::lazy_projection_tests::isolated_ui_case(
            "terminal_clipboard::tests::selection_pointer_owns_mouse_mode_and_command_link_uses_displayed_frame",
        ) {
            return;
        }
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        app.test_account = true;
        let seat = SeatId::from_u128(1);
        let record =
            bbt_proto::SeatRecord::new(bbt_mux::Ulid(1), bbt_mux::Ulid(90), "link seat", "zsh");
        let mut model = bboxy450_term::model::build_at(
            &[record],
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
            Instant::now(),
            1000,
        );
        let view = bbt_proto::View {
            seat: bbt_mux::Ulid(1),
            generation: model.ui.seats[&seat].process_generation,
            view_generation: bbt_mux::Ulid(7),
        };
        model.screen_baselines.insert(seat, (view, 0));
        app.focus.stage = Some(model.ui.window.stage.clone());
        app.scene.set_model(&model, &model.ui.window.stage);
        app.live_state = Some(Arc::new(model));
        app.solve_ui([1400., 1000.]).unwrap();
        let body = app
            .pane_bounds
            .iter()
            .find(|(id, _)| *id == seat)
            .unwrap()
            .1;
        let mut term = bbt_core::Term::new(40, 2, 0);
        term.advance(b"\x1b[?1000hhttps://example.com/path");
        let display = Display {
            seat,
            view,
            screen: Arc::new(bbt_proto::Screen::from_term(
                &term,
                String::new(),
                None,
                None,
            )),
            rect: Rect::new(body.x, body.y, body.w.min(400.), 40.),
            cell: [10., 20.],
            historical_row: None,
        };
        app.frame_observations.record_displayed([display]);
        let at = [body.x + 1., body.y + 1.];
        app.pointer_pressed(at);
        assert!(
            app.terminal_selection.dragging,
            "ordinary native drag must own TUI mouse mode"
        );
        app.extend_terminal_selection([body.x + 50., body.y + 1.]);
        app.pointer_released();
        assert_eq!(app.terminal_copy_text().as_deref(), Some("https"));
        app.modifiers = ModifiersState::SUPER;
        assert_eq!(app.cursor_at(at), bbt_ui::cursor::Cursor::Pointer);
        app.pointer_pressed(at);
        assert!(
            !app.terminal_selection.dragging,
            "command click must activate the link rather than start selection"
        );
        assert!(
            app.open_terminal_link(seat, at),
            "private account must consume link without launching browser"
        );
        let mut other = app.live_state.as_deref().unwrap().clone();
        other.screen_baselines.get_mut(&seat).unwrap().0.generation += 1;
        app.live_state = Some(Arc::new(other));
        assert!(
            !app.open_terminal_link(seat, at),
            "stale displayed frame must never launch"
        );
    }
}
