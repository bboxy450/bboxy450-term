use bboxy_firehose::{COLS, ROWS, SCENES, expected_hash, fnv1a64, scene_bytes};
use bbt_core::{Color, Column, Dimensions, Line, Term};
use serde_json::{Value, json};

fn colour(color: Color) -> Value {
    match color {
        Color::Indexed(index) => json!([0, index]),
        Color::Named(name) => json!([1, name as u16]),
        Color::Spec(rgb) => json!([2, rgb.r, rgb.g, rgb.b]),
    }
}

fn screen(term: &Term) -> Vec<u8> {
    let grid = term.grid();
    let mut cells = Vec::new();
    for row in 0..grid.screen_lines() {
        for col in 0..grid.columns() {
            let cell = &grid[Line(row as i32)][Column(col)];
            cells.push(json!([
                cell.c as u32,
                colour(cell.fg),
                colour(cell.bg),
                cell.flags.bits(),
                cell.zerowidth()
                    .unwrap_or_default()
                    .iter()
                    .map(|c| *c as u32)
                    .collect::<Vec<_>>(),
                cell.underline_color().map(colour),
                cell.hyperlink()
                    .map(|link| [link.id().to_owned(), link.uri().to_owned()]),
            ]));
        }
    }
    serde_json::to_vec(&json!([
        grid.columns(),
        grid.screen_lines(),
        grid.cursor.point.line.0,
        grid.cursor.point.column.0,
        term.mode().bits(),
        cells
    ]))
    .unwrap()
}

#[test]
fn frozen_semantic_screens_survive_arbitrary_pty_chunking() {
    let mut mismatches = Vec::new();
    for scene in SCENES {
        let bytes = scene_bytes(scene);
        for chunk_size in [bytes.len(), 1, 7, 64, 4096] {
            let mut term = Term::new(COLS, ROWS, 0);
            for chunk in bytes.chunks(chunk_size) {
                term.advance(chunk);
            }
            assert!(
                !term.sync_pending(),
                "{} left synchronized output open",
                scene.name
            );
            let hash = format!("{:016x}", fnv1a64(&screen(&term)));
            if chunk_size == bytes.len() {
                println!("{} {hash}", scene.name);
            }
            if Some(hash.as_str()) != expected_hash(scene.name) {
                mismatches.push(format!("{} chunk={chunk_size} hash={hash}", scene.name));
            }
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
fn originals_are_control_only_and_embedded_verbatim() {
    for (scene_name, original) in [
        (
            "osc7501-claude",
            include_bytes!("../../../.bboxy/runs/osc7501-claude.bin").as_slice(),
        ),
        (
            "osc7501-preflight",
            include_bytes!("../../../.bboxy/runs/osc7501-claude-preflight.bin").as_slice(),
        ),
    ] {
        let scene = SCENES.iter().find(|s| s.name == scene_name).unwrap();
        assert!(scene.bytes.starts_with(original));
        let mut bytes = original;
        while !bytes.is_empty() {
            assert!(bytes.starts_with(b"\x1b]7501;"));
            bytes = &bytes[bytes.iter().position(|byte| *byte == 7).unwrap() + 1..];
        }
    }
    for capture in [
        include_bytes!("../../../.bboxy/runs/osc7501-codex.bin").as_slice(),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-bboxy-baseline.bin"),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-choice-unavailable.bin"),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-explicit-untrusted.bin"),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-hooks-disabled.bin"),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-offset-diagnostic.bin"),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-preflight.bin"),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-startup-diagnostic.bin"),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-startup-gated.bin"),
        include_bytes!("../../../.bboxy/runs/osc7501-codex-trust-chooser.bin"),
    ] {
        assert!(
            capture.is_empty(),
            "new capture content requires review before embedding"
        );
    }
}

#[test]
fn scenes_exercise_real_grid_effects_and_leave_modes_balanced() {
    use bbt_core::term::{TermMode, cell::Flags};
    for scene in SCENES {
        let mut term = Term::new(COLS, ROWS, 0);
        term.advance(&scene_bytes(scene));
        assert!(!term.mode().contains(TermMode::ALT_SCREEN));
        assert!(term.mode().contains(TermMode::SHOW_CURSOR));
        let text: String = term.grid().display_iter().map(|c| c.cell.c).collect();
        match scene.name {
            "palette" => {
                for flag in [
                    Flags::BOLD,
                    Flags::ITALIC,
                    Flags::DIM,
                    Flags::INVERSE,
                    Flags::UNDERLINE,
                    Flags::DOUBLE_UNDERLINE,
                    Flags::UNDERCURL,
                    Flags::DOTTED_UNDERLINE,
                    Flags::DASHED_UNDERLINE,
                    Flags::STRIKEOUT,
                ] {
                    assert!(
                        term.grid()
                            .display_iter()
                            .any(|c| c.cell.flags.contains(flag)),
                        "missing {flag:?}"
                    );
                }
            }
            "unicode" => {
                assert!(
                    term.grid()
                        .display_iter()
                        .any(|c| c.cell.flags.contains(Flags::WIDE_CHAR))
                );
                assert!(
                    term.grid()
                        .display_iter()
                        .any(|c| c.cell.zerowidth().is_some_and(|z| !z.is_empty()))
                );
            }
            "progress" => {
                assert!(text.contains("RIGHT"));
                assert!(!text.contains("WRONG"));
            }
            "alternate" => {
                assert!(text.contains("Persistent transcript line A"));
                assert!(text.contains("Returned to the primary"));
            }
            "scroll-links-sync" => {
                assert!(text.contains("Pinned header"));
                assert!(text.contains("Pinned footer"));
                assert!(text.contains("[committed]"));
                assert!(!text.contains("[assembling]"));
                assert!(!text.contains("region event 01"));
                assert!(
                    term.grid()
                        .display_iter()
                        .any(|c| c.cell.hyperlink().is_some())
                );
            }
            _ => {}
        }
    }
}

#[test]
fn cursor_scenes_are_steady_and_the_next_scene_restores_the_default() {
    use bbt_core::ansi::CursorShape;
    for (name, shape) in [
        ("cursor-beam", CursorShape::Beam),
        ("cursor-underline", CursorShape::Underline),
    ] {
        let scene = SCENES.iter().find(|scene| scene.name == name).unwrap();
        for chunk_size in [1, 7, scene.bytes.len()] {
            let mut term = Term::new(COLS, ROWS, 0);
            let default = term.cursor_style();
            for chunk in scene_bytes(scene).chunks(chunk_size) {
                term.advance(chunk);
            }
            assert_eq!(term.cursor_style().shape, shape);
            assert!(!term.cursor_style().blinking);
            assert_eq!(term.grid().cursor.point.line, Line(23));
            assert_eq!(term.grid().cursor.point.column, Column(0));
            term.advance(&scene_bytes(&SCENES[0]));
            assert_eq!(term.cursor_style(), default);
        }
    }
}
