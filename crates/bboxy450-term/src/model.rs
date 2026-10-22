//! Back-model assembly and terminal pane preparation. Socket ownership is C1a's.
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

use bbt_core::{
    Color, NamedColor,
    term::{TermMode, cell::Flags},
};
use bbt_proto::{GuiSeat, Screen, SeatId, SeatRecord, SeatState, registry};
use bbt_render::{
    cells::terminal::{
        self, Cell, Cursor, CursorShape, Grid, PreparedGrid, TerminalRenderer, Underline,
    },
    device::DeviceContext,
};
use bbt_ui_model::{self as ui, layout::Rect, tokens::Theme};

use crate::grids::Grids;

/// A complete immutable UI frame. The IO owner publishes one Arc<Model>; no
/// terminal rows are cloned or locked by the UI while taking that Arc.
#[derive(Clone)]
pub struct Model {
    pub ui: ui::Model,
    pub screens: BTreeMap<SeatId, Arc<Screen>>,
    pub frames: BTreeMap<SeatId, Arc<PreparedGrid>>,
    pub states: BTreeMap<SeatId, SeatState>,
}
impl Default for Model {
    fn default() -> Self {
        Self {
            ui: ui::Model::new(vec![], vec![]).expect("empty model"),
            screens: BTreeMap::new(),
            frames: BTreeMap::new(),
            states: BTreeMap::new(),
        }
    }
}

impl Model {
    /// Project the portable state after build, before preparation/publication.
    /// State comes from logic; this does not infer activity from terminal text.
    pub fn apply_seat_states(&mut self, seats: &[GuiSeat]) {
        self.states.retain(|id, _| self.ui.seats.contains_key(id));
        for seat in seats {
            let id = SeatId::from_u128(seat.record.id.0);
            let Some(row) = self.ui.seats.get_mut(&id) else {
                continue;
            };
            if row.process_generation != seat.state.generation {
                continue;
            }
            row.activity = seat.state.activity;
            row.grade = seat.state.grade;
            row.title = seat.state.title.clone();
            self.states.insert(id, seat.state.clone());
        }
    }
}

/// Pure assembly on IO. Keep the current layout when its membership is unchanged
/// and keep every unchanged screen Arc; no screen content is copied here.
pub fn build(seats: &[SeatRecord], grids: &Grids, previous: &Model) -> Model {
    let screens: BTreeMap<_, _> = seats
        .iter()
        .filter_map(|record| {
            let id = SeatId::from_u128(record.id.0);
            grids.screen(id).map(|screen| (id, screen))
        })
        .collect();
    let now = Instant::now();
    let now_ms = bbt_mux::store::now_ms();
    let metadata: BTreeMap<_, _> = seats
        .iter()
        .map(|record| {
            let id = SeatId::from_u128(record.id.0);
            let swarm = ui::SwarmId::from_u128(record.swarm.0);
            let mut seat = previous.ui.seats.get(&id).cloned().unwrap_or_else(|| {
                ui::Seat::new(id, swarm, [0; 32], &record.name, &record.harness_kind)
            });
            seat.swarm = swarm;
            seat.name = record.name.clone();
            seat.harness = ui::HarnessId(record.harness_kind.clone());
            seat.process_generation = record.generation;
            seat.role = Some(ui::Role {
                name: record.role.name.clone(),
                class: match record.role.class {
                    registry::Class::Oracle => ui::RoleClass::Oracle,
                    registry::Class::Worker => ui::RoleClass::Worker,
                },
            });
            seat.where_ = match &record.location {
                registry::Location::Local => ui::Where::Local,
                registry::Location::Ssh { host } => ui::Where::Ssh { host: host.clone() },
                registry::Location::Cloud { provider, host } => ui::Where::Cloud {
                    provider: provider.clone(),
                    host: host.clone(),
                },
            };
            seat.muted = record.muted;
            seat.tasks = record
                .tasks
                .iter()
                .map(|id| ui::TaskId::from_u128(id.0))
                .collect();
            seat.lifecycle = match record.lifecycle {
                registry::Lifecycle::Open => ui::Lifecycle::Open,
                registry::Lifecycle::ClosedRetained => ui::Lifecycle::ClosedRetained,
                registry::Lifecycle::ClosedGrace => record
                    .close
                    .as_ref()
                    .and_then(|close| {
                        now.checked_add(Duration::from_millis(
                            close.deadline_ms.saturating_sub(now_ms),
                        ))
                    })
                    .map_or(ui::Lifecycle::ClosedRetained, |until| {
                        ui::Lifecycle::ClosedGrace { until }
                    }),
            };
            if let Some(screen) = screens.get(&id) {
                seat.title = screen.title.clone();
            }
            (id, seat)
        })
        .collect();
    let mut groups: BTreeMap<ui::SwarmId, Vec<SeatId>> = BTreeMap::new();
    for seat in metadata
        .values()
        .filter(|s| matches!(s.lifecycle, ui::Lifecycle::Open))
    {
        groups.entry(seat.swarm).or_default().push(seat.id);
    }
    let swarms = groups
        .into_iter()
        .map(|(id, ids)| {
            if let Some(old) = previous
                .ui
                .window
                .sidebar
                .swarms
                .iter()
                .find(|s| s.id == id)
            {
                let mut old_ids: Vec<_> = old
                    .tabs
                    .iter()
                    .flat_map(|tab| tab.seats.iter().copied())
                    .collect();
                old_ids.sort();
                if old_ids == ids {
                    return old.clone();
                }
                let mut next = old.clone();
                next.tabs = vec![ui::Tab::new(
                    ui::TabId::from_u128(id.0.0),
                    "seats",
                    split(&ids),
                )];
                return next;
            }
            ui::Swarm::new(
                id,
                "swarm",
                vec![ui::Tab::new(
                    ui::TabId::from_u128(id.0.0),
                    "seats",
                    split(&ids),
                )],
            )
        })
        .collect();
    let mut model = ui::Model::new(swarms, metadata.into_values().collect())
        .expect("unique records and valid seat trees");
    let fallback_stage = model.window.stage.clone();
    let next_swarms = std::mem::take(&mut model.window.sidebar.swarms);
    model.window = previous.ui.window.clone();
    model
        .window
        .sidebar
        .open
        .retain(|id| next_swarms.iter().any(|s| s.id == *id));
    for swarm in &next_swarms {
        if !previous
            .ui
            .window
            .sidebar
            .swarms
            .iter()
            .any(|old| old.id == swarm.id)
        {
            model.window.sidebar.open.insert(swarm.id);
        }
    }
    model.window.sidebar.swarms = next_swarms;
    model.window.stage = fallback_stage;
    let stage = &previous.ui.window.stage;
    if model
        .window
        .sidebar
        .swarms
        .iter()
        .any(|s| Some(s.id) == stage.swarm && s.tabs.iter().any(|t| Some(t.id) == stage.tab))
    {
        model.window.stage = stage.clone();
        if stage.focused.is_none_or(|id| {
            !model
                .seats
                .get(&id)
                .is_some_and(|s| matches!(s.lifecycle, ui::Lifecycle::Open))
        }) {
            model.window.stage.focused = model
                .window
                .sidebar
                .swarms
                .iter()
                .flat_map(|s| &s.tabs)
                .find(|t| Some(t.id) == stage.tab)
                .and_then(|t| t.seats.first().copied());
        }
    }
    model.window.overlay = previous.ui.window.overlay.clone();
    let frames = previous
        .frames
        .iter()
        .filter_map(
            |(seat, frame)| match (screens.get(seat), previous.screens.get(seat)) {
                (Some(next), Some(old)) if Arc::ptr_eq(next, old) => Some((*seat, frame.clone())),
                _ => None,
            },
        )
        .collect();
    let states = previous
        .states
        .iter()
        .filter(|(id, _)| model.seats.contains_key(id))
        .map(|(id, state)| (*id, state.clone()))
        .collect();
    Model {
        ui: model,
        screens,
        frames,
        states,
    }
}

fn split(ids: &[SeatId]) -> ui::SplitNode {
    if ids.len() == 1 {
        return ui::SplitNode::Leaf(ids[0]);
    }
    let mid = ids.len() / 2;
    ui::SplitNode::split(
        ui::Axis::Row,
        mid as f32 / ids.len() as f32,
        split(&ids[..mid]),
        split(&ids[mid..]),
    )
}

struct CachedPane {
    screen: Arc<Screen>,
    focused: bool,
    theme: Theme,
    frame: Arc<PreparedGrid>,
}
/// Retained worker-side glyph cache, sharing the app's one GPU device. The IO
/// loop calls prepare after build and before publishing the complete Model.
pub struct PanePreparer {
    context: Arc<DeviceContext>,
    renderer: TerminalRenderer,
    cached: BTreeMap<SeatId, CachedPane>,
}
impl PanePreparer {
    pub fn new(context: Arc<DeviceContext>) -> bbt_render::Result<Self> {
        Self::with_options(context, terminal::Options::default())
    }

    pub fn with_options(
        context: Arc<DeviceContext>,
        options: terminal::Options,
    ) -> bbt_render::Result<Self> {
        bbt_rt::assert_not_ui_thread();
        let renderer = TerminalRenderer::new(&context, options)?;
        Ok(Self {
            context,
            renderer,
            cached: BTreeMap::new(),
        })
    }
    pub fn prepare(&mut self, model: &mut Model) -> bbt_render::Result<()> {
        bbt_rt::assert_not_ui_thread();
        self.cached
            .retain(|seat, _| model.screens.contains_key(seat));
        let theme = model.ui.window.theme;
        let mut frames = BTreeMap::new();
        for (&seat, screen) in &model.screens {
            let focused = model.ui.window.stage.focused == Some(seat);
            let reuse = self.cached.get(&seat).is_some_and(|old| {
                Arc::ptr_eq(&old.screen, screen) && old.focused == focused && old.theme == theme
            });
            if !reuse {
                let grid = screen_grid(screen, theme, focused)?;
                let frame = Arc::new(self.renderer.prepare(&self.context, &grid)?);
                self.cached.insert(
                    seat,
                    CachedPane {
                        screen: screen.clone(),
                        focused,
                        theme,
                        frame,
                    },
                );
            }
            frames.insert(seat, self.cached[&seat].frame.clone());
        }
        model.frames = frames;
        Ok(())
    }
}

pub struct DrawCtx<'a> {
    pub frame: &'a PreparedGrid,
    pub encoder: &'a mut wgpu::CommandEncoder,
    pub target: &'a wgpu::TextureView,
    pub scale: f32,
}
/// UI-only encoding of a frame already prepared on IO. No rasterization,
/// allocation of a glyph atlas, device poll, readback or lock occurs here.
pub fn draw_pane(screen: &Screen, rect: Rect, ctx: &mut DrawCtx<'_>) {
    if screen.cols == 0 || screen.rows == 0 || !ctx.scale.is_finite() || ctx.scale <= 0. {
        return;
    }
    let rect = rect.normalized();
    let x = (rect.x * ctx.scale).round().max(0.) as u32;
    let y = (rect.y * ctx.scale).round().max(0.) as u32;
    let w = (rect.w * ctx.scale).floor().max(0.) as u32;
    let h = (rect.h * ctx.scale).floor().max(0.) as u32;
    ctx.frame
        .encode(ctx.encoder, ctx.target, [x, y], [x, y, w, h]);
}

fn rgb(value: u32) -> [u8; 4] {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]
}
fn palette(screen: &Screen, index: usize, theme: Theme) -> [u8; 4] {
    if let Some(v) = screen.palette.get(index)
        && let (Some(r), Some(g), Some(b)) = (v["r"].as_u64(), v["g"].as_u64(), v["b"].as_u64())
    {
        return [r as u8, g as u8, b as u8, 255];
    }
    const ANSI: [u32; 16] = [
        0x2e3436, 0xcc0000, 0x4e9a06, 0xc4a000, 0x3465a4, 0x75507b, 0x06989a, 0xd3d7cf, 0x555753,
        0xef2929, 0x8ae234, 0xfce94f, 0x729fcf, 0xad7fa8, 0x34e2e2, 0xeeeeec,
    ];
    match index {
        0..=15 => rgb(ANSI[index]),
        16..=231 => {
            let n = index - 16;
            let c = [0, 95, 135, 175, 215, 255];
            [c[n / 36], c[n / 6 % 6], c[n % 6], 255]
        }
        232..=255 => {
            let v = (8 + (index - 232) * 10) as u8;
            [v, v, v, 255]
        }
        257 => {
            if theme == Theme::Light {
                rgb(0xf2f4f8)
            } else {
                rgb(0x0f141d)
            }
        }
        259..=266 => {
            let mut c = rgb(ANSI[index - 259]);
            for v in &mut c[..3] {
                *v /= 2;
            }
            c
        }
        _ => {
            if theme == Theme::Light {
                rgb(0x171f2b)
            } else {
                rgb(0xe8edf7)
            }
        }
    }
}
fn color(screen: &Screen, value: Color, theme: Theme, bold: bool) -> [u8; 4] {
    match value {
        Color::Spec(v) => [v.r, v.g, v.b, 255],
        Color::Indexed(i) => palette(
            screen,
            usize::from(if bold && i < 8 { i + 8 } else { i }),
            theme,
        ),
        Color::Named(n) => {
            let i = n as usize;
            palette(screen, if bold && i < 8 { i + 8 } else { i }, theme)
        }
    }
}
/// Convert rows only on IO; the cells renderer has no parser/protocol dependency.
pub fn screen_grid(screen: &Screen, theme: Theme, focused: bool) -> bbt_render::Result<Grid> {
    if screen.cols == 0
        || screen.rows == 0
        || screen.cols > 512
        || screen.rows > 256
        || screen.cells.len() != screen.rows
        || screen.cells.iter().any(|r| r.len() != screen.cols)
    {
        return Err("invalid terminal screen dimensions".into());
    }
    let cells = screen
        .cells
        .iter()
        .flatten()
        .map(|c| {
            let f = c.flags;
            let width = if f.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER) {
                0
            } else if f.contains(Flags::WIDE_CHAR) {
                2
            } else {
                1
            };
            let bold = f.contains(Flags::BOLD);
            let mut foreground = color(screen, c.fg, theme, bold);
            let mut background = color(screen, c.bg, theme, false);
            if f.contains(Flags::INVERSE) {
                std::mem::swap(&mut foreground, &mut background);
            }
            Cell {
                text: if width == 0 {
                    String::new()
                } else {
                    std::iter::once(c.c)
                        .chain(c.zerowidth().unwrap_or_default().iter().copied())
                        .collect()
                },
                width,
                foreground,
                background,
                underline_color: c
                    .underline_color()
                    .map_or(foreground, |v| color(screen, v, theme, false)),
                bold,
                italic: f.contains(Flags::ITALIC),
                dim: f.contains(Flags::DIM),
                hidden: f.contains(Flags::HIDDEN),
                underline: if f.contains(Flags::UNDERCURL) {
                    Underline::Curly
                } else if f.contains(Flags::DOUBLE_UNDERLINE) {
                    Underline::Double
                } else if f.contains(Flags::DOTTED_UNDERLINE) {
                    Underline::Dotted
                } else if f.contains(Flags::DASHED_UNDERLINE) {
                    Underline::Dashed
                } else if f.contains(Flags::UNDERLINE) {
                    Underline::Single
                } else {
                    Underline::None
                },
                strike: f.contains(Flags::STRIKEOUT),
            }
        })
        .collect();
    let shape = screen.cursor_style["shape"].as_str().unwrap_or("Block");
    let cursor = (screen.modes & TermMode::SHOW_CURSOR.bits() as u64 != 0
        && shape != "Hidden"
        && screen.cursor.0 >= 0
        && (screen.cursor.0 as usize) < screen.rows
        && screen.cursor.1 < screen.cols)
        .then(|| Cursor {
            column: screen.cursor.1 as u16,
            row: screen.cursor.0 as u16,
            shape: match shape {
                "Beam" => CursorShape::Bar,
                "Underline" => CursorShape::Underline,
                _ => CursorShape::Block,
            },
            focused: focused && shape != "HollowBlock",
            color: color(screen, Color::Named(NamedColor::Cursor), theme, false),
        });
    Ok(Grid {
        columns: screen.cols as u16,
        rows: screen.rows as u16,
        cells,
        cursor,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_mux::Ulid;
    use bbt_proto::{Snapshot, Update, View};

    fn install(grids: &mut Grids, record: &SeatRecord, term: &bbt_core::Term) -> SeatId {
        let id = SeatId::from_u128(record.id.0);
        let view = View {
            seat: record.id,
            generation: record.generation,
            view_generation: Ulid::new(),
        };
        grids.set_watched(&[(id, view)]);
        grids
            .apply(
                id,
                &Update::Snapshot {
                    snapshot: Box::new(Snapshot {
                        view,
                        sequence: 1,
                        screen: Screen::from_term(term, "terminal title".into(), None, None),
                    }),
                },
            )
            .unwrap();
        id
    }

    #[test]
    fn metadata_build_keeps_screen_identity_and_canonical_state_ignores_stale_generations() {
        let mut grids = Grids::new();
        let mut record = SeatRecord::new(Ulid::new(), Ulid::new(), "first", "zsh");
        let id = install(&mut grids, &record, &bbt_core::Term::new(8, 2, 0));
        let mut first = build(&[record.clone()], &grids, &Model::default());
        first.ui.window.sidebar.open.clear();
        record.name = "renamed".into();
        let mut next = build(&[record.clone()], &grids, &first);
        assert!(Arc::ptr_eq(&first.screens[&id], &next.screens[&id]));
        assert_eq!(first.ui.window.stage, next.ui.window.stage);
        assert!(next.ui.window.sidebar.open.is_empty());
        assert_eq!(next.ui.seats[&id].name, "renamed");
        let mut seat = GuiSeat {
            record: record.clone(),
            state: SeatState {
                generation: record.generation,
                process: bbt_proto::ProcessStatus::Running { pid: 123 },
                activity: ui::Activity::Waiting(ui::WaitKind::Permission),
                grade: ui::Grade::Reported,
                title: "permission".into(),
                detail: None,
                turn_open: true,
                observed_ms: 100,
                subagents: vec![],
            },
        };
        next.apply_seat_states(&[seat.clone()]);
        assert_eq!(
            next.ui.seats[&id].activity,
            ui::Activity::Waiting(ui::WaitKind::Permission)
        );
        assert_eq!(next.ui.seats[&id].title, "permission");
        seat.state.generation += 1;
        seat.state.title = "stale process".into();
        next.apply_seat_states(&[seat]);
        assert_eq!(next.ui.seats[&id].title, "permission");
    }

    #[test]
    fn screen_rows_preserve_palette_styles_width_combining_and_cursor() {
        let mut term = bbt_core::Term::new(12, 2, 0);
        term.advance(b"\x1b]4;1;rgb:aa/bb/cc\x07\x1b[31mX\x1b[0m");
        term.advance("\x1b[1;3;4;9;38;2;10;20;30m中e\u{301}\x1b[0m\x1b[6 q".as_bytes());
        let mut screen = Screen::from_term(&term, String::new(), None, None);
        let grid = screen_grid(&screen, Theme::Dark, true).unwrap();
        assert_eq!(grid.cells[0].foreground, [0xaa, 0xbb, 0xcc, 255]);
        assert_eq!(grid.cells[1].text, "中");
        assert_eq!(grid.cells[1].width, 2);
        assert_eq!(grid.cells[2].width, 0);
        assert!(grid.cells[2].text.is_empty());
        assert_eq!(grid.cells[3].text, "e\u{301}");
        assert_eq!(grid.cells[1].foreground, [10, 20, 30, 255]);
        assert!(grid.cells[1].bold && grid.cells[1].italic && grid.cells[1].strike);
        assert_eq!(grid.cells[1].underline, Underline::Single);
        assert_eq!(grid.cursor.unwrap().shape, CursorShape::Bar);
        screen.cells[0].pop();
        assert!(screen_grid(&screen, Theme::Dark, false).is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn worker_preparation_reuses_unchanged_frames_and_updates_focus_without_grid_copy() {
        fn send<T: Send>() {}
        send::<PanePreparer>();
        let context = Arc::new(bbt_rt::handle().block_on(DeviceContext::new()).unwrap());
        let mut preparer = PanePreparer::new(context).unwrap();
        let record = SeatRecord::new(Ulid::new(), Ulid::new(), "zsh", "zsh");
        let mut grids = Grids::new();
        let id = install(&mut grids, &record, &bbt_core::Term::new(8, 2, 0));
        let mut first = build(&[record.clone()], &grids, &Model::default());
        preparer.prepare(&mut first).unwrap();
        let mut next = build(&[record], &grids, &first);
        preparer.prepare(&mut next).unwrap();
        assert!(Arc::ptr_eq(&first.screens[&id], &next.screens[&id]));
        assert!(Arc::ptr_eq(&first.frames[&id], &next.frames[&id]));
        next.ui.window.stage.focused = None;
        preparer.prepare(&mut next).unwrap();
        assert!(Arc::ptr_eq(&first.screens[&id], &next.screens[&id]));
        assert!(!Arc::ptr_eq(&first.frames[&id], &next.frames[&id]));
    }
}
