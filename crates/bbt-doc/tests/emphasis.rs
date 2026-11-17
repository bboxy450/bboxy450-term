// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A reply's `[[Done]]` draws as a pill, a knowledge `[[slug]]` stays a link,
//! lead labels and action lead-ins take symbols, and marks take their tone.
use bbt_ui::{self as ui, components as c};
use bbt_ui_model::layout::{Rect, TextMeasure, TextStyle};
use unicode_segmentation::UnicodeSegmentation;

struct Fonts;
impl TextMeasure for Fonts {
    fn width(&self, text: &str, font: &TextStyle) -> f32 {
        text.graphemes(true).count() as f32 * font.size * 0.6
    }
}
impl c::Measure for Fonts {
    fn font_metrics(&self, font: &TextStyle) -> [f32; 2] {
        [font.size * 0.8, font.size * 0.2]
    }
}
fn walk<'a>(node: &'a c::Layout, names: &mut Vec<&'a str>, text: &mut String) {
    if let c::LayoutKind::Named(name) = &node.kind {
        names.push(name);
    }
    if let c::LayoutKind::Text { runs, .. } = &node.kind {
        for run in runs {
            text.push_str(&run.text);
        }
    }
    for child in &node.children {
        walk(child, names, text);
    }
}

#[test]
fn pills_labels_and_marks_draw_while_knowledge_links_stay_links() {
    let source = "**Finding:** [[Investigate]] the deploy stopped.\n\n\
- **Result:** [[Passed 212/212]] tests → [[Failed]] migration ✓\n\
- [[Costs money]] about $4. **Check:** the bill.\n\
- See [[tasks/2026-11-13-acp-steering-parity]].\n";
    for theme in [ui::Theme::Light, ui::Theme::Dark] {
        let ctx = ui::Context::new(theme, ui::Density::Cozy);
        let layout = bbt_doc::parse(source)
            .body_element(ctx, &Fonts, 640., &[])
            .layout(Rect::new(0., 0., 640., 800.), ctx, &Fonts)
            .unwrap();
        let (mut names, mut text) = (vec![], String::new());
        walk(&layout, &mut names, &mut text);
        let pills = names
            .iter()
            .filter(|n| n.contains("/pill") && !n.ends_with("/text"))
            .count();
        assert_eq!(pills, 4, "four pills: {names:?}");
        assert!(text.contains("⌕\u{a0}Investigate"), "{text}");
        assert!(text.contains("✓\u{a0}Passed\u{a0}212/212"), "{text}");
        assert!(text.contains("$\u{a0}Costs\u{a0}money"), "{text}");
        // Labels and action lead-ins carry their symbols.
        assert!(text.contains("⌕ Finding:"), "{text}");
        assert!(text.contains("⚑ Result:"), "{text}");
        assert!(text.contains("☑ Check:"), "{text}");
        // The knowledge slug stays a wiki link, not a pill.
        assert!(names.iter().any(|n| n.contains("/link")), "{names:?}");
        assert!(!text.contains("tasks/2026-11-13-acp-steering-parity\u{a0}"));
    }
}
