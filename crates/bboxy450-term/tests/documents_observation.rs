// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxy450_term::documents::observation::{admitted_test_run, collect};
use bbt_mux::Ulid;
use bbt_proto::{GuiIdentity, GuiIdentityState, gui::*};
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::{draw::*, layout::*};

// Adapted from bbt-doc/tests/layout.rs's deterministic retained-layout measure.
struct Fonts;
impl TextMeasure for Fonts {
    fn width(&self, text: &str, style: &bbt_ui_model::layout::TextStyle) -> f32 {
        text.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Fonts {
    fn font_metrics(&self, style: &bbt_ui_model::layout::TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn source() -> UiDocumentSource {
    UiDocumentSource {
        record_id: "admitted-fixture".into(),
        number: Some(1),
        path: "e2e/fixture.md".into(),
        digest: "exact-digest".into(),
        etag: "exact-etag".into(),
        workspace: SwarmScope {
            swarm: Ulid(1),
            revision: 2,
        },
        root: "admitted-root".into(),
        provider_revision: 3,
    }
}
fn native(markdown: &str) -> (c::Layout, Vec<c::Paint>) {
    let context = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let layout = bbt_doc::parse(markdown)
        .body_element(context, &Fonts, 400., &[])
        .layout(Rect::new(20., 30., 400., 0.), context, &Fonts)
        .unwrap();
    let paints = c::paint(&layout, context);
    (layout, paints)
}
fn primitives(paints: &mut [c::Paint], change: &mut impl FnMut(&mut Prim)) {
    fn visit(items: &mut [Prim], change: &mut impl FnMut(&mut Prim)) {
        for primitive in items {
            if let Prim::Group { primitives, .. } = primitive {
                visit(primitives, change);
            }
            change(primitive);
        }
    }
    for paint in paints {
        match paint {
            c::Paint::Group { ops, .. } => primitives(ops, change),
            c::Paint::Draw(list) | c::Paint::Underlay(list) => visit(&mut list.primitives, change),
            _ => {}
        }
    }
}

#[test]
fn submitted_text_path_transform_and_clip_are_observed_instead_of_reconstructed() {
    let (layout, mut paint) =
        native("NativeGlyphs\n\n```mermaid\nflowchart LR\nA[One] --> B[Two]\n```\n");
    let before = collect(&layout, &paint, source(), Ulid(9), 11).unwrap();
    assert!(before.omitted.is_empty(), "{:?}", before.omitted);
    assert!(!before.paths.is_empty());
    primitives(&mut paint, &mut |primitive| match primitive {
        Prim::Text { style, .. } => style.scale = TypeScale::Custom(7.),
        Prim::Path { scale, .. } => *scale = 0.5,
        _ => {}
    });
    let clip = Rect::new(22., 32., 120., 80.);
    for op in &mut paint {
        if let c::Paint::Draw(list) = op {
            list.primitives.insert(
                0,
                Prim::Clip {
                    rect: clip,
                    radii: [3.; 4],
                },
            );
            list.unclip();
        }
    }
    let paint = vec![c::Paint::Group {
        ops: paint,
        opacity: 0.5,
        saturation: 1.,
        mask: None,
    }];
    let after = collect(&layout, &paint, source(), Ulid(9), 12).unwrap();
    assert!(after.omitted.is_empty(), "{:?}", after.omitted);
    assert_eq!(after.source, before.source);
    assert_eq!(after.frame_seq, 12);
    assert_eq!(after.private_test_run, Ulid(9));
    assert_eq!(after.texts[0].rect, before.texts[0].rect);
    assert_eq!(after.texts[0].line_height, before.texts[0].line_height);
    assert!(after.texts.iter().all(|run| run.font_size == 7.));
    assert!(after.paths.iter().all(|path| path.scale == 0.5));
    assert_eq!(after.paths[0].commands, before.paths[0].commands);
    assert!(
        after.texts[0]
            .clips
            .iter()
            .map(|i| &after.clips[usize::from(*i)])
            .any(|clip| clip
                == &UiDocumentClip {
                    rect: [22., 32., 120., 80.],
                    radii: [3.; 4]
                })
    );
    assert_eq!(after.texts[0].color[3], before.texts[0].color[3] * 0.5);
}

#[test]
fn collection_stays_bounded_and_reports_text_command_byte_and_walk_omissions() {
    let many = "Visible paragraph\n\n".repeat(5000);
    let (layout, paint) = native(&many);
    let observed = collect(&layout, &paint, source(), Ulid(9), 1).unwrap();
    assert!(observed.omitted.walk_truncated);
    assert!(observed.texts.len() <= DOCUMENT_TEXT_RUNS);
    assert!(serde_json::to_vec(&observed).unwrap().len() <= DOCUMENT_OBSERVATION_BYTES);

    let (mut layout, _) = native("ByteBudget\n");
    fn grow_text(node: &mut c::Layout) {
        if let c::LayoutKind::Text { runs, .. } = &mut node.kind {
            for run in runs {
                run.text = "B".repeat(1024);
            }
        }
        for child in &mut node.children {
            grow_text(child);
        }
    }
    grow_text(&mut layout);
    let context = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
    let paint = (0..100)
        .flat_map(|_| c::paint(&layout, context))
        .collect::<Vec<_>>();
    let observed = collect(&layout, &paint, source(), Ulid(9), 2).unwrap();
    assert!(observed.omitted.bytes > 0 && observed.omitted.walk_truncated);
    assert!(serde_json::to_vec(&observed).unwrap().len() <= DOCUMENT_OBSERVATION_BYTES);

    let points = std::sync::Arc::from(vec![
        PathCommand::Move([0., 0.]);
        DOCUMENT_PATH_COMMANDS + 1
    ]);
    let leaf = c::Layout {
        rect: Rect::new(20., 30., 10., 10.),
        baseline: 0.,
        children: vec![],
        kind: c::LayoutKind::Path {
            points,
            stroke: PathStroke::default(),
            color: [1.; 4],
            fill: None,
            scale: 1.,
        },
    };
    let paint = c::paint(&leaf, ui::Context::new(ui::Theme::Dark, ui::Density::Cozy));
    let observed = collect(&leaf, &paint, source(), Ulid(9), 2).unwrap();
    assert!(observed.paths.is_empty());
    assert_eq!(
        observed.omitted.path_commands as usize,
        DOCUMENT_PATH_COMMANDS + 1
    );

    let (layout, mut paint) = native("ByteBudget\n");
    primitives(&mut paint, &mut |primitive| {
        if let Prim::Text { text, .. } = primitive {
            *text = "Unmatched production primitive".into();
        }
    });
    let observed = collect(&layout, &paint, source(), Ulid(9), 3).unwrap();
    assert!(observed.texts.is_empty());
    assert!(observed.omitted.unsupported_paint && observed.omitted.text_runs > 0);
    let paint = vec![c::Paint::Group {
        ops: paint,
        opacity: 1.,
        saturation: 0.5,
        mask: None,
    }];
    assert!(
        collect(&layout, &paint, source(), Ulid(9), 4)
            .unwrap()
            .omitted
            .unsupported_paint
    );
}

#[test]
fn display_text_and_a_launch_selector_cannot_substitute_for_typed_private_identity() {
    let mut identity = GuiIdentity {
        state: GuiIdentityState::Certified,
        private_test_run: None,
        reason: Some("test-account:00000000000000000000000009".into()),
    };
    assert_eq!(admitted_test_run(Some(&identity), true), None);
    identity.private_test_run = Some(Ulid(9));
    assert_eq!(
        admitted_test_run(Some(&identity), true),
        cfg!(debug_assertions).then_some(Ulid(9))
    );
    assert_eq!(admitted_test_run(Some(&identity), false), None);
    for state in [GuiIdentityState::Held, GuiIdentityState::Pending] {
        identity.state = state;
        assert_eq!(admitted_test_run(Some(&identity), true), None);
    }
    assert_eq!(admitted_test_run(None, true), None);
}
