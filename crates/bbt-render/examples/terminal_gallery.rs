//! Offline parser -> cell snapshot -> actual Metal cells pass.
#![allow(
    clippy::disallowed_methods,
    reason = "offline gallery captures, not application UI code"
)]
use bboxy_firehose::{COLS, ROWS, SCENES, scene_bytes};
use bbt_core::{
    Color, Column, Line, NamedColor, Term, ansi,
    term::{TermMode, cell::Flags},
};
use bbt_render::{
    FontConfig, HeadlessRenderer, Scene, TextRun,
    cells::terminal::{self, Cell, Cursor, CursorShape, Grid, TerminalRenderer, Underline},
    device::DeviceContext,
    write_png,
};
use serde_json::json;
use std::{fs, path::PathBuf};

fn rgb(value: u32) -> [u8; 4] {
    [(value >> 16) as u8, (value >> 8) as u8, value as u8, 255]
}
fn palette(index: u8) -> [u8; 4] {
    const ANSI: [u32; 16] = [
        0x2e3436, 0xcc0000, 0x4e9a06, 0xc4a000, 0x3465a4, 0x75507b, 0x06989a, 0xd3d7cf, 0x555753,
        0xef2929, 0x8ae234, 0xfce94f, 0x729fcf, 0xad7fa8, 0x34e2e2, 0xeeeeec,
    ];
    match index {
        0..=15 => rgb(ANSI[usize::from(index)]),
        16..=231 => {
            let n = index - 16;
            let c = [0, 95, 135, 175, 215, 255];
            [
                c[usize::from(n / 36)],
                c[usize::from(n / 6 % 6)],
                c[usize::from(n % 6)],
                255,
            ]
        }
        _ => {
            let value = 8 + (index - 232) * 10;
            [value, value, value, 255]
        }
    }
}
fn color(value: Color, light: bool, bold: bool) -> [u8; 4] {
    let fg = if light {
        [23, 31, 43, 255]
    } else {
        [232, 237, 247, 255]
    };
    let bg = if light {
        [242, 244, 248, 255]
    } else {
        [15, 20, 29, 255]
    };
    match value {
        Color::Indexed(i) => palette(if bold && i < 8 { i + 8 } else { i }),
        Color::Spec(v) => [v.r, v.g, v.b, 255],
        Color::Named(NamedColor::Background) => bg,
        Color::Named(n) if (n as u16) < 16 => {
            let i = n as u8;
            palette(if bold && i < 8 { i + 8 } else { i })
        }
        Color::Named(_) => fg,
    }
}
fn snapshot(term: &Term, light: bool, focused: bool) -> Grid {
    let mut cells = Vec::new();
    for row in 0..ROWS {
        for col in 0..COLS {
            let c = &term.grid()[Line(row as i32)][Column(col)];
            let f = c.flags;
            let width = if f.contains(Flags::WIDE_CHAR_SPACER) {
                0
            } else if f.contains(Flags::WIDE_CHAR) {
                2
            } else {
                1
            };
            let text = if width == 0 {
                String::new()
            } else {
                std::iter::once(c.c)
                    .chain(c.zerowidth().unwrap_or_default().iter().copied())
                    .collect()
            };
            let bold = f.contains(Flags::BOLD);
            let mut foreground = color(c.fg, light, bold);
            let mut background = color(c.bg, light, false);
            if f.contains(Flags::INVERSE) {
                std::mem::swap(&mut foreground, &mut background);
            }
            cells.push(Cell {
                text,
                width,
                foreground,
                background,
                underline_color: c
                    .underline_color()
                    .map_or(foreground, |v| color(v, light, false)),
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
            });
        }
    }
    let point = term.grid().cursor.point;
    let shape = term.cursor_style().shape;
    Grid {
        columns: COLS as u16,
        rows: ROWS as u16,
        cells,
        cursor: (term.mode().contains(TermMode::SHOW_CURSOR) && shape != ansi::CursorShape::Hidden)
            .then_some(Cursor {
                column: point.column.0 as u16,
                row: point.line.0 as u16,
                shape: match shape {
                    ansi::CursorShape::Beam => CursorShape::Bar,
                    ansi::CursorShape::Underline => CursorShape::Underline,
                    _ => CursorShape::Block,
                },
                focused: focused && shape != ansi::CursorShape::HollowBlock,
                color: color(Color::Named(NamedColor::Foreground), light, false),
            }),
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let baseline = args.iter().any(|a| a == "--baseline");
    let out = args.iter().find(|a| !a.starts_with('-')).map_or_else(
        || PathBuf::from("crates/bboxy-firehose/reference/gallery/native"),
        PathBuf::from,
    );
    fs::create_dir_all(&out)?;
    let context = pollster::block_on(DeviceContext::new())?;
    let legacy = if baseline {
        Some(HeadlessRenderer::new()?)
    } else {
        None
    };
    let mut report = Vec::new();
    for scale in [1, 2] {
        for light in [false, true] {
            let font = FontConfig {
                family: "Menlo".into(),
                size: 13.,
                scale,
            };
            let mut renderer = TerminalRenderer::new(
                &context,
                terminal::Options {
                    font: font.clone(),
                    ..Default::default()
                },
            )?;
            for scene in SCENES {
                let mut term = Term::new(COLS, ROWS, 0);
                term.advance(&scene_bytes(scene));
                let grid = snapshot(&term, light, scene.name.starts_with("cursor-"));
                let variant = format!(
                    "{}-{}-{}x",
                    scene.name,
                    if light { "light" } else { "dark" },
                    scale
                );
                if let Some(legacy) = &legacy {
                    let metrics = bbt_render::text::Font::new(&font)?.metrics();
                    let mut old = Scene {
                        columns: COLS as u16,
                        rows: ROWS as u16,
                        cell_size: [metrics.cell_width, metrics.cell_height],
                        backgrounds: grid.cells.iter().map(|c| c.background).collect(),
                        text: Vec::new(),
                        font: font.clone(),
                    };
                    for (i, cell) in grid.cells.iter().enumerate() {
                        if cell.width > 0 && !cell.text.trim().is_empty() {
                            old.text.push(TextRun {
                                column: (i % COLS) as u16,
                                row: (i / COLS) as u16,
                                text: cell.text.clone(),
                                foreground: cell.foreground,
                            });
                        }
                    }
                    let [width, height] = old.pixel_size();
                    let actual = legacy.render_to_rgba(&old, width, height);
                    let legacy_error = actual.as_ref().err().map(ToString::to_string);
                    let unsupported = old.text.iter().filter(|r| !r.text.is_ascii()).count();
                    let rgba = match actual {
                        Ok(image) => image,
                        Err(_) => {
                            for run in &mut old.text {
                                if !run.text.is_ascii() {
                                    run.text = "?".into();
                                }
                            }
                            legacy.render_to_rgba(&old, width, height)?
                        }
                    };
                    let file = format!("{variant}.png");
                    write_png(out.join(&file), &rgba, width, height)?;
                    report.push(json!({"name":scene.name,"variant":variant,"file":file,"width":width,"height":height,"legacyError":legacy_error,"diagnosticReplacements":unsupported,"baselineKind":if legacy_error.is_some(){"ASCII replacement diagnostic; original rendering failed"}else{"actual legacy cells pass"}}));
                } else {
                    let image = renderer.render_to_rgba(&context, &grid)?;
                    let file = format!("{variant}.png");
                    write_png(out.join(&file), &image.rgba, image.width, image.height)?;
                    report.push(json!({"name":scene.name,"variant":variant,"file":file,"width":image.width,"height":image.height,"faces":renderer.effective_faces().collect::<Vec<_>>(),"cachedGlyphs":renderer.cached_glyphs()}));
                }
                println!("rendered {variant}");
            }
        }
    }
    fs::write(
        out.join("manifest.json"),
        serde_json::to_vec_pretty(
            &json!({"baseline":baseline,"adapter":format!("{:?}",context.adapter_info),"cases":report}),
        )?,
    )?;
    Ok(())
}
