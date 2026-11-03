// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    modules::startup::{History, STEPS, Startup},
};
use bbt_ui_model::layout::Rect;

fn inspect(layout: &c::Layout, text: &mut Vec<String>, actions: &mut Vec<String>) {
    if let c::LayoutKind::Text { runs, .. } = &layout.kind {
        text.push(runs.iter().map(|r| r.text.as_str()).collect());
    }
    if let c::LayoutKind::Named(name) = &layout.kind {
        actions.push(name.clone());
    }
    for child in &layout.children {
        assert!(
            child.rect.x >= layout.rect.x - 0.5 && child.rect.right() <= layout.rect.right() + 0.5,
            "child overflow: {:?} outside {:?}",
            child.rect,
            layout.rect
        );
        assert!(
            child.rect.y >= layout.rect.y - 0.5
                && child.rect.bottom() <= layout.rect.bottom() + 0.5,
            "vertical overflow: {:?} outside {:?}",
            child.rect,
            layout.rect
        );
        inspect(child, text, actions);
    }
}

#[test]
fn startup_checklist_keeps_every_step_actions_and_wrapped_diagnostics_in_bounds() {
    for theme in bboxybook::gallery::THEMES {
        for density in bboxybook::gallery::DENSITIES {
            for width in [320., 560.] {
                for active in 0..4 {
                    for failed in [false, true] {
                        let data = Startup {
                            title: "Opening your cockpit".into(), description: "Your sessions appear when each check is complete.".into(),
                            elapsed_secs: 3, active_step: active,
                            history: (active == 2).then(|| History { stream: "account".into(), verified: 25, total: 100 }),
                            details: vec![], error: failed.then(|| "History signature failed while verifying a retained account stream. Startup is paused until the signed history can be verified. The records and the diagnostic paths below remain available for inspection.".into()),
                            logs: if failed { vec![
                                ["App log".into(), "/tmp/bboxy-startup/private-account-with-a-long-descriptive-directory-name/run/app.log".into()],
                                ["Runtime log".into(), "/tmp/bboxy-startup/private-account-with-a-long-descriptive-directory-name/run/logic.log".into()],
                            ] } else { vec![] },
                        };
                        let ctx = ui::Context::new(theme, density);
                        let layout = data
                            .element(ctx)
                            .unwrap()
                            .layout(
                                Rect::new(0., 0., width, 0.),
                                ctx,
                                &bboxybook::components::Metrics,
                            )
                            .unwrap();
                        let (mut text, mut actions) = (vec![], vec![]);
                        inspect(&layout, &mut text, &mut actions);
                        if width == 560. {
                            assert!(
                                layout.rect.h <= 576.,
                                "startup must leave 12px on each side of the app's 600px minimum window: {}",
                                layout.rect.h
                            );
                        }
                        let positions =
                            STEPS.map(|step| text.iter().position(|s| s == step).unwrap());
                        assert!(positions.windows(2).all(|p| p[0] < p[1]));
                        assert_eq!(text.iter().filter(|s| s.as_str() == "Done").count(), active);
                        assert_eq!(
                            text.iter().filter(|s| s.as_str() == "Pending").count(),
                            3 - active
                        );
                        assert!(
                            text.iter()
                                .any(|s| s == if failed { "Failed" } else { "In progress" })
                        );
                        assert!(actions.contains(&"startup/quit".into()));
                        assert_eq!(actions.contains(&"startup/retry".into()), failed);
                    }
                }
            }
        }
    }
}

#[test]
fn startup_history_zero_and_over_reported_totals_have_finite_layout() {
    for (verified, total) in [
        (0, 0),
        (0, 100),
        (100, 100),
        (101, 100),
        (u64::MAX, u64::MAX),
    ] {
        let data = Startup {
            title: "Opening your cockpit".into(),
            description: String::new(),
            elapsed_secs: 0,
            active_step: 2,
            history: Some(History {
                stream: "account".into(),
                verified,
                total,
            }),
            details: vec![],
            error: None,
            logs: vec![],
        };
        let ctx = ui::Context::new(ui::Theme::Dark, ui::Density::Cozy);
        let layout = data
            .element(ctx)
            .unwrap()
            .layout(
                Rect::new(0., 0., 560., 0.),
                ctx,
                &bboxybook::components::Metrics,
            )
            .unwrap();
        inspect(&layout, &mut vec![], &mut vec![]);
        assert!(layout.rect.h.is_finite());
    }
}
