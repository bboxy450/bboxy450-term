#![allow(
    clippy::disallowed_methods,
    reason = "test/fixture IO, not the UI thread"
)]

use std::sync::{Arc, Mutex};

use bbt_core::{Checkpoint, Color, Column, Dimensions, Line, NamedColor, Rgb, Term};

fn restored(term: &Term) -> Term {
    let bytes = serde_json::to_vec(&term.checkpoint()).unwrap();
    let checkpoint: Checkpoint = serde_json::from_slice(&bytes).unwrap();
    let mut result = Term::new(2, 1, 0);
    result.restore(checkpoint);
    result
}

// keeps: spec terminal-core §D2
#[test]
fn red_foreground_survives_4096_bytes_and_a_durable_checkpoint() {
    let mut term = Term::new(80, 24, 128);
    term.advance(b"\x1b[31m");
    term.advance(&[b'x'; 4096]);
    let mut term = restored(&term);
    assert_eq!(term.grid().columns(), 80);
    assert_eq!(term.grid().screen_lines(), 24);
    assert_eq!(
        term.grid().cursor.template.fg,
        Color::Named(NamedColor::Red)
    );
    let point = term.grid().cursor.point;
    term.advance(b"R");
    assert_eq!(term.grid()[point].c, 'R');
    assert_eq!(term.grid()[point].fg, Color::Named(NamedColor::Red));
}

// keeps: spec terminal-core §D2
#[test]
fn checkpoint_retains_an_incomplete_csi_and_utf8_codepoint() {
    let mut term = Term::new(8, 2, 0);
    term.advance(b"\x1b[38;2;16;32;48");
    let mut term = restored(&term);
    term.advance(b"m\xe2\x82");
    let mut term = restored(&term);
    term.advance(b"\xac");
    assert_eq!(term.grid()[Line(0)][Column(0)].c, '€');
    assert_eq!(
        term.grid()[Line(0)][Column(0)].fg,
        Color::Spec(Rgb {
            r: 16,
            g: 32,
            b: 48
        })
    );
}

// keeps: spec terminal-core §D2
#[test]
fn osc_7501_reaches_the_hook_with_raw_parameters_for_bel_and_st() {
    let calls = Arc::new(Mutex::new(Vec::<Vec<Vec<u8>>>::new()));
    let observed = calls.clone();
    let mut term = Term::new(8, 2, 0);
    term.on_unknown_osc(move |params| {
        observed
            .lock()
            .unwrap()
            .push(params.iter().map(|p| p.to_vec()).collect());
    });
    term.advance(b"\x1b]7501;state=working:app=fixture:id=one\x07");
    term.advance(b"\x1b]7501;state=clear:id=one\x1b\\");
    assert_eq!(
        *calls.lock().unwrap(),
        vec![
            vec![
                b"7501".to_vec(),
                b"state=working:app=fixture:id=one".to_vec()
            ],
            vec![b"7501".to_vec(), b"state=clear:id=one".to_vec()],
        ]
    );
}

// keeps: spec terminal-core §D2
#[test]
fn incomplete_osc_restores_without_replaying_old_effects_or_losing_the_live_hook() {
    let mut original = Term::new(8, 2, 0);
    original.advance(b"\x1b]0;old title\x07\x1b]7502;v=1:ev=Session");
    let checkpoint: Checkpoint =
        serde_json::from_slice(&serde_json::to_vec(&original.checkpoint()).unwrap()).unwrap();
    let calls = Arc::new(Mutex::new(Vec::<Vec<Vec<u8>>>::new()));
    let observed = calls.clone();
    let mut term = Term::new(8, 2, 0);
    term.on_unknown_osc(move |params| {
        observed
            .lock()
            .unwrap()
            .push(params.iter().map(|p| p.to_vec()).collect());
    });
    term.restore(checkpoint);
    assert!(term.take_events().is_empty());
    assert!(calls.lock().unwrap().is_empty());
    term.advance(b"Start\x1b\\");
    assert_eq!(
        *calls.lock().unwrap(),
        vec![vec![b"7502".to_vec(), b"v=1:ev=SessionStart".to_vec()],]
    );
}

// keeps: spec terminal-core §D2
#[test]
fn saved_cursor_and_inactive_primary_screen_survive_checkpoint() {
    let mut term = Term::new(8, 2, 0);
    term.advance(b"\x1b[31mP\x1b7\x1b[?1049h\x1b[32mA");
    let mut term = restored(&term);
    term.advance(b"\x1b[?1049l\x1b8Q");
    assert_eq!(term.grid()[Line(0)][Column(0)].c, 'P');
    assert_eq!(term.grid()[Line(0)][Column(1)].c, 'Q');
    assert_eq!(
        term.grid()[Line(0)][Column(1)].fg,
        Color::Named(NamedColor::Red)
    );
}

// keeps: spec terminal-core §D2
#[test]
fn synchronized_output_buffer_and_preceding_character_survive_checkpoint() {
    let mut term = Term::new(8, 2, 0);
    term.advance(b"\x1b[?2026h\x1b[31mX");
    assert!(term.sync_pending());
    assert_eq!(term.grid()[Line(0)][Column(0)].c, ' ');
    let mut term = restored(&term);
    assert!(term.sync_pending());
    term.advance(b"\x1b[?2026l\x1b[2b");
    assert!(!term.sync_pending());
    for column in 0..3 {
        assert_eq!(term.grid()[Line(0)][Column(column)].c, 'X');
        assert_eq!(
            term.grid()[Line(0)][Column(column)].fg,
            Color::Named(NamedColor::Red)
        );
    }
}

// keeps: spec terminal-core §D2
#[test]
fn viewport_only_drops_existing_history_even_while_the_primary_is_inactive() {
    let mut term = Term::new(8, 2, 32);
    term.advance(&[b'x'; 200]);
    assert!(term.grid().history_size() > 0);
    term.advance(b"\x1b[?1049h");
    term.viewport_only();
    term.advance(b"\x1b[?1049l");
    assert_eq!(term.grid().history_size(), 0);
    term.advance(&[b'x'; 200]);
    assert_eq!(term.grid().history_size(), 0);
}
