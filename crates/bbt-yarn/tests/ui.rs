#![allow(missing_docs)]

// keeps: spec yarn §§0,4 — public source, arbitration, phase or compiler contract
#[test]
fn reactor_mutations_and_negative_controls() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/*.rs");
    tests.pass("tests/ui-pass/*.rs");
}
