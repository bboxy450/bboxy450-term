use bboxybook::{atoms, gallery};
use bbt_render::chrome::ChromeRenderer;
use std::collections::BTreeSet;

#[test]
fn every_atom_renders_across_density_and_theme_and_references_have_canonical_keys() {
    let cases = atoms::cases();
    assert_eq!(cases.len(), 112);
    assert_eq!(
        cases
            .iter()
            .map(|c| c.relative())
            .collect::<BTreeSet<_>>()
            .len(),
        112
    );
    assert!(
        cases
            .iter()
            .all(|c| c.fixture.as_ref().unwrap()["layer"] == "atoms")
    );
    let mut renderer = ChromeRenderer::new(bboxybook::images().unwrap()).unwrap();
    for theme in gallery::THEMES {
        for density in gallery::DENSITIES {
            let mut unique = BTreeSet::new();
            for kind in atoms::KINDS {
                let list = atoms::preview(kind, theme, density).unwrap();
                list.validate().unwrap();
                let image = renderer.render_to_rgba(&list, 640, 160, 1.).unwrap();
                assert!(image.chunks_exact(4).all(|p| p[3] == 255));
                let hash = image.iter().fold(0xcbf29ce484222325u64, |hash, &b| {
                    (hash ^ b as u64).wrapping_mul(0x100000001b3)
                });
                unique.insert(hash);
            }
            assert_eq!(
                unique.len(),
                atoms::KINDS.len(),
                "every entry must draw its own atom"
            );
        }
    }
}
