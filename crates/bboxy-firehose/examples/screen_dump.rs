//! Independent parser output for comparison with the vendored xterm reference.
use bboxy_firehose::{COLS, ROWS, SCENES, scene_bytes};
use bbt_core::{Color, Column, Line, Term, ansi::CursorShape, term::cell::Flags};
use serde_json::{Value, json};

fn colour(color: Color) -> Value {
    match color {
        Color::Indexed(index) => json!(["palette", index]),
        Color::Named(name) if (name as u16) < 16 => json!(["palette", name as u16]),
        Color::Named(_) => json!(["default"]),
        Color::Spec(rgb) => json!([
            "rgb",
            (u32::from(rgb.r) << 16) | (u32::from(rgb.g) << 8) | u32::from(rgb.b)
        ]),
    }
}

fn main() {
    let scenes: Vec<_> = SCENES.iter().map(|scene| {
        let mut term = Term::new(COLS, ROWS, 0);
        term.advance(&scene_bytes(scene));
        let grid = term.grid();
        let mut cells = Vec::new();
        for row in 0..ROWS {
            for col in 0..COLS {
                let cell = &grid[Line(row as i32)][Column(col)];
                let width = if cell.flags.contains(Flags::WIDE_CHAR_SPACER) { 0 } else if cell.flags.contains(Flags::WIDE_CHAR) { 2 } else { 1 };
                let chars = if width == 0 { String::new() } else {
                    std::iter::once(cell.c).chain(cell.zerowidth().unwrap_or_default().iter().copied()).collect()
                };
                cells.push(json!([
                    chars, width, colour(cell.fg), colour(cell.bg),
                    cell.flags.contains(Flags::BOLD), cell.flags.contains(Flags::DIM),
                    cell.flags.contains(Flags::ITALIC), cell.flags.contains(Flags::INVERSE),
                    cell.flags.intersects(Flags::ALL_UNDERLINES), cell.flags.contains(Flags::STRIKEOUT),
                ]));
            }
        }
        let cursor = term.cursor_style();
        let shape = match cursor.shape {
            CursorShape::Beam => "bar",
            CursorShape::Underline => "underline",
            CursorShape::Block => "block",
            CursorShape::HollowBlock => "outline",
            CursorShape::Hidden => "hidden",
        };
        json!({"name": scene.name, "cursor": [grid.cursor.point.line.0, grid.cursor.point.column.0], "cursorStyle": shape, "cursorBlink": cursor.blinking, "cells": cells})
    }).collect();
    println!("{}", serde_json::to_string(&scenes).unwrap());
}
