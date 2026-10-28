// Project integration: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Existing Apache-2.0 crate terms and retained upstream notices apply.
// Credit bboxy450 integration accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_core::{Dimensions, Term};

// keeps: spec terminal-core §D1
#[test]
fn resizing_reflows_content_keeps_the_cursor_in_bounds_and_preserves_modes() {
    let mut term = Term::new(20, 4, 16);
    term.advance(b"\x1b[?2004h\x1b[31mhello resize\r\nsecond row");
    let modes = *term.mode();
    for (cols, rows) in [(6, 2), (30, 8)] {
        term.resize(cols, rows);
        assert_eq!(term.grid().columns(), cols);
        assert_eq!(term.grid().screen_lines(), rows);
        assert_eq!(*term.mode(), modes);
        let cursor = term.grid().cursor.point;
        assert!(cursor.line.0 >= 0 && cursor.line.0 < rows as i32);
        assert!(cursor.column.0 < cols);
        assert!(term.grid().display_iter().any(|cell| cell.cell.c != ' '));
    }
    term.resize(0, 0);
    assert_eq!(term.grid().columns(), bbt_core::term::MIN_COLUMNS);
    assert_eq!(term.grid().screen_lines(), bbt_core::term::MIN_SCREEN_LINES);
    assert_eq!(*term.mode(), modes);
    assert_eq!(term.grid().cursor.point.line.0, 0);
    assert!(term.grid().cursor.point.column.0 < term.grid().columns());
}
