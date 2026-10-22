//! Component composition only. Input, mutable forms and FX remain separate briefs.
use crate::Result;
use bbt_ui_model::{
    draw::*,
    layout::{Rect, solve_split},
    *,
};

pub fn height(id: &str, state: &str, d: Density) -> u32 {
    let row = d.tokens().row_h as u32;
    match id {
        "sidebar" => 788,
        "title_strip" => {
            if state.contains("breathe") {
                620
            } else {
                440
            }
        }
        "settings_tab" => 820,
        "tokens" => 600,
        "seat_row" => row * 4 + 40,
        "tab_row" => {
            if state.starts_with("expanded") {
                row * 4 + 50
            } else {
                row + 28
            }
        }
        "glyph" => 96,
        "harness-icon" => 120,
        "mini_grid" => 142,
        "launcher" => 540,
        "logo" => 560,
        "toolbar-icon" => 140,
        "message-row" => 340,
        "graph_tab" => 720,
        "provenance_card" | "subscriptions_card" => 400,
        "swarm_card" => 330,
        "pane" => 250,
        "nested_subagent_rows" => {
            if state == "overflow" {
                208
            } else {
                84
            }
        }
        _ => 150,
    }
}
const NAMES: [&str; 15] = [
    "Vesper Greywind",
    "Nyx Copperwing",
    "Lumen Foxglove",
    "Rook Ashwhisker",
    "Sable Quill",
    "Pip Hollow",
    "Marrow Finch",
    "zsh",
    "Tamsin Voss",
    "Corvin Ember",
    "Ilka Thistle",
    "Dax Riven",
    "Wren Starling",
    "Halcyon Moss",
    "Juniper Vale",
];
const DARK: [&str; 15] = [
    "#FFC53D", "#FF4FA3", "#B07BFF", "#2EE6A8", "#2E6BFF", "#FF7A45", "#58D7FF", "#9AA3C7",
    "#F2D16B", "#5BE08C", "#E96BFF", "#FF6B6B", "#4FD1FF", "#C9A96B", "#8AF0D0",
];
const LIGHT: [&str; 15] = [
    "#B8860B", "#C2185B", "#6A3FD1", "#0E8F6A", "#1E4FD1", "#D1541C", "#0E7FA8", "#5B6480",
    "#9A7B10", "#1F8F4A", "#9C1FBF", "#C63B3B", "#0E86B8", "#8A6B2E", "#1E9C7E",
];
fn sid(i: usize) -> SeatId {
    SeatId::from_u128(i as u128 + 1)
}
fn col(i: usize, ctx: &DrawCtx) -> Color {
    Color::hex(if ctx.theme == Theme::Dark {
        DARK[i % 15]
    } else {
        LIGHT[i % 15]
    })
}
fn derived_color(key: &str, ctx: &DrawCtx) -> Color {
    let h = SeatColor::Derived.resolve(key, ctx.theme);
    Color::hsl(h.hue, h.saturation / 100.0, h.lightness / 100.0)
}
fn seat_color(s: &Seat, ctx: &DrawCtx) -> Color {
    if let SeatColor::Override(h) = s.color {
        Color::hsl(h.hue, h.saturation / 100.0, h.lightness / 100.0)
    } else {
        col(s.id.0.0.saturating_sub(1) as usize, ctx)
    }
}
fn activity(s: &str) -> Activity {
    match s {
        "active" => Activity::Active,
        "waiting" => Activity::Waiting(WaitKind::Permission),
        "errored" => Activity::Errored,
        _ => Activity::Idle,
    }
}
fn seat(i: usize) -> Seat {
    let harness = match i {
        0 | 2 | 14 => "claude",
        5 | 10 => "pi",
        6 | 12 => "omp",
        7 => "zsh",
        _ => "codex",
    };
    let mut s = Seat::new(
        sid(i),
        SwarmId::from_u128(1),
        [i as u8; 32],
        NAMES[i % 15],
        harness,
    );
    s.title = [
        "swarm control",
        "messaging spec",
        "ux lab",
        "gpu research",
        "harness hooks",
        "relay bench",
        "parse logs",
        "~/mydev",
    ][i % 8]
        .into();
    s.activity = match i {
        0 | 1 | 8 | 10 | 14 => Activity::Active,
        3 | 12 => Activity::Waiting(WaitKind::Permission),
        5 => Activity::Errored,
        _ => Activity::Idle,
    };
    s.grade = if harness == "claude" {
        Grade::Reported
    } else if harness == "codex" {
        Grade::Shim
    } else {
        Grade::Inferred
    };
    if i != 7 {
        s.role = Some(Role {
            name: if i == 0 {
                "oracle"
            } else if i == 2 {
                "creativity"
            } else {
                "executor"
            }
            .into(),
            class: if i == 0 {
                RoleClass::Oracle
            } else {
                RoleClass::Worker
            },
        });
    }
    if i == 3 {
        s.where_ = Where::Cloud {
            provider: "demo".into(),
            host: "eu-2".into(),
        };
    }
    if i == 4 || i == 13 {
        s.where_ = Where::Ssh { host: "rig".into() };
    }
    s
}
pub(crate) fn text(
    out: &mut DrawList,
    r: Rect,
    value: impl Into<std::sync::Arc<str>>,
    size: f32,
    color: Color,
    weight: u16,
) {
    out.text(
        r,
        value,
        TextStyle {
            scale: TypeScale::Custom(size),
            weight,
            ..Default::default()
        },
        color,
    );
}
pub(crate) fn icon(out: &mut DrawList, r: Rect, name: &str, color: Color) {
    out.push(Prim::Icon {
        rect: r,
        icon: name.into(),
        color,
    });
}
pub(crate) fn panel(out: &mut DrawList, r: Rect, ctx: &DrawCtx) {
    out.push(Prim::Rect {
        rect: r,
        radii: [STYLE.radius.card as f32; 4],
        fill: ctx
            .ground(Color::hex(ctx.theme.palette().surface))
            .alpha(STYLE.material.card_alpha as f32),
        stroke: Some((1.0, ctx.hairline())),
        inner_highlight: None,
        shadow: None,
    });
}

pub fn seat_row(
    s: &Seat,
    r: Rect,
    focused: bool,
    shortcut: Option<usize>,
    ctx: &DrawCtx,
    out: &mut DrawList,
) {
    out.clip(r, [0.0; 4]);
    let d = ctx.density.tokens();
    let color = seat_color(s, ctx);
    if focused {
        out.rect(
            r,
            STYLE.radius.row as f32,
            ctx.color(Color::hex(ctx.theme.palette().fg).alpha(STYLE.tint.selection_alpha as f32)),
        );
    }
    let mut x = r.x + d.pad_x as f32;
    let cy = r.y + r.h * 0.5;
    let gap = d.gap as f32;
    if s.where_ != Where::Local {
        WhereMark(&s.where_).draw(Rect::new(x, cy - 6.0, 12.0, 12.0), ctx, out);
        x += 12.0 + gap;
    }
    let tile = 16.0;
    let tile_color = Color::hex(match s.harness.0.as_str() {
        "claude" => "#D97757",
        "codex" => "#416DE5",
        "pi" => "#8A65CE",
        "omp" => "#249D89",
        _ => "#58617A",
    });
    IconTile {
        icon: &s.harness.0,
        color: tile_color,
    }
    .draw(Rect::new(x, cy - tile * 0.5, tile, tile), ctx, out);
    x += tile + gap;
    let glyph = d.glyph as f32;
    let mut right = r.right() - d.pad_x as f32;
    right -= glyph;
    StateGlyph {
        activity: s.activity,
        grade: s.grade,
        color,
    }
    .draw(Rect::new(right, cy - glyph * 0.5, glyph, glyph), ctx, out);
    right -= gap;
    if let Some(n) = shortcut {
        right -= 28.0;
        Badge {
            label: &format!("⌘{n}"),
            selected: focused,
            mono: true,
        }
        .draw(Rect::new(right, cy - 8.0, 28.0, 16.0), ctx, out);
        right -= gap;
    }
    if let Some((n, _)) = s.unread {
        right -= 20.0;
        out.rect(
            Rect::new(right, cy - 7.0, 18.0, 14.0),
            7.0,
            Color::hex(TOKENS.color.role.oracle),
        );
        text(
            out,
            Rect::new(right + 5.0, cy - 7.0, 15.0, 15.0),
            n.to_string(),
            9.0,
            Color::WHITE,
            600,
        );
        right -= gap;
    }
    let role_space = if s.role.is_some() { 12.0 + gap } else { 0.0 };
    let mute_space = if s.muted { 12.0 + gap } else { 0.0 };
    let natural = bbt_render::chrome::text::rasterize(
        &s.name,
        TextStyle {
            scale: TypeScale::Custom(d.font as f32),
            weight: 600,
            ..Default::default()
        },
        None,
        1.0,
    )
    .map(|m| m.advance)
    .unwrap_or(s.name.chars().count() as f32 * d.font as f32 * 0.52);
    let name_w = natural.min((right - x - role_space - mute_space).max(1.0));
    text(
        out,
        Rect::new(x, cy - d.font as f32 * 0.7, name_w, d.font as f32 * 1.4),
        s.name.as_str(),
        d.font as f32,
        ctx.color(color).alpha(if s.muted { 0.7 } else { 1.0 }),
        600,
    );
    x += name_w + gap;
    if let Some(role) = &s.role {
        RoleMark(role).draw(Rect::new(x, cy - 7.0, 12.0, 14.0), ctx, out);
        x += 12.0 + gap;
    }
    if s.muted {
        icon(
            out,
            Rect::new(x, cy - 5.5, 11.0, 11.0),
            "muted",
            ctx.color(Color::hex(TOKENS.color.state.waiting)),
        );
        x += 11.0 + gap;
    }
    if ctx.density.shows_title() && right - x > 12.0 {
        text(
            out,
            Rect::new(x, cy - 7.0, (right - x).max(0.0), 14.0),
            s.title.as_str(),
            d.font_small as f32,
            ctx.muted(),
            400,
        );
    }
    out.unclip();
}
fn split(dir: Axis, ratio: f32, a: SplitNode, b: SplitNode) -> SplitNode {
    SplitNode::Split {
        dir,
        ratio,
        a: Box::new(a),
        b: Box::new(b),
    }
}
pub fn tree15() -> SplitNode {
    fn parse(v: &serde_json::Value) -> SplitNode {
        if let Some(i) = v["seat"].as_u64() {
            SplitNode::Leaf(sid(i as usize))
        } else {
            split(
                if v["axis"] == "V" {
                    Axis::Row
                } else {
                    Axis::Column
                },
                v["ratio"].as_str().unwrap().parse().unwrap(),
                parse(&v["a"]),
                parse(&v["b"]),
            )
        }
    }
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../bbt-ui-model/tests/fixtures/scene15.json"
    ))
    .unwrap();
    parse(&fixture["tree"])
}
fn tree3() -> SplitNode {
    split(
        Axis::Row,
        0.6,
        SplitNode::Leaf(sid(0)),
        split(
            Axis::Column,
            0.5,
            SplitNode::Leaf(sid(1)),
            SplitNode::Leaf(sid(3)),
        ),
    )
}
pub fn mini_grid(
    tree: &SplitNode,
    r: Rect,
    focused: Option<SeatId>,
    ctx: &DrawCtx,
    out: &mut DrawList,
) {
    out.clip(r, [TOKENS.radius.grid as f32; 4]);
    for (id, rect) in solve_split(tree, r, 1.0).panes {
        let s = seat(id.0.0 as usize - 1);
        let c = if Some(id) == focused {
            ctx.theme.palette().accent
        } else {
            match s.activity {
                Activity::Waiting(_) => TOKENS.color.state.waiting,
                Activity::Errored => TOKENS.color.state.errored,
                _ => ctx.theme.palette().line,
            }
        };
        out.rect(rect, 1.0, ctx.color(Color::hex(c)));
    }
    out.unclip();
}
pub(crate) fn band(
    name: &str,
    count: usize,
    r: Rect,
    color: Color,
    ctx: &DrawCtx,
    out: &mut DrawList,
) {
    out.push(Prim::Gloss {
        rect: r,
        radii: [STYLE.radius.tile as f32; 4],
        color: ctx.color(color),
    });
    out.text(
        Rect::new(r.x + 12.0, r.y + (r.h - 14.0) * 0.5, r.w - 65.0, 14.0),
        name,
        TextStyle {
            scale: TypeScale::Band,
            weight: 600,
            tracking_em: 0.16,
            uppercase: true,
            ..Default::default()
        },
        ctx.color(Color::WHITE.alpha(0.92)),
    );
    text(
        out,
        Rect::new(
            r.x + 12.0 + name.len() as f32 * 8.0 + 10.0,
            r.y + (r.h - 14.0) * 0.5,
            24.0,
            14.0,
        ),
        count.to_string(),
        10.5,
        ctx.color(Color::WHITE.alpha(0.7)),
        400,
    );
    Swatch(color).draw(
        Rect::new(r.right() - 22.0, r.y + (r.h - 10.0) * 0.5, 10.0, 10.0),
        ctx,
        out,
    );
}
pub(crate) fn new_seat(r: Rect, ctx: &DrawCtx, out: &mut DrawList) {
    text(
        out,
        Rect::new(r.x + 8.0, r.y + 3.0, r.w - 45.0, r.h),
        "+  new seat",
        STYLE.r#type.section as f32,
        ctx.muted(),
        500,
    );
    Badge {
        label: "⌘T",
        selected: false,
        mono: true,
    }
    .draw(Rect::new(r.right() - 35.0, r.y + 4.0, 28.0, 16.0), ctx, out);
}
fn closed_strip(r: Rect, ctx: &DrawCtx, out: &mut DrawList) {
    out.rect(
        r,
        STYLE.radius.row as f32,
        ctx.color(
            Color::hex(ctx.theme.palette().fg).alpha(STYLE.tint.selection_hover_alpha as f32),
        ),
    );
    text(
        out,
        Rect::new(r.x + 8.0, r.y + 4.0, r.w - 100.0, r.h),
        "Nyx Copperwing · 52s",
        11.0,
        ctx.muted(),
        400,
    );
    Badge {
        label: "undo ⌘Z",
        selected: false,
        mono: true,
    }
    .draw(Rect::new(r.right() - 75.0, r.y + 4.0, 68.0, 18.0), ctx, out);
}
fn tab_row(
    tree: &SplitNode,
    name: &str,
    r: Rect,
    expanded: bool,
    ctx: &DrawCtx,
    out: &mut DrawList,
) {
    text(
        out,
        Rect::new(r.x + 8.0, r.y + 3.0, r.w - 72.0, r.h),
        name,
        ctx.density.tokens().font_small as f32,
        ctx.muted(),
        400,
    );
    mini_grid(
        tree,
        Rect::new(r.right() - 32.0, r.y + (r.h - 16.0) * 0.5, 24.0, 16.0),
        if expanded { Some(sid(0)) } else { None },
        ctx,
        out,
    );
    if !expanded {
        for (i, id) in tree.leaves().into_iter().take(6).enumerate() {
            StateGlyph {
                activity: seat(id.0.0 as usize - 1).activity,
                grade: Grade::Reported,
                color: col(i, ctx),
            }
            .draw(
                Rect::new(r.right() - 105.0 + i as f32 * 10.0, r.y + 6.0, 8.0, 8.0),
                ctx,
                out,
            );
        }
    }
}
fn card(
    r: Rect,
    name: &str,
    indices: &[usize],
    open: bool,
    focused: bool,
    ctx: &DrawCtx,
    out: &mut DrawList,
) {
    let dim = if focused { *ctx } else { ctx.dimmed() };
    let color = derived_color(&format!("space-{name}"), ctx);
    panel(out, r, &dim);
    out.clip(r, [STYLE.radius.card as f32; 4]);
    let h = STYLE.spacing.band as f32;
    band(
        name,
        indices.len(),
        Rect::new(r.x + 4.0, r.y + 4.0, r.w - 8.0, h),
        color,
        &dim,
        out,
    );
    if open {
        let mut y = r.y + h + 6.0;
        let row = ctx.density.tokens().row_h as f32;
        let tab_h = ctx.density.tokens().tab_h as f32;
        let tree = if indices.len() == 15 {
            tree15()
        } else {
            tree3()
        };
        tab_row(
            &tree,
            if indices.len() == 15 {
                "all fifteen"
            } else {
                "core"
            },
            Rect::new(r.x + 6.0, y, r.w - 12.0, tab_h),
            true,
            &dim,
            out,
        );
        y += tab_h;
        for (n, i) in indices.iter().enumerate() {
            let nested = indices.len() == 15 || n < 3;
            seat_row(
                &seat(*i),
                Rect::new(
                    r.x + 6.0 + if nested { 8.0 } else { 0.0 },
                    y,
                    r.w - 12.0 - if nested { 8.0 } else { 0.0 },
                    row,
                ),
                *i == 0,
                if nested { None } else { Some(n - 1) },
                &dim,
                out,
            );
            y += row;
        }
        new_seat(Rect::new(r.x + 6.0, y + 6.0, r.w - 12.0, row), &dim, out);
        y += row + 12.0;
        text(
            out,
            Rect::new(r.x + 12.0, y, r.w - 24.0, tab_h),
            "Shadows",
            ctx.density.tokens().font_small as f32,
            dim.muted(),
            400,
        );
        y += tab_h;
        for (i, name) in ["Kestrel Nine", "Orrin Slate", "Ione Marsh"]
            .iter()
            .enumerate()
        {
            let mut ph = seat(i);
            ph.name = (*name).into();
            ph.title.clear();
            ph.grade = Grade::Reported;
            ph.activity = [
                Activity::Active,
                Activity::Idle,
                Activity::Waiting(WaitKind::Question),
            ][i];
            ph.where_ = Where::Local;
            let c = Color::hex(ctx.theme.palette().fg_dim).to_hsl();
            ph.color = SeatColor::Override(Hsl {
                hue: c[0],
                saturation: c[1] * 100.0,
                lightness: c[2] * 100.0,
            });
            seat_row(
                &ph,
                Rect::new(r.x + 6.0, y, r.w - 12.0, row),
                false,
                None,
                &dim,
                out,
            );
            y += row;
        }
    }
    out.push(Prim::Rect {
        rect: Rect::new(
            r.x,
            r.bottom() - if open { 4.0 } else { 3.0 },
            r.w,
            if open { 4.0 } else { 3.0 },
        ),
        radii: [0.0, 0.0, 12.0, 12.0],
        fill: ctx.identity(color),
        stroke: None,
        inner_highlight: None,
        shadow: None,
    });
    out.unclip();
}
fn sidebar(r: Rect, count: usize, ctx: &DrawCtx, out: &mut DrawList) {
    out.rect(
        r,
        0.0,
        Color::hex(ctx.theme.palette().sidebar)
            .alpha(STYLE.material.panel_alpha as f32)
            .mix(
                Color::hex(ctx.theme.palette().bg),
                1.0 - STYLE.material.panel_alpha as f32,
            ),
    );
    out.clip(r, [0.0; 4]);
    let row = ctx.density.tokens().row_h as f32;
    let grid = STYLE.spacing.grid as f32;
    let mut y = r.y + grid;
    let groups = if count == 15 {
        vec![("swarm", (0..15).collect::<Vec<_>>(), true)]
    } else {
        vec![
            ("term", vec![0, 1, 3, 2, 4], true),
            ("winnow", vec![5, 6, 7], false),
        ]
    };
    for (name, indices, open) in groups {
        let h = if open {
            STYLE.spacing.band as f32
                + 24.0
                + ctx.density.tokens().tab_h as f32 * 2.0
                + row * (indices.len() as f32 + 4.0)
        } else {
            STYLE.spacing.band as f32 + 11.0
        };
        card(
            Rect::new(r.x + 8.0, y, r.w - 16.0, h),
            name,
            &indices,
            open,
            name != "winnow",
            ctx,
            out,
        );
        y += h + TOKENS.layout.cohort_gap as f32;
    }
    let new = Rect::new(r.x + 8.0, y, r.w - 16.0, 40.0);
    panel(out, new, ctx);
    text(
        out,
        Rect::new(new.x + 14.0, new.y + 11.0, new.w - 28.0, 20.0),
        "+  new swarm",
        STYLE.r#type.row as f32,
        ctx.muted(),
        500,
    );
    icon(
        out,
        Rect::new(r.x + 12.0, r.bottom() - 24.0, 16.0, 16.0),
        "archive",
        ctx.muted(),
    );
    text(
        out,
        Rect::new(r.right() - 27.0, r.bottom() - 28.0, 20.0, 22.0),
        "+",
        20.0,
        ctx.muted(),
        400,
    );
    out.unclip();
}
fn close_dot(r: Rect, hover: bool, ctx: &DrawCtx, out: &mut DrawList) {
    out.rect(r, r.h, ctx.color(Color::hex("#FF5F57")));
    if hover {
        out.line(
            [r.x + 3.0, r.y + 3.0],
            [r.right() - 3.0, r.bottom() - 3.0],
            1.0,
            Color::BLACK.alpha(0.6),
        );
        out.line(
            [r.right() - 3.0, r.y + 3.0],
            [r.x + 3.0, r.bottom() - 3.0],
            1.0,
            Color::BLACK.alpha(0.6),
        );
    }
}
pub(crate) fn strip(s: &Seat, r: Rect, focused: bool, ctx: &DrawCtx, out: &mut DrawList) {
    let ctx = if focused { *ctx } else { ctx.dimmed() };
    let color = seat_color(s, &ctx);
    out.rect(r, 0.0, ctx.color(color));
    out.clip(r, [0.0; 4]);
    close_dot(
        Rect::new(r.x + 6.0, r.y + 5.0, 12.0, 12.0),
        false,
        &ctx,
        out,
    );
    let mut x = r.x + 25.0;
    if s.where_ != Where::Local {
        WhereMark(&s.where_).draw(Rect::new(x, r.y + 5.0, 12.0, 12.0), &ctx, out);
        x += 17.0;
    }
    icon(
        out,
        Rect::new(x, r.y + 5.0, 13.0, 13.0),
        &s.harness.0,
        ctx.color(Color::WHITE.alpha(0.9)),
    );
    x += 18.0;
    let name_w = (s.name.len() as f32 * 5.6).min((r.w - 150.0).max(12.0));
    text(
        out,
        Rect::new(x, r.y + 3.0, name_w, 16.0),
        s.name.as_str(),
        11.0,
        ctx.color(Color::WHITE.alpha(0.95)),
        600,
    );
    x += name_w + 5.0;
    if let Some(role) = &s.role {
        RoleMark(role).draw(Rect::new(x, r.y + 3.0, 12.0, 16.0), &ctx, out);
        x += 16.0;
    }
    let title = if let Activity::Waiting(kind) = s.activity {
        format!(
            "{} · waiting for {}",
            s.title,
            match kind {
                WaitKind::Permission => "permission",
                WaitKind::Question => "question",
                WaitKind::Auth => "auth",
            }
        )
    } else {
        s.title.clone()
    };
    text(
        out,
        Rect::new(x, r.y + 3.0, (r.right() - x - 76.0).max(0.0), 16.0),
        title,
        10.5,
        ctx.color(Color::WHITE.alpha(0.8)),
        400,
    );
    Badge {
        label: "⌘1",
        selected: false,
        mono: true,
    }
    .draw(
        Rect::new(r.right() - 72.0, r.y + 3.0, 28.0, 16.0),
        &ctx,
        out,
    );
    icon(
        out,
        Rect::new(r.right() - 37.0, r.y + 5.0, 12.0, 12.0),
        if s.muted { "muted" } else { "unmuted" },
        ctx.color(Color::WHITE.alpha(0.8)),
    );
    StateGlyph {
        activity: s.activity,
        grade: s.grade,
        color,
    }
    .draw(
        Rect::new(r.right() - 18.0, r.y + 5.0, 11.0, 11.0),
        &ctx,
        out,
    );
    out.unclip();
}
pub(crate) fn pane(s: &Seat, r: Rect, focused: bool, ctx: &DrawCtx, out: &mut DrawList) {
    let color = match s.activity {
        Activity::Errored => Color::hex(TOKENS.color.state.errored),
        _ => seat_color(s, ctx),
    };
    let dim = if focused { *ctx } else { ctx.dimmed() };
    out.rect(r, 0.0, dim.ground(Color::hex(ctx.theme.palette().bg)));
    let b = if s.activity == Activity::Active {
        TOKENS.strip.border_active
    } else {
        TOKENS.strip.border_idle
    } as f32;
    out.push(Prim::Rect {
        rect: r,
        radii: [0.0; 4],
        fill: Color::TRANSPARENT,
        stroke: Some((b, ctx.identity(color))),
        inner_highlight: None,
        shadow: None,
    });
    strip(
        s,
        Rect::new(
            r.x + b,
            r.y + b,
            (r.w - 2.0 * b).max(0.0),
            TOKENS.strip.height as f32,
        ),
        focused,
        ctx,
        out,
    );
}
pub(crate) fn titlebar(r: Rect, ctx: &DrawCtx, out: &mut DrawList) {
    out.rect(
        r,
        0.0,
        Color::hex(ctx.theme.palette().surface).alpha(STYLE.material.panel_alpha as f32),
    );
    out.line(
        [r.x, r.bottom() - 0.5],
        [r.right(), r.bottom() - 0.5],
        1.0,
        ctx.hairline(),
    );
    icon(
        out,
        Rect::new(r.x + 14.0, r.y + 10.0, 18.0, 18.0),
        "sidebarfoxopen",
        ctx.fg(),
    );
    out.push(Prim::Image {
        rect: Rect::new(r.x + 42.0, r.y + 10.0, 18.0, 18.0),
        image: "bboxy".into(),
        radius: 9.0,
    });
    text(
        out,
        Rect::new(r.x + 70.0, r.y + 10.0, 240.0, 20.0),
        "bboxy450-term",
        STYLE.r#type.body as f32,
        ctx.fg(),
        500,
    );
    text(
        out,
        Rect::new(r.x + 320.0, r.y + 11.0, 300.0, 18.0),
        "swarm  /  Vesper Greywind",
        11.0,
        ctx.muted(),
        400,
    );
    icon(
        out,
        Rect::new(r.right() - 34.0, r.y + 10.0, 18.0, 18.0),
        if ctx.theme == Theme::Dark {
            "sun"
        } else {
            "moon"
        },
        ctx.muted(),
    );
}
fn showcase(r: Rect, n: usize, ctx: &DrawCtx, out: &mut DrawList) {
    let h = STYLE.spacing.title_bar as f32;
    titlebar(Rect::new(r.x, r.y, r.w, h), ctx, out);
    let w = TOKENS.sidebar.width_compact as f32;
    sidebar(Rect::new(r.x, r.y + h, w, r.h - h), n, ctx, out);
    let tree = if n == 15 { tree15() } else { tree3() };
    for (id, rect) in solve_split(
        &tree,
        Rect::new(r.x + w, r.y + h, r.w - w, r.h - h),
        TOKENS.layout.gutter as f32,
    )
    .panes
    {
        pane(&seat(id.0.0 as usize - 1), rect, id == sid(0), ctx, out);
    }
}

fn message(r: Rect, role: &str, ctx: &DrawCtx, out: &mut DrawList) {
    let color = Color::hex(match role {
        "oracle" => TOKENS.color.role.oracle,
        "creativity" => TOKENS.color.role.creativity,
        "unsigned" => TOKENS.color.role.unsigned,
        _ => TOKENS.color.role.executor,
    });
    if role == "oracle" {
        out.rect(r, 4.0, color.alpha(0.07));
    }
    out.rect(Rect::new(r.x, r.y + 4.0, 3.0, r.h - 8.0), 1.0, color);
    text(
        out,
        Rect::new(r.x + 13.0, r.y + 3.0, r.w - 100.0, 15.0),
        format!(
            "{} <{}>",
            if role == "oracle" { NAMES[0] } else { NAMES[1] },
            role.to_uppercase()
        ),
        11.0,
        color,
        600,
    );
    text(
        out,
        Rect::new(r.x + 13.0, r.y + 21.0, r.w - 30.0, 18.0),
        "B claimed. File:line pointers checked. Report is ready.",
        12.0,
        ctx.fg(),
        400,
    );
    text(
        out,
        Rect::new(r.right() - 44.0, r.y + 3.0, 40.0, 15.0),
        "12:07",
        10.0,
        ctx.muted(),
        400,
    );
    out.rect(Rect::new(r.right() - 8.0, r.y + 9.0, 5.0, 5.0), 5.0, color);
}
fn info_card(r: Rect, title: &str, rows: &[&str], ctx: &DrawCtx, out: &mut DrawList) {
    panel(out, r, ctx);
    text(
        out,
        Rect::new(r.x + 16.0, r.y + 12.0, r.w - 32.0, 24.0),
        title,
        16.0,
        ctx.fg(),
        500,
    );
    for (i, row) in rows.iter().enumerate() {
        let y = r.y + 48.0 + i as f32 * 30.0;
        if y + 24.0 > r.bottom() {
            break;
        }
        text(
            out,
            Rect::new(r.x + 16.0, y, r.w - 32.0, 25.0),
            *row,
            12.0,
            ctx.muted(),
            400,
        );
    }
}
fn task_row(r: Rect, ctx: &DrawCtx, out: &mut DrawList) {
    text(
        out,
        Rect::new(r.x + 8.0, r.y + 4.0, r.w - 130.0, 20.0),
        "◕  Renderer harness",
        11.0,
        ctx.fg(),
        500,
    );
    Badge {
        label: "reported",
        selected: false,
        mono: false,
    }
    .draw(
        Rect::new(r.right() - 112.0, r.y + 3.0, 62.0, 18.0),
        ctx,
        out,
    );
    Badge {
        label: "accept",
        selected: true,
        mono: false,
    }
    .draw(Rect::new(r.right() - 47.0, r.y + 3.0, 44.0, 18.0), ctx, out);
}

pub fn draw(id: &str, state: &str, r: Rect, ctx: &DrawCtx, out: &mut DrawList) -> Result<()> {
    let local = if state == "unfocused" {
        ctx.dimmed()
    } else {
        *ctx
    };
    let ctx = &local;
    let row = ctx.density.tokens().row_h as f32;
    match id {
        "showcase-3" => showcase(r, 3, ctx, out),
        "showcase-15" => showcase(r, 15, ctx, out),
        "seat_row" => {
            let n = if state == "zsh seat" || state == "long names" || state == "subtitle opt-in" {
                1
            } else if state == "unread badge" {
                3
            } else {
                4
            };
            for i in 0..n {
                let mut s = seat(i);
                s.name = NAMES[i % 4].into();
                s.title = [
                    "gpu research",
                    "messaging spec",
                    "ux divergence",
                    "harness hooks",
                ][i % 4]
                    .into();
                s.activity = if state == "focused × state" {
                    [
                        Activity::Active,
                        Activity::Idle,
                        Activity::Waiting(WaitKind::Permission),
                        Activity::Errored,
                    ][i]
                } else {
                    activity(state)
                };
                s.grade = Grade::Reported;
                s.where_ = Where::Local;
                if state == "unread badge" {
                    s.unread = Some((
                        if i == 0 { 3 } else { 12 },
                        Role {
                            name: "oracle".into(),
                            class: RoleClass::Oracle,
                        },
                    ));
                }
                if state == "long names" {
                    s.name = "Mireille Bellweather-Lockridge".into();
                }
                if state == "zsh seat" {
                    s = seat(7);
                }
                let rr = Rect::new(r.x, r.y + i as f32 * (row + 4.0), r.w.min(300.0), row);
                seat_row(&s, rr, state == "focused × state", Some(i + 1), ctx, out);
            }
        }
        "glyph" => {
            if state.starts_with("grades") {
                for (i, grade) in [Grade::Reported, Grade::Shim, Grade::Inferred]
                    .into_iter()
                    .enumerate()
                {
                    for (j, a) in [
                        Activity::Active,
                        Activity::Idle,
                        Activity::Waiting(WaitKind::Permission),
                        Activity::Errored,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        StateGlyph {
                            activity: a,
                            grade,
                            color: col(0, ctx),
                        }
                        .draw(
                            Rect::new(
                                r.x + i as f32 * 120.0 + j as f32 * 24.0,
                                r.y,
                                ctx.density.tokens().glyph as f32,
                                ctx.density.tokens().glyph as f32,
                            ),
                            ctx,
                            out,
                        );
                    }
                    text(
                        out,
                        Rect::new(r.x + i as f32 * 120.0, r.y + 28.0, 110.0, 14.0),
                        ["reported", "shim", "inferred"][i],
                        10.0,
                        ctx.muted(),
                        400,
                    );
                }
            } else if state.starts_with("waiting kinds") {
                for (i, k) in [WaitKind::Permission, WaitKind::Question, WaitKind::Auth]
                    .into_iter()
                    .enumerate()
                {
                    StateGlyph {
                        activity: Activity::Waiting(k),
                        grade: Grade::Reported,
                        color: col(0, ctx),
                    }
                    .draw(
                        Rect::new(r.x + i as f32 * 88.0, r.y, 12.0, 12.0),
                        ctx,
                        out,
                    );
                    text(
                        out,
                        Rect::new(r.x + i as f32 * 88.0, r.y + 24.0, 80.0, 14.0),
                        ["permission", "question", "auth"][i],
                        10.0,
                        ctx.muted(),
                        400,
                    );
                }
            } else if state == "all four in a row" {
                for (i, a) in [
                    Activity::Active,
                    Activity::Idle,
                    Activity::Waiting(WaitKind::Permission),
                    Activity::Errored,
                ]
                .into_iter()
                .enumerate()
                {
                    let g = ctx.density.tokens().glyph as f32;
                    StateGlyph {
                        activity: a,
                        grade: Grade::Reported,
                        color: col(0, ctx),
                    }
                    .draw(
                        Rect::new(r.x + i as f32 * (g + 18.0), r.y, g, g),
                        ctx,
                        out,
                    );
                }
            } else {
                let mut x = r.x;
                for (label, g) in [
                    ("touch 16", 16.0),
                    ("comfortable 12", 12.0),
                    ("compact 10", 10.0),
                ] {
                    StateGlyph {
                        activity: activity(state),
                        grade: Grade::Reported,
                        color: col(0, ctx),
                    }
                    .draw(Rect::new(x + 20.0, r.y, g, g), ctx, out);
                    text(
                        out,
                        Rect::new(x, r.y + 24.0, 86.0, 14.0),
                        label,
                        10.0,
                        ctx.muted(),
                        400,
                    );
                    x += 95.0;
                }
            }
        }
        "harness-icon" => {
            let mut x = r.x;
            for size in [12.0, 16.0, 20.0, 32.0, 64.0] {
                icon(
                    out,
                    Rect::new(x, r.y + 64.0 - size, size, size),
                    state,
                    ctx.fg(),
                );
                text(
                    out,
                    Rect::new(x, r.y + 70.0, size.max(20.0), 14.0),
                    size.to_string(),
                    10.0,
                    ctx.muted(),
                    400,
                );
                x += size + 28.0;
            }
            icon(
                out,
                Rect::new(x, r.y + 32.0, 32.0, 32.0),
                state,
                Color::hex(ctx.theme.palette().accent),
            );
        }
        "mini_grid" => {
            let base = if state == "single" {
                SplitNode::Leaf(sid(0))
            } else if state == "V" {
                split(
                    Axis::Row,
                    0.5,
                    SplitNode::Leaf(sid(0)),
                    SplitNode::Leaf(sid(1)),
                )
            } else if state == "H" {
                split(
                    Axis::Column,
                    0.5,
                    SplitNode::Leaf(sid(0)),
                    SplitNode::Leaf(sid(1)),
                )
            } else {
                tree3()
            };
            let mut x = r.x;
            for (w, h) in [(24.0, 16.0), (48.0, 32.0), (120.0, 80.0)] {
                mini_grid(
                    &base,
                    Rect::new(x, r.y + (80.0 - h) * 0.5, w, h),
                    if state == "focused cell" {
                        Some(sid(1))
                    } else {
                        None
                    },
                    ctx,
                    out,
                );
                x += w + 24.0;
            }
        }
        "tab_row" => {
            let n = if state.starts_with('1') {
                1
            } else if state.starts_with('8') {
                8
            } else {
                3
            };
            let tree = if n == 1 {
                SplitNode::Leaf(sid(7))
            } else {
                tree3()
            };
            let h = ctx.density.tokens().tab_h as f32;
            if n == 1 {
                seat_row(
                    &seat(7),
                    Rect::new(r.x, r.y, 300.0, row),
                    false,
                    Some(1),
                    ctx,
                    out,
                );
            } else {
                tab_row(
                    &tree,
                    "messaging",
                    Rect::new(r.x, r.y, 300.0, h),
                    state.starts_with("expanded"),
                    ctx,
                    out,
                );
                if state.starts_with("expanded") {
                    for i in 0..3 {
                        seat_row(
                            &seat(i),
                            Rect::new(r.x + 12.0, r.y + h + i as f32 * row, 288.0, row),
                            state.ends_with("focus") && i == 1,
                            Some(i + 1),
                            ctx,
                            out,
                        );
                    }
                }
            }
        }
        "band" => {
            let open = state == "expanded";
            card(
                Rect::new(r.x, r.y, 300.0, if open { 170.0 } else { 36.0 }),
                "term",
                &[0, 1, 3],
                open,
                true,
                ctx,
                out,
            );
        }
        "sidebar" => {
            let width = if state == "phone sheet" {
                340.0
            } else {
                TOKENS.sidebar.width as f32
            };
            sidebar(
                Rect::new(r.x, r.y, width, r.h),
                if state == "10 seats" { 3 } else { 15 },
                ctx,
                out,
            );
        }
        "title_strip" => {
            let n = if state == "focused vs unfocused" {
                8
            } else if state == "three seats, one focused" {
                3
            } else {
                2
            };
            out.rect(r, 12.0, Color::hex(ctx.theme.palette().navy));
            let gap = 28.0;
            let w = (r.w - gap * 3.0) / 2.0;
            let h = if n == 8 {
                110.0
            } else {
                (r.h - gap * 2.0).min(260.0)
            };
            for i in 0..n {
                let mut s = seat(i % 3);
                s.activity = if state.contains("focused") {
                    [
                        Activity::Active,
                        Activity::Idle,
                        Activity::Waiting(WaitKind::Permission),
                        Activity::Errored,
                    ][i % 4]
                } else {
                    activity(state)
                };
                pane(
                    &s,
                    Rect::new(
                        r.x + gap + (i % 2) as f32 * (w + gap),
                        r.y + gap + (i / 2) as f32 * (h + 20.0),
                        w,
                        h,
                    ),
                    i == 0 || n == 8 && i < 4,
                    ctx,
                    out,
                );
            }
        }
        "pane" => pane(
            &seat(0),
            Rect::new(r.x, r.y, r.w.min(700.0), r.h),
            state != "unfocused",
            ctx,
            out,
        ),
        "title_bar" => titlebar(
            Rect::new(r.x, r.y, r.w, STYLE.spacing.title_bar as f32),
            ctx,
            out,
        ),
        "swarm_card" => card(
            Rect::new(r.x, r.y, 300.0, r.h),
            "term",
            &[0, 1, 3, 2, 4],
            true,
            state != "unfocused",
            ctx,
            out,
        ),
        "new_seat_row" => new_seat(Rect::new(r.x, r.y, 300.0, row), ctx, out),
        "closed_strip" => closed_strip(Rect::new(r.x, r.y, 300.0, row), ctx, out),
        "new_swarm_button" => {
            panel(out, Rect::new(r.x, r.y, 260.0, 40.0), ctx);
            text(
                out,
                Rect::new(r.x + 12.0, r.y + 11.0, 230.0, 20.0),
                "+  new swarm",
                12.5,
                ctx.muted(),
                500,
            );
        }
        "message-row" | "stream_dock" => {
            let count = if state == "stream" { 4 } else { 2 };
            for i in 0..count {
                message(
                    Rect::new(r.x, r.y + i as f32 * 60.0, r.w.min(700.0), 54.0),
                    if state == "stream" {
                        ["oracle", "executor", "creativity", "unsigned"][i]
                    } else {
                        state
                    },
                    ctx,
                    out,
                );
            }
        }
        "launcher" => launcher(r, state, ctx, out),
        "settings_tab" => settings(r, state, ctx, out),
        "logo" => {
            // identity PNG only, downscaled; no vector mark exists (operator purge, 2026-10-21)
            let mut x = r.x;
            for size in [16.0, 32.0, 64.0, 128.0, 256.0, 512.0] {
                out.push(Prim::Image {
                    rect: Rect::new(x, r.y, size, size),
                    image: "bboxy".into(),
                    radius: 0.0,
                });
                x += size + 28.0;
            }
        }
        "toolbar-icon" => {
            if state == "in our title bar" {
                titlebar(Rect::new(r.x, r.y, 640.0, 38.0), ctx, out);
                out.rect(
                    Rect::new(r.x, r.y + 38.0, 640.0, 120.0),
                    0.0,
                    Color::hex(ctx.theme.palette().bg),
                );
            } else if state == "in the menubar" {
                out.rect(Rect::new(r.x, r.y, 520.0, 24.0), 6.0, Color::hex("#1C1C1E"));
                icon(
                    out,
                    Rect::new(r.x + 10.0, r.y + 4.0, 16.0, 16.0),
                    "sidebarfoxopen",
                    Color::WHITE,
                );
                text(
                    out,
                    Rect::new(r.x + 310.0, r.y + 4.0, 200.0, 18.0),
                    "◐ 3 waiting   Wed 12:14",
                    12.0,
                    Color::WHITE.alpha(0.7),
                    400,
                );
            } else {
                let name = if state.contains("open") {
                    "sidebarfoxopen"
                } else if state == "ear pair" {
                    "ears"
                } else {
                    "sidebarfox"
                };
                let mut x = r.x;
                let sizes: &[f32] = if state == "sizes ladder" {
                    &[12.0, 16.0, 20.0, 24.0]
                } else {
                    &[12.0, 16.0, 20.0, 24.0, 32.0, 64.0]
                };
                let baseline = if state == "sizes ladder" { 24.0 } else { 64.0 };
                for size in sizes {
                    if state == "sizes ladder" {
                        for (i, mark) in ["sidebarfox", "sidebarfoxopen", "gear"].iter().enumerate()
                        {
                            icon(
                                out,
                                Rect::new(
                                    x + i as f32 * (size + 8.0),
                                    r.y + baseline - size,
                                    *size,
                                    *size,
                                ),
                                mark,
                                ctx.fg(),
                            );
                        }
                        text(
                            out,
                            Rect::new(x + size + 8.0, r.y + baseline + 6.0, 32.0, 14.0),
                            size.to_string(),
                            10.0,
                            ctx.muted(),
                            400,
                        );
                        x += size * 3.0 + 16.0 + 28.0;
                    } else {
                        let width = (*size).max(12.0);
                        icon(
                            out,
                            Rect::new(x, r.y + baseline - size, *size, *size),
                            name,
                            ctx.fg(),
                        );
                        text(
                            out,
                            Rect::new(x, r.y + baseline + 6.0, width.max(22.0), 14.0),
                            size.to_string(),
                            10.0,
                            ctx.muted(),
                            400,
                        );
                        x += width + 28.0;
                    }
                }
                if state != "sizes ladder" {
                    icon(
                        out,
                        Rect::new(x, r.y + baseline - 32.0, 32.0, 32.0),
                        name,
                        Color::hex(ctx.theme.palette().accent),
                    );
                    text(
                        out,
                        Rect::new(x, r.y + baseline + 6.0, 50.0, 14.0),
                        "tinted",
                        10.0,
                        ctx.muted(),
                        400,
                    );
                }
            }
        }
        "tokens" => tokens(r, state, ctx, out),
        "tile" => IconTile {
            icon: "claude",
            color: Color::hex("#D97757"),
        }
        .draw(Rect::new(r.x, r.y, 30.0, 30.0), ctx, out),
        "badge" | "kbd" => Badge {
            label: if id == "kbd" { "⌘1" } else { "3" },
            selected: state == "focused",
            mono: true,
        }
        .draw(Rect::new(r.x, r.y, 32.0, 22.0), ctx, out),
        "role_mark" => {
            RoleMark(seat(0).role.as_ref().unwrap()).draw(Rect::new(r.x, r.y, 18.0, 18.0), ctx, out)
        }
        "where_mark" => {
            WhereMark(&Where::Ssh { host: "rig".into() }).draw(
                Rect::new(r.x, r.y, 16.0, 16.0),
                ctx,
                out,
            );
            WhereMark(&Where::Cloud {
                provider: "demo".into(),
                host: "eu-2".into(),
            })
            .draw(Rect::new(r.x + 40.0, r.y, 16.0, 16.0), ctx, out);
        }
        "swatch" => Swatch(col(0, ctx)).draw(Rect::new(r.x, r.y, 12.0, 12.0), ctx, out),
        "hairline" => out.line(
            [r.x, r.y + 5.0],
            [r.x + 240.0, r.y + 5.0],
            1.0,
            ctx.hairline(),
        ),
        "pill" => Pill(&[
            ("Launch", "↵"),
            ("Split right", "⌘↵"),
            ("Split down", "⌘⇧↵"),
        ])
        .draw(Rect::new(r.x, r.y, 450.0, 34.0), ctx, out),
        "crumb" => {
            out.rect(
                Rect::new(r.x, r.y, 110.0, 24.0),
                4.0,
                ctx.color(Color::hex(ctx.theme.palette().fg).alpha(0.09)),
            );
            text(
                out,
                Rect::new(r.x + 6.0, r.y + 4.0, 100.0, 16.0),
                "SSH  ›  rig",
                11.0,
                ctx.muted(),
                400,
            );
        }
        "chevron" => Chevron.draw(Rect::new(r.x, r.y, 12.0, 18.0), ctx, out),
        "bumper" => out.push(Prim::Rect {
            rect: Rect::new(r.x, r.y, 260.0, 4.0),
            radii: [0.0, 0.0, 12.0, 12.0],
            fill: ctx.identity(col(0, ctx)),
            stroke: None,
            inner_highlight: None,
            shadow: None,
        }),
        "close_dot" => close_dot(
            Rect::new(r.x, r.y, 12.0, 12.0),
            state == "focused",
            ctx,
            out,
        ),
        "mute_toggle" | "muted_mark" => icon(
            out,
            Rect::new(r.x, r.y, 14.0, 14.0),
            if state == "focused" || id == "muted_mark" {
                "muted"
            } else {
                "unmuted"
            },
            ctx.color(Color::hex(TOKENS.color.state.waiting)),
        ),
        "ghost" => {
            let rr = Rect::new(r.x, r.y, 160.0, 28.0);
            out.rect(
                rr,
                999.0,
                Color::hex(ctx.theme.palette().surface).alpha(STYLE.material.panel_alpha as f32),
            );
            text(
                out,
                Rect::new(r.x + 10.0, r.y + 5.0, 140.0, 18.0),
                NAMES[1],
                12.5,
                col(1, ctx),
                600,
            );
        }
        "drop_zone" => {
            pane(&seat(0), Rect::new(r.x, r.y, 500.0, 100.0), true, ctx, out);
            out.rect(
                Rect::new(r.x + 250.0, r.y, 250.0, 100.0),
                0.0,
                Color::hex(ctx.theme.palette().accent).alpha(0.22),
            );
        }
        "divider" => {
            out.rect(
                Rect::new(r.x, r.y, 120.0, 80.0),
                0.0,
                Color::hex(ctx.theme.palette().surface_2),
            );
            out.rect(
                Rect::new(r.x + 122.0, r.y, 120.0, 80.0),
                0.0,
                Color::hex(ctx.theme.palette().surface_2),
            );
        }
        "cursor" => out.rect(Rect::new(r.x, r.y, 8.0, 16.0), 0.0, ctx.fg()),
        "glow_canvas" => {
            pane(&seat(0), Rect::new(r.x, r.y, 450.0, 100.0), true, ctx, out);
            text(
                out,
                Rect::new(r.x + 12.0, r.y + 40.0, 420.0, 20.0),
                "FX pass deferred to W3.3",
                12.0,
                ctx.muted(),
                400,
            );
        }
        "gap_marker" => {
            let caption = if state == "retained" {
                "earlier output evicted · retained from 000042"
            } else {
                "earlier output unavailable"
            };
            out.line(
                [r.x, r.y + 9.0],
                [r.x + 60.0, r.y + 9.0],
                1.0,
                ctx.hairline(),
            );
            out.text(
                Rect::new(r.x + 70.0, r.y, r.w.min(440.0), 18.0),
                caption,
                TextStyle {
                    scale: TypeScale::Custom(10.0),
                    mono: true,
                    ..Default::default()
                },
                ctx.muted(),
            );
            out.line(
                [r.x + 500.0, r.y + 9.0],
                [r.x + r.w.min(700.0), r.y + 9.0],
                1.0,
                ctx.hairline(),
            );
        }
        "tick_ladder" => {
            let marks = match state {
                "sender-stored" => "✓",
                "recipient-stored" | "native-queued" | "native-written" => "✓✓",
                "native-unknown" => "✓◌",
                _ => "✓✓✓",
            };
            out.text(
                Rect::new(r.x, r.y, 60.0, 18.0),
                marks,
                TextStyle {
                    scale: TypeScale::Custom(11.0),
                    mono: true,
                    ..Default::default()
                },
                ctx.muted()
                    .alpha(if state == "native-queued" { 0.72 } else { 1.0 }),
            );
        }
        "nested_subagent_rows" => {
            let n = if state == "overflow" { 7 } else { 2 };
            for i in 0..n {
                let y = r.y + i as f32 * 20.0;
                out.line([r.x + 12.0, y], [r.x + 12.0, y + 20.0], 1.0, ctx.hairline());
                out.line(
                    [r.x + 12.0, y + 10.0],
                    [r.x + 22.0, y + 10.0],
                    1.0,
                    ctx.hairline(),
                );
                StateGlyph {
                    activity: if i == 1 {
                        Activity::Waiting(WaitKind::Question)
                    } else {
                        Activity::Active
                    },
                    grade: if state == "codex" {
                        Grade::Shim
                    } else {
                        Grade::Reported
                    },
                    color: col(0, ctx),
                }
                .draw(Rect::new(r.x + 28.0, y + 4.0, 11.0, 11.0), ctx, out);
                text(
                    out,
                    Rect::new(r.x + 48.0, y + 2.0, 350.0, 18.0),
                    if i == 1 {
                        "Check assumptions"
                    } else {
                        "Inspect renderer"
                    },
                    11.0,
                    ctx.fg(),
                    400,
                );
            }
            if state == "overflow" {
                text(
                    out,
                    Rect::new(r.x + 28.0, r.y + 140.0, 300.0, 18.0),
                    "+3 running",
                    11.0,
                    ctx.muted(),
                    400,
                );
            }
        }
        "task_row" => task_row(Rect::new(r.x, r.y, 500.0, 28.0), ctx, out),
        "epic_group" => info_card(
            Rect::new(r.x, r.y, 700.0, r.h),
            "EPIC  Render terminal   2/4",
            &[
                "✓ SPEC  Renderer · Vesper",
                "◕ TASK  Native gallery · Nyx",
                "◔ TASK  Chrome parity · Nyx",
            ],
            ctx,
            out,
        ),
        "artifact_row" => {
            text(
                out,
                Rect::new(r.x, r.y, 460.0, 25.0),
                "▧  IMAGE   target/render/smoke.png   a723f1",
                11.0,
                ctx.muted(),
                400,
            );
            Badge {
                label: NAMES[1],
                selected: false,
                mono: false,
            }
            .draw(Rect::new(r.x + 470.0, r.y, 130.0, 22.0), ctx, out);
        }
        "responsible_chip" => Badge {
            label: NAMES[1],
            selected: false,
            mono: false,
        }
        .draw(Rect::new(r.x, r.y, 135.0, 22.0), ctx, out),
        "touched_count" => {
            icon(
                out,
                Rect::new(r.x, r.y + 3.0, 12.0, 12.0),
                "archive",
                ctx.muted(),
            );
            text(
                out,
                Rect::new(r.x + 18.0, r.y, 45.0, 20.0),
                "2 artifacts",
                10.0,
                ctx.muted(),
                400,
            );
        }
        "wikilink_chip" => {
            Badge {
                label: "renderer   derives:0.8",
                selected: false,
                mono: true,
            }
            .draw(Rect::new(r.x, r.y, 190.0, 24.0), ctx, out);
        }
        "provenance_card" => info_card(
            Rect::new(r.x, r.y, 700.0, r.h),
            "Provenance",
            &[
                "SPEC  Renderer",
                "created by  Vesper Greywind     updated by  Nyx Copperwing",
                "renderer  derives:0.8      terminal-core  constrains:1.0",
                "a723f1c9  ✓ signed by Boxy's GPG primary",
                "Activity log · hash chained",
                "13:40  Nyx Copperwing  created  renderer.md",
                "23:59  Daily checkpoint · signed",
            ],
            ctx,
            out,
        ),
        "subscriptions_card" => info_card(
            Rect::new(r.x, r.y, 700.0, r.h),
            "Subscriptions",
            &[
                "Vesper Greywind   task/*  AUTO · delegated tasks",
                "seat/*/state     doc/specs/**",
                "Nyx Copperwing   task/w2-1  AUTO · delegated task",
                "doc/specs/mux-daemon  AUTO · derived_from",
                "artifact/*/for:w2-1",
            ],
            ctx,
            out,
        ),
        "external_lane" => info_card(
            Rect::new(r.x, r.y, 700.0, r.h),
            "External events",
            &[
                "github   PR #14 review requested             13:45",
                "ci       macos-15 · render smoke · passed    13:58",
                "cloud    eu-2 · seat reconnected             14:10",
            ],
            ctx,
            out,
        ),
        "graph_tab" => {
            info_card(
                Rect::new(r.x, r.y, r.w.min(760.0), 160.0),
                "Agents",
                &[
                    "Vesper Greywind · oracle · reviewing",
                    "Nyx Copperwing · executor · running · Renderer harness",
                    "Rook Ashwhisker · executor · blocked on transport",
                ],
                ctx,
                out,
            );
            info_card(
                Rect::new(r.x, r.y + 180.0, r.w.min(760.0), 180.0),
                "Work tree",
                &[
                    "EPIC Renderer   2/4",
                    "  ✓ SPEC Terminal core",
                    "  ◕ TASK Native gallery · Nyx",
                    "    IMAGE target/bboxybook/sidebar.png · responsible Nyx",
                ],
                ctx,
                out,
            );
            info_card(
                Rect::new(r.x, r.y + 380.0, r.w.min(760.0), 160.0),
                "Artifact history",
                &[
                    "13:40  Nyx  created  renderer.md  reported",
                    "13:52  Nyx  claimed  smoke.png  reported",
                ],
                ctx,
                out,
            );
        }
        "banner" => {
            text(
                out,
                Rect::new(r.x, r.y, r.w, 22.0),
                "Split right",
                12.0,
                ctx.muted(),
                500,
            );
            mini_grid(
                &tree3(),
                Rect::new(r.x, r.y + 30.0, 240.0, 70.0),
                Some(sid(1)),
                ctx,
                out,
            );
        }
        _ => return Err(format!("unimplemented gallery component {id}").into()),
    }
    Ok(())
}
fn launcher(r: Rect, state: &str, ctx: &DrawCtx, out: &mut DrawList) {
    let w = r.w.min(760.0);
    let h = r.h.min(480.0);
    let rr = Rect::new(r.x, r.y, w, h);
    out.push(Prim::Rect {
        rect: rr,
        radii: [STYLE.radius.panel as f32; 4],
        fill: Color::hex(ctx.theme.palette().surface).alpha(STYLE.material.panel_alpha as f32),
        stroke: Some((1.0, ctx.hairline())),
        inner_highlight: Some(Color::WHITE.alpha(STYLE.material.inner_highlight_alpha as f32)),
        shadow: Some((20.0, 40.0, Color::BLACK.alpha(0.45))),
    });
    let query = if state.starts_with("typed") {
        state.trim_start_matches("typed ").trim_matches('"')
    } else {
        "Search harnesses, seats, settings…"
    };
    text(
        out,
        Rect::new(r.x + 24.0, r.y + 20.0, w - 48.0, 34.0),
        query,
        22.0,
        ctx.muted(),
        400,
    );
    out.line(
        [r.x, r.y + 72.0],
        [r.x + w, r.y + 72.0],
        1.0,
        ctx.hairline(),
    );
    text(
        out,
        Rect::new(r.x + 24.0, r.y + 86.0, w - 48.0, 20.0),
        "Harnesses",
        12.0,
        ctx.muted(),
        500,
    );
    let mut y = r.y + 118.0;
    if state != "no match" {
        for (i, (label, name)) in [
            ("Claude", "claude"),
            ("Codex", "codex"),
            ("pi", "pi"),
            ("omp", "omp"),
            ("zsh", "zsh"),
        ]
        .iter()
        .enumerate()
        {
            if state.contains("\"co\"") && *name != "codex" {
                continue;
            }
            if state.contains("\"z\"") && *name != "zsh" {
                continue;
            }
            let rr = Rect::new(r.x + 10.0, y, w - 20.0, 46.0);
            if i == 0 {
                out.rect(
                    rr,
                    STYLE.radius.row as f32,
                    ctx.color(
                        Color::hex(ctx.theme.palette().fg).alpha(STYLE.tint.selection_alpha as f32),
                    ),
                );
            }
            IconTile {
                icon: name,
                color: col(i, ctx),
            }
            .draw(Rect::new(rr.x + 14.0, y + 8.0, 30.0, 30.0), ctx, out);
            text(
                out,
                Rect::new(rr.x + 56.0, y + 11.0, rr.w - 140.0, 24.0),
                *label,
                16.0,
                ctx.fg(),
                500,
            );
            Chevron.draw(Rect::new(rr.right() - 28.0, y + 14.0, 12.0, 18.0), ctx, out);
            y += 46.0;
        }
    } else {
        text(
            out,
            Rect::new(r.x + 24.0, y, w - 48.0, 24.0),
            "No match",
            15.0,
            ctx.muted(),
            400,
        );
    }
    text(
        out,
        Rect::new(r.x + 24.0, rr.bottom() - 45.0, 220.0, 22.0),
        NAMES[1],
        14.0,
        ctx.fg(),
        600,
    );
}
fn settings(r: Rect, state: &str, ctx: &DrawCtx, out: &mut DrawList) {
    let nav = 150.0;
    for (i, s) in [
        "Account",
        "Cloud",
        "Machines",
        "Roles",
        "Harnesses",
        "Appearance",
        "Shortcuts",
    ]
    .iter()
    .enumerate()
    {
        if *s == state {
            out.rect(
                Rect::new(r.x, r.y + i as f32 * 34.0, nav - 12.0, 30.0),
                8.0,
                Color::hex(ctx.theme.palette().fg).alpha(STYLE.tint.selection_alpha as f32),
            );
        }
        text(
            out,
            Rect::new(r.x + 10.0, r.y + 7.0 + i as f32 * 34.0, nav - 20.0, 22.0),
            *s,
            12.5,
            ctx.muted(),
            500,
        );
    }
    let rr = Rect::new(r.x + nav + 20.0, r.y, (r.w - nav - 20.0).min(800.0), r.h);
    let rows: &[&str] = match state {
        "Account" => &[
            "Boxy (bboxy450) <bboxy450@gmail.com>",
            "9B07 E849 480C 6832 8CA2 0684 844E CC05 87DE 7AB0",
            "Primary secret stays in GnuPG / gpg-agent",
            "Unlock through pinentry · agent cache",
            "Device certificates · enroll by signed request",
            "Revoked here; peers learn on next contact",
            "Recovery restores identity only",
            "Google · link for cloud (deferred)",
            "Per-seat commit signing · off / assumed",
        ],
        "Roles" => &[
            "oracle   built-in   class oracle",
            "executor   built-in   class worker",
            "creativity   built-in   class worker",
            "+ new role",
            "Changing class re-issues this role's seat grants",
        ],
        "Harnesses" | "new harness" => &[
            "Claude   OSC 7501",
            "Codex   OSC 7501",
            "zsh   output cadence",
            "+ new harness",
            "Generic input is guarded; never submits on its own",
        ],
        "Appearance" => &[
            "Theme  auto / dark / light",
            "Density  touch / comfortable / cozy / compact",
            "Gutter  2 / 4 px",
            "Glow  enabled",
            "Subtitles  opt-in",
        ],
        "Shortcuts" => &[
            "⌘T   New seat",
            "⌘D   Split right",
            "⌘⇧D  Split below",
            "⌘M   Mute seat",
            "⌘Z   Undo close",
            "⌘,   Settings",
        ],
        "Machines" => &[
            "rig   ~/.ssh/config",
            "alpha   ~/.ssh/config",
            "SSH machines are read from config.toml",
        ],
        _ => &[
            "Cloud providers",
            "Google is a link for cloud, never the account identity",
            "Provider setup is deferred",
        ],
    };
    info_card(rr, state, rows, ctx, out);
}
fn tokens(r: Rect, state: &str, ctx: &DrawCtx, out: &mut DrawList) {
    let p = ctx.theme.palette();
    let values: Vec<(&str, Color)> = if state == "state + role" {
        vec![
            ("state.idle", Color::hex(TOKENS.color.state.idle)),
            ("state.waiting", Color::hex(TOKENS.color.state.waiting)),
            ("state.errored", Color::hex(TOKENS.color.state.errored)),
            ("role.oracle", Color::hex(TOKENS.color.role.oracle)),
            ("role.executor", Color::hex(TOKENS.color.role.executor)),
            ("role.creativity", Color::hex(TOKENS.color.role.creativity)),
        ]
    } else if state == "seat colours" {
        (0..36).map(|i| ("seat", col(i, ctx))).collect()
    } else {
        vec![
            ("bg", Color::hex(p.bg)),
            ("surface", Color::hex(p.surface)),
            ("surface_2", Color::hex(p.surface_2)),
            ("line", Color::hex(p.line)),
            ("fg", Color::hex(p.fg)),
            ("fg_dim", Color::hex(p.fg_dim)),
            ("accent", Color::hex(p.accent)),
            ("accent_soft", Color::hex(p.accent_soft)),
            ("navy", Color::hex(p.navy)),
            ("sidebar", Color::hex(p.sidebar)),
        ]
    };
    for (i, (name, c)) in values.iter().enumerate() {
        let x = r.x + (i % 6) as f32 * 175.0;
        let y = r.y + (i / 6) as f32 * 100.0;
        out.rect(Rect::new(x, y, 160.0, 60.0), 6.0, *c);
        text(
            out,
            Rect::new(x, y + 66.0, 160.0, 18.0),
            *name,
            11.0,
            ctx.muted(),
            400,
        );
    }
}
