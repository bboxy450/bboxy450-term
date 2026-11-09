// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Sidebar drag and drop. A press on a swarm header or seat row waits for release:
//! released in place it is the ordinary click; moved, it drops onto the row under
//! the pointer. Native input and `ctl ui drag` share this path.
use crate::*;
use bbt_ui_model::{SeatId, SidebarMove, SwarmId};

/// The scene key of a pressed region: `toggle-swarm:<id>` drags `swarm:<id>`.
fn drag_key(pressed: &str) -> String {
    pressed
        .strip_prefix("toggle-")
        .unwrap_or(pressed)
        .to_owned()
}

/// Movement before a press becomes a drag, in logical pixels.
const SLOP: f32 = 4.;

#[derive(Debug, Clone)]
pub(super) struct RowPress {
    id: String,
    from: [f32; 2],
    dragging: bool,
}

impl App {
    /// Defers a draggable row's navigation to release. False for other regions.
    pub(super) fn begin_row_press(&mut self, id: &str, at: [f32; 2]) -> bool {
        if !(id.starts_with("toggle-swarm:") || id.starts_with("seat:")) {
            return false;
        }
        self.row_press = Some(RowPress {
            id: id.into(),
            from: at,
            dragging: false,
        });
        true
    }
    pub(super) fn move_row_press(&mut self, at: [f32; 2]) -> bool {
        let Some(press) = &mut self.row_press else {
            return false;
        };
        if !press.dragging && (at[0] - press.from[0]).hypot(at[1] - press.from[1]) >= SLOP {
            press.dragging = true;
        }
        self.pointer = at;
        if press.dragging {
            // The scene draws the row under the pointer and slides the others aside.
            let drag = crate::scene::SidebarDrag {
                key: drag_key(&press.id),
                dy: at[1] - press.from[1],
            };
            self.scene.set_sidebar_drag(Some(drag));
            self.request_redraw();
        }
        true
    }
    /// Completes the press: a click navigates, a drag queues its move.
    pub(super) fn end_row_press(&mut self) -> bool {
        let Some(press) = self.row_press.take() else {
            return false;
        };
        if !press.dragging {
            self.logger.write(format!("ui navigation: {}", press.id));
            self.navigate_to(&press.id);
            return true;
        }
        // Drop where the preview showed it; the scene eases every row into place.
        let previewed = self
            .scene
            .sidebar_drop_target()
            .and_then(|target| self.keyed_move(&press.id, &target));
        self.scene.set_sidebar_drag(None);
        self.request_redraw();
        if let Some(change) = previewed.or_else(|| self.row_drop(&press.id, self.pointer)) {
            self.logger.write(format!("ui sidebar move: {change:?}"));
            self.queue(live::UiMsg::SidebarMove(change));
        }
        true
    }
    /// The move a drop onto `target` (a scene key) makes for the pressed region.
    fn keyed_move(&self, pressed: &str, target: &str) -> Option<SidebarMove> {
        let model = self.live_state.as_ref()?;
        let swarm = |text: &str| {
            model
                .ui
                .window
                .sidebar
                .swarms
                .iter()
                .find(|s| s.id.to_string() == text)
                .map(|s| s.id)
        };
        match (pressed.split_once(':')?, target.split_once(':')?) {
            (("toggle-swarm", a), ("swarm", b)) => Some(SidebarMove::Swarm {
                swarm: swarm(a)?,
                target: swarm(b)?,
            }),
            (("seat", a), ("seat", b)) => Some(SidebarMove::Seat {
                seat: a.parse().ok()?,
                target: b.parse().ok()?,
            }),
            _ => None,
        }
    }
    fn row_drop(&self, dragged: &str, at: [f32; 2]) -> Option<SidebarMove> {
        let model = self.live_state.as_ref()?;
        let swarm = |text: &str| {
            model
                .ui
                .window
                .sidebar
                .swarms
                .iter()
                .find(|s| s.id.to_string() == text)
                .map(|s| s.id)
        };
        let seat = |text: &str| text.parse::<SeatId>().ok();
        let contains =
            |r: &Rect| at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom();
        let target = self
            .navigation_regions
            .iter()
            .filter(|(id, rect)| {
                (id.starts_with("toggle-swarm:") || id.starts_with("seat:"))
                    && id.as_str() != dragged
                    && contains(rect)
            })
            .map(|(id, _)| id.as_str())
            .next()?;
        let seat_swarm =
            |id: SeatId| -> Option<SwarmId> { model.ui.seats.get(&id).map(|s| s.swarm) };
        if let Some(dragged) = dragged.strip_prefix("toggle-swarm:").and_then(swarm) {
            let target = match target.split_once(':')? {
                ("toggle-swarm", id) => swarm(id)?,
                (_, id) => seat_swarm(seat(id)?)?,
            };
            return Some(SidebarMove::Swarm {
                swarm: dragged,
                target,
            });
        }
        let dragged = seat(dragged.strip_prefix("seat:")?)?;
        let target = seat(target.strip_prefix("seat:")?)?;
        (seat_swarm(dragged)? == seat_swarm(target)?).then_some(SidebarMove::Seat {
            seat: dragged,
            target,
        })
    }
    /// `ctl ui drag` from a swarm header or seat row. False when `from` is neither.
    pub(super) fn drag_row(&mut self, from: [f32; 2], to: [f32; 2]) -> bool {
        self.pointer_pressed(from);
        if self.row_press.is_none() {
            self.pointer_released();
            return false;
        }
        self.row_press.as_mut().unwrap().dragging = true;
        self.move_row_press(to);
        self.pointer_released();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The retained sidebar layout and the App input path native clicks and ctl use.
    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "isolated process-wide UI registration in private fixture"
    )]
    fn dragging_rows_queues_moves_and_a_still_press_stays_the_click() {
        const CHILD: &str = "BBOXY_SIDEBAR_REORDER_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let home = tempfile::tempdir().unwrap();
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "sidebar_reorder::tests::dragging_rows_queues_moves_and_a_still_press_stays_the_click", "--nocapture", "--test-threads=1"])
                .env(CHILD, "1")
                .env("HOME", home.path())
                .env("ZDOTDIR", home.path())
                .env("CODEX_HOME", home.path().join(".codex"))
                .env("CLAUDE_CONFIG_DIR", home.path().join(".claude"))
                .output()
                .unwrap();
            assert!(
                output.status.success()
                    && String::from_utf8_lossy(&output.stdout).contains("1 passed"),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let (mut app, _) = crate::lazy_projection_tests::fixture();
        app.startup.attached();
        let records: Vec<_> = [(1, 90), (2, 90), (3, 91)]
            .into_iter()
            .map(|(id, swarm)| {
                bbt_proto::SeatRecord::new(
                    bbt_mux::Ulid(id),
                    bbt_mux::Ulid(swarm),
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
        app.live_commands = Some(live::disconnected_commands());
        app.solve_ui([1400., 1000.]).unwrap();
        let center = |app: &App, id: String| {
            let rect = app
                .navigation_regions
                .iter()
                .find(|(region, _)| *region == id)
                .unwrap_or_else(|| panic!("no region {id}: {:?}", app.navigation_regions))
                .1;
            [rect.x + rect.w / 2., rect.y + rect.h / 2.]
        };
        let (swarm, seat) = (SwarmId::from_u128, SeatId::from_u128);
        let header = |app: &App, id| center(app, format!("toggle-swarm:{}", swarm(id)));
        let row = |app: &App, id| center(app, format!("seat:{}", seat(id)));
        let last = |app: &App| match app.pending_commands.back() {
            Some(live::UiMsg::SidebarMove(change)) => Some(*change),
            _ => None,
        };
        let (from, to) = (header(&app, 91), header(&app, 90));
        assert_eq!(app.drag_sidebar(from, to), Ok(None));
        assert_eq!(
            last(&app),
            Some(SidebarMove::Swarm {
                swarm: swarm(91),
                target: swarm(90)
            })
        );
        // A seat row of another swarm is that swarm's drop target.
        let (from, to) = (header(&app, 90), row(&app, 3));
        app.drag_sidebar(from, to).unwrap();
        assert_eq!(
            last(&app),
            Some(SidebarMove::Swarm {
                swarm: swarm(90),
                target: swarm(91)
            })
        );
        let (from, to) = (row(&app, 2), row(&app, 1));
        app.drag_sidebar(from, to).unwrap();
        assert_eq!(
            last(&app),
            Some(SidebarMove::Seat {
                seat: seat(2),
                target: seat(1)
            })
        );
        // Seats never cross swarms.
        let queued = app.pending_commands.len();
        let (from, to) = (row(&app, 3), row(&app, 1));
        app.drag_sidebar(from, to).unwrap();
        assert_eq!(app.pending_commands.len(), queued);
        // A press that does not move is the ordinary click, applied on release.
        let open = app
            .scene
            .sidebar_open()
            .get(&format!("swarm:{}", swarm(91)))
            .copied();
        let at = header(&app, 91);
        app.pointer_pressed(at);
        assert!(app.row_press.is_some());
        app.move_row_press([at[0] + 1., at[1]]);
        app.pointer_released();
        assert!(app.row_press.is_none());
        assert_eq!(app.pending_commands.len(), queued);
        assert_ne!(
            app.scene
                .sidebar_open()
                .get(&format!("swarm:{}", swarm(91)))
                .copied(),
            open
        );
    }
}
