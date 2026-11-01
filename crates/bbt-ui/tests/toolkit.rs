// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    modules::{
        approval::{Approval, Key, OptionItem},
        diff::{Diff, Folds, Kind},
        tool_call::{Status, ToolCall},
        transcript::{Apply, ScrollStick, StreamToView},
    },
    text_edit::{Command, CompletionPopover, Editor, Motion},
};
#[test]
fn stream_to_view_orders_once_and_retires_dropped_and_aba_generations() {
    let mut v = StreamToView::default();
    let a = v.open(String::new());
    assert_eq!(v.apply(a, 1, |s| s.push('b')), Apply::Gap { expected: 0 });
    assert_eq!(v.apply(a, 0, |s| s.push('a')), Apply::Applied);
    assert_eq!(v.apply(a, 0, |_| panic!("duplicate")), Apply::Duplicate);
    assert_eq!(v.apply(a, 1, |s| s.push('b')), Apply::Applied);
    assert_eq!(v.value().unwrap(), "ab");
    v.retire();
    assert_eq!(v.apply(a, 2, |_| panic!("dropped")), Apply::Retired);
    let b = v.open("B".into());
    let a2 = v.open("A".into());
    assert_ne!(a, a2);
    for old in [a, b] {
        assert_eq!(v.apply(old, 0, |_| panic!("stale")), Apply::Retired);
    }
    assert_eq!(v.apply(a2, 0, |s| s.push('!')), Apply::Applied);
    assert_eq!(v.value().unwrap(), "A!");
}

#[test]
fn coalesced_frames_apply_stream_deltas_and_rearm_a_budgeted_tail() {
    use bbt_rt::{DirtySet, ViewRing, Waker};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    #[derive(Default)]
    struct Wakes(AtomicUsize);
    impl Waker for Wakes {
        fn wake(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    fn drive(rearm: bool) -> (Vec<usize>, usize, usize) {
        let wakes = Arc::new(Wakes::default());
        let dirty = DirtySet::new(1, wakes.clone());
        let queue = ViewRing::new(8, dirty.source(0));
        let mut view = StreamToView::default();
        let generation = view.open(Vec::new());
        for sequence in 0..3 {
            queue.push((generation, sequence));
        }
        assert_eq!(wakes.0.load(Ordering::SeqCst), 1, "arrivals must coalesce");
        let mut frames = 0;
        while !dirty.drain().is_empty() {
            frames += 1;
            assert!(frames <= 2, "no busy frame loop");
            for _ in 0..2 {
                if let Some((generation, sequence)) = queue.pop() {
                    assert_eq!(
                        view.apply(generation, sequence as u64, |v| v.push(sequence)),
                        Apply::Applied
                    );
                }
            }
            if rearm {
                queue.complete_drain();
            }
        }
        (view.value().unwrap().clone(), frames, queue.len())
    }
    assert_eq!(drive(true), (vec![0, 1, 2], 2, 0));
    // Actual API negative control: omit the bounded-drain completion and the
    // final admitted delta has no wake. The formal Broken model omits rearm too.
    assert_eq!(drive(false), (vec![0, 1], 1, 1));
}
#[test]
fn scroll_stick_survives_growth_shrink_and_restores_only_on_explicit_follow() {
    let mut s = ScrollStick::default();
    assert_eq!(s.layout_offset(), f32::MAX);
    s.user_scroll(42., 100.);
    assert!(!s.following());
    for maximum in [200., 10., 0., 400.] {
        // ScrollView clamps the projection. Growth/reflow never changes intent.
        assert_eq!(s.layout_offset().min(maximum), 42_f32.min(maximum));
        assert_eq!(s.layout_offset(), 42.);
    }
    s.user_scroll(f32::NAN, 100.);
    assert_eq!(s.layout_offset(), 42.);
    s.jump_to_latest();
    assert!(s.following());
    s.user_scroll(100., 100.);
    assert!(s.following());
}
#[test]
fn diff_numbers_zero_count_hunks_and_fold_state_are_independent() {
    let d=Diff::parse("--- a/lib.rs\n+++ b/lib.rs\n@@ -4,2 +4,3 @@ f\n same\n-old\n+new\n+more\n@@ -9,0 +11 @@\n+added\n\\ No newline at end of file\n").unwrap();
    assert_eq!(d.hunks.len(), 2);
    let l = &d.hunks[0].lines;
    assert_eq!((l[0].old, l[0].new), (Some(4), Some(4)));
    assert_eq!(
        (l[1].old, l[1].new, l[1].kind),
        (Some(5), None, Kind::Removed)
    );
    assert_eq!((l[3].old, l[3].new), (None, Some(6)));
    assert_eq!(d.hunks[1].lines[0].old, None);
    assert!(Diff::parse("@@ -1,2 +1 @@\n one\n").is_err());
    let mut f = Folds::default();
    f.toggle(0);
    assert!(f.closed(0));
    assert!(!f.closed(1));
    f.toggle(0);
    assert!(!f.closed(0));
}

#[test]
fn diff_blank_context_and_one_bad_line_preserve_the_other_lines() {
    let blank = Diff::parse("@@ -1,3 +1,3 @@\n first\n\n last\n").expect("empty context is valid");
    assert_eq!(blank.hunks[0].lines[1].text, "");
    assert_eq!(blank.hunks[0].lines[1].kind, Kind::Context);
    let bad = Diff::parse("@@ -1,3 +1,3 @@\n first\n?damaged\n last\n")
        .expect("only damaged line degrades");
    assert_eq!(bad.hunks[0].lines[0].text, "first");
    assert_eq!(bad.hunks[0].lines[1].text, "?damaged");
    assert_eq!(bad.hunks[0].lines[1].kind, Kind::Note);
    assert_eq!(
        (bad.hunks[0].lines[2].old, bad.hunks[0].lines[2].new),
        (Some(3), Some(3))
    );
}
#[test]
fn approval_skips_disabled_wraps_and_never_chooses_without_enter() {
    let mut p = Approval::new(
        "prompt",
        "Run command?",
        vec![
            OptionItem {
                key: "once".into(),
                label: "Allow once".into(),
                enabled: true,
            },
            OptionItem {
                key: "always".into(),
                label: "Allow always".into(),
                enabled: false,
            },
            OptionItem {
                key: "deny".into(),
                label: "Deny".into(),
                enabled: true,
            },
        ],
    );
    assert_eq!(p.selected(), Some(0));
    assert_eq!(p.handle(Key::Down), None);
    assert_eq!(p.selected(), Some(2));
    assert_eq!(p.handle(Key::Enter), Some("deny".into()));
    p.handle(Key::Down);
    assert_eq!(p.selected(), Some(0));
    p.handle(Key::Up);
    assert_eq!(p.selected(), Some(2));
    p.handle(Key::Home);
    assert_eq!(p.selected(), Some(0));
    p.handle(Key::End);
    assert_eq!(p.selected(), Some(2));
    for o in &mut p.options {
        o.enabled = false;
    }
    assert_eq!(p.handle(Key::Enter), None);
    assert_eq!(p.selected(), None);
}

#[test]
fn moving_approval_selection_changes_the_painted_focus() {
    use bbt_ui::{self as ui, components as c};
    use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
    struct Metrics;
    impl TextMeasure for Metrics {
        fn width(&self, text: &str, style: &TextStyle) -> f32 {
            text.chars().count() as f32 * style.size * 0.5
        }
    }
    impl c::Measure for Metrics {
        fn font_metrics(&self, s: &TextStyle) -> [f32; 2] {
            [s.size * 0.8, s.size * 0.2]
        }
    }
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let mut p = Approval::new(
        "choice",
        "Run?",
        vec![
            OptionItem {
                key: "allow".into(),
                label: "Allow".into(),
                enabled: true,
            },
            OptionItem {
                key: "deny".into(),
                label: "Deny".into(),
                enabled: true,
            },
        ],
    );
    let paint = |p: &Approval| {
        let layout = p
            .element(ctx)
            .layout(Rect::new(0., 0., 400., 0.), ctx, &Metrics)
            .unwrap();
        format!("{:?}", c::paint(&layout, ctx))
    };
    let before = paint(&p);
    p.handle(Key::Down);
    assert_ne!(before, paint(&p));
}
#[test]
fn tool_disclosures_toggle_only_the_requested_section() {
    let mut t = ToolCall {
        key: "read".into(),
        title: "Read config".into(),
        status: Status::Succeeded,
        arguments: "{}".into(),
        output: "ok".into(),
        arguments_open: false,
        output_open: false,
    };
    assert!(t.toggle("read/arguments/toggle"));
    assert!(t.arguments_open);
    assert!(!t.output_open);
    assert!(!t.toggle("other/output/toggle"));
    assert!(t.toggle("read/output/toggle"));
    assert!(t.output_open);
}
fn motion(e: &mut Editor, s: &mut String, motion: Motion) {
    e.handle(
        s,
        Command::Move {
            motion,
            extend: false,
        },
    );
}
#[test]
fn unicode_vertical_word_line_motions_undo_redo_and_history_restore_the_draft() {
    let mut s = "ab🙂cd\nx\n123🙂567".to_owned();
    let mut e = Editor::new(&s);
    e.handle(
        &mut s,
        Command::SetCaret {
            at: 5,
            extend: false,
        },
    ); // first line, after c
    motion(&mut e, &mut s, Motion::Down);
    assert_eq!(e.edit().caret, Some(8));
    motion(&mut e, &mut s, Motion::Down);
    assert_eq!(e.edit().caret, Some(14));
    motion(&mut e, &mut s, Motion::Up);
    motion(&mut e, &mut s, Motion::Up);
    assert_eq!(e.edit().caret, Some(5));
    motion(&mut e, &mut s, Motion::LineStart);
    assert_eq!(e.edit().caret, Some(0));
    motion(&mut e, &mut s, Motion::LineEnd);
    assert_eq!(e.edit().caret, Some(6));
    s = "éclair 🙂 next_word".into();
    e.reset(&s);
    motion(&mut e, &mut s, Motion::WordLeft);
    assert_eq!(e.edit().caret, Some(10));
    motion(&mut e, &mut s, Motion::WordRight);
    assert_eq!(e.edit().caret, Some(19));
    e.handle(&mut s, Command::Insert("!".into()));
    assert!(s.ends_with('!'));
    assert!(e.handle(&mut s, Command::Undo));
    assert!(!s.ends_with('!'));
    assert!(e.handle(&mut s, Command::Redo));
    assert!(s.ends_with('!'));
    e.handle(&mut s, Command::Undo);
    e.handle(&mut s, Command::Insert("?".into()));
    assert!(!e.handle(&mut s, Command::Redo));
    e.remember_input("first");
    e.remember_input("second");
    let draft = s.clone();
    e.handle(&mut s, Command::HistoryPrevious);
    assert_eq!(s, "second");
    e.handle(&mut s, Command::HistoryPrevious);
    assert_eq!(s, "first");
    e.handle(&mut s, Command::HistoryNext);
    e.handle(&mut s, Command::HistoryNext);
    assert_eq!(s, draft);
}
#[test]
fn completions_capture_query_and_caret_and_cannot_replace_a_stale_draft() {
    let mut s = "Read @src/ma".to_owned();
    let mut e = Editor::new(&s);
    let request = e.completion(&s).unwrap();
    assert_eq!(request.trigger, '@');
    assert_eq!(request.query, "src/ma");
    let mut popup = CompletionPopover::new(request, vec!["@src/main.rs".into()]);
    let command = popup.handle(Key::Enter).unwrap();
    assert!(e.handle(&mut s, command));
    assert_eq!(s, "Read @src/main.rs");
    assert!(e.handle(&mut s, Command::Undo));
    assert_eq!(s, "Read @src/ma");
    let stale = popup.handle(Key::Enter).unwrap();
    e.handle(&mut s, Command::Insert("p".into()));
    assert!(!e.handle(&mut s, stale));
    assert_eq!(s, "Read @src/map");
    s = "/hel".into();
    e.reset(&s);
    assert_eq!(e.completion(&s).unwrap().trigger, '/');
    e.handle(
        &mut s,
        Command::Preedit {
            text: "x".into(),
            caret: Some(1),
        },
    );
    assert!(e.completion(&s).is_none());
    assert!(!e.handle(&mut s, Command::Undo));
}

#[test]
fn multiple_file_diff_headers_stay_with_their_hunks() {
    let d=Diff::parse("--- a/one\n+++ b/one\n@@ -1 +1 @@\n-old\n+new\n--- a/two\n+++ b/two\n@@ -1 +1 @@\n-old2\n+new2\n").unwrap();
    assert_eq!(d.headers, vec!["--- a/one", "+++ b/one"]);
    assert_eq!(d.hunks[1].file_headers, vec!["--- a/two", "+++ b/two"]);
    assert!(d.trailer.is_empty());
}
