// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![allow(
    clippy::disallowed_methods,
    reason = "headless worker witness, never a UI callback"
)]
use bbt_doc::highlight::{Highlighter, Highlights, RustHighlighter, Work};
#[test]
fn owned_worker_prepares_language_spans_and_unknown_languages_preserve_bytes() {
    let job = Work {
        generation: 3,
        revision: 7,
        source: "```rust\npub fn main() { let n = 42; }\n```\n".into(),
        light: false,
    };
    let prepared = bbt_rt::handle().block_on(job.prepare_off_thread()).unwrap();
    assert_eq!((prepared.generation, prepared.revision), (3, 7));
    let spans = prepared
        .highlights
        .spans(&prepared.document.blocks[0])
        .unwrap();
    assert!(spans.len() > 2);
    assert!(spans.windows(2).any(|p| p[0].color != p[1].color));
    let h = RustHighlighter::new(true);
    let input = "é🙂\n\tunknown();\n";
    assert_eq!(
        h.highlight("unsupported_language", input)
            .iter()
            .map(|s| s.text.as_str())
            .collect::<String>(),
        input
    );
    let empty = bbt_doc::parse("```plain\n\n```\n");
    assert!(
        Highlights::prepare(&empty, &h)
            .spans(&empty.blocks[0])
            .is_some()
    );
}

#[test]
fn late_preparation_is_retired_after_new_revision_drop_and_aba_reopen() {
    use bbt_doc::highlight::Preparation;
    let mut view = Preparation::default();
    let a = view.request("# A".into(), false).prepare();
    assert!(view.install(a.clone()));
    let b = view.request("# B".into(), false).prepare();
    assert_eq!(view.prepared().unwrap().document.source, "# A");
    assert!(!view.install(a.clone()));
    assert!(view.install(b.clone()));
    view.retire();
    assert!(!view.install(b));
    let a2 = view.request("# A".into(), false).prepare();
    assert!(!view.install(a));
    assert!(view.install(a2));
}

#[cfg(debug_assertions)]
#[test]
fn ui_thread_guard_refuses_direct_preparation_but_worker_preparation_succeeds() {
    bbt_rt::register_ui_thread();
    let job = || Work {
        generation: 1,
        revision: 1,
        source: "```rs\nfn main() {}\n```".into(),
        light: false,
    };
    assert!(std::panic::catch_unwind(|| job().prepare()).is_err());
    assert!(
        bbt_rt::handle()
            .block_on(job().prepare_off_thread())
            .is_ok()
    );
}
