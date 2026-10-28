// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{self as ui, components as c, modules::graph_tab::*};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, s: &str, t: &TextStyle) -> f32 {
        s.chars().count() as f32 * t.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, t: &TextStyle) -> [f32; 2] {
        [t.size * 0.8, t.size * 0.2]
    }
}
fn layout(view: &GraphTab, w: f32) -> c::Layout {
    let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    view.element_width(ctx, &Metrics, w)
        .layout(Rect::new(0., 0., w, 0.), ctx, &Metrics)
        .unwrap()
}
fn labels(n: &c::Layout) -> String {
    let mut value = if let c::LayoutKind::Text { runs, .. } = &n.kind {
        runs.iter().map(|r| r.text.as_str()).collect()
    } else {
        String::new()
    };
    for ch in &n.children {
        value += &labels(ch)
    }
    value
}
// keeps: spec gui-components §1.3
#[test]
fn graph_projects_live_and_blocked_tasks_without_inferring_authority() {
    let view = GraphTab {
        swarm: "test".into(),
        only: Some(vec!["agents".into()]),
        seats: vec![Seat {
            name: "worker".into(),
            state: "waiting".into(),
            tasks: vec![
                Work {
                    title: "current work".into(),
                    state: "running".into(),
                    ..Default::default()
                },
                Work {
                    state: "blocked".into(),
                    waits: Some("upstream".into()),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }],
        ..Default::default()
    };
    let out = labels(&layout(&view, 900.));
    assert!(out.contains("current work"));
    assert!(out.contains("waits on upstream"));
    assert!(!out.contains("Provenance"));
    assert!(!out.contains("verified"));
}
// keeps: spec gui-components §1.3
#[test]
fn work_tree_counts_terminal_items_and_keeps_artifacts_and_invariants_as_leaves() {
    let view = GraphTab {
        only: Some(vec!["work".into()]),
        epics: vec![Epic {
            title: "an epic".into(),
            items: vec![
                Work {
                    title: "upright".into(),
                    kind: "invariant".into(),
                    binds: vec![Binding {
                        title: "violating bug".into(),
                        rel: "violated-by".into(),
                    }],
                    ..Default::default()
                },
                Work {
                    title: "a completed task".into(),
                    state: "accepted".into(),
                    artifacts: vec![Artifact {
                        r#ref: "target/report.png".into(),
                        kind: "image".into(),
                        responsible: None,
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
        }],
        ..Default::default()
    };
    let out = labels(&layout(&view, 1000.));
    assert!(out.contains("1/2"));
    assert!(out.contains("target/report.png"));
    assert!(out.contains("violated-by"));
    assert!(!out.contains("responsible"));
}
// keeps: spec gui-components §1.3
#[test]
fn subscriptions_wrap_at_available_width_without_changing_patterns() {
    let view = GraphTab {
        only: Some(vec!["subscriptions".into()]),
        subscriptions: Some(vec![Subscription {
            seat: "worker".into(),
            patterns: (0..6)
                .map(|i| Pattern {
                    pattern: format!("doc/specs/long-pattern-{i}"),
                    auto: i == 0,
                    why: "delegated".into(),
                })
                .collect(),
            ..Default::default()
        }]),
        ..Default::default()
    };
    let narrow = layout(&view, 450.);
    let wide = layout(&view, 1200.);
    assert!(narrow.rect.h > wide.rect.h);
    let out = labels(&narrow);
    assert!(out.contains("AUTO"));
    assert!(out.contains("doc/specs/long-pattern-5"));
}
// keeps: spec gui-components §1.3
#[test]
fn provenance_and_history_respect_signed_flags_and_bounded_reverse_order() {
    let view = GraphTab {
        history: (0..15)
            .map(|i| History {
                r#ref: format!("file-{i}"),
                ..Default::default()
            })
            .collect(),
        provenance: Some(Provenance {
            items: vec![ProvenanceItem {
                title: "unsigned record".into(),
                digest: Some(Digest {
                    content: "0123456789abcdef".into(),
                    signed: false,
                    by: "untrusted string".into(),
                }),
                ..Default::default()
            }],
            log: vec![LogEntry {
                checkpoint: true,
                by: "operator".into(),
                ..Default::default()
            }],
        }),
        only: Some(vec!["provenance".into(), "history".into()]),
        ..Default::default()
    };
    let out = labels(&layout(&view, 1200.));
    assert!(out.contains("unsigned"));
    assert!(!out.contains("signed · untrusted string"));
    assert!(out.contains("daily checkpoint · signed by operator"));
    assert!(!out.contains("file-2"));
    assert!(out.find("file-14").unwrap() < out.find("file-3").unwrap());
}
// keeps: spec gui-components §1.3
#[test]
fn annotated_links_preserve_source_identity_display_and_weight() {
    let links = parse_links("see [[source|Display]]{derives:0.6} and [[upright]]{violates:1.0}");
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].slug, "source");
    assert_eq!(links[0].display, "Display");
    assert_eq!(links[0].weight.as_deref(), Some("0.6"));
    assert_eq!(links[1].relation.as_deref(), Some("violates"));
    assert!(parse_links("an incomplete [[link").is_empty());
    let escaped = parse_links(r"[[source\|#47]]{derives:0.6} [[#47\|decision]]");
    assert_eq!(escaped[0].slug, "source");
    assert_eq!(escaped[0].display, "#47");
    assert_eq!(escaped[0].weight.as_deref(), Some("0.6"));
    assert_eq!(escaped[1].slug, "#47");
    assert_eq!(escaped[1].display, "decision");
}

// keeps: spec gui-components §1.3
#[test]
fn channels_compose_published_edges_at_true_size_and_scroll_in_the_viewport() {
    let view = GraphTab {
        only: Some(vec!["channels".into()]),
        channels: Some(Channels {
            folders: vec![
                ChannelFolder {
                    path: "c".into(),
                    label: "channels/work/".into(),
                    ..Default::default()
                },
                ChannelFolder {
                    path: "c/render".into(),
                    label: "render/".into(),
                    depth: 1,
                    ..Default::default()
                },
            ],
            sinks: vec![ChannelSink {
                id: "worker".into(),
                name: "A worker".into(),
                ..Default::default()
            }],
            links: vec![
                ChannelLink {
                    from: "c/render".into(),
                    to: "worker".into(),
                    w: Some(0.5),
                    filter: Some("kind=chunk".into()),
                    paused: true,
                    ..Default::default()
                },
                ChannelLink {
                    from: "not-published".into(),
                    to: "worker".into(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }),
        ..Default::default()
    };
    fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
        if matches!(&n.kind,c::LayoutKind::Named(k)if k==key) {
            Some(n)
        } else {
            n.children.iter().find_map(|n| find(n, key))
        }
    }
    let small = layout(&view, 700.);
    let big = layout(&view, 1400.);
    let a = find(&small, "channels/tree").unwrap();
    let b = find(&big, "channels/tree").unwrap();
    assert_eq!(a.rect.h, 68.);
    assert_eq!(a.rect, b.rect);
    let text = labels(&big);
    assert!(text.contains("channels/work/"));
    assert!(text.contains("kind=chunk"));
    assert!(text.contains("seats/worker/inbox"));
    assert!(!text.contains("not-published"));
    assert!(find(&big, "channels/link0").is_some());
    assert!(find(&big, "channels/traffic0").is_some());
    assert!(find(&big, "channels/link1").is_none());
}
