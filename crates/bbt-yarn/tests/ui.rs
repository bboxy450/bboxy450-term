// Project integration: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Fork source: https://github.com/Gonzih/kittens-old/tree/293e134526bb2deee96f1197d6268aee64495482
// Existing MIT OR Apache-2.0 terms and Kittens contributor notices apply.
// Credit bboxy450 integration accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(missing_docs)]

// keeps: spec yarn §§0,4 — public source, arbitration, phase or compiler contract
#[test]
fn reactor_mutations_and_negative_controls() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
    tests.pass("tests/ui-pass/*.rs");
}
