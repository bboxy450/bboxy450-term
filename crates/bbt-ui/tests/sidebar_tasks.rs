// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::{
        composer::{
            Attachment, Composer, Conversation, Regions as ComposerRegions, Report, Verification,
        },
        stream_dock::{Action, Entry, Message, Regions, StreamDock, WorkItem},
    },
};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn named<'a>(tree: &'a c::Layout, key: &str) -> &'a c::Layout {
    fn find<'a>(tree: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&tree.kind, c::LayoutKind::Named(name) if name == key) {
            Some(tree)
        } else {
            tree.children.iter().find_map(|child| find(child, key))
        }
    }
    find(tree, key).unwrap_or_else(|| panic!("missing {key}"))
}
fn view() -> Conversation {
    Conversation {
        stream: StreamDock {
            dock: true,
            framed: false,
            entries: (0..12)
                .map(|i| Entry::Message {
                    data: Box::new(Message {
                        text: format!("Chat message {i}"),
                        attach: if i == 0 {
                            vec![Attachment {
                                kind: "document".into(),
                                title: "Context".into(),
                                path: "context.md".into(),
                                ..Default::default()
                            }]
                        } else {
                            vec![]
                        },
                        ..Default::default()
                    }),
                    show_channel: Some(false),
                })
                .collect(),
        },
        tasks: Some(StreamDock {
            dock: true,
            framed: false,
            entries: vec![
                Entry::Task {
                    data: WorkItem {
                        id: "short-task".into(),
                        title: "A short task".into(),
                        state: "open".into(),
                        ..Default::default()
                    },
                    delegate: true,
                },
                Entry::Report {
                    id: "long-report".into(),
                    data: Report {
                        task: "A measured report".into(),
                        status: "reported".into(),
                        summary: "Report summary with variable wrapped content. ".repeat(35),
                        verify_result: Some(Verification {
                            exit_code: 0,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    review: true,
                },
            ],
        }),
        composer: Composer {
            text: "Editable draft\n".repeat(80),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn solve(view: &Conversation, size: [f32; 2], tasks: Option<f32>) -> c::Layout {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    view.element_sidebar(ctx, size, &Metrics, Some(80.), tasks)
        .unwrap()
        .layout(Rect::new(0., 0., size[0], size[1]), ctx, &Metrics)
        .unwrap()
}
fn regions(tree: &c::Layout, view: &Conversation) -> Regions {
    let mut out = Regions::dock_from(tree, &view.stream, 0);
    out.extend(Regions::dock_from(
        tree,
        view.tasks.as_ref().unwrap(),
        view.stream.entries.len(),
    ));
    out
}
fn center(rect: Rect) -> [f32; 2] {
    [rect.x + rect.w / 2., rect.y + rect.h / 2.]
}
#[test]
fn bounded_chat_and_tasks_keep_composer_controls_and_expose_fitted_sections() {
    let view = view();
    for width in [160., 300., 520.] {
        for height in [300., 500., 800.] {
            for requested in [None, Some(0.), Some(96.), Some(4096.)] {
                let tree = solve(&view, [width, height], requested);
                let chat = named(&tree, "conversation/chat-section").rect;
                let tasks = named(&tree, "conversation/task-section").rect;
                assert!((tree.rect.h - height).abs() < 0.01);
                assert!(
                    (tasks.bottom() + bbt_ui_model::TOKENS.layout.cohort_gap as f32 - chat.y).abs()
                        < 0.01
                );
                assert!((chat.bottom() - height).abs() < 0.01);
                assert!(tasks.h >= 32. && tasks.h <= height);
                for key in [
                    "composer/address",
                    "composer/send",
                    "composer/foot",
                    "conversation/rows",
                ] {
                    let rect = named(&tree, key).rect;
                    assert!(
                        rect.y >= chat.y && rect.bottom() <= chat.bottom() + 0.01,
                        "{width}/{height}/{requested:?}/{key}: {rect:?}, chat={chat:?}"
                    );
                }
                let tasks_rows = named(&tree, "conversation/tasks").rect;
                assert!(tasks_rows.y >= tasks.y && tasks_rows.bottom() <= tasks.bottom() + 0.01);
                assert!(ComposerRegions::new(&tree).unwrap().body.h <= 80.);
            }
        }
    }
    for height in [0., 32., 96.] {
        let tree = solve(&view, [300., height], Some(4096.));
        assert!((tree.rect.h - height).abs() < 0.01);
        for key in [
            "conversation/chat-section",
            "conversation/task-section",
            "conversation/rows",
            "conversation/tasks",
        ] {
            let rect = named(&tree, key).rect;
            assert!(rect.h >= 0. && rect.w >= 0., "{height}/{key}: {rect:?}");
        }
    }
}
#[test]
fn task_and_report_actions_use_disjoint_ids_and_their_own_scroll_clip() {
    let mut view = view();
    let initial = solve(&view, [320., 680.], Some(200.));
    let start = view.stream.entries.len();
    let task_key = format!("e{start}");
    let report_key = format!("e{}/accept", start + 1);
    assert_eq!(
        regions(&initial, &view).hit(center(named(&initial, &task_key).rect)),
        Some(Action::Task("short-task".into()))
    );
    let accept = named(&initial, &report_key).rect;
    assert_eq!(
        regions(&initial, &view).hit(center(accept)),
        None,
        "offscreen report action must not escape Tasks clip"
    );
    let chat = named(&initial, "e0").rect;
    let attachment = named(&initial, "e0/body/attach/a0").rect;
    assert_eq!(
        regions(&initial, &view).hit(center(attachment)),
        Some(Action::OpenRecord("context.md".into()))
    );
    let max = Regions::scroll_extent(&initial, "conversation/tasks").unwrap()[1];
    assert!(max > 0.);
    view.task_scroll = max;
    let task_scrolled = solve(&view, [320., 680.], Some(200.));
    assert_eq!(
        named(&task_scrolled, "e0").rect,
        chat,
        "Tasks scrolling must leave Chat fixed"
    );
    let accept = named(&task_scrolled, &report_key).rect;
    assert_eq!(
        regions(&task_scrolled, &view).hit(center(accept)),
        Some(Action::Accept("long-report".into()))
    );
    let tasks_at = named(&task_scrolled, "conversation/tasks").rect;
    assert!(accept.y >= tasks_at.y && accept.bottom() <= tasks_at.bottom() + 0.01);
    view.scroll = Regions::scroll_extent(&task_scrolled, "conversation/rows").unwrap()[1];
    assert!(view.scroll > 0.);
    let chat_scrolled = solve(&view, [320., 680.], Some(200.));
    assert_eq!(
        named(&chat_scrolled, &report_key).rect,
        accept,
        "Chat scrolling must leave Tasks fixed"
    );
    assert_eq!(
        regions(&chat_scrolled, &view).hit(center(accept)),
        Some(Action::Accept("long-report".into()))
    );
    let hidden_attachment = named(&chat_scrolled, "e0/body/attach/a0").rect;
    assert!(
        !matches!(
            regions(&chat_scrolled, &view).hit(center(hidden_attachment)),
            Some(Action::OpenRecord(_))
        ),
        "scrolled-out Chat links cannot steal clicks from Tasks above it"
    );
}
#[test]
fn legacy_sidebar_tree_stays_identical_until_tasks_are_requested() {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let view = Conversation::default();
    let rect = Rect::new(0., 0., 300., 420.);
    let legacy = view
        .element_bounded(ctx, [300., 420.], &Metrics, Some(80.))
        .unwrap()
        .layout(rect, ctx, &Metrics)
        .unwrap();
    let sidebar = view
        .element_sidebar(ctx, [300., 420.], &Metrics, Some(80.), Some(200.))
        .unwrap()
        .layout(rect, ctx, &Metrics)
        .unwrap();
    assert_eq!(format!("{legacy:?}"), format!("{sidebar:?}"));
}

#[test]
fn tasks_and_chat_fold_independently_without_discarding_draft_or_actions() {
    let view = view();
    let draft = view.composer.text.clone();
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    fn contains(n: &c::Layout, key: &str) -> bool {
        matches!(&n.kind,c::LayoutKind::Named(s) if s==key)
            || n.children.iter().any(|c| contains(c, key))
    }
    for open in [[false, false], [false, true], [true, false], [true, true]] {
        let element = view
            .element_sidebar_open(ctx, [300., 500.], &Metrics, [Some(80.), Some(160.)], open)
            .unwrap();
        let tree = element
            .layout(Rect::new(0., 0., 300., 500.), ctx, &Metrics)
            .unwrap();
        assert!(contains(&tree, "conversation/task-section/head"));
        assert!(contains(&tree, "conversation/chat-section/head"));
        assert_eq!(contains(&tree, "conversation/tasks"), open[0]);
        assert_eq!(contains(&tree, "conversation/rows"), open[1]);
        assert_eq!(contains(&tree, "composer"), open[1]);
        assert_eq!(view.composer.text, draft);
    }
}
