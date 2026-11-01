// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxybook::{Result, toolkit};
use bbt_render::chrome::ChromeRenderer;
use std::collections::{BTreeMap, BTreeSet};

fn named(layout: &bbt_ui::components::Layout, prefix: &str) -> bool {
    matches!(&layout.kind, bbt_ui::components::LayoutKind::Named(k) if k.starts_with(prefix))
        || layout.children.iter().any(|c| named(c, prefix))
}
fn code_colours(layout: &bbt_ui::components::Layout, out: &mut BTreeSet<[u32; 4]>) {
    if let bbt_ui::components::LayoutKind::Text { runs, .. } = &layout.kind {
        for r in runs.iter().filter(|r| r.style.mono) {
            out.insert(r.style.color.map(f32::to_bits));
        }
    }
    for c in &layout.children {
        code_colours(c, out);
    }
}
#[test]
fn every_rich_widget_story_renders_headless_across_themes_and_densities() -> Result<()> {
    let mut renderer = ChromeRenderer::new(bboxybook::images()?)?;
    let cases = toolkit::cases();
    assert_eq!(cases.len(), 16 * 8);
    let mut following = BTreeMap::new();
    for case in &cases {
        let layout = toolkit::layout_case(case)?;
        fn count_text(layout: &bbt_ui::components::Layout) -> usize {
            usize::from(matches!(
                layout.kind,
                bbt_ui::components::LayoutKind::Text { .. }
            )) + layout.children.iter().map(count_text).sum::<usize>()
        }
        assert!(count_text(&layout) > 0, "{} has no text", case.relative());
        if case.state == "diff-folded" {
            assert!(!named(&layout, "diff/hunk/0/line/"));
            assert!(named(&layout, "diff/hunk/1/line/"));
        } else if case.state == "diff" {
            assert!(named(&layout, "diff/hunk/0/line/"));
        } else if case.state == "syntax-highlighted" {
            let mut colours = BTreeSet::new();
            code_colours(&layout, &mut colours);
            assert!(colours.len() > 1, "code must render syntax colours");
        }
        let pixels = toolkit::render_case(&mut renderer, case)?;
        assert_eq!(pixels.len(), (case.width * case.height * 4) as usize);
        assert!(
            pixels
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[..3] != pixels[..3])
                .count()
                > 100,
            "{} is blank",
            case.relative()
        );
        let key = (case.density.clone(), case.theme.clone());
        if case.state == "transcript" {
            following.insert(key, pixels);
        } else if case.state == "transcript-scrolled" {
            assert_ne!(
                following.remove(&key).unwrap(),
                pixels,
                "manual scroll must change the visible transcript"
            );
        }
    }
    eprintln!("{} toolkit stories rendered headless", cases.len());
    Ok(())
}
