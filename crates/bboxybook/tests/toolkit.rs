// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bboxybook::{Result, toolkit};
use bbt_render::chrome::ChromeRenderer;
#[test]
fn every_rich_widget_story_renders_headless_across_themes_and_densities() -> Result<()> {
    let mut renderer = ChromeRenderer::new(bboxybook::images()?)?;
    let cases = toolkit::cases();
    assert_eq!(cases.len(), 16 * 8);
    for case in &cases {
        let layout = toolkit::layout_case(case)?;
        fn count_text(layout: &bbt_ui::components::Layout) -> usize {
            usize::from(matches!(
                layout.kind,
                bbt_ui::components::LayoutKind::Text { .. }
            )) + layout.children.iter().map(count_text).sum::<usize>()
        }
        assert!(count_text(&layout) > 0, "{} has no text", case.relative());
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
    }
    eprintln!("{} toolkit stories rendered headless", cases.len());
    Ok(())
}
