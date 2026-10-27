#![allow(clippy::disallowed_methods)] // In-memory checkpoint fixture, never UI.

use bbt_core::{Column, Dimensions, Line};
use bbt_supervisor::{
    SeatSize, Ulid,
    ring::{Playback, Saved},
};

// A one-row growth used to allocate/serialize 1000 spare rows. With real window
// widths this exceeded the 16 MiB checkpoint budget and prevented logic recovery.
#[test]
fn resize_checkpoint_keeps_both_screens_and_fits_the_existing_budget() {
    let mut state = Playback::new(
        Ulid::new(),
        1,
        SeatSize {
            cols: 159,
            rows: 58,
            ..Default::default()
        },
    );
    state.term.resize(159, 59);
    state
        .term
        .advance(b"\x1b[Hprimary\x1b[?1049h\x1b[Halternate");
    state.term.advance(b"\x1b[?2026h\x1b[2;1Hpending");
    let saved = state.saved();
    let decoded = serde_json::to_vec(&saved).unwrap();
    assert!(decoded.len() < 4 << 20, "{} decoded bytes", decoded.len());
    let bytes = saved.encode().unwrap();
    let mut recovered = Playback::restore(Saved::decode(&bytes).unwrap());
    assert_eq!(recovered.term.grid().screen_lines(), 59);
    assert!(recovered.term.sync_pending());
    recovered.term.advance(b"\x1b[?2026l");
    assert_eq!(recovered.term.grid()[Line(0)][Column(0)].c, 'a');
    assert_eq!(recovered.term.grid()[Line(1)][Column(0)].c, 'p');
    recovered.term.advance(b"\x1b[?1049l");
    assert_eq!(recovered.term.grid()[Line(0)][Column(0)].c, 'p');
}
