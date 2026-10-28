//! Native and ctl pointer resize share this nonblocking input boundary.
//! Origin: bboxy450-term, MIT OR Apache-2.0. Retain credit on reuse:
//! https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::*;

impl App {
    pub(super) fn begin_sidebar_resize(&mut self, at: [f32; 2]) -> bool {
        if self.document.open || self.launcher.is_some() {
            return false;
        }
        let Some(handle) = self.sidebar_resize_region else {
            return false;
        };
        if !self
            .sidebar_drag
            .begin(at, handle, self.scene.sidebar_width_for(self.solved_size))
        {
            return false;
        }
        self.pointer = at;
        self.query_dragging = false;
        self.composer_dragging = false;
        if matches!(self.local_tab, Some(LocalTab::Settings { .. })) {
            self.settings_action(settings_controller::Action::EndDrag);
        }
        self.sidebar_cursor();
        true
    }
    pub(super) fn move_sidebar_resize(&mut self, at: [f32; 2]) -> bool {
        if self.sidebar_drag.active() && (self.document.open || self.launcher.is_some()) {
            self.end_sidebar_resize();
            return false;
        }
        let Some(width) = self.sidebar_drag.update(at[0]) else {
            return false;
        };
        self.pointer = at;
        self.sidebar_width = Some(width);
        self.request_redraw();
        true
    }
    pub(super) fn end_sidebar_resize(&mut self) {
        self.sidebar_drag.finish();
        self.sidebar_cursor();
    }
    pub(super) fn sidebar_cursor(&self) {
        let over = !self.document.open
            && self.launcher.is_none()
            && self.sidebar_resize_region.is_some_and(|r| {
                self.pointer[0] >= r.x
                    && self.pointer[0] < r.right()
                    && self.pointer[1] >= r.y
                    && self.pointer[1] < r.bottom()
            });
        if let Some(window) = &self.window {
            window.set_cursor(if self.sidebar_drag.active() || over {
                winit::window::CursorIcon::EwResize
            } else {
                winit::window::CursorIcon::Default
            });
        }
    }
    pub(super) fn drag_sidebar(&mut self, from: [f32; 2], to: [f32; 2]) -> Result<(), String> {
        if !self.begin_sidebar_resize(from) {
            return Err("ui_drag_refused: start on the sidebar resize handle".into());
        }
        self.move_sidebar_resize(to);
        self.end_sidebar_resize();
        Ok(())
    }
}
