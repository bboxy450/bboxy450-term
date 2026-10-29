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
    pub(super) fn terminal_display(&self, seat: SeatId) -> Option<Display> {
        if self.app_overlays.open()
            || self.document.open
            || self.launcher.is_some()
            || self.local_tab.is_some()
            || self.composer.view.focus
            || self.reject_focus
        {
            return None;
        }
        let model = self.live_state.as_ref()?;
        let focused = self
            .focus
            .stage
            .as_ref()
            .unwrap_or(&model.ui.window.stage)
            .focused;
        if focused != Some(seat) {
            return None;
        }
        let body = self.pane_bounds.iter().find(|(id, _)| *id == seat)?.1;
        let prepared = model.prepared_screens.get(&seat)?;
        let frame = model.frames.get(&seat)?;
        let screen = &prepared.screen;
        let mut view = model.screen_baselines.get(&seat)?.0;
        // The process identity is current even when an older prepared frame is retained for resize.
        view.generation = model.ui.seats.get(&seat)?.process_generation;
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
            rect: Rect::new(
                body.x,
                body.y,
                body.w.min(frame.width as f32 / scale),
                body.h.min(frame.height as f32 / scale),
            ),
        })
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
        if let Some(display) = self
            .terminal_selection
            .seat()
            .and_then(|seat| self.terminal_display(seat))
        {
            self.terminal_selection.extend(&display, at);
        } else {
            self.terminal_selection.clear();
        }
        self.request_redraw();
        true
    }
    pub(super) fn terminal_copy_text(&self) -> Option<String> {
        let display = self.terminal_display(self.terminal_selection.seat()?)?;
        self.terminal_selection.text(&display)
    }
    pub(super) fn terminal_copy_shortcut(&self, input: &keyboard::Input<'_>) -> bool {
        input.state.is_pressed()
            && input.modifiers.state == ModifiersState::SUPER
            && keyboard::shortcut_key(input) == Some('c')
            && self.terminal_copy_text().is_some()
    }
    pub(super) fn copy_terminal_selection(&mut self) -> bool {
        let Some(text) = self.terminal_copy_text() else {
            return false;
        };
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
    pub(super) fn paint_terminal_selection(&self) -> Vec<bbt_ui::components::Paint> {
        let Some(display) = self
            .terminal_selection
            .seat()
            .and_then(|seat| self.terminal_display(seat))
        else {
            return vec![];
        };
        let mut draw = bbt_ui_model::draw::DrawList::default();
        for rect in self.terminal_selection.rectangles(&display) {
            draw.rect(rect, 0., bbt_ui_model::Color([0.25, 0.50, 0.95, 0.35]));
        }
        vec![bbt_ui::components::Paint::Draw(draw)]
    }
}
