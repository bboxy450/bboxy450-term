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
        if self.app_overlays.open() {
            return self.app_overlays.cursor(at);
        }
        // A modal owns the whole window, including its otherwise inert backdrop.
        // Never advertise a control underneath it as clickable.
        if self.document.open {
            return self
                .document_layout
                .as_ref()
                .map_or(Cursor::Default, |layout| {
                    let cursor = layout.regions.cursor(at);
                    if cursor == Cursor::Default && self.ui_selection.contains(at) {
                        Cursor::Text
                    } else {
                        cursor
                    }
                });
        }
        if self.launcher.is_some() {
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
                        if layout.row_available(index) {
                            Cursor::Pointer
                        } else {
                            Cursor::Default
                        }
                    } else {
                        Cursor::Default
                    }
                });
        }
        if self.titlebar.actionable(at) {
            return Cursor::Pointer;
        }
        if self.sidebar_vertical_drag.is_some() {
            return Cursor::NsResize;
        }
        if self.sidebar_drag.active() {
            return Cursor::EwResize;
        }
        if self.composer_dragging
            || self.query_dragging
            || self.terminal_selection.dragging
            || self.ui_selection.dragging
        {
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
        if self.panel_header(at).is_some() {
            return Cursor::Pointer;
        }
        if self.sidebar_vertical_handle(at).is_some() {
            return Cursor::NsResize;
        }
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
            return if self.ui_selection.contains(at) && layout.actions.hit(at).is_none() {
                Cursor::Text
            } else {
                layout.actions.cursor(at)
            };
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
                return if self.ui_selection.contains(at) {
                    Cursor::Text
                } else {
                    Cursor::Default
                };
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
        if self.modifiers.super_key()
            && self.pane_bounds.iter().any(|(seat, _)| {
                self.terminal_display(*seat)
                    .and_then(|d| d.link_at(at))
                    .is_some()
            })
        {
            return Cursor::Pointer;
        }
        if self.ui_selection.contains(at) {
            return Cursor::Text;
        }
        if self.pane_bounds.iter().any(|(seat, _)| {
            self.terminal_display(*seat)
                .is_some_and(|d| d.contains(at) && d.allows_selection(self.modifiers.shift_key()))
        }) {
            return Cursor::Text;
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

    fn redraw_requests(app: &App) -> u64 {
        let counters = serde_json::to_value(app.wake_counters.borrow().snapshot()).unwrap();
        counters["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| source["redraws"].as_u64().unwrap())
            .sum()
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
        let before = redraw_requests(&app);
        for _ in 0..1000 {
            assert!(app.move_sidebar_resize(resize));
        }
        assert_eq!(redraw_requests(&app), before, "stationary width capture");
        assert!(app.move_sidebar_resize([-1000., resize[1]]));
        assert_eq!(redraw_requests(&app), before + 1);
        for _ in 0..1000 {
            assert!(app.move_sidebar_resize([-2000., resize[1]]));
        }
        assert_eq!(redraw_requests(&app), before + 1, "clamped width capture");
        assert!(app.sidebar_drag.active());
        assert!(app.move_sidebar_resize(resize));
        assert_eq!(redraw_requests(&app), before + 2, "return from width clamp");
        app.end_sidebar_resize();

        let loaded = app.preferences.loaded;
        let splits = app.preferences.preferences.sidebar;
        app.preferences.loaded = true;
        for handle in app.conversation_layout.as_ref().unwrap().splitters {
            let point = center(handle);
            assert!(app.begin_sidebar_vertical(point));
            let before = redraw_requests(&app);
            for _ in 0..1000 {
                assert!(app.move_sidebar_resize(point));
            }
            assert_eq!(redraw_requests(&app), before, "stationary vertical capture");
            assert!(app.move_sidebar_vertical([point[0], point[1] + 10_000.]));
            let before = redraw_requests(&app);
            for _ in 0..1000 {
                assert!(app.move_sidebar_resize([point[0], point[1] + 20_000.]));
            }
            assert_eq!(redraw_requests(&app), before, "clamped vertical capture");
            assert!(app.sidebar_vertical_drag.is_some());
            assert!(app.move_sidebar_resize([point[0], point[1] - 10_000.]));
            assert_eq!(
                redraw_requests(&app),
                before + 1,
                "return from vertical clamp"
            );
            app.end_sidebar_resize();
            app.preferences.preferences.sidebar = splits;
        }
        app.preferences.loaded = loaded;

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
        let before = redraw_requests(&app);
        app.launcher_command(LauncherCommand::Hover(row));
        let after_first = redraw_requests(&app);
        assert!(after_first <= before + 1);
        for _ in 0..1000 {
            app.launcher_command(LauncherCommand::Hover(row));
        }
        assert_eq!(redraw_requests(&app), after_first, "same launcher row");
        let next = (row + 1) % app.launcher_layout.as_ref().unwrap().rows.len();
        assert_ne!(row, next);
        app.launcher_command(LauncherCommand::Hover(next));
        assert_eq!(redraw_requests(&app), after_first + 1, "new launcher row");
        app.launcher_command(LauncherCommand::Down);
        assert!(
            redraw_requests(&app) > after_first + 1,
            "keyboard navigation still paints"
        );
        app.solve_ui([1400., 1000.]).unwrap();

        // A cursor uses availability from the painted projection even if newer
        // launcher data has not produced its next frame yet.
        let items = std::mem::take(&mut app.launcher.as_mut().unwrap().launcher.items);
        let layout = app.launcher_layout.as_ref().unwrap();
        let at = center(layout.rows[row]);
        for _ in 0..1000 {
            assert_eq!(app.cursor_at(at), Cursor::Pointer);
        }
        app.launcher.as_mut().unwrap().launcher.items = items;

        let launcher = &mut app.launcher.as_mut().unwrap().launcher;
        launcher.items = vec![
            bbt_ui::modules::launcher::Item {
                id: "fixed".into(),
                label: "Fixed".into(),
                kind: "fixed".into(),
                ..Default::default()
            },
            bbt_ui::modules::launcher::Item {
                id: "down".into(),
                label: "Down".into(),
                down: true,
                ..Default::default()
            },
            bbt_ui::modules::launcher::Item {
                id: "offline".into(),
                label: "Offline".into(),
                link: Some(bbt_ui::modules::link::LinkState::Offline),
                ..Default::default()
            },
            bbt_ui::modules::launcher::Item {
                id: "available".into(),
                label: "Available".into(),
                ..Default::default()
            },
        ];
        launcher.selected = None;
        app.solve_ui([1400., 1000.]).unwrap();
        let layout = app.launcher_layout.as_ref().unwrap();
        for (row, expected) in [
            Cursor::Default,
            Cursor::Default,
            Cursor::Default,
            Cursor::Pointer,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(app.cursor_at(center(layout.rows[row])), expected);
        }
        let before = redraw_requests(&app);
        app.launcher_command(LauncherCommand::ActivateAt {
            index: 1,
            modifiers: Default::default(),
        });
        assert!(app.launcher.is_some(), "disabled row cannot activate");
        assert_eq!(redraw_requests(&app), before);

        use bbt_ui::modules::launcher::{FieldKey, Source, SpawnField};
        assert!(app.launcher.as_mut().unwrap().open_parameters(
            "available",
            vec![
                SpawnField {
                    key: FieldKey::Role,
                    value: "executor".into(),
                    source: Source::BuiltIn,
                    choices: None,
                    fixed: true
                },
                SpawnField {
                    key: FieldKey::Command,
                    value: "zsh".into(),
                    source: Source::BuiltIn,
                    choices: None,
                    fixed: false
                },
            ]
        ));
        app.launcher
            .as_mut()
            .unwrap()
            .launcher
            .params
            .as_mut()
            .unwrap()
            .runs = "retained preview".into();
        app.solve_ui([1400., 1000.]).unwrap();
        let before = redraw_requests(&app);
        for _ in 0..1000 {
            app.launcher_command(LauncherCommand::Hover(0));
        }
        assert_eq!(redraw_requests(&app), before, "same parameter row");
        app.launcher_command(LauncherCommand::Hover(1));
        assert_eq!(redraw_requests(&app), before + 1);
        assert_eq!(
            app.launcher
                .as_ref()
                .unwrap()
                .launcher
                .params
                .as_ref()
                .unwrap()
                .runs,
            "retained preview"
        );
        app.launcher_command(LauncherCommand::SetCaret {
            at: 0,
            extend: false,
        });
        assert_eq!(
            app.launcher
                .as_ref()
                .unwrap()
                .launcher
                .params
                .as_ref()
                .unwrap()
                .runs,
            "retained preview"
        );
        app.launcher_command(LauncherCommand::Down);
        assert_ne!(
            app.launcher
                .as_ref()
                .unwrap()
                .launcher
                .params
                .as_ref()
                .unwrap()
                .runs,
            "retained preview",
            "navigation still resolves its preview"
        );
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

        app.local_tab = None;
        app.composer.view.focus = false;
        let seat = SeatId::from_u128(1);
        app.focus.stage.as_mut().unwrap().focused = Some(seat);
        app.solve_ui([1400., 1000.]).unwrap();
        let mut model = app.live_state.as_deref().unwrap().clone();
        let view = View {
            seat: bbt_mux::Ulid(seat.0.0),
            generation: model.ui.seats[&seat].process_generation,
            view_generation: bbt_mux::Ulid(7),
        };
        model.screen_baselines.insert(seat, (view, 0));
        app.live_state = Some(Arc::new(model));
        let body = app
            .pane_bounds
            .iter()
            .find(|(id, _)| *id == seat)
            .unwrap()
            .1;
        let mut term = bbt_core::Term::new(8, 2, 0);
        term.advance(b"abcdef");
        let display = bboxy450_term::terminal_selection::Display {
            seat,
            view,
            screen: Arc::new(bbt_proto::Screen::from_term(
                &term,
                String::new(),
                None,
                None,
            )),
            rect: Rect::new(body.x, body.y, 80., 40.),
            cell: [10., 20.],
            historical_row: None,
        };
        app.frame_observations
            .record_displayed(vec![display.clone()]);
        assert!(
            app.terminal_selection
                .begin(display, [body.x, body.y], false)
        );
        let before = redraw_requests(&app);
        for _ in 0..1000 {
            assert!(app.extend_terminal_selection([body.x + 1., body.y + 2.]));
        }
        assert_eq!(redraw_requests(&app), before, "same terminal cell");
        assert!(app.extend_terminal_selection([body.x + 20., body.y + 2.]));
        assert_eq!(redraw_requests(&app), before + 1, "new selection endpoint");
        assert_eq!(app.terminal_copy_text().as_deref(), Some("ab"));
        assert!(app.extend_terminal_selection([body.right() + 1000., body.bottom() + 1000.]));
        let before = redraw_requests(&app);
        for _ in 0..1000 {
            assert!(app.extend_terminal_selection([body.right() + 2000., body.bottom() + 2000.]));
        }
        assert_eq!(redraw_requests(&app), before, "clamped selection endpoint");
        app.pointer_released();
        assert!(!app.terminal_selection.dragging);
    }
}
