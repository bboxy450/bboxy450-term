// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Native callbacks dispatch the same application actions as keyboard and ctl.
use crate::*;
use bboxy450_term_desktop::Action;

impl App {
    pub(super) fn install_desktop_menu(&mut self) {
        if self.desktop_menu.is_some() {
            return;
        }
        let proxy = self.proxy.clone();
        match bboxy450_term_desktop::MenuBar::install(Box::new(move |action| {
            let _ = proxy.send_event(UiEvent::DesktopAction(
                action,
                bboxy450_term_desktop::MenuBar::current_key_scancode(),
            ));
        })) {
            Ok(menu) => self.desktop_menu = Some(menu),
            Err(reason) => self
                .logger
                .write(format!("desktop menu unavailable: {reason}")),
        }
    }
    pub(super) fn consume_desktop_font_key(&mut self, action: Action, scancode: Option<u32>) {
        use winit::platform::scancode::PhysicalKeyExtScancode;
        if matches!(
            action,
            Action::IncreaseTerminalFont | Action::DecreaseTerminalFont
        ) && let Some(scancode) = scancode
        {
            self.keyboard
                .consumed(PhysicalKey::from_scancode(scancode), ElementState::Pressed);
        }
    }
    /// Closing is completed by the native/headless event owner using its existing lifecycle.
    pub(super) fn desktop_action(&mut self, action: Action) -> Option<bool> {
        if self.app_overlays.open() {
            match action {
                Action::CloseFocusedSeat => {
                    self.close_app_overlay();
                    return None;
                }
                Action::NewTab
                | Action::SplitRight
                | Action::SplitDown
                | Action::Settings
                | Action::MuteFocusedSeat => {
                    return None;
                }
                _ => {}
            }
        }
        match action {
            Action::CloseWindow => return Some(false),
            Action::HardQuit => return Some(true),
            Action::CloseFocusedSeat => self.close_focused_seat(),
            Action::MuteFocusedSeat => self.mute_focused(),
            Action::NewTab => self.open_launcher(),
            Action::SplitRight => self.split_launcher(bbt_proto::seat_state::Side::Right),
            Action::SplitDown => self.split_launcher(bbt_proto::seat_state::Side::Bottom),
            Action::Settings => self.open_settings("Account"),
            Action::IncreaseTerminalFont => self.change_terminal_font(1),
            Action::DecreaseTerminalFont => self.change_terminal_font(-1),
            Action::ShowWindow
            | Action::HideWindow
            | Action::ToggleFullscreen
            | Action::Minimize
            | Action::Zoom => {
                if let Some(window) = &self.window
                    && let Err(reason) =
                        bboxy450_term_desktop::apply_window_action(window.as_ref(), action)
                {
                    self.logger.write(format!("desktop window: {reason}"));
                }
            }
        }
        self.request_redraw();
        None
    }
    pub(super) fn titlebar_double_click(&mut self) {
        if let Some(window) = &self.window {
            let action = bboxy450_term_desktop::titlebar_action();
            if let Err(reason) =
                bboxy450_term_desktop::apply_titlebar_action(window.as_ref(), action)
            {
                self.logger.write(format!("desktop titlebar: {reason}"));
            }
        }
    }
}
