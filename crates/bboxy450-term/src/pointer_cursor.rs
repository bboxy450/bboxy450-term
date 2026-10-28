// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Cursor intent follows the same retained hit regions and layer order as clicks.
use crate::*;
use bbt_ui::cursor::Cursor;

fn contains(rect: Rect, at: [f32; 2]) -> bool {
    at[0] >= rect.x && at[0] < rect.right() && at[1] >= rect.y && at[1] < rect.bottom()
}

/// Winit resolves these standard shapes to the platform's own cursor resources.
fn native(cursor: Cursor) -> winit::window::CursorIcon {
    use winit::window::CursorIcon;
    match cursor {
        Cursor::Default => CursorIcon::Default,
        Cursor::Pointer => CursorIcon::Pointer,
        Cursor::Text => CursorIcon::Text,
        Cursor::EwResize => CursorIcon::EwResize,
        Cursor::NsResize => CursorIcon::NsResize,
    }
}

impl App {
    pub(super) fn cursor_at(&self, at: [f32; 2]) -> Cursor {
        // A modal owns the whole window, including its otherwise inert backdrop.
        // Never advertise a control underneath it as clickable.
        if self.document.open {
            return self
                .document_layout
                .as_ref()
                .map_or(Cursor::Default, |layout| layout.regions.cursor(at));
        }
        if let Some(controller) = &self.launcher {
            if self.query_dragging {
                return Cursor::Text;
            }
            return self
                .launcher_layout
                .as_ref()
                .map_or(Cursor::Default, |layout| {
                    if layout.query_contains(at) {
                        Cursor::Text
                    } else if let Some(index) = layout.row_at(at) {
                        let launcher = &controller.launcher;
                        let unavailable = launcher.view().rows.get(index).is_none_or(|item| {
                            item.kind == "fixed"
                                || item.down
                                || item.link.as_ref().is_some_and(|link| link.down())
                        });
                        if unavailable {
                            Cursor::Default
                        } else {
                            Cursor::Pointer
                        }
                    } else {
                        Cursor::Default
                    }
                });
        }
        if self.titlebar.actionable(at) { return Cursor::Pointer; }
        if self.sidebar_vertical_drag.is_some() { return Cursor::NsResize; }
        if self.sidebar_drag.active() {
            return Cursor::EwResize;
        }
        if self.composer_dragging || self.query_dragging {
            return Cursor::Text;
        }
        if matches!(&self.local_tab, Some(LocalTab::Settings {controller,..}) if controller.dragging())
        {
            return Cursor::EwResize;
        }
        if self
            .sidebar_resize_region
            .is_some_and(|rect| contains(rect, at))
        {
            return Cursor::EwResize;
        }
        if self.sidebar_vertical_handle(at).is_some() { return Cursor::NsResize; }
        let child_control = self
            .close_regions
            .iter()
            .any(|(_, rect, _)| contains(*rect, at))
            || self
                .swarm_folder_regions
                .iter()
                .any(|(_, rect, _)| contains(*rect, at));
        if !child_control
            && self
                .navigation_regions
                .iter()
                .any(|(_, rect)| contains(*rect, at))
        {
            return Cursor::Pointer;
        }
        if let Some(layout) = &self.conversation_layout
            && layout.regions.contains(at)
        {
            if layout.regions.hit(at).is_some() {
                return layout.regions.cursor(at, &self.composer);
            }
            return layout.actions.cursor(at);
        }
        if let Some(page) = &self.local_page_layout {
            if page.closes(at) {
                return Cursor::Pointer;
            }
            if let Some(settings) = &page.settings
                && settings.hit(at).is_some()
            {
                return settings.cursor(at);
            }
            if page.focuses(at) {
                return Cursor::Default;
            }
        }
        if let Some(layout) = &self.conversation_layout
            && layout.actions.hit(at).is_some()
        {
            return layout.actions.cursor(at);
        }
        if self.task_regions.hit(at).is_some() {
            return self.task_regions.cursor(at);
        }
        if self
            .starting_close_regions
            .iter()
            .any(|(_, rect)| contains(*rect, at))
            || self
                .close_regions
                .iter()
                .any(|(_, rect, _)| contains(*rect, at))
        {
            return Cursor::Pointer;
        }
        if let Some((_, _, reset)) = self
            .swarm_folder_regions
            .iter()
            .find(|(_, rect, _)| contains(*rect, at))
        {
            return if *reset || !self.swarm_folder_pending {
                Cursor::Pointer
            } else {
                Cursor::Default
            };
        }
        Cursor::Default
    }

    pub(super) fn update_cursor(&self) {
        let cursor = if self.pointer_inside && self.frontmost {
            self.cursor_at(self.pointer)
        } else {
            Cursor::Default
        };
        if self.cursor_icon.replace(cursor) != cursor
            && let Some(window) = &self.window
        {
            window.set_cursor(native(cursor));
        }
    }

    pub(super) fn pointer_left(&mut self) {
        self.pointer_inside = false;
        self.update_cursor();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn center(rect: Rect) -> [f32; 2] {
        [rect.x + rect.w * 0.5, rect.y + rect.h * 0.5]
    }

    #[test]
    fn standard_cursor_shapes_use_the_portable_window_api() {
        use winit::window::CursorIcon;
        for (intent, expected) in [
            (Cursor::Default, CursorIcon::Default),
            (Cursor::Pointer, CursorIcon::Pointer),
            (Cursor::Text, CursorIcon::Text),
            (Cursor::EwResize, CursorIcon::EwResize),
            (Cursor::NsResize, CursorIcon::NsResize),
        ] {
            assert_eq!(native(intent), expected);
        }
    }

    // The same App and retained layouts used by native input, with a private
    // process-wide UI registration and HOME; no OS pointer injection or prompts.
    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "isolated process-wide UI registration in private fixture"
    )]
    fn pointer_cursor_follows_real_controls_overlay_order_resize_and_layout_changes() {
        const CHILD: &str = "BBOXY_POINTER_CURSOR_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let home = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "pointer_cursor::tests::pointer_cursor_follows_real_controls_overlay_order_resize_and_layout_changes", "--nocapture", "--test-threads=1"])
                .env(CHILD, "1")
                .env("HOME", home.path())
                .env("ZDOTDIR", home.path())
                .env("CODEX_HOME", home.path().join(".codex"))
                .env("CLAUDE_CONFIG_DIR", home.path().join(".claude"))
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
        let latest = Arc::new(Latest::new(inbox.dirty.paint_source(5)));
        let mut app = App::owner(
            UiProxy::Headless(Arc::new(|_| {})),
            inbox,
            log::Logger::new(),
            latest,
            &["--headless".into()],
            Verify::default(),
        );
        let records: Vec<_> = (1..=2)
            .map(|id| {
                bbt_proto::SeatRecord::new(
                    bbt_mux::Ulid(id),
                    bbt_mux::Ulid(90),
                    format!("seat {id}"),
                    "zsh",
                )
            })
            .collect();
        let model = Arc::new(bboxy450_term::model::build_at(
            &records,
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
            Instant::now(),
            1000,
        ));
        app.scene.set_model(&model, &model.ui.window.stage);
        app.focus.stage = Some(model.ui.window.stage.clone());
        app.live_state = Some(model);
        app.solve_ui([1400., 1000.]).unwrap();
        assert_eq!(app.cursor_at([1399., 999.]), Cursor::Default);
        assert!(app.navigation_regions.len() > 4);
        for (_, rect) in &app.navigation_regions {
            assert_eq!(app.cursor_at(center(*rect)), Cursor::Pointer);
        }
        for (_, rect, _) in &app.close_regions {
            assert_eq!(app.cursor_at(center(*rect)), Cursor::Pointer);
        }
        let choose = app
            .swarm_folder_regions
            .iter()
            .find(|(_, _, reset)| !reset)
            .unwrap()
            .1;
        assert_eq!(app.cursor_at(center(choose)), Cursor::Pointer);
        app.swarm_folder_pending = true;
        assert_eq!(app.cursor_at(center(choose)), Cursor::Default);
        app.swarm_folder_pending = false;

        for handle in app.conversation_layout.as_ref().unwrap().splitters {
            assert_eq!(app.cursor_at(center(handle)), Cursor::NsResize);
        }
        let resize = center(app.sidebar_resize_region.unwrap());
        assert_eq!(app.cursor_at(resize), Cursor::EwResize);
        app.composer_dragging = true;
        assert_eq!(
            app.cursor_at(resize),
            Cursor::Text,
            "captured text selection owns the cursor over passive resize handles"
        );
        app.composer_dragging = false;
        app.begin_sidebar_resize(resize);
        assert_eq!(app.cursor_at([1300., 600.]), Cursor::EwResize);
        app.end_sidebar_resize();

        let settings = center(
            app.navigation_regions
                .iter()
                .find(|(key, _)| key == "settings")
                .unwrap()
                .1,
        );
        app.pointer = settings;
        app.pointer_inside = true;
        app.frontmost = true;
        app.solve_ui([1400., 1000.]).unwrap();
        assert_eq!(app.cursor_icon.get(), Cursor::Pointer);
        app.pointer_pressed(settings);
        app.solve_ui([1400., 1000.]).unwrap();
        let page = app.local_page_layout.as_ref().unwrap();
        assert_eq!(app.cursor_at(center(page.close)), Cursor::Pointer);
        assert_eq!(
            app.cursor_at(center(page.settings.as_ref().unwrap().search)),
            Cursor::Text
        );

        app.open_launcher();
        app.solve_ui([1400., 1000.]).unwrap();
        assert_eq!(
            app.cursor_at(settings),
            Cursor::Default,
            "launcher masks sidebar actions"
        );
        let launcher = app.launcher_layout.as_ref().unwrap();
        assert_eq!(app.cursor_at(center(launcher.query)), Cursor::Text);
        let row = app
            .launcher
            .as_ref()
            .unwrap()
            .launcher
            .view()
            .rows
            .iter()
            .position(|item| !item.down && !item.link.as_ref().is_some_and(|link| link.down()))
            .unwrap();
        assert_eq!(app.cursor_at(center(launcher.rows[row])), Cursor::Pointer);
        app.query_dragging = true;
        assert_eq!(app.cursor_at(settings), Cursor::Text);
        app.query_dragging = false;
        app.close_launcher();
        app.solve_ui([1400., 1000.]).unwrap();
        assert_eq!(
            app.cursor_icon.get(),
            Cursor::Pointer,
            "stationary pointer refreshes after closing overlay"
        );

        app.document.open = true;
        app.document_layout = None;
        app.update_cursor();
        assert_eq!(
            app.cursor_icon.get(),
            Cursor::Default,
            "opening document blocks even before its first layout"
        );
        app.document.open = false;
        app.solve_ui([1400., 1000.]).unwrap();
        assert_eq!(app.cursor_icon.get(), Cursor::Pointer);
        app.frontmost = false;
        app.update_cursor();
        assert_eq!(app.cursor_icon.get(), Cursor::Default);
        app.frontmost = true;
        app.update_cursor();
        assert_eq!(
            app.cursor_icon.get(),
            Cursor::Pointer,
            "keyboard refocus preserves the stationary pointer's physical location"
        );
        app.pointer_left();
        assert_eq!(app.cursor_icon.get(), Cursor::Default);
    }
}
